// core file trait that anything involving reading or writing implements

use alloc::{boxed::Box, string::String, vec::Vec};

#[derive(Debug)]
pub enum FileError {
    ReadError(String),
    WriteError(String),
    CloseError(String),
    FlushError(String),
    PermissionError(String),
    NotFoundError(String),
}

pub trait File {
    /// Read from the file into the buffer
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FileError>;

    /// Write from the buffer into the file
    fn write(&mut self, buf: &[u8])  -> Result<usize, FileError>;

    /// close the file (if path is None, close the file that was opened with the open function, arg exists as some implementations need to be able to have multiple files open at once)
    fn close(&mut self, path: Option<&str>) -> Result<(), FileError>;

    /// flush all pending writes - note that for some implementations this may not be necessary, and this function may do nothing, but it is still required to be implemented (even if it just returns Ok(()))
    /// for example, the virtual filesystem needs to implement, as disk writes are comparatively expensive when compared to memory writes
    fn flush(&mut self) -> Result<(), FileError>;
}

pub trait FileSystem {
    /// open a file for reading and writing
    fn open(&mut self, path: &str) -> Result<Box<dyn File>, FileError>;

    /// create a file for reading and writing
    fn create(&mut self, path: &str, owner: u64, perms: [u8;3]) -> Result<Box<dyn File>, FileError>;

    /// delete a file
    fn delete(&mut self, path: &str) -> Result<(), FileError>;

    /// check if a file exists
    fn exists(&mut self, path: &str) -> bool;

    /// change the permissions of a file
    fn chmod(&mut self, path: &str, perms: [u8;3]) -> Result<(), FileError>;

    /// change the owner of a file
    fn chown(&mut self, path: &str, owner: u64) -> Result<(), FileError>;

    /// list the contents of a directory
    fn list(&mut self, path: &str) -> Result<Vec<String>, FileError>;

    /// get the owner of a file
    fn get_owner(&mut self, path: &str) -> Result<u64, FileError>;

    /// get the permissions of a file
    fn get_perms(&mut self, path: &str) -> Result<[u8;3], FileError>;
}