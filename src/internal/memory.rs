use x86_64::structures::paging::{OffsetPageTable, PageTable};
use x86_64::{PhysAddr, VirtAddr};

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = x86_64::registers::control::Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr } // unsafe bit
}

/// Translates the given virtual address to the mapped physical address
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    use x86_64::structures::paging::page_table::FrameError;

    let (level_4_table_frame, _) = x86_64::registers::control::Cr3::read();
    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];

    let mut frame = level_4_table_frame;

    for &index in &table_indexes {
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        let entry = &table[index];

        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => {
                panic!("we're just going to ignore huge pages for now");
            }
        };
    }

    Some(frame.start_address() + u64::from(addr.page_offset()))
}

unsafe fn init_page_table(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };

    unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }
}

use x86_64::structures::paging::{FrameAllocator, Page, PhysFrame, Size4KiB};

/// Create a mapping in the page table that maps the given page to the given frame
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::Mapper;
    use x86_64::structures::paging::PageTableFlags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    let map_to_result = unsafe { mapper.map_to(page, frame, flags, frame_allocator) };
    map_to_result.expect("map_to failed").flush();
}

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

/// A FrameAllocator that returns usable frames from the bootloader's memory map
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map
    pub fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

/// Initialize the memory system
pub fn init(boot_info: &'static bootloader::bootinfo::BootInfo) {
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { init_page_table(phys_mem_offset) };
    let mut frame_allocator = BootInfoFrameAllocator::init(&boot_info.memory_map);

    crate::internal::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
}
