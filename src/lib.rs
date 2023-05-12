// to make everything in our OS available to integration tests, define a library that manually exposes all of our OS
// apis to them.

#![no_std]
#![reexport_test_harness_main = "test_main"]

// be able to make x86 interrupt function pointers to handle interrupts.
#![feature(abi_x86_interrupt)]
// define our own heap allocation functions.

#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]

extern crate alloc;

pub mod allocator;
pub mod memory;
pub mod emulation;
pub mod time;
pub mod registers;
pub mod sound;
pub mod serial;
pub mod vga_help;
pub mod vga_buffer;
pub mod vga_draw;
pub mod interrupts;
pub mod keyboard;
pub mod gdt;

use core::panic::PanicInfo;

pub trait Testable {
    fn run(&self) -> ();
}

// all functions are testable.
// we just wrap it in prints.
impl<T> Testable for T
where
    T: Fn()
{
    fn run(&self)
    {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable])
{
    serial_println!("running {} tests", tests.len()); for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

// the testing framework built into rust just passes all the #[test_case]'s to the test runner specified, when test_main() is called
// or, by default in std projects it's called when main() is called during a cargo test.
#[test_case]
fn trivial_assertion() 
{
    assert_eq!(1, 1);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// in the cargo.toml, we mapped a qemu "device" on the command line to the port 0xf4.
/// when we write any value to that, qemu will quit.
/// this is a wrapper around that logic.
pub fn exit_qemu(exit_code: QemuExitCode)
{
    // the x86 equivalent, literally use the "in" and "out" instructions. 
    //     ; Load the port address into the DX register
    //     mov dx, [port_address]
    //     ; Load the exit code into the EAX register
    //     mov eax, [exit_code]
    //     ; Write the exit code to the port
    //     out dx, eax

    // all this crate does is give us wrappers for the Port type and the "out" instruction, now just a port.write() command.
    use x86_64::instructions::port::Port;

    unsafe {
        // 16 bit port numbers.
        // mov dx, 0xf4
        let mut port = Port::new(0xf4);
        // out dx, eax
        port.write(exit_code as u32);
    }
}

use core::arch::asm;

fn adjust_pic_speed()
{
    unsafe {
        asm!(
            "mov ax, 1193180",
            "mov bx, $0",
            "div bx",
            "mov cx, ax",
            "mov al, 0x36",
            "out 0x43, al",
            "mov al, cl",
            "out 0x40, al",
            "mov al, ch",
            "out 0x40, al",
        );
    }
}

pub fn init()
{
    // //// WHY DOES CHANGING VIDEO MODES BOOTLOOP
    println!("Initializing the machine...");
    // initialize the interrupt table, so that they actually work at all.
    // we can invoke interrupts with the x86 crate.
    println!("Initting the IDT");
    interrupts::init_idt();
    println!("Finished initting the IDT");

    // gdt will also load the tss with all the stack pointer tables for interrupts and exceptions.
    println!("Initting the GDT");
    gdt::init();
    println!("Finished initting the GDT");

    println!("Initting the PIC");
    // adjust_pic_speed();
    unsafe {
        // init the PIC chain going into the CPU.
        interrupts::PICS.lock().initialize();
    };
    println!("Finished initting the PIC");

    x86_64::instructions::interrupts::enable();

    use core::arch::asm;

    unsafe {
        asm!(
            "mov ah, 0x00",
            "mov ah, 0x13",
            "int 0x10",
            options(nostack, preserves_flags),
        );
    }

    println!("Done initializing!");
}

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    serial_println!("{}", info);
    println!("{}", info);

    loop {}
}
