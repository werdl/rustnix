use crate::internal::file::Stream;

#[derive(Debug)]
pub struct Zero {}

impl Stream for Zero {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, crate::internal::file::FileError> {
        // fill buf with 0s
        for i in 0..buf.len() {
            buf[i] = 0;
        }

        Ok(buf.len())
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, crate::internal::file::FileError> {
        Ok(buf.len()) // writing to /dev/zero is always successful
    }

    fn close(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }
}

impl Zero {
    pub fn new() -> Self {
        Zero {}
    }
}

#[test_case]
fn test_zero() {
    let mut zero = Zero::new();
    let mut buf = [0u8; 10];

    assert_eq!(zero.read(&mut buf).unwrap(), 10);
    assert_eq!(buf.iter().all(|&x| x == 0), true);
    assert_eq!(zero.write(&buf).unwrap(), 10);
    assert_eq!(zero.flush().unwrap(), ());
    assert_eq!(zero.close().unwrap(), ());
}
