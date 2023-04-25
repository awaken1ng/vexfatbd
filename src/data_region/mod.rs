use std::fmt::Debug;

use arbitrary_int::{u5, u6};
use bitbybit::bitfield;
use bytemuck::{Pod, Zeroable};

pub mod allocation_bitmap;
pub mod file;
pub mod upcase_table;
pub mod volume_label;

#[bitfield(u8)]
#[derive(Debug, Zeroable, Pod, PartialEq)]
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
#[derive(Zeroable, Pod, PartialEq)]
pub struct GeneralPrimaryFlags {
    /// The `allocation_possible` field shall describe whether or not an allocation in the Cluster Heap is possible for the given directory entry.
    ///
    /// The valid values for this field shall be:
    /// - `0`, which means an associated allocation of clusters is not possible and the `first_cluster` and `data_length` fields are actually undefined
    ///   (structures which derive from this template may redefine those fields)
    /// - `1`, which means an associated allocation of clusters is possible and the `first_cluster` and `data_length` fields are as defined
    #[bit(0, rw)]
    allocation_possible: bool,

    /// The `no_fat_chain` field shall indicate whether or not the active FAT describes the given allocation's cluster chain.
    ///
    /// The valid values for this field shall be:
    /// - `0`, which means the corresponding FAT entries for the allocation's cluster chain are valid and implementations shall interpret them;
    ///   if the AllocationPossible field contains the value 0, or if the AllocationPossible field contains the value 1 and the FirstCluster field contains the value 0,
    ///   then this field's only valid value is 0
    /// - `1`, which means the associated allocation is one contiguous series of clusters;
    ///   the corresponding FAT entries for the clusters are invalid and implementations shall not interpret them;
    ///   implementations may use the following equation to calculate the size of the associated allocation:
    ///   `data_length / (1 << sectors_per_cluster_shift * 1 << bytes_per_sector_shift)` rounded up to the nearest integer
    ///
    /// If critical primary directory entry structures which derive from this template redefine the GeneralPrimaryFlags field,
    /// then the corresponding FAT entries for any associated allocation's cluster chain are valid.
    #[bit(1, rw)]
    no_fat_chain: bool,

    #[bits(2..=7, rw)]
    custom_defined: u6,
}

impl Debug for GeneralPrimaryFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeneralPrimaryFlags")
            .field("allocation_possible", &self.allocation_possible())
            .field("no_fat_chain", &self.no_fat_chain())
            .finish()
    }
}
