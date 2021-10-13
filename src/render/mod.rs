pub mod frame;
pub mod palette;

use ppu::Ppu;

// TODO: Use appropriate palette
pub fn render(ppu: &Ppu, frame: &mut frame::Frame) {
    // two nametables exist
    let bank = ppu.ctrl.bkgnd_pattern_addr();

    // TODO: just for now, lets use the first nametable
    for i in 0 .. 0x3c0 {
        let tile = ppu.vram[i] as u16;
        let tile_x = i % 32;
        let tile_y = i / 32;
        let tile = &ppu.chr_rom[
            (bank + tile * 16) as usize ..= (bank + tile * 16 + 15) as usize];

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            for x in (0..=7).rev() {
                let val = (1 & upper) << 1 | (1 & lower);
                upper = upper >> 1;
                lower = lower >> 1;
                // TODO: just for now
                let rgb = match val {
                    0 => palette::SYSTEM_PALLETE[0x01],
                    1 => palette::SYSTEM_PALLETE[0x23],
                    2 => palette::SYSTEM_PALLETE[0x27],
                    3 => palette::SYSTEM_PALLETE[0x30],
                    _ => panic!(),
                };
                frame.set_pixel(tile_x * 8 + x, tile_y * 8 + y, rgb);
            }
        }
    }
}
