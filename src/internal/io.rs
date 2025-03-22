use alloc::{
    collections::BTreeMap,
    format,
    string::String,
};

use spin::Mutex;

use crate::internal::{
    devices::{rand::Rand, zero::Zero},
    file::Stream,
};

use super::{console::Console, devices::null::Null, fs::FileHandle};

/// File table
pub static FILES: Mutex<BTreeMap<i8, File>> = Mutex::new(BTreeMap::new());

/// Device
#[derive(Debug)]
pub enum Device {
    /// Null device
    Null(Null),
    /// Zero device
    Zero(Zero),
    /// Random device
    Rand(Rand),
}

/// (device number, flags)
impl TryFrom<(u8, u8)> for Device {
    type Error = String;

    fn try_from(value: (u8, u8)) -> Result<Self, Self::Error> {
        match value.0 {
            0 => Ok(Device::Null(Null::new(value.1))),
            1 => Ok(Device::Zero(Zero::new(value.1))),
            2 => Ok(Device::Rand(Rand::new(value.1))),
            _ => Err(format!("Invalid device number: {}", value.0)),
        }
    }
}

/// a file, which could be a normal file, a stream, or a device
#[derive(Debug)]
pub enum File {
    /// A normal file
    File(FileHandle),
    /// A standard stream (stdin, stdout, stderr)
    StdStream(Console),
    /// A device
    Device(Device),
}

impl Stream for Device {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, super::file::FileError> {
        match self {
            Device::Null(inner) => inner.read(buf),
            Device::Zero(inner) => inner.read(buf),
            Device::Rand(inner) => inner.read(buf),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, super::file::FileError> {
        match self {
            Device::Null(inner) => inner.write(buf),
            Device::Zero(inner) => inner.write(buf),
            Device::Rand(inner) => inner.write(buf),
        }
    }

    fn close(&mut self) -> Result<(), super::file::FileError> {
        match self {
            Device::Null(inner) => inner.close(),
            Device::Zero(inner) => inner.close(),
            Device::Rand(inner) => inner.close(),
        }
    }

    fn flush(&mut self) -> Result<(), super::file::FileError> {
        match self {
            Device::Null(inner) => inner.flush(),
            Device::Zero(inner) => inner.flush(),
            Device::Rand(inner) => inner.flush(),
        }
    }

    fn poll(&mut self, event: super::file::IOEvent) -> bool {
        match self {
            Device::Null(inner) => inner.poll(event),
            Device::Zero(inner) => inner.poll(event),
            Device::Rand(inner) => inner.poll(event),
        }
    }
}

impl Stream for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, super::file::FileError> {
        match self {
            File::File(file) => file.read(buf),
            File::StdStream(stream) => stream.read(buf),
            File::Device(device) => device.read(buf),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, super::file::FileError> {
        match self {
            File::File(file) => file.write(buf),
            File::StdStream(stream) => stream.write(buf),
            File::Device(device) => device.write(buf),
        }
    }

    fn close(&mut self) -> Result<(), super::file::FileError> {
        match self {
            File::File(file) => file.close(),
            File::StdStream(stream) => stream.close(),
            File::Device(device) => device.close(),
        }
    }

    fn flush(&mut self) -> Result<(), super::file::FileError> {
        match self {
            File::File(file) => file.flush(),
            File::StdStream(stream) => stream.flush(),
            File::Device(device) => device.flush(),
        }
    }

    fn poll(&mut self, event: super::file::IOEvent) -> bool {
        match self {
            File::File(file) => file.poll(event),
            File::StdStream(stream) => stream.poll(event),
            File::Device(device) => device.poll(event),
        }
    }
}
