use alloc::{format, string::String};

use crate::internal::{file::Stream, fs::FsError, io::File, process::PROCESS_TABLE, user};

/// process handle
#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: u32,
    pub path: String,
}

impl ProcInfo {
    fn resolve(&self) -> Result<String, FsError> {
        // path will be in the form <route>, where <route> forms a part of the overall path /proc/pid/<route>
        // e.g. /proc/1/used_memory -> used_memory
        match self.path.as_str() {
            "ppid" => Ok(format!(
                "{}",
                PROCESS_TABLE
                    .read()
                    .get(self.pid as usize)
                    .ok_or_else(|| { FsError::InvalidPath })?
                    .ppid
            )),
            "used_memory" => Ok(format!(
                "{}",
                PROCESS_TABLE
                    .read()
                    .get(self.pid as usize)
                    .ok_or_else(|| { FsError::InvalidPath })?
                    .allocator
                    .lock()
                    .used()
            )),
            "heap_size" => Ok(format!(
                "{}",
                PROCESS_TABLE
                    .read()
                    .get(self.pid as usize)
                    .ok_or_else(|| { FsError::InvalidPath })?
                    .allocator
                    .lock()
                    .size()
            )),
            "uid" => {
                let str_user = PROCESS_TABLE
                    .read()
                    .get(self.pid as usize)
                    .ok_or_else(|| FsError::InvalidPath)?
                    .data
                    .clone()
                    .user;

                if str_user.is_none() {
                    return Err(FsError::ReadError);
                } else {
                    let uid = user::get_uid(&str_user.unwrap()); // safe as we've checked

                    if uid.is_none() {
                        return Err(FsError::ReadError);
                    } else {
                        return Ok(format!("{}", uid.unwrap()));
                    }
                }
            }
            _ => Err(FsError::InvalidPath),
        }
    }

    pub fn new(pid: u32, path: String) -> Self {
        return Self { pid, path };
    }
}

impl Stream for ProcInfo {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, crate::internal::file::FileError> {
        let out = self.resolve()?;
        let bytes = out.as_bytes();

        // safely copy the bytes to the buffer
        let mut i = 0;
        while i < buf.len() && i < bytes.len() {
            buf[i] = bytes[i];
            i += 1;
        }
        // return the number of bytes read
        Ok(i)
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, crate::internal::file::FileError> {
        Err(crate::internal::file::FileError::PermissionError(
            FsError::WriteError.into(),
        ))
    }

    fn seek(&mut self, _offset: usize) -> Result<usize, crate::internal::file::FileError> {
        Err(crate::internal::file::FileError::PermissionError(
            FsError::WriteError.into(),
        ))
    }

    fn flush(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn poll(&mut self, event: crate::internal::file::IOEvent) -> bool {
        match event {
            crate::internal::file::IOEvent::Read => true,
            crate::internal::file::IOEvent::Write => false,
        }
    }
}
