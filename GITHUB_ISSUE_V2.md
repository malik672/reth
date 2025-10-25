# Storage Worker Thread Count Should Be Configurable

## Problem

The default storage worker thread count is calculated with a static formula:

```rust
// crates/engine/primitives/src/config.rs:13-16
fn default_storage_worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| (n.get() * 2).clamp(2, 64))
        .unwrap_or(8)
}
```

**Formula:** `(CPU_cores * 2).clamp(2, 64)`

This creates a one-size-fits-all approach that doesn't account for the actual workload characteristics, specifically **page fault rates and memory pressure**.

---

## Real-World Context

### Mainnet Ethereum on 256GB Machine

**State size breakdown:**
- Hashed tables: ~130GB
- Trie tables: ~37GB
- Plain state: ~130GB
- **Total: ~297GB** (exceeds 256GB RAM by 41GB)

**Observed behavior:**
- Continuous page faults: **36k/sec** (constant)
- This is not optional - it's a consequence of state > RAM
- The `* 2` multiplier appears necessary to hide this I/O latency

### Thread Utilization Requirements

With 36k page faults/sec:
- Each page fault blocks a thread for ~1-10ms (disk I/O)
- Having only `num_cores` threads means more contention
- The `* 2` multiplier helps by providing threads that can proceed while others block
- **On this machine: 64 threads might be appropriate or even insufficient**

### Different Scenarios

| Scenario | State Size | RAM | Page Faults | Appropriate Thread Count |
|----------|-----------|-----|-------------|------------------------|
| Mainnet on 256GB | 297GB | 256GB | 36k+/sec | 64 (maybe more) |
| Mainnet on 512GB | 297GB | 512GB | <1k/sec | 16-32 (minimal paging) |
| Small chain on 32GB | 10GB | 32GB | ~0/sec | 8-16 (no paging) |
| Dev/test | <1GB | 8GB | ~0/sec | 4-8 (no paging) |

---

## The Issue

The **current formula assumes a one-size-fits-all approach**, but the optimal thread count depends heavily on:

1. **Page fault rate** - How much is the system paging?
2. **Available RAM vs state size** - Is memory constrained?
3. **Expected workload** - Are we running Mainnet or a small test chain?
4. **Hardware characteristics** - SSD speed, CPU cores, RAM speed

**Current behavior:**
- 8-core machine: 16 threads (might be overkill for low-paging workload)
- 32-core machine: 64 threads (might be exactly right for high-paging Mainnet)
- 64-core machine: 64 threads (might be insufficient for high-paging Mainnet)

There's no way to tune this for your specific hardware and workload.

---

## Proposed Solution

### Option 1: CLI Configuration (Simple, Immediate)

Add a command-line flag to override the default:

```bash
reth --storage-worker-threads 32  # Override default calculation
```

**Pros:**
- Easy to implement
- Users can tune for their hardware
- No breaking changes

**Cons:**
- Requires operator knowledge to set correctly
- Still not adaptive

### Option 2: Environment Variable (Immediate)

Allow configuration via environment variable:

```bash
RETH_STORAGE_WORKER_THREADS=32 reth
```

**Pros:**
- Easy to implement
- Useful for containerized deployments
- Complements CLI flag

### Option 3: Adaptive Configuration (Future)

Make thread count adaptive based on observed page fault rate:

```rust
fn calculate_worker_threads() -> usize {
    let page_faults_per_sec = measure_page_faults();

    if page_faults_per_sec < 1000 {
        // Minimal paging, use fewer threads
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(8)
    } else if page_faults_per_sec > 50000 {
        // Heavy paging, use more threads
        std::thread::available_parallelism()
            .map(|n| (n.get() * 3).clamp(16, 128))
            .unwrap_or(16)
    } else {
        // Moderate paging, use the current formula
        std::thread::available_parallelism()
            .map(|n| (n.get() * 2).clamp(2, 64))
            .unwrap_or(8)
    }
}
```

**Pros:**
- Automatically tunes for the actual workload
- No operator knowledge required
- Optimal for any hardware configuration

**Cons:**
- Requires measuring page faults (platform-dependent)
- More complex implementation
- Better for future iteration

---

## Why This Matters

### Example: Mainnet operators

An operator running Mainnet on a 256GB machine:
- Current: Gets 64 threads (no choice)
- With Option 1: Can tune if they understand page faults
- With Option 3: System automatically detects high paging and uses 64+ threads

### Example: Small chain operators

An operator running a test chain on a dev machine:
- Current: Gets 64 threads (wasteful)
- With Option 1: Can reduce to 8 threads (saves memory/CPU)
- With Option 3: System automatically detects low paging and uses 8 threads

---

## Implementation Approach

### Recommended: Start with Option 1 + 2 (Immediate)

1. Add `--storage-worker-threads` CLI argument
2. Add `RETH_STORAGE_WORKER_THREADS` env var support
3. Document the tradeoff (more threads = hides page fault latency, less threads = saves memory)
4. Let operators tune based on their workload

### Future: Option 3 (Adaptive)

Once configurability is in place, consider adaptive tuning based on:
- `perf_event_open` (Linux) to measure actual page faults
- Periodic recalibration during node operation
- Gradual thread pool resizing

---

## Code References

- **Current config:** `crates/engine/primitives/src/config.rs:12-22`
- **Thread spawning:** `crates/trie/parallel/src/proof_task.rs:343-360`
- **Worker loop:** `crates/trie/parallel/src/proof_task.rs:191-310`
- **CLI args:** `crates/node/core/src/args/engine.rs`

---

## Related PRs

- **PR #18887:** Introduced worker pooling for storage (Oct 10, 2025)
- **PR #19012:** Background initialization of workers (Oct 15, 2025)

---

## Measurement Methodology

For operators wanting to measure their own setup:

```bash
# Linux: Check page fault rate
vmstat 1

# Look at "in" (interrupts) and "po" (page outs) columns
# Calculate page faults/sec = (po - previous_po) per interval

# Watch page faults in real-time
watch -n 1 'grep -E "pgfault|pswpin|pswpout" /proc/vmstat'
```

Real-world data from Mainnet on 256GB machine:
- Page fault rate: ~36k/sec (continuous, during normal operation)
- This correlates with network activity and block production
- Optimal thread count for this workload: 64+ (to hide I/O latency)

---

## Decision Tree for Operators

```
Running Mainnet?
  YES → Likely need 32-64 threads (memory-constrained system)
        Check page faults: vmstat
        If >20k/sec: Use 64 threads
        If <5k/sec: Can reduce to 32
  NO → Running small chain?
    YES → Likely need 4-16 threads (minimal paging)
          Measure page faults first
          Start low, increase if needed
    NO → Use default (probably fine)
```

---

## Next Steps

1. **Discuss:** Is configurability the right first step?
2. **Implement:** Add CLI flag + env var (5-10 min work)
3. **Test:** Verify on different hardware configurations
4. **Document:** Add to docs with decision tree above
5. **Future:** Consider adaptive scaling (Phase 2)

---

## Summary

The `(cores * 2).clamp(2, 64)` formula works well for some scenarios (high-paging Mainnet) but is suboptimal for others (low-paging small chains). Rather than changing the default formula, we should **let operators tune it for their specific workload**.

This is a **configuration problem**, not a **code problem**.
