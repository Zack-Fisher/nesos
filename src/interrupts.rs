use core::iter::Chain;

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::{print, println, vga_help, registers, time::tick};
use lazy_static::lazy_static;

use crate::gdt;

use pic8259::ChainedPics;
use spin;

// we chain two interrupt controllers together
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// actually getting mutable access to these.
// i guess we do need to "program" them later.
// they are "programmable" interrupt controllers, meaning they take in device input and can actually filter out and moderate
// the interrupts before they're sent directly to the CPU.
// i guess this also implies the existance of simply an IC interrupt controller, that buses in interrupts from external devices raw.
pub static PICS: spin::Mutex<ChainedPics> = 
    spin::Mutex::new(unsafe {
        ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
    });

// note that these aren't just normal devices.
// these are anything that can generate an interrupt, so TIMERS will count as well.

// each pin of the interrupt controller can generate different interrupts.

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex
{
    // the timer is at index zero.
    Timer = PIC_1_OFFSET,
    Keyboard, // allow the PS2 keyboard to interact, give it a port in our interrupt table.
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// all interrupts generate a stack frame.
lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        //// notice that all of these have different args they take in.
        // the "breakpoint" exception is when the code throws an "int3" interrupt, which is what most
        // debuggers use to set breakpoints. it comes with a stack frame. 
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            // a "double fault" is when the CPU fails to find the pointer for the normal fault.
            // if the double fault fails, it "triple faults" which is just a hard reset on most hardware.

            // set the handler function
            idt.double_fault.set_handler_fn(double_fault_handler)
                // then set which backup stack it'll use to handle this exception in particular.
                // we specify an index, but remember the actual pointers are loaded into the GDT through the task state segment structure pointer.
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);

            // and NOW, after all of that, a stack overflow will not triple fault and boot loop, rather it'll double fault, then
            // the stack will be properly handled, since we didn't overflow the backup stack.
            // any other exception may still SO, but that's not that big of a deal.
            // double fault is our ace in the hole. it should catch any unhandled exceptions.
        }
        // so we have these special exception handlers that take in args.
        idt.page_fault.set_handler_fn(page_fault_handler);

        // then we have the general indexed ones, that just take in the stack frame. 
        // these are the actual "interrupts" we're going to be using. this is the timer interrupt.
        // they should all have the same sig?
        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);

        idt[16]
            .set_handler_fn(vga_mode_interrupt_handler); 
        idt
    };
}

pub fn init_idt()
{
    // not mutating, just reading the IDT pointer. so we don't even have to spinlock the static IDT.
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: InterruptStackFrame
)
{
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
)
{
    // note: :#? is to debug prettyprint. :? is just normal prints of structures.
    println!("EXCEPTION: PAGE FAULT\n{:#?}\nERROR CODE: {:#?}", stack_frame, error_code);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64
) -> !
{
    use x86_64::registers::control::Cr2;

    println!("Accessed address: {:?}", Cr2::read());
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}, ERROR CODE: {}", stack_frame, error_code);
}

extern "x86-interrupt" fn timer_interrupt_handler(
    stack_frame: InterruptStackFrame,
)
{
    // increment the tick count and implement stuff like sleeping.
    // actually USE the timer as a TIMER.
    tick();

    // the communication between the PICs and the CPU isn't one way.
    // for certain devices, the CPU needs to bark back at it to get it to run again.
    // the timer, for instance, needs this unsafe ping to keep working.
    unsafe {
        // requires mutable access?

        // CMD_END_OF_INTERRUPT is 0x20. all it does is write to the leg of the PIC that it's done by writing this one byte.
        // "i heard you!" "0x20"
        // unsafe fn end_of_interrupt(&mut self) {
        //     self.command.write(CMD_END_OF_INTERRUPT);
        // }

        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

// these are normal IO ports, the 16 bit ones.
// 0x60 to 0x64 are just the ps/2 ports. that's the way it is.
extern "x86-interrupt" fn keyboard_interrupt_handler(
    stack_frame: InterruptStackFrame,
)
{
    // using IO PMIO instead of MMIO
    use x86_64::instructions::port::Port;

    // the interrupt can only tell us WHEN to read the data, we have to read it from a different place, which in this case
    // is the 0x60 IO port.
    let mut port = Port::new(0x60);
    // the keyboard IO port will only let us keep interrupting the CPU if we actually read the Port data.
    let scancode: u8 = unsafe {port.read()};

    crate::keyboard::handle_keycode(scancode);

    unsafe {
        // the same "only the first interrupt works" thing happens here.
        // it seems like the PIC just expects us to notify it for every single device ping, which isn't too unreasonable.
        // the PIC doesn't want to burn itself out pinging a CPU that doesn't exist.
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn vga_mode_interrupt_handler(
    stack_frame: InterruptStackFrame,
)
{
    use core::arch::asm;

    let mut mode: u8 = 0x00;

    // grab the VGA value from the ah register that it should be moved to.
    unsafe {
        mode = registers::get_ah_value();
    }

    vga_help::handle_vga_interrupt(mode);
}