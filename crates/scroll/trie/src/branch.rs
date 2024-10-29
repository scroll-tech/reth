use alloy_primitives::{hex, keccak256, B256};
use alloy_trie::TrieMask;
use core::{fmt, ops::Range, slice::Iter};

// #[allow(unused_imports)]
// use alloc::vec::Vec;

/// The range of valid child indexes.
pub(crate) const CHILD_INDEX_RANGE: Range<u8> = 0..2;

/// A reference to [BranchNode] and its state mask.
/// NOTE: The stack may contain more items that specified in the state mask.
#[derive(Clone)]
pub(crate) struct BranchNodeRef<'a> {
    /// Reference to the collection of RLP encoded nodes.
    /// NOTE: The referenced stack might have more items than the number of children
    /// for this node. We should only ever access items starting from
    /// [BranchNodeRef::first_child_index].
    pub stack: &'a [B256],
    /// Reference to bitmask indicating the presence of children at
    /// the respective nibble positions.
    pub state_mask: TrieMask,
}

impl fmt::Debug for BranchNodeRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BranchNodeRef")
            .field("stack", &self.stack.iter().map(hex::encode).collect::<Vec<_>>())
            .field("state_mask", &self.state_mask)
            .field("first_child_index", &self.first_child_index())
            .finish()
    }
}

impl<'a> BranchNodeRef<'a> {
    /// Create a new branch node from the stack of nodes.
    #[inline]
    pub(crate) const fn new(stack: &'a [B256], state_mask: TrieMask) -> Self {
        Self { stack, state_mask }
    }

    /// Returns the stack index of the first child for this node.
    ///
    /// # Panics
    ///
    /// If the stack length is less than number of children specified in state mask.
    /// Means that the node is in inconsistent state.
    #[inline]
    pub(crate) fn first_child_index(&self) -> usize {
        self.stack.len().checked_sub(self.state_mask.count_ones() as usize).unwrap()
    }

    #[inline]
    fn children(&self) -> impl Iterator<Item = (u8, Option<&B256>)> + '_ {
        BranchChildrenIter::new(self)
    }

    /// Given the hash mask of children, return an iterator over stack items
    /// that match the mask.
    #[inline]
    pub(crate) fn child_hashes(&self, hash_mask: TrieMask) -> impl Iterator<Item = B256> + '_ {
        self.children()
            .filter_map(|(i, c)| c.map(|c| (i, c)))
            .filter(move |(index, _)| hash_mask.is_bit_set(*index))
            .map(|(_, child)| B256::from_slice(&child[..]))
    }

    pub(crate) fn hash(&self) -> B256 {
        let mut children_iter = self.children();
        let mut bytes: [u8; 64] = [0u8; 64];

        if let Some((_, Some(child))) = children_iter.next() {
            bytes[..32].copy_from_slice(child.as_slice());
        }

        if let Some((_, Some(child))) = children_iter.next() {
            bytes[32..].copy_from_slice(child.as_slice());
        }

        keccak256(bytes.as_slice())
    }
}

/// Iterator over branch node children.
#[derive(Debug)]
struct BranchChildrenIter<'a> {
    range: Range<u8>,
    state_mask: TrieMask,
    stack_iter: Iter<'a, B256>,
}

impl<'a> BranchChildrenIter<'a> {
    /// Create new iterator over branch node children.
    fn new(node: &BranchNodeRef<'a>) -> Self {
        Self {
            range: CHILD_INDEX_RANGE,
            state_mask: node.state_mask,
            stack_iter: node.stack[node.first_child_index()..].iter(),
        }
    }
}

impl<'a> Iterator for BranchChildrenIter<'a> {
    type Item = (u8, Option<&'a B256>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let i = self.range.next()?;
        let value = if self.state_mask.is_bit_set(i) {
            // SAFETY: `first_child_index` guarantees that `stack` is exactly
            // `state_mask.count_ones()` long.
            Some(unsafe { self.stack_iter.next().unwrap_unchecked() })
        } else {
            None
        };
        Some((i, value))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl core::iter::FusedIterator for BranchChildrenIter<'_> {}

impl ExactSizeIterator for BranchChildrenIter<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.range.len()
    }
}
