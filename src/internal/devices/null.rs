use crate::internal::{devices::zero, file::Stream};

#[derive(Debug)]
pub struct Null {
    inner: zero::Zero,
}

impl Null {
    pub fn new() -> Self {
        Null {
            inner: zero::Zero::new(),
        }
    }
}

impl Stream for Null {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, crate::internal::file::FileError> {
        // EOF
        Ok(0)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, crate::internal::file::FileError> {
        self.inner.write(buf)
    }

    fn close(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }
}

#[test_case]
fn test_null() {
    let mut null = Null::new();
    let mut buf = [0u8; 10];

    assert_eq!(null.read(&mut buf).unwrap(), 0);
    assert_eq!(null.write(&buf).unwrap(), 10);
    assert_eq!(null.flush().unwrap(), ());
    assert_eq!(null.close().unwrap(), ());
}
