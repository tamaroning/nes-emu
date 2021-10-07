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

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.stat |= 0b0000_0010;
        } else {
            self.stat &= 0b1111_1101;
        }

        if result & 0b1000_0000 != 0 {
            self.stat |= 0b1000_0000;
        } else {
            self.stat &= 0b0111_1111;
        }
    }

    pub fn interpret(&mut self, program: Vec<u8>) {
        loop {
            let opcode = program[self.pc as usize];
            self.pc += 1;
            println!("opcode: 0x{:X}", opcode);

            match opcode {
                0x00 => { // BRK
                    return;
                }
                0xA9 => { // LDA imm
                    let imm = program[self.pc as usize];
                    self.pc += 1;
                    self.lda_imm(imm);
                },
                0xAA => self.tax(),
                0xE8 => self.inx(),
                _ => panic!("0x{:X} is not impremented", opcode),
            }
        }
    }

    fn lda_imm(&mut self, value: u8) {
        self.a = value;
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
}