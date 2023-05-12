use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }

    fn default() -> ColorCode 
    {
        ColorCode::new(Color::Red, Color::Magenta)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

impl ScreenChar
{
    fn blank() -> ScreenChar
    {
        ScreenChar {ascii_character: b' ', color_code: ColorCode::new(Color::Yellow, Color::Black)}
    }
    
    fn basic(byte: u8) -> ScreenChar
    {
        ScreenChar {ascii_character: byte, color_code: ColorCode::new(Color::Yellow, Color::Black)}
    }
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

use volatile::Volatile;

#[derive(Clone)]
#[repr(transparent)] 
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer, 
}

impl Writer {
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                self.position_write(byte, row, col);

                // move the cursor forward.
                self.column_position += 1;
            }
        }
    }

    fn position_write(&mut self, byte: u8, row: usize, col: usize)
    {
        let color_code = self.color_code;
        // write the byte to the buffer.
        self.buffer.chars[row][col].write(ScreenChar {
            ascii_character: byte,
            color_code,
        });
    }

    pub fn fill_screen_w_char(&mut self, ch: u8)
    {
        for row in 0..BUFFER_HEIGHT
        {
            for col in 0..BUFFER_WIDTH
            {
                self.position_write(ch, row, col);
            }
        }
    }

    fn new_line(&mut self) {
        // it is absolutely crucial that we don't index out of bounds here.
        // it's 1, not 0.
        for row in 1..BUFFER_HEIGHT
        {
            for col in 0..BUFFER_WIDTH
            {
                // use the read and write methods of the Volatile type wrapper in the buffer.
                // we want to write to this static raw pointer as safely as possible.
                let character = self.buffer.chars[row][col].read();
                self.position_write(character.ascii_character, row - 1, col);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize)
    {
        for col in 0..BUFFER_WIDTH
        {
            self.buffer.chars[row][col].write(ScreenChar::blank());
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

use spin::Mutex;

// eval this at runtime lazily, rather than at compile time.
lazy_static::lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::default(),
        // really weird that we can cast a raw pointer to a struct like this, esp an array.
        buffer: unsafe {&mut *(0xB8000 as *mut Buffer)},
    });
}

#[macro_export]
macro_rules! print {
    // basic formatting is internal, and thus part of core::
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    // handle the no args case
    () => ($crate::print!("\n"));
    // then use the print! macro to handle the typical case.
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    // disable interrupts within a given scope.
    // if there's an interrupt in the middle of this function call, we're screwed if that interrupt tries to print with the same writer.
    // since the WRITER is already locked from the interrupted _print call.
    // so, we just turn them off and execute WRITER.write in an interrupt-free environment.
    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}


// TESTS
#[test_case]
fn test_println_simple()
{
    println!("ttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttestest");
}

#[test_case]
fn test_println_many()
{
    for _ in 0..200 {
        println!("ttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttesttestest");
    }
}

#[test_case]
fn test_println_output()
{
    let s = "hello mario";
    println!("{}", s);
    for (i, c)in s.chars().enumerate()
    {
        let screen_char = WRITER.lock().buffer.chars[BUFFER_HEIGHT - 2][i].read();
        assert_eq!(char::from(screen_char.ascii_character), c);
    }
}
