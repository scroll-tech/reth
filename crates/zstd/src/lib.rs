use std::io;
use std::io::{Read, Write};

pub mod bulk;
pub mod dict;

pub struct Decoder<'a, R> {
    _phantom1: std::marker::PhantomData<&'a ()>,
    _phantom2: std::marker::PhantomData<R>,
}

pub struct Encoder<'a, R> {
    _phantom1: std::marker::PhantomData<&'a ()>,
    _phantom2: std::marker::PhantomData<R>,
}

impl<R> Decoder<'_, R> {
    pub fn new(_reader: R) -> io::Result<Self> {
        unimplemented!("zstd not available")
    }

    pub fn with_dictionary(_reader: R, _dictionary: &[u8]) -> io::Result<Self> {
        unimplemented!("zstd not available")
    }
}

impl<R> Read for Decoder<'_, R> {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        unimplemented!("zstd not available")
    }
}


impl<W> Encoder<'_, W> {
    pub fn new(_writer: W, _level: i32) -> io::Result<Self> {
        unimplemented!("zstd not available")
    }

    pub fn finish(self) -> io::Result<W>  {
        unimplemented!("zstd not available")
    }
}

impl<'a, W> Write for Encoder<'a, W> {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        unimplemented!("zstd not available")
    }

    fn flush(&mut self) -> io::Result<()> {
        unimplemented!("zstd not available")
    }
}