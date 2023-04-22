use std::io;

use arbitrary_int::{u10, u4, u5, u6, u7};
use bitbybit::bitfield;
use bytemuck::{Pod, Zeroable};

use super::{EntryType, GeneralPrimaryFlags};

#[bitfield(u16)]
#[derive(Zeroable, Pod)]
pub struct FileAttributes {
    #[bit(0, rw)]
    read_only: bool,

    #[bit(1, rw)]
    hidden: bool,

    #[bit(2, rw)]
    system: bool,

    #[bit(3, rw)]
    reserved_1: bool,

    #[bit(4, rw)]
    directory: bool,

    #[bit(5, rw)]
    archive: bool,

    #[bits(6..=15, rw)]
    reserved_2: u10,
}

#[bitfield(u32)]
#[derive(Zeroable, Pod)]
struct Timestamp {
    /// The `double_seconds` field shall describe the seconds portion of the Timestamp field, in two-second multiples.
    ///
    /// The valid range of values for this field shall be:
    /// - 0, which represents 0 seconds
    /// - 29, which represents 58 seconds
    #[bits(0..=4, rw)]
    double_seconds: u5,

    /// The `minute` field shall describe the minutes portion of the Timestamp field.
    ///
    /// The valid range of values for this field shall be:
    /// - 0, which represents 0 minutes
    /// - 59, which represents 59 minutes
    #[bits(5..=10, rw)]
    minute: u6,

    /// The `hour` field shall describe the hours portion of the Timestamp field.
    ///
    /// The valid range of values for this field shall be:
    /// - 0, which represents 00:00 hours
    /// - 23, which represents 23:00 hours
    #[bits(11..=15, rw)]
    hour: u5,

    /// The `day` field shall describe the day portion of the Timestamp field.
    ///
    /// The valid range of values for this field shall be:
    /// - 1, which is the first day of the given month
    /// - The last day of the given month (the given month defines the number of valid days)
    #[bits(16..=20, rw)]
    day: u5,

    /// The `month` field shall describe the month portion of the Timestamp field.
    ///
    /// The valid range of values for this field shall be:
    /// - At least 1, which represents January
    /// - At most 12, which represents December
    #[bits(21..=24, rw)]
    month: u4,

    /// The `year` field shall describe the year portion of the Timestamp field, relative to the year 1980.
    /// This field represents the year 1980 with the value 0 and the year 2107 with the value 127.
    ///
    /// All possible values for this field are valid.
    #[bits(25..=31, rw)]
    year: u7,
}

/// 10msIncrement fields shall provide additional time resolution to their corresponding Timestamp fields in ten-millisecond multiples.
///
/// The valid range of values for these fields shall be:
/// - At least 0, which represents 0 milliseconds
/// - At most 199, which represents 1990 milliseconds
#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
struct TenMsIncrement(u8);

#[bitfield(u8)]
#[derive(Zeroable, Pod)]
struct UtcOffset {
    #[bits(0..=6, rw)]
    offset_from_utc: u7,

    #[bit(7, rw)]
    offset_valid: bool,
}

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct FileDirectoryEntry {
    entry_type: EntryType,
    pub secondary_count: u8,
    pub set_checksum: u16,
    pub file_attributes: FileAttributes,
    reserved_1: u16,
    create_timestamp: u32,
    last_modified_timestamp: u32,
    last_accessed_timestamp: u32,
    create_10ms_increment: u8,
    last_modified_10ms_increment: u8,
    create_utc_offset: u8,
    last_modified_utc_offset: u8,
    last_accessed_utc_offset: u8,
    reserved_2: [u8; 7],
}

impl FileDirectoryEntry {
    pub fn new_file() -> Self {
        Self {
            entry_type: EntryType::new_with_raw_value(0)
                .with_type_code(u5::new(5))
                .with_in_use(true), // 0x85
            secondary_count: Default::default(),
            set_checksum: 0,
            file_attributes: FileAttributes::new_with_raw_value(0).with_read_only(true),
            reserved_1: 0,
            create_timestamp: 0,
            last_modified_timestamp: 0,
            last_accessed_timestamp: 0,
            create_10ms_increment: 0,
            last_modified_10ms_increment: 0,
            create_utc_offset: 0,
            last_modified_utc_offset: 0,
            last_accessed_utc_offset: 0,
            reserved_2: [0; 7],
        }
    }

    pub fn new_directory() -> Self {
        let mut ret = Self::new_file();
        ret.file_attributes = ret.file_attributes.with_directory(true);
        ret
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct StreamExtensionDirectoryEntry {
    entry_type: EntryType,
    pub general_secondary_flags: GeneralPrimaryFlags,
    reserved_1: u8,
    pub name_length: u8,
    pub name_hash: u16,
    reserved_2: u16,

    /// The `valid_data_length` field shall describe how far into the data stream user data has been written.
    /// Implementations shall update this field as they write data further out into the data stream.
    /// On the storage media, the data between the valid data length and the data length of the data stream is undefined.
    /// Implementations shall return zeroes for read operations beyond the valid data length.
    ///
    /// If the corresponding File directory entry describes a directory, then the only valid value for this field is equal to the value of the `data_length` field.
    /// Otherwise, the range of valid values for this field shall be:
    /// - At least 0, which means no user data has been written out to the data stream
    /// - At most `data_length`, which means user data has been written out to the entire length of the data stream
    pub valid_data_length: u64,

    reserved_3: u32,

    /// The FirstCluster field shall contain the index of the first cluster of an allocation in the Cluster Heap associated with the given directory entry.
    ///
    /// The valid range of values for this field shall be:
    /// - Exactly 0, which means no cluster allocation exists
    /// - Between 2 and ClusterCount + 1, which is the range of valid cluster indices
    ///
    /// Structures which derive from this template may redefine both the `first_cluster` and `data_length` fields,
    /// if a cluster allocation is not compatible with the derivative structure.
    ///
    /// If the `no_fat_chain` bit is 1 then `first_cluster` must point to a valid cluster in the cluster heap.
    ///
    /// This field shall contain the index of the first cluster of the data stream, which hosts the user data.
    pub first_cluster: u32, // FAT index

    /// The `data_length` field shall conform to the definition provided in the Generic Secondary DirectoryEntry template (see Section 6.4.4).
    ///
    /// If the corresponding File directory entry describes a directory, then the valid value for this field is the entire size of the associated allocation, in bytes, which may be 0.
    /// Further, for directories, the maximum value for this field is 256MB.
    pub data_length: u64,
}

impl StreamExtensionDirectoryEntry {
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

impl Default for StreamExtensionDirectoryEntry {
    fn default() -> Self {
        Self {
            entry_type: EntryType::new_with_raw_value(0)
                .with_type_category(true)
                .with_in_use(true), // 0xC0
            general_secondary_flags: GeneralPrimaryFlags::new_with_raw_value(0)
                .with_allocation_possible(true)
                .with_no_fat_chain(true),
            reserved_1: 0,
            name_length: 0,
            name_hash: 0,
            reserved_2: 0,
            valid_data_length: 0,
            reserved_3: 0,
            first_cluster: 0,
            data_length: 0,
        }
    }
}

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct FileNameDirectoryEntry {
    entry_type: EntryType,
    general_secondary_flags: GeneralPrimaryFlags,
    pub file_name: [u16; 15],
}

impl FileNameDirectoryEntry {
    pub fn new(name: &[u16]) -> Result<Vec<Self>, FileDirectoryEntryError> {
        let contains_illegal_chars = name.iter().any(|ch| {
            matches!(
                ch,
                0x00..=0x1F | 0x22 | 0x2A | 0x2F | 0x3A | 0x3C | 0x3E | 0x3F | 0x5C | 0x7C
            )
        });
        if contains_illegal_chars {
            return Err(FileDirectoryEntryError::IllegalCharactersInName);
        }

        let mut entries = Vec::new();
        for chunk in name.chunks(15) {
            let mut entry = Self::default();
            for (out, char) in entry.file_name.iter_mut().zip(chunk.iter().cloned()) {
                *out = char
            }

            entries.push(entry);
        }

        Ok(entries)
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

impl Default for FileNameDirectoryEntry {
    fn default() -> Self {
        Self {
            entry_type: EntryType::new_with_raw_value(0)
                .with_type_code(u5::new(1))
                .with_type_category(true)
                .with_in_use(true), // 0xC1
            general_secondary_flags: GeneralPrimaryFlags::new_with_raw_value(0),
            file_name: [0; 15],
        }
    }
}

#[derive(Debug)]
pub enum FileDirectoryEntryError {
    EmptyName,
    NameTooLong,
    DuplicateName,
    IllegalCharactersInName,
    IoError(io::Error),
    OutOfFreeSpace,
}

impl PartialEq for FileDirectoryEntryError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::IoError(l0), Self::IoError(r0)) => l0.kind() == r0.kind(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

pub fn name_hash(file_name: &[u16]) -> u16 {
    let bytes: &[u8] = bytemuck::cast_slice(file_name);
    entry_checksum(0, bytes, false)
}

pub fn entry_checksum(init_checksum: u16, data: &[u8], primary: bool) -> u16 {
    let mut checksum = init_checksum;
    for (index, byte) in data.iter().cloned().enumerate() {
        // skip itself, `set_checksum` field
        if primary && (index == 2 || index == 3) {
            continue;
        }

        checksum = (if (checksum & 1) > 0 { 0x8000 } else { 0u16 })
            .wrapping_add(checksum >> 1)
            .wrapping_add(u16::from(byte));
    }

    checksum
}

#[test]
fn hash() {
    let name = "LOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOONG";
    let utf16: Vec<u16> = name.encode_utf16().collect();
    assert_eq!(name_hash(utf16.as_slice()), 0x344B);

    let name = "LOOOOOOOOOOOOOOOOONG";
    let utf16: Vec<u16> = name.encode_utf16().collect();
    assert_eq!(name_hash(utf16.as_slice()), 0xA585);
}
