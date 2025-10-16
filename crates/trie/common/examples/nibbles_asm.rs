// Example to examine assembly of to_compact
use bytes::{BufMut, BytesMut};
use reth_trie_common::Nibbles;

// Mimic the current implementation (loop version)
#[inline(never)]
pub fn to_compact_current(nibbles: &Nibbles, buf: &mut BytesMut) -> usize {
    // This is what StoredNibbles::to_compact currently does
    for i in nibbles.iter() {
        buf.put_u8(i);
    }
    nibbles.len()
}

// Proposed optimized implementation (slice version)
#[inline(never)]
pub fn to_compact_optimized(nibbles: &Nibbles, buf: &mut BytesMut) -> usize {
    // Proposed optimization
    let bytes: Vec<u8> = nibbles.iter().collect();
    buf.put_slice(&bytes);
    nibbles.len()
}

// Alternative optimization if Nibbles has as_slice or similar
#[inline(never)]
pub fn to_compact_optimized_v2(nibbles: &Nibbles, buf: &mut BytesMut) -> usize {
    // If we can get a slice directly (checking if to_vec exists)
    buf.put_slice(&nibbles.to_vec());
    nibbles.len()
}

fn benchmark_size(size: usize, iterations: usize) {
    let data: Vec<u8> = (0..size).map(|i| (i % 16) as u8).collect();
    let nibbles = Nibbles::from_nibbles_unchecked(data);
    let mut buf1 = BytesMut::with_capacity(size * 2);
    let mut buf2 = BytesMut::with_capacity(size * 2);
    let mut buf3 = BytesMut::with_capacity(size * 2);

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        buf1.clear();
        let len = to_compact_current(&nibbles, &mut buf1);
        std::hint::black_box(len);
    }
    let current_time = start.elapsed();

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        buf2.clear();
        let len = to_compact_optimized(&nibbles, &mut buf2);
        std::hint::black_box(len);
    }
    let optimized_time = start.elapsed();

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        buf3.clear();
        let len = to_compact_optimized_v2(&nibbles, &mut buf3);
        std::hint::black_box(len);
    }
    let optimized_v2_time = start.elapsed();

    // Verify all produce same output
    buf1.clear();
    buf2.clear();
    buf3.clear();
    to_compact_current(&nibbles, &mut buf1);
    to_compact_optimized(&nibbles, &mut buf2);
    to_compact_optimized_v2(&nibbles, &mut buf3);

    assert_eq!(buf1, buf2);
    assert_eq!(buf1, buf3);

    println!("Nibbles length: {}", nibbles.len());
    println!("Iterations: {}", iterations);
    println!();
    println!("Current (loop):        {:>10.2?}  ({:.2} ns/iter)", current_time, current_time.as_nanos() as f64 / iterations as f64);
    println!("Optimized (collect):   {:>10.2?}  ({:.2} ns/iter)", optimized_time, optimized_time.as_nanos() as f64 / iterations as f64);
    println!("Optimized (to_vec):    {:>10.2?}  ({:.2} ns/iter)", optimized_v2_time, optimized_v2_time.as_nanos() as f64 / iterations as f64);
    println!();
    let collect_speedup = current_time.as_secs_f64() / optimized_time.as_secs_f64();
    let tovec_speedup = current_time.as_secs_f64() / optimized_v2_time.as_secs_f64();
    if collect_speedup > 1.0 {
        println!("Speedup (collect):     {:.2}x faster", collect_speedup);
    } else {
        println!("Speedup (collect):     {:.2}x SLOWER", 1.0 / collect_speedup);
    }
    if tovec_speedup > 1.0 {
        println!("Speedup (to_vec):      {:.2}x faster", tovec_speedup);
    } else {
        println!("Speedup (to_vec):      {:.2}x SLOWER", 1.0 / tovec_speedup);
    }
}

fn main() {
    println!("=== Nibbles to_compact Performance Test ===\n");

    println!("--- Small (8 bytes) ---");
    benchmark_size(8, 100_000);

    println!("\n--- Medium (32 bytes) ---");
    benchmark_size(32, 100_000);

    println!("\n--- Large (64 bytes) ---");
    benchmark_size(64, 100_000);

    println!("\n--- Very Large (256 bytes) ---");
    benchmark_size(256, 10_000);
}
