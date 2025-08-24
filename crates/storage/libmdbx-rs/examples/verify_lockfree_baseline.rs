use reth_libmdbx::{Environment, WriteFlags};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;
use tempfile::tempdir;

fn main() {
    println!("Baseline performance test (original implementation with locks)...");
    
    let dir = tempdir().unwrap();
    let env = Environment::builder().open(dir.path()).unwrap();
    
    // Setup test data
    {
        let rw_txn = env.begin_rw_txn().unwrap();
        let db = rw_txn.create_db(None, Default::default()).unwrap();
        
        for i in 0..100u32 {
            let key = format!("key{:04}", i);
            let value = i.to_le_bytes();
            rw_txn.put(db.dbi(), &key, &value, WriteFlags::empty()).unwrap();
        }
        
        rw_txn.commit().unwrap();
    }
    
    // Test 1: Single-threaded read (baseline)
    println!("Test 1: Single-threaded reads (baseline)...");
    test_single_threaded(&env);
    
    // Test 2: Multi-threaded concurrent reads (shows lock contention)
    println!("Test 2: Multi-threaded concurrent reads (with lock contention)...");
    test_concurrent_reads(&env);
}

fn test_single_threaded(env: &Environment) {
    let start = Instant::now();
    
    let ro_txn = env.begin_ro_txn().unwrap();
    let db = ro_txn.open_db(None).unwrap();
    
    for i in 0..1000 {
        let key = format!("key{:04}", i % 100);
        let _: Option<Vec<u8>> = ro_txn.get(db.dbi(), key.as_bytes()).unwrap();
    }
    
    let duration = start.elapsed();
    println!("  1000 single-threaded reads: {:?}", duration);
    println!("  Average per read: {:?}", duration / 1000);
}

fn test_concurrent_reads(env: &Environment) {
    let num_threads = 4;
    let reads_per_thread = 250;
    let barrier = Arc::new(Barrier::new(num_threads));
    
    let start = Instant::now();
    let mut handles = vec![];
    
    for _thread_id in 0..num_threads {
        let env = env.clone();
        let barrier = Arc::clone(&barrier);
        
        let handle = thread::spawn(move || {
            barrier.wait(); // Synchronize start
            
            let ro_txn = env.begin_ro_txn().unwrap();
            let db = ro_txn.open_db(None).unwrap();
            
            for i in 0..reads_per_thread {
                let key = format!("key{:04}", i % 100);
                let _: Option<Vec<u8>> = ro_txn.get(db.dbi(), key.as_bytes()).unwrap();
            }
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start.elapsed();
    let total_reads = num_threads * reads_per_thread;
    
    println!("  {} concurrent reads: {:?}", total_reads, duration);
    println!("  Average per read: {:?}", duration / total_reads as u32);
    println!("  Reads/second: {:.0}", total_reads as f64 / duration.as_secs_f64());
    
    // With original implementation, concurrent reads should show mutex serialization
    let theoretical_serial_time = duration * num_threads as u32;
    let speedup = theoretical_serial_time.as_secs_f64() / duration.as_secs_f64();
    println!("  Effective speedup: {:.1}x", speedup);
    
    if speedup > 2.0 {
        println!("  ⚠️  Unexpected - reads appear to be concurrent (expected serialization)");
    } else {
        println!("  ✅ Expected behavior - reads are serialized due to mutex contention");
    }
}