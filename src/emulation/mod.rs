pub mod construct;

extern crate alloc;

use alloc::vec;
use runes::apu::APU;
use runes::cartridge::{BankType, Cartridge, MirrorType};
use runes::controller::{stdctl, InputPoller};
use runes::mapper;
use runes::memory::{CPUMemory, PPUMemory};
use runes::mos6502;
use runes::ppu;
use runes::utils;

use alloc::{vec::Vec, boxed::Box};

use core::mem::transmute;

use crate::emulation::construct::TerminalKeyboard;
use crate::{println, serial_println};


const RGB_COLORS: [u32; 64] = [
    0x666666, 0x002a88, 0x1412a7, 0x3b00a4, 0x5c007e, 0x6e0040, 0x6c0600,
    0x561d00, 0x333500, 0x0b4800, 0x005200, 0x004f08, 0x00404d, 0x000000,
    0x000000, 0x000000, 0xadadad, 0x155fd9, 0x4240ff, 0x7527fe, 0xa01acc,
    0xb71e7b, 0xb53120, 0x994e00, 0x6b6d00, 0x388700, 0x0c9300, 0x008f32,
    0x007c8d, 0x000000, 0x000000, 0x000000, 0xfffeff, 0x64b0ff, 0x9290ff,
    0xc676ff, 0xf36aff, 0xfe6ecc, 0xfe8170, 0xea9e22, 0xbcbe00, 0x88d800,
    0x5ce430, 0x45e082, 0x48cdde, 0x4f4f4f, 0x000000, 0x000000, 0xfffeff,
    0xc0dfff, 0xd3d2ff, 0xe8c8ff, 0xfbc2ff, 0xfec4ea, 0xfeccc5, 0xf7d8a5,
    0xe4e594, 0xcfef96, 0xbdf4ab, 0xb3f3cc, 0xb5ebf2, 0xb8b8b8, 0x000000,
    0x000000,
];

const PIX_WIDTH: u32 = 256;
const PIX_HEIGHT: u32 = 240;
const FB_PITCH: usize = PIX_WIDTH as usize * 3;
const FB_SIZE: usize = PIX_HEIGHT as usize * FB_PITCH;
const AUDIO_SAMPLES: u16 = 441;
const AUDIO_EXTRA_SAMPLES: u16 = 4410;
const AUDIO_ALL_SAMPLES: u16 = AUDIO_SAMPLES + AUDIO_EXTRA_SAMPLES;

pub struct SimpleCart {
    chr_rom: Vec<u8>,
    prg_rom: Vec<u8>,
    sram: Vec<u8>,
    pub mirror_type: MirrorType,
}

impl SimpleCart {
    pub fn new(
        chr_rom: Vec<u8>,
        prg_rom: Vec<u8>,
        sram: Vec<u8>,
        mirror_type: MirrorType,
    ) -> Self {
        SimpleCart {
            chr_rom,
            prg_rom,
            sram,
            mirror_type,
        }
    }

    fn load_vec(vec: &mut Vec<u8>, reader: &mut dyn utils::Read) -> bool {
        let len = vec.len();
        match reader.read(vec) {
            Some(x) => x == len,
            None => false,
        }
    }

    fn save_vec(vec: &Vec<u8>, writer: &mut dyn utils::Write) -> bool {
        let len = vec.len();
        match writer.write(vec) {
            Some(x) => x == len,
            None => false,
        }
    }
}

impl Cartridge for SimpleCart {
    fn get_size(&self, kind: BankType) -> usize {
        match kind {
            BankType::PrgRom => self.prg_rom.len(),
            BankType::ChrRom => self.chr_rom.len(),
            BankType::Sram => self.sram.len(),
        }
    }
    fn get_bank<'a>(
        &self,
        base: usize,
        size: usize,
        kind: BankType,
    ) -> &'a [u8] {
        unsafe {
            &*((&(match kind {
                BankType::PrgRom => &self.prg_rom,
                BankType::ChrRom => &self.chr_rom,
                BankType::Sram => &self.sram,
            })[base..base + size]) as *const [u8])
        }
    }

    fn get_bank_mut<'a>(
        &mut self,
        base: usize,
        size: usize,
        kind: BankType,
    ) -> &'a mut [u8] {
        unsafe {
            &mut *((&mut (match kind {
                BankType::PrgRom => &mut self.prg_rom,
                BankType::ChrRom => &mut self.chr_rom,
                BankType::Sram => &mut self.sram,
            })[base..base + size]) as *mut [u8])
        }
    }

    fn get_mirror_type(&self) -> MirrorType {
        self.mirror_type
    }
    fn set_mirror_type(&mut self, mt: MirrorType) {
        self.mirror_type = mt
    }

    fn load(&mut self, reader: &mut dyn utils::Read) -> bool {
        self.load_sram(reader) &&
            SimpleCart::load_vec(&mut self.chr_rom, reader) &&
            utils::load_prefix(&mut self.mirror_type, 0, reader)
    }

    fn save(&self, writer: &mut dyn utils::Write) -> bool {
        self.save_sram(writer) &&
            SimpleCart::save_vec(&self.chr_rom, writer) &&
            utils::save_prefix(&self.mirror_type, 0, writer)
    }

    fn load_sram(&mut self, reader: &mut dyn utils::Read) -> bool {
        SimpleCart::load_vec(&mut self.sram, reader)
    }

    fn save_sram(&self, writer: &mut dyn utils::Write) -> bool {
        SimpleCart::save_vec(&self.sram, writer)
    }
}

// use the keyname that the pc_keyboard library generates to match against.
fn keyboard_mapping(key: &str) -> u8 {
    match key {
        I => stdctl::UP,
        K => stdctl::DOWN,
        J => stdctl::LEFT,
        L => stdctl::RIGHT,
        Z => stdctl::A,
        X => stdctl::B,
        Return => stdctl::START,
        S => stdctl::SELECT,
        Up => stdctl::UP,
        Down => stdctl::DOWN,
        Left => stdctl::LEFT,
        Right => stdctl::RIGHT,
        _ => 0,
    }
}

#[inline(always)]
fn get_rgb(color: u8) -> (u8, u8, u8) {
    let c = RGB_COLORS[color as usize];
    ((c >> 16) as u8, ((c >> 8) & 0xff) as u8, (c & 0xff) as u8)
}

struct CircularBuffer {
    buffer: [i16; (AUDIO_ALL_SAMPLES + 1) as usize],
    head: usize,
    tail: usize,
}

impl CircularBuffer {
    fn new() -> Self {
        CircularBuffer {
            buffer: [0; (AUDIO_ALL_SAMPLES + 1) as usize],
            head: 1,
            tail: 0,
        }
    }

    fn enque(&mut self, sample: i16) {
        self.buffer[self.tail] = sample;
        self.tail += 1;
        if self.tail == self.buffer.len() {
            self.tail = 0
        }
    }

    fn deque(&mut self) -> i16 {
        let res = self.buffer[self.head];
        if self.head != self.tail {
            let mut h = self.head + 1;
            if h == self.buffer.len() {
                h = 0
            }
            if h != self.tail {
                self.head = h
            } else {
                self.tail = self.head
            }
        }
        res
    }
}

// struct AudioSync {
//     time_barrier: Condvar,
//     buffer: Mutex<(CircularBuffer, u16)>,
// }

// struct SDLAudio<'a>(&'a AudioSync);
// struct SDLAudioPlayback<'a>(&'a AudioSync);

// impl<'a> sdl2::audio::AudioCallback for SDLAudioPlayback<'a> {
//     type Channel = i16;
//     fn callback(&mut self, out: &mut [i16]) {
//         let mut m = self.0.buffer.lock().unwrap();
//         {
//             let b = &mut m.0;
//             /*
//             let l1 = (b.tail + b.buffer.len() - b.head) % b.buffer.len();
//             print!("{} ", l1);
//             */
//             for x in out.iter_mut() {
//                 *x = b.deque()
//             }
//         }
//         //println!("{}", m.1);
//         if m.1 >= AUDIO_SAMPLES {
//             m.1 -= AUDIO_SAMPLES;
//             self.0.time_barrier.notify_one();
//         } else {
//             //println!("audio frame skipping {}", m.1);
//             m.1 = 0;
//         }
//     }
// }

//// TODO: make own audio structure, implement this similarly
// impl<'a> apu::Speaker for SDLAudio<'a> {
//     fn queue(&mut self, sample: i16) {
//         let mut m = self.0.buffer.lock().unwrap();
//         {
//             let b = &mut m.0;
//             b.enque(sample);
//         }
//         m.1 += 1;
//         while m.1 >= AUDIO_ALL_SAMPLES {
//             m = self.0.time_barrier.wait(m).unwrap();
//         }
//     }
// }

#[repr(C, packed)]
struct INesHeader {
    magic: [u8; 4],
    prg_rom_nbanks: u8,
    chr_rom_nbanks: u8,
    flags6: u8,
    flags7: u8,
    prg_ram_nbanks: u8,
    flags9: u8,
    flags10: u8,
    padding: [u8; 5],
}

// #[allow(dead_code)]
// fn print_cpu_trace(cpu: &mos6502::CPU) {
//     let pc = cpu.get_pc();
//     let mem = cpu.get_mem();
//     let opcode = mem.read_without_tick(pc) as usize;
//     let len = mos6502::INST_LENGTH[opcode];
//     let mut code = vec![0; len as usize];
//     for i in 0..len as u16 {
//         code[i as usize] = mem.read_without_tick(pc + i);
//     }
//     println!(
//         "0x{:04x} {} a:{:02x} x:{:02x} y:{:02x} s: {:02x} sp: {:02x}",
//         pc,
//         disasm::parse(opcode as u8, &code[1..]),
//         cpu.get_a(),
//         cpu.get_x(),
//         cpu.get_y(),
//         cpu.get_status(),
//         cpu.get_sp()
//     );
// }

pub fn run_rom() {
    println!("Booting NES...");

    //// it's actually really easy, this just inputs it at compile time.
    //// SLICK!
    // uses relative pathing
    // to change the rom, just change the path.
    // or just change the file, and always have a rom.nes at the root of the project, so that the script just has to replace the rom.nes file
    // and then recompile the OS.
    // what's easiest? this is important, this is the entire point of the project.
    let rom = include_bytes!("../../rom/smb.nes");

    // then slice the array for the header.
    let rheader = &rom[0..16];

    serial_println!("{:#?}", rheader);

    // transmute these bytes into a packed C struct.
    // slick! one line!
    let header = INesHeader {
        magic: [rheader[0], rheader[1], rheader[2], rheader[3]],
        prg_rom_nbanks: rheader[4],
        chr_rom_nbanks: rheader[5],
        flags6: rheader[6],
        flags7: rheader[7],
        prg_ram_nbanks: rheader[8],
        flags9: rheader[9],
        flags10: rheader[10],
        padding: [0; 5],
    };

    let mirror = match ((header.flags6 >> 2) & 2) | (header.flags6 & 1) {
        0 => MirrorType::Horizontal,
        1 => MirrorType::Vertical,
        2 => MirrorType::Single0,
        3 => MirrorType::Single1,
        _ => MirrorType::Four,
    };
    let mapper_id = (header.flags7 & 0xf0) | (header.flags6 >> 4);

    let magic = b"NES\x1a";
    println!("magic: {:#?}", header.magic);
    println!("desired magic: {:#?}", magic);
    if header.magic != magic.as_ref() {
        println!("Not an INES file, cannot run.");
        return;
    }

    println!(
        "prg size:{}, chr size:{}, mirror type:{}, mapper:{}",
        header.prg_rom_nbanks, header.chr_rom_nbanks, mirror as u8, mapper_id
    );
    if header.flags6 & 0x04 == 0x04 {
        // let mut trainer: [u8; 512] = [0; 512];
        // file.read(&mut trainer[..]).unwrap();
        // println!("skipping trainer");
    }

    let prg_len = header.prg_rom_nbanks as usize * 0x4000;
    let mut chr_len = header.chr_rom_nbanks as usize * 0x2000;
    if chr_len == 0 {
        chr_len = 0x2000;
    }

    let mut prg_rom = vec![0; prg_len];
    let mut chr_rom = vec![0; chr_len];
    let sram = vec![0; 0x2000];
    println!("zero initializing the SRAM");

    /* construct mapper from cartridge data */
    let cart = SimpleCart::new(chr_rom, prg_rom, sram, mirror);
    println!("constructing the cart");
    let mut m: Box<dyn mapper::Mapper> = match mapper_id {
        0 | 2 => Box::new(mapper::Mapper2::new(cart)),
        1 => Box::new(mapper::Mapper1::new(cart)),
        4 => Box::new(mapper::Mapper4::new(cart)),
        _ => panic!("unsupported mapper {}", mapper_id),
    };

    println!("constructing the devices");
    let keyboard = TerminalKeyboard {};

    // p1 controller init.
    // pass a pointer to a pollable object.
    // it'll just call the poll method and update the CPU controller MMIO with the u8 controller state byte.
    let p1ctl = stdctl::Joystick::new(&keyboard);

    /* setup the emulated machine */
    let mapper = mapper::RefMapper::new(&mut (*m) as &mut dyn mapper::Mapper);
    let mut cpu =
        mos6502::CPU::new(CPUMemory::new(&mapper, Some(&p1ctl), None));

    // need to pass the ppu anything that implements "Screen" in ppu.rs
    let mut win = construct::TerminalScreen::new();
    let mut ppu = ppu::PPU::new(PPUMemory::new(&mapper), &mut win);

    // same deal here. just fill the implementation.
    let mut spkr = construct::TerminalAudio {};
    let mut apu = APU::new(&mut spkr);

    let cpu_ptr = &mut cpu as *mut mos6502::CPU;

    println!("attaching the devices");
    cpu.mem.bus.attach(cpu_ptr, &mut ppu, &mut apu);

    //// tries to load the default_sram_name? what is this?
    //// this is probably just savestate stuff, which we don't need right now.
        // None => match File::open(&default_sram_name) {
    println!("powering up the initialized CPU.");
    cpu.powerup();

    loop {
        /* consume the leftover cycles from the last instruction */
        while cpu.cycle > 0 {
            cpu.mem.bus.tick()
        }

        println!("{}", cpu.get_sp());
        cpu.step();
    }
}
