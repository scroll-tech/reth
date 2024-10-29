use alloy_trie::Nibbles;

/// The maximum number of bits a key can contain.
const MAX_BITS: usize = 248;

// The maximum number of nibbles a key can contain.
const MAX_NIBBLES: usize = MAX_BITS / 4;

pub(crate) trait UnpackBits {
    /// This takes the `Nibbles` representation and converts it to a bit representation in which
    /// there is a byte for each bit in the nibble.
    ///
    /// We truncate the Nibbles such that we only have 248 bits.
    fn unpack_bits(&self) -> Self;

    // TODO: introduce unpack_bits_truncated method
}

impl UnpackBits for Nibbles {
    fn unpack_bits(&self) -> Self {
        let capacity = core::cmp::min(self.len() * 4, MAX_BITS);
        let mut bits = Vec::with_capacity(capacity);

        for byte in self.as_slice().iter().take(MAX_NIBBLES) {
            for i in (0..4).rev() {
                let bit = (byte >> i) & 1;
                bits.push(bit);
            }
        }
        Nibbles::from_vec_unchecked(bits)
    }
}
