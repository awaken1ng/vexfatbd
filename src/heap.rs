use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::mem::size_of;
use std::path::Path;

use itertools::Itertools;
use static_assertions::const_assert;

use crate::data_region::allocation_bitmap::{AllocationBitmap, AllocationBitmapDirectoryEntry};
use crate::data_region::file::{
    new_file, new_folder, FileDirectoryEntry, FileNameDirectoryEntry, StreamExtensionDirectoryEntry,
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
    lookup: HashMap<u32, u32>,
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
        let mut lookup = HashMap::new();
        heap.insert(
            root_directory_start_cluster,
            Cluster {
                data: ClusterData::DirectoryEntries(DirectoryEntries(vec![
                    DirectoryEntry::VolumeLabel(VolumeLabelDirectoryEntry::empty()),
                    DirectoryEntry::AllocationBitmap(
                        AllocationBitmapDirectoryEntry::new_first_fat(
                            allocation_bitmap_start_cluster,
                            u64::from(cluster_count),
                        ),
                    ),
                    DirectoryEntry::UpcaseTable(UpcaseTableDirectoryEntry::default()),
                ])),
            },
        );
        lookup.insert(root_directory_start_cluster, root_directory_start_cluster);

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
            lookup,
        }
    }

    pub fn read_sector(&mut self, sector: u32, buffer: &mut [u8]) {
        let cluster_index = sector / self.sectors_per_cluster;
        let sector_in_cluster = sector % self.sectors_per_cluster;
        self.read_sector_in_cluster(cluster_index, sector_in_cluster, buffer);
    }

    /// `sector` is cluster relative index
    fn read_sector_in_cluster(&mut self, cluster_index: u32, sector: u32, buffer: &mut [u8]) {
        if (cluster_index >= self.allocation_bitmap_start_cluster)
            && (cluster_index < self.allocation_bitmap_end_cluster)
        {
            let relative_cluster = cluster_index - self.allocation_bitmap_start_cluster;
            let bitmap_sector = (relative_cluster * self.sectors_per_cluster) + sector;
            self.allocation_bitmap.read_sector(bitmap_sector, buffer);
        } else if cluster_index >= self.upcase_table_start_cluster
            && cluster_index < self.upcase_table_end_cluster
        {
            let relative_cluster = cluster_index - self.upcase_table_start_cluster;
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
        } else if let Some(first_cluster) = self.lookup.get(&cluster_index).cloned() {
            let cluster = self.heap.get_mut(&first_cluster).unwrap();
            let sector = (cluster_index - first_cluster) * self.sectors_per_cluster + sector;
            match &mut cluster.data {
                ClusterData::DirectoryEntries(entries) => entries.read_sector(sector, buffer),
                ClusterData::FileMappedData(file) => {
                    file.read_sector(u64::from(sector) * u64::from(self.bytes_per_sector), buffer)
                }
            }
        }
    }

    pub(crate) fn root_directory_cluster(&self) -> u32 {
        self.upcase_table_end_cluster
    }

    fn insert_entries_into_cluster(
        &mut self,
        cluster_index: u32,
        entries: Vec<DirectoryEntry>,
    ) -> u32 {
        let cluster = self.heap.entry(cluster_index).or_insert(Cluster {
            data: ClusterData::DirectoryEntries(DirectoryEntries(Vec::new())),
        });
        let cluster_entries = match &mut cluster.data {
            ClusterData::DirectoryEntries(e) => e,
            _ => panic!(),
        };
        let entries_in_cluster_count = cluster_entries.0.len();

        let entries_in_current_cluster = {
            let cluster_size = self.sectors_per_cluster * self.bytes_per_sector;
            let max_entries_in_cluster = cluster_size / DirectoryEntry::SIZE as u32;
            let new_length = (entries_in_cluster_count + entries.len()) as u32;
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
            cluster_entries.0.push(entry);
        }
        self.lookup.insert(cluster_index, cluster_index);

        if entries.len() > 0 {
            assert!(entries.len() <= 128); // FIXME
            let next_cluster_index = self.allocation_bitmap.allocate_next_cluster();
            let next_cluster = Cluster {
                data: ClusterData::DirectoryEntries(DirectoryEntries(entries.collect())),
            };
            self.heap.insert(next_cluster_index, next_cluster);
            self.lookup.insert(next_cluster_index, cluster_index);
            next_cluster_index
        } else {
            cluster_index
        }
    }

    fn insert_file_into_heap(&mut self, first_data_cluster_index: u32, mut file: File) -> u32 {
        let file_size_bytes = file.seek(std::io::SeekFrom::End(0)).unwrap();
        let file_size_clusters = unsigned_rounded_up_div(
            file_size_bytes,
            u64::from(self.sectors_per_cluster) * u64::from(self.bytes_per_sector),
        );

        // allocate space for the file
        self.lookup
            .insert(first_data_cluster_index, first_data_cluster_index);
        for i in 0..file_size_clusters as u32 {
            self.lookup
                .insert(first_data_cluster_index + i, first_data_cluster_index);
            self.allocation_bitmap.allocate_next_cluster();
        }

        self.heap.insert(
            first_data_cluster_index,
            Cluster {
                data: ClusterData::FileMappedData(FileMappedData { file }),
            },
        );

        first_data_cluster_index + file_size_clusters as u32
    }

    pub fn add_directory(&mut self, root_cluster: u32, file_name: &str) -> u32 {
        let fat_cluster = root_cluster + 2;

        // empty directory is going to take up 1 cluster of space
        let cluster_size = self.sectors_per_cluster * self.bytes_per_sector;
        let entries = new_folder(file_name, fat_cluster + 1, u64::from(cluster_size)).unwrap();
        let end_cluster = self.insert_entries_into_cluster(root_cluster, entries);
        end_cluster + 1
    }

    pub fn add_file<P>(&mut self, first_cluster: u32, path: P) -> u32
    where
        P: AsRef<Path>,
    {
        let fat_cluster = first_cluster + 2;

        let entries = new_file(&path, fat_cluster + 1).unwrap();
        let entry_end_cluster = self.insert_entries_into_cluster(first_cluster, entries);

        let file = std::fs::File::open(path).unwrap();
        let data_end_cluster = self.insert_file_into_heap(entry_end_cluster + 1, file);
        data_end_cluster + 1
    }
}

struct DirectoryEntries(Vec<DirectoryEntry>);

impl DirectoryEntries {
    fn read_sector(&self, sector: u32, buffer: &mut [u8]) {
        let bytes_per_sector = buffer.len();
        let bytes_to_skip = sector as usize * bytes_per_sector;

        let slices = self.0.iter().map(|item| item.as_bytes().iter()).collect();
        let sector_data = SliceChain::new(slices)
            .skip(bytes_to_skip)
            .take(bytes_per_sector);
        for (buffer_byte, sector_byte) in buffer.iter_mut().zip(sector_data) {
            *buffer_byte = *sector_byte;
        }
    }
}

struct FileMappedData {
    file: File,
}

impl FileMappedData {
    fn read_sector(&mut self, offset: u64, buffer: &mut [u8]) {
        self.file
            .seek(std::io::SeekFrom::Start(offset))
            .unwrap();
        let _ = self.file.read(buffer).unwrap();
    }
}

enum ClusterData {
    DirectoryEntries(DirectoryEntries),
    FileMappedData(FileMappedData),
}

struct Cluster {
    data: ClusterData,
}

#[test]
fn heap_read() {
    const BYTES_PER_SECTOR: usize = 512;
    let mut heap = ClusterHeap::new(BYTES_PER_SECTOR as _, 8, 512);
    heap.add_directory(heap.root_directory_cluster(), "hello world");

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
