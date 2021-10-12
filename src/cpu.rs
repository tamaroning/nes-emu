#![allow(dead_code)]
use std::collections::HashMap;
use bitflags::bitflags;
use instructions;
use memory::Bus;
use memory::Mem;

bitflags!{
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
    pub struct StatFlags: u8 {
        const NEGATIVE  = 0b10000000;
        const OVERFLOW  = 0b01000000;
        const BREAK2    = 0b00100000;
        const BREAK     = 0b00010000;
        const DECIMAL   = 0b00001000;
        const INTERRUPT_DISABLE = 0b00000100;
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
    pub bus: Bus,
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
    Relative,
    Implied,
}

impl Mem for Cpu {
    fn mem_read(&mut self, addr: u16) -> u8 {
        self.bus.mem_read(addr)
    }

    fn mem_read_u16(&mut self, pos: u16) -> u16 {
        self.bus.mem_read_u16(pos)
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.bus.mem_write(addr, data);
    }

    fn mem_write_u16(&mut self, addr: u16, data: u16) {
        self.bus.mem_write_u16(addr, data)
    }

    fn read_prg_rom(&self, addr: u16) -> u8 {
        self.bus.read_prg_rom(addr)
    }
}

mod interrupt {
    #[derive(PartialEq, Eq)]
    pub enum InterruptType {
        NMI,
    }

    #[derive(PartialEq, Eq)]
    pub(super) struct Interrupt {
        pub(super) ty: InterruptType,
        pub(super) vector_addr: u16,
        pub(super) b_flag_mask: u8,
        pub(super) cpu_cycles: u8,
    }

    pub(super) const NMI: Interrupt = Interrupt {
        ty: InterruptType::NMI,
        vector_addr: 0xfffa,
        b_flag_mask: 0b00100000,
        cpu_cycles: 2,
    };
}

impl Cpu {
    pub fn new(bus: Bus) -> Self {
        Cpu {
            pc: 0,
            sp: STACK_RESET,
            a: 0,
            x: 0,
            y: 0,
            stat: StatFlags::from_bits_truncate(0b100100),
            bus: bus,
        }
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
        self.pc = self.mem_read_u16(0xFFFC);
        self.sp = STACK_RESET;
    }

    pub fn load(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.mem_write(0x8000 + i, program[i as usize]);
        }
    }

    fn interrupt(&mut self, interrupt: interrupt:: Interrupt) {
        self.stack_push_u16(self.pc);
        let mut stat = self.stat.clone();
        stat.set(StatFlags::BREAK, interrupt.b_flag_mask & 0b010000 == 1);
        stat.set(StatFlags::BREAK2, interrupt.b_flag_mask & 0b100000 == 1);
        self.stack_push(stat.bits);
        self.stat.insert(StatFlags::INTERRUPT_DISABLE);
        self.bus.tick(interrupt.cpu_cycles);
        self.pc = self.mem_read_u16(interrupt.vector_addr);
    }

    pub fn get_operand_address(&mut self, mode: &AddressingMode) -> u16 {
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
            &AddressingMode::Implied | &AddressingMode::Relative => panic!(),
        }
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F) 
    where F: FnMut(&mut Cpu) {
        let ref instructions: HashMap<u8, &'static instructions::Instruction> = *instructions::INSTRUCTION_MAP;
        
        loop {
            // check interruptions
            if let Some(_nmi) = self.bus.poll_nmi_status() {
                self.interrupt(interrupt::NMI);
            }
            callback(self);

            let opcode = self.mem_read(self.pc);
            self.pc += 1;
            let pc_to_operand = self.pc;

            // debug
            //println!("PC: {:04X} opcode: 0x{:X}", self.pc, opcode);
            let cur_inst = instructions.get(&opcode).expect(&format!("opcode 0x{:X} is not recognized", opcode));

            match opcode {
                // BRK
                // TODO: interruption
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
                // AND
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => {
                    self.and(&cur_inst.mode);
                },
                0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 => {
                    self.ora(&cur_inst.mode);
                }
                // ASL accumulator
                0x0a => {
                    self.asl_accumulator();
                },
                // ASL
                0x06 | 0x16 | 0x0e | 0x1e => {
                    self.asl(&cur_inst.mode);
                },
                // LSR accumulator
                0x4a => {
                    self.lsr_accumulator();
                },
                // LSR
                0x46 | 0x56 | 0x4e | 0x5e => {
                    self.lsr(&cur_inst.mode);
                },
                // ROL accumulator
                0x2a => self.rol_accumulator(),
                // ROL
                0x26 | 0x36 | 0x2e | 0x3e => {
                    self.rol(&cur_inst.mode);
                },
                // ROR accumulator
                0x6a => self.ror_accumulator(),
                // ROR
                0x66 | 0x76 | 0x6e | 0x7e => {
                    self.ror(&cur_inst.mode);
                },
                //BIT
                0x24 | 0x2c => {
                    self.bit(&cur_inst.mode);
                },
                // CMP
                0xc9 | 0xc5 | 0xd5 | 0xcd | 0xdd | 0xd9 | 0xc1 | 0xd1 => {
                    self.compare(&cur_inst.mode, self.a);
                },
                // CPX
                0xe0 | 0xe4 | 0xec => {
                    self.compare(&cur_inst.mode, self.x);
                },
                // CPY
                0xc0 | 0xc4 | 0xcc => {
                    self.compare(&cur_inst.mode, self.y);
                },
                // DEC
                0xc6 | 0xd6 | 0xce | 0xde => {
                    self.dec(&cur_inst.mode);
                },
                // DEX
                0xca => self.dex(),
                // DEY
                0x88 => self.dey(),
                // INC
                0xe6 | 0xf6 | 0xee | 0xfe => {
                    self.inc(&cur_inst.mode);
                },
                // INX
                0xe8 => self.inx(),
                // INY
                0xc8 => self.iny(),
                // EOR
                0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => {
                    self.eor(&cur_inst.mode);
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
                // JMP Indirect
                0x6c => {
                    let addr = self.mem_read_u16(self.pc);
                    let indirect_ref = if addr & 0x00ff == 0x00ff {
                        let low = self.mem_read(addr);
                        let high = self.mem_read(addr & 0xFF00);
                        (high as u16) << 8 | (low as u16)
                    } else {
                        self.mem_read_u16(addr)
                    };
                    self.pc = indirect_ref;
                }
                // JSR absolute
                0x20 => {
                    self.stack_push_u16(self.pc + 2 - 1);
                    let addr = self.mem_read_u16(self.pc);
                    self.pc = addr;
                },
                // BCC
                0x90 => self.branch(!self.stat.contains(StatFlags::CARRY)),
                // BCS
                0xb0 => self.branch(self.stat.contains(StatFlags::CARRY)),
                // BEQ
                0xf0 => self.branch(self.stat.contains(StatFlags::ZERO)),
                // BNE
                0xd0 => self.branch(!self.stat.contains(StatFlags::ZERO)),
                // BPL
                0x10 => self.branch(!self.stat.contains(StatFlags::NEGATIVE)),
                // BMI
                0x30 => self.branch(self.stat.contains(StatFlags::NEGATIVE)),
                // BVC
                0x50 => self.branch(!self.stat.contains(StatFlags::OVERFLOW)),
                // BVS
                0x70 => self.branch(self.stat.contains(StatFlags::OVERFLOW)),
                // CLC
                0x18 => self.stat.remove(StatFlags::CARRY),
                // SEC
                0x38 => self.stat.insert(StatFlags::CARRY),
                // CLI
                0x58 => self.stat.remove(StatFlags::INTERRUPT_DISABLE),
                // SEI
                0x78 => self.stat.insert(StatFlags::INTERRUPT_DISABLE),
                // CLV
                0xb8 => self.stat.remove(StatFlags::OVERFLOW),
                // CLD
                0xd8 => self.stat.remove(StatFlags::DECIMAL),
                // SED
                0xf8 => self.stat.insert(StatFlags::DECIMAL),
                // NOP
                0xea => (),

                /* Atari 6502 instructions (Unofficial) */

                // DCP
                0xc7 | 0xd7 | 0xcf | 0xdf | 0xdb | 0xd3 | 0xc3 => {
                    let addr = self.get_operand_address(&cur_inst.mode);
                    let mut data = self.mem_read(addr);
                    data = data.wrapping_sub(1);
                    self.mem_write(addr, data);
                    if data <= self.a {
                        self.set_carry();
                    }
                    self.update_zero_and_negative_flags(self.a.wrapping_sub(data));
                },
                // RLA
                0x27 | 0x37 | 0x2F | 0x3F | 0x3b | 0x33 | 0x23 => {
                    let data = self.rol(&cur_inst.mode);
                    self.and_with_a(data);
                },
                // SLO
                0x07 | 0x17 | 0x0F | 0x1f | 0x1b | 0x03 | 0x13 => {
                    let data = self.asl(&cur_inst.mode);
                    self.or_with_a(data);
                },
                // SRE
                0x47 | 0x57 | 0x4F | 0x5f | 0x5b | 0x43 | 0x53 => {
                    let data = self.lsr(&cur_inst.mode);
                    self.xor_with_a(data);
                },
                // SKB
                // TODO: should read memory?
                0x80 | 0x82 | 0x89 | 0xc2 | 0xe2 => (),
                // AXS
                0xcb => {
                    let addr = self.get_operand_address(&cur_inst.mode);
                    let data = self.mem_read(addr);
                    let and = self.x & self.a;
                    let res = and.wrapping_sub(data);
                    if data <= and {
                        self.set_carry();
                    }
                    self.update_zero_and_negative_flags(res);
                    self.x = res;
                },
                // ARR
                0x6b => {
                    let addr = self.get_operand_address(&cur_inst.mode);
                    let data = self.mem_read(addr);
                    self.and_with_a(data);
                    self.ror_accumulator();
                    // TODO: correct?
                    let res = self.a;
                    let bit_5 = (res >> 5) & 1;
                    let bit_6 = (res >> 6) & 1;
                    if bit_6 == 1 {
                        self.set_carry();
                    } else {
                        self.clear_carry();
                    }
                    if bit_5 ^ bit_6 == 1 {
                        self.set_overflow();
                    } else {
                        self.clear_overflow();
                    }
                    self.update_zero_and_negative_flags(res)
                },
                // SBC
                0xeb => {
                    let addr = self.get_operand_address(&cur_inst.mode);
                    let data = self.mem_read(addr);
                    self.sub_from_a(data);
                },
                // ANC
                0x0b | 0x2b => {
                    let addr = self.get_operand_address(&cur_inst.mode);
                    let data = self.mem_read(addr);
                    self.and_with_a(data);
                    if self.stat.contains(StatFlags::NEGATIVE) {
                        self.set_carry();
                    } else {
                        self.clear_carry();
                    }
                },
                // ALR
                0x4b => {
                    let addr = self.get_operand_address(&cur_inst.mode);
                    let data = self.mem_read(addr);
                    self.add_to_a(data);
                    self.lsr_accumulator();
                },
                // NOP (but do read memory)
                0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74 | 0xd4 | 0xf4 | 0x0c | 0x1c
                    | 0x3c | 0x5c | 0x7c | 0xdc | 0xfc => {
                    let addr = self.get_operand_address(&cur_inst.mode);
                    let _data = self.mem_read(addr);
                },
                // RRA
                0x67 | 0x77 | 0x6f | 0x7f | 0x7b | 0x63 | 0x73 => {
                    let data = self.ror(&cur_inst.mode);
                    self.add_to_a(data);
                },
                // ISB
                0xe7 | 0xf7 | 0xef | 0xff | 0xfb | 0xe3 | 0xf3 => {
                    let data = self.inc(&cur_inst.mode);
                    self.sub_from_a(data);
                },
                // NOP (do NOTHING)
                0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xb2 | 0xd2 | 0xf2 => (),
                // NOP
                0x1a | 0x3a | 0x5a | 0x7a | 0xda | 0xfa => (),
                // LAX
                0xa7 | 0xb7 | 0xaf | 0xbf | 0xa3 | 0xb3 => {
                    let addr = self.get_operand_address(&cur_inst.mode);
                    let data = self.mem_read(addr);
                    self.a = data;
                    self.x = self.a;
                },
                // SAX
                0x87 | 0x97 | 0x8f | 0x83 => {
                    let data = self.a & self.x;
                    let addr = self.get_operand_address(&cur_inst.mode);
                    self.mem_write(addr, data);
                },
                // LXA
                0xab => {
                    self.lda(&cur_inst.mode);
                    self.tax();
                },
                // XAA
                0x8b => {
                    self.a = self.x;
                    self.update_zero_and_negative_flags(self.a);
                    let addr = self.get_operand_address(&cur_inst.mode);
                    let data = self.mem_read(addr);
                    self.and_with_a(data);
                },
                /* LAS */
                0xbb => {
                    let addr = self.get_operand_address(&cur_inst.mode);
                    let mut data = self.mem_read(addr);
                    data = data & self.sp;
                    self.a = data;
                    self.x = data;
                    self.sp = data;
                    self.update_zero_and_negative_flags(data);
                },
                // TAS
                0x9b => {
                    let data = self.a & self.x;
                    self.sp = data;
                    let mem_address =
                        self.mem_read_u16(self.pc) + self.y as u16;

                    let data = ((mem_address >> 8) as u8 + 1) & self.sp;
                    self.mem_write(mem_address, data)
                }

                // AHX  Indirect Y
                0x93 => {
                    let pos: u8 = self.mem_read(self.pc);
                    let mem_address = self.mem_read_u16(pos as u16) + self.y as u16;
                    let data = self.a & self.x & (mem_address >> 8) as u8;
                    self.mem_write(mem_address, data)
                },
                // AHX Absolute Y
                0x9f => {
                    let mem_address = self.mem_read_u16(self.pc) + self.y as u16;
                    let data = self.a & self.x & (mem_address >> 8) as u8;
                    self.mem_write(mem_address, data)
                },
                /* SHX */
                0x9e => {
                    let mem_address = self.mem_read_u16(self.pc) + self.y as u16;
                    // TODO: if cross page boundry {
                    //     mem_address &= (self.x as u16) << 8;
                    // }
                    let data = self.x & ((mem_address >> 8) as u8 + 1);
                    self.mem_write(mem_address, data)
                },
                /* SHY */
                0x9c => {
                    let mem_address = self.mem_read_u16(self.pc) + self.x as u16;
                    let data = self.y & ((mem_address >> 8) as u8 + 1);
                    self.mem_write(mem_address, data)
                },
                //_ => panic!("0x{:X} is not impremented", opcode),
            }

            // notify PPU about ticks the current instruction took
            // TODO: support variable cycles isntructions (BNE etc.)
            self.bus.tick(cur_inst.cycles);

            // add up pc unless current instruction is jxx
            if pc_to_operand == self.pc {
                self.pc += (cur_inst.len - 1) as u16;
            }
        }
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

    fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let val = self.mem_read(addr);
        self.a = val & self.a;
        // TODO: necessary?
        self.update_zero_and_negative_flags(self.a);
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let val = self.mem_read(addr);
        self.a = val | self.a;
        // TODO: necessary?
        self.update_zero_and_negative_flags(self.a);
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.a = data ^ self.a;
        // TODO: necessay?
        self.update_zero_and_negative_flags(self.a);
    }

    fn tax(&mut self) {
        self.x = self.a;
        self.update_zero_and_negative_flags(self.x);
    }

    fn asl_accumulator(&mut self) {
        let mut data = self.a;
        if data >> 1 == 1 {
            self.set_carry();
        } else {
            self.clear_carry();
        }
        data = data << 1;
        self.a = data;
        // TODO: necessary?
        self.update_zero_and_negative_flags(self.a);
    }

    fn asl(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        if data >> 1 == 1 {
            self.set_carry();
        } else {
            self.clear_carry();
        }
        data = data << 1;
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }

    fn lsr_accumulator(&mut self) {
        let mut data = self.a;
        if data & 1 == 1 {
            self.set_carry();
        } else {
            self.clear_carry();
        }
        data = data >> 1;
        self.a = data;
        // TODO: necessary?
        self.update_zero_and_negative_flags(self.a);
    }

    fn lsr(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        if data & 1 == 1 {
            self.set_carry();
        } else {
            self.clear_carry();
        }
        data = data >> 1;
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }

    fn rol_accumulator(&mut self) {
        let mut data = self.a;
        let tmp_carry = self.stat.contains(StatFlags::CARRY);
        if data >> 7 == 1 {
            self.set_carry();
        } else {
            self.clear_carry();
        }
        data = data << 1;
        if tmp_carry {
            data = data | 1;
        }
        self.a = data;
        // TODO: necessary?
        self.update_zero_and_negative_flags(data);
    }

    fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        let tmp_carry = self.stat.contains(StatFlags::CARRY);
        if data >> 7 == 1 {
            self.set_carry();
        } else {
            self.clear_carry();
        }
        data = data << 1;
        if tmp_carry {
            data = data | 1;
        }
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }

    fn ror_accumulator(&mut self) {
        let mut data = self.a;
        let tmp_carry = self.stat.contains(StatFlags::CARRY);
        if data & 1 == 1 {
            self.set_carry();
        } else {
            self.clear_carry();
        }
        data = data >> 1;
        if tmp_carry {
            data = data | 0b1000_0000;
        }
        self.a = data;
        // TODO: necessary?
        self.update_zero_and_negative_flags(data);
    }

    fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        let tmp_carry = self.stat.contains(StatFlags::CARRY);
        if data & 1 == 1 {
            self.set_carry();
        } else {
            self.clear_carry();
        }
        data = data >> 1;
        if tmp_carry {
            data = data | 0b1000_0000;
        }
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        let and = self.a & data;
        if and == 0 {
            self.set_zero();
        } else {
            self.clear_zero();
        }
        self.stat.set(StatFlags::NEGATIVE, data & (1<<7) > 0);
        self.stat.set(StatFlags::OVERFLOW, data & (1<<6) > 0);
    }

    fn compare(&mut self, mode: &AddressingMode, with: u8) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        if data < with {
            self.set_carry();
        } else {
            self.clear_carry();
        }
        self.update_zero_and_negative_flags(with.wrapping_sub(data));
    }

    fn dec(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        data = data.wrapping_sub(1);
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
    }

    fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.x);
    }

    fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.y);
    }

    fn inc(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        data = data.wrapping_add(1);
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }

    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.x);
    }

    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.y);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let val = self.mem_read(addr);
        self.add_to_a((val as i8).wrapping_neg().wrapping_sub(1) as u8);
    }

    // ignore decimal mode
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

    fn branch(&mut self, cond: bool) {
        if cond {
            let rel = self.mem_read(self.pc) as i8;
            self.pc = self.pc.wrapping_add(1).wrapping_add(rel as u16);
        }
    }

    fn and_with_a(&mut self, data: u8) {
        self.a = data & self.a;
    }

    fn xor_with_a(&mut self, data: u8) {
        self.a = data ^ self.a;
    }

    fn or_with_a(&mut self, data: u8) {
        self.a = data | self.a;
    }

    fn sub_from_a(&mut self, data: u8) {
        self.add_to_a(((data as i8).wrapping_neg().wrapping_sub(1)) as u8)
    }

    fn set_zero(&mut self) {
        self.stat.insert(StatFlags::ZERO);
    }

    fn clear_zero(&mut self) {
        self.stat.remove(StatFlags::ZERO);
    }

    fn set_carry(&mut self) {
        self.stat.insert(StatFlags::CARRY);
    }

    fn clear_carry(&mut self) {
        self.stat.remove(StatFlags::CARRY);
    }

    fn set_overflow(&mut self) {
        self.stat.insert(StatFlags::OVERFLOW);
    }

    fn clear_overflow(&mut self) {
        self.stat.remove(StatFlags::OVERFLOW);
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.set_zero();
        } else {
            self.clear_zero();
        }

        if result & 0b1000_0000 != 0 {
            self.stat.insert(StatFlags::NEGATIVE)
        } else {
            self.stat.remove(StatFlags::NEGATIVE);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ines::test;
    use trace::trace;

    #[test]
    fn test_0xa9_lda_immidiate_load_data() {
        let mut rom = test::create_rom();
        let prg = vec![0xa9, 0x05, 0x00];
        for i in 0..prg.len() {
            rom.prg_rom[i] = prg[i];
        }
        let bus = Bus::new(rom);
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.pc = 0x8000;
        cpu.run();
        assert!(cpu.stat.bits() & 0b0000_0010 == 0b00);
        assert!(cpu.stat.bits() & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut rom = test::create_rom();
        let prg = vec![0xa9, 0x00, 0x00];
        for i in 0..prg.len() {
            rom.prg_rom[i] = prg[i];
        }
        let bus = Bus::new(rom);
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.pc = 0x8000;
        cpu.run();
        assert!(cpu.stat.contains(StatFlags::ZERO));
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut rom = test::create_rom();
        let prg = vec![0xa9, 0x0a, 0xaa, 0x00];
        for i in 0..prg.len() {
            rom.prg_rom[i] = prg[i];
        }
        let bus = Bus::new(rom);
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.pc = 0x8000;
        cpu.run();
        assert_eq!(cpu.x, 10)
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut rom = test::create_rom();
        let prg = vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00];
        for i in 0..prg.len() {
            rom.prg_rom[i] = prg[i];
        }
        let bus = Bus::new(rom);
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.pc = 0x8000;
        cpu.run();
        assert_eq!(cpu.x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let mut rom = test::create_rom();
        let prg = vec![0xa2, 0xff, 0xe8, 0xe8, 0x00];
        for i in 0..prg.len() {
            rom.prg_rom[i] = prg[i];
        }
        let bus = Bus::new(rom);
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.pc = 0x8000;
        cpu.run();
        assert_eq!(cpu.x, 1)
    }

    #[test]
    fn test_lda_from_memory() {
        let mut rom = test::create_rom();
        let prg = vec![0xa5, 0x10, 0x00];
        for i in 0..prg.len() {
            rom.prg_rom[i] = prg[i];
        }
        let bus = Bus::new(rom);
        let mut cpu = Cpu::new(bus);
        cpu.reset();
        cpu.pc = 0x8000;
        cpu.mem_write(0x10, 0x55);
        cpu.run();
        assert_eq!(cpu.a, 0x55);
    }
}

