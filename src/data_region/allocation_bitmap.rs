use arbitrary_int::{u5, u7};
use bitbybit::bitfield;
use bytemuck::{Pod, Zeroable};

use super::EntryType;

pub struct AllocationBitmap {
    data: Vec<u8>,
}

impl AllocationBitmap {
    pub fn new(allocated_clusters: u64) -> Self {
        let bits_in_last_byte = allocated_clusters % 8;
        let full_bytes = ((allocated_clusters - bits_in_last_byte) / 8) as usize;
        let last_byte = (1 << bits_in_last_byte) - 1;

        let mut data = vec![0xFF; full_bytes];
        data.push(last_byte);

        Self { data }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.data.as_slice()
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
}

#[test]
fn allocation_bitmap() {
    let mut bitmap = AllocationBitmap::new(0);

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
    fn new(is_second_fat: bool, cluster_index: u32, cluster_count: u64) -> Self {
        Self {
            entry_type: EntryType::new_with_raw_value(0)
                .with_type_code(u5::new(1))
                .with_in_use(true), // 0x81
            bitmap_flags: BitmapFlags::new_with_raw_value(0).with_is_second_fat(is_second_fat),
            reserved: [0; 18],
            first_cluster: 2, // FAT index, 0 in heap
            data_length: cluster_count / 8,
        }
    }

    pub fn new_first_fat(cluster_index: u32, cluster_count: u64) -> Self {
        Self::new(false, cluster_index, cluster_count)
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}
