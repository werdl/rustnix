use crate::internal::{
    devices::zero,
    file::{FileFlags, Stream},
    fs::FsError,
};

/// Null device
#[derive(Debug)]
pub struct Null {
    inner: zero::Zero,
}

impl Null {
    /// Create a new null device
    pub fn new(flags: u8) -> Self {
        Null {
            inner: zero::Zero::new(flags),
        }
    }
}

impl Stream for Null {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, crate::internal::file::FileError> {
        if !(self.inner.flags & (FileFlags::Read as u8) != 0) {
            return Err(crate::internal::file::FileError::PermissionError(
                FsError::ReadError.into(),
            ));
        }
        Ok(0)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, crate::internal::file::FileError> {
        if !(self.inner.flags & (FileFlags::Write as u8) != 0) {
            return Err(crate::internal::file::FileError::PermissionError(
                FsError::WriteError.into(),
            ));
        }
        self.inner.write(buf)
    }

    fn close(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn poll(&mut self, event: crate::internal::file::IOEvent) -> bool {
        match event {
            crate::internal::file::IOEvent::Read => self.inner.poll(event),
            crate::internal::file::IOEvent::Write => self.inner.poll(event),
        }
    }
}

/// Test the null device
#[test_case]
fn test_null() {
    let mut null = Null::new(FileFlags::Read | FileFlags::Write);
    let mut buf = [0u8; 10];

    assert_eq!(null.read(&mut buf).unwrap(), 0);
    assert_eq!(null.write(&buf).unwrap(), 10);
    assert_eq!(null.flush().unwrap(), ());
    assert_eq!(null.close().unwrap(), ());
}
