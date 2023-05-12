#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nesos::test_runner)]
#![reexport_test_harness_main = "test_main"]
// integration tests are handled in a different file. this is an example of one.
// we have to redefine all the exclusions and weird things we did in main, because this is a STANDALONE executable that
// will be picked up and run by cargo test.

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> !
{
    test_main();

    loop {}
}

pub fn test_runner(tests: &[&dyn Fn()])
{
    unimplemented!();
}
