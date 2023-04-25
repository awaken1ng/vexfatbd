use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Seek};
use std::mem::size_of;
use std::path::Path;

use itertools::Itertools;
use static_assertions::const_assert;

use crate::data_region::allocation_bitmap::{AllocationBitmap, AllocationBitmapDirectoryEntry};
use crate::data_region::file::{
    entry_checksum, is_illegal_file_name_character, name_hash, FileAttributes, FileDirectoryEntry,
    FileDirectoryEntryError, FileNameDirectoryEntry, StreamExtensionDirectoryEntry,
};
use crate::data_region::upcase_table::{upcased_name, UpcaseTableDirectoryEntry, UPCASE_TABLE};
use crate::data_region::volume_label::VolumeLabelDirectoryEntry;
use crate::fat_region::{FileAllocationTable, END_OF_CHAIN};
use crate::utils::{unsigned_rounded_up_div, SliceChain};

#[derive(Debug, PartialEq)]
pub enum DirectoryEntry {
    VolumeLabel(VolumeLabelDirectoryEntry),
    AllocationBitmap(AllocationBitmapDirectoryEntry),
    UpcaseTable(UpcaseTableDirectoryEntry),
    File(FileDirectoryEntry),
    StreamExtension(StreamExtensionDirectoryEntry),
    FileName(FileNameDirectoryEntry),
}

impl DirectoryEntry {
    const SIZE: usize = 32;

    fn new_from_bytes(buffer: &[u8]) -> Option<Self> {
        assert_eq!(buffer.len(), 32);

        match buffer[0] {
            0x81 => {
                let entry: &AllocationBitmapDirectoryEntry = bytemuck::from_bytes(buffer);

                if entry.bitmap_flags.reserved().value() != 0 || entry.reserved != [0; 18] {
                    return None;
                }

                Some(Self::AllocationBitmap(*entry))
            }
            0x82 => {
                let entry: &UpcaseTableDirectoryEntry = bytemuck::from_bytes(buffer);

                if entry.reserved_1 != [0; 3] || entry.reserved_2 != [0; 12] {
                    return None;
                }

                Some(Self::UpcaseTable(*entry))
            }
            0x83 => {
                let entry: &VolumeLabelDirectoryEntry = bytemuck::from_bytes(buffer);

                if entry.character_count > 11 || entry.reserved != [0; 8] {
                    return None;
                }

                let contains_illegal_chars: bool = entry
                    .volume_label
                    .iter()
                    .cloned()
                    .any(is_illegal_file_name_character);
                if contains_illegal_chars {
                    return None;
                }

                Some(Self::VolumeLabel(*entry))
            }
            0x85 => {
                let entry: &FileDirectoryEntry = bytemuck::from_bytes(buffer);

                if entry.secondary_count < 2
                    || entry.secondary_count > 18
                    || entry.file_attributes.reserved_1()
                    || entry.file_attributes.reserved_2().value() != 0
                    || entry.reserved_1 != 0
                    || entry.reserved_2 != [0; 7]
                {
                    return None;
                }

                Some(Self::File(*entry))
            }
            0xC0 => {
                let entry: &StreamExtensionDirectoryEntry = bytemuck::from_bytes(buffer);

                if entry.general_secondary_flags.custom_defined().value() > 0
                    || entry.reserved_1 != 0
                    || entry.reserved_2 != 0
                    || entry.reserved_3 != 0
                {
                    return None;
                }

                Some(Self::StreamExtension(*entry))
            }
            0xC1 => {
                let entry: &FileNameDirectoryEntry = bytemuck::from_bytes(buffer);

                if entry.general_secondary_flags.raw_value() != 0 {
                    return None;
                }

                let contains_illegal_chars: bool = entry
                    .file_name
                    .iter()
                    .cloned()
                    .any(is_illegal_file_name_character);
                if contains_illegal_chars {
                    return None;
                }

                Some(Self::FileName(*entry))
            }
            _ => None,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        match self {
            DirectoryEntry::VolumeLabel(entry) => entry.as_bytes(),
            DirectoryEntry::AllocationBitmap(entry) => entry.as_bytes(),
            DirectoryEntry::UpcaseTable(entry) => entry.as_bytes(),
            DirectoryEntry::File(entry) => entry.as_bytes(),
            DirectoryEntry::StreamExtension(entry) => entry.as_bytes(),
            DirectoryEntry::FileName(entry) => entry.as_bytes(),
        }
    }
}

const_assert!(size_of::<DirectoryEntry>() - 8 == DirectoryEntry::SIZE); // 8 - enum discriminant

pub struct ClusterHeap {
    bytes_per_sector: u32,
    sectors_per_cluster: u32,

    pub fat: FileAllocationTable,

    allocation_bitmap: AllocationBitmap,
    allocation_bitmap_start_cluster: u32,
    allocation_bitmap_end_cluster: u32,

    upcase_table_start_cluster: u32,
    upcase_table_end_cluster: u32,

    heap: HashMap<u32, Cluster>,
    cluster_lookup: HashMap<u32, u32>,
    parent_lookup: HashMap<u32, u32>,
}

impl ClusterHeap {
    pub fn new(bytes_per_sector: u32, sectors_per_cluster: u32, cluster_count: u32) -> Self {
        let bytes_per_cluster = sectors_per_cluster * bytes_per_sector;

        let mut allocation_bitmap = AllocationBitmap::new(cluster_count);
        let allocation_bitmap_start_cluster = 0;
        let allocation_bitmap_size_clusters =
            unsigned_rounded_up_div(allocation_bitmap.size(), bytes_per_cluster);
        let allocation_bitmap_end_cluster =
            allocation_bitmap_start_cluster + allocation_bitmap_size_clusters;

        let upcase_table_start_cluster = allocation_bitmap_end_cluster;
        let upcase_table_size_clusters =
            unsigned_rounded_up_div(2 * UPCASE_TABLE.len() as u32, bytes_per_cluster);
        let upcase_table_end_cluster = upcase_table_start_cluster + upcase_table_size_clusters;

        let root_directory_start_cluster = upcase_table_end_cluster;

        let mut heap = HashMap::new();
        let mut cluster_lookup = HashMap::new();
        heap.insert(
            root_directory_start_cluster,
            Cluster {
                data: ClusterData::DirectoryEntries(DirectoryEntries(vec![
                    DirectoryEntry::VolumeLabel(VolumeLabelDirectoryEntry::empty()),
                    DirectoryEntry::AllocationBitmap(
                        AllocationBitmapDirectoryEntry::new_first_fat(
                            allocation_bitmap_start_cluster,
                            u64::from(cluster_count),
                        ),
                    ),
                    DirectoryEntry::UpcaseTable(UpcaseTableDirectoryEntry::default()),
                ])),
            },
        );
        cluster_lookup.insert(root_directory_start_cluster, root_directory_start_cluster);

        for _ in 0..=upcase_table_end_cluster {
            allocation_bitmap.allocate_next_cluster();
        }

        let mut fat = FileAllocationTable::empty();

        for (cluster, next_cluster) in
            (allocation_bitmap_start_cluster..allocation_bitmap_end_cluster).tuple_windows()
        {
            fat.set_cluster(cluster, next_cluster);
        }
        fat.set_cluster(allocation_bitmap_end_cluster - 1, END_OF_CHAIN);

        for (cluster, next_cluster) in
            (upcase_table_start_cluster..upcase_table_end_cluster).tuple_windows()
        {
            fat.set_cluster(cluster, next_cluster);
        }
        fat.set_cluster(upcase_table_end_cluster - 1, END_OF_CHAIN);

        fat.set_cluster(root_directory_start_cluster, END_OF_CHAIN);

        Self {
            bytes_per_sector,
            sectors_per_cluster,

            fat,

            allocation_bitmap,
            allocation_bitmap_start_cluster,
            allocation_bitmap_end_cluster,

            upcase_table_start_cluster,
            upcase_table_end_cluster,

            heap,
            cluster_lookup,
            parent_lookup: HashMap::new(),
        }
    }

    pub fn read_sector(&mut self, sector: u32, buffer: &mut [u8]) {
        let cluster_index = sector / self.sectors_per_cluster;
        let sector_in_cluster = sector % self.sectors_per_cluster;
        self.read_sector_in_cluster(cluster_index, sector_in_cluster, buffer);
    }

    /// `sector` is cluster relative index
    fn read_sector_in_cluster(&mut self, cluster_index: u32, sector: u32, buffer: &mut [u8]) {
        if (cluster_index >= self.allocation_bitmap_start_cluster)
            && (cluster_index < self.allocation_bitmap_end_cluster)
        {
            let relative_cluster = cluster_index - self.allocation_bitmap_start_cluster;
            let bitmap_sector = (relative_cluster * self.sectors_per_cluster) + sector;
            self.allocation_bitmap.read_sector(bitmap_sector, buffer);
        } else if cluster_index >= self.upcase_table_start_cluster
            && cluster_index < self.upcase_table_end_cluster
        {
            let relative_cluster = cluster_index - self.upcase_table_start_cluster;
            let sector = (relative_cluster * self.sectors_per_cluster) + sector;

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
        } else if let Some(first_cluster) = self.cluster_lookup.get(&cluster_index).cloned() {
            let cluster = self.heap.get_mut(&first_cluster).unwrap();
            let sector = (cluster_index - first_cluster) * self.sectors_per_cluster + sector;
            match &mut cluster.data {
                ClusterData::DirectoryEntries(entries) => entries.read_sector(sector, buffer),
                ClusterData::FileMappedData(file) => {
                    file.read_sector(u64::from(sector) * u64::from(self.bytes_per_sector), buffer)
                }
            }
        }
    }

    pub(crate) fn root_directory_cluster(&self) -> u32 {
        self.upcase_table_end_cluster
    }

    fn is_name_in_cluster(&self, cluster_index: u32, upcased_name_hash: u16) -> bool {
        match self.heap.get(&cluster_index) {
            Some(cluster) => {
                if let ClusterData::DirectoryEntries(entries) = &cluster.data {
                    for entry in entries.0.iter() {
                        if let DirectoryEntry::StreamExtension(stream_extension) = entry {
                            if stream_extension.name_hash == upcased_name_hash {
                                return true;
                            }
                        }
                    }
                }

                false
            }
            None => false,
        }
    }

    fn is_name_in_cluster_chain(&self, root_index: u32, upcased_name_hash: u16) -> bool {
        if self.is_name_in_cluster(root_index, upcased_name_hash) {
            true
        } else {
            for next_cluster in self.fat.chain(root_index) {
                if self.is_name_in_cluster(next_cluster, upcased_name_hash) {
                    return true;
                }
            }

            false
        }
    }

    fn increase_parent_directory_size(&mut self, dir_cluster: u32) {
        if dir_cluster == self.root_directory_cluster() {
            return;
        }

        let parent_cluster = self.parent_lookup.get(&dir_cluster).cloned().unwrap();
        let cluster_chain: Vec<u32> = [parent_cluster]
            .into_iter()
            .chain(self.fat.chain(parent_cluster))
            .collect();

        // look for stream extension entry, keep track of file entry while doing so
        let mut file_entry_pos = None;
        let mut stream_ext_pos = None;

        'outer: for (cluster_idx, cluster_id) in cluster_chain.iter().cloned().enumerate() {
            let cluster = self.heap.get_mut(&cluster_id).unwrap();
            for (entry_idx, entry) in cluster.as_entries_mut().unwrap().iter_mut().enumerate() {
                match entry {
                    DirectoryEntry::File(_) => {
                        file_entry_pos = Some((cluster_idx, entry_idx));
                    }
                    DirectoryEntry::StreamExtension(stream_ext) => {
                        // FAT index
                        if stream_ext.first_cluster != dir_cluster + 2 {
                            continue;
                        }

                        stream_ext_pos = Some((cluster_idx, entry_idx));

                        // update length and flags in stream in extension
                        let cluster_size = self.sectors_per_cluster * self.bytes_per_sector;

                        stream_ext.general_secondary_flags =
                            stream_ext.general_secondary_flags.with_no_fat_chain(false);
                        stream_ext.data_length += u64::from(cluster_size);
                        stream_ext.valid_data_length = stream_ext.data_length;

                        break 'outer;
                    }
                    _ => continue,
                }
            }
        }

        let (stream_ext_chain_idx, stream_ext_entry_idx) = stream_ext_pos.unwrap();
        let (file_chain_idx, file_entry_idx) = file_entry_pos.unwrap();

        let file_name_entries_count;

        // calculate the checksum
        let mut checksum = 0;

        {
            let cluster_id = cluster_chain[file_chain_idx];
            let cluster = self.heap.get(&cluster_id).unwrap();
            let entry = cluster.as_entries().unwrap().get(file_entry_idx).unwrap();
            match entry {
                DirectoryEntry::File(file) => {
                    file_name_entries_count = file.secondary_count - 1;
                    checksum = entry_checksum(checksum, bytemuck::bytes_of(file), true);
                }
                _ => panic!("expected file entry, got {entry:?}"),
            }
        }

        {
            let cluster_id = cluster_chain[stream_ext_chain_idx];
            let cluster = self.heap.get(&cluster_id).unwrap();
            let entry = cluster
                .as_entries()
                .unwrap()
                .get(stream_ext_entry_idx)
                .unwrap();
            match entry {
                DirectoryEntry::StreamExtension(stream_ext) => {
                    checksum = entry_checksum(checksum, bytemuck::bytes_of(stream_ext), false);
                }
                _ => panic!("expected stream extension entry, got {entry:?}"),
            }
        }

        {
            let cluster_id = cluster_chain[stream_ext_chain_idx];
            let cluster = self.heap.get(&cluster_id).unwrap();

            let mut entries_found = 0;

            for entry in cluster
                .as_entries()
                .unwrap()
                .iter()
                .skip(stream_ext_entry_idx + 1)
                .take(usize::from(file_name_entries_count))
            {
                match entry {
                    DirectoryEntry::FileName(file_name_entry) => {
                        checksum =
                            entry_checksum(checksum, bytemuck::bytes_of(file_name_entry), false);
                        entries_found += 1;
                    }
                    _ => panic!("expeced file name entry, got {entry:?}"),
                }
            }

            let remaining_entries = file_name_entries_count - entries_found;
            if remaining_entries > 0 {
                let cluster_id = cluster_chain[stream_ext_chain_idx + 1];
                let cluster = self.heap.get(&cluster_id).unwrap();

                for entry in cluster
                    .as_entries()
                    .unwrap()
                    .iter()
                    .take(usize::from(remaining_entries))
                {
                    match entry {
                        DirectoryEntry::FileName(file_name_entry) => {
                            checksum = entry_checksum(
                                checksum,
                                bytemuck::bytes_of(file_name_entry),
                                false,
                            );
                            entries_found += 1;
                        }
                        _ => panic!("expeced file name entry, got {entry:?}"),
                    }
                }
            }

            assert_eq!(entries_found, file_name_entries_count);
        }

        // update the checksum
        {
            let cluster_id = cluster_chain[file_chain_idx];
            let cluster = self.heap.get_mut(&cluster_id).unwrap();
            let entry = cluster
                .as_entries_mut()
                .unwrap()
                .get_mut(file_entry_idx)
                .unwrap();
            match entry {
                DirectoryEntry::File(file_entry) => {
                    file_entry.set_checksum = checksum;
                }
                _ => panic!("expected file entry, got {entry:?}"),
            }
        }
    }

    /// Add directory into specified root directory, returns first cluster of inserted directory
    pub fn add_directory(
        &mut self,
        root_cluster: u32,
        name: &str,
    ) -> Result<u32, FileDirectoryEntryError> {
        // file name entries
        let name_length: u8 = name
            .len()
            .try_into()
            .map_err(|_| FileDirectoryEntryError::NameTooLong)?;
        if name_length == 0 {
            return Err(FileDirectoryEntryError::EmptyName);
        }
        let name_utf16: Vec<_> = name.encode_utf16().collect();
        let upcased_name = upcased_name(&name_utf16);
        let name_hash = name_hash(&upcased_name);
        if self.is_name_in_cluster_chain(root_cluster, name_hash) {
            return Err(FileDirectoryEntryError::DuplicateName);
        }
        let file_name_entries = FileNameDirectoryEntry::new(&name_utf16)?;

        let secondary_count = 1 + file_name_entries.len() as u8; // stream extension entry and 1..=17 file name entries

        // figure out how many entries we can fit into current cluster
        let cluster_size = self.sectors_per_cluster * self.bytes_per_sector;
        let max_entries_in_cluster = cluster_size / DirectoryEntry::SIZE as u32;

        let mut end_cluster = self.fat.chain(root_cluster).last().unwrap_or(root_cluster);
        let mut previous_cluster = end_cluster;
        let entries_in_cluster = self
            .heap
            .entry(end_cluster)
            .or_insert(Cluster {
                data: ClusterData::DirectoryEntries(DirectoryEntries(Vec::new())),
            })
            .as_entries()
            .unwrap()
            .len() as u32;

        let entries_to_insert = 1 + u32::from(secondary_count);
        let entries_to_insert_in_new_cluster =
            entries_to_insert.saturating_sub(max_entries_in_cluster - entries_in_cluster);
        let entries_to_insert_in_this_cluster =
            entries_to_insert.saturating_sub(entries_to_insert_in_new_cluster);

        if entries_to_insert_in_new_cluster > 0 {
            // new entires will not fit into current last cluster, allocate a new one
            previous_cluster = end_cluster;
            end_cluster = self
                .allocation_bitmap
                .allocate_next_cluster()
                .ok_or(FileDirectoryEntryError::OutOfFreeSpace)?;
            self.heap.insert(
                end_cluster,
                Cluster {
                    data: ClusterData::DirectoryEntries(DirectoryEntries(Vec::new())),
                },
            );
            self.fat.set_cluster(previous_cluster, end_cluster);
            self.fat.set_cluster(end_cluster, END_OF_CHAIN);
            self.cluster_lookup.insert(end_cluster, end_cluster);
            self.increase_parent_directory_size(root_cluster);
        }

        // stream extension entry
        let directory_cluster = self
            .allocation_bitmap
            .allocate_next_cluster()
            .ok_or(FileDirectoryEntryError::OutOfFreeSpace)?;
        let mut stream_extension_entry = StreamExtensionDirectoryEntry::default();
        stream_extension_entry.name_length = name_length;
        stream_extension_entry.name_hash = name_hash;
        stream_extension_entry.first_cluster = directory_cluster + 2; // FAT index
        stream_extension_entry.data_length = u64::from(cluster_size); // empty directory is 1 cluster big
        stream_extension_entry.valid_data_length = stream_extension_entry.data_length;
        self.parent_lookup
            .insert(directory_cluster, root_cluster);
        self.cluster_lookup.insert(directory_cluster, directory_cluster);
        assert!(self
            .heap
            .insert(
                directory_cluster,
                Cluster {
                    data: ClusterData::DirectoryEntries(DirectoryEntries(Vec::new())),
                }
            )
            .is_none());

        // file entry
        let mut file_entry = FileDirectoryEntry::new_directory();
        file_entry.secondary_count = secondary_count;
        file_entry.set_checksum = {
            let mut checksum = entry_checksum(0, bytemuck::bytes_of(&file_entry), true);
            checksum = entry_checksum(checksum, bytemuck::bytes_of(&stream_extension_entry), false);
            for file_name_entry in &file_name_entries {
                checksum = entry_checksum(checksum, bytemuck::bytes_of(file_name_entry), false);
            }

            checksum
        };

        // insert entries into cluster(s)
        let mut entries = vec![
            DirectoryEntry::File(file_entry),
            DirectoryEntry::StreamExtension(stream_extension_entry),
        ];
        entries.extend(file_name_entries.into_iter().map(DirectoryEntry::FileName));

        let mut entries = entries.into_iter();
        let cluster = self
            .heap
            .get_mut(&previous_cluster)
            .unwrap()
            .as_entries_mut()
            .unwrap();
        for _ in 0..entries_to_insert_in_this_cluster {
            let entry = entries.next().unwrap();
            cluster.push(entry);
        }

        if entries.len() > 0 {
            let cluster = self
                .heap
                .get_mut(&end_cluster)
                .unwrap()
                .as_entries_mut()
                .unwrap();
            for _ in 0..entries_to_insert_in_new_cluster {
                let entry = entries.next().unwrap();
                cluster.push(entry);
            }

            assert_eq!(entries.len(), 0);
        }

        Ok(directory_cluster)
    }

    pub fn map_file_with_name<P>(
        &mut self,
        dir_cluster: u32,
        path: P,
        name: &str,
    ) -> Result<u32, FileDirectoryEntryError>
    where
        P: AsRef<Path>,
    {
        // file name entries
        let name_length: u8 = name
            .len()
            .try_into()
            .map_err(|_| FileDirectoryEntryError::NameTooLong)?;
        if name_length == 0 {
            return Err(FileDirectoryEntryError::EmptyName);
        }
        let name_utf16: Vec<_> = name.encode_utf16().collect();
        let upcased_name = upcased_name(&name_utf16);
        let name_hash = name_hash(&upcased_name);
        if self.is_name_in_cluster_chain(dir_cluster, name_hash) {
            return Err(FileDirectoryEntryError::DuplicateName);
        }
        let file_name_entries = FileNameDirectoryEntry::new(&name_utf16)?;

        let mut file = File::open(&path).map_err(FileDirectoryEntryError::IoError)?;
        let file_size_bytes = file
            .seek(std::io::SeekFrom::End(0))
            .map_err(FileDirectoryEntryError::IoError)?;

        let secondary_count = 1 + file_name_entries.len() as u8; // stream extension entry and 1..=17 file name entries

        // figure out how many entries we can fit into current cluster
        let cluster_size = self.sectors_per_cluster * self.bytes_per_sector;
        let max_entries_in_cluster = cluster_size / DirectoryEntry::SIZE as u32;

        let mut end_dir_cluster = self.fat.chain(dir_cluster).last().unwrap_or(dir_cluster);
        let mut previous_dir_cluster = end_dir_cluster;
        let entries_in_cluster = self
            .heap
            .entry(end_dir_cluster)
            .or_insert(Cluster {
                data: ClusterData::DirectoryEntries(DirectoryEntries(Vec::new())),
            })
            .as_entries()
            .unwrap()
            .len() as u32;

        let entries_to_insert = 1 + u32::from(secondary_count);
        let entries_to_insert_in_new_cluster =
            entries_to_insert.saturating_sub(max_entries_in_cluster - entries_in_cluster);
        let entries_to_insert_in_this_cluster =
            entries_to_insert.saturating_sub(entries_to_insert_in_new_cluster);

        if entries_to_insert_in_new_cluster > 0 {
            // new entires will not fit into current last cluster, allocate a new one
            previous_dir_cluster = end_dir_cluster;
            end_dir_cluster = self
                .allocation_bitmap
                .allocate_next_cluster()
                .ok_or(FileDirectoryEntryError::OutOfFreeSpace)?;
            self.heap.insert(
                end_dir_cluster,
                Cluster {
                    data: ClusterData::DirectoryEntries(DirectoryEntries(Vec::new())),
                },
            );
            self.fat.set_cluster(previous_dir_cluster, end_dir_cluster);
            self.fat.set_cluster(end_dir_cluster, END_OF_CHAIN);
            self.cluster_lookup.insert(end_dir_cluster, end_dir_cluster);
            self.increase_parent_directory_size(dir_cluster);
        }

        // stream extension entry
        let file_cluster = self
            .allocation_bitmap
            .allocate_next_cluster()
            .ok_or(FileDirectoryEntryError::OutOfFreeSpace)?;
        let mut stream_extension_entry = StreamExtensionDirectoryEntry::default();
        stream_extension_entry.name_length = name_length;
        stream_extension_entry.name_hash = name_hash;
        stream_extension_entry.first_cluster = file_cluster + 2; // FAT index
        stream_extension_entry.data_length = file_size_bytes;
        stream_extension_entry.valid_data_length = stream_extension_entry.data_length;
        self.cluster_lookup.insert(file_cluster, file_cluster);
        self.parent_lookup.insert(file_cluster, dir_cluster);

        // file entry
        let mut file_entry = FileDirectoryEntry::new_file();
        file_entry.secondary_count = secondary_count;
        file_entry.set_checksum = {
            let mut checksum = entry_checksum(0, bytemuck::bytes_of(&file_entry), true);
            checksum = entry_checksum(checksum, bytemuck::bytes_of(&stream_extension_entry), false);
            for file_name_entry in &file_name_entries {
                checksum = entry_checksum(checksum, bytemuck::bytes_of(file_name_entry), false);
            }

            checksum
        };
        file_entry.file_attributes = FileAttributes::new_with_raw_value(0).with_read_only(true);

        // insert entries into cluster(s)
        let mut entries = vec![
            DirectoryEntry::File(file_entry),
            DirectoryEntry::StreamExtension(stream_extension_entry),
        ];
        entries.extend(file_name_entries.into_iter().map(DirectoryEntry::FileName));

        let mut entries = entries.into_iter();
        let cluster = self
            .heap
            .get_mut(&previous_dir_cluster)
            .unwrap()
            .as_entries_mut()
            .unwrap();
        for _ in 0..entries_to_insert_in_this_cluster {
            let entry = entries.next().unwrap();
            cluster.push(entry);
        }

        if entries.len() > 0 {
            let cluster = self
                .heap
                .get_mut(&end_dir_cluster)
                .unwrap()
                .as_entries_mut()
                .unwrap();
            for _ in 0..entries_to_insert_in_new_cluster {
                let entry = entries.next().unwrap();
                cluster.push(entry);
            }

            assert_eq!(entries.len(), 0);
        }

        // allocate space for the file
        let file_size_clusters = if file_size_bytes > 1 {
            unsigned_rounded_up_div(file_size_bytes, u64::from(cluster_size))
        } else {
            1
        };
        for i in 1..file_size_clusters as u32 {
            self.cluster_lookup.insert(file_cluster + i, file_cluster);
            assert_eq!(
                file_cluster + i,
                self.allocation_bitmap
                    .allocate_next_cluster()
                    .ok_or(FileDirectoryEntryError::OutOfFreeSpace)?
            );
        }

        // insert file into heap
        self.heap.insert(
            file_cluster,
            Cluster {
                data: ClusterData::FileMappedData(FileMappedData { file }),
            },
        );

        Ok(file_cluster)
    }

    /// Map file into specified directory, returns first cluster of inserted file
    pub fn map_file<P>(&mut self, dir_cluster: u32, path: P) -> Result<u32, FileDirectoryEntryError>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        let name = path
            .file_name()
            .ok_or(FileDirectoryEntryError::EmptyName)?
            .to_string_lossy();

        self.map_file_with_name(dir_cluster, path, &name)
    }
}

struct DirectoryEntries(Vec<DirectoryEntry>);

impl DirectoryEntries {
    fn read_sector(&self, sector: u32, buffer: &mut [u8]) {
        let bytes_per_sector = buffer.len();
        let bytes_to_skip = sector as usize * bytes_per_sector;

        let slices = self.0.iter().map(|item| item.as_bytes().iter()).collect();
        let sector_data = SliceChain::new(slices)
            .skip(bytes_to_skip)
            .take(bytes_per_sector);
        for (buffer_byte, sector_byte) in buffer.iter_mut().zip(sector_data) {
            *buffer_byte = *sector_byte;
        }
    }
}

impl Debug for DirectoryEntries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectoryEntries")
            .field("entries", &self.0)
            .field("len", &self.0.len())
            .finish()
    }
}

#[derive(Debug)]
struct FileMappedData {
    file: File,
}

impl FileMappedData {
    fn read_sector(&mut self, offset: u64, buffer: &mut [u8]) {
        self.file.seek(std::io::SeekFrom::Start(offset)).unwrap();
        let _ = self.file.read(buffer).unwrap();
    }
}

#[derive(Debug)]
enum ClusterData {
    DirectoryEntries(DirectoryEntries),
    FileMappedData(FileMappedData),
}

#[derive(Debug)]
struct Cluster {
    data: ClusterData,
}

impl Cluster {
    fn as_entries(&self) -> Option<&[DirectoryEntry]> {
        match &self.data {
            ClusterData::DirectoryEntries(entries) => Some(&entries.0),
            ClusterData::FileMappedData(_) => None,
        }
    }

    fn as_entries_mut(&mut self) -> Option<&mut Vec<DirectoryEntry>> {
        match &mut self.data {
            ClusterData::DirectoryEntries(entries) => Some(&mut entries.0),
            ClusterData::FileMappedData(_) => None,
        }
    }
}

#[test]
fn heap_read() {
    const BYTES_PER_SECTOR: usize = 512;
    let mut heap = ClusterHeap::new(BYTES_PER_SECTOR as _, 8, 512);
    assert_eq!(
        heap.add_directory(heap.root_directory_cluster(), "hello world"),
        Ok(4)
    );

    // allocation bitmap
    let mut buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.allocation_bitmap_start_cluster, 0, &mut buffer);
    assert_eq!(buffer[0], 0b00011111); // 5 clusters
    assert_eq!(&buffer[1..], [0; BYTES_PER_SECTOR - 1]);

    // upcase table
    let mut buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 0, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[..256]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 1, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[256..512]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 2, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[512..768]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 3, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[768..1024]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 4, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[1024..1280]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 5, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[1280..1536]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 6, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[1536..1792]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster, 7, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[1792..2048]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster + 1, 0, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[2048..2304]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster + 1, 1, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[2304..2560]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster + 1, 2, &mut buffer);
    assert_eq!(buffer, bytemuck::cast_slice(&UPCASE_TABLE[2560..2816]));

    buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_start_cluster + 1, 3, &mut buffer);
    assert_eq!(&buffer[..204], bytemuck::cast_slice(&UPCASE_TABLE[2816..]));
    assert_eq!(&buffer[204..], [0; 308]);

    // first entry
    let mut buffer = [0; BYTES_PER_SECTOR];
    heap.read_sector_in_cluster(heap.upcase_table_end_cluster, 0, &mut buffer);
    assert_eq!(&buffer[..32], VolumeLabelDirectoryEntry::empty().as_bytes());
}

#[test]
fn name_duplication() {
    let mut heap = ClusterHeap::new(512, 8, 512);
    let root_cluster = heap.root_directory_cluster();
    assert!(heap.add_directory(root_cluster, "name").is_ok());
    assert_eq!(
        heap.add_directory(root_cluster, "name"),
        Err(FileDirectoryEntryError::DuplicateName)
    );
}

#[test]
fn fragmentation() {
    fn long_name(offset: usize) -> String {
        let mut name = String::new();

        name.push('L');
        for _ in 0..253 - offset {
            name.push('O');
        }
        name.push('G');

        name
    }

    // with 512 byte sectors, and 8 sectors per cluster, each cluster is 4096 bytes
    // each entry is 32 bytes, so single cluster fits 128 entries at most
    let mut heap = ClusterHeap::new(512, 8, 512); // 3 default entries
    let root_cluster = heap
        .add_directory(heap.root_directory_cluster(), "subroot")
        .unwrap();
    assert_eq!(root_cluster, 4);

    assert_eq!(heap.add_directory(root_cluster, &long_name(0)), Ok(5)); // 22 entries
    assert_eq!(heap.add_directory(root_cluster, &long_name(1)), Ok(6)); // 41
    assert_eq!(heap.add_directory(root_cluster, &long_name(2)), Ok(7)); // 60
    assert_eq!(heap.add_directory(root_cluster, &long_name(3)), Ok(8)); // 79
    assert_eq!(heap.add_directory(root_cluster, &long_name(4)), Ok(9)); // 98
    assert_eq!(heap.add_directory(root_cluster, &long_name(5)), Ok(10)); // 117
    let mut heap_keys: Vec<_> = heap.heap.keys().cloned().collect();
    let mut lookup_keys: Vec<_> = heap.cluster_lookup.keys().cloned().collect();
    heap_keys.sort_unstable();
    lookup_keys.sort_unstable();
    assert_eq!(heap_keys, [3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(lookup_keys, [3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(heap.fat.chain(root_cluster).next(), None);

    assert_eq!(heap.add_directory(root_cluster, &long_name(6)), Ok(12)); // 136
    let mut heap_keys: Vec<_> = heap.heap.keys().cloned().collect();
    let mut lookup_keys: Vec<_> = heap.cluster_lookup.keys().cloned().collect();
    heap_keys.sort_unstable();
    lookup_keys.sort_unstable();
    assert_eq!(heap_keys, [3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    assert_eq!(lookup_keys, [3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    assert_eq!(heap.fat.chain(root_cluster).next(), Some(11));

    let cluster = heap.heap.get(&root_cluster).unwrap();
    let entries = cluster.as_entries().unwrap();
    let mut first_clusters = entries.iter().filter_map(|e| match e {
        DirectoryEntry::StreamExtension(se) => Some(se.first_cluster - 2),
        _ => None,
    });
    assert_eq!(first_clusters.next(), Some(5));
    assert_eq!(first_clusters.next(), Some(6));
    assert_eq!(first_clusters.next(), Some(7));
    assert_eq!(first_clusters.next(), Some(8));
    assert_eq!(first_clusters.next(), Some(9));
    assert_eq!(first_clusters.next(), Some(10));
    assert_eq!(first_clusters.next(), Some(12));
    assert_eq!(first_clusters.next(), None);
}
