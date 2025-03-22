use core::{
    ptr::NonNull,
    sync::atomic::{AtomicU16, AtomicU32, Ordering},
};

use acpi::{AcpiHandler, AcpiTables, PhysicalMapping};
use alloc::boxed::Box;
use lazy_static::lazy_static;
use log::{info, warn};
use x86_64::{instructions::port::Port, PhysAddr};

use crate::internal::memory;

lazy_static! {
    static ref PM1A_CNT_BLK: AtomicU32 = AtomicU32::new(0);
    static ref SLP_TYPA: AtomicU16 = AtomicU16::new(0);
}

static SLP_LEN: u16 = 1 << 13;

/// ACPI handler
#[derive(Debug, Clone, Copy)]
pub struct HandleAcpi;

impl AcpiHandler for HandleAcpi {
    unsafe fn map_physical_region<T>(&self, addr: usize, size: usize) -> PhysicalMapping<Self, T> {
        let phys_addr = PhysAddr::new(addr as u64);
        let virt_addr = memory::reverse_translate(phys_addr);
        let ptr = NonNull::new(virt_addr.as_mut_ptr()).unwrap();
        unsafe { PhysicalMapping::new(addr, ptr, size, size, Self) }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
}

struct HandleAml;

fn read_addr<T>(addr: usize) -> T
where
    T: Copy,
{
    let virtual_address = memory::reverse_translate(PhysAddr::new(addr as u64));
    unsafe { *virtual_address.as_ptr::<T>() }
}

impl aml::Handler for HandleAml {
    fn read_u8(&self, address: usize) -> u8 {
        read_addr::<u8>(address)
    }
    fn read_u16(&self, address: usize) -> u16 {
        read_addr::<u16>(address)
    }
    fn read_u32(&self, address: usize) -> u32 {
        read_addr::<u32>(address)
    }
    fn read_u64(&self, address: usize) -> u64 {
        read_addr::<u64>(address)
    }

    fn write_u8(&mut self, _: usize, _: u8) {
        unimplemented!()
    }
    fn write_u16(&mut self, _: usize, _: u16) {
        unimplemented!()
    }
    fn write_u32(&mut self, _: usize, _: u32) {
        unimplemented!()
    }
    fn write_u64(&mut self, _: usize, _: u64) {
        unimplemented!()
    }
    fn read_io_u8(&self, _: u16) -> u8 {
        unimplemented!()
    }
    fn read_io_u16(&self, _: u16) -> u16 {
        unimplemented!()
    }
    fn read_io_u32(&self, _: u16) -> u32 {
        unimplemented!()
    }
    fn write_io_u8(&self, _: u16, _: u8) {
        unimplemented!()
    }
    fn write_io_u16(&self, _: u16, _: u16) {
        unimplemented!()
    }
    fn write_io_u32(&self, _: u16, _: u32) {
        unimplemented!()
    }
    fn read_pci_u8(&self, _: u16, _: u8, _: u8, _: u8, _: u16) -> u8 {
        unimplemented!()
    }
    fn read_pci_u16(&self, _: u16, _: u8, _: u8, _: u8, _: u16) -> u16 {
        unimplemented!()
    }
    fn read_pci_u32(&self, _: u16, _: u8, _: u8, _: u8, _: u16) -> u32 {
        unimplemented!()
    }
    fn write_pci_u8(&self, _: u16, _: u8, _: u8, _: u8, _: u16, _: u8) {
        unimplemented!()
    }
    fn write_pci_u16(&self, _: u16, _: u8, _: u8, _: u8, _: u16, _: u16) {
        unimplemented!()
    }
    fn write_pci_u32(&self, _: u16, _: u8, _: u8, _: u8, _: u16, _: u32) {
        unimplemented!()
    }
}

fn log_cpu(cpu: &acpi::platform::Processor) {
    let state = match cpu.state {
        acpi::platform::ProcessorState::Disabled => "disabled",
        acpi::platform::ProcessorState::Running => "running",
        acpi::platform::ProcessorState::WaitingForSipi => "waiting for SIPI",
    };

    info!(
        "CPU: {} (APIC ID: {}, State: {})",
        cpu.processor_uid, cpu.local_apic_id, state
    );
}

/// Initialize ACPI and log CPU information
pub fn init() {
    match unsafe { AcpiTables::search_for_rsdp_bios(HandleAcpi) } {
        Ok(acpi) => {
            if let Ok(info) = acpi.platform_info() {
                if let Some(info) = info.processor_info {
                    log_cpu(&info.boot_processor);
                    for processor in info.application_processors.iter() {
                        log_cpu(&processor);
                    }
                }
            }

            if let Ok(fadt) = acpi.find_table::<acpi::fadt::Fadt>() {
                if let Ok(block) = fadt.pm1a_control_block() {
                    PM1A_CNT_BLK.store(block.address as u32, Ordering::Relaxed);
                }
            }

            if let Ok(dsdt) = acpi.dsdt() {
                let phys = PhysAddr::new(dsdt.address as u64);
                let virt = memory::reverse_translate(phys);
                let table =
                    unsafe { core::slice::from_raw_parts(virt.as_ptr(), dsdt.length as usize) };
                let handler = Box::new(HandleAml);
                let mut aml = aml::AmlContext::new(handler, aml::DebugVerbosity::None);

                if aml.parse_table(table).is_ok() {
                    let name = aml::AmlName::from_str("\\_S5").unwrap();
                    let res = aml.namespace.get_by_path(&name);
                    if let Ok(aml::AmlValue::Package(pkg)) = res {
                        if let Some(aml::AmlValue::Integer(val)) = pkg.get(0) {
                            SLP_TYPA.store(*val as u16, Ordering::Relaxed);
                        }
                    }
                } else {
                    warn!("ACPI: Failed to get S5 package");
                }
            } else {
                warn!("ACPI: Failed to parse DSDT");
            }
        }
        Err(e) => {
            warn!("Failed to initialize ACPI: {:?}", e);
        }
    }
}

/// Shutdown the system
pub fn shutdown() {
    info!("ACPI: Shutting down system");
    unsafe {
        let mut port: Port<u16> = Port::new(PM1A_CNT_BLK.load(Ordering::Relaxed) as u16);
        port.write(SLP_TYPA.load(Ordering::Relaxed) | SLP_LEN);
    }
}
