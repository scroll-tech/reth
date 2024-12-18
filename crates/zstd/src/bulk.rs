use std::io;

pub struct Compressor<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl Compressor<'_> {
    pub fn with_dictionary(_level: i32, _dictionary: &[u8]) -> io::Result<Self> {
        unimplemented!("zstd not available")
    }

    pub fn compress(&mut self, _data: &[u8]) -> io::Result<Vec<u8>> {
        unimplemented!("zstd not available")
    }

    pub fn compress_to_buffer<C:?Sized>(
        &mut self,
        _source: &[u8],
        _destination: &mut C,
    ) -> io::Result<usize> {
        unimplemented!("zstd not available")
    }
}

pub struct Decompressor<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Decompressor<'a> {
    pub fn upper_bound(_data: &[u8]) -> Option<usize> {
        unimplemented!("zstd not available")
    }

    pub fn with_dictionary(_dictionary: &[u8]) -> io::Result<Self> {
        unimplemented!("zstd not available")
    }

    pub fn with_prepared_dictionary<'b>(
        _dictionary: &'a crate::dict::DecoderDictionary<'b>,
    ) -> io::Result<Self>
    where
        'b: 'a,
    {
        unimplemented!("zstd not available")
    }

    pub fn decompress_to_buffer<C: ?Sized>(
        &mut self,
        _source: &[u8],
        _destination: &mut C,
    ) -> io::Result<usize>  {
        unimplemented!("zstd not available")
    }
}