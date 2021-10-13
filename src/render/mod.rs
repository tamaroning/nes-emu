pub mod frame;
pub mod palette;

use ppu::Ppu;

fn bg_palette(ppu: &Ppu, tile_column: usize, tile_row: usize) -> [u8; 4] {
    let attr_table_idx = tile_row / 4 * 8 + tile_column / 4;
    let attr_byte = ppu.vram[0x3c0 + attr_table_idx];

    let palette_idx = match (tile_column % 4 / 2, tile_row % 4 / 2) {
        (0, 0) => attr_byte & 0b11,
        (1,0) => (attr_byte >> 2) & 0b11,
        (0,1) => (attr_byte >> 4) & 0b11,
        (1,1) => (attr_byte >> 6) & 0b11,
        (_,_) => panic!(),
    };

    let palette_start: usize = 1 + (palette_idx as usize) * 4;
    [ppu.palette_table[0], ppu.palette_table[palette_start], ppu.palette_table[palette_start + 1], ppu.palette_table[palette_start + 2]]
}

// TODO: Use appropriate palette
pub fn render(ppu: &Ppu, frame: &mut frame::Frame) {
    // two nametables exist
    let bank = ppu.ctrl.bkgnd_pattern_addr();

    // TODO: just for now, lets use the first nametable
    for i in 0 .. 0x3c0 {
        let tile = ppu.vram[i] as u16;
        let tile_column = i % 32;
        let tile_row = i / 32;
        let tile = &ppu.chr_rom[
            (bank + tile * 16) as usize ..= (bank + tile * 16 + 15) as usize];
        let palette = bg_palette(ppu, tile_column, tile_row);
        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            for x in (0..=7).rev() {
                let val = (1 & upper) << 1 | (1 & lower);
                upper = upper >> 1;
                lower = lower >> 1;
                // TODO: just for now
                let rgb = match val {
                    0 => palette::SYSTEM_PALETTE[ppu.palette_table[0] as usize],
                    1 => palette::SYSTEM_PALETTE[palette[1] as usize],
                    2 => palette::SYSTEM_PALETTE[palette[2] as usize],
                    3 => palette::SYSTEM_PALETTE[palette[3] as usize],
                    _ => panic!(),
                };
                frame.set_pixel(tile_column * 8 + x, tile_row * 8 + y, rgb);
            }
        }
    }
}
