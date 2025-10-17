// Production assembly test for StoredNibbles::to_compact
use bytes::BytesMut;
use reth_codecs::Compact;
use reth_trie_common::{Nibbles, StoredNibbles};

#[inline(never)]
pub fn production_to_compact(nibbles: &StoredNibbles, buf: &mut BytesMut) -> usize {
    nibbles.to_compact(buf)
}

fn main() {
    let nibbles_data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let nibbles = Nibbles::from_nibbles_unchecked(nibbles_data);
    let stored = StoredNibbles::from(nibbles);

    let mut buf = BytesMut::with_capacity(64);

    // Benchmark
    let iterations = 1_000_000;
    let start = std::time::Instant::now();

    for _ in 0..iterations {
        buf.clear();
        let len = production_to_compact(&stored, &mut buf);
        std::hint::black_box(len);
    }

    let elapsed = start.elapsed();

    println!("Iterations: {}", iterations);
    println!("Total time: {:?}", elapsed);
    println!("Time per iteration: {:.2} ns", elapsed.as_nanos() as f64 / iterations as f64);
    println!("Output length: {} bytes", buf.len());
}
