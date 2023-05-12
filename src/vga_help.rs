use core::arch::asm;

use vga::writers::{Graphics640x480x16, GraphicsWriter};

use crate::{println, vga_draw::draw_line};
use vga::colors::Color16;

pub fn draw_mode()
{
    let mode = Graphics640x480x16::new();
    mode.set_mode();

    mode.clear_screen(Color16::Black);
    mode.draw_line((5, 7), (30, 100), Color16::Blue);
    mode.draw_line((50, 7), (30, 200), Color16::Red);
}

pub fn handle_vga_interrupt(num: u8)
{
    println!("handling vga interrupt {}", num);

    match num
    {
        0x13 => {
            draw_mode();
        }, 
        _ => println!("invalid VGA mode specified.")
    }

}
