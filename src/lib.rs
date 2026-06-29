use core::slice;
use std::io::{self, Read, Write};

pub struct AsyncReadBuf {
    buf: Vec<u8>,
    capacity: usize,
}

impl AsyncReadBuf {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buf[..self.capacity]
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

pub trait AsyncRead
where
    Self: Read,
{
    fn read_async(&mut self, dst: &mut AsyncReadBuf) -> io::Result<bool> {
        let len = dst.buf.len();
        let capacity = dst.capacity;
        let buf_slice = unsafe {
            let ptr = dst.buf.as_mut_ptr();
            slice::from_raw_parts_mut(ptr.add(len), capacity - len)
        };

        match self.read(buf_slice) {
            Ok(read_size) => {
                unsafe { dst.buf.set_len(len + read_size) };
            }
            Err(err)
                if !matches!(
                    err.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                return Err(err.into());
            }
            _ => (),
        }

        if dst.buf.len() == dst.capacity {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub struct AsyncWriteBuf {
    buf: Vec<u8>,
    pos: usize,
    capacity: usize,
}

impl AsyncWriteBuf {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            pos: 0,
            capacity: 0,
        }
    }

    pub fn set_data(&mut self, data: &[u8]) {
        self.buf = data.into();
        self.pos = 0;
        self.capacity = data.len();
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buf[..self.capacity]
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl From<&[u8]> for AsyncWriteBuf {
    fn from(value: &[u8]) -> Self {
        let mut buf = Self::new();
        buf.set_data(&value);
        buf
    }
}

pub trait AsyncWrite
where
    Self: Write,
{
    fn write_async(&mut self, src: &mut AsyncWriteBuf) -> io::Result<bool> {
        let pos = src.pos;
        let capacity = src.capacity;
        let buf_slice = unsafe {
            let ptr = src.buf.as_ptr();
            slice::from_raw_parts(ptr.add(pos), capacity - pos)
        };

        match self.write(buf_slice) {
            Ok(size) => {
                src.pos += size;
            }
            Err(err)
                if !matches!(
                    err.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                return Err(err.into());
            }
            _ => (),
        }

        if src.pos == src.capacity {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl<T: Read> AsyncRead for T {}
impl<T: Write> AsyncWrite for T {}