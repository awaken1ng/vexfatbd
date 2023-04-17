use crate::{
    data_region::volume_label::VolumeLabelDirectoryEntry,
    utils::{unsigned_align_to, unsigned_rounded_up_div},
};

mod boot_region;
pub(crate) mod data_region;
mod fat_region;
mod heap;
mod utils;

use heap::ClusterHeap;

#[derive(Debug)]
pub enum ReadError {
    OutOfBounds,
}

pub struct VirtualExFatBlockDevice {
    // boot sector
    volume_length: u64,
    fat_offset: u32,
    fat_length: u32,
    cluster_heap_offset: u32,
    cluster_count: u32,
    volume_serial_number: u32,
    bytes_per_sector_shift: u8,
    sectors_per_cluster_shift: u8,
    number_of_fats: u8,

    heap: ClusterHeap,
}

impl VirtualExFatBlockDevice {
    pub fn new(cluster_count: u32) -> Self {
        let bytes_per_sector_shift = 9; // 512 byte sectors
        let sectors_per_cluster_shift = 3; // 8 sectors, 4096 byte clusters
        let number_of_fats = 1u8;

        let min_fat_length =
            unsigned_rounded_up_div((cluster_count + 2) * 4, 1 << bytes_per_sector_shift);

        let fat_length = unsigned_align_to(min_fat_length, 1 << sectors_per_cluster_shift); // sectors
        let fat_offset = 24; // sectors, no alignment
        let cluster_heap_offset = fat_offset + fat_length; // sectors, no alignment
        let volume_length = u64::from(cluster_heap_offset)
            + (u64::from(cluster_count) * (1 << sectors_per_cluster_shift)); // sectors

        let min_volume_length = (1 << 20) / (1 << bytes_per_sector_shift);
        let min_fat_offset = 24;
        let min_cluster_heap_offset = fat_offset + (fat_length * u32::from(number_of_fats));

        assert!(volume_length >= min_volume_length);
        assert!(fat_offset >= min_fat_offset);
        assert!(fat_length >= min_fat_length);
        assert!(cluster_heap_offset >= min_cluster_heap_offset);

        let max_fat_offset = cluster_heap_offset - (fat_length * u32::from(number_of_fats));
        let max_fat_length = (cluster_heap_offset - fat_offset) / u32::from(number_of_fats);
        let max_cluster_heap_offset: u32 = u64::min(
            u64::from(u32::MAX),
            volume_length - (u64::from(cluster_count) * (1 << sectors_per_cluster_shift)),
        )
        .try_into()
        .unwrap();
        assert!(fat_offset <= max_fat_offset);
        assert!(fat_length <= max_fat_length);
        assert!(cluster_heap_offset <= max_cluster_heap_offset);

        Self {
            volume_length,
            cluster_heap_offset,
            cluster_count,
            volume_serial_number: rand::random(),
            fat_offset,
            fat_length,
            bytes_per_sector_shift,
            sectors_per_cluster_shift,
            number_of_fats,
            heap: ClusterHeap::new(
                1 << bytes_per_sector_shift,
                1 << sectors_per_cluster_shift,
                cluster_count,
            ),
        }
    }

    /// `buffer` is assumed to be zeroed
    pub fn read_sector(&self, sector: u64, buffer: &mut [u8]) -> Result<(), ReadError> {
        assert_eq!(buffer.len(), 1 << self.bytes_per_sector_shift);

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
                region.bytes_per_sector_shift = self.bytes_per_sector_shift;
                region.sectors_per_cluster_shift = self.sectors_per_cluster_shift;
                region.number_of_fats = self.number_of_fats;
                region.drive_select = 0x80;
                region.percent_in_use = 0xFF; // not available
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
                    let mut buffer = vec![0; 1 << self.bytes_per_sector_shift];
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
                let fat_alignment_size_sectors = u64::from(self.fat_offset) - 24;
                let fat_alignment_end_sector =
                    fat_alignment_start_sector + fat_alignment_size_sectors;
                if sector >= fat_alignment_start_sector && sector < fat_alignment_end_sector {
                    return Ok(());
                }

                // first FAT
                let first_fat_start_sector = u64::from(self.fat_offset);
                let first_fat_size_sectors = u64::from(self.fat_length);
                let first_fat_end_sector = first_fat_start_sector + first_fat_size_sectors;
                if sector >= first_fat_start_sector && sector < first_fat_end_sector {
                    let fat_sector = sector - first_fat_start_sector;
                    self.heap.fat.read_sector_first(fat_sector, buffer);
                    return Ok(());
                }

                // second FAT
                if self.number_of_fats > 1 {
                    let second_fat_start_sector =
                        u64::from(self.fat_offset) + u64::from(self.fat_length);
                    let second_fat_size_sectors =
                        u64::from(self.fat_length) * u64::from(self.number_of_fats - 1);
                    let second_fat_end_sector = second_fat_start_sector + second_fat_size_sectors;
                    if sector >= second_fat_start_sector && sector < second_fat_end_sector {
                        let _fat_sector = sector - second_fat_start_sector;
                        unimplemented!();
                    }
                }

                // data region

                // cluster heap alignment
                let cluster_heap_alignment_start_sector = u64::from(self.fat_offset)
                    + u64::from(self.fat_length) * u64::from(self.number_of_fats);
                let cluster_heap_alignment_size_sectors =
                    u64::from(self.cluster_heap_offset) - cluster_heap_alignment_start_sector;
                let cluster_heap_alignment_end_sector =
                    cluster_heap_alignment_start_sector + cluster_heap_alignment_size_sectors;
                if sector >= cluster_heap_alignment_start_sector
                    && sector < cluster_heap_alignment_end_sector
                {
                    return Ok(());
                }

                // cluster heap
                let cluster_heap_start_sector = u64::from(self.cluster_heap_offset);
                let cluster_heap_size_sectors =
                    u64::from(self.cluster_count) * (1 << self.sectors_per_cluster_shift);
                let cluster_heap_end_sector = cluster_heap_start_sector + cluster_heap_size_sectors;
                if sector >= cluster_heap_start_sector && sector < cluster_heap_end_sector {
                    let heap_sector = sector - cluster_heap_start_sector;
                    self.heap.read_sector(heap_sector, buffer);
                    return Ok(());
                }

                // excess space
                let excess_space_start_sector =
                    u64::from(self.cluster_heap_offset) + cluster_heap_size_sectors;
                let excess_space_size_sectors = self.volume_length - excess_space_start_sector;
                let excess_space_end_sector = excess_space_start_sector + excess_space_size_sectors;
                if sector >= excess_space_start_sector && sector < excess_space_end_sector {
                    return Ok(());
                }

                Err(ReadError::OutOfBounds)
            }
        }
    }
}

#[test]
fn read_sector() {
    // 4 KiB clusters, 4 TiB - 3 clusters (2 reserved by FAT, 1 used during rounding) volume
    let vexfat = VirtualExFatBlockDevice::new(1073741824 - 3);

    let mut buffer = [0; 512];
    vexfat
        .read_sector(vexfat.fat_offset.into(), &mut buffer)
        .unwrap();
    assert_eq!(&buffer[..8], &[248, 255, 255, 255, 255, 255, 255, 255]);
    assert_eq!(&buffer[8..512], &[0; 504]);

    let allocation_bitmap_size_sectors = 32770 * 8;
    buffer = [0; 512];
    vexfat
        .read_sector(
            u64::from(vexfat.cluster_heap_offset + allocation_bitmap_size_sectors),
            &mut buffer,
        )
        .unwrap();
    assert_eq!(&buffer[..32], VolumeLabelDirectoryEntry::empty().as_bytes());

    // 4 KiB clusters, 4 MiB volume
    let vexfat = VirtualExFatBlockDevice::new(512);

    let mut buffer = [0; 512];
    vexfat
        .read_sector(vexfat.fat_offset.into(), &mut buffer)
        .unwrap();
    assert_eq!(&buffer[..8], &[248, 255, 255, 255, 255, 255, 255, 255]);
    assert_eq!(&buffer[8..512], &[0; 504]);

    let mut buffer = [0; 512];
    vexfat
        .read_sector(vexfat.cluster_heap_offset.into(), &mut buffer)
        .unwrap();
    assert_eq!(buffer[0], 0x0F); // 4 clusters
    assert_eq!(&buffer[1..], [0; 511]);
}
