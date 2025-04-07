use core::sync::atomic::{AtomicUsize, Ordering};

use log::warn;
use spin::Once;
use x86_64::structures::paging::page_table::FrameError;
use x86_64::structures::paging::{Mapper, OffsetPageTable, PageTable, PageTableFlags};
use x86_64::{PhysAddr, VirtAddr};

/// The offset of the physical memory
static PHYSICAL_MEMORY_OFFSET: Once<u64> = Once::new(); // will get overwritten by init
static MEMORY_MAP: Once<&'static bootloader::bootinfo::MemoryMap> = Once::new(); // will get overwritten by init
static ALLOCATED_FRAMES: AtomicUsize = AtomicUsize::new(0);

/// fetch the physical memory offset )
pub fn physical_memory_offset() -> VirtAddr {
    VirtAddr::new(
        *PHYSICAL_MEMORY_OFFSET
            .get()
            .expect("Couldn't fetch physical memory offset"),
    )
}

/// The active level 4 page table
pub unsafe fn active_level_4_table() -> &'static mut PageTable {
    let (level_4_table_frame, _) = x86_64::registers::control::Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset() + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

pub unsafe fn active_page_table() -> &'static mut PageTable {
    let phys = x86_64::registers::control::Cr3::read().0.start_address();
    let virt = VirtAddr::new(phys.as_u64());
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

/// Translates the given virtual address to the mapped physical address
pub unsafe fn translate_addr(addr: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(
        addr,
        VirtAddr::new(*PHYSICAL_MEMORY_OFFSET.call_once(|| addr.as_u64())),
    )
}

/// Translates the given physical address to the virtual address
pub fn reverse_translate(addr: PhysAddr) -> VirtAddr {
    VirtAddr::new(addr.as_u64() + *PHYSICAL_MEMORY_OFFSET.call_once(|| addr.as_u64()))
}

fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
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
    let _ = *PHYSICAL_MEMORY_OFFSET.call_once(|| physical_memory_offset.as_u64());

    let level_4_table = unsafe { active_level_4_table() };

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
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map
    pub fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
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
        let next = ALLOCATED_FRAMES.fetch_add(1, Ordering::SeqCst);
        // FIXME: When the heap is larger than a few megabytes,
        // creating an iterator for each allocation become very slow.
        self.usable_frames().nth(next)
    }
}

/// Initialize the memory system
pub fn init(boot_info: &'static bootloader::bootinfo::BootInfo) {
    crate::internal::vga::trace("Initializing memory");
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { init_page_table(phys_mem_offset) };
    let mut frame_allocator = BootInfoFrameAllocator::init(&boot_info.memory_map);

    MEMORY_MAP.call_once(|| &boot_info.memory_map);

    crate::internal::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
}

pub fn create_page_table(frame: PhysFrame) -> &'static mut PageTable {
    let phys_addr = frame.start_address();
    let virt_addr = reverse_translate(phys_addr);
    let page_table_ptr: *mut PageTable = virt_addr.as_mut_ptr();
    unsafe { &mut *page_table_ptr }
}

pub fn frame_allocator() -> BootInfoFrameAllocator {
    unsafe { BootInfoFrameAllocator::init(MEMORY_MAP.get_unchecked()) }
}

pub fn alloc_pages(mapper: &mut OffsetPageTable, addr: u64, size: usize) -> Result<(), ()> {
    let size = size.saturating_sub(1) as u64;
    let mut frame_allocator = frame_allocator();

    let pages = {
        let start_page = Page::containing_address(VirtAddr::new(addr));
        let end_page = Page::containing_address(VirtAddr::new(addr + size));
        Page::range_inclusive(start_page, end_page)
    };

    let flags =
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

    for page in pages {
        if let Some(frame) = frame_allocator.allocate_frame() {
            let res = unsafe { mapper.map_to(page, frame, flags, &mut frame_allocator) };
            if let Ok(mapping) = res {
                //debug!("Mapped {:?} to {:?}", page, frame);
                mapping.flush();
            } else {
                warn!("Could not map {:?} to {:?}", page, frame);
                if let Ok(old_frame) = mapper.translate_page(page) {
                    warn!("Already mapped to {:?}", old_frame);
                }
                return Err(());
            }
        } else {
            warn!("Could not allocate frame for {:?}", page);
            return Err(());
        }
    }

    Ok(())
}

pub fn free_pages(mapper: &mut OffsetPageTable, addr: u64, size: usize) {
    let size = size.saturating_sub(1) as u64;

    let pages = {
        let start_page: Page<Size4KiB> = Page::containing_address(VirtAddr::new(addr));
        let end_page = Page::containing_address(VirtAddr::new(addr + size));
        Page::range_inclusive(start_page, end_page)
    };

    for page in pages {
        let res =  mapper.unmap(page);
        if let Err(_err) = res {
            // we don't warn here because the process exiter will unmap all MAX_PROC_SIZE
        }
    }
}
