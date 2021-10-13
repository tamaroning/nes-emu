mod address;
mod control;
mod mask;
mod scroll;
mod status;

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
    pub oam_data: [u8; 256],
    pub oam_addr: u8,
    pub mirroring: Mirroring,
    pub ctrl: control::ControlRegister,
    mask: mask::MaskRegister,
    addr: address::AddrRegister,
    stat: status::StatusRegister,
    scroll: scroll::ScrollRegister,
    internal_buf: u8,
    // manage tick
    scanline: u16,
    cycles: usize,
    pub nmi_interrupt: Option<u8>,
}

impl Ppu {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        Ppu {
            chr_rom: chr_rom,
            palette_table: [0; 32],
            vram: [0; 2048],
            oam_data: [0; 256],
            oam_addr: 0,
            mirroring: mirroring,
            ctrl: control::ControlRegister::new(),
            mask: mask::MaskRegister::new(),
            addr: address::AddrRegister::new(),
            stat: status::StatusRegister::new(),
            scroll: scroll::ScrollRegister::new(),
            internal_buf: 0,
            scanline: 0,
            cycles: 0,
            nmi_interrupt: None,
        }
    }

    pub fn write_to_ctrl(&mut self, value: u8) {
        let prev_nmi_status = self.ctrl.generate_vbalnk_nmi();
        self.ctrl.update(value);
        if !prev_nmi_status && self.ctrl.generate_vbalnk_nmi() && self.stat.is_in_vblank() {
            self.nmi_interrupt = Some(1);
        }
    }

    pub fn write_to_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    pub fn write_to_scroll(&mut self, value: u8) {
        self.scroll.write(value);
    }

    pub fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);
    }

    pub fn write_to_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    pub fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
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

    pub fn write_to_data(&mut self, value: u8) {
        let addr = self.addr.get();
        match addr {
            0..=0x1fff => panic!("Cannot write to character ROM"),
            0x2000..=0x2fff => {
                self.vram[self.mirror_vram_addr(addr) as usize] = value;
            }
            0x3000..=0x3eff => unimplemented!("Shouldn't write here"),
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let origin = addr - 0x10;
                self.palette_table[(origin - 0x3f00) as usize] = value;
            }
            0x3f00..=0x3fff => {
                self.palette_table[(addr - 0x3f00) as usize] = value;
            }
            _ => panic!("Unexpected accesss"),
        }
        self.inc_vram_addr();
    }

    pub fn read_data(&mut self) -> u8 {
        // temporary buffer used to keep the value
        // that is read during the previous read request
        let addr = self.addr.get();
        self.inc_vram_addr();

        match addr {
            0x0000..=0x1fff => {
                let res = self.internal_buf;
                self.internal_buf = self.chr_rom[addr as usize];
                res
            }
            0x2000..=0x2fff => {
                let res = self.internal_buf;
                self.internal_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                res
            }
            0x3000..=0x3eff => panic!("Unexpected access"),
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let origin = addr - 0x10;
                self.palette_table[(origin - 0x3f00) as usize]
            }
            0x3f00..=0x3fff => self.palette_table[(addr - 0x3f00) as usize],
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
        let vram_index = mirrored_vram - 0x2000;
        let name_table = vram_index / 0x400;
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    pub fn tick(&mut self, cycles: u8) -> bool {
        self.cycles += cycles as usize;
        if self.cycles >= 341 {
            self.cycles -= 341;
            self.scanline += 1;
            // must trigger NMI interruption and refresh screen
            // while scanline is in range 241 ~ 262
            if self.scanline == 241 {
                self.stat.set_vblank_status(true);
                self.stat.set_sprite_zero_hit(false);
                if self.ctrl.generate_vbalnk_nmi() {
                    self.nmi_interrupt = Some(1);
                }
            }
            if self.scanline >= 262 {
                self.scanline = 0;
                self.nmi_interrupt = None;
                self.stat.set_sprite_zero_hit(false);
                self.stat.clear_vblank_status();
                return true;
            }
        }
        return false;
    }

    pub fn new_empty_rom() -> Self {
        Ppu::new(vec![0; 2048], Mirroring::Horizontal)
    }
}


#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = Ppu::new_empty_rom();
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);
        ppu.write_to_data(0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = Ppu::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.addr.get(), 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }
}