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

fn sprite_palette(ppu: &Ppu, palette_idx: u8) -> [u8; 4] {
    let start = 0x11 + (palette_idx * 4) as usize;
    [0, ppu.palette_table[start], ppu.palette_table[start + 1], ppu.palette_table[start + 2]]
}

// TODO: Use appropriate palette
pub fn render(ppu: &Ppu, frame: &mut frame::Frame) {
    // draw background
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
    // draw sprites
    for i in (0..ppu.oam_data.len()).step_by(4).rev() {
        let tile_idx = ppu.oam_data[i + 1] as u16;
        let tile_x = ppu.oam_data[i + 3] as usize;
        let tile_y = ppu.oam_data[i] as usize;

        let flip_vertical = if ppu.oam_data[i + 2] >> 7 & 1 == 1 {
            true
        } else {
            false
        };
        let flip_horizontal = if ppu.oam_data[i + 2] >> 6 & 1 == 1 {
            true
        } else {
            false
        };
        let palette_idx = ppu.oam_data[i + 2] & 0b11;
        let sprite_palette = sprite_palette(ppu, palette_idx);
        let bank: u16 = ppu.ctrl.sprite_pattern_addr();
        let tile = &ppu.chr_rom[(bank + tile_idx * 16) as usize ..= (bank + tile_idx * 16 + 15) as usize];

        for y in 0 ..= 7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            'xloop: for x in (0..=7).rev() {
                let val = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match val {
                    0 => continue 'xloop,
                    1 => palette::SYSTEM_PALETTE[sprite_palette[1] as usize],
                    2 => palette::SYSTEM_PALETTE[sprite_palette[2] as usize],
                    3 => palette::SYSTEM_PALETTE[sprite_palette[3] as usize],
                    _ => panic!(),
                };
                match (flip_horizontal, flip_vertical) {
                    (false, false) => frame.set_pixel(tile_x + x, tile_y + y, rgb),
                    (true, false) => frame.set_pixel(tile_x + 7 - x, tile_y + y, rgb),
                    (false, true) => frame.set_pixel(tile_x + x, tile_y + 7 - y, rgb),
                    (true, true) => frame.set_pixel(tile_x + 7 - x, tile_y + 7 - y, rgb),
                }
            }
        }
    }
}
