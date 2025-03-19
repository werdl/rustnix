use alloc::string::ToString;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rand::RngCore;

use crate::internal::clk::get_unix_time;
use crate::internal::file::Stream;


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

    fn close(&mut self, _path: Option<&str>) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }
}
