use boot_region::{BYTES_PER_SECTOR_SHIFT, BYTES_PER_SECTOR};

mod boot_region;

#[derive(Debug)]
pub enum ReadError {
    OutOfBounds
}

pub struct VirtualExFat {
    // boot sector
    volume_length: u64,
    cluster_heap_offset: u32,
    cluster_count: u32,
    volume_serial_number: u32,
    fat_offset: u32,
    fat_length: u32,
    sectors_per_cluster_shift: u8,
    number_of_fats: u8,
}

impl VirtualExFat {
    pub fn new() -> Self {
        Self {
            volume_length: 6144,
            cluster_heap_offset: 4096,
            cluster_count: 256,
            volume_serial_number: rand::random(),
            fat_offset: 2048,
            fat_length: 8,
            sectors_per_cluster_shift: 3, // 8 bytes
            number_of_fats: 1,
        }
    }

    /// `buffer` is assumed to be 512 bytes and zeroed
    pub fn read_sector(&self, sector: u32, buffer: &mut [u8]) -> Result<(), ReadError> {
        match sector {
            // main boot region
            0 => {
                // main boot sector
                let region: &mut boot_region::BootSector = bytemuck::from_bytes_mut(buffer);
                region.jump_boot = [0xEB, 0x76, 0x90];
                region.filesystem_name = [b'E', b'X', b'F', b'A', b'T', b' ', b' ', b' '];
                region.volume_length = self.volume_length;
                region.fat_offset = self.fat_offset;
                region.fat_length = self.fat_length;
                region.cluster_heap_offset = self.cluster_heap_offset;
                region.cluster_count = self.cluster_count;
                region.first_cluster_of_root_directory = 5;
                region.volume_serial_number = self.volume_serial_number;
                region.filesystem_revision = 256; // 1.00
                region.bytes_per_sector_shift = BYTES_PER_SECTOR_SHIFT;
                region.sectors_per_cluster_shift = self.sectors_per_cluster_shift;
                region.number_of_fats = self.number_of_fats;
                region.drive_select = 0x80;
                region.percent_in_use = 0;
                region.boot_signature = [0x55, 0xAA];

                Ok(())
            }
            1..=8 => {
                // main extended boot sectors
                buffer[510] = 0x55;
                buffer[511] = 0xAA;

                Ok(())
            }
            9 => {
                // main OEM parameters
                for byte in buffer.iter_mut().take(512) {
                    *byte = 0xFF;
                }

                Ok(())
            }
            10 => {
                // main reserved
                Ok(())
            }
            11 => {
                // main boot checksum
                let mut checksum = 0u32;

                for sector in 0..11 {
                    let mut buffer = [0; BYTES_PER_SECTOR as usize];
                    self.read_sector(sector, &mut buffer).unwrap();

                    for (index, byte) in buffer.iter().enumerate() {
                        // skip `volume_flags` and `percent_in_use`
                        if sector == 0 && (index == 106 || index == 107 || index == 112) {
                            continue;
                        }

                        checksum = (if checksum & 1 > 0 { 0x80000000 } else { 0 })
                            + (checksum >> 1)
                            + u32::from(*byte);
                    }
                }

                let buffer: &mut [u32] = bytemuck::cast_slice_mut(buffer);
                for four_bytes in buffer.iter_mut() {
                    *four_bytes = checksum;
                }

                Ok(())
            }

            // backup boot region
            12 => {
                // backup boot sector
                self.read_sector(0, buffer)
            }
            13..=20 => {
                // backup extended boot sectors
                self.read_sector(sector - 12, buffer)
            }
            21 => {
                // backup OEM parameters
                self.read_sector(9, buffer)
            }
            22 => {
                // backup reserved
                self.read_sector(10, buffer)
            }
            23 => {
                // backup boot checksum
                self.read_sector(11, buffer)
            }

            _ => {
                // FAT region

                // FAT alignment
                let fat_alignment_start_sector = 24;
                let fat_alignment_size = self.fat_offset - 24;
                let fat_alignment_end_sector = fat_alignment_start_sector + fat_alignment_size;
                if sector >= fat_alignment_start_sector && sector < fat_alignment_end_sector {
                    return Ok(())
                }

                // first FAT
                let first_fat_start_sector = self.fat_offset;
                let first_fat_size = self.fat_length;
                let first_fat_end_sector = first_fat_start_sector + first_fat_size;
                if sector >= first_fat_start_sector && sector < first_fat_end_sector {
                    return Ok(())
                }

                // second FAT
                if self.number_of_fats > 1 {
                    let second_fat_start_sector = self.fat_offset + self.fat_length;
                    let second_fat_size = self.fat_length * u32::from(self.number_of_fats - 1);
                    let second_fat_end_sector = second_fat_start_sector + second_fat_size;
                    if sector >= second_fat_start_sector && sector < second_fat_end_sector {
                        return Ok(())
                    }
                }

                // data region

                // cluster heap alignment
                let cluster_heap_alignment_start_sector = self.fat_offset + self.fat_length * u32::from(self.number_of_fats);
                let cluster_heap_alignment_size = self.cluster_heap_offset - (self.fat_offset + self.fat_length * u32::from(self.number_of_fats));
                let cluster_heap_alignment_end_sector = cluster_heap_alignment_start_sector + cluster_heap_alignment_size;
                if sector >= cluster_heap_alignment_start_sector && sector < cluster_heap_alignment_end_sector {
                    return Ok(())
                }

                // cluster heap
                let cluster_heap_start_sector = self.cluster_heap_offset;
                let cluster_heap_size = self.cluster_count * (1 << self.sectors_per_cluster_shift);
                let cluster_heap_end_sector = cluster_heap_start_sector + cluster_heap_size;
                if sector >= cluster_heap_start_sector && sector < cluster_heap_end_sector {
                    return Ok(())
                }

                // excess space
                let excess_space_start_sector = self.cluster_heap_offset + self.cluster_count * (1 << self.sectors_per_cluster_shift);
                let excess_space_size = self.volume_length - u64::from(self.cluster_heap_offset + self.cluster_count * (1 << self.sectors_per_cluster_shift));
                let excess_space_end_sector = excess_space_start_sector + excess_space_size as u32;
                if sector >= excess_space_start_sector && sector < excess_space_end_sector {
                    return Ok(())
                }

                dbg!(fat_alignment_start_sector);
                dbg!(fat_alignment_end_sector);
                dbg!(first_fat_start_sector);
                dbg!(first_fat_end_sector);
                dbg!(cluster_heap_alignment_start_sector);
                dbg!(cluster_heap_alignment_end_sector);

                dbg!(cluster_heap_start_sector);
                dbg!(cluster_heap_end_sector);

                dbg!(excess_space_start_sector);
                dbg!(excess_space_end_sector);

                Err(ReadError::OutOfBounds)
            }
        }
    }
}
