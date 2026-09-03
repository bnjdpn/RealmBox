#include "Config.h"
#include "GameTime.h"
#include "LFGMgr.h"
#include "Log.h"
#include "Map.h"
#include "MotionMaster.h"
#include "Player.h"
#include "Playerbots.h"
#include "PresencePolicy.h"
#include "Random.h"
#include "ScriptMgr.h"
#include "TravelMgr.h"
#include "WorldSession.h"

#include <algorithm>
#include <cmath>
#include <unordered_map>
#include <unordered_set>
#include <vector>

#ifndef MOD_PLAYERBOTS
#error "mod-realmbox-presence requires mod-playerbots"
#endif

namespace
{
struct PresenceConfig
{
    bool enabled = true;
    uint32 scanIntervalMs = 3000;
    float targetFraction = 0.60f;
    uint32 minBotsPerPlayer = 4;
    uint32 maxBotsPerPlayer = 30;
    float nearbyRadius = 260.0f;
    float spawnMinRadius = 110.0f;
    float spawnMaxRadius = 220.0f;
    float disappearanceGuardRadius = 150.0f;
    uint32 pointAttempts = 12;
    uint32 maxMovesPerScan = 1;
    uint32 playerCooldownSeconds = 2;
    uint32 botCooldownSeconds = 120;
    uint32 releasedBotGraceSeconds = 300;
    uint32 autonomyReturnSeconds = 600;
    uint32 maxLevelDelta = 0;
    bool sameZoneOnly = true;
};

uint32 ReadUInt(char const* key, uint32 defaultValue, uint32 minValue, uint32 maxValue)
{
    int32 value = sConfigMgr->GetOption<int32>(key, static_cast<int32>(defaultValue));
    return static_cast<uint32>(std::clamp(value, static_cast<int32>(minValue), static_cast<int32>(maxValue)));
}

float ReadFloat(char const* key, float defaultValue, float minValue, float maxValue)
{
    return std::clamp(sConfigMgr->GetOption<float>(key, defaultValue), minValue, maxValue);
}

class RealmBoxPresenceWorldScript final : public WorldScript
{
public:
    RealmBoxPresenceWorldScript()
        : WorldScript("RealmBoxPresenceWorldScript",
              { WORLDHOOK_ON_STARTUP, WORLDHOOK_ON_UPDATE, WORLDHOOK_ON_AFTER_CONFIG_LOAD })
    {
    }

    void OnStartup() override { LoadConfig(); }

    void OnAfterConfigLoad(bool reload) override
    {
        if (reload)
            LoadConfig();
    }

    void OnUpdate(uint32 diff) override
    {
        _elapsedMs += diff;
        if (_elapsedMs < _config.scanIntervalMs)
            return;

        _elapsedMs = 0;
        if (!_config.enabled)
        {
            ReleaseAutonomousBots();
            return;
        }

        RunPresencePass();
    }

private:
    using LowGuid = ObjectGuid::LowType;
    using DeadlineMap = std::unordered_map<LowGuid, uint64>;

    void LoadConfig()
    {
        bool wasEnabled = _config.enabled;
        PresenceConfig next;
        next.enabled = sConfigMgr->GetOption<bool>("RealmBoxPresence.Enabled", true);
        next.scanIntervalMs = ReadUInt("RealmBoxPresence.ScanIntervalMs", 3000, 1000, 60000);
        next.targetFraction = ReadFloat("RealmBoxPresence.TargetFraction", 0.60f, 0.0f, 1.0f);
        next.minBotsPerPlayer = ReadUInt("RealmBoxPresence.MinBotsPerPlayer", 4, 0, 100);
        next.maxBotsPerPlayer = ReadUInt("RealmBoxPresence.MaxBotsPerPlayer", 30, 0, 100);
        next.maxBotsPerPlayer = std::max(next.maxBotsPerPlayer, next.minBotsPerPlayer);
        next.nearbyRadius = ReadFloat("RealmBoxPresence.NearbyRadius", 260.0f, 30.0f, 1000.0f);
        next.spawnMinRadius = ReadFloat("RealmBoxPresence.SpawnMinRadius", 110.0f, 20.0f, 900.0f);
        next.spawnMaxRadius = ReadFloat("RealmBoxPresence.SpawnMaxRadius", 220.0f, 30.0f, 1000.0f);
        next.spawnMaxRadius = std::max(next.spawnMaxRadius, next.spawnMinRadius + 1.0f);
        next.disappearanceGuardRadius =
            ReadFloat("RealmBoxPresence.DisappearanceGuardRadius", 150.0f, 0.0f, 500.0f);
        next.pointAttempts = ReadUInt("RealmBoxPresence.PointAttempts", 12, 1, 50);
        next.maxMovesPerScan = ReadUInt("RealmBoxPresence.MaxMovesPerScan", 1, 1, 10);
        next.playerCooldownSeconds = ReadUInt("RealmBoxPresence.PlayerCooldownSeconds", 2, 0, 3600);
        next.botCooldownSeconds = ReadUInt("RealmBoxPresence.BotCooldownSeconds", 120, 0, 86400);
        next.releasedBotGraceSeconds = ReadUInt("RealmBoxPresence.ReleasedBotGraceSeconds", 300, 0, 86400);
        next.autonomyReturnSeconds = RealmBoxPresence::ClampAutonomyReturnSeconds(
            sConfigMgr->GetOption<int32>("RealmBoxPresence.AutonomyReturnSeconds", 600));
        next.maxLevelDelta = ReadUInt("RealmBoxPresence.MaxLevelDelta", 0, 0, 79);
        next.sameZoneOnly = sConfigMgr->GetOption<bool>("RealmBoxPresence.SameZoneOnly", true);

        _config = next;
        _elapsedMs = 0;
        _nextAnchor = 0;

        // A profile reload must not inherit placement delays from the previous profile.
        // Released-playerbot grace is deliberately retained while enabled so a config reload
        // cannot immediately recapture a companion that has just left a real player's group.
        _playerNextMoveAt.clear();
        _botNextMoveAt.clear();
        if (!_config.enabled)
        {
            _releasedUntil.clear();
            _previouslyControlled.clear();
        }
        else if (!wasEnabled)
        {
            _releasedUntil.clear();
            _previouslyControlled.clear();
        }

        LOG_INFO("playerbots",
            "[RealmBoxPresence] enabled={}, target={}%, nearby={} yd, placement={}..{} yd, max/player={}, autonomy-return={}s",
            _config.enabled, static_cast<uint32>(_config.targetFraction * 100.0f), _config.nearbyRadius,
            _config.spawnMinRadius, _config.spawnMaxRadius, _config.maxBotsPerPlayer,
            _config.autonomyReturnSeconds);
    }

    static bool IsUsable(Player* player)
    {
        return player && player->GetSession() && player->IsInWorld() && !player->GetSession()->isLogingOut() &&
               !player->IsDuringRemoveFromWorld() && !player->IsBeingTeleported();
    }

    static bool IsOpenWorld(Player* player)
    {
        return player && player->GetMap() && !player->GetMap()->Instanceable();
    }

    bool IsEligibleAnchor(Player* player) const
    {
        return IsRealPlayer(player) && IsUsable(player) && IsOpenWorld(player) && player->IsAlive() &&
               !player->IsInCombat() && !player->IsInFlight() && !player->GetTransport() && !player->GetVehicle();
    }

    RealmBoxPresence::AutonomousBotState AutonomousBotStateFor(Player* bot) const
    {
        if (!bot)
            return {};

        PlayerbotAI* botAI = GET_PLAYERBOT_AI(bot);
        return {
            .usable = IsUsable(bot),
            .openWorld = IsOpenWorld(bot),
            .alive = bot->IsAlive(),
            .randomBot = sRandomPlayerbotMgr.IsRandomBot(bot),
            .hasAI = botAI != nullptr,
            .hasMaster = botAI && botAI->GetMaster(),
            .grouped = bot->GetGroup() != nullptr,
            .inBattleground = bot->InBattleground(),
            .inArena = bot->InArena(),
            .queuedForBattleground = bot->InBattlegroundQueue(),
            .inRandomLfgDungeon = bot->inRandomLfgDungeon(),
        };
    }

    bool IsAutonomousRandomBot(Player* bot) const
    {
        return RealmBoxPresence::IsAutonomousBot(AutonomousBotStateFor(bot));
    }

    RealmBoxPresence::MoveSafetyState MoveSafetyStateFor(Player* bot, PlayerbotAI* botAI) const
    {
        if (!bot || !botAI)
            return {};

        return {
            .hasAI = true,
            .inCombat = bot->IsInCombat(),
            .inFlight = bot->IsInFlight(),
            .onTransport = bot->GetTransport() != nullptr,
            .inVehicle = bot->GetVehicle() != nullptr,
            .canMove = botAI->CanMove(),
            .lfgIdle = sLFGMgr->GetState(bot->GetGUID()) == lfg::LFG_STATE_NONE,
        };
    }

    bool IsSafeToMove(Player* bot, PlayerbotAI* botAI) const
    {
        return RealmBoxPresence::IsSafeToMove(MoveSafetyStateFor(bot, botAI));
    }

    static bool DeadlineActive(DeadlineMap const& deadlines, LowGuid guid, uint64 now)
    {
        auto const itr = deadlines.find(guid);
        return itr != deadlines.end() && itr->second > now;
    }

    static void PruneDeadlines(DeadlineMap& deadlines, uint64 now)
    {
        for (auto itr = deadlines.begin(); itr != deadlines.end();)
        {
            if (itr->second <= now)
                itr = deadlines.erase(itr);
            else
                ++itr;
        }
    }

    void TrackReleasedBots(PlayerBotMap const& bots, uint64 now)
    {
        std::unordered_set<LowGuid> currentlyControlled;
        for (auto const& [guid, bot] : bots)
        {
            if (!bot || !sRandomPlayerbotMgr.IsRandomBot(bot))
                continue;

            PlayerbotAI* botAI = GET_PLAYERBOT_AI(bot);
            if (!botAI)
                continue;

            LowGuid lowGuid = guid.GetCounter();
            if (bot->GetGroup() || botAI->GetMaster())
                currentlyControlled.insert(lowGuid);
            else if (_previouslyControlled.contains(lowGuid))
                _releasedUntil[lowGuid] = now + _config.releasedBotGraceSeconds;
        }

        _previouslyControlled.swap(currentlyControlled);
    }

    void ReleaseAutonomousBots()
    {
        uint64 now = GameTime::GetGameTime().count();
        PruneDeadlines(_realmBoxPlacedUntil, now);
        if (_realmBoxPlacedUntil.empty())
            return;

        PlayerBotMap bots = sRandomPlayerbotMgr.GetAllBots();
        for (auto const& [guid, bot] : bots)
        {
            if (!bot)
                continue;

            LowGuid lowGuid = guid.GetCounter();
            auto tracked = _realmBoxPlacedUntil.find(lowGuid);
            if (tracked == _realmBoxPlacedUntil.end())
                continue;

            PlayerbotAI* botAI = GET_PLAYERBOT_AI(bot);
            bool visibleToRealPlayer = _config.disappearanceGuardRadius > 0.0f && botAI &&
                                       botAI->HasPlayerNearby(_config.disappearanceGuardRadius);
            if (!RealmBoxPresence::CanScheduleAutonomyReturn({
                    .bot = AutonomousBotStateFor(bot),
                    .movement = MoveSafetyStateFor(bot, botAI),
                    .trackedByThisInstance = true,
                    .visibleToRealPlayer = visibleToRealPlayer,
                }))
                continue;

            // Never replace an already-earlier deadline. Ownership is deliberately memory-only:
            // Playerbots exposes no public metadata that can distinguish RealmBox's old SetValue
            // event from a native event with the same value and lifetime.
            if (!RealmBoxPresence::WouldAccelerateAutonomyReturn(
                    tracked->second, now, _config.autonomyReturnSeconds))
            {
                _realmBoxPlacedUntil.erase(tracked);
                continue;
            }

            sRandomPlayerbotMgr.ScheduleTeleport(lowGuid, _config.autonomyReturnSeconds);
            _realmBoxPlacedUntil.erase(tracked);
            LOG_DEBUG("playerbots", "[RealmBoxPresence] returned autonomous bot {} to its native travel schedule",
                bot->GetName());
        }
    }

    bool BuildPlacement(Player* anchor, WorldPosition& target) const
    {
        Map* map = anchor->GetMap();
        WorldPosition origin(anchor);

        for (uint32 attempt = 0; attempt < _config.pointAttempts; ++attempt)
        {
            target = origin;
            // Validate the segment on the anchor's map. Passing a bot from another map here would
            // make CanReachPositionAndGetValidCoords start from coordinates on the wrong map.
            if (!target.GetReachableRandomPointOnGround(anchor, _config.spawnMaxRadius, true))
                continue;

            float distance = anchor->GetDistance2d(target.GetPositionX(), target.GetPositionY());
            if (distance < _config.spawnMinRadius)
                continue;

            if (_config.sameZoneOnly &&
                map->GetZoneId(anchor->GetPhaseMask(), target.GetPositionX(), target.GetPositionY(),
                    target.GetPositionZ()) != anchor->GetZoneId())
                continue;

            if (map->IsInWater(anchor->GetPhaseMask(), target.GetPositionX(), target.GetPositionY(),
                    target.GetPositionZ(), anchor->GetCollisionHeight()))
                continue;

            target.setO(frand(0.0f, 6.283185307f));
            return true;
        }

        return false;
    }

    bool TeleportAutonomousBot(Player* bot, PlayerbotAI* botAI, WorldPosition const& target, uint64 now)
    {
        bot->GetMotionMaster()->Clear();
        botAI->Reset(true);
        bot->RemoveAurasWithInterruptFlags(AURA_INTERRUPT_FLAG_TELEPORTED | AURA_INTERRUPT_FLAG_CHANGE_MAP);

        if (!bot->TeleportTo(target))
            return false;

        // Give the bot a bounded stay near the player, then hand its travel lifecycle back to
        // RandomPlayerbotMgr. ScheduleTeleport persists the real deadline instead of using the
        // generic SetValue API, whose validity is the full max-in-world duration.
        LowGuid lowGuid = bot->GetGUID().GetCounter();
        sRandomPlayerbotMgr.ScheduleTeleport(lowGuid, _config.autonomyReturnSeconds);
        _realmBoxPlacedUntil[lowGuid] = now + _config.autonomyReturnSeconds;
        bot->SendMovementFlagUpdate();
        return true;
    }

    bool MoveOneBotFor(Player* anchor, std::vector<Player*> const& anchors, PlayerBotMap const& bots, uint64 now)
    {
        if (_config.targetFraction <= 0.0f || _config.maxBotsPerPlayer == 0)
            return false;

        uint32 totalAutonomousBots = 0;
        std::vector<Player*> factionBots;
        factionBots.reserve(bots.size());
        for (auto const& [guid, bot] : bots)
        {
            (void)guid;
            if (!IsAutonomousRandomBot(bot))
                continue;

            ++totalAutonomousBots;
            if (bot->GetTeamId() == anchor->GetTeamId())
                factionBots.push_back(bot);
        }

        if (!totalAutonomousBots || factionBots.empty())
            return false;

        // Never pull hostile-faction bots into the player's zone. If the global majority target is
        // larger than the friendly population, converge on every available same-faction bot.
        uint32 desired = RealmBoxPresence::CalculateDesiredNearby(totalAutonomousBots,
            static_cast<uint32>(factionBots.size()), static_cast<uint32>(anchors.size()), _config.targetFraction,
            _config.minBotsPerPlayer, _config.maxBotsPerPlayer);

        uint32 nearby = 0;
        for (Player* bot : factionBots)
        {
            if (bot->GetMapId() == anchor->GetMapId() && anchor->InSamePhase(bot) &&
                anchor->GetDistance2d(bot) <= _config.nearbyRadius)
                ++nearby;
        }
        if (nearby >= desired)
            return false;

        std::vector<Player*> candidates;
        for (Player* bot : factionBots)
        {
            LowGuid guid = bot->GetGUID().GetCounter();
            PlayerbotAI* botAI = GET_PLAYERBOT_AI(bot);
            if (!anchor->InSamePhase(bot) || DeadlineActive(_botNextMoveAt, guid, now) ||
                DeadlineActive(_releasedUntil, guid, now) || !IsSafeToMove(bot, botAI))
                continue;

            if (_config.maxLevelDelta &&
                std::abs(static_cast<int32>(bot->GetLevel()) - static_cast<int32>(anchor->GetLevel())) >
                    static_cast<int32>(_config.maxLevelDelta))
                continue;

            if (bot->GetMapId() == anchor->GetMapId() &&
                anchor->GetDistance2d(bot) <= _config.nearbyRadius)
                continue;

            // Never make a bot disappear while another real player can see it.
            if (_config.disappearanceGuardRadius > 0.0f &&
                botAI->HasPlayerNearby(_config.disappearanceGuardRadius))
                continue;

            candidates.push_back(bot);
        }

        if (candidates.empty())
            return false;

        WorldPosition target;
        if (!BuildPlacement(anchor, target))
            return false;

        uint32 start = urand(0, static_cast<uint32>(candidates.size() - 1));
        for (uint32 offset = 0; offset < candidates.size(); ++offset)
        {
            Player* bot = candidates[(start + offset) % candidates.size()];
            PlayerbotAI* botAI = GET_PLAYERBOT_AI(bot);
            if (!TeleportAutonomousBot(bot, botAI, target, now))
                continue;

            _botNextMoveAt[bot->GetGUID().GetCounter()] = now + _config.botCooldownSeconds;
            LOG_DEBUG("playerbots", "[RealmBoxPresence] moved autonomous bot {} near {} ({}/{})", bot->GetName(),
                anchor->GetName(), nearby + 1, desired);
            return true;
        }

        return false;
    }

    void RunPresencePass()
    {
        uint64 now = static_cast<uint64>(GameTime::GetGameTime().count());
        PlayerBotMap bots = sRandomPlayerbotMgr.GetAllBots();
        TrackReleasedBots(bots, now);
        PruneDeadlines(_playerNextMoveAt, now);
        PruneDeadlines(_botNextMoveAt, now);
        PruneDeadlines(_releasedUntil, now);

        std::vector<Player*> anchors;
        for (Player* player : sRandomPlayerbotMgr.GetPlayers())
        {
            if (IsEligibleAnchor(player))
                anchors.push_back(player);
        }
        if (anchors.empty())
            return;

        _nextAnchor %= anchors.size();
        uint32 moved = 0;
        for (size_t offset = 0; offset < anchors.size() && moved < _config.maxMovesPerScan; ++offset)
        {
            Player* anchor = anchors[(_nextAnchor + offset) % anchors.size()];
            LowGuid guid = anchor->GetGUID().GetCounter();
            if (DeadlineActive(_playerNextMoveAt, guid, now))
                continue;

            if (MoveOneBotFor(anchor, anchors, bots, now))
            {
                _playerNextMoveAt[guid] = now + _config.playerCooldownSeconds;
                ++moved;
            }
        }

        _nextAnchor = (_nextAnchor + 1) % anchors.size();
    }

    PresenceConfig _config;
    uint32 _elapsedMs = 0;
    size_t _nextAnchor = 0;
    DeadlineMap _playerNextMoveAt;
    DeadlineMap _botNextMoveAt;
    DeadlineMap _releasedUntil;
    DeadlineMap _realmBoxPlacedUntil;
    std::unordered_set<LowGuid> _previouslyControlled;
};
} // namespace realmbox presence internals

void AddSC_realmbox_presence()
{
    new RealmBoxPresenceWorldScript();
}
