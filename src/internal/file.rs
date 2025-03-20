// core file trait that anything involving reading or writing implements

use core::fmt::{Display, Formatter};

use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};

use super::fs::FsError;

#[derive(Debug)]
pub enum FileError {
    ReadError(String),
    WriteError(String),
    CloseError(String),
    FlushError(String),
    PermissionError(String),
    NotFoundError(String),
}

impl From<FsError> for FileError {
    fn from(fs_error: FsError) -> Self {
        match fs_error {
            FsError::InvalidPath => FileError::PermissionError(fs_error.to_string()),
            FsError::FileNotFound => FileError::NotFoundError(fs_error.to_string()),
            FsError::FileExists => FileError::WriteError(fs_error.to_string()),
            FsError::DiskFull => FileError::WriteError(fs_error.to_string()),
            FsError::OutOfInodes => FileError::WriteError(fs_error.to_string()),
            FsError::OutOfDataBlocks => FileError::WriteError(fs_error.to_string()),
            FsError::InvalidInode => FileError::WriteError(fs_error.to_string()),
            FsError::InvalidDataBlock => FileError::WriteError(fs_error.to_string()),
            FsError::InvalidSuperblock => FileError::ReadError(fs_error.to_string()),
            FsError::InvalidInodeTable => FileError::ReadError(fs_error.to_string()),
            FsError::InvalidMetadata => FileError::ReadError(fs_error.to_string()),
            FsError::WriteError => FileError::WriteError(fs_error.to_string()),
            FsError::ReadError => FileError::ReadError(fs_error.to_string()),
        }
    }
}

impl Display for FileError {
    fn fmt(&self, f: &mut Formatter) -> alloc::fmt::Result {
        match self {
            FileError::ReadError(s) => write!(f, "ReadError: {}", s),
            FileError::WriteError(s) => write!(f, "WriteError: {}", s),
            FileError::CloseError(s) => write!(f, "CloseError: {}", s),
            FileError::FlushError(s) => write!(f, "FlushError: {}", s),
            FileError::PermissionError(s) => write!(f, "PermissionError: {}", s),
            FileError::NotFoundError(s) => write!(f, "NotFoundError: {}", s),
        }
    }
}

pub trait Stream {
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
    fn open(&mut self, path: &str) -> Result<Box<dyn Stream>, FileError>;

    /// create a file for reading and writing
    fn create(&mut self, path: &str, owner: u64, perms: [u8;3]) -> Result<Box<dyn Stream>, FileError>;

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

pub fn absolute_path(path: &str) -> String {
    if path.starts_with("/") {
        return path.to_string();
    }

    let mut abs_path = "/".to_string();

    abs_path.push_str(path);

    abs_path
}
