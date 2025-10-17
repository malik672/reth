use bytes::BufMut;
use reth_codecs::Compact;
use reth_trie_common::StoredNibbles;

// Force this to not be inlined so we can see the assembly
#[inline(never)]
pub fn optimized_to_compact(nibbles: &StoredNibbles, buf: &mut bytes::BytesMut) -> usize {
    nibbles.to_compact(buf)
}

fn main() {
    let nibbles = StoredNibbles::from_hex_str("0x12345678").unwrap();
    let mut buf = bytes::BytesMut::with_capacity(64);
    let len = optimized_to_compact(&nibbles, &mut buf);
    println!("Serialized {} nibbles", len);
}
