use reth_trie_common::{prefix_set::PrefixSetMut, Nibbles};

#[inline(never)]
pub fn contains_realworld(prefix_set: &mut reth_trie_common::prefix_set::PrefixSet, prefix: &Nibbles) -> bool {
    prefix_set.contains(prefix)
}

fn main() {
    let mut prefix_set_mut = PrefixSetMut::with_capacity(1000);

    // Simulate real-world trie keys: account hashes converted to nibbles
    // These represent 32-byte keccak256 hashes unpacked to 64 nibbles
    // Using realistic distribution of ethereum account addresses

    // Common prefixes (many accounts start with 0x0, 0x1, etc.)
    for i in 0..200 {
        let mut nibbles = vec![0, 0];
        nibbles.extend((i as u64).to_be_bytes().iter().flat_map(|b| [b >> 4, b & 0xF]));
        // Pad to 64 nibbles (32 bytes unpacked)
        nibbles.resize(64, 0);
        prefix_set_mut.insert(Nibbles::from_nibbles_unchecked(nibbles));
    }

    // Scattered addresses across the keyspace
    for i in 0..300 {
        let mut nibbles = Vec::with_capacity(64);
        let hash = (i * 0x123456789abcdef_u64).to_be_bytes();
        nibbles.extend(hash.iter().flat_map(|b| [b >> 4, b & 0xF]));
        nibbles.resize(64, ((i % 16) as u8));
        prefix_set_mut.insert(Nibbles::from_nibbles_unchecked(nibbles));
    }

    // Storage keys (varied lengths, common in state tries)
    for i in 0..500 {
        let len = 10 + (i % 54); // Variable length keys
        let mut nibbles = Vec::with_capacity(len);
        let base = (i as u64 * 0xdeadbeef).to_be_bytes();
        nibbles.extend(base.iter().flat_map(|b| [b >> 4, b & 0xF]));
        if nibbles.len() > len {
            nibbles.truncate(len);
        }
        prefix_set_mut.insert(Nibbles::from_nibbles_unchecked(nibbles));
    }

    let mut prefix_set = prefix_set_mut.freeze();

    // Test lookups with realistic prefixes
    let test_prefix1 = Nibbles::from_nibbles_unchecked(vec![0, 0, 1, 2]);
    let test_prefix2 = Nibbles::from_nibbles_unchecked(vec![0xd, 0xe, 0xa, 0xd]);
    let test_prefix3 = Nibbles::from_nibbles_unchecked(vec![0, 0, 0, 0, 0, 0, 0, 0]);

    let result1 = contains_realworld(&mut prefix_set, &test_prefix1);
    let result2 = contains_realworld(&mut prefix_set, &test_prefix2);
    let result3 = contains_realworld(&mut prefix_set, &test_prefix3);

    println!("Contains [0,0,1,2]: {}", result1);
    println!("Contains [d,e,a,d]: {}", result2);
    println!("Contains [0x8]: {}", result3);
}

