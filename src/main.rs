#![allow(dead_code)]
mod cpu;
mod instructions;
mod memory;
mod ines;
mod trace;
#[macro_use]
extern crate lazy_static;
extern crate bitflags;
extern crate sdl2;
use std::path::Path;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use memory::Mem;
use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::sys::exit;

/*
fn handle_user_input(cpu: &mut cpu::Cpu, event_pump: &mut EventPump) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                std::process::exit(0)
            },
            Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                cpu.mem_write(0xff, 0x77);
            },
            Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                cpu.mem_write(0xff, 0x73);
            },
            Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                cpu.mem_write(0xff, 0x61);
            },
            Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                cpu.mem_write(0xff, 0x64);
            }
            _ => (),
        }
    }
}

fn color(byte: u8) -> Color {
    match byte {
        // only 0, 1 are used
        0 => sdl2::pixels::Color::BLACK,
        1 => sdl2::pixels::Color::WHITE,
        2 | 9 => sdl2::pixels::Color::GREY,
        3 | 10 => sdl2::pixels::Color::RED,
        4 | 11 => sdl2::pixels::Color::GREEN,
        5 | 12 => sdl2::pixels::Color::BLUE,
        6 | 13 => sdl2::pixels::Color::MAGENTA,
        7 | 14 => sdl2::pixels::Color::YELLOW,
        _ => sdl2::pixels::Color::CYAN,
    }
}

fn read_screen_state(cpu: &cpu::Cpu, frame: &mut [u8; 32 * 3 * 32]) -> bool {
    let mut frame_idx = 0;
    let mut update = false;
    // 0x200~0x600 used to output graphic information
    for i in 0x0200..0x600 {
        // convert a bit in memory to (r, g, b)
        let color_idx = cpu.mem_read(i as u16);
        let (b1, b2, b3) = color(color_idx).rgb();
        // write on graphic memory
        if frame[frame_idx] != b1 || frame[frame_idx + 1] != b2 || frame[frame_idx + 2] != b3 {
            frame[frame_idx] = b1;
            frame[frame_idx + 1] = b2;
            frame[frame_idx + 2] = b3;
            update = true;
        }
        frame_idx += 3;
    }
    update
 }
*/

fn main() {
    println!("NES emulator");
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("usage: nes-emu <file path>");
        std::process::exit(0);
    }

    // init sdl2
    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();
    let window = video_subsys
        .window("test", 320, 320)
        .position_centered()
        .build().unwrap();
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(10.0, 10.0).unwrap();

    // create texture
    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 32, 32).unwrap();
    
    // open nes file
    let path = Path::new(args[1].as_str());
    let mut file = File::open(path)
        .unwrap();
    let mut raw: [u8; 0x20000] = [0; 0x20000];
    file.read(&mut raw).unwrap();
    let raw = raw.to_vec();
    
    // load program
    let mut rom = ines::Rom::analyze_raw(&raw).unwrap();
    let bus = memory::Bus::new(rom);
    let mut cpu = cpu::Cpu::new(bus);
    cpu.reset();
    cpu.pc = 0xc000;

    cpu.run_with_callback(move |cpu| {
        let opcode = cpu.mem_read(cpu.pc);
        println!("{:X}", opcode);
        println!("{}", trace::trace(cpu));
    });
    

}
