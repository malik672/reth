use reth_trie_common::{prefix_set::PrefixSetMut, Nibbles};

#[inline(never)]
pub fn contains_simple(prefix_set: &mut reth_trie_common::prefix_set::PrefixSet, prefix: &Nibbles) -> bool {
    prefix_set.contains(prefix)
}

fn main() {
    let mut prefix_set_mut = PrefixSetMut::default();

    // Simple test data - small dataset
    prefix_set_mut.insert(Nibbles::from_nibbles([1, 2, 3, 4]));
    prefix_set_mut.insert(Nibbles::from_nibbles([1, 2, 5, 6]));
    prefix_set_mut.insert(Nibbles::from_nibbles([2, 3, 4, 5]));
    prefix_set_mut.insert(Nibbles::from_nibbles([3, 4, 5, 6]));
    prefix_set_mut.insert(Nibbles::from_nibbles([5, 6, 7, 8]));

    let mut prefix_set = prefix_set_mut.freeze();

    // Test lookups
    let test_prefix = Nibbles::from_nibbles([1, 2]);
    let result = contains_simple(&mut prefix_set, &test_prefix);

    println!("Contains [1, 2]: {}", result);
}
