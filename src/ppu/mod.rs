mod address;
mod mask;
mod control;
mod status;
mod scroll;

// PPU Memory Map
//  _______________  $FFFF
// | Mirrors       |      
// | $0000-$3FFF   |      
// |_ _ _ _ _ _ _ _| $4000
// | Palettes      |      
// |_ _ _ _ _ _ _ _| $3F00
// | Name Tabels   |      
// | (VRAM)        |      
// |_ _ _ _ _ _ _ _| $2000
// | Pattern Tables|      
// | (CHR ROM)     |      
// |_______________| $0000

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
    ctrl: control::ControlRegister,
    mask: mask::MaskRegister,
    addr: address::AddrRegister,
    stat: status::StatusRegister,
    scroll: scroll::ScrollRegister,
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
            ctrl: control::ControlRegister::new(),
            mask: mask::MaskRegister::new(),
            addr: address::AddrRegister::new(),
            stat: status::StatusRegister::new(),
            scroll: scroll::ScrollRegister::new(),
            internal_buf: 0,
        }
    }

    pub fn write_to_ctrl(&mut self, value: u8) {
        self.ctrl.update(value);
    }

    pub fn write_to_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    pub fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);
    }

    pub fn read_status(&mut self) -> u8 {
        let data = self.stat.snapshot();
        self.stat.clear_vblank_status();
        self.addr.reset_latch();
        self.scroll.reset_latch();
        data
    }

    fn inc_vram_addr(&mut self) {
        self.addr.inc(self.ctrl.inc_vram_addr());
    }

    pub fn read_data(&mut self) -> u8 {
        // temporary buffer used to keep the value
        // that is read during the previous read request
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

    // PPU memory address to VRAM index
    // Horizontal:
    //   [ A ] [ a ]
    //   [ B ] [ b ]
    // Vertical:
    //   [ A ] [ B ]
    //   [ a ] [ b ]
    pub fn mirror_vram_addr(&self, addr: u16) -> u16 {
        // mirror down 0x3000-0x3eff to 0x2000-0x2eff
        let mirrored_vram = addr & 0b10111111111111;
        let vram_index = mirrored_vram - 0x200;
        let name_table = vram_index / 0x400;
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }
}
