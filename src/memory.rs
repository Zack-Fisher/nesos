// page tables themselves are 4kb,
// but the pages are small, only pointing to the 4kb slot they imply.

use crate::println;

use x86_64::{
    structures::paging::{PageTable, Page, page, page_table, OffsetPageTable, FrameAllocator, Size4KiB, PhysFrame, Mapper},
    VirtAddr, PhysAddr,
};

// pass it the MemoryMap that the bootloader gives us instead of a static offset?
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator { memory_map, next: 0 }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get all the usable regions. this is already done for us in the bootloader.
        let regions = self.memory_map.iter();
        let usable_regions = regions
            .filter(|r| r.region_type == MemoryRegionType::Usable);

        let addr_ranges = usable_regions
            .map(|r| r.range.start_addr()..r.range.end_addr());

        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));

        // return an iterator of all the usable frames of physmem, so we can just pick one.
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator
{
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        // just iterate through all of the frames.
        // maybe set the MemoryRegionType here as well?
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator
{
    // try to allocate a frame, eg try to find an empty frame in memory
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        None
    }
}

// we'll bake the offset into the actual data structure, so we don't have to pass around the boot info.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static>
{
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    // convert a u64 to a pointer to that same u64
    // this conversion is possible since the PageTable is a packed C struct.
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    // return a reference to the actual page table data as a struct.
    &mut *page_table_ptr
}

pub fn create_example_mapping (
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
)
{
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;

    let map_to_result = unsafe {
        // use the OffsetMapper's map_to function.
        // frame is a physical 4k slot of memory.
        // page with flags -> frame by the frame allocator.

        // the mapper creates the page if we haven't already.
        mapper.map_to(page, frame, flags, frame_allocator)
    };

    map_to_result.expect("map_to failed").flush();
}

//// 4kb structure with 64bit PTEs. (512 of them)
// #[repr(align(4096))]
// #[repr(C)]
// #[derive(Clone)]
// pub struct PageTable {
//     entries: [PageTableEntry; ENTRY_COUNT],
// }

pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr>
{
    translate_addr_inner(addr, physical_memory_offset)
}

fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr>
{
    use x86_64::structures::paging::page_table::FrameError;
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    // u16 index into a page table, PT only has 512 PTEs, so this is safe.
    // we're trying to find the page a VirtAddr belongs to, so we need an index in every single table
    // each memory address resides in one p1index.
    // the whole of our memory is paged.
    let table_indexes = [
        addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()
    ];
    let mut frame = level_4_table_frame;

    // we've already got the index into the table that our memory is at.
    // we just need to check if a page is actually present at that address, then we can return it. 
    // otherwise, return None.
    // we reuse the frame we get at each loop, and traverse down the page table.
    for &index in &table_indexes {
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe {&*table_ptr};

        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge frames are not supported."),
        };
    }

    // then cast a u64 to a PhysAddr.
    Some(frame.start_address() + u64::from(addr.page_offset()))
}
