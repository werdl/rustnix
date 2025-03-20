use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::internal::{
    devices::{null::Null, rand::Rand},
    file::Stream,
    fs::{self, FileMetadata},
    ata,
    file::{self, FileSystem}
};

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

    fn close(&mut self, _path: Option<&str>) -> Result<(), super::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), super::file::FileError> {
        Ok(())
    }
}

pub struct File {
    pub path: String,
    metadata: FileMetadata,
}

impl Stream for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, file::FileError> {
        // get first good fs
        let (bus, dsk) = fs::get_first_good_fs()?;

        let mut fses = fs::FILESYSTEMS.lock();

        // read from file
        let fs = fses.get_mut(&(bus, dsk)).unwrap();

        let mut file = fs.open(&self
            .path
            .split('/')
            .last()
            .expect("File path is empty"))
            .map_err(|e| file::FileError::ReadError(e.to_string()))?;

        file.read(buf)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, file::FileError> {
        // get first good fs
        let (bus, dsk) = fs::get_first_good_fs()?;

        let mut fses = fs::FILESYSTEMS.lock();

        // read from file
        let fs = fses.get_mut(&(bus, dsk)).unwrap();

        let mut file = fs.open(&self
            .path
            .split('/')
            .last()
            .expect("File path is empty"))
            .map_err(|e| file::FileError::WriteError(e.to_string()))?;

        file.write(buf)
    }

    fn close(&mut self, _path: Option<&str>) -> Result<(), file::FileError> {
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

pub enum Device {
    Null(Null),
    Rand(Rand),
}

impl TryFrom<u8> for Device {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Device::Null(Null::new())),
            1 => Ok(Device::Rand(Rand::new())),
            _ => Err(format!("Invalid device number: {}", value)),
        }
    }
}

pub enum Resource {
    File(File),
    StdStream(StdStreams),
    Device(Device),
}

impl Stream for Device {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, super::file::FileError> {
        match self {
            Device::Null(inner) => inner.read(buf),
            Device::Rand(inner) => inner.read(buf),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, super::file::FileError> {
        match self {
            Device::Null(inner) => inner.write(buf),
            Device::Rand(inner) => inner.write(buf),
        }
    }

    fn close(&mut self, _path: Option<&str>) -> Result<(), super::file::FileError> {
        match self {
            Device::Null(inner) => inner.close(_path),
            Device::Rand(inner) => inner.close(_path),
        }
    }

    fn flush(&mut self) -> Result<(), super::file::FileError> {
        match self {
            Device::Null(inner) => inner.flush(),
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

    fn close(&mut self, path: Option<&str>) -> Result<(), super::file::FileError> {
        match self {
            Resource::File(file) => file.close(path),
            Resource::StdStream(stream) => stream.close(path),
            Resource::Device(device) => device.close(path),
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
