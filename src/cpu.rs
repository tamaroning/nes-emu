pub struct Cpu {
    // general resgisters
    pub pc: u16,
    pub sp: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub stat: u8, // processor status
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
        }
    }

    pub fn interpret(&mut self, program: Vec<u8>) {
        loop {
            let opcode = program[self.pc as usize];
            self.pc += 1;
            println!("{:X}", opcode);

            match opcode {
                0x00 => { // BRK
                    return;
                }
                0xa9 => { // LDA imm
                    let param = program[self.pc as usize];
                    self.pc += 1;
                    self.a = param;

                    if self.a == 0 {
                        self.stat |= 0b0000_0010;
                    } else {
                        self.stat &= 0b1111_1101; 
                    }

                    if self.a & 0b1000_0000 != 0 {
                        self.stat |= 0b1000_0000;
                    } else {
                        self.stat &= 0b0111_1111;
                    }
                }
                _ => panic!("0x{:X} is not impremented", opcode),
            }
        }
    }
}