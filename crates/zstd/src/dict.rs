pub struct DecoderDictionary<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl DecoderDictionary<'static> {
    pub fn copy(_dictionary: &[u8]) -> Self {
        unimplemented!("zstd not available")
    }
}