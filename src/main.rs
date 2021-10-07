mod cpu;

fn main() {
    println!("Hello NES emulator!");
}

#[cfg(test)]
mod test {
    use super::*;
    use cpu::Cpu;

    #[test]
    fn test_lda_imm() {
        let mut cpu = Cpu::new();
        cpu.interpret(vec![0xa9, 0x05, 0x00]);
        assert!(cpu.stat & 0b0000_0010 == 0b00);
        assert!(cpu.stat & 0b1000_0000 == 0);
    }

    #[test]
    fn test_lda_imm_zero() {
        let mut cpu = Cpu::new();
        cpu.interpret(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.stat & 0b0000_0010 == 0b10);
    }
}