use arbitrary_int::{u5, u7};
use bitbybit::bitfield;
use bytemuck::{Pod, Zeroable};

use super::EntryType;

pub struct AllocationBitmap {
    cluster_count: u32,
    data: Vec<u8>,
}

impl AllocationBitmap {
    pub fn new(cluster_count: u32) -> Self {
        Self {
            cluster_count,
            data: Vec::new(),
        }
    }

    /// Size of the allocation bitmap in bytes
    pub fn size(&self) -> u32 {
        self.cluster_count / 8
    }

    fn set_cluster(&mut self, cluster_index: u32, allocated: bool) {
        let bitmap_index = (cluster_index / 8) as usize;

        let extend_by = (bitmap_index + 1) - self.data.len();
        if extend_by > 0 {
            self.data.extend(vec![0; extend_by]);
        }

        let byte = self.data.get_mut(bitmap_index).unwrap();
        if allocated {
            *byte |= 1 << (cluster_index % 8);
        } else {
            *byte &= !(1 << (cluster_index % 8));
        }
    }

    pub fn read_sector(&self, sector: u32, buffer: &mut [u8]) {
        let bytes_per_sector = buffer.len();
        let bytes_to_skip = sector as usize * bytes_per_sector;
        let sector_data = self
            .data
            .iter()
            .skip(bytes_to_skip)
            .take(bytes_per_sector)
            .cloned();

        for (out, byte) in buffer.iter_mut().zip(sector_data) {
            *out = byte;
        }
    }

    fn allocated_clusters_count(&self) -> u32 {
        let all_but_last_eight = self.data.len().saturating_sub(1) * 8;
        let last_eight = match self.data.last().cloned().unwrap_or_default() {
            0b11111111 => 8, // 8 clusters allocated
            0b01111111 => 7,
            0b00111111 => 6,
            0b00011111 => 5,
            0b00001111 => 4,
            0b00000111 => 3,
            0b00000011 => 2,
            0b00000001 => 1,
            0b00000000 => 0,
            _ => unreachable!(),
        };

        (all_but_last_eight + last_eight).try_into().unwrap()
    }

    pub fn allocate_next_cluster(&mut self) -> Option<u32> {
        let next_cluster = self.allocated_clusters_count();

        if next_cluster == self.cluster_count {
            None
        } else {
            self.set_cluster(next_cluster, true);
            Some(next_cluster)
        }
    }
}

#[test]
fn allocation_bitmap() {
    let mut bitmap = AllocationBitmap::new(512);

    bitmap.allocate_next_cluster();
    assert_eq!(&bitmap.data, &[0b00000001]);

    bitmap.allocate_next_cluster();
    bitmap.allocate_next_cluster();
    assert_eq!(&bitmap.data, &[0b00000111]);

    bitmap.allocate_next_cluster();
    bitmap.allocate_next_cluster();
    bitmap.allocate_next_cluster();
    bitmap.allocate_next_cluster();
    bitmap.allocate_next_cluster();
    assert_eq!(&bitmap.data, &[0b11111111]);

    bitmap.allocate_next_cluster();
    assert_eq!(&bitmap.data, &[0b11111111, 0b00000001]);
}

#[test]
fn out_of_memory() {
    let mut bitmap = AllocationBitmap::new(8);

    assert_eq!(bitmap.allocate_next_cluster(), Some(0));
    assert_eq!(bitmap.allocate_next_cluster(), Some(1));
    assert_eq!(bitmap.allocate_next_cluster(), Some(2));
    assert_eq!(bitmap.allocate_next_cluster(), Some(3));
    assert_eq!(bitmap.allocate_next_cluster(), Some(4));
    assert_eq!(bitmap.allocate_next_cluster(), Some(5));
    assert_eq!(bitmap.allocate_next_cluster(), Some(6));
    assert_eq!(bitmap.allocate_next_cluster(), Some(7));
    assert_eq!(bitmap.allocate_next_cluster(), None);
}

#[bitfield(u8)]
#[derive(Zeroable, Pod)]
struct BitmapFlags {
    #[bit(0, rw)]
    is_second_fat: bool,

    #[bits(1..=7, rw)]
    reserved: u7,
}

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct AllocationBitmapDirectoryEntry {
    entry_type: EntryType,
    bitmap_flags: BitmapFlags,
    reserved: [u8; 18],
    first_cluster: u32,
    data_length: u64,
}

impl AllocationBitmapDirectoryEntry {
    fn new(cluster_index: u32, cluster_count: u64, is_second_fat: bool) -> Self {
        Self {
            entry_type: EntryType::new_with_raw_value(0)
                .with_type_code(u5::new(1))
                .with_in_use(true), // 0x81
            bitmap_flags: BitmapFlags::new_with_raw_value(0).with_is_second_fat(is_second_fat),
            reserved: [0; 18],
            first_cluster: cluster_index + 2, // convert to FAT index
            data_length: cluster_count / 8,
        }
    }

    pub fn new_first_fat(cluster_index: u32, cluster_count: u64) -> Self {
        Self::new(cluster_index, cluster_count, false)
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}
