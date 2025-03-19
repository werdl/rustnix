use crate::internal::file::Stream;

pub struct Null {}

impl Stream for Null {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, crate::internal::file::FileError> {
        // fill buf with 0s
        for i in 0..buf.len() {
            buf[i] = 0;
        }

        Ok(buf.len())
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, crate::internal::file::FileError> {
        Ok(0)
    }

    fn close(&mut self, _path: Option<&str>) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }
}
