use alloy_primitives::{hex, keccak256, B256};
use alloy_trie::Nibbles;
use core::fmt;

/// Reference to the extension node. See [ExtensionNode] from more information.
pub struct ExtensionNodeRef<'a> {
    /// The key for this extension node.
    pub key: &'a Nibbles,
    /// A pointer to the child node.
    pub child: &'a B256,
}

impl<'a> ExtensionNodeRef<'a> {
    /// Creates a new extension node with the given key and a pointer to the child.
    #[inline]
    pub const fn new(key: &'a Nibbles, child: &'a B256) -> Self {
        Self { key, child }
    }

    pub fn hash(&self) -> B256 {
        let mut bytes = Vec::with_capacity(self.key.len() + 32);
        bytes.extend_from_slice(&self.key.as_slice());
        bytes.extend_from_slice(self.child.as_slice());
        keccak256(bytes)
    }
}

impl fmt::Debug for ExtensionNodeRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtensionNodeRef")
            .field("key", &self.key)
            .field("node", &hex::encode(self.child))
            .finish()
    }
}
