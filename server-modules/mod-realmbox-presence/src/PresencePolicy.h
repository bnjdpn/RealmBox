#ifndef REALMBOX_PRESENCE_POLICY_H
#define REALMBOX_PRESENCE_POLICY_H

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <limits>

namespace RealmBoxPresence
{
constexpr std::uint32_t MinimumAutonomyReturnSeconds = 30;
constexpr std::uint32_t MaximumAutonomyReturnSeconds = 86400;

constexpr std::uint32_t ClampAutonomyReturnSeconds(std::int64_t seconds)
{
    return static_cast<std::uint32_t>(std::clamp<std::int64_t>(
        seconds, MinimumAutonomyReturnSeconds, MaximumAutonomyReturnSeconds));
}

struct AutonomousBotState
{
    bool usable = false;
    bool openWorld = false;
    bool alive = false;
    bool randomBot = false;
    bool hasAI = false;
    bool hasMaster = false;
    bool grouped = false;
    bool inBattleground = false;
    bool inArena = false;
    bool queuedForBattleground = false;
    bool inRandomLfgDungeon = false;
};

constexpr bool IsAutonomousBot(AutonomousBotState const& state)
{
    return state.usable && state.openWorld && state.alive && state.randomBot && state.hasAI && !state.hasMaster &&
           !state.grouped && !state.inBattleground && !state.inArena && !state.queuedForBattleground &&
           !state.inRandomLfgDungeon;
}

struct MoveSafetyState
{
    bool hasAI = false;
    bool inCombat = false;
    bool inFlight = false;
    bool onTransport = false;
    bool inVehicle = false;
    bool canMove = false;
    bool lfgIdle = false;
};

constexpr bool IsSafeToMove(MoveSafetyState const& state)
{
    return state.hasAI && !state.inCombat && !state.inFlight && !state.onTransport && !state.inVehicle &&
           state.canMove && state.lfgIdle;
}

struct AutonomyReturnState
{
    AutonomousBotState bot;
    MoveSafetyState movement;
    bool trackedByThisInstance = false;
    bool visibleToRealPlayer = false;
};

constexpr bool CanScheduleAutonomyReturn(AutonomyReturnState const& state)
{
    return state.trackedByThisInstance && IsAutonomousBot(state.bot) && IsSafeToMove(state.movement) &&
           !state.visibleToRealPlayer;
}

constexpr bool WouldAccelerateAutonomyReturn(
    std::uint64_t trackedReturnAt, std::uint64_t now, std::uint32_t requestedDelaySeconds)
{
    return trackedReturnAt > now && (trackedReturnAt - now) > requestedDelaySeconds;
}

inline std::uint32_t CalculateDesiredNearby(std::uint32_t totalAutonomousBots,
    std::uint32_t sameFactionBots, std::uint32_t realPlayerCount, float targetFraction,
    std::uint32_t minimumPerPlayer, std::uint32_t maximumPerPlayer)
{
    if (!totalAutonomousBots || !sameFactionBots || !realPlayerCount || targetFraction <= 0.0f ||
        !maximumPerPlayer)
        return 0;

    float rawDesired = (static_cast<float>(totalAutonomousBots) * targetFraction) / realPlayerCount;
    float nearestInteger = std::round(rawDesired);
    float integerTolerance = std::numeric_limits<float>::epsilon() *
                             std::max(1.0f, std::abs(rawDesired)) * 8.0f;
    if (std::abs(rawDesired - nearestInteger) <= integerTolerance)
        rawDesired = nearestInteger;

    std::uint32_t desired = static_cast<std::uint32_t>(std::ceil(rawDesired));
    minimumPerPlayer = std::min(minimumPerPlayer, maximumPerPlayer);
    desired = std::clamp(desired, minimumPerPlayer, maximumPerPlayer);
    return std::min(desired, sameFactionBots);
}
} // namespace RealmBoxPresence

#endif
