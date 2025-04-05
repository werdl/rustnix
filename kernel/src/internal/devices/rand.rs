#![allow(unused_imports)] // needed for .to_string() calls, not used directly and thus falsely detected as unused
use alloc::string::ToString;
use rand::RngCore;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::internal::clk::get_unix_time;
use crate::internal::file::{FileFlags, Stream};
use crate::internal::fs::FsError;

/// The random device
#[derive(Debug, Clone)]
pub struct Rand {
    inner: SmallRng,
    flags: u8,
}

impl Rand {
    /// Create a new random device
    pub fn new(flags: u8) -> Self {
        Rand {
            inner: SmallRng::seed_from_u64(get_unix_time()),
            flags,
        }
    }
}

impl Stream for Rand {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, crate::internal::file::FileError> {
        if !(self.flags & (FileFlags::Read as u8) != 0) {
            return Err(crate::internal::file::FileError::PermissionError(
                FsError::ReadError.into(),
            ));
        }
        self.inner.fill_bytes(buf);
        Ok(buf.len())
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, crate::internal::file::FileError> {
        if !(self.flags & (FileFlags::Write as u8) != 0) {
            return Err(crate::internal::file::FileError::PermissionError(
                FsError::WriteError.into(),
            ));
        }
        Err(crate::internal::file::FileError::WriteError(
            FsError::UnwritableFile.into(),
        ))
    }

    fn close(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn poll(&mut self, event: crate::internal::file::IOEvent) -> bool {
        match event {
            crate::internal::file::IOEvent::Read => self.flags & (FileFlags::Read as u8) != 0,
            crate::internal::file::IOEvent::Write => self.flags & (FileFlags::Read as u8) != 0,
        }
    }

    fn seek(&mut self, pos: usize) -> Result<usize, crate::internal::file::FileError> {
        // Seek is a no-op for the random device
        Ok(pos)
    }
}

impl Rand {
    /// Generate a random value of type T
    pub fn random<T>(&mut self) -> T
    where
        rand::distr::StandardUniform: rand::distr::Distribution<T>,
    {
        self.inner.random::<T>()
    }
}

/// Test the random device
#[test_case]
fn test_rand() {
    let mut rand = Rand::new(FileFlags::Read | FileFlags::Write);
    let mut buf = [0u8; 10];

    assert_eq!(rand.read(&mut buf).unwrap(), 10);

    // we'll just ignore this edge case (hypothetically, it could fail, but it's very unlikely (1 in 2^4096))
    assert_eq!(buf.iter().all(|&x| x == 0), false);
    assert_eq!(
        rand.write(&buf).unwrap_err(),
        crate::internal::file::FileError::WriteError(FsError::UnwritableFile.into())
    );
    assert_eq!(rand.flush().unwrap(), ());
    assert_eq!(rand.close().unwrap(), ());
}
