use crate::{branch::BranchNodeRef, key::UnpackBits, sub_tree::SubTreeRef};
use alloy_primitives::{keccak256, map::HashMap, B256};
use alloy_trie::{
    hash_builder::{HashBuilderValue, HashBuilderValueRef},
    nodes::LeafNodeRef,
    proof::{ProofNodes, ProofRetainer},
    BranchNodeCompact, Nibbles, TrieMask, EMPTY_ROOT_HASH,
};
use core::cmp;
use tracing::trace;

#[derive(Debug, Default)]
#[allow(missing_docs)]
pub struct HashBuilder {
    pub key: Nibbles,
    pub value: HashBuilderValue,
    pub stack: Vec<B256>,

    pub groups: Vec<TrieMask>,
    pub tree_masks: Vec<TrieMask>,
    pub hash_masks: Vec<TrieMask>,

    pub stored_in_database: bool,

    pub updated_branch_nodes: Option<HashMap<Nibbles, BranchNodeCompact>>,
    pub proof_retainer: Option<ProofRetainer>,
}

impl HashBuilder {
    /// Enables the Hash Builder to store updated branch nodes.
    ///
    /// Call [HashBuilder::split] to get the updates to branch nodes.
    pub fn with_updates(mut self, retain_updates: bool) -> Self {
        self.set_updates(retain_updates);
        self
    }

    /// Enable specified proof retainer.
    pub fn with_proof_retainer(mut self, retainer: ProofRetainer) -> Self {
        self.proof_retainer = Some(retainer);
        self
    }

    /// Enables the Hash Builder to store updated branch nodes.
    ///
    /// Call [HashBuilder::split] to get the updates to branch nodes.
    pub fn set_updates(&mut self, retain_updates: bool) {
        if retain_updates {
            self.updated_branch_nodes = Some(HashMap::default());
        }
    }

    /// Splits the [HashBuilder] into a [HashBuilder] and hash builder updates.
    pub fn split(mut self) -> (Self, HashMap<Nibbles, BranchNodeCompact>) {
        let updates = self.updated_branch_nodes.take();
        (self, updates.unwrap_or_default())
    }

    /// Take and return retained proof nodes.
    pub fn take_proof_nodes(&mut self) -> ProofNodes {
        self.proof_retainer.take().map(ProofRetainer::into_proof_nodes).unwrap_or_default()
    }

    /// The number of total updates accrued.
    /// Returns `0` if [Self::with_updates] was not called.
    pub fn updates_len(&self) -> usize {
        self.updated_branch_nodes.as_ref().map(|u| u.len()).unwrap_or(0)
    }

    /// Print the current stack of the Hash Builder.
    pub fn print_stack(&self) {
        println!("============ STACK ===============");
        for item in &self.stack {
            println!("{}", alloy_primitives::hex::encode(item));
        }
        println!("============ END STACK ===============");
    }

    /// Adds a new leaf element and its value to the trie hash builder.
    pub fn add_leaf(&mut self, key: Nibbles, value: &[u8]) {
        let key = key.unpack_bits();
        assert!(key > self.key, "add_leaf key {:?} self.key {:?}", key, self.key);
        if !self.key.is_empty() {
            self.update(&key);
        }
        self.set_key_value(key, HashBuilderValueRef::Bytes(value));
    }

    /// Adds a new branch element and its hash to the trie hash builder.
    pub fn add_branch(&mut self, key: Nibbles, value: B256, stored_in_database: bool) {
        let key = key.unpack_bits();
        assert!(
            key > self.key || (self.key.is_empty() && key.is_empty()),
            "add_branch key {:?} self.key {:?}",
            key,
            self.key
        );
        if !self.key.is_empty() {
            self.update(&key);
        } else if key.is_empty() {
            self.stack.push(value);
        }
        self.set_key_value(key, HashBuilderValueRef::Hash(&value));
        self.stored_in_database = stored_in_database;
    }

    /// Returns the current root hash of the trie builder.
    pub fn root(&mut self) -> B256 {
        // Clears the internal state
        if !self.key.is_empty() {
            self.update(&Nibbles::default());
            self.key.clear();
            self.value.clear();
        }
        let root = self.current_root();
        if root == EMPTY_ROOT_HASH {
            if let Some(proof_retainer) = self.proof_retainer.as_mut() {
                proof_retainer.retain(&Nibbles::default(), &[])
            }
        }
        root
    }

    #[inline]
    fn set_key_value(&mut self, key: Nibbles, value: HashBuilderValueRef<'_>) {
        self.log_key_value("old value");
        self.key = key;
        self.value.set_from_ref(value);
        self.log_key_value("new value");
    }

    fn log_key_value(&self, msg: &str) {
        trace!(target: "trie::hash_builder",
            key = ?self.key,
            value = ?self.value,
            "{msg}",
        );
    }

    fn current_root(&self) -> B256 {
        if let Some(node_ref) = self.stack.last() {
            *node_ref
        } else {
            EMPTY_ROOT_HASH
        }
    }

    /// Given a new element, it appends it to the stack and proceeds to loop through the stack state
    /// and convert the nodes it can into branch / extension nodes and hash them. This ensures
    /// that the top of the stack always contains the merkle root corresponding to the trie
    /// built so far.
    fn update(&mut self, succeeding: &Nibbles) {
        let mut build_extensions = false;
        // current / self.key is always the latest added element in the trie
        let mut current = self.key.clone();
        debug_assert!(!current.is_empty());

        trace!(target: "trie::hash_builder", ?current, ?succeeding, "updating merkle tree");

        let mut i = 0usize;
        loop {
            let _span = tracing::trace_span!(target: "trie::hash_builder", "loop", i, ?current, build_extensions).entered();

            let preceding_exists = !self.groups.is_empty();
            let preceding_len = self.groups.len().saturating_sub(1);

            let common_prefix_len = succeeding.common_prefix_length(current.as_slice());
            let len = cmp::max(preceding_len, common_prefix_len);
            assert!(len < current.len(), "len {} current.len {}", len, current.len());

            trace!(
                target: "trie::hash_builder",
                ?len,
                ?common_prefix_len,
                ?preceding_len,
                preceding_exists,
                "prefix lengths after comparing keys"
            );

            // Adjust the state masks for branch calculation
            let extra_digit = current[len];
            if self.groups.len() <= len {
                let new_len = len + 1;
                trace!(target: "trie::hash_builder", new_len, old_len = self.groups.len(), "scaling state masks to fit");
                self.groups.resize(new_len, TrieMask::default());
            }
            self.groups[len] |= TrieMask::from_nibble(extra_digit);
            trace!(
                target: "trie::hash_builder",
                ?extra_digit,
                groups = ?self.groups,
            );

            // Adjust the tree masks for exporting to the DB
            if self.tree_masks.len() < current.len() {
                self.resize_masks(current.len());
            }

            let mut len_from = len;
            if !succeeding.is_empty() || preceding_exists {
                len_from += 1;
            }
            trace!(target: "trie::hash_builder", "skipping {len_from} nibbles");

            // The key without the common prefix
            let short_node_key = current.slice(len_from..);
            trace!(target: "trie::hash_builder", ?short_node_key);

            // Concatenate the 2 nodes together
            if !build_extensions {
                match self.value.as_ref() {
                    HashBuilderValueRef::Bytes(leaf_value) => {
                        let leaf_node = LeafNodeRef::new(&short_node_key, leaf_value);
                        // TODO: replace with appropriate account hashing
                        let leaf_hash = keccak256(leaf_value);
                        println!("leaf hash: {:?}", leaf_hash);
                        trace!(
                            target: "trie::hash_builder",
                            ?leaf_node,
                            ?leaf_hash,
                            "pushing leaf node",
                        );
                        self.stack.push(leaf_hash);
                        // self.retain_proof_from_stack(&current.slice(..len_from));
                    }
                    HashBuilderValueRef::Hash(hash) => {
                        trace!(target: "trie::hash_builder", ?hash, "pushing branch node hash");
                        self.stack.push(*hash);

                        if self.stored_in_database {
                            self.tree_masks[current.len() - 1] |=
                                TrieMask::from_nibble(current.last().unwrap());
                        }
                        self.hash_masks[current.len() - 1] |=
                            TrieMask::from_nibble(current.last().unwrap());

                        build_extensions = true;
                    }
                }
            }

            if build_extensions && !short_node_key.is_empty() {
                self.update_masks(&current, len_from);
                let stack_last = self.stack.pop().expect("there should be at least one stack item");
                let sub_tree = SubTreeRef::new(&short_node_key, &stack_last);
                let sub_tree_root = sub_tree.root();

                trace!(
                    target: "trie::hash_builder",
                    ?short_node_key,
                    ?sub_tree_root,
                    "pushing subtree root",
                );
                self.stack.push(sub_tree_root);
                // self.retain_proof_from_stack(&current.slice(..len_from));
                self.resize_masks(len_from);
            }

            if preceding_len <= common_prefix_len && !succeeding.is_empty() {
                trace!(target: "trie::hash_builder", "no common prefix to create branch nodes from, returning");
                return;
            }

            // Insert branch nodes in the stack
            if !succeeding.is_empty() || preceding_exists {
                // Pushes the corresponding branch node to the stack
                let children = self.push_branch_node(&current, len);
                println!("children: {:?}", children);
                // Need to store the branch node in an efficient format outside of the hash builder
                self.store_branch_node(&current, len, children);
            }

            self.groups.resize(len, TrieMask::default());
            self.resize_masks(len);

            if preceding_len == 0 {
                trace!(target: "trie::hash_builder", "0 or 1 state masks means we have no more elements to process");
                return;
            }

            current.truncate(preceding_len);
            trace!(target: "trie::hash_builder", ?current, "truncated nibbles to {} bytes", preceding_len);

            trace!(target: "trie::hash_builder", groups = ?self.groups, "popping empty state masks");
            while self.groups.last() == Some(&TrieMask::default()) {
                self.groups.pop();
            }

            build_extensions = true;

            i += 1;
        }
    }

    /// Given the size of the longest common prefix, it proceeds to create a branch node
    /// from the state mask and existing stack state, and store its RLP to the top of the stack,
    /// after popping all the relevant elements from the stack.
    ///
    /// Returns the hashes of the children of the branch node, only if `updated_branch_nodes` is
    /// enabled.
    fn push_branch_node(&mut self, _current: &Nibbles, len: usize) -> Vec<B256> {
        let state_mask = self.groups[len];
        let hash_mask = self.hash_masks[len];
        let branch_node = BranchNodeRef::new(&self.stack, state_mask);
        // Avoid calculating this value if it's not needed.
        let children = if self.updated_branch_nodes.is_some() {
            branch_node.child_hashes(hash_mask).collect()
        } else {
            vec![]
        };

        let branch_hash = branch_node.hash();
        println!("branch hash: {:?}", branch_hash);

        // TODO: enable proof retention
        // self.retain_proof_from_stack(&current.slice(..len));

        // Clears the stack from the branch node elements
        let first_child_idx = self.stack.len() - state_mask.count_ones() as usize;
        trace!(
            target: "trie::hash_builder",
            new_len = first_child_idx,
            old_len = self.stack.len(),
            "resizing stack to prepare branch node"
        );
        self.stack.resize_with(first_child_idx, Default::default);

        // trace!(target: "trie::hash_builder", ?rlp, "pushing branch node with {state_mask:?} mask
        // from stack");

        // TODO compute branch node hash
        self.stack.push(branch_hash);
        children
    }

    /// Given the current nibble prefix and the highest common prefix length, proceeds
    /// to update the masks for the next level and store the branch node and the
    /// masks in the database. We will use that when consuming the intermediate nodes
    /// from the database to efficiently build the trie.
    fn store_branch_node(&mut self, current: &Nibbles, len: usize, children: Vec<B256>) {
        trace!(target: "trie::hash_builder", ?current, ?len, ?children, "store branch node");
        if len > 0 {
            let parent_index = len - 1;
            self.hash_masks[parent_index] |= TrieMask::from_nibble(current[parent_index]);
        }

        let store_in_db_trie = !self.tree_masks[len].is_empty() || !self.hash_masks[len].is_empty();
        if store_in_db_trie {
            if len > 0 {
                let parent_index = len - 1;
                self.tree_masks[parent_index] |= TrieMask::from_nibble(current[parent_index]);
            }

            if self.updated_branch_nodes.is_some() {
                let common_prefix = current.slice(..len);
                let node = BranchNodeCompact::new(
                    self.groups[len],
                    self.tree_masks[len],
                    self.hash_masks[len],
                    children,
                    (len == 0).then(|| self.current_root()),
                );
                trace!(target: "trie::hash_builder", ?node, "intermediate node");
                self.updated_branch_nodes.as_mut().unwrap().insert(common_prefix, node);
            }
        }
    }

    // fn retain_proof_from_stack(&mut self, prefix: &Nibbles) {
    //     if let Some(proof_retainer) = self.proof_retainer.as_mut() {
    //         proof_retainer.retain(
    //             prefix,
    //             self.stack.last().expect("there should be at least one stack item").as_ref(),
    //         );
    //     }
    // }

    fn update_masks(&mut self, current: &Nibbles, len_from: usize) {
        if len_from > 0 {
            let flag = TrieMask::from_nibble(current[len_from - 1]);

            self.hash_masks[len_from - 1] &= !flag;

            if !self.tree_masks[current.len() - 1].is_empty() {
                self.tree_masks[len_from - 1] |= flag;
            }
        }
    }

    fn resize_masks(&mut self, new_len: usize) {
        trace!(
            target: "trie::hash_builder",
            new_len,
            old_tree_mask_len = self.tree_masks.len(),
            old_hash_mask_len = self.hash_masks.len(),
            "resizing tree/hash masks"
        );
        self.tree_masks.resize(new_len, TrieMask::default());
        self.hash_masks.resize(new_len, TrieMask::default());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::collections::BTreeMap;
    use hex_literal::hex;

    #[test]
    fn test_convert_to_bit_representation() {
        let nibbles = Nibbles::from_nibbles_unchecked(hex!("01020304")).unpack_bits();
        let expected = hex!("00000001000001000000010100010000");
        assert_eq!(nibbles.as_slice(), expected);
    }

    #[test]
    fn test_convert_to_bit_representation_truncation() {
        // 64 byte nibble
        let hex = hex!("0102030405060708090a0b0c0d0e0f0102030405060708090a0b0c0d0e0f0102030405060708090a0b0c0d0e0f0102030405060708090a0b0c0d0e0f01020304");
        assert_eq!(hex.len(), 64);
        let nibbles = Nibbles::from_nibbles_unchecked(hex).unpack_bits();
        assert_eq!(nibbles.len(), 248);
    }

    #[test]
    fn test_basic_trie() {
        // Test a basic trie consisting of three key value pairs:
        // - (0xF, 15)
        // - (0x0, 0)
        // - (0x1, 1)
        // The branch associated with key 0xF will be collapsed into a single leaf.

        let mut hb = HashBuilder::default().with_updates(true);
        let data = BTreeMap::from([
            // binary key: (0,0,0,0)
            (hex!("00").to_vec(), Vec::from([0u8])),
            // binary key: (0,0,0,1)
            (hex!("01").to_vec(), Vec::from([1u8])),
            // binary key: (1,1,1,1)
            (hex!("0F").to_vec(), Vec::from([15u8])),
        ]);
        data.iter().for_each(|(key, val)| {
            let nibbles = Nibbles::from_nibbles_unchecked(key);
            hb.add_leaf(nibbles, val.as_ref());
        });
        let root = hb.root();

        const EMPTY_NODE: [u8; 32] = [0u8; 32];
        let expected = {
            let leaf_0 = keccak256(data.get(hex!("00").as_slice()).unwrap());
            let leaf_1 = keccak256(data.get(hex!("01").as_slice()).unwrap());
            let leaf_f = keccak256(data.get(hex!("0F").as_slice()).unwrap());
            let node_000 = keccak256([leaf_0.as_slice(), leaf_1.as_slice()].concat());
            let node_00 = keccak256([node_000.as_slice(), &EMPTY_NODE].concat());
            let node_0 = keccak256([node_00.as_slice(), &EMPTY_NODE].concat());
            keccak256([node_0.as_slice(), leaf_f.as_slice()].concat())
        };

        assert_eq!(expected, root);
    }

    #[test]
    fn test_generates_branch_node() {
        let mut hb = HashBuilder::default().with_updates(true);
        let data = BTreeMap::from([
            // binary key: (0,0,0,0)
            (hex!("00").to_vec(), Vec::from([0u8])),
            // binary key: (0,0,0,1)
            (hex!("01").to_vec(), Vec::from([1u8])),
            // binary key: (0,0,1,0)
            (hex!("02").to_vec(), Vec::from([2u8])),
            // binary key: (1,1,1,1)
            (hex!("0F").to_vec(), Vec::from([15u8])),
        ]);
        data.iter().for_each(|(key, val)| {
            let nibbles = Nibbles::from_nibbles_unchecked(key);
            hb.add_leaf(nibbles, val.as_ref());
        });
        let root = hb.root();

        const EMPTY_NODE: [u8; 32] = [0u8; 32];
        let expected = {
            let leaf_0 = keccak256(data.get(hex!("00").as_slice()).unwrap());
            let leaf_1 = keccak256(data.get(hex!("01").as_slice()).unwrap());
            let leaf_2 = keccak256(data.get(hex!("02").as_slice()).unwrap());
            let leaf_f = keccak256(data.get(hex!("0F").as_slice()).unwrap());
            let node_000 = keccak256([leaf_0.as_slice(), leaf_1.as_slice()].concat());
            let node_00 = keccak256([node_000.as_slice(), leaf_2.as_slice()].concat());
            let node_0 = keccak256([node_00.as_slice(), &EMPTY_NODE].concat());
            keccak256([node_0.as_slice(), leaf_f.as_slice()].concat())
        };

        assert_eq!(root, expected);

        let (_, updates) = hb.split();
        for (key, update) in updates {
            // TODO add additional assertions
            println!("key: {:?}", key);
            println!("update: {:?}", update);
        }
    }
}
