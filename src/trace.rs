use std::collections::HashMap;
use cpu::Cpu;
use cpu::AddressingMode;
use memory::Mem;
use instructions;

pub fn trace(cpu: &mut Cpu) -> String {
    let ref insts: HashMap<u8, &'static instructions::Instruction> = *instructions::INSTRUCTION_MAP;
    let code = cpu.mem_read(cpu.pc);
    let cur_inst = insts.get(&code).unwrap();

    let inst_begin = cpu.pc;
    let mut hex_dump = vec![];
    hex_dump.push(code);

    let (mem_addr, stored_value) = match cur_inst.mode {
        AddressingMode::Immediate | AddressingMode::Implied | AddressingMode::Relative => (0,0),
        _ => {
            cpu.pc += 1;
            let addr = cpu.get_operand_address(&cur_inst.mode);
            cpu.pc -= 1;
            (addr, cpu.mem_read(addr))
        }
    };

    let tmp = match cur_inst.len {
        1 => match cur_inst.opcode {
            0x0a | 0x4a | 0x2a | 0x6a => format!("A "),
            _ => String::from(""),
        },
        2 => {
            let address: u8 = cpu.mem_read(inst_begin + 1);
            hex_dump.push(address);

            match cur_inst.mode {
                AddressingMode::Immediate => format!("#${:02x}", address),
                AddressingMode::ZeroPage => format!("${:02x} = {:02x}", mem_addr, stored_value),
                AddressingMode::ZeroPageX => format!(
                    "${:02x},X @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::ZeroPageY => format!(
                    "${:02x},Y @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::IndirectX => format!(
                    "(${:02x},X) @ {:02x} = {:04x} = {:02x}",
                    address,
                    (address.wrapping_add(cpu.x)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::IndirectY => format!(
                    "(${:02x}),Y = {:04x} @ {:04x} = {:02x}",
                    address,
                    (mem_addr.wrapping_sub(cpu.y as u16)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::Implied | AddressingMode::Relative => {
                    // assuming local jumps: BNE, BVS, etc....
                    let address: usize =
                        (inst_begin as usize + 2).wrapping_add((address as i8) as usize);
                    format!("${:04x}", address)
                }

                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 2. code {:02x}",
                    cur_inst.mode, cur_inst.opcode
                ),
            }
        },
        3 => {
            let address_low = cpu.mem_read(inst_begin + 1);
            let address_high = cpu.mem_read(inst_begin + 2);
            hex_dump.push(address_low);
            hex_dump.push(address_high);

            let address = cpu.mem_read_u16(inst_begin + 1);

            match cur_inst.mode {
                AddressingMode::Implied | AddressingMode::Relative => {
                    if cur_inst.opcode == 0x6c {
                        //jmp indirect
                        let jmp_addr = if address & 0x00FF == 0x00FF {
                            let lo = cpu.mem_read(address);
                            let hi = cpu.mem_read(address & 0xFF00);
                            (hi as u16) << 8 | (lo as u16)
                        } else {
                            cpu.mem_read_u16(address)
                        };

                        // let jmp_addr = cpu.mem_read_u16(address);
                        format!("(${:04x}) = {:04x}", address, jmp_addr)
                    } else {
                        format!("${:04x}", address)
                    }
                }
                AddressingMode::Absolute => format!("${:04x} = {:02x}", mem_addr, stored_value),
                AddressingMode::AbsoluteX => format!(
                    "${:04x},X @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::AbsoluteY => format!(
                    "${:04x},Y @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 3. code {:02x}",
                    cur_inst.mode, cur_inst.opcode
                ),
            }
        },
        _ => String::from(""),
    };

    let hex_str = hex_dump
        .iter()
        .map(|z| format!("{:02x}", z))
        .collect::<Vec<String>>()
        .join(" ");
    let asm_str = format!("{:04x}  {:8} {: >4} {}", inst_begin, hex_str, cur_inst.mnemonic, tmp)
        .trim()
        .to_string();

        format!(
            "{:47} A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x}",
            asm_str, cpu.a, cpu.x, cpu.y, cpu.stat, cpu.sp,
        )
        .to_ascii_uppercase()
}

#[cfg(test)]
mod test {
    use super::*;
    use memory::Bus;
    use ppu::Ppu;
    use ines::test;

    #[test]
    fn test_format_trace() {
        let mut bus = Bus::new(test::create_rom(), |ppu: &Ppu| {});
        bus.mem_write(100, 0xa2);
        bus.mem_write(101, 0x01);
        bus.mem_write(102, 0xca);
        bus.mem_write(103, 0x88);
        bus.mem_write(104, 0x00);

        let mut cpu = Cpu::new(bus);
        cpu.pc = 100;
        cpu.a = 1;
        cpu.x = 2;
        cpu.y = 3;
        let mut result: Vec<String> = vec![];
        cpu.run_with_callback(|cpu| {
            println!("{}", trace(cpu));
            result.push(trace(cpu));
        });
        assert_eq!(
            "0064  A2 01     LDX #$01                        A:01 X:02 Y:03 P:24 SP:FD",
            result[0]
        );
        assert_eq!(
            "0066  CA        DEX                             A:01 X:01 Y:03 P:24 SP:FD",
            result[1]
        );
        assert_eq!(
            "0067  88        DEY                             A:01 X:00 Y:03 P:26 SP:FD",
            result[2]
        );
    }

    #[test]
    fn test_format_mem_access() {
        let mut bus = Bus::new(test::create_rom(), |ppu: &Ppu| {});
        // ORA ($33), Y
        bus.mem_write(100, 0x11);
        bus.mem_write(101, 0x33);

        //data
        bus.mem_write(0x33, 00);
        bus.mem_write(0x34, 04);

        //target cell
        bus.mem_write(0x400, 0xAA);

        let mut cpu = Cpu::new(bus);
        cpu.pc = 0x64;
        cpu.y = 0;
        let mut result: Vec<String> = vec![];
        cpu.run_with_callback(|cpu| {
            result.push(trace(cpu));
        });
        assert_eq!(
            "0064  11 33     ORA ($33),Y = 0400 @ 0400 = AA  A:00 X:00 Y:00 P:24 SP:FD",
            result[0]
        );
    }
}