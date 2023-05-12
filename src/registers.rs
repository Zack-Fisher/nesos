//helper functions related to registers.

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use crate::println;

// get any 64 bit register.
unsafe fn get_register(name: &str) -> u64
{
    use core::arch::asm;

    let mut value: u64 = 0;

    match name
    {
        "rax" => {
            unsafe {
                asm!("mov {}, rax", out(reg) value);
            }
        }
        "rbx" => {
            unsafe {
                asm!("mov {}, rbx", out(reg) value);
            }
        }
        "rcx" => {
            unsafe {
                asm!("mov {}, rcx", out(reg) value);
            }
        }
        "rdx" => {
            unsafe {
                asm!("mov {}, rdx", out(reg) value);
            }
        }
        _ => println!("tried to grab invalid register {}", name)
    }

    value
}

pub fn get_ah_value() -> u8 {
    let mut rax: u64 = 0;
    unsafe {rax = get_register("rax");}
    let ah: u8;
    ah = ((rax >> 8) & 0xFF) as u8;
    ah
}
