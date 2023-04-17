use std::collections::HashMap;
use std::mem::size_of;

use static_assertions::const_assert;

use crate::data_region::allocation_bitmap::{AllocationBitmap, AllocationBitmapDirectoryEntry};
use crate::data_region::upcase_table::{UpcaseTableDirectoryEntry, UPCASE_TABLE};
use crate::data_region::volume_label::VolumeLabelDirectoryEntry;
use crate::fat_region::FileAllocationTable;
use crate::utils::{unsigned_rounded_up_div, SliceChain};

pub enum DirectoryEntry {
    VolumeLabel(VolumeLabelDirectoryEntry),
    AllocationBitmap(AllocationBitmapDirectoryEntry),
    UpcaseTable(UpcaseTableDirectoryEntry),
}

impl DirectoryEntry {
    fn as_bytes(&self) -> &[u8] {
        match self {
            DirectoryEntry::VolumeLabel(entry) => entry.as_bytes(),
            DirectoryEntry::AllocationBitmap(entry) => entry.as_bytes(),
            DirectoryEntry::UpcaseTable(entry) => entry.as_bytes(),
        }
    }

    fn size(&self) -> usize {
        const SIZE: usize = 32;
        const_assert!(size_of::<DirectoryEntry>() - 8 == SIZE); // 8 - enum discriminant

        SIZE
    }
}

pub struct ClusterHeap {
    bytes_per_sector: u32,
    sectors_per_cluster: u32,

    pub fat: FileAllocationTable,

    allocation_bitmap: AllocationBitmap,
    allocation_bitmap_start_cluster: u64,
    allocation_bitmap_end_cluster: u64,

    upcase_table_start_cluster: u64,
    upcase_table_end_cluster: u64,

    heap: HashMap<u64, Cluster>,
}

impl ClusterHeap {
    pub fn new(bytes_per_sector: u32, sectors_per_cluster: u32, cluster_count: u32) -> Self {
        let bytes_per_cluster = u64::from(sectors_per_cluster * bytes_per_sector);

        let fat = FileAllocationTable::empty();

        let mut allocation_bitmap = AllocationBitmap::new(cluster_count);
        let allocation_bitmap_start_cluster: u32 = 0;
        let allocation_bitmap_size_clusters =
            unsigned_rounded_up_div(u64::from(allocation_bitmap.size()), bytes_per_cluster);
        let allocation_bitmap_end_cluster =
            u64::from(allocation_bitmap_start_cluster) + allocation_bitmap_size_clusters;

        let upcase_table_start_cluster = allocation_bitmap_end_cluster;
        let upcase_table_size_clusters =
            unsigned_rounded_up_div(2 * UPCASE_TABLE.len() as u64, bytes_per_cluster);
        let upcase_table_end_cluster = upcase_table_start_cluster + upcase_table_size_clusters;

        let mut heap = HashMap::new();
        heap.insert(
            upcase_table_end_cluster,
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

        dbg!(upcase_table_end_cluster);
        for i in 0..=upcase_table_end_cluster {
            allocation_bitmap.set_cluster(i, true);
        }

        Self {
            bytes_per_sector,
            sectors_per_cluster,

            fat,

            allocation_bitmap,
            allocation_bitmap_start_cluster: u64::from(allocation_bitmap_start_cluster),
            allocation_bitmap_end_cluster,

            upcase_table_start_cluster,
            upcase_table_end_cluster,

            heap,
        }
    }

    pub fn read_sector(&self, sector: u64, buffer: &mut [u8]) {
        let cluster_index = sector / u64::from(self.sectors_per_cluster);
        let sector_in_cluster = sector % u64::from(self.sectors_per_cluster);
        self.read_sector_in_cluster(cluster_index, sector_in_cluster, buffer);
    }

    /// `sector` is cluster relative index
    fn read_sector_in_cluster(&self, cluster: u64, sector: u64, buffer: &mut [u8]) {
        dbg!(cluster);
        if (cluster >= self.allocation_bitmap_start_cluster)
            && (cluster < self.allocation_bitmap_end_cluster)
        {
            println!("reading allocation bitmap");
            self.allocation_bitmap.read_sector(sector, buffer);
        } else if cluster >= self.upcase_table_start_cluster
            && cluster < self.upcase_table_end_cluster
        {
            println!("reading upcase table");
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
            println!("reading heap");
            cluster.read_sector(sector, buffer);
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
    fn read_sector(&self, sector: u64, buffer: &mut [u8]) {
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
    assert_eq!(buffer[0], 0x0F); // 4 clusters
    assert_eq!(&buffer[1..], [0; BYTES_PER_SECTOR - 1]);

    // upcase table
    let mut buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 0, &mut buffer);
    assert_eq!(
        buffer,
        bytemuck::cast_slice(&UPCASE_TABLE[..BYTES_PER_SECTOR / 2])
    );

    // first entry
    let mut buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_end_cluster, 0, &mut buffer);
    assert_eq!(&buffer[..32], VolumeLabelDirectoryEntry::empty().as_bytes());
}
