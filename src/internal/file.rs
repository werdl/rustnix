// core file trait that anything involving reading or writing implements

use core::{
    fmt::{Display, Formatter},
    ops::BitOr,
};

use alloc::{boxed::Box, string::String, vec::Vec};

use super::{fs::FsError, syscall::Error};

/// FileInner is a struct that contains the error and an optional message
#[derive(Debug)]
pub struct FileInner {
    fs_error: FsError,
    message: Option<String>,
}

impl From<FsError> for FileInner {
    fn from(fs_error: FsError) -> Self {
        FileInner {
            fs_error: fs_error,
            message: None,
        }
    }
}

/// FileError is an enum that contains all the possible errors that can occur when working with files
#[derive(Debug)]
pub enum FileError {
    /// Error reading from a file
    ReadError(FileInner),
    /// Error writing to a file
    WriteError(FileInner),
    /// Error closing a file
    CloseError(FileInner),
    /// Error flushing a file
    FlushError(FileInner),
    /// Error with permissions
    PermissionError(FileInner),
    /// File not found
    NotFoundError(FileInner),
}

impl From<FsError> for FileError {
    fn from(fs_error: FsError) -> Self {
        match fs_error {
            FsError::InvalidPath => FileError::PermissionError(fs_error.into()),
            FsError::FileNotFound => FileError::NotFoundError(fs_error.into()),
            FsError::FileExists => FileError::WriteError(fs_error.into()),
            FsError::DiskFull => FileError::WriteError(fs_error.into()),
            FsError::OutOfInodes => FileError::WriteError(fs_error.into()),
            FsError::OutOfDataBlocks => FileError::WriteError(fs_error.into()),
            FsError::InvalidInode => FileError::WriteError(fs_error.into()),
            FsError::InvalidDataBlock => FileError::WriteError(fs_error.into()),
            FsError::InvalidSuperblock => FileError::ReadError(fs_error.into()),
            FsError::InvalidInodeTable => FileError::ReadError(fs_error.into()),
            FsError::InvalidMetadata => FileError::ReadError(fs_error.into()),
            FsError::WriteError => FileError::WriteError(fs_error.into()),
            FsError::ReadError => FileError::ReadError(fs_error.into()),
            FsError::UnwritableFile => FileError::WriteError(fs_error.into()),
            FsError::UnreadableFile => FileError::ReadError(fs_error.into()),
            FsError::FilesystemNotFound => FileError::NotFoundError(fs_error.into()),
            FsError::FilesystemExists => FileError::WriteError(fs_error.into()),
            FsError::InvalidFileDescriptor => FileError::PermissionError(fs_error.into()),
        }
    }
}

fn print_msg_or_error(
    f: &mut Formatter,
    msg: &Option<String>,
    fs_error: &FsError,
) -> alloc::fmt::Result {
    // write the error anyway, even if there is a message
    write!(f, "{:?}", fs_error)?;

    if let Some(msg) = msg {
        write!(f, ": {}", msg)?;
    }

    Ok(())
}

impl Display for FileError {
    fn fmt(&self, f: &mut Formatter) -> alloc::fmt::Result {
        match self {
            FileError::ReadError(s) => print_msg_or_error(f, &s.message, &s.fs_error),
            FileError::WriteError(s) => print_msg_or_error(f, &s.message, &s.fs_error),
            FileError::CloseError(s) => print_msg_or_error(f, &s.message, &s.fs_error),
            FileError::FlushError(s) => print_msg_or_error(f, &s.message, &s.fs_error),
            FileError::PermissionError(s) => print_msg_or_error(f, &s.message, &s.fs_error),
            FileError::NotFoundError(s) => print_msg_or_error(f, &s.message, &s.fs_error),
        }
    }
}

/// implement conversion to POSIX error codes
impl From<FsError> for Error {
    fn from(fs_error: FsError) -> Self {
        match fs_error {
            FsError::InvalidPath => Error::EINVAL,
            FsError::FileNotFound => Error::ENOENT,
            FsError::FileExists => Error::EEXIST,
            FsError::DiskFull => Error::ENOSPC,
            FsError::OutOfInodes => Error::ENOSPC,
            FsError::OutOfDataBlocks => Error::ENOSPC,
            FsError::InvalidInode => Error::EIO,
            FsError::InvalidDataBlock => Error::EIO,
            FsError::InvalidSuperblock => Error::EIO,
            FsError::InvalidInodeTable => Error::EIO,
            FsError::InvalidMetadata => Error::EIO,
            FsError::WriteError => Error::EIO,
            FsError::ReadError => Error::EIO,
            FsError::UnwritableFile => Error::EIO,
            FsError::UnreadableFile => Error::EIO,
            FsError::FilesystemNotFound => Error::ENOENT,
            FsError::FilesystemExists => Error::EEXIST,
            FsError::InvalidFileDescriptor => Error::EBADF,
        }
    }
}

impl From<FileError> for Error {
    fn from(file_error: FileError) -> Self {
        match file_error {
            FileError::ReadError(s) => Error::from(s.fs_error),
            FileError::WriteError(s) => Error::from(s.fs_error),
            FileError::CloseError(s) => Error::from(s.fs_error),
            FileError::FlushError(s) => Error::from(s.fs_error),
            FileError::PermissionError(s) => Error::from(s.fs_error),
            FileError::NotFoundError(s) => Error::from(s.fs_error),
        }
    }
}

/// IOEvent is an enum that contains the possible events that can occur when reading or writing to a file
pub enum IOEvent {
    /// Read event
    Read,
    /// Write event
    Write,
}

/// Stream is a trait that contains the functions that need to be implemented when reading or writing to a file
pub trait Stream {
    /// Read from the file into the buffer
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FileError>;

    /// Write from the buffer into the file
    fn write(&mut self, buf: &[u8]) -> Result<usize, FileError>;

    /// close the file
    fn close(&mut self) -> Result<(), FileError>;

    /// flush all pending writes - note that for some implementations this may not be necessary, and this function may do nothing, but it is still required to be implemented (even if it just returns Ok(()))
    /// for example, the virtual filesystem needs to implement, as disk writes are comparatively expensive when compared to memory writes
    fn flush(&mut self) -> Result<(), FileError>;

    /// poll the file for read readiness
    fn poll(&mut self, event: IOEvent) -> bool;
}

/// FileFlags is an enum that contains the possible flags that can be set when opening a file
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum FileFlags {
    /// Read flag
    Read = 1,
    /// Write flag
    Write = 2,
    /// Append flag - write to the end of the file
    Append = 4,
    /// Create flag - create the file if it does not exist
    Create = 8,
    /// Truncate flag - truncate the file if it already exists
    Truncate = 16,
    /// Device flag - open a device file
    Device = 32,
}

impl FileFlags {
    /// check if a flag is set
    pub fn is_set(&self, flags: u8) -> bool {
        flags & (*self as u8) != 0
    }
}

impl BitOr for FileFlags {
    type Output = u8;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as u8 | rhs as u8
    }
}

/// FileSystem is a trait that contains the functions that need to be implemented when working with a filesystem
pub trait FileSystem {
    /// open a file
    fn open(&mut self, path: &str, flags: u8) -> Result<Box<dyn Stream>, FileError>;

    /// delete a file
    fn delete(&mut self, path: &str) -> Result<(), FileError>;

    /// check if a file exists
    fn exists(&mut self, path: &str) -> bool;

    /// change the permissions of a file
    fn chmod(&mut self, path: &str, perms: [u8; 3]) -> Result<(), FileError>;

    /// change the owner of a file
    fn chown(&mut self, path: &str, owner: u64) -> Result<(), FileError>;

    /// list the contents of a directory
    fn list(&mut self, path: &str) -> Result<Vec<String>, FileError>;

    /// get the owner of a file
    fn get_owner(&mut self, path: &str) -> Result<u64, FileError>;

    /// get the permissions of a file
    fn get_perms(&mut self, path: &str) -> Result<[u8; 3], FileError>;
}

/// turn a relative path into an absolute path
pub fn absolute_path(path: &str) -> String {
    if path.starts_with("/") {
        return path.into();
    }

    let mut abs_path: String = "/".into();

    abs_path.push_str(path);

    abs_path
}
