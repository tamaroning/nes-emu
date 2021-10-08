use ppu::Mirroring;
/*
    iNES 1.0 format is as follows
    - starts with 16 bytes NES header
    - optional 512 bytes trainer (ignore)
    - PRG ROM
    - CHR ROM

    note: not support iNES 2.0 format
*/

const PRG_ROM_PAGE_SIZE: usize = 16384;
const CHR_ROM_PAGE_SIZE: usize = 8192;

#[derive(Debug)]
pub struct Rom {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mapper: u8,
    pub mirroring: Mirroring,
}

impl Rom {
    pub fn analyze_raw(raw: &Vec<u8>) -> Result<Rom, &str>
    {
        // magic
        if &raw[0..4] != vec![0x4e, 0x45, 0x53, 0x1a] {
            return Err("Not iNES file format");
        }
        // mapper
        let mapper = (raw[7] & 0b1111_0000) | (raw[6] >> 4);
        // iNES version
        let ines_version = (raw[7] >> 2) & 0b11;
        if ines_version != 0 {
            return Err("Only iNES 1.0 is supported");
        }

        // mirroring type
        let is_four_screen = raw[6] & 0b1000 != 0;
        let is_vertical = raw[6] & 0b1 != 0;
        let mirroring = match (is_four_screen, is_vertical) {
            (true, _) => Mirroring::FourScreen,
            (false, true) => Mirroring::Vertical,
            (false, false) => Mirroring::Horizontal,
        };

        // PRG/CHR ROM size
        let prg_rom_size = raw[4] as usize * PRG_ROM_PAGE_SIZE;
        let chr_rom_size = raw[5] as usize * CHR_ROM_PAGE_SIZE;

        // trainer (used to run programs on different hardwares)
        let is_exist_trainer = raw[6] & 0b100 == 0;

        let prg_rom_begin = 16 + if is_exist_trainer {0} else {512};
        let chr_rom_begin = prg_rom_begin + prg_rom_size;

        Ok(Rom {
            prg_rom: raw[prg_rom_begin..(prg_rom_begin + prg_rom_size)].to_vec(),
            chr_rom: raw[chr_rom_begin..(chr_rom_begin + chr_rom_size)].to_vec(),
            mapper: mapper,
            mirroring: mirroring,
        })
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    struct TestRom {
        header: Vec<u8>,
        trainer: Option<Vec<u8>>,
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
    }

    fn create_raw(rom: TestRom) -> Vec<u8> {
        let mut res = Vec::with_capacity(
            rom.header.len()
                + rom.trainer.as_ref().map_or(0, |v| {v.len()})
                + rom.prg_rom.len()
                + rom.chr_rom.len(),
        );

        res.extend(&rom.header);
        if let Some(c) = rom.trainer {
            res.extend(c);
        }
        res.extend(&rom.prg_rom);
        res.extend(&rom.chr_rom);

        res
    }

    pub fn create_rom() -> Rom {
        let raw = create_raw(TestRom {
            header: vec![0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 00, 00, 00, 00, 00, 00, 00, 00, 00,],
            trainer: None,
            prg_rom: vec![1; 2 * PRG_ROM_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_ROM_PAGE_SIZE],
        });

        Rom::analyze_raw(&raw).unwrap()
    }

    #[test]
    fn test() {
        let raw = create_raw (TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 00, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            prg_rom: vec![1; 2 * PRG_ROM_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_ROM_PAGE_SIZE],
        });
        let rom = Rom::analyze_raw(&raw).unwrap();
        
        assert_eq!(rom.chr_rom, vec!(2; 1 * CHR_ROM_PAGE_SIZE));
        assert_eq!(rom.prg_rom, vec!(1; 2 * PRG_ROM_PAGE_SIZE));
        assert_eq!(rom.mapper, 3);
        assert_eq!(rom.mirroring, Mirroring::Vertical);
    }

}