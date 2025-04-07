use core::sync::atomic::{fence, Ordering};

use crate::syscall;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    Success        =   0,
    Failure        =   1,
    UsageError     =  64,
    DataError      =  65,
    OpenError      = 128,
    ReadError      = 129,
    ExecError      = 130,
    PageFaultError = 200,
    ShellExit      = 255,
}

impl From<u8> for ExitCode {
    fn from(code: u8) -> Self {
        match code {
            0 => ExitCode::Success,
            1 => ExitCode::Failure,
            64 => ExitCode::UsageError,
            65 => ExitCode::DataError,
            128 => ExitCode::OpenError,
            129 => ExitCode::ReadError,
            130 => ExitCode::ExecError,
            200 => ExitCode::PageFaultError,
            _ => ExitCode::ShellExit,
        }
    }
}
