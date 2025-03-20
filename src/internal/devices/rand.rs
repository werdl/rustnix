use alloc::string::ToString;
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use rand::RngCore;

use crate::internal::clk::get_unix_time;
use crate::internal::file::Stream;

#[derive(Debug)]
pub struct Rand {
    inner: SmallRng,
}

impl Rand {
    pub fn new() -> Self {
        Rand {
            inner: SmallRng::seed_from_u64(get_unix_time()),
        }
    }
}

impl Stream for Rand {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, crate::internal::file::FileError> {
        self.inner.fill_bytes(buf);
        Ok(buf.len())
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, crate::internal::file::FileError> {
        Err(crate::internal::file::FileError::WriteError("Cannot write to random device".to_string()))
    }

    fn close(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }
}

impl Rand {
    pub fn random<T>(&mut self) -> T
    where
        rand::distr::StandardUniform: rand::distr::Distribution<T>,
    {
        self.inner.random::<T>()
    }
}

#[test_case]
fn test_rand() {
    let mut rand = Rand::new();
    let mut buf = [0u8; 10];

    assert_eq!(rand.read(&mut buf).unwrap(), 10);

    // we'll just ignore this edge case (hypothetically, it could fail, but it's very unlikely (1 in 2^4096))
    assert_eq!(buf.iter().all(|&x| x == 0), false);
    assert_eq!(rand.write(&buf).unwrap_err().to_string(), "WriteError: Cannot write to random device");
    assert_eq!(rand.flush().unwrap(), ());
    assert_eq!(rand.close().unwrap(), ());
}
