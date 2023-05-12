use pc_keyboard::{Keyboard, layouts, ScancodeSet1};
use runes::{ppu::Screen, apu::Speaker, controller::InputPoller};
use spin::Mutex;
use vga::{writers::{Graphics640x480x16, GraphicsWriter}, colors::Color16};

use crate::vga_buffer::Color;

pub struct TerminalScreen
{
    mode: Graphics640x480x16,
}

//// EXAMPLE IMPLEMENTATION
    // impl<'a> ppu::Screen for SDLWindow<'a> {
    //     #[inline(always)]
    //     fn put(&mut self, x: u8, y: u8, color: u8) {
    //         let (r, g, b) = get_rgb(color);
    //         let base = (y as usize * FB_PITCH) + x as usize * 3;
    //         self.frame_buffer[base] = r;
    //         self.frame_buffer[base + 1] = g;
    //         self.frame_buffer[base + 2] = b;
    //     }

    //     fn render(&mut self) {
    //         self.texture
    //             .update(None, &self.frame_buffer, FB_PITCH)
    //             .unwrap();
    //     }

    //     fn frame(&mut self) {
    //         self.canvas.clear();
    //         self.canvas
    //             .copy(&self.texture, self.copy_area, None)
    //             .unwrap();
    //         self.canvas.present();
    //         self.event.poll();
    //     }
    // }

impl TerminalScreen
{
    pub fn new() -> TerminalScreen
    {
        let m = Graphics640x480x16::new();
        m.set_mode();
        TerminalScreen { mode: m }
    }
}

impl Screen for TerminalScreen
{
    fn put(&mut self, x: u8, y: u8, color: u8)
    {
        self.mode.set_pixel(x.into(), y.into(), color16_from_u8(color));
    }
    fn render(&mut self)
    {
    }
    fn frame(&mut self)
    {
        self.mode.clear_screen(Color16::Black);
    }
}

fn color16_from_u8(color: u8) -> Color16
{
    Color16::Blue
}

pub struct TerminalAudio
{

}

impl Speaker for TerminalAudio
{
    fn queue(&mut self, sample: i16)
    {

    }
}

// input actionstate wrapper around the Keyboard struct registered with the PIC ps2 interrupts.
pub struct TerminalKeyboard
{
}

impl InputPoller for TerminalKeyboard {
    fn poll(&self) -> u8 {
        let state: u8 = 0;

        state
    }
}
