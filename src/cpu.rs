use std::collections::HashMap;
use instructions;

const STAT_NEGATIVE : u8 = 0b10000000;
const STAT_OVERFLOW : u8 = 0b01000000;
//const UNUSED      : u8 = 0b00100000;
const STAT_BRK      : u8 = 0b00010000;
const STAT_DECIMAL  : u8 = 0b00001000;
const STAT_INTERRUPT: u8 = 0b00000100;
const STAT_ZERO     : u8 = 0b00000010;
const STAT_CARRY    : u8 = 0b00000001;

pub struct Cpu {
    // general resgisters
    pub pc: u16,
    pub sp: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    // processor status
    /*
        bit 7: negative (MSB of A)
        bit 6: overflow
        bit 5: hardwired 1 (not used)
        bit 4: brk flag (6502 has two kinds of interrupt, BRK and IRQ)
        bit 3: decimal (if set 1, run in BCD mode)
        bit 2: interrupt flag (1: forbid interruption)
        bit 1: zero
        bit 0: carry
    */
    pub stat: u8,
    // 64 KiB memory
    memory: [u8; 0x10000],
}

#[derive(Debug)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndirectX,
    IndirectY,
    NoneAddressing,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            pc: 0,
            sp: 0,
            a: 0,
            x: 0,
            y: 0,
            stat: 0,
            memory: [0; 0x10000],
        }
    }

    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let low = self.mem_read(pos) as u16;
        let high = self.mem_read(pos + 1) as u16;
        (high << 8) |  low
    }

    pub fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }

    pub fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let high = (data >> 8) as u8;
        let low = (data & 0xFF) as u8;
        self.mem_write(pos, low);
        self.mem_write(pos + 1, high);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        // When inserted a new cartridge
        // CPU receives Reset interrupt
        self.reset();
        self.run();
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.stat = 0;
        // entry point stored at 0xFFFC
        self.pc = self.mem_read_u16(0xFFFC);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000);
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            &AddressingMode::Immediate => self.pc,
            &AddressingMode::ZeroPage => self.mem_read(self.pc) as u16,
            &AddressingMode::Absolute => self.mem_read_u16(self.pc) as u16,
            &AddressingMode::ZeroPageX => {
                let base = self.mem_read(self.pc);
                let addr = base.wrapping_add(self.x) as u16;
                addr
            },
            &AddressingMode::ZeroPageY => {
                let base = self.mem_read(self.pc);
                let addr = base.wrapping_add(self.y) as u16;
                addr
            },
            &AddressingMode::AbsoluteX => {
                let base = self.mem_read_u16(self.pc);
                let addr = base.wrapping_add(self.x as u16);
                addr
            },
            &AddressingMode::AbsoluteY => {
                let base = self.mem_read_u16(self.pc);
                let addr = base.wrapping_add(self.y as u16);
                addr
            },
            &AddressingMode::IndirectX => {
                let base = self.mem_read(self.pc);
                let ptr = base.wrapping_add(self.x);
                let low = self.mem_read(ptr as u16);
                let high = self.mem_read(ptr.wrapping_add(1) as u16);
                (high as u16) << 8 | (low as u16)
            },
            &AddressingMode::IndirectY => {
                let base = self.mem_read(self.pc);
                let ptr = base.wrapping_add(self.y);
                let low = self.mem_read(ptr as u16);
                let high = self.mem_read(ptr.wrapping_add(1) as u16);
                (high as u16) << 8 | (low as u16)
            },
            &AddressingMode::NoneAddressing => panic!(),
        }
    }

    pub fn run(&mut self) {
        let ref instructions: HashMap<u8, &'static instructions::Instruction> = *instructions::INSTRUCTION_MAP;
        
        loop {
            let opcode = self.mem_read(self.pc);
            self.pc += 1;

            println!("opcode: 0x{:X}", opcode);
            let cur_inst = instructions.get(&opcode).expect(&format!("opcode 0x{:X} is not recognized", opcode));

            match opcode {
                // BRK
                0x00 => return,
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => {
                    self.lda(&cur_inst.mode);
                },
                // LDX
                0xa2 => {
                    self.ldx(&cur_inst.mode);
                }
                // STA
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                    self.sta(&cur_inst.mode);
                },
                0xAA => self.tax(),
                0xe8 => self.inx(),
            
                _ => panic!("0x{:X} is not impremented", opcode),
            }

            // increment pc
            self.pc += (cur_inst.len - 1) as u16;
        }
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.a);
    } 

    fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.x = self.mem_read(addr);
        self.update_zero_and_negative_flags(self.x);
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.a = self.mem_read(addr);
        self.update_zero_and_negative_flags(self.a);
    }

    fn tax(&mut self) {
        self.x = self.a;
        self.update_zero_and_negative_flags(self.x);
    }

    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.x);
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.stat |= STAT_ZERO;
        } else {
            self.stat &= !STAT_ZERO;
        }

        if result & 0b1000_0000 != 0 {
            self.stat |= STAT_OVERFLOW;
        } else {
            self.stat &= !STAT_OVERFLOW;
        }
    }
}
