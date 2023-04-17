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

    pub fn set_cluster(&mut self, cluster_index: u64, allocated: bool) {
        let index = (cluster_index / 8) as usize;

        let extend_by = (index + 1) - self.data.len();
        if extend_by > 0 {
            self.data.extend(vec![0; extend_by]);
        }

        let byte = self.data.get_mut(index).unwrap();
        if allocated {
            *byte |= 1 << (cluster_index % 8);
        } else {
            *byte &= !(1 << (cluster_index % 8));
        }
    }

    pub fn read_sector(&self, sector: u64, buffer: &mut [u8]) {
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
}

#[test]
fn allocation_bitmap() {
    let mut bitmap = AllocationBitmap::new(512);

    bitmap.set_cluster(1, true);
    bitmap.set_cluster(2, true);
    assert_eq!(&bitmap.data, &[6]);

    bitmap.set_cluster(2, false);
    assert_eq!(&bitmap.data, &[2]);
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
