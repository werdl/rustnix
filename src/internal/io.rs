use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use lazy_static::lazy_static;
use log::{debug, trace};
use spin::Mutex;

use crate::internal::{
    ata,
    devices::{rand::Rand, zero::Zero},
    file::Stream,
    file::{self, FileSystem},
    fs::{self, FileMetadata},
};

use super::{console::Console, devices::null::Null, file::FileFlags, fs::FileHandle};

pub static FILES: Mutex<BTreeMap<u8, File>> = Mutex::new(BTreeMap::new());

#[derive(Debug)]
pub enum Device {
    Null(Null),
    Zero(Zero),
    Rand(Rand),
}

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

#[derive(Debug)]
pub enum File {
    File(FileHandle),
    StdStream(Console),
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

pub fn open(path: &str, flags: u8) -> u8 {
    let resource = match path {
        "/dev/null" => File::Device(Device::Null(Null::new(flags))),
        "/dev/zero" => File::Device(Device::Zero(Zero::new(flags))),
        "/dev/random" => File::Device(Device::Rand(Rand::new(flags))),
        "/dev/stdin" => File::StdStream(Console::new()),
        "/dev/stdout" => File::StdStream(Console::new()),
        "/dev/stderr" => File::StdStream(Console::new()),
        _ => {
            // assume it's a file
            let file_handle = FileHandle::new_with_likely_fs(path.to_string(), flags);

            if file_handle.is_err() {
                return 0;
            }
            File::File(file_handle.unwrap())
        }
    };

    let mut files = FILES.lock();

    let fd = files.len() as u8 + 1;

    files.insert(fd, resource);
    fd
}

pub fn write(fd: u8, buf: &[u8]) -> Result<usize, super::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&fd);

    match resource {
        Some(resource) => resource.write(buf),
        None => Err(super::file::FileError::WriteError(format!(
            "Invalid file descriptor: {}",
            fd
        ))),
    }
}

pub fn read(fd: u8, buf: &mut [u8]) -> Result<usize, super::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&fd);

    match resource {
        Some(resource) => resource.read(buf),
        None => Err(super::file::FileError::ReadError(format!(
            "Invalid file descriptor: {}",
            fd
        ))),
    }
}

pub fn close(fd: u8) -> Result<(), super::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<File> = files.remove(&fd);


    match resource {
        Some(mut resource) => resource.close(),
        None => Err(super::file::FileError::CloseError(format!(
            "Invalid file descriptor: {}",
            fd
        ))),
    }
}

pub fn flush(fd: u8) -> Result<(), super::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<&mut File> = files.get_mut(&fd);

    match resource {
        Some(resource) => resource.flush(),
        None => Err(super::file::FileError::FlushError(format!(
            "Invalid file descriptor: {}",
            fd
        ))),
    }
}
