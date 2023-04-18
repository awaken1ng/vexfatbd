use std::collections::HashMap;
use std::mem::size_of;

use itertools::Itertools;
use static_assertions::const_assert;

use crate::data_region::allocation_bitmap::{AllocationBitmap, AllocationBitmapDirectoryEntry};
use crate::data_region::file::{
    new_folder, FileDirectoryEntry, FileNameDirectoryEntry, StreamExtensionDirectoryEntry,
};
use crate::data_region::upcase_table::{UpcaseTableDirectoryEntry, UPCASE_TABLE};
use crate::data_region::volume_label::VolumeLabelDirectoryEntry;
use crate::fat_region::{FileAllocationTable, END_OF_CHAIN};
use crate::utils::{unsigned_rounded_up_div, SliceChain};

pub enum DirectoryEntry {
    VolumeLabel(VolumeLabelDirectoryEntry),
    AllocationBitmap(AllocationBitmapDirectoryEntry),
    UpcaseTable(UpcaseTableDirectoryEntry),
    File(FileDirectoryEntry),
    StreamExtension(StreamExtensionDirectoryEntry),
    FileName(FileNameDirectoryEntry),
}

impl DirectoryEntry {
    const SIZE: usize = 32;

    fn as_bytes(&self) -> &[u8] {
        match self {
            DirectoryEntry::VolumeLabel(entry) => entry.as_bytes(),
            DirectoryEntry::AllocationBitmap(entry) => entry.as_bytes(),
            DirectoryEntry::UpcaseTable(entry) => entry.as_bytes(),
            DirectoryEntry::File(entry) => entry.as_bytes(),
            DirectoryEntry::StreamExtension(entry) => entry.as_bytes(),
            DirectoryEntry::FileName(entry) => entry.as_bytes(),
        }
    }
}

const_assert!(size_of::<DirectoryEntry>() - 8 == DirectoryEntry::SIZE); // 8 - enum discriminant

pub struct ClusterHeap {
    bytes_per_sector: u32,
    sectors_per_cluster: u32,

    pub fat: FileAllocationTable,

    allocation_bitmap: AllocationBitmap,
    allocation_bitmap_start_cluster: u32,
    allocation_bitmap_end_cluster: u32,

    upcase_table_start_cluster: u32,
    upcase_table_end_cluster: u32,

    heap: HashMap<u32, Cluster>,
}

impl ClusterHeap {
    pub fn new(bytes_per_sector: u32, sectors_per_cluster: u32, cluster_count: u32) -> Self {
        let bytes_per_cluster = sectors_per_cluster * bytes_per_sector;

        let mut allocation_bitmap = AllocationBitmap::new(cluster_count);
        let allocation_bitmap_start_cluster = 0;
        let allocation_bitmap_size_clusters =
            unsigned_rounded_up_div(allocation_bitmap.size(), bytes_per_cluster);
        let allocation_bitmap_end_cluster =
            allocation_bitmap_start_cluster + allocation_bitmap_size_clusters;

        let upcase_table_start_cluster = allocation_bitmap_end_cluster;
        let upcase_table_size_clusters =
            unsigned_rounded_up_div(2 * UPCASE_TABLE.len() as u32, bytes_per_cluster);
        let upcase_table_end_cluster = upcase_table_start_cluster + upcase_table_size_clusters;

        let root_directory_start_cluster = upcase_table_end_cluster;

        let mut heap = HashMap::new();
        heap.insert(
            root_directory_start_cluster,
            Cluster {
                data: vec![
                    ClusterData::DirectoryEntry(DirectoryEntry::VolumeLabel(
                        VolumeLabelDirectoryEntry::empty(),
                    )),
                    ClusterData::DirectoryEntry(DirectoryEntry::AllocationBitmap(
                        AllocationBitmapDirectoryEntry::new_first_fat(
                            allocation_bitmap_start_cluster,
                            u64::from(cluster_count),
                        ),
                    )),
                    ClusterData::DirectoryEntry(DirectoryEntry::UpcaseTable(
                        UpcaseTableDirectoryEntry::default(),
                    )),
                ],
            },
        );

        for _ in 0..=upcase_table_end_cluster {
            allocation_bitmap.allocate_next_cluster();
        }

        let mut fat = FileAllocationTable::empty();

        for (cluster, next_cluster) in
            (allocation_bitmap_start_cluster..allocation_bitmap_end_cluster).tuple_windows()
        {
            fat.set_cluster(cluster, next_cluster);
        }
        fat.set_cluster(allocation_bitmap_end_cluster - 1, END_OF_CHAIN);

        for (cluster, next_cluster) in
            (upcase_table_start_cluster..upcase_table_end_cluster).tuple_windows()
        {
            fat.set_cluster(cluster, next_cluster);
        }
        fat.set_cluster(upcase_table_end_cluster - 1, END_OF_CHAIN);

        fat.set_cluster(root_directory_start_cluster, END_OF_CHAIN);

        Self {
            bytes_per_sector,
            sectors_per_cluster,

            fat,

            allocation_bitmap,
            allocation_bitmap_start_cluster,
            allocation_bitmap_end_cluster,

            upcase_table_start_cluster,
            upcase_table_end_cluster,

            heap,
        }
    }

    pub fn read_sector(&self, sector: u32, buffer: &mut [u8]) {
        let cluster_index = sector / self.sectors_per_cluster;
        let sector_in_cluster = sector % self.sectors_per_cluster;
        self.read_sector_in_cluster(cluster_index, sector_in_cluster, buffer);
    }

    /// `sector` is cluster relative index
    fn read_sector_in_cluster(&self, cluster: u32, sector: u32, buffer: &mut [u8]) {
        if (cluster >= self.allocation_bitmap_start_cluster)
            && (cluster < self.allocation_bitmap_end_cluster)
        {
            let relative_cluster = cluster - self.allocation_bitmap_start_cluster;
            let bitmap_sector = (relative_cluster * self.sectors_per_cluster) + sector;
            self.allocation_bitmap.read_sector(bitmap_sector, buffer);
        } else if cluster >= self.upcase_table_start_cluster
            && cluster < self.upcase_table_end_cluster
        {
            let relative_cluster = cluster - self.upcase_table_start_cluster;
            let sector = (relative_cluster * self.sectors_per_cluster) + sector;

            let bytes_to_skip = sector as usize * self.bytes_per_sector as usize;
            let table: &[u8] = bytemuck::cast_slice(&UPCASE_TABLE);
            let sector_data = table
                .iter()
                .skip(bytes_to_skip)
                .take(self.bytes_per_sector as usize)
                .cloned();

            for (out, byte) in buffer.iter_mut().zip(sector_data) {
                *out = byte;
            }
        } else if let Some(cluster) = self.heap.get(&cluster) {
            cluster.read_sector(sector, buffer);
        }
    }

    pub(crate) fn root_directory_cluster(&self) -> u32 {
        self.upcase_table_end_cluster
    }

    pub fn add_directory(&mut self, root_cluster: u32, file_name: &str) {
        dbg!(root_cluster);
        let cluster_size = self.sectors_per_cluster * self.bytes_per_sector;
        let cluster = self.heap.get_mut(&root_cluster).unwrap();

        // empty directory is going to take up 1 cluster of space
        let entries = new_folder(file_name, root_cluster, u64::from(cluster_size)).unwrap();

        let entries_in_current_cluster = {
            let max_entries_in_cluster = cluster_size / DirectoryEntry::SIZE as u32;
            let new_length = (cluster.data.len() + entries.len()) as u32;
            if new_length > max_entries_in_cluster {
                let entries_in_next_cluster = (max_entries_in_cluster - new_length) as usize;

                entries.len() - entries_in_next_cluster
            } else {
                entries.len()
            }
        };

        let mut entries = entries.into_iter();
        for _ in 0..entries_in_current_cluster {
            let entry = entries.next().unwrap();
            cluster.data.push(ClusterData::DirectoryEntry(entry));
        }

        if entries.len() > 0 {
            let next_cluster_index = self.allocation_bitmap.allocate_next_cluster();
            let next_cluster = Cluster {
                data: entries.map(ClusterData::DirectoryEntry).collect(),
            };
            self.heap.insert(next_cluster_index, next_cluster);
        }
    }
}

#[test]
fn feature() {
    ClusterHeap::new(512, 8, 512);
}

enum ClusterData {
    DirectoryEntry(DirectoryEntry),
}

impl ClusterData {
    fn as_bytes(&self) -> &[u8] {
        match self {
            ClusterData::DirectoryEntry(e) => e.as_bytes(),
        }
    }
}

struct Cluster {
    data: Vec<ClusterData>,
}

impl Cluster {
    fn read_sector(&self, sector: u32, buffer: &mut [u8]) {
        let bytes_per_sector = buffer.len();
        let bytes_to_skip = sector as usize * bytes_per_sector;

        let slices = self
            .data
            .iter()
            .map(|item| item.as_bytes().iter())
            .collect();
        let sector_data = SliceChain::new(slices)
            .skip(bytes_to_skip)
            .take(bytes_per_sector);
        for (buffer_byte, sector_byte) in buffer.iter_mut().zip(sector_data) {
            *buffer_byte = *sector_byte;
        }
    }
}

#[test]
fn heap_read() {
    const BYTES_PER_SECTOR: usize = 512;
    let heap = ClusterHeap::new(BYTES_PER_SECTOR as _, 8, 512);

    // allocation bitmap
    let mut buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.allocation_bitmap_start_cluster, 0, &mut buffer);
    assert_eq!(buffer[0], 0b00001111); // 4 clusters
    assert_eq!(&buffer[1..], [0; BYTES_PER_SECTOR - 1]);

    // upcase table
    let mut buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 0, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[..256]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 1, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[256..512]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 2, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[512..768]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 3, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[768..1024]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 4, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[1024..1280]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 5, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[1280..1536]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 6, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[1536..1792]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 7, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[1792..2048]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster + 1, 0, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[2048..2304]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster + 1, 1, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[2304..2560]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster + 1, 2, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[2560..2816]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster + 1, 3, &mut buffer);
    assert_eq!(&buffer[..204], bytemuck::cast_slice(&UPCASE_TABLE[2816..]));
    assert_eq!(&buffer[204..], [0; 308]);

    // first entry
    let mut buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_end_cluster, 0, &mut buffer);
    assert_eq!(&buffer[..32], VolumeLabelDirectoryEntry::empty().as_bytes());
}
