#include "RealmBoxOllamaBackoff.h"

#include <cassert>
#include <cstdint>
#include <initializer_list>
#include <limits>

using Backoff = RealmBoxOllamaBackoff;

static void Fail(Backoff& policy, uint64_t now)
{
    const auto permit = policy.Acquire(now);
    assert(permit.allowed);
    policy.Complete(permit, false, now);
}

static void Open(Backoff& policy, uint64_t now)
{
    Fail(policy, now);
    Fail(policy, now);
    Fail(policy, now);
}

int main()
{
    {
        Backoff policy;
        Open(policy, 100);
        assert(!policy.Acquire(5099).allowed);
        const auto probe = policy.Acquire(5100);
        assert(probe.allowed && probe.probe);
        assert(!policy.Acquire(5100).allowed); // one shared half-open probe
        policy.Complete(probe, true, 5101);
        const auto recovered = policy.Acquire(5101);
        assert(recovered.allowed && !recovered.probe);
    }
    {
        Backoff policy;
        Fail(policy, 0);
        Fail(policy, 1);
        const auto success = policy.Acquire(2);
        policy.Complete(success, true, 2);
        Fail(policy, 3);
        Fail(policy, 4);
        assert(policy.Acquire(5).allowed); // success reset the failure streak
    }
    {
        Backoff policy;
        Open(policy, 0);
        uint64_t retryAt = 0;
        for (const uint64_t cooldown : { 5000u, 10000u, 20000u, 40000u, 60000u, 60000u })
        {
            retryAt += cooldown;
            assert(!policy.Acquire(retryAt - 1).allowed);
            const auto probe = policy.Acquire(retryAt);
            assert(probe.allowed && probe.probe);
            policy.Complete(probe, false, retryAt);
        }
    }
    {
        Backoff policy;
        const auto staleSuccess = policy.Acquire(0);
        const auto staleFailure = policy.Acquire(0);
        Open(policy, 0);
        policy.Complete(staleSuccess, true, 1000);
        policy.Complete(staleFailure, false, 4000);
        assert(!policy.Acquire(4999).allowed);
        const auto probe = policy.Acquire(5000);
        assert(probe.allowed); // stale failure did not extend the cooldown
        policy.Reset();
        policy.Complete(probe, false, 5001);
        assert(policy.Acquire(5001).allowed); // stale probe cannot undo reload
    }
    {
        Backoff policy;
        const uint64_t maximum = std::numeric_limits<uint64_t>::max();
        Open(policy, maximum - 100);
        assert(!policy.Acquire(maximum).allowed); // deadline never wraps to zero
        policy.Reset();
        assert(policy.Acquire(maximum).allowed);
    }
    return 0;
}
