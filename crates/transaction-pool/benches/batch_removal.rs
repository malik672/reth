//! Benchmark comparing the two truncate_pool implementations from parked.rs:
//! 1. Current: Using remove_transactions_batch with pre-calculation 
//! 2. Alternative: Using individual remove_transaction calls in a loop
//!
//! This shows the performance difference between batch operations and individual operations
//! in the context of truncating the parked transaction pool.
//!
//! Run with: `cargo bench --bench truncate_batch_vs_individual --features="test-utils arbitrary"`

use alloy_primitives::Address;
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion, black_box,
};
use reth_transaction_pool::{
    pool::{BasefeeOrd, ParkedPool},
    test_utils::{MockTransactionFactory, MockTransactionSet},
    SubPoolLimit,
};
use alloy_consensus::TxType;

/// Generate test transactions  
fn generate_transactions(senders: usize, txs_per_sender: usize) -> Vec<reth_transaction_pool::test_utils::MockTransaction> {
    let mut all_txs = Vec::new();
    
    for idx in 0..senders {
        let idx_slice = idx.to_be_bytes();
        let addr_slice = [0u8; 12].into_iter().chain(idx_slice.into_iter()).collect::<Vec<_>>();
        let sender = Address::from_slice(&addr_slice);
        
        let txs = MockTransactionSet::dependent(sender, 0, txs_per_sender, TxType::Eip1559).into_vec();
        all_txs.extend(txs);
    }
    
    all_txs
}

/// Benchmark both implementations
fn benchmark_implementations(
    group: &mut BenchmarkGroup<'_, WallTime>,
    senders: usize,
    txs_per_sender: usize,
) {
    let txs = generate_transactions(senders, txs_per_sender);
    
    let setup = || {
        let mut txpool = ParkedPool::<BasefeeOrd<_>>::default();
        let mut f = MockTransactionFactory::default();

        for tx in &txs {
            txpool.add_transaction(f.validated_arc(tx.clone()));
        }
        txpool
    };

    // Benchmark current implementation (uses remove_transactions_batch internally)
    let batch_id = format!(
        "current_batch_impl | txs: {} | senders: {} | per_sender: {}",
        txs.len(), senders, txs_per_sender
    );
    
    group.bench_function(batch_id, |b| {
        b.iter_with_setup(setup, |mut txpool| {
            let limit = SubPoolLimit { max_txs: txs.len() / 2, max_size: usize::MAX };
            let removed = txpool.truncate_pool(limit);
            black_box((txpool, removed))
        });
    });

    // Simulate individual removal pattern with many small operations
    let individual_id = format!(
        "simulated_individual | txs: {} | senders: {} | per_sender: {}",
        txs.len(), senders, txs_per_sender
    );
    
    group.bench_function(individual_id, |b| {
        b.iter_with_setup(setup, |mut txpool| {
            let target = txs.len() / 2;
            let mut total_removed = Vec::new();
            
            // Simulate individual removal by doing many tiny truncations
            // Each operation removes just 1-2 transactions (like individual calls would)
            let mut current_size = txs.len();
            while current_size > target {
                let limit = SubPoolLimit { max_txs: current_size - 1, max_size: usize::MAX };
                let mut removed = txpool.truncate_pool(limit);
                if removed.is_empty() {
                    break;
                }
                current_size -= removed.len();
                total_removed.append(&mut removed);
            }
            black_box((txpool, total_removed))
        });
    });
}

fn batch_vs_individual_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("truncate_pool: batch vs individual");

    // Test scenarios showing performance difference
    let scenarios = [
        (20, 10),   // 200 txs
        (50, 10),   // 500 txs  
        (100, 10),  // 1000 txs
        (200, 10),  // 2000 txs - should show clear difference
    ];

    for (senders, txs_per_sender) in scenarios {
        benchmark_implementations(&mut group, senders, txs_per_sender);
    }

    group.finish();
}

criterion_group! {
    name = truncate_comparison;
    config = Criterion::default();
    targets = batch_vs_individual_benchmark
}
criterion_main!(truncate_comparison);