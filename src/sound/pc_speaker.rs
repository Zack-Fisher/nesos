use crate::println;

// PIT - programmable interval timer.
// The PIT's channel 2 data port
const PIT_CHANNEL_2_DATA_PORT: u16 = 0x42;

// The PIT's command port
const PIT_COMMAND_PORT: u16 = 0x43;

// The PC speaker's control port
const PC_SPEAKER_CONTROL_PORT: u16 = 0x61;

// the pit takes in a frequency and turns the pc speaker on and off really fast, that's how it plays music and that's 
// why it's necessary for the pit to be there.

// you can't just "send it a frequency", you have to first activate the PITs vibration, and THEN turn on the pc speaker.

pub fn start_sound(frequency: u32)
{
    let pit_frequency: u32 = 1193180;
    let pit_divisor: u16 = (pit_frequency / frequency) as u16;

    unsafe {
        use x86_64::instructions::port::Port;

        // Enable PIT channel 2 and set it to square wave mode
        Port::new(PIT_COMMAND_PORT).write(0xB6 as u8);

        // Set the PIT divisor
        Port::new(PIT_CHANNEL_2_DATA_PORT).write((pit_divisor & 0xFF) as u8);
        Port::new(PIT_CHANNEL_2_DATA_PORT).write((pit_divisor >> 8) as u8);

        // Enable the PC speaker
        let mut control_port = x86_64::instructions::port::Port::<u8>::new(PC_SPEAKER_CONTROL_PORT);
        let current_value = control_port.read();
        control_port.write(current_value | 0x03);
    }
}

pub fn stop_sound()
{
    unsafe {
        // Disable the PC speaker
        let mut control_port = x86_64::instructions::port::Port::<u8>::new(PC_SPEAKER_CONTROL_PORT);
        let current_value = control_port.read();
        control_port.write(current_value & 0xFC);
    }
}

// note: read the hard_sleep function def, it just uses the PIC ticks as a timer.
// this probably won't be good enough for real audio, i might have to use a different timing mechanism or make the PIC
// tick faster, which is apparently possible.]
pub fn play_sound(frequency: u32, duration_ticks: u64) {
    start_sound(frequency);

    // just write the sound value to the speaker, wait, then turn off the speaker.
    crate::time::hard_sleep(duration_ticks);

    stop_sound();
}

pub fn boot_sound() 
{
    for i in 1..5
    {
        play_sound(100, i);
        play_sound(200, i * 2);
    }

    play_sound(400, 5);
}

pub fn drum_roll(tick_gap: u64, times_around: u64, base_frequency: u32)
{
    for _ in 0..times_around
    {
        play_sound(base_frequency, tick_gap);
        play_sound(base_frequency + 50, tick_gap);
    }
}
