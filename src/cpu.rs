#![allow(dead_code)]
use std::collections::HashMap;
use bitflags::bitflags;
use instructions;

/*
7  bit  0
---- ----
NVss DIZC
|||| ||||
|||| |||+- Carry
|||| ||+-- Zero
|||| |+--- Interrupt Disable
|||| +---- Decimal (no effect on NES)
||++------ No CPU effect, see: the B flag
|+-------- Overflow
+--------- Negative
*/
bitflags!{
    pub struct StatFlags: u8 {
        const NEGATIVE  = 0b10000000;
        const OVERFLOW  = 0b01000000;
        const BREAK2    = 0b00100000;
        const BREAK     = 0b00010000;
        const DECIMAL   = 0b00001000;
        const INTERRUPT = 0b00000100;
        const ZERO      = 0b00000010;
        const CARRY     = 0b00000001;
    }
}

const STACK_BASE: u16 = 0x0100;
const STACK_RESET: u8 = 0xfd;

pub struct Cpu {
    // general resgisters
    pub pc: u16,
    pub sp: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub stat: StatFlags,
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
    Implied,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            pc: 0,
            sp: STACK_RESET,
            a: 0,
            x: 0,
            y: 0,
            stat: StatFlags::from_bits_truncate(0b100100),
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
        self.stat = StatFlags::from_bits_truncate(0b100100);
        // entry point stored at 0xFFFC
        self.pc = self.mem_read_u16(0xFFFC);
        self.sp = STACK_RESET;
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
            &AddressingMode::Implied => panic!(),
        }
    }

    pub fn run(&mut self) {
        let ref instructions: HashMap<u8, &'static instructions::Instruction> = *instructions::INSTRUCTION_MAP;
        
        loop {
            let opcode = self.mem_read(self.pc);
            self.pc += 1;
            let pc_to_operand = self.pc;

            //println!("opcode: 0x{:X}", opcode);
            let cur_inst = instructions.get(&opcode).expect(&format!("opcode 0x{:X} is not recognized", opcode));

            match opcode {
                // BRK
                0x00 => return,
                // TAX
                0xAA => {
                    self.x = self.a;
                    self.update_zero_and_negative_flags(self.x);
                },
                // TXA
                0x8a => {
                    self.a = self.x;
                    self.update_zero_and_negative_flags(self.a);
                }
                // TAY
                0xa8 => {
                    self.y = self.a;
                    self.update_zero_and_negative_flags(self.y);
                },
                // TYA
                0x98 => {
                    self.a = self.y;
                    self.update_zero_and_negative_flags(self.a);
                },
                // TSX
                0xba => {
                    self.x = self.sp;
                    self.update_zero_and_negative_flags(self.x);
                },
                // TXS
                0x9a => {
                    self.sp = self.x;
                    self.update_zero_and_negative_flags(self.sp);
                },
                // INX
                0xe8 => self.inx(),
                // INY
                0xc8 => self.iny(),
                // LDA
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => {
                    self.lda(&cur_inst.mode);
                },
                // LDX
                0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe => {
                    self.ldx(&cur_inst.mode);
                },
                // LDY
                0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => {
                    self.ldy(&cur_inst.mode);
                }
                // STA
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                    self.sta(&cur_inst.mode);
                },
                // STX
                0x86 | 0x96 | 0x8e => {
                    self.stx(&cur_inst.mode);
                },
                // STY
                0x84 | 0x94 | 0x8c => {
                    self.sty(&cur_inst.mode)
                },
                // ADC
                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => {
                    self.adc(&cur_inst.mode);
                },
                // SBC
                0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => {
                    self.sbc(&cur_inst.mode);
                },
                // PHA
                0x48 => self.stack_push(self.a),
                // PLA
                0x68 => { self.a = self.stack_pop(); }
                // PHP
                0x08 => self.php(),
                // PLP
                0x28 => self.plp(),
                //RTI
                0x40 => {
                    self.stat.bits = self.stack_pop();
                    self.stat.remove(StatFlags::BREAK);
                    self.stat.insert(StatFlags::BREAK2);
                    self.pc = self.stack_pop_u16();
                }
                //RTS
                0x60 => {
                    self.pc = self.stack_pop_u16() + 1;
                }
                // JMP absolute
                0x4c => {
                    let addr = self.mem_read_u16(self.pc);
                    self.pc = addr;
                },
                // JSR absolute
                0x20 => {
                    self.stack_push_u16(self.pc + 2 - 1);
                    let addr = self.mem_read_u16(self.pc);
                    self.pc = addr;
                },

                _ => panic!("0x{:X} is not impremented", opcode),
            }

            // add up pc unless current instruction is jxx
            if pc_to_operand == self.pc {
                self.pc += (cur_inst.len - 1) as u16;
            }
        }
    }

    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.x);
    }

    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.y);
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.a = self.mem_read(addr);
        self.update_zero_and_negative_flags(self.a);
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.x = self.mem_read(addr);
        self.update_zero_and_negative_flags(self.x);
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.y = self.mem_read(addr);
        self.update_zero_and_negative_flags(self.y);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.a);
    }

    fn stx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.x);
    }

    fn sty(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.y);
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let val = self.mem_read(addr);
        self.add_to_a(val);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let val = self.mem_read(addr);
        self.add_to_a((val as i8).wrapping_neg().wrapping_sub(1) as u8);
    }

    // TODO: ignore decimal mode
    fn add_to_a(&mut self, data: u8) {
        let sum = self.a as u16 + data as u16
            + (if self.stat.contains(StatFlags::CARRY) {1} else {0});
        let res =  sum as u8;
        
        // update N V Z C
        let carry = sum > 0xff;
        if carry {
            self.stat.insert(StatFlags::CARRY);
        } else {
            self.stat.remove(StatFlags::CARRY);
        }
        // TODO: Is this correct?
        if (data ^ res) & (res ^ self.a) & 0x80 != 0 {
            self.stat.insert(StatFlags::OVERFLOW);
        } else {
            self.stat.remove(StatFlags::OVERFLOW);
        }
        self.update_zero_and_negative_flags(res);

        self.a = res;
    }

    fn php(&mut self) {
        let mut stat = self.stat.clone();
        stat.insert(StatFlags::BREAK);
        stat.insert(StatFlags::BREAK2);
        self.stack_push(stat.bits());
    }

    fn plp(&mut self) {
        self.stat.bits = self.stack_pop();
        self.stat.remove(StatFlags::BREAK);
        self.stat.remove(StatFlags::BREAK2);
    }

    fn stack_push(&mut self, data: u8) {
        self.mem_write((STACK_BASE as u16) + self.sp as u16, data);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn stack_push_u16(&mut self, data: u16) {
        let high = (data >> 8) as u8;
        let low = (data & 0xff) as u8;
        self.stack_push(high);
        self.stack_push(low);
    }

    fn stack_pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.mem_read((STACK_BASE as u16) + self.sp as u16)
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let low = self.stack_pop() as u16;
        let high = self.stack_pop() as u16;
        high << 8 | low
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.stat.insert(StatFlags::ZERO);
        } else {
            self.stat.remove(StatFlags::ZERO);
        }

        if result & 0b1000_0000 != 0 {
            self.stat.insert(StatFlags::NEGATIVE)
        } else {
            self.stat.remove(StatFlags::NEGATIVE);
        }
    }
}
