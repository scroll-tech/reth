use alloy_primitives::{hex, keccak256, B256};
use alloy_trie::Nibbles;
use core::fmt;

/// Reference to a subtree containing a single child.
pub(crate) struct SubTreeRef<'a> {
    /// The key to the child node.
    pub key: &'a Nibbles,
    /// A pointer to the child node.
    pub child: &'a B256,
}

impl<'a> SubTreeRef<'a> {
    /// Creates a new subtree with the given key and a pointer to the child.
    #[inline]
    pub(crate) const fn new(key: &'a Nibbles, child: &'a B256) -> Self {
        Self { key, child }
    }

    pub(crate) fn root(&self) -> B256 {
        let mut tree_root = *self.child;
        for &bit in self.key.as_slice().iter().rev() {
            let mut bytes = [0u8; 64];
            if bit == 0 {
                bytes[..32].copy_from_slice(tree_root.as_slice());
            } else {
                bytes[32..].copy_from_slice(tree_root.as_slice());
            }
            tree_root = keccak256(&bytes);
        }
        tree_root
    }
}

impl fmt::Debug for SubTreeRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubTreeRef")
            .field("key", &self.key)
            .field("node", &hex::encode(self.child))
            .finish()
    }
}
