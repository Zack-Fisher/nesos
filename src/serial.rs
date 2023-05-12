// uart_16650 is a really basic serial port model that everything else is back-compat with.
use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;

// make a public writer, without a wrapper class?
lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        // return the crafted serial port out of the block. slick!
        Mutex::new(serial_port)
    };
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments)
{
    // serial is already mostly done for us, it's just a protocol of sending out and in port messages.
    // memory IO and port IO, memory IO is like the VGA buffer, where it's at a specific point in memory.
    // port IO is the same thing, just less absolute in how it's addressed.
    // ports are an abstraction layer over memory, since memory is hard-coded and ports are machine-mapped.
    use core::fmt::Write;
    // expect panics on failure with the message, and we already set our panic function so we're golden.
    // the lib already defines fmt::Write?
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("failed write to serial")
            ;
    });
}

#[macro_export]
macro_rules! serial_print {
    // basic formatting is internal, and thus part of core::
    ($($arg:tt)*) => (
        $crate::serial::_print(format_args!($($arg)*))
    );
}

// a more complicated match
#[macro_export]
macro_rules! serial_println {
    // handle the no args case
    () => ($crate::print!("\n"));
    ($fmt:expr) => (
        $crate::serial_print!(concat!($fmt, "\n"));
    );
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*
    ));
}

