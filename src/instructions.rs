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
        Instruction::new(0x00, "BRK", 1, 7, AddressingMode::NoneAddressing),
        Instruction::new(0xaa, "TAX", 1, 2, AddressingMode::NoneAddressing),
        Instruction::new(0xe8, "INX", 1, 2, AddressingMode::NoneAddressing),

        Instruction::new(0xa9, "LDA", 2, 2, AddressingMode::Immediate),
        Instruction::new(0xa5, "LDA", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0xb5, "LDA", 2, 4, AddressingMode::ZeroPageX),
        Instruction::new(0xad, "LDA", 3, 4, AddressingMode::Absolute),
        Instruction::new(0xbd, "LDA", 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteX),
        Instruction::new(0xb9, "LDA", 3, 4/*+1 if page crossed*/, AddressingMode::AbsoluteY),
        Instruction::new(0xa1, "LDA", 2, 6, AddressingMode::IndirectX),
        Instruction::new(0xb1, "LDA", 2, 5/*+1 if page crossed*/, AddressingMode::IndirectY),

        Instruction::new(0xa2, "LDX", 2, 2, AddressingMode::Immediate),

        Instruction::new(0x85, "STA", 2, 3, AddressingMode::ZeroPage),
        Instruction::new(0x95, "STA", 2, 4, AddressingMode::ZeroPageX),
        Instruction::new(0x8d, "STA", 3, 4, AddressingMode::Absolute),
        Instruction::new(0x9d, "STA", 3, 5, AddressingMode::AbsoluteX),
        Instruction::new(0x99, "STA", 3, 5, AddressingMode::AbsoluteY),
        Instruction::new(0x81, "STA", 2, 6, AddressingMode::IndirectX),
        Instruction::new(0x91, "STA", 2, 6, AddressingMode::IndirectY),
    ];

    pub static ref INSTRUCTION_MAP: HashMap<u8, &'static Instruction> = {
        let mut map = HashMap::new();
        for cpu_inst in &*CPU_INSTRUCTIONS {
            map.insert(cpu_inst.opcode, cpu_inst);
        }
        map
    };
}
