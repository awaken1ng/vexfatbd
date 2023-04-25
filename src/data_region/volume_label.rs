use std::fmt::{Debug, Write};

use arbitrary_int::u5;
use bytemuck::{Pod, Zeroable};

use super::EntryType;

#[derive(Clone, Copy, Zeroable, Pod, PartialEq)]
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

impl Debug for VolumeLabelDirectoryEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VolumeLabelDirectoryEntry {{ ")?;

        let stripped = self.volume_label.into_iter().filter(|&ch| ch != 0);
        let volume = char::decode_utf16(stripped).map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER));
        for ch in volume {
            f.write_char(ch)?;
        }

        write!(f, " }}")?;

        Ok(())
    }
}
