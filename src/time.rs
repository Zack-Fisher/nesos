use core::convert::TryInto;

use lazy_static::lazy_static;

use crate::{println, print};

// variable that holds the ticks counted up by the PIC timer in PIC offset 0
lazy_static! {
    pub static ref tick_count: spin::Mutex<u64> = 
        spin::Mutex::new(0);
}

// called directly from the interrupt.
pub fn tick()
{
    *tick_count.lock() += 1;
    print!(".");
}

// block the thread?
pub fn hard_sleep(ticks: u64)
{
    let tick_offset = *tick_count.lock();
    let target = ticks + tick_offset;
    

    loop 
    {
        if target < *tick_count.lock()
        {
            break;
        }
        else {
            // wait until next interrupt.
            x86_64::instructions::hlt();
        }
    }
}

pub fn soft_sleep(ticks: usize)
{

}
