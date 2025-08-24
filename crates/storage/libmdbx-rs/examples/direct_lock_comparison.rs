use reth_libmdbx::{Environment, WriteFlags};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;
use tempfile::tempdir;

fn main() {
    println!("Direct lock comparison test...");
    
    let dir = tempdir().unwrap();
    let env = Environment::builder().open(dir.path()).unwrap();
    
    // Setup test data
    {
        let rw_txn = env.begin_rw_txn().unwrap();
        let db = rw_txn.create_db(None, Default::default()).unwrap();
        
        for i in 0..10u32 {
            let key = format!("key{:04}", i);
            let value = i.to_le_bytes();
            rw_txn.put(db.dbi(), &key, &value, WriteFlags::empty()).unwrap();
        }
        
        rw_txn.commit().unwrap();
    }
    
    // Test: Many short concurrent transactions
    println!("Testing with many short-lived concurrent read transactions...");
    test_many_short_transactions(&env);
}

fn test_many_short_transactions(env: &Environment) {
    let num_threads = 4;
    let txns_per_thread = 100;
    let barrier = Arc::new(Barrier::new(num_threads));
    
    let start = Instant::now();
    let mut handles = vec![];
    
    for _thread_id in 0..num_threads {
        let env = env.clone();
        let barrier = Arc::clone(&barrier);
        
        let handle = thread::spawn(move || {
            barrier.wait(); // Synchronize start
            
            for i in 0..txns_per_thread {
                // Create a new transaction for each read (more realistic)
                let ro_txn = env.begin_ro_txn().unwrap();
                let db = ro_txn.open_db(None).unwrap();
                
                let key = format!("key{:04}", i % 10);
                let _: Option<Vec<u8>> = ro_txn.get(db.dbi(), key.as_bytes()).unwrap();
                
                // Transaction gets dropped here, triggering cleanup
            }
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start.elapsed();
    let total_txns = num_threads * txns_per_thread;
    
    println!("  {} short transactions: {:?}", total_txns, duration);
    println!("  Average per transaction: {:?}", duration / total_txns as u32);
    println!("  Transactions/second: {:.0}", total_txns as f64 / duration.as_secs_f64());
    
    // Expected: Lock-free should be significantly faster for transaction creation/cleanup
}