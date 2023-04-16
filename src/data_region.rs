use std::collections::HashMap;

use arbitrary_int::{u5, u6};
use bitbybit::bitfield;
use bytemuck::{Pod, Zeroable};

mod allocation_bitmap;
mod file;
mod upcase_table;
mod volume_label;

use crate::utils::Chain;

use self::{
    allocation_bitmap::{AllocationBitmap, AllocationBitmapDirectoryEntry},
    file::FileDirectoryEntries,
    upcase_table::{UpcaseTableDirectoryEntry, UPCASE_TABLE},
    volume_label::VolumeLabelDirectoryEntry,
};

#[bitfield(u8)]
#[derive(Zeroable, Pod)]
struct EntryType {
    /// The `TypeCode` field partially describes the specific type of the given directory entry.
    /// This field, plus the `TypeImportance` and `TypeCategory` fields (see Section 6.2.1.2 and Section 6.2.1.3, respectively)
    /// uniquely identify the type of the given directory entry.
    ///
    /// All possible values of this field are valid,
    /// unless the `TypeImportance` and `TypeCategory` fields both contain the value 0;
    /// in that case, the value 0 is invalid for this field.
    #[bits(0..=4, rw)]
    type_code: u5,

    /// The `TypeImportance` field shall describe the importance of the given directory entry.
    ///
    /// The valid values for this field shall be:
    /// - 0, which means the given directory entry is critical (see Section 6.3.1.2.1 and Section 6.4.1.2.1 for critical primary and critical secondary directory entries, respectively)
    /// - 1, which means the given directory entry is benign (see Section 6.3.1.2.2 and Section 6.4.1.2.2 for benign primary and benign secondary directory entries, respectively)
    #[bit(5, rw)]
    type_importance: bool,

    /// The `TypeCategory` field shall describe the category of the given directory entry.
    ///
    /// The valid values for this field shall be:
    /// - 0, which means the given directory entry is primary (see Section 6.3)
    /// - 1, which means the given directory entry is secondary (see Section 6.4)
    #[bit(6, rw)]
    type_category: bool,

    /// The `InUse` field shall describe whether the given directory entry in use or not.
    ///
    /// The valid values for this field shall be:
    /// - 0, which means the given directory entry is not in use; this means the given structure actually is an unused directory entry
    /// - 1, which means the given directory entry is in use; this means the given structure is a regular directory entry
    #[bit(7, rw)]
    in_use: bool,
}

#[bitfield(u8)]
#[derive(Zeroable, Pod)]
struct GeneralPrimaryFlags {
    #[bit(0, rw)]
    allocation_possible: bool,

    #[bit(1, rw)]
    no_fat_chain: bool,

    #[bits(2..=7, rw)]
    custom_defined: u6,
}

enum ClusterData {
    AllocationBitmap(AllocationBitmap),
    UpcaseTable(Vec<u16>),
    DirectoryEntry(DirectoryEntry),
}

impl ClusterData {
    fn as_bytes(&self) -> &[u8] {
        match self {
            ClusterData::AllocationBitmap(ab) => ab.as_bytes(),
            ClusterData::UpcaseTable(ut) => bytemuck::cast_slice(ut.as_slice()),
            ClusterData::DirectoryEntry(e) => e.as_bytes(),
        }
    }
}

enum DirectoryEntry {
    VolumeLabel(VolumeLabelDirectoryEntry),
    AllocationBitmap(AllocationBitmapDirectoryEntry),
    UpcaseTable(UpcaseTableDirectoryEntry),
}

impl DirectoryEntry {
    fn as_bytes(&self) -> &[u8] {
        match self {
            DirectoryEntry::VolumeLabel(entry) => entry.as_bytes(),
            DirectoryEntry::AllocationBitmap(entry) => entry.as_bytes(),
            DirectoryEntry::UpcaseTable(entry) => entry.as_bytes(),
            DirectoryEntry::File(entry) => entry.as_bytes(),
        }
    }

    fn size(&self) -> usize {
        self.as_bytes().len()
    }
}

struct Cluster {
    data: Vec<ClusterData>,
}

impl Cluster {
    fn read_sector(&self, sector: u64, buffer: &mut [u8]) {
        let bytes_per_sector = buffer.len();
        let bytes_to_skip = sector as usize * bytes_per_sector;

        let slices = self
            .data
            .iter()
            .map(|item| item.as_bytes().iter())
            .collect();
        let sector_data = Chain::new(slices)
            .skip(bytes_to_skip)
            .take(bytes_per_sector);
        for (buffer_byte, sector_byte) in buffer.iter_mut().zip(sector_data) {
            *buffer_byte = *sector_byte;
        }
    }
}

pub struct ClusterHeap {
    clusters: HashMap<u64, Cluster>,
    sectors_per_cluster: u32,
}

impl ClusterHeap {
    pub fn new(cluster_count: u32) -> Self {
        let mut clusters = HashMap::new();

        clusters.insert(
            0,
            Cluster {
                data: vec![ClusterData::AllocationBitmap(AllocationBitmap::new(4))], // these 4 clusters
            },
        );

        clusters.insert(
            1,
            Cluster {
                data: vec![ClusterData::UpcaseTable(Vec::from(&UPCASE_TABLE[..2048]))],
            },
        );

        clusters.insert(
            2,
            Cluster {
                data: vec![ClusterData::UpcaseTable(Vec::from(&UPCASE_TABLE[2048..]))],
            },
        );

        clusters.insert(
            3,
            Cluster {
                data: vec![
                    ClusterData::DirectoryEntry(DirectoryEntry::VolumeLabel(
                        VolumeLabelDirectoryEntry::empty(),
                    )),
                    ClusterData::DirectoryEntry(DirectoryEntry::AllocationBitmap(
                        AllocationBitmapDirectoryEntry::new_first_fat(2, u64::from(cluster_count)),
                    )),
                    ClusterData::DirectoryEntry(DirectoryEntry::UpcaseTable(
                        UpcaseTableDirectoryEntry::default(),
                    )),
                ],
            },
        );

        Self {
            clusters,
            sectors_per_cluster: 8,
        }
    }

    pub fn read_sector(&self, sector: u64, buffer: &mut [u8]) {
        let cluster_index = sector / u64::from(self.sectors_per_cluster);
        let sector_in_cluster = sector % u64::from(self.sectors_per_cluster);
        self.read_sector_in_cluster(cluster_index, sector_in_cluster, buffer);
    }

    fn read_sector_in_cluster(&self, cluster: u64, sector: u64, buffer: &mut [u8]) {
        if let Some(cluster) = self.clusters.get(&cluster) {
            cluster.read_sector(sector, buffer);
        }
    }
}

#[test]
fn cluster_read() {
    let mut buffer = [0; 512];
    let mut cluster = Cluster {
        data: vec![ClusterData::UpcaseTable(Vec::from(&UPCASE_TABLE[..2048c]))],
    };
    cluster.read_sector(0, &mut buffer);
    assert_eq!(bytemuck::cast_slice::<_, u16>(&buffer), &UPCASE_TABLE[..256]);

    buffer = [0; 512];
    cluster.read_sector(1, &mut buffer);
    assert_eq!(bytemuck::cast_slice::<_, u16>(&buffer), &UPCASE_TABLE[256..512]);

    buffer = [0; 512];
    cluster.data = vec![
        ClusterData::UpcaseTable(Vec::from(&UPCASE_TABLE[..16])),
        ClusterData::UpcaseTable(Vec::from(&UPCASE_TABLE[16..32])),
        ClusterData::UpcaseTable(Vec::from(&UPCASE_TABLE[32..48])),
        ClusterData::UpcaseTable(Vec::from(&UPCASE_TABLE[48..64])),
    ];
    cluster.read_sector(0, &mut buffer);
    assert_eq!(bytemuck::cast_slice::<_, u16>(&buffer[..128]), &UPCASE_TABLE[..64]);
    assert_eq!(buffer[128..], [0; 384]);
}

#[test]
fn heap_read() {
    let heap = ClusterHeap::new(512);

    let mut buffer = [0; 512];
    heap.read_sector(0, &mut buffer);

    assert_eq!(&buffer[..2], &[15, 0])
}
