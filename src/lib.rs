use std::{
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use crate::utils::{unsigned_align_to, unsigned_rounded_up_div};

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
    pub first_cluster_of_root_directory: u32,
    volume_serial_number: u32,
    bytes_per_sector_shift: u8,
    sectors_per_cluster_shift: u8,
    number_of_fats: u8,

    heap: ClusterHeap,

    current_sector: u64,
    current_offset_in_sector: u64,
}

impl VirtualExFatBlockDevice {
    pub fn new(cluster_count: u32) -> Self {
        Self::new_with_serial_number(cluster_count, rand::random())
    }

    pub fn new_with_serial_number(cluster_count: u32, volume_serial_number: u32) -> Self {
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

        let heap = ClusterHeap::new(
            1 << bytes_per_sector_shift,
            1 << sectors_per_cluster_shift,
            cluster_count,
        );

        Self {
            volume_length,
            cluster_heap_offset,
            cluster_count,
            first_cluster_of_root_directory: heap.root_directory_cluster() + 2,
            volume_serial_number,
            fat_offset,
            fat_length,
            bytes_per_sector_shift,
            sectors_per_cluster_shift,
            number_of_fats,
            heap,
            current_sector: 0,
            current_offset_in_sector: 0,
        }
    }

    /// `buffer` is assumed to be zeroed
    pub fn read_sector(&mut self, sector_index: u64, buffer: &mut [u8]) -> Result<(), ReadError> {
        assert_eq!(buffer.len(), 1 << self.bytes_per_sector_shift);

        match sector_index {
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
                region.first_cluster_of_root_directory = self.first_cluster_of_root_directory;
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
                self.read_sector(sector_index - 12, buffer)
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
                if sector_index >= fat_alignment_start_sector
                    && sector_index < fat_alignment_end_sector
                {
                    return Ok(());
                }

                // first FAT
                let first_fat_start_sector = u64::from(self.fat_offset);
                let first_fat_size_sectors = u64::from(self.fat_length);
                let first_fat_end_sector = first_fat_start_sector + first_fat_size_sectors;
                if sector_index >= first_fat_start_sector && sector_index < first_fat_end_sector {
                    let fat_sector = sector_index - first_fat_start_sector;
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
                    if sector_index >= second_fat_start_sector
                        && sector_index < second_fat_end_sector
                    {
                        let _fat_sector = sector_index - second_fat_start_sector;
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
                if sector_index >= cluster_heap_alignment_start_sector
                    && sector_index < cluster_heap_alignment_end_sector
                {
                    return Ok(());
                }

                // cluster heap
                let cluster_heap_start_sector = u64::from(self.cluster_heap_offset);
                let cluster_heap_size_sectors =
                    u64::from(self.cluster_count) * (1 << self.sectors_per_cluster_shift);
                let cluster_heap_end_sector = cluster_heap_start_sector + cluster_heap_size_sectors;
                if sector_index >= cluster_heap_start_sector
                    && sector_index < cluster_heap_end_sector
                {
                    let heap_sector = (sector_index - cluster_heap_start_sector) as u32;
                    self.heap.read_sector(heap_sector, buffer);
                    return Ok(());
                }

                // excess space
                let excess_space_start_sector =
                    u64::from(self.cluster_heap_offset) + cluster_heap_size_sectors;
                let excess_space_size_sectors = self.volume_length - excess_space_start_sector;
                let excess_space_end_sector = excess_space_start_sector + excess_space_size_sectors;
                if sector_index >= excess_space_start_sector
                    && sector_index < excess_space_end_sector
                {
                    return Ok(());
                }

                Err(ReadError::OutOfBounds)
            }
        }
    }

    pub fn add_directory(&mut self, root_cluster: u32, file_name: &str) -> u32 {
        self.heap.add_directory(root_cluster, file_name)
    }

    pub fn add_file<P>(&mut self, first_cluster: u32, path: P) -> u32
    where
        P: AsRef<Path>,
    {
        self.heap.add_file(first_cluster, path)
    }
}

impl Seek for VirtualExFatBlockDevice {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match pos {
            SeekFrom::Start(offset) => {
                let bytes_per_sector = 1 << self.bytes_per_sector_shift;
                let whole_sectors = offset / bytes_per_sector;
                self.current_sector = whole_sectors;

                let whole_sectors_bytes = whole_sectors * bytes_per_sector;
                let partial_sector_bytes = offset - whole_sectors_bytes;
                self.current_offset_in_sector = partial_sector_bytes;

                Ok(offset)
            }
            SeekFrom::End(offset) => {
                let sector_count =
                    u64::from(self.cluster_count) * (1 << self.sectors_per_cluster_shift);
                let volume_size = (sector_count * (1 << self.bytes_per_sector_shift)) as i64;
                let absolute_offset: u64 = (volume_size - offset) as u64;

                self.seek(SeekFrom::Start(absolute_offset))
            }
            SeekFrom::Current(offset) => {
                let current_offset = ((self.current_sector * (1 << self.bytes_per_sector_shift))
                    + self.current_offset_in_sector) as i64;

                self.seek(SeekFrom::Start((current_offset + offset) as u64))
            }
        }
    }
}

impl Read for VirtualExFatBlockDevice {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let bytes_per_sector = 1 << self.bytes_per_sector_shift;
        let bytes_requested = buffer.len();
        let mut bytes_read = 0;
        let mut index = 0;

        loop {
            let mut sector = vec![0; bytes_per_sector];
            self.read_sector(self.current_sector, &mut sector).ok();

            let bytes_in_this_sector = bytes_per_sector - self.current_offset_in_sector as usize;
            let to_read = if bytes_requested >= bytes_in_this_sector {
                bytes_in_this_sector
            } else {
                bytes_requested
            };

            for byte in sector
                .into_iter()
                .skip(self.current_offset_in_sector as _)
                .take(to_read)
            {
                buffer[index] = byte;
                index += 1;
            }

            self.current_offset_in_sector += to_read as u64;

            let whole_sectors = self.current_offset_in_sector / bytes_per_sector as u64;
            self.current_sector += whole_sectors;
            self.current_offset_in_sector -= whole_sectors * bytes_per_sector as u64;

            bytes_read += to_read;
            if bytes_read >= bytes_requested {
                break;
            }
        }

        Ok(bytes_read)
    }
}

#[test]
fn read_sector() {
    use crate::data_region::volume_label::VolumeLabelDirectoryEntry;

    // 4 KiB clusters, 4 TiB - 3 clusters (2 reserved by FAT, 1 used during rounding) volume
    let mut vexfat = VirtualExFatBlockDevice::new(1073741824 - 3);

    let mut buffer = [0; 512];
    vexfat
        .read_sector(vexfat.fat_offset.into(), &mut buffer)
        .unwrap();
    assert_eq!(
        &buffer[..16],
        &[248, 255, 255, 255, 255, 255, 255, 255, 3, 0, 0, 0, 4, 0, 0, 0]
    );

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
    let mut vexfat = VirtualExFatBlockDevice::new(512);

    let mut buffer = [0; 512];
    vexfat
        .read_sector(vexfat.fat_offset.into(), &mut buffer)
        .unwrap();
    assert_eq!(
        &buffer[..24],
        &[
            248, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 4, 0, 0, 0, 255, 255, 255,
            255, 255, 255, 255, 255
        ]
    );
    assert_eq!(&buffer[24..512], &[0; 488]);

    let mut buffer = [0; 512];
    vexfat
        .read_sector(vexfat.cluster_heap_offset.into(), &mut buffer)
        .unwrap();
    assert_eq!(buffer[0], 0b00001111); // 4 clusters
    assert_eq!(&buffer[1..], [0; 511]);
}

#[test]
fn read() {
    let mut vexfat = VirtualExFatBlockDevice::new_with_serial_number(512, 0);

    let mut by_sector = Vec::new();
    let sectors = u64::from(vexfat.cluster_count * (1 << vexfat.sectors_per_cluster_shift));
    for sector in 0..sectors {
        let mut buffer = [0; 512];
        vexfat.read_sector(sector, &mut buffer).unwrap();
        by_sector.extend(buffer);
    }

    vexfat.seek(SeekFrom::Start(0)).unwrap();
    assert_eq!(vexfat.current_sector, 0);
    assert_eq!(vexfat.current_offset_in_sector, 0);

    let mut by_cluster = Vec::new();
    for _ in 0..vexfat.cluster_count {
        let mut buffer = [0; 4096];
        vexfat.read_exact(&mut buffer).unwrap();
        by_cluster.extend(buffer);
    }

    vexfat.seek(SeekFrom::Start(0)).unwrap();
    assert_eq!(vexfat.current_sector, 0);
    assert_eq!(vexfat.current_offset_in_sector, 0);
    let mut by_bytes = Vec::new();
    let volume_size = vexfat.cluster_count
        * (1 << vexfat.sectors_per_cluster_shift)
        * (1 << vexfat.bytes_per_sector_shift);
    for _ in 0..volume_size {
        let mut buffer = [0; 1];
        vexfat.read_exact(&mut buffer).unwrap();
        by_bytes.extend(buffer);
    }

    assert_eq!(by_sector.len(), by_cluster.len());
    assert_eq!(by_bytes.len(), by_bytes.len());
    assert!(by_sector == by_cluster);
    assert!(by_sector == by_bytes);
}
