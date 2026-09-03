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
