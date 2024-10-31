#![allow(missing_docs)]
use std::time::Duration;

use alloy_primitives::B256;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use proptest::{prelude::*, strategy::ValueTree, test_runner::TestRunner};
use reth_trie::{
    trie_cursor::{subnode, subnode_opt, CursorSubNode},
    BranchNodeCompact, Nibbles, TrieMask,
};


fn generate_test_data(size: usize) -> Vec<(Nibbles, Option<BranchNodeCompact>)> {
    let mut runner = TestRunner::new(ProptestConfig::with_cases(1000));
    
   
    let nibbles_strategy = prop::collection::vec(0u8..16, 1..32)
        .prop_map(|v| Nibbles::from_nibbles_unchecked(v));
    
    
    let branch_node_strategy = (
        any::<u16>(), 
        any::<u16>(), 
        any::<u16>(), 
        any::<bool>(), 
    ).prop_map(|(mut state, mut tree, mut hash, has_root)| {
        let mut node = BranchNodeCompact::default();
        
        state |= 1;
        tree |= 1;
        hash |= 1;
        
        node.state_mask = TrieMask::new(state);
        node.tree_mask = TrieMask::new(tree);
        node.hash_mask = TrieMask::new(hash);
        
        if has_root {
            node.root_hash = Some(B256::random());
        }
        Some(node)
    });

    (0..size)
        .map(|_| {
            let nibbles = nibbles_strategy.new_tree(&mut runner).unwrap().current();
            let node = branch_node_strategy.new_tree(&mut runner).unwrap().current();
            (nibbles, node)
        })
        .collect()
}

pub fn bench_cursor_subnode(c: &mut Criterion) {
    let mut group = c.benchmark_group("CursorSubNode");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);
    group.warm_up_time(Duration::from_secs(2));


    for size in [10, 100, 1000].iter() {
        let test_data = generate_test_data(*size);
        
        group.bench_function(BenchmarkId::new("v1/creation", size), |b| {
            b.iter_batched(
                || test_data.clone(),
                |data| {
                    for (key, node) in data {
                        black_box(subnode::CursorSubNode::new(key, node));
                    }
                },
                criterion::BatchSize::LargeInput,
            )
        });

        group.bench_function(BenchmarkId::new("v2/creation", size), |b| {
            b.iter_batched(
                || test_data.clone(),
                |data| {
                    for (key, node) in data {
                        black_box(subnode_opt::CursorSubNode::new(key, node));
                    }
                },
                criterion::BatchSize::LargeInput,
            )
        });

 
        let v1_cursors: Vec<_> = test_data
            .iter()
            .map(|(k, n)| subnode::CursorSubNode::new(k.clone(), n.clone()))
            .collect();
        let v2_cursors: Vec<_> = test_data
            .iter()
            .map(|(k, n)| subnode_opt::CursorSubNode::new(k.clone(), n.clone()))
            .collect();

       
        group.bench_function(BenchmarkId::new("v1/flags", size), |b| {
            b.iter(|| {
                for cursor in &v1_cursors {
                    black_box(cursor.state_flag());
                    black_box(cursor.tree_flag());
                    black_box(cursor.hash_flag());
                }
            })
        });

        group.bench_function(BenchmarkId::new("v2/flags", size), |b| {
            b.iter(|| {
                for cursor in &v2_cursors {
                    black_box(cursor.state_flag());
                    black_box(cursor.tree_flag());
                    black_box(cursor.hash_flag());
                }
            })
        });

        group.bench_function(BenchmarkId::new("v1/full_key_access", size), |b| {
            b.iter(|| {
                for cursor in &v1_cursors {
                    black_box(cursor.full_key());
                }
            })
        });

        group.bench_function(BenchmarkId::new("v2/full_key_access", size), |b| {
            b.iter(|| {
                for cursor in &v2_cursors {
                    black_box(cursor.full_key());
                }
            })
        });
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .significance_level(0.01)
        .noise_threshold(0.02);
    targets = bench_cursor_subnode
);
criterion_main!(benches);