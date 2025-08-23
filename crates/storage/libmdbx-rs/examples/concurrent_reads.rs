use reth_libmdbx::{Environment, WriteFlags};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn main() {
    println!("Testing concurrent read performance...");
    
    // Setup test database
    let dir = tempdir().unwrap();
    let env = Environment::builder().open(dir.path()).unwrap();
    
    // Insert test data
    {
        let rw_txn = env.begin_rw_txn().unwrap();
        let db = rw_txn.create_db(None, Default::default()).unwrap();
        
        for i in 0..1000 {
            let key = format!("key{:06}", i);
            let value = format!("value{:06}", i);
            rw_txn.put(db.dbi(), &key, &value, WriteFlags::empty()).unwrap();
        }
        
        rw_txn.commit().unwrap();
    }
    
    // Test concurrent reads
    let num_threads = 4;
    let reads_per_thread = 10000;
    let barrier = Arc::new(Barrier::new(num_threads));
    
    let mut handles = vec![];
    
    let start = Instant::now();
    
    for thread_id in 0..num_threads {
        let env = env.clone();
        let barrier = Arc::clone(&barrier);
        
        let handle = thread::spawn(move || {
            // All threads start at the same time
            barrier.wait();
            
            let ro_txn = env.begin_ro_txn().unwrap();
            let db = ro_txn.open_db(None).unwrap();
            
            let thread_start = Instant::now();
            
            for i in 0..reads_per_thread {
                let key = format!("key{:06}", (i + thread_id * 100) % 1000);
                let _value: Option<Vec<u8>> = ro_txn.get(db.dbi(), key.as_bytes()).unwrap();
            }
            
            let thread_duration = thread_start.elapsed();
            println!("Thread {} completed {} reads in {:?}", 
                     thread_id, reads_per_thread, thread_duration);
            
            thread_duration
        });
        
        handles.push(handle);
    }
    
    let mut total_duration = Duration::new(0, 0);
    for handle in handles {
        total_duration += handle.join().unwrap();
    }
    
    let wall_clock_time = start.elapsed();
    let total_reads = num_threads * reads_per_thread;
    
    println!("\n=== Results ===");
    println!("Total reads: {}", total_reads);
    println!("Wall clock time: {:?}", wall_clock_time);
    println!("Combined thread time: {:?}", total_duration);
    println!("Reads/second (wall clock): {:.0}", total_reads as f64 / wall_clock_time.as_secs_f64());
    println!("Average latency: {:?}", wall_clock_time / total_reads as u32);
    
    // If our optimization is working, the wall clock time should be much less
    // than the combined thread time, showing true concurrency
    let concurrency_factor = total_duration.as_secs_f64() / wall_clock_time.as_secs_f64();
    println!("Concurrency factor: {:.2}x", concurrency_factor);
    
    if concurrency_factor > 2.0 {
        println!("✅ Lock-free optimization is working! True concurrent reads achieved.");
    } else {
        println!("⚠️  Reads may still be serialized. Check for lock contention.");
    }
}