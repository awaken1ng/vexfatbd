use std::{fs, io::Seek, path::Path};

use arbitrary_int::{u10, u4, u5, u6, u7};
use bitbybit::bitfield;
use bytemuck::{Pod, Zeroable};
use itertools::Itertools;

use crate::{data_region::upcase_table::upcased_file_name, heap::DirectoryEntry};

use super::{EntryType, GeneralPrimaryFlags};

#[bitfield(u16)]
#[derive(Zeroable, Pod)]
struct FileAttributes {
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
    secondary_count: u8,
    set_checksum: u16,
    file_attributes: FileAttributes,
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
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct StreamExtensionDirectoryEntry {
    entry_type: EntryType,
    general_secondary_flags: GeneralPrimaryFlags,
    reserved_1: u8,
    name_length: u8,
    name_hash: u16,
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
    valid_data_length: u64,

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
    first_cluster: u32, // FAT index

    /// The `data_length` field shall conform to the definition provided in the Generic Secondary DirectoryEntry template (see Section 6.4.4).
    ///
    /// If the corresponding File directory entry describes a directory, then the valid value for this field is the entire size of the associated allocation, in bytes, which may be 0.
    /// Further, for directories, the maximum value for this field is 256MB.
    data_length: u64,
}

impl StreamExtensionDirectoryEntry {
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct FileNameDirectoryEntry {
    entry_type: EntryType,
    general_secondary_flags: GeneralPrimaryFlags,
    file_name: [u16; 15],
}

impl FileNameDirectoryEntry {
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

#[derive(Debug)]
pub enum FileDirectoryEntryError {
    NameTooLong,
    IllegalCharactersInName,
}

fn new_file_name_entry(
    file_name: &str,
) -> Result<Vec<FileNameDirectoryEntry>, FileDirectoryEntryError> {
    let contains_illegal_chars = file_name.chars().any(|ch| {
        matches!(
            ch,
            '\0'..='\x1F'
                | '\x22'
                | '\x2A'
                | '\x2F'
                | '\x3A'
                | '\x3C'
                | '\x3E'
                | '\x3F'
                | '\x5C'
                | '\x7C'
        )
    });
    if contains_illegal_chars {
        return Err(FileDirectoryEntryError::IllegalCharactersInName);
    }

    let mut file_name_entries = Vec::new();
    for chunk in file_name.encode_utf16().chunks(15).into_iter() {
        let mut entry = FileNameDirectoryEntry {
            entry_type: EntryType::new_with_raw_value(0)
                .with_type_code(u5::new(1))
                .with_type_category(true)
                .with_in_use(true), // 0xC1
            general_secondary_flags: GeneralPrimaryFlags::new_with_raw_value(0),
            file_name: [0; 15],
        };

        for (out, char) in entry.file_name.iter_mut().zip(chunk) {
            *out = char;
        }

        file_name_entries.push(entry);
    }

    Ok(file_name_entries)
}

pub fn new_folder(
    file_name: &str,
    first_cluster: u32,
    cluster_size: u64,
) -> Result<Vec<DirectoryEntry>, FileDirectoryEntryError> {
    let name_length: u8 = file_name
        .encode_utf16()
        .count()
        .try_into()
        .map_err(|_| FileDirectoryEntryError::NameTooLong)?;

    let upcased_name = upcased_file_name(file_name);
    let name_hash = name_hash(&upcased_name);

    let file_name_entries = new_file_name_entry(file_name)?;

    let stream_extension_entry = StreamExtensionDirectoryEntry {
        entry_type: EntryType::new_with_raw_value(0)
            .with_type_category(true)
            .with_in_use(true), // 0xC0
        general_secondary_flags: GeneralPrimaryFlags::new_with_raw_value(0)
            .with_allocation_possible(true)
            .with_no_fat_chain(true),
        reserved_1: 0,
        name_length,
        name_hash,
        reserved_2: 0,
        valid_data_length: cluster_size,
        reserved_3: 0,
        first_cluster,
        data_length: cluster_size,
    };

    let mut file_entry = FileDirectoryEntry {
        entry_type: EntryType::new_with_raw_value(0)
            .with_type_code(u5::new(5))
            .with_in_use(true), // 0x85
        secondary_count: 1 + file_name_entries.len() as u8,
        set_checksum: 0, // set later
        file_attributes: FileAttributes::new_with_raw_value(0).with_directory(true),
        reserved_1: 0,
        create_timestamp: 0,             // TODO
        last_modified_timestamp: 0,      // TODO
        last_accessed_timestamp: 0,      // TODO
        create_10ms_increment: 0,        // TODO
        last_modified_10ms_increment: 0, // TODO
        create_utc_offset: 0,            // TODO
        last_modified_utc_offset: 0,     // TODO
        last_accessed_utc_offset: 0,     // TODO
        reserved_2: [0; 7],
    };

    let mut checksum = entry_checksum(0, bytemuck::bytes_of(&file_entry), true);
    checksum = entry_checksum(checksum, bytemuck::bytes_of(&stream_extension_entry), false);
    for file_name_entry in &file_name_entries {
        checksum = entry_checksum(checksum, bytemuck::bytes_of(file_name_entry), false);
    }
    file_entry.set_checksum = checksum;

    let mut entries = vec![
        DirectoryEntry::File(file_entry),
        DirectoryEntry::StreamExtension(stream_extension_entry),
    ];
    entries.extend(file_name_entries.into_iter().map(DirectoryEntry::FileName));

    Ok(entries)
}

pub fn new_file<P>(
    path: P,
    first_cluster: u32,
) -> Result<Vec<DirectoryEntry>, FileDirectoryEntryError>
where
    P: AsRef<Path>,
{
    let file_name = path
        .as_ref()
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let name_length: u8 = file_name
        .encode_utf16()
        .count()
        .try_into()
        .map_err(|_| FileDirectoryEntryError::NameTooLong)?;

    let upcased_name = upcased_file_name(&file_name);
    let name_hash = name_hash(&upcased_name);

    let file_name_entries = new_file_name_entry(&file_name)?;

    let mut file = fs::File::open(path).unwrap();
    let file_size = file.seek(std::io::SeekFrom::End(0)).unwrap();

    let stream_extension_entry = StreamExtensionDirectoryEntry {
        entry_type: EntryType::new_with_raw_value(0)
            .with_type_category(true)
            .with_in_use(true), // 0xC0
        general_secondary_flags: GeneralPrimaryFlags::new_with_raw_value(0)
            .with_allocation_possible(true)
            .with_no_fat_chain(true),
        reserved_1: 0,
        name_length,
        name_hash,
        reserved_2: 0,
        valid_data_length: file_size,
        reserved_3: 0,
        first_cluster,
        data_length: file_size,
    };

    let mut file_entry = FileDirectoryEntry {
        entry_type: EntryType::new_with_raw_value(0)
            .with_type_code(u5::new(5))
            .with_in_use(true), // 0x85
        secondary_count: 1 + file_name_entries.len() as u8,
        set_checksum: 0, // set later
        file_attributes: FileAttributes::new_with_raw_value(0).with_read_only(true),
        reserved_1: 0,
        create_timestamp: 0,             // TODO
        last_modified_timestamp: 0,      // TODO
        last_accessed_timestamp: 0,      // TODO
        create_10ms_increment: 0,        // TODO
        last_modified_10ms_increment: 0, // TODO
        create_utc_offset: 0,            // TODO
        last_modified_utc_offset: 0,     // TODO
        last_accessed_utc_offset: 0,     // TODO
        reserved_2: [0; 7],
    };

    let mut checksum = entry_checksum(0, bytemuck::bytes_of(&file_entry), true);
    checksum = entry_checksum(checksum, bytemuck::bytes_of(&stream_extension_entry), false);
    for file_name_entry in &file_name_entries {
        checksum = entry_checksum(checksum, bytemuck::bytes_of(file_name_entry), false);
    }
    file_entry.set_checksum = checksum;

    let mut entries = vec![
        DirectoryEntry::File(file_entry),
        DirectoryEntry::StreamExtension(stream_extension_entry),
    ];
    entries.extend(file_name_entries.into_iter().map(DirectoryEntry::FileName));

    Ok(entries)
}

fn name_hash(file_name: &[u16]) -> u16 {
    let bytes: &[u8] = bytemuck::cast_slice(file_name);

    let mut name_hash = 0;
    for byte in bytes {
        name_hash = (if (name_hash & 1) > 0 { 0x8000 } else { 0u16 })
            .wrapping_add(name_hash >> 1)
            .wrapping_add(u16::from(*byte));
    }

    name_hash
}

fn entry_checksum(init_checksum: u16, entry: &[u8], primary: bool) -> u16 {
    assert_eq!(entry.len(), 32);

    let mut checksum = init_checksum;
    for (index, byte) in entry.iter().cloned().enumerate() {
        // skip itself, `set_checksum` field
        if primary && (index == 2 || index == 3) {
            continue;
        }

        checksum =
            (if (checksum & 1) > 0 { 0x8000 } else { 0 }) + (checksum >> 1) + u16::from(byte);
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
