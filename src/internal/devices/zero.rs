use alloc::string::ToString;

use crate::internal::file::{FileFlags, Stream};

#[derive(Debug)]
pub struct Zero {
    pub flags: u8,
}

impl Stream for Zero {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, crate::internal::file::FileError> {
        if !(self.flags & (FileFlags::Read as u8) != 0) {
            return Err(crate::internal::file::FileError::PermissionError("No permission to read".to_string()));
        }
        // fill buf with 0s
        for i in 0..buf.len() {
            buf[i] = 0;
        }

        Ok(buf.len())
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, crate::internal::file::FileError> {
        if !(self.flags & (FileFlags::Write as u8) != 0) {
            return Err(crate::internal::file::FileError::PermissionError("No permission to write".to_string()));
        }
        Ok(buf.len()) // writing to /dev/zero is always successful
    }

    fn close(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn poll(&mut self, event: crate::internal::file::IOEvent) -> bool {
        match event {
            crate::internal::file::IOEvent::Read => !(self.flags & (FileFlags::Read as u8) != 0),
            crate::internal::file::IOEvent::Write => !(self.flags & (FileFlags::Write as u8) != 0),
        }
    }
}

impl Zero {
    pub fn new(flags: u8) -> Self {
        Zero {
            flags,
        }
    }
}

#[test_case]
fn test_zero() {
    let mut zero = Zero::new(FileFlags::Read | FileFlags::Write);
    let mut buf = [0u8; 10];

    assert_eq!(zero.read(&mut buf).unwrap(), 10);
    assert_eq!(buf.iter().all(|&x| x == 0), true);
    assert_eq!(zero.write(&buf).unwrap(), 10);
    assert_eq!(zero.flush().unwrap(), ());
    assert_eq!(zero.close().unwrap(), ());
}
