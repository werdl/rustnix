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

use super::devices::null::Null;

pub static FILES: Mutex<BTreeMap<u8, Resource>> = Mutex::new(BTreeMap::new());

#[derive(Debug)]
pub enum StdStreams {
    Stdin {
        stream: Vec<u8>,
        reader_cb: fn(&[u8]),
    },

    Stdout {
        stream: Vec<u8>,
        writer_cb: fn(&[u8]),
    },

    Stderr {
        stream: Vec<u8>,
        writer_cb: fn(&[u8]),
    },
}

impl Stream for StdStreams {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, super::file::FileError> {
        match self {
            StdStreams::Stdin { stream, reader_cb } => {
                reader_cb(&buf);
                // copy buf to stream
                stream.extend_from_slice(buf);

                Ok(buf.len())
            }
            _ => Err(super::file::FileError::ReadError(format!(
                "Cannot read from {} device",
                match self {
                    StdStreams::Stdin { .. } => "stdin",
                    StdStreams::Stdout { .. } => "stdout",
                    StdStreams::Stderr { .. } => "stderr",
                }
            ))),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, super::file::FileError> {
        match self {
            StdStreams::Stdout { stream, writer_cb } => {
                writer_cb(&buf);
                // copy buf to stream
                stream.extend_from_slice(buf);

                Ok(buf.len())
            }
            StdStreams::Stderr { stream, writer_cb } => {
                writer_cb(&buf);
                // copy buf to stream
                stream.extend_from_slice(buf);

                Ok(buf.len())
            }
            _ => Err(super::file::FileError::WriteError(
                "Cannot read from stdin device".to_string(),
            )),
        }
    }

    fn close(&mut self) -> Result<(), super::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), super::file::FileError> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct File {
    pub path: String,
}

impl Stream for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, file::FileError> {
        // get first good fs
        let (bus, dsk) = fs::get_first_good_fs()?;

        let mut fses = fs::FILESYSTEMS.lock();

        // read from file
        let fs = fses.get_mut(&(bus, dsk)).unwrap();

        let mut file = fs
            .open(&self.path.split('/').last().expect("File path is empty"))
            .map_err(|e| file::FileError::ReadError(e.to_string()))?;

        file.read(buf)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, file::FileError> {
        // get first good fs
        let (bus, dsk) = fs::get_first_good_fs()?;

        let mut fses = fs::FILESYSTEMS.lock();

        // read from file
        let fs = fses.get_mut(&(bus, dsk)).unwrap();

        let mut file = fs
            .open(&self.path.split('/').last().expect("File path is empty"))
            .map_err(|e| file::FileError::WriteError(e.to_string()))?;

        file.write(buf)
    }

    fn close(&mut self) -> Result<(), file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), file::FileError> {
        // write to disk
        let (bus, dsk) = fs::get_first_good_fs()?;

        let mut fses = fs::FILESYSTEMS.lock();

        // flush
        let fs = fses.get_mut(&(bus, dsk)).unwrap();

        fs.phys_fs.write_to_disk(bus, dsk)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum Device {
    Null(Null),
    Zero(Zero),
    Rand(Rand),
}

impl TryFrom<u8> for Device {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Device::Null(Null::new())),
            1 => Ok(Device::Zero(Zero::new())),
            2 => Ok(Device::Rand(Rand::new())),
            _ => Err(format!("Invalid device number: {}", value)),
        }
    }
}

#[derive(Debug)]
pub enum Resource {
    File(File),
    StdStream(StdStreams),
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
}

impl Stream for Resource {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, super::file::FileError> {
        match self {
            Resource::File(file) => file.read(buf),
            Resource::StdStream(stream) => stream.read(buf),
            Resource::Device(device) => device.read(buf),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, super::file::FileError> {
        match self {
            Resource::File(file) => file.write(buf),
            Resource::StdStream(stream) => stream.write(buf),
            Resource::Device(device) => device.write(buf),
        }
    }

    fn close(&mut self) -> Result<(), super::file::FileError> {
        match self {
            Resource::File(file) => file.close(),
            Resource::StdStream(stream) => stream.close(),
            Resource::Device(device) => device.close(),
        }
    }

    fn flush(&mut self) -> Result<(), super::file::FileError> {
        match self {
            Resource::File(file) => file.flush(),
            Resource::StdStream(stream) => stream.flush(),
            Resource::Device(device) => device.flush(),
        }
    }
}

pub fn open(path: &str) -> u8 {
    let resource = match path {
        "/dev/null" => Resource::Device(Device::Null(Null::new())),
        "/dev/zero" => Resource::Device(Device::Zero(Zero::new())),
        "/dev/random" => Resource::Device(Device::Rand(Rand::new())),
        // TODO: streams
        _ => Resource::File(File {
            path: path.to_string(),
        }),
    };

    let mut files = FILES.lock();

    let fd = files.len() as u8;

    files.insert(fd, resource);
    fd
}

pub fn write(fd: u8, buf: &[u8]) -> Result<usize, super::file::FileError> {
    let mut files = FILES.lock();

    let resource: Option<&mut Resource> = files.get_mut(&fd);

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

    let resource: Option<&mut Resource> = files.get_mut(&fd);

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

    let resource: Option<Resource> = files.remove(&fd);


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

    let resource: Option<&mut Resource> = files.get_mut(&fd);

    match resource {
        Some(resource) => resource.flush(),
        None => Err(super::file::FileError::FlushError(format!(
            "Invalid file descriptor: {}",
            fd
        ))),
    }
}
