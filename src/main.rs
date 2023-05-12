#![no_std]
#![no_main]

// rename the test harness so that we can call it manually from _start.
// the unit tests are typically called from the main entrypoint, but we overrode that with _start.
// so, we call the test_main test_main instead of just overriding main
#![reexport_test_harness_main = "test_main"]

#![feature(custom_test_frameworks)]
#![test_runner(nesos::test_runner)]

extern crate alloc;
use alloc::boxed::Box;
use vga::writers::{Graphics320x200x256, GraphicsWriter};

use core::panic::PanicInfo;
use nesos::{println, serial_println, vga_draw, memory::{translate_addr, BootInfoFrameAllocator}, allocator};
use x86_64::{structures::paging::{page, Translate, Page, Size4KiB}, VirtAddr};

fn double_info(print_string: &str)
{
    println!("{}", print_string);
    serial_println!("{}", print_string);
}

//// this is a "page fault". you deref bad memory, and it's not mem-mapped to any page on the system.
fn page_fault_ex()
{
    unsafe {
        *(0xfffff0 as *mut u64) = 32;
    };
}

//// this is an example of a "breakpoint" exception. in most cases, it is not a real error, just used by debuggers.
fn breakpoint_ex()
{
    double_info("SENDING TEST EXCEPTION: ");
    x86_64::instructions::interrupts::int3();
    double_info("IT DID NOT CRASH.");
}

//// double fault by causing kernel SO. this does not triple fault, since our double fault exception can switch to the 0th backup stack
// vocab!!!!
// interrupt stack table
// task state segment 
// global descriptor table.
// in the IST in the TSS in the GDT
fn stack_overflow() {
    stack_overflow();
}

fn halt_loop() -> ! 
{
    loop {
        // the halt instruction puts the cpu in a low-power state until the next interrupt.
        // in this way, interrupts can also be used to wake up the dormant CPU.
        // basically, "hey, i'm not doing anything right now. stop executing blank code."
        x86_64::instructions::hlt();
    }
}

fn sleep_loop() -> !
{
    loop {
        nesos::time::hard_sleep(5);
        println!("hello");
    }
}

use bootloader::{BootInfo, entry_point};

// "hey, generate a start function and pass us the correct stuff, bootloader."
entry_point!(kernel_start);

// return type of ! must actually never return. we need this loop at the bottom.
// the bootloader passes us a struct that tells us how memory and pages are laid out for us.
// the pages just get mapped wherever convenient, and we can do the virtmapping ourselves.
fn kernel_start(boot_info: &'static BootInfo) -> ! {
    nesos::init();
    println!("Welcome to NESOS!");

    // grab the address of the highest level page table.
    // we can get the rest of them from here.
    use x86_64::registers::control::Cr3;

    // cr3 contains 4kb of data that describes the top level table
    // cr3 points to the virtual address of the page.
    // so, we need to somehow map it so that the absolute address is obvious from the virtual address of the page.
    // identity map the top level page, or somehow make it offset.
    let (level_4_page_table, _) = Cr3::read();
    let (frame, num) = Cr3::read_raw();
    println!("{:#?}, {}", frame, num);

    // println!("level 4 page at: {:?}", level_4_page_table.start_address());
    // // a mutable reference to a word
    // let ptr8 = 0x1000 as *mut u8;
    // let ptr16 = 0x1000 as *mut u16;
    // let ptr32 = 0x1000 as *mut u32;
    // unsafe {println!("{}", *ptr8);}

    // let addresses = [
    //     0xb8000,
    //     0x201008,
    //     0x0100_0020_1a10,
    //     // maps to zero, naturally.
    //     boot_info.physical_memory_offset,
    // ];

    // for &address in &addresses
    // {
    //     let virt = VirtAddr::new(address);
    //     // our pagetables are offset.
    //     // the pagetables contain memory.
    //     // the type already knows our page structure.
    //     // all pages are the same size for now.
    //     // this will return null on error, rather than panicking.
    //     let phys = mapper.translate_addr(virt);
    //     println!("{:?} -> {:?}", virt, phys);
    // }

    // // let page = Page::containing_address(VirtAddr::new(0));
    // // the page will be allocated if it hasn't already.
    // let page = Page::containing_address(VirtAddr::new(982374892734));
    // // alloc the vga text buffer to this arbitrary address in virt memory.
    // nesos::memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    // let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    // unsafe {page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};

    // // play a simple boot sound, welcome the user
    // nesos::sound::pc_speaker::boot_sound();

    // nesos::sound::pc_speaker::drum_roll(1, 30, 200);

    #[cfg(test)]
    test_main();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    // init the static offsetpagetable searcher we're going to use.
    let mut mapper = unsafe {nesos::memory::init(phys_mem_offset)};
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap init failed");

    nesos::emulation::run_rom();

    halt_loop();
}
