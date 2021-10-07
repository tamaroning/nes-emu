#![allow(dead_code)]
use cpu::AddressingMode;
use std::collections::HashMap;

pub struct Instruction {
    pub opcode: u8,
    pub mnemonic: &'static str,
    pub len: u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}

impl Instruction {
    fn new(opcode: u8, mnemonic: &'static str, len: u8, cycles: u8, mode: AddressingMode) -> Self {
        Instruction {
            opcode: opcode,
            mnemonic: mnemonic,
            len: len,
            cycles: cycles,
            mode: mode
        }
    } 
}

lazy_static! {
    pub static ref CPU_INSTRUCTIONS: Vec<Instruction> = vec![
        Instruction::new(0x00, "BRK", 1, 7, AddressingMode::Implied),
        Instruction::new(0xaa, "TAX", 1, 2, AddressingMode::Implied),
        Instruction::new(0x8a, "TXA", 1, 2, AddressingMode::Implied),
        Instruction::new(0xa8, "TAY", 1, 2, AddressingMode::Implied),
        Instruction::new(0x98, "TYA", 1, 2, AddressingMode::Implied),
        Instruction::new(0xba, "TSX", 1, 2, AddressingMode::Implied),
        Instruction::new(0x9a, "TXS", 1, 2, AddressingMode::Implied),

        Instruction::new(0xa9, "LDA", 2, 2, AddressingMode::Immediate),
        Instruction::new(0xa5, "LDA", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0xb5, "LDA", 2, 4, AddressingMode::ZeroPageX),
        Instruction::new(0xad, "LDA", 3, 4, AddressingMode::Absolute),
        Instruction::new(0xbd, "LDA", 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteX),
        Instruction::new(0xb9, "LDA", 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteY),
        Instruction::new(0xa1, "LDA", 2, 6, AddressingMode::IndirectX),
        Instruction::new(0xb1, "LDA", 2, 5/*+1 if page crossed*/, AddressingMode::IndirectY),

        Instruction::new(0xa2, "LDX", 2, 2, AddressingMode::Immediate),
        Instruction::new(0xa6, "LDX", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0xb6, "LDX", 2, 4, AddressingMode::ZeroPageY),
        Instruction::new(0xae, "LDX", 3, 4, AddressingMode::Absolute),
        Instruction::new(0xbe, "LDX", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteY),

        Instruction::new(0xa0, "LDY", 2, 2, AddressingMode::Immediate),
        Instruction::new(0xa4, "LDY", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0xb4, "LDY", 2, 4, AddressingMode::ZeroPageX),
        Instruction::new(0xac, "LDY", 3, 4, AddressingMode::Absolute),
        Instruction::new(0xbc, "LDY", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteX),

        Instruction::new(0x85, "STA", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0x95, "STA", 2, 4, AddressingMode::ZeroPageX),
        Instruction::new(0x8d, "STA", 3, 4, AddressingMode::Absolute),
        Instruction::new(0x9d, "STA", 3, 5, AddressingMode::AbsoluteX),
        Instruction::new(0x99, "STA", 3, 5, AddressingMode::AbsoluteY),
        Instruction::new(0x81, "STA", 2, 6, AddressingMode::IndirectX),
        Instruction::new(0x91, "STA", 2, 6, AddressingMode::IndirectY),

        Instruction::new(0x86, "STX", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0x96, "STX", 2, 4, AddressingMode::ZeroPageY),
        Instruction::new(0x8e, "STX", 3, 4, AddressingMode::Absolute),
        
        Instruction::new(0x84, "STY", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0x94, "STY", 2, 4, AddressingMode::ZeroPageX),
        Instruction::new(0x8c, "STY", 3, 4, AddressingMode::Absolute),

        Instruction::new(0x69, "ADC", 2, 2, AddressingMode::Immediate),
        Instruction::new(0x65, "ADC", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0x75, "ADC", 2, 4, AddressingMode::ZeroPageX),
        Instruction::new(0x6d, "ADC", 3, 4, AddressingMode::Absolute),
        Instruction::new(0x7d, "ADC", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteX),
        Instruction::new(0x79, "ADC", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteY),
        Instruction::new(0x61, "ADC", 2, 6, AddressingMode::IndirectX),
        Instruction::new(0x71, "ADC", 2, 5/*+1 if page crossed */, AddressingMode::IndirectY),

        Instruction::new(0x29, "AND", 2, 2, AddressingMode::Immediate),
        Instruction::new(0x25, "AND", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0x35, "AND", 2, 4, AddressingMode::ZeroPageX),
        Instruction::new(0x2d, "AND", 3, 4, AddressingMode::Absolute),
        Instruction::new(0x3d, "AND", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteX),
        Instruction::new(0x39, "AND", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteY),
        Instruction::new(0x21, "AND", 2, 6, AddressingMode::IndirectX),
        Instruction::new(0x31, "AND", 2, 5/*+1 if page crossed */, AddressingMode::IndirectY),

        Instruction::new(0xe9, "SBC", 2, 2, AddressingMode::Immediate),
        Instruction::new(0xe5, "SBC", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0xf5, "SBC", 2, 4, AddressingMode::ZeroPageX),
        Instruction::new(0xed, "SBC", 3, 4, AddressingMode::Absolute),
        Instruction::new(0xfd, "SBC", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteX),
        Instruction::new(0xf9, "SBC", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteY),
        Instruction::new(0xe1, "SBC", 2, 6, AddressingMode::IndirectX),
        Instruction::new(0xf1, "SBC", 2, 5/*+1 if page crossed */, AddressingMode::IndirectY),

        Instruction::new(0x0a, "ASL", 1, 2, AddressingMode::Implied), // accumulator
        Instruction::new(0x06, "ASL", 2, 5, AddressingMode::ZeroPage),
        Instruction::new(0x16, "ASL", 2, 6, AddressingMode::ZeroPageX),
        Instruction::new(0x0e, "ASL", 3, 6, AddressingMode::Absolute),
        Instruction::new(0x1e, "ASL", 3, 7, AddressingMode::AbsoluteX),

        Instruction::new(0x4a, "LSR", 1, 2, AddressingMode::Implied), // accumulator
        Instruction::new(0x46, "LSR", 2, 5, AddressingMode::ZeroPage),
        Instruction::new(0x56, "LSR", 2, 6, AddressingMode::ZeroPageX),
        Instruction::new(0x4e, "LSR", 3, 6, AddressingMode::Absolute),
        Instruction::new(0x5e, "LSR", 3, 7, AddressingMode::AbsoluteX),
        
        Instruction::new(0x24, "BIT", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0x2c, "BIT", 3, 4, AddressingMode::Absolute),

        Instruction::new(0xc9, "CMP", 2, 2, AddressingMode::Immediate),
        Instruction::new(0xc5, "CMP", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0xd5, "CMP", 2, 4, AddressingMode::ZeroPageX),
        Instruction::new(0xcd, "CMP", 3, 4, AddressingMode::Absolute),
        Instruction::new(0xdd, "CMP", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteX),
        Instruction::new(0xd9, "CMP", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteY),
        Instruction::new(0xc1, "CMP", 2, 6, AddressingMode::IndirectX),
        Instruction::new(0xd1, "CMP", 2, 5/*+1 if page crossed */, AddressingMode::IndirectY),

        Instruction::new(0xe0, "CPX", 2, 2, AddressingMode::Immediate),
        Instruction::new(0xe4, "CPX", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0xec, "CPX", 3, 4, AddressingMode::Absolute),

        Instruction::new(0xc0, "CPY", 2, 2, AddressingMode::Immediate),
        Instruction::new(0xc4, "CPY", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0xcc, "CPY", 3, 4, AddressingMode::Absolute),

        Instruction::new(0xc6, "DEC", 2, 5, AddressingMode::ZeroPage),
        Instruction::new(0xd6, "DEC", 2, 6, AddressingMode::ZeroPageX),
        Instruction::new(0xce, "DEC", 3, 6, AddressingMode::Absolute),
        Instruction::new(0xde, "DEC", 3, 7, AddressingMode::AbsoluteX),

        Instruction::new(0xca, "DEX", 1, 2, AddressingMode::Implied),
        Instruction::new(0x88, "DEY", 1, 2, AddressingMode::Implied),

        Instruction::new(0xe6, "INC", 2, 5, AddressingMode::ZeroPage),
        Instruction::new(0xf6, "INC", 2, 6, AddressingMode::ZeroPageX),
        Instruction::new(0xee, "INC", 3, 6, AddressingMode::Absolute),
        Instruction::new(0xfe, "INC", 3, 7, AddressingMode::AbsoluteX),

        Instruction::new(0xe8, "INX", 1, 2, AddressingMode::Implied),
        Instruction::new(0xc8, "INY", 1, 2, AddressingMode::Implied),

        Instruction::new(0x49, "EOR", 2, 2, AddressingMode::Immediate),
        Instruction::new(0x45, "EOR", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0x55, "EOR", 2, 4, AddressingMode::ZeroPageX),
        Instruction::new(0x4d, "EOR", 3, 4, AddressingMode::Absolute),
        Instruction::new(0x5d, "EOR", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteX),
        Instruction::new(0x59, "EOR", 3, 4/*+1 if page crossed */, AddressingMode::AbsoluteY),
        Instruction::new(0x41, "EOR", 2, 6, AddressingMode::IndirectX),
        Instruction::new(0x51, "EOR", 2, 5/*+1 if page crossed */, AddressingMode::IndirectY),

        Instruction::new(0x48, "PHA", 1, 2, AddressingMode::Implied),
        Instruction::new(0x68, "PLA", 1, 4, AddressingMode::Implied),
        Instruction::new(0x08, "PHP", 1, 3, AddressingMode::Implied),
        Instruction::new(0x28, "PLP", 1, 3, AddressingMode::Implied),

        Instruction::new(0x40, "RTI", 1, 6, AddressingMode::Implied),
        Instruction::new(0x60, "RTS", 1, 6, AddressingMode::Implied),

        Instruction::new(0x4c, "JMP", 3, 5, AddressingMode::Absolute),
        Instruction::new(0x20, "JSR", 3, 6, AddressingMode::Absolute),
    ];

    pub static ref INSTRUCTION_MAP: HashMap<u8, &'static Instruction> = {
        let mut map = HashMap::new();
        for cpu_inst in &*CPU_INSTRUCTIONS {
            map.insert(cpu_inst.opcode, cpu_inst);
        }
        map
    };
}
