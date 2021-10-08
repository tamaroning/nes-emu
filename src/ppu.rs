#[macro_use]
use bitflags::bitflags;

#[derive(Debug, PartialEq)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    FourScreen,
}

#[derive(Debug)]
pub struct Ppu {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; 32],
    pub vram: [u8; 2048],
    pub oam: [u8; 256],
    pub mirroring: Mirroring,
    pub ctrl: ControlRegister,
    addr: AddrRegister,
    internal_buf: u8,
}

impl Ppu {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        Ppu {
            chr_rom: chr_rom,
            palette_table: [0; 32],
            vram: [0; 2048],
            oam: [0; 256],
            mirroring: mirroring,
            ctrl: ControlRegister::new(),
            addr: AddrRegister::new(),
            internal_buf: 0,
        }
    }

    fn write_to_ctrl(&mut self, value: u8) {
        self.ctrl.update(value);
    }

    fn inc_vram_addr(&mut self) {
        self.addr.inc(self.ctrl.inc_vram_addr());
    }

    pub fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();
        self.inc_vram_addr();

        match addr {
            0x0000 ..= 0x1fff => {
                let res = self.internal_buf;
                self.internal_buf = self.chr_rom[addr as usize];
                res
            },
            0x2000 ..= 0x2fff => {
                let res = self.internal_buf;
                self.internal_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                res
            },
            0x3000 ..= 0x3eff => panic!("unexpected"),
            0x3f00 ..= 0x3fff => {
                self.palette_table[(addr - 0x3f00) as usize]
            }
            _ => panic!("unexpected"),
        }
    }

    pub fn mirror_vram_addr(&self, addr: u16) -> u16 {
        todo!();
    }
}

#[derive(Debug)]
pub struct AddrRegister {
    value: (u8, u8),
    hi_ptr: bool,
}

impl AddrRegister {
    pub fn new() -> Self {
        AddrRegister {
            value: (0, 0),
            hi_ptr: true,
        }
    }

    fn write_to_ppu_addr(&mut self, value: u8) {
        self.update(value);
    }

    fn set(&mut self, data: u16) {
        self.value.0 = (data >> 8) as u8;
        self.value.1 = (data & 0xff) as u8;
    }

    pub fn get(&self) -> u16 {
        ((self.value.0 as u16) << 8) | (self.value.1 as u16)
    }

    pub fn update(&mut self, data: u8) {
        if self.hi_ptr {
            self.value.0 = data;
        } else {
            self.value.1 = data;
        }
        if self.get() > 0x3fff {
            self.set(self.get() & 0b11111111111111);
        }
        self.hi_ptr = !self.hi_ptr;
    }

    pub fn inc(&mut self, by: u8) {
        let low = self.value.1;
        self.value.1 = self.value.1.wrapping_add(by);
        if low > self.value.1 {
            self.value.0 = self.value.0.wrapping_add(1);
        }
        if self.get() > 0x3fff {
            self.set(self.get() & 0b11111111111111);
        }
    }

    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }
}

bitflags! {

    // 7  bit  0
    // ---- ----
    // VPHB SINN
    // |||| ||||
    // |||| ||++- Base nametable address
    // |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    // |||| |+--- VRAM address increment per CPU read/write of PPUDATA
    // |||| |     (0: add 1, going across; 1: add 32, going down)
    // |||| +---- Sprite pattern table address for 8x8 sprites
    // ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
    // |||+------ Background pattern table address (0: $0000; 1: $1000)
    // ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels)
    // |+-------- PPU master/slave select
    // |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
    // +--------- Generate an NMI at the start of the
    //            vertical blanking interval (0: off; 1: on)
    pub struct ControlRegister: u8 {
        const NAMETABLE1              = 0b00000001;
        const NAMETABLE2              = 0b00000010;
        const VRAM_ADD_INCREMENT      = 0b00000100;
        const SPRITE_PATTERN_ADDR     = 0b00001000;
        const BACKROUND_PATTERN_ADDR  = 0b00010000;
        const SPRITE_SIZE             = 0b00100000;
        const MASTER_SLAVE_SELECT     = 0b01000000;
        const GENERATE_NMI            = 0b10000000;
    }
}

impl ControlRegister {
    fn new() -> Self {
        ControlRegister::from_bits_truncate(0b00000000)
    }

    pub fn inc_vram_addr(&self) -> u8 {
        if !self.contains(ControlRegister::VRAM_ADD_INCREMENT) {
            1
        } else {
            32
        }
    }

    pub fn update(&mut self, data: u8) {
        self.bits = data;
    }
}
