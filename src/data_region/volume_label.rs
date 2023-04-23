use arbitrary_int::u5;
use bytemuck::{Zeroable, Pod};

use super::EntryType;

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct VolumeLabelDirectoryEntry {
    entry_type: EntryType,
    pub character_count: u8,
    pub volume_label: [u16; 11],
    pub reserved: [u8; 8],
}

impl VolumeLabelDirectoryEntry {
    pub fn empty() -> Self {
        Self {
            entry_type: EntryType::new_with_raw_value(0)
                .with_type_code(u5::new(3))
                .with_in_use(true), // 0x83
            character_count: 0,
            volume_label: Default::default(),
            reserved: Default::default(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}
