use std::mem::size_of;

pub const END_OF_CHAIN: u32 = 0xFFFFFFFF - 2;

pub struct FileAllocationTable {
    first: Vec<u32>,
}

impl FileAllocationTable {
    pub fn empty() -> Self {
        Self {
            first: vec![0xFFFFFFF8, 0xFFFFFFFF],
        }
    }

    fn read_sector(&self, fat_sector: u64, buffer: &mut [u8], list: &[u32]) {
        let entries_per_sector = buffer.len() / size_of::<u32>();
        let buffer: &mut [u32] = bytemuck::cast_slice_mut(buffer);

        for (out, fat_entry) in buffer.iter_mut().zip(
            list.iter()
                .skip(fat_sector as usize * entries_per_sector)
                .take(entries_per_sector)
                .cloned(),
        ) {
            *out = fat_entry;
        }
    }

    pub fn read_sector_first(&self, fat_sector: u64, buffer: &mut [u8]) {
        self.read_sector(fat_sector, buffer, &self.first);
    }

    pub fn set_cluster(&mut self, cluster_index: u32, next_cluster: u32) {
        let fat_cluster_index = (cluster_index + 2) as usize;

        let extend_by = (fat_cluster_index + 1).saturating_sub(self.first.len());
        if extend_by > 0 {
            self.first.extend(vec![0; extend_by]);
        }

        self.first[fat_cluster_index] = next_cluster + 2;
    }
}

#[test]
fn set_cluster() {
    let mut fat = FileAllocationTable::empty();

    fat.set_cluster(0, END_OF_CHAIN);
    assert_eq!(fat.first, &[0xFFFFFFF8, 0xFFFFFFFF, 0xFFFFFFFF])
}
