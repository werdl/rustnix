/*
    This code taken from moros, https://github.com/vinc/moros, with minimal modifications.
    The original code is licensed under the MIT license, as is this project.
    The original code from this section can be found at: https://github.com/vinc/moros/blob/trunk/src/sys/ata.rs
*/

use crate::internal::clk;
use alloc::vec::Vec;
use alloc::{borrow::ToOwned, string::String};
use bit_field::BitField;
use core::convert::TryInto;
use core::fmt;
use core::hint::spin_loop;
use lazy_static::lazy_static;
use log::{debug, error, trace, warn};
use spin::Mutex;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

// Information Technology
// AT Attachment with Packet Interface Extension (ATA/ATAPI-4)
// (1998)

/// The size of a block in bytes
pub const BLOCK_SIZE: usize = 512;

/// Keep track of the last selected bus and drive pair to speed up operations
pub static LAST_SELECTED: Mutex<Option<(u8, u8)>> = Mutex::new(None);

#[repr(u16)]
#[derive(Debug, Clone, Copy)]
enum Command {
    Read = 0x20,
    Write = 0x30,
    Identify = 0xEC,
}

enum IdentifyResponse {
    Ata([u16; 256]),
    Atapi,
    Sata,
    None,
}

#[allow(dead_code)]
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
enum Status {
    ERR = 0,  // Error
    IDX = 1,  // (obsolete)
    CORR = 2, // (obsolete)
    DRQ = 3,  // Data Request
    DSC = 4,  // (command dependant)
    DF = 5,   // (command dependant)
    DRDY = 6, // Device Ready
    BSY = 7,  // Busy
}

/// An ATA bus
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Bus {
    id: u8,
    irq: u8,

    data_register: Port<u16>,
    error_register: PortReadOnly<u8>,
    features_register: PortWriteOnly<u8>,
    sector_count_register: Port<u8>,
    lba0_register: Port<u8>,
    lba1_register: Port<u8>,
    lba2_register: Port<u8>,
    drive_register: Port<u8>,
    status_register: PortReadOnly<u8>,
    command_register: PortWriteOnly<u8>,

    alternate_status_register: PortReadOnly<u8>,
    control_register: PortWriteOnly<u8>,
    drive_blockess_register: PortReadOnly<u8>,
}

impl Bus {
    /// Create a new bus
    pub fn new(id: u8, io_base: u16, ctrl_base: u16, irq: u8) -> Self {
        Self {
            id,
            irq,
            data_register: Port::new(io_base + 0),
            error_register: PortReadOnly::new(io_base + 1),
            features_register: PortWriteOnly::new(io_base + 1),
            sector_count_register: Port::new(io_base + 2),
            lba0_register: Port::new(io_base + 3),
            lba1_register: Port::new(io_base + 4),
            lba2_register: Port::new(io_base + 5),
            drive_register: Port::new(io_base + 6),
            status_register: PortReadOnly::new(io_base + 7),
            command_register: PortWriteOnly::new(io_base + 7),
            alternate_status_register: PortReadOnly::new(ctrl_base + 0),
            control_register: PortWriteOnly::new(ctrl_base + 0),
            drive_blockess_register: PortReadOnly::new(ctrl_base + 1),
        }
    }

    fn check_floating_bus(&mut self) -> Result<(), ()> {
        match self.status() {
            0xFF | 0x7F => Err(()),
            _ => Ok(()),
        }
    }

    fn wait(&mut self, ns: u64) {
        for _ in 0..ns {
            spin_loop();
        }
    }

    fn clear_interrupt(&mut self) -> u8 {
        unsafe { self.status_register.read() }
    }

    fn status(&mut self) -> u8 {
        unsafe { self.alternate_status_register.read() }
    }

    fn lba1(&mut self) -> u8 {
        unsafe { self.lba1_register.read() }
    }

    fn lba2(&mut self) -> u8 {
        unsafe { self.lba2_register.read() }
    }

    fn read_data(&mut self) -> u16 {
        unsafe { self.data_register.read() }
    }

    fn write_data(&mut self, data: u16) {
        unsafe { self.data_register.write(data) }
    }

    fn is_error(&mut self) -> bool {
        self.status().get_bit(Status::ERR as usize)
    }
    fn poll(&mut self, bit: Status, val: bool) -> Result<(), ()> {
        let start = clk::get_time_since_boot();
        while self.status().get_bit(bit as usize) != val {
            if clk::get_time_since_boot() - start > 1.0 {
                error!("ATA hanged while polling {:?} bit in status register", bit);
                self.debug();
                return Err(());
            }
            spin_loop();
        }
        Ok(())
    }

    fn select_drive(&mut self, drive: u8) -> Result<(), ()> {
        self.poll(Status::BSY, false)?;
        self.poll(Status::DRQ, false)?;

        // Skip the rest if this drive was already selected
        if *LAST_SELECTED.lock() == Some((self.id, drive)) {
            return Ok(());
        } else {
            *LAST_SELECTED.lock() = Some((self.id, drive));
        }

        unsafe {
            // Bit 4 => DEV
            // Bit 5 => 1
            // Bit 7 => 1
            self.drive_register.write(0xA0 | (drive << 4))
        }

        // wait at least 400 ns
        self.wait(400);

        self.poll(Status::BSY, false)?;
        self.poll(Status::DRQ, false)?;
        Ok(())
    }

    fn write_command_params(&mut self, drive: u8, block: u32) -> Result<(), ()> {
        let lba = true;
        let mut bytes = block.to_le_bytes();
        bytes[3].set_bit(4, drive > 0);
        bytes[3].set_bit(5, true);
        bytes[3].set_bit(6, lba);
        bytes[3].set_bit(7, true);
        unsafe {
            self.sector_count_register.write(1);
            self.lba0_register.write(bytes[0]);
            self.lba1_register.write(bytes[1]);
            self.lba2_register.write(bytes[2]);
            self.drive_register.write(bytes[3]);
        }
        Ok(())
    }

    fn write_command(&mut self, cmd: Command) -> Result<(), ()> {
        unsafe { self.command_register.write(cmd as u8) }
        self.wait(400); // Wait at least 400 ns
        self.status(); // Ignore results of first read
        self.clear_interrupt();
        if self.status() == 0 {
            // Drive does not exist
            return Err(());
        }
        if self.is_error() {
            warn!("ATA {:?} command errored", cmd);
            self.debug();
            return Err(());
        }
        self.poll(Status::BSY, false)?;
        self.poll(Status::DRQ, true)?;
        Ok(())
    }

    fn setup_pio(&mut self, drive: u8, block: u32) -> Result<(), ()> {
        self.select_drive(drive)?;
        self.write_command_params(drive, block)?;
        Ok(())
    }

    fn read(&mut self, drive: u8, block: u32, buf: &mut [u8]) -> Result<(), ()> {
        assert!(buf.len() == BLOCK_SIZE);
        self.setup_pio(drive, block)?;
        self.write_command(Command::Read)?;
        for chunk in buf.chunks_mut(2) {
            let data = self.read_data().to_le_bytes();
            chunk.clone_from_slice(&data);
        }
        if self.is_error() {
            error!("ATA read: data error");
            self.debug();
            Err(())
        } else {
            Ok(())
        }
    }

    fn write(&mut self, drive: u8, block: u32, buf: &[u8]) -> Result<(), ()> {
        assert!(buf.len() == BLOCK_SIZE);
        self.setup_pio(drive, block)?;
        self.write_command(Command::Write)?;
        for chunk in buf.chunks(2) {
            let data = u16::from_le_bytes(chunk.try_into().unwrap());
            self.write_data(data);
        }
        if self.is_error() {
            error!("ATA write: data error");
            self.debug();
            Err(())
        } else {
            Ok(())
        }
    }

    fn identify_drive(&mut self, drive: u8) -> Result<IdentifyResponse, ()> {
        if self.check_floating_bus().is_err() {
            return Ok(IdentifyResponse::None);
        }
        self.select_drive(drive)?;
        self.write_command_params(drive, 0)?;
        if self.write_command(Command::Identify).is_err() {
            if self.status() == 0 {
                return Ok(IdentifyResponse::None);
            } else {
                return Err(());
            }
        }
        match (self.lba1(), self.lba2()) {
            (0x00, 0x00) => Ok(IdentifyResponse::Ata([(); 256].map(|_| self.read_data()))),
            (0x14, 0xEB) => Ok(IdentifyResponse::Atapi),
            (0x3C, 0xC3) => Ok(IdentifyResponse::Sata),
            (_, _) => Err(()),
        }
    }

    #[allow(dead_code)]
    fn reset(&mut self) {
        unsafe {
            self.control_register.write(4); // Set SRST bit
            self.wait(5); // Wait at least 5 ns
            self.control_register.write(0); // Then clear it
            self.wait(2000); // Wait at least 2 ms
        }
    }

    #[allow(dead_code)]
    fn debug(&mut self) {
        unsafe {
            trace!(
                "ATA status register: 0b{:08b} <BSY|DRDY|#|#|DRQ|#|#|ERR>",
                self.alternate_status_register.read()
            );
            trace!(
                "ATA error register:  0b{:08b} <#|#|#|#|#|ABRT|#|#>",
                self.error_register.read()
            );
        }
    }
}

lazy_static! {
    /// The ATA buses
    pub static ref BUSES: Mutex<Vec<Bus>> = Mutex::new(Vec::new());
}

/// Initialize the ATA driver
pub fn init() {
    trace!("Initializing ATA driver");
    {
        let mut buses = BUSES.lock();
        buses.push(Bus::new(0, 0x1F0, 0x3F6, 14));
        buses.push(Bus::new(1, 0x170, 0x376, 15));
    }

    for drive in list() {
        debug!("ATA {}:{} {}", drive.bus, drive.dsk, drive);
    }

    // print probable disk
    if let Ok((bus, dsk)) = likely_disk() {
        debug!(
            "Probable disk: {}:{} {}",
            bus,
            dsk,
            Drive::open(bus, dsk).unwrap()
        );
    } else {
        warn!("No disk found");
    }
}

/// An ATA drive
#[derive(Clone, Debug)]
pub struct Drive {
    /// The bus number
    pub bus: u8,
    /// The drive number
    pub dsk: u8,
    model: String,
    serial: String,
    block_count: u32,
    _block_index: u32,
}

impl Drive {
    /// Get the size of a block in bytes
    pub fn size() -> usize {
        BLOCK_SIZE
    }

    /// Open a drive
    pub fn open(bus: u8, dsk: u8) -> Option<Self> {
        let mut buses = BUSES.lock();
        let res = buses[bus as usize].identify_drive(dsk);
        if let Ok(IdentifyResponse::Ata(res)) = res {
            let buf = res.map(u16::to_be_bytes).concat();
            let model = String::from_utf8_lossy(&buf[54..94]).trim().into();
            let serial = String::from_utf8_lossy(&buf[20..40]).trim().into();
            let block_count = u32::from_be_bytes(buf[120..124].try_into().unwrap()).rotate_left(16);
            let block_index = 0;

            Some(Self {
                bus,
                dsk,
                model,
                serial,
                block_count,
                _block_index: block_index,
            })
        } else {
            None
        }
    }

    /// Get the size of a block in bytes
    pub const fn block_size(&self) -> u32 {
        BLOCK_SIZE as u32
    }

    /// Get the number of blocks
    pub fn block_count(&self) -> u32 {
        self.block_count
    }

    fn humanized_size(&self) -> (usize, String) {
        let size = self.block_size() as usize;
        let count = self.block_count() as usize;
        let bytes = size * count;
        if bytes >> 20 < 1000 {
            (bytes >> 20, String::from("MB".to_owned()))
        } else {
            (bytes >> 30, String::from("GB".to_owned()))
        }
    }
}

impl fmt::Display for Drive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (size, unit) = self.humanized_size();
        write!(f, "{} {} ({} {})", self.model, self.serial, size, unit)
    }
}

/// List all drives
pub fn list() -> Vec<Drive> {
    let mut res = Vec::new();
    for bus in 0..2 {
        for dsk in 0..2 {
            if let Some(drive) = Drive::open(bus, dsk) {
                res.push(drive)
            }
        }
    }
    res
}

/// Read a block from a drive
pub fn read(bus: u8, drive: u8, block: u32, buf: &mut [u8]) -> Result<(), ()> {
    let mut buses = BUSES.lock();
    buses[bus as usize].read(drive, block, buf)
}

/// Write a block to a drive
pub fn write(bus: u8, drive: u8, block: u32, buf: &[u8]) -> Result<(), ()> {
    let mut buses = BUSES.lock();
    buses[bus as usize].write(drive, block, buf)
}

/// Get the bus and drive of the most likely disk (the one with the most blocks)
pub fn likely_disk() -> Result<(u8, u8), ()> {
    let drives = list();
    if drives.len() == 0 {
        return Err(());
    }

    // get the drive with the most blocks
    let mut max = 0;

    let mut bus = 0;
    let mut dsk = 0;

    for drive in drives {
        if drive.block_count() > max {
            max = drive.block_count();
            bus = drive.bus;
            dsk = drive.dsk;
        }
    }

    Ok((bus, dsk))
}
