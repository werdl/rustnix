use alloc::{
    collections::BTreeMap,
    format,
    string::String,
};

use spin::Mutex;

use crate::{internal::{
    devices::{rand::Rand, zero::Zero},
    file::Stream,
}, kprint};

use super::{console::Console, devices::{null::Null, proc::ProcInfo}, fs::FileHandle};

/// File table
pub static FILES: Mutex<BTreeMap<usize, File>> = Mutex::new(BTreeMap::new());

/// stdout
#[derive(Debug, Clone)]
pub struct Stdout;

impl Stdout {
    /// Create a new Stdout
    pub fn new() -> Self {
        Stdout {}
    }
}

impl Stream for Stdout {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, super::file::FileError> {
        Err(super::file::FileError::ReadError(super::fs::FsError::UnreadableFile.into()))
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, super::file::FileError> {
        kprint!("{}", String::from_utf8_lossy(buf));
        Ok(buf.len())
    }

    fn close(&mut self) -> Result<(), super::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), super::file::FileError> {
        Ok(())
    }

    fn poll(&mut self, _event: super::file::IOEvent) -> bool {
        true
    }

    fn seek(&mut self, pos: usize) -> Result<usize, super::file::FileError> {
        Ok(pos)
    }
}

/// stderr
#[derive(Debug, Clone)]
pub struct Stderr;

impl Stderr {
    /// Create a new Stderr
    pub fn new() -> Self {
        Stderr {}
    }
}

impl Stream for Stderr {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, super::file::FileError> {
        Err(super::file::FileError::ReadError(super::fs::FsError::UnreadableFile.into()))
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, super::file::FileError> {
        kprint!("{}", String::from_utf8_lossy(buf));
        Ok(buf.len())
    }

    fn close(&mut self) -> Result<(), super::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), super::file::FileError> {
        Ok(())
    }

    fn poll(&mut self, _event: super::file::IOEvent) -> bool {
        true
    }

    fn seek(&mut self, pos: usize) -> Result<usize, super::file::FileError> {
        Ok(pos)
    }
}

/// Device
#[derive(Debug, Clone)]
pub enum Device {
    /// stdin device
    Stdin(Console),
    /// stdout device
    Stdout(Stdout),
    /// stderr device
    Stderr(Stderr),
    /// Null device
    Null(Null),
    /// Zero device
    Zero(Zero),
    /// Random device
    Rand(Rand),
}

/// Number of devices - must be updated when adding new devices
pub const NUM_DEVICES: usize = 6;

/// stdin device number and file descriptor
pub const STDIN: u8 = 0;
/// stdout device number and file descriptor
pub const STDOUT: u8 = 1;
/// stderr device number and file descriptor
pub const STDERR: u8 = 2;
/// null device number and file descriptor
pub const NULL: u8 = 3;
/// zero device number and file descriptor
pub const ZERO: u8 = 4;
/// random device number and file descriptor
pub const RAND: u8 = 5;

/// (device number, flags)
impl TryFrom<(u8, u8)> for Device {
    type Error = String;

    fn try_from(value: (u8, u8)) -> Result<Self, Self::Error> {
        match value.0 {
            STDIN => Ok(Device::Stdin(Console::new())),
            STDOUT => Ok(Device::Stdout(Stdout::new())),
            STDERR => Ok(Device::Stderr(Stderr::new())),
            NULL => Ok(Device::Null(Null::new(value.1))),
            ZERO => Ok(Device::Zero(Zero::new(value.1))),
            RAND => Ok(Device::Rand(Rand::new(value.1))),
            _ => Err(format!("Invalid device number: {}", value.0)),
        }
    }
}

/// a file, which could be a normal file, a stream, or a device
#[derive(Debug, Clone)]
pub enum File {
    /// A normal file
    File(FileHandle),
    /// A device
    Device(Device),
    /// proc info
    ProcInfo(ProcInfo),
}

impl Stream for Device {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, super::file::FileError> {
        match self {
            Device::Stdin(inner) => inner.read(buf),
            Device::Stdout(inner) => inner.read(buf),
            Device::Stderr(inner) => inner.read(buf),
            Device::Null(inner) => inner.read(buf),
            Device::Zero(inner) => inner.read(buf),
            Device::Rand(inner) => inner.read(buf),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, super::file::FileError> {
        match self {
            Device::Stdin(inner) => inner.write(buf),
            Device::Stdout(inner) => inner.write(buf),
            Device::Stderr(inner) => inner.write(buf),
            Device::Null(inner) => inner.write(buf),
            Device::Zero(inner) => inner.write(buf),
            Device::Rand(inner) => inner.write(buf),
        }
    }

    fn close(&mut self) -> Result<(), super::file::FileError> {
        match self {
            Device::Stdin(inner) => inner.close(),
            Device::Stdout(inner) => inner.close(),
            Device::Stderr(inner) => inner.close(),
            Device::Null(inner) => inner.close(),
            Device::Zero(inner) => inner.close(),
            Device::Rand(inner) => inner.close(),
        }
    }

    fn flush(&mut self) -> Result<(), super::file::FileError> {
        match self {
            Device::Stdin(inner) => inner.flush(),
            Device::Stdout(inner) => inner.flush(),
            Device::Stderr(inner) => inner.flush(),
            Device::Null(inner) => inner.flush(),
            Device::Zero(inner) => inner.flush(),
            Device::Rand(inner) => inner.flush(),
        }
    }

    fn poll(&mut self, event: super::file::IOEvent) -> bool {
        match self {
            Device::Stdin(inner) => inner.poll(event),
            Device::Stdout(inner) => inner.poll(event),
            Device::Stderr(inner) => inner.poll(event),
            Device::Null(inner) => inner.poll(event),
            Device::Zero(inner) => inner.poll(event),
            Device::Rand(inner) => inner.poll(event),
        }
    }

    fn seek(&mut self, pos: usize) -> Result<usize, super::file::FileError> {
        match self {
            Device::Stdin(inner) => inner.seek(pos),
            Device::Stdout(inner) => inner.seek(pos),
            Device::Stderr(inner) => inner.seek(pos),
            Device::Null(inner) => inner.seek(pos),
            Device::Zero(inner) => inner.seek(pos),
            Device::Rand(inner) => inner.seek(pos),
        }
    }
}

impl Stream for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, super::file::FileError> {
        match self {
            File::File(file) => file.read(buf),
            File::Device(device) => device.read(buf),
            File::ProcInfo(proc_info) => proc_info.read(buf),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, super::file::FileError> {
        match self {
            File::File(file) => file.write(buf),
            File::Device(device) => device.write(buf),
            File::ProcInfo(proc_info) => proc_info.write(buf),
        }
    }

    fn close(&mut self) -> Result<(), super::file::FileError> {
        match self {
            File::File(file) => file.close(),
            File::Device(device) => device.close(),
            File::ProcInfo(proc_info) => proc_info.close(),
        }
    }

    fn flush(&mut self) -> Result<(), super::file::FileError> {
        match self {
            File::File(file) => file.flush(),
            File::Device(device) => device.flush(),
            File::ProcInfo(proc_info) => proc_info.flush(),
        }
    }

    fn poll(&mut self, event: super::file::IOEvent) -> bool {
        match self {
            File::File(file) => file.poll(event),
            File::Device(device) => device.poll(event),
            File::ProcInfo(proc_info) => proc_info.poll(event),
        }
    }

    fn seek(&mut self, pos: usize) -> Result<usize, super::file::FileError> {
        match self {
            File::File(file) => file.seek(pos),
            File::Device(device) => device.seek(pos),
            File::ProcInfo(proc_info) => proc_info.seek(pos),
        }
    }
}
