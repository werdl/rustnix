use alloc::{format, string::ToString, vec::Vec};
use x86_64::registers::mxcsr::read;
use crate::internal::file::File;

pub enum IoDevice {
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

impl File for IoDevice {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, super::file::FileError> {
        match self {
            IoDevice::Stdin { stream, reader_cb } => {
                reader_cb(&buf);
                // copy buf to stream
                stream.extend_from_slice(buf);

                Ok(buf.len())
            },
            _ => Err(super::file::FileError::ReadError(format!("Cannot read from {} device", match self {
                IoDevice::Stdin { .. } => "stdin",
                IoDevice::Stdout { .. } => "stdout",
                IoDevice::Stderr { .. } => "stderr",
            })))
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, super::file::FileError> {
        match self {
            IoDevice::Stdout { stream, writer_cb } => {
                writer_cb(&buf);
                // copy buf to stream
                stream.extend_from_slice(buf);

                Ok(buf.len())
            },
            IoDevice::Stderr { stream, writer_cb } => {
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
