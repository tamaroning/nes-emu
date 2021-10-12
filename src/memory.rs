use ines::Rom;
use ppu::Ppu;

const RAM: u16 = 0x0000;
const RAM_MIRROR_END: u16 = 0x1fff;
const PPU_REGISTERS_MIRROR_END: u16 = 0x3fff;
const PRG_ROM: u16 = 0x8000;
const PRG_ROM_END: u16 = 0xFFFF;

//  _______________ $10000  _______________
// | PRG-ROM       |       |               |
// | Upper Bank    |       |               |
// |_ _ _ _ _ _ _ _| $C000 | PRG-ROM       |
// | PRG-ROM       |       |               |
// | Lower Bank    |       |               |
// |_______________| $8000 |_______________|
// | SRAM          |       | SRAM          |
// |_______________| $6000 |_______________|
// | Expansion ROM |       | Expansion ROM |
// |_______________| $4020 |_______________|
// | I/O Registers |       |               |
// |_ _ _ _ _ _ _ _| $4000 |               |
// | Mirrors       |       | I/O Registers |
// | $2000-$2007   |       |               |
// |_ _ _ _ _ _ _ _| $2008 |               |
// | I/O Registers |       |               |
// |_______________| $2000 |_______________|
// | Mirrors       |       |               |
// | $0000-$07FF   |       |               |
// |_ _ _ _ _ _ _ _| $0800 |               |
// | RAM           |       | RAM           |
// |_ _ _ _ _ _ _ _| $0200 |               |
// | Stack         |       |               |
// |_ _ _ _ _ _ _ _| $0100 |               |
// | Zero Page     |       |               |
// |_______________| $0000 |_______________|

pub struct Bus {
    // 0x800 = 2048
    cpu_vram: [u8; 0x800],
    prg_rom: Vec<u8>,
    ppu: Ppu,
}

impl Bus {
    pub fn new(rom: Rom) -> Self {
        let ppu = Ppu::new(rom.chr_rom, rom.mirroring);
        Bus {
            cpu_vram: [0; 0x800],
            prg_rom: rom.prg_rom,
            ppu: ppu,
        }
    }
}

pub trait Mem {
    fn mem_read(&mut self, addr: u16) -> u8;
    fn mem_read_u16(&mut self, pos: u16) -> u16;
    fn mem_write(&mut self, addr: u16, data: u8);
    fn mem_write_u16(&mut self, addr: u16, data: u16);
    fn read_prg_rom(&self, addr: u16) -> u8;
}

impl Mem for Bus {
    fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            // 0x0000 ~ 0x1fff used as RAM
            RAM ..= RAM_MIRROR_END => {
                let lower_11_bits = addr & 0b00000111_11111111;
                self.cpu_vram[lower_11_bits as usize]
            },
            // write only
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 => {
                panic!("read write only memory");
            },
            0x2002 => self.ppu.read_status(),
            0x2004 => todo!("read PPU OAM data"),
            0x2007 => self.ppu.read_data(), 
            0x2008 ..= PPU_REGISTERS_MIRROR_END => {
                let mirrored = addr & 0b00100000_00000111;
                self.mem_read(mirrored)
            },
            PRG_ROM ..= PRG_ROM_END => self.read_prg_rom(addr),
            _ => panic!("Invalid memory access")
        }
    }
    
    fn mem_read_u16(&mut self, pos: u16) -> u16 {
        let low = self.mem_read(pos) as u16;
        let high = self.mem_read(pos + 1) as u16;
        (high << 8) |  low
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            // 0x0000 ~ 0x1fff used as RAM
            RAM ..= RAM_MIRROR_END => {
                let lower_11_bits = addr & 0b00000111_11111111;
                self.cpu_vram[lower_11_bits as usize] = data;
            },
            0x2000 => {
                self.ppu.write_to_ctrl(data);
            },
            0x2001 => {
                self.ppu.write_to_mask(data);
            },
            0x2002 => panic!("Cannot write to PPU status register"),
            0x2003 => {
                todo!("write to PPU OAM addr");
            },
            0x2004 => {
                todo!("write tp ")
            },
            0x2006 => {
                self.ppu.write_to_ppu_addr(data);
            },
            0x2007 => {
                todo!("write PPU data reg")
            },
            0x2008 ..= PPU_REGISTERS_MIRROR_END => {
                let mirrored = addr & 0b00100000_00000111;
                self.mem_write(mirrored, data);
            },
            0x8000 ..=0xffff => panic!("cannot write to program ROM"),
            _ => (), // TODO: should not ignore
        }
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let high = (data >> 8) as u8;
        let low = (data & 0xFF) as u8;
        self.mem_write(pos, low);
        self.mem_write(pos + 1, high);
    }

    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= PRG_ROM;
        if self.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            addr %= 0x4000;
        }
        self.prg_rom[addr as usize]
    }
}
