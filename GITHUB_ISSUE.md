# Storage Worker Thread Pool Over-Provisioning: Analysis & Solution

## Executive Summary

The current storage worker thread architecture (introduced in PR #18887) over-provisions threads based on CPU cores, when the **real bottleneck is database transactions**, not CPU parallelism.

**Proposed solution:** Replace pre-spawned worker threads with a bounded **transaction pool** using Tokio's existing global thread pool.

---

## The Problem

### Current Architecture (PR #18887)

```rust
// crates/engine/primitives/src/config.rs:13-16
fn default_storage_worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| (n.get() * 2).clamp(2, 64))  // ← 64 threads on 32-core!
        .unwrap_or(8)
}

// crates/trie/parallel/src/proof_task.rs:343-360
for worker_id in 0..storage_worker_count {  // Pre-spawn all threads
    let tx = provider_ro.into_tx();          // Each gets ONE transaction
    executor.spawn_blocking(move ||
        storage_worker_loop(proof_task_tx, work_rx, worker_id)
    );
}
```

### The Inefficiency

| System | Cores | Threads Spawned | Typical Accounts | Actual Usage | Memory Waste |
|--------|-------|-----------------|------------------|--------------|--------------|
| Small server | 8 | 16 | 15 | ~94% ✅ | ~32MB |
| Medium server | 16 | 32 | 15 | ~47% ⚠️ | ~64MB |
| Large server | 32 | 64 | 15 | ~23% ❌ | ~128MB |
| Huge server | 64 | 64 | 15 | ~23% ❌ | ~128MB |

**Real-world data from Mainnet (256GB machine):**
- Working set: ~297GB (exceeds 256GB RAM)
- Page faults: 8.6M/sec during tip processing
- Modified accounts per block: typically 10-20

---

## Why The `* 2` Multiplier Exists (And Why It's Wrong)

### The Original Intent

The `* 2` multiplier was designed to **hide page fault latency**:

```
Thread 1: Read MDBX page → Page fault (blocked on disk I/O)
Thread 2: Continue work while Thread 1 blocked
Thread 3: Continue work while Thread 2 blocked
```

### Why It Doesn't Work

1. **Actual bottleneck is transactions, not threads**
   - MDBX page faults are system-level, not worker-level
   - Extra threads don't help; OS scheduler does the masking
   - Only **N transactions** can exist simultaneously

2. **Over-provisioning wastes resources**
   - 64 threads × ~2MB stack each = ~128MB+ idle memory
   - Excess context switching overhead (64 threads vs 15 with work)
   - No actual performance benefit for CPU-bound work

3. **Already solved in pre-PR #18887 code**
   - Previous implementation had on-demand transaction pooling
   - Used `Vec<ProofTaskTx>` and reused transactions
   - Limited parallelism to `max_concurrency` (5-10), not thread count

---

## The Solution: Transaction Pool, Not Thread Pool

### Pattern That Was Already Working (Pre-PR #18887)

```rust
// OLD CORRECT PATTERN (deleted by PR #18887)
pub fn get_or_create_tx(&mut self) -> Option<ProofTaskTx> {
    // Try to reuse an existing transaction from the pool
    if let Some(proof_task_tx) = self.proof_task_txs.pop() {
        return Some(proof_task_tx);  // ← Transaction pooling
    }

    // Create new transaction up to limit
    if self.total_transactions < max_concurrency {
        return Some(create_new_tx());
    }

    None  // Wait for one to be available
}

// Use Tokio's thread pool (already optimal):
self.executor.spawn_blocking(move || {
    proof_task_tx.do_work();
    tx_pool.push(proof_task_tx);  // ← Return tool when done
});
```

### Proposed Implementation

```rust
// Create bounded transaction pool (the REAL bottleneck)
let tx_pool = crossbeam_channel::bounded(num_cores);

// Fill it with reusable transactions
for _ in 0..num_cores {
    tx_pool.send(ProofTaskTx::new(create_tx())).unwrap();
}

// When work arrives:
let proof_task_tx = tx_pool.recv()?;  // Get a free transaction
executor.spawn_blocking(move || {
    let result = proof_task_tx.compute_storage_proof(...);
    tx_pool.send(proof_task_tx)?;  // Return transaction
});
```

### Benefits

| Aspect | Current (PR #18887) | Proposed |
|--------|---------------------|----------|
| **Threads** | 64 pre-spawned | Tokio's global pool (reused) |
| **Transactions** | 1 per thread (64 total) | num_cores (8-64) |
| **Memory** | ~128MB+ idle stacks | ~0 extra |
| **Context switching** | Excessive (64 threads) | Minimal |
| **Parallelism limit** | Wasted capacity | Matches actual bottleneck |
| **Adaptive to load** | Static | Dynamic (Tokio queues) |

---

## Historical Context: Why It Was Changed

### PR #18887 ("worker pooling for storage")
- **Goal:** Improve proof generation performance
- **Solution:** Pre-spawn worker threads with pooled transactions
- **Result:** Worked, but for wrong reasons (memory-mapped I/O masking, not thread count)

### PR #19012 ("background init of workers")
- **Goal:** Improve startup performance by deferring transaction creation
- **Solution:** Lazy transaction initialization inside worker loop (background init)
- **Result:** Faster node startup, but didn't address fundamental over-provisioning

**The architects understood the transaction pooling pattern was correct, they just implemented it suboptimally.**

---

## Proposed Implementation

### Option 1: Simple Fix (Immediate)
Cap thread count at actual CPU cores:

```rust
fn default_storage_worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())  // No *2 multiplier
        .unwrap_or(8)
}
```

### Option 2: Correct Fix (Recommended)
Replace thread pool with transaction pool:

1. Delete `storage_worker_loop` and pre-spawning logic
2. Create bounded channel: `crossbeam_channel::bounded(num_cores)`
3. Pre-fill with N transactions
4. Route all jobs through channel → transaction pool → Tokio's pool
5. Jobs return transactions when complete

### Option 3: Best Fix (Future)
Combine Option 2 with adaptive scaling based on observed page fault rates.

---

## Code References

- **Current config:** `crates/engine/primitives/src/config.rs:13-16`
- **Current worker spawning:** `crates/trie/parallel/src/proof_task.rs:343-360`
- **Current worker loop:** `crates/trie/parallel/src/proof_task.rs:191-310`
- **Old transaction pooling (deleted):** https://github.com/paradigmxyz/reth/blob/397a30def^/crates/trie/parallel/src/proof_task.rs#L77-L89

### Related PRs

- **PR #18887:** Introduced worker pooling (regression from transaction pooling)
- **PR #19012:** Background init of workers (performance improvement via lazy transaction init, still over-provisioned)
- **Pre-PR #18887:** Had correct transaction pooling pattern

---

## Related Discussion

- **X thread:** https://x.com/malik672_/status/1981282756190404829
- **Feedback from @alessandrod (Agave senior dev):** "Definitely makes sense to cap, and no reason to *2 for memory"
- **Page fault metrics (Mainnet 256GB machine):** 8.6M faults/sec during tip processing

---

## Acknowledgments

This analysis and solution proposed by community contributor. The transaction pooling pattern is documented in the pre-PR #18887 codebase and represents the correct architectural approach.
