#include "PresencePolicy.h"
#include "gtest/gtest.h"

TEST(RealmBoxPresencePolicy, GlobalMajorityIsClampedToFriendlyPopulation)
{
    EXPECT_EQ(RealmBoxPresence::CalculateDesiredNearby(50, 27, 1, 0.60f, 4, 30), 27u);
    EXPECT_EQ(RealmBoxPresence::CalculateDesiredNearby(50, 23, 1, 0.60f, 4, 30), 23u);
}

TEST(RealmBoxPresencePolicy, GlobalTargetIsSharedAcrossRealPlayers)
{
    EXPECT_EQ(RealmBoxPresence::CalculateDesiredNearby(50, 27, 2, 0.60f, 4, 30), 15u);
}

TEST(RealmBoxPresencePolicy, DisabledOrEmptyConfigurationsMoveNobody)
{
    EXPECT_EQ(RealmBoxPresence::CalculateDesiredNearby(50, 27, 1, 0.0f, 4, 30), 0u);
    EXPECT_EQ(RealmBoxPresence::CalculateDesiredNearby(0, 0, 1, 0.60f, 4, 30), 0u);
    EXPECT_EQ(RealmBoxPresence::CalculateDesiredNearby(50, 27, 0, 0.60f, 4, 30), 0u);
    EXPECT_EQ(RealmBoxPresence::CalculateDesiredNearby(50, 27, 1, 0.60f, 0, 0), 0u);
}

TEST(RealmBoxPresencePolicy, GroupedOrMasteredBotsAreNeverAutonomousCandidates)
{
    RealmBoxPresence::AutonomousBotState state{
        .usable = true,
        .openWorld = true,
        .alive = true,
        .randomBot = true,
        .hasAI = true,
    };
    EXPECT_TRUE(RealmBoxPresence::IsAutonomousBot(state));

    state.grouped = true;
    EXPECT_FALSE(RealmBoxPresence::IsAutonomousBot(state));
    state.grouped = false;
    state.hasMaster = true;
    EXPECT_FALSE(RealmBoxPresence::IsAutonomousBot(state));
}

TEST(RealmBoxPresencePolicy, UnsafeActivitiesBlockRelocation)
{
    RealmBoxPresence::MoveSafetyState state{
        .hasAI = true,
        .canMove = true,
        .lfgIdle = true,
    };
    EXPECT_TRUE(RealmBoxPresence::IsSafeToMove(state));

    state.inCombat = true;
    EXPECT_FALSE(RealmBoxPresence::IsSafeToMove(state));
    state.inCombat = false;
    state.onTransport = true;
    EXPECT_FALSE(RealmBoxPresence::IsSafeToMove(state));
    state.onTransport = false;
    state.lfgIdle = false;
    EXPECT_FALSE(RealmBoxPresence::IsSafeToMove(state));
}

TEST(RealmBoxPresencePolicy, AutonomyReturnSecondsIsBounded)
{
    EXPECT_EQ(RealmBoxPresence::ClampAutonomyReturnSeconds(-1), 30u);
    EXPECT_EQ(RealmBoxPresence::ClampAutonomyReturnSeconds(0), 30u);
    EXPECT_EQ(RealmBoxPresence::ClampAutonomyReturnSeconds(29), 30u);
    EXPECT_EQ(RealmBoxPresence::ClampAutonomyReturnSeconds(600), 600u);
    EXPECT_EQ(RealmBoxPresence::ClampAutonomyReturnSeconds(86400), 86400u);
    EXPECT_EQ(RealmBoxPresence::ClampAutonomyReturnSeconds(86401), 86400u);
}

TEST(RealmBoxPresencePolicy, AutonomyReturnRequiresAnUnseenSafeAutonomousBot)
{
    RealmBoxPresence::AutonomyReturnState state{
        .bot = {
            .usable = true,
            .openWorld = true,
            .alive = true,
            .randomBot = true,
            .hasAI = true,
        },
        .movement = {
            .hasAI = true,
            .canMove = true,
            .lfgIdle = true,
        },
        .trackedByThisInstance = true,
    };
    EXPECT_TRUE(RealmBoxPresence::CanScheduleAutonomyReturn(state));

    state.trackedByThisInstance = false;
    EXPECT_FALSE(RealmBoxPresence::CanScheduleAutonomyReturn(state));
    state.trackedByThisInstance = true;
    state.visibleToRealPlayer = true;
    EXPECT_FALSE(RealmBoxPresence::CanScheduleAutonomyReturn(state));
    state.visibleToRealPlayer = false;
    state.bot.grouped = true;
    EXPECT_FALSE(RealmBoxPresence::CanScheduleAutonomyReturn(state));
    state.bot.grouped = false;
    state.bot.hasMaster = true;
    EXPECT_FALSE(RealmBoxPresence::CanScheduleAutonomyReturn(state));
    state.bot.hasMaster = false;
    state.movement.inCombat = true;
    EXPECT_FALSE(RealmBoxPresence::CanScheduleAutonomyReturn(state));
}

TEST(RealmBoxPresencePolicy, AutonomyReturnOnlyReplacesALaterTrackedDeadline)
{
    EXPECT_TRUE(RealmBoxPresence::WouldAccelerateAutonomyReturn(1000, 100, 60));
    EXPECT_FALSE(RealmBoxPresence::WouldAccelerateAutonomyReturn(160, 100, 60));
    EXPECT_FALSE(RealmBoxPresence::WouldAccelerateAutonomyReturn(150, 100, 60));
    EXPECT_FALSE(RealmBoxPresence::WouldAccelerateAutonomyReturn(100, 100, 60));
    EXPECT_FALSE(RealmBoxPresence::WouldAccelerateAutonomyReturn(90, 100, 60));
}
