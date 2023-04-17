use std::mem::size_of;

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

    pub fn read_sector_second(&self, fat_sector: u64, buffer: &mut [u8]) {
        self.read_sector(fat_sector, buffer, &self.second);
    }
}
