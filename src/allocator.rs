use alloc::alloc::{GlobalAlloc, Layout};
use x86_64::{structures::paging::{FrameAllocator, Size4KiB, mapper::MapToError, Page, frame, PageTableFlags, Mapper}, VirtAddr};
use core::ptr::null_mut;
use linked_list_allocator::LockedHeap;

// virtual addresses can be as large as we need if we already have a page allocator and a frame allocator.
// in a real OS, processes will have their own heap.
// it's important to have a virtual addressing system, so we don't have to worry about conflicts like this.
pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024;

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>>
{
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        // return a pagerange iterator.
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            // allocate all the frames for the heap pages in memory.
            // will allocate the page if it doesn't exist?
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        };
    }

    unsafe {
        // point the heap to our heap virtaddr, and it will access and write to them.
        ALLOCATOR.init(HEAP_START as u64, HEAP_SIZE as u64);
    }

    // now our heap is allocated, and we can put things there.
    Ok(())
}

pub struct Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // return garbage.
        null_mut() 
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        panic!("do not call this")
    }
}

pub struct LockedHeapAllocator {
    heap: spin::Mutex<HeapAllocator>,
}

struct HeapAllocator {
    start: u64,
    size: u64,
    offset: u64,
}

impl LockedHeapAllocator {
    pub fn init(&mut self, start: u64, size: u64) {
        let mut heap = self.heap.lock();
        heap.start = start;
        heap.offset = 0u64;
    }

    pub fn flush(&mut self) {
        todo!();
    }
}

unsafe impl GlobalAlloc for LockedHeapAllocator {
    // if we go out of bounds, we're just plain screwed.
    // don't do this.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut heap = self.heap.lock();
        heap.offset += layout.size() as u64;
        let ptr = (heap.start + heap.offset) as *mut u8;
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        panic!("no dealloc on a heap allocator.")
    }
}

// provide dummy alloc
#[global_allocator]
static mut ALLOCATOR: LockedHeapAllocator = LockedHeapAllocator {heap: spin::Mutex::new(HeapAllocator {start: 0, size: 0, offset: 0})};
