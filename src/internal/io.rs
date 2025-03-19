use alloc::{format, string::{String, ToString}, vec::Vec};

use crate::internal::{devices::{null::Null, rand::Rand}, file::Stream, fs::FileMetadata};

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
    }
}

impl Stream for StdStreams {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, super::file::FileError> {
        match self {
            StdStreams::Stdin { stream, reader_cb } => {
                reader_cb(&buf);
                // copy buf to stream
                stream.extend_from_slice(buf);

                Ok(buf.len())
            },
            _ => Err(super::file::FileError::ReadError(format!("Cannot read from {} device", match self {
                StdStreams::Stdin { .. } => "stdin",
                StdStreams::Stdout { .. } => "stdout",
                StdStreams::Stderr { .. } => "stderr",
            })))
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, super::file::FileError> {
        match self {
            StdStreams::Stdout { stream, writer_cb } => {
                writer_cb(&buf);
                // copy buf to stream
                stream.extend_from_slice(buf);

                Ok(buf.len())
            },
            StdStreams::Stderr { stream, writer_cb } => {
                writer_cb(&buf);
                // copy buf to stream
                stream.extend_from_slice(buf);

                Ok(buf.len())
            },
            _ => Err(super::file::FileError::WriteError("Cannot read from stdin device".to_string()))
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

pub enum Device {
    Null(Null),
    Rand(Rand),
}

pub enum Resource {
    File(File),
    StdStream(StdStreams),
    Device(Device),
}
