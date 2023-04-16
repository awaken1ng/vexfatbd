use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct BootSector {
    /// The JumpBoot field shall contain the jump instruction for CPUs common in personal computers,
    /// which, when executed, "jumps" the CPU to execute the boot-strapping instructions in the `boot_code` field.
    ///
    /// The valid value for this field is (in order of low-order byte to high-order byte) EBh 76h 90h.
    pub jump_boot: [u8; 3],

    /// The `filesystem_name` field shall contain the name of the file system on the volume.
    ///
    /// The valid value for this field is, in ASCII characters, `"EXFAT "`, which includes three trailing white spaces.
    pub filesystem_name: [u8; 8],

    /// The `must_be_zero` field shall directly correspond with the range of bytes the packed BIOS parameter block consumes on FAT12/16/32 volumes.
    ///
    /// The valid value for this field is 0, which helps to prevent FAT12/16/32 implementations from mistakenly mounting an exFAT volume.
    must_be_zero: [u8; 53],

    /// The `partition_offset` field shall describe the media-relative sector offset of the partition which hosts the given exFAT volume.
    /// This field aids boot-strapping from the volume using extended INT 13h on personal computers.
    ///
    /// All possible values for this field are valid; however, the value 0 indicates implementations shall ignore this field.
    pub partition_offset: u64,

    /// The `volume_length` field shall describe the size of the given exFAT volume in sectors.
    ///
    /// The valid range of values for this field shall be:
    /// - At least `1 << 20` / `1 << bytes_per_sector_shift`, which ensures the smallest volume is no less than 1MB
    /// - At most `(1 << 64) - 1`, the largest value this field can describe.
    ///
    /// However, if the size of the Excess Space sub-region is 0, then the largest value of this field is `cluster_heap_offset + ((1 << 32) - 11) * (1 << sectors_per_cluster_shift)`.
    pub volume_length: u64,

    /// The `fat_offset` field shall describe the volume-relative sector offset of the First FAT.
    /// This field enables implementations to align the First FAT to the characteristics of the underlying storage media.
    ///
    /// The valid range of values for this field shall be:
    /// - At least `24`, which accounts for the sectors the Main Boot and Backup Boot regions consume
    /// - At most `cluster_heap_offset - (fat_length * number_of_fats)`, which accounts for the sectors the Cluster Heap consumes
    pub fat_offset: u32,

    /// The `fat_length` field shall describe the length, in sectors, of each FAT table (the volume may contain up to two FATs).
    ///
    /// The valid range of values for this field shall be:
    /// - At least `(cluster_count + 2) * 4 / (1 << bytes_per_sector_shift)` rounded up to the nearest integer,
    ///   which ensures each FAT has sufficient space for describing all the clusters in the Cluster Heap
    /// - At most `(cluster_heap_offset - fat_offset) / number_of_fats` rounded down to the nearest integer,
    ///   which ensures the FATs exist before the Cluster Heap
    ///
    /// This field may contain a value in excess of its lower bound (as described above) to enable the Second FAT,
    /// if present, to also be aligned to the characteristics of the underlying storage media.
    /// The contents of the space which exceeds what the FAT itself requires, if any, are undefined.
    pub fat_length: u32,

    /// The `cluster_heap_offset` field shall describe the volume-relative sector offset of the Cluster Heap.
    /// This field enables implementations to align the Cluster Heap to the characteristics of the underlying storage media.
    ///
    /// The valid range of values for this field shall be:
    /// - At least `fat_offset + fat_length * number_of_fats`, to account for the sectors all the preceding regions consume
    /// - At most `(1 << 32) - 1` or `volume_length - (cluster_count * 1 << sectors_per_cluster_shift)`, whichever calculation is less
    pub cluster_heap_offset: u32,

    /// The `cluster_count` field shall describe the number of clusters the Cluster Heap contains.
    ///
    /// The valid value for this field shall be the lesser of the following:
    /// - `(volume_length - cluster_heap_offset) / 1 << sectors_per_cluster_shift` rounded down to the nearest integer,
    ///   which is exactly the number of clusters which can fit between the beginning of the Cluster Heap and the end of the volume
    /// - `(1 << 32) - 11`, which is the maximum number of clusters a FAT can describe
    ///
    /// The value of the `cluster_count` field determines the minimum size of a FAT.
    /// To avoid extremely large FATs, implementations can control the number of clusters in the Cluster Heap by increasing the cluster size (via the `sectors_per_cluster_shift` field).
    /// This specification recommends no more than 224- 2 clusters in the Cluster Heap.
    /// However, implementations shall be able to handle volumes with up to 232- 11 clusters in the Cluster Heap.
    pub cluster_count: u32,

    /// The `first_cluster_of_root_directory` field shall contain the cluster index of the first cluster of the root directory.
    /// Implementations should make every effort to place the first cluster of the root directory in the first non-bad cluster after the clusters the Allocation Bitmap and Up-case Table consume.
    ///
    /// The valid range of values for this field shall be:
    /// - At least `2`, the index of the first cluster in the Cluster Heap
    /// - At most `cluster_count + 1`, the index of the last cluster in the Cluster Heap
    pub first_cluster_of_root_directory: u32,

    /// The `volume_serial_number` field shall contain a unique serial number.
    /// This assists implementations to distinguish among different exFAT volumes.
    /// Implementations should generate the serial number by combining the date and time of formatting the exFAT volume.
    /// The mechanism for combining date and time to form a serial number is implementation-specific.
    ///
    /// All possible values for this field are valid.
    pub volume_serial_number: u32,

    /// The `filesystem_revision` field shall describe the major and minor revision numbers of the exFAT structures on the given volume.
    ///
    /// The high-order byte is the major revision number and the low-order byte is the minor revision number.
    /// For example, if the high-order byte contains the value 01h and if the low-order byte contains the value 05h,
    /// then the `filesystem_revision` field describes the revision number 1.05.
    /// Likewise, if the high-order byte contains the value 0Ah and if the low-order byte contains the value 0Fh,
    /// then the `filesystem_revision` field describes the revision number 10.15.
    ///
    /// The valid range of values for this field shall be:
    /// - At least `0` for the low-order byte and `1` for the high-order byte
    /// - At most `99` for the low-order byte and `99` for the high-order byte
    ///
    /// The revision number of exFAT this specification describes is 1.00.
    /// Implementations of this specification should mount any exFAT volume with major revision number 1 and shall not mount any exFAT volume with any other major revision number.
    /// Implementations shall honor the minor revision number and shall not perform operations or create any file system structures not described in the given minor revision number's corresponding specification.
    pub filesystem_revision: u16,

    /// The `volume_flags` field shall contain flags which indicate the status of various file system structures on the exFAT volume (see Table 5).
    ///
    /// Implementations shall not include this field when computing its respective Main Boot or Backup Boot region checksum. When referring to the Backup Boot Sector, implementations shall treat this field as stale.
    pub volume_flags: u16,

    /// The `bytes_per_sector_shift` field shall describe the bytes per sector expressed as log2(N), where N is the number of bytes per sector. For example, for 512 bytes per sector, the value of this field is 9.
    ///
    /// The valid range of values for this field shall be:
    /// - At least 9 (sector size of 512 bytes), which is the smallest sector possible for an exFAT volume
    /// - At most 12 (sector size of 4096 bytes), which is the memory page size of CPUs common in personal computers
    pub bytes_per_sector_shift: u8,

    /// The `SectorsPerClusterShift` field shall describe the sectors per cluster expressed as log2(N), where N is number of sectors per cluster.
    /// For example, for 8 sectors per cluster, the value of this field is 3.
    ///
    /// The valid range of values for this field shall be:
    /// - At least `0` (1 sector per cluster), which is the smallest cluster possible
    /// - At most `25 - bytes_per_sector_shift`, which evaluates to a cluster size of 32MB
    pub sectors_per_cluster_shift: u8,

    /// The `number_of_fats` field shall describe the number of FATs and Allocation Bitmaps the volume contains.
    ///
    /// The valid range of values for this field shall be:
    /// - `1`, which indicates the volume only contains the First FAT and First Allocation Bitmap
    /// - `2`, which indicates the volume contains the First FAT, Second FAT, First Allocation Bitmap, and Second Allocation Bitmap; this value is only valid for TexFAT volumes
    pub number_of_fats: u8, /// 1..=2

    /// The `drive_select` field shall contain the extended INT 13h drive number, which aids boot-strapping from this volume using extended INT 13h on personal computers.
    ///
    /// All possible values for this field are valid.
    /// Similar fields in previous FAT-based file systems frequently contained the value 80h.
    pub drive_select: u8,

    /// The `percent_in_use` field shall describe the percentage of clusters in the Cluster Heap which are allocated.
    ///
    /// The valid range of values for this field shall be:
    /// - Between `0` and `100` inclusively, which is the percentage of allocated clusters in the Cluster Heap, rounded down to the nearest integer
    /// - Exactly `FFh`, which indicates the percentage of allocated clusters in the Cluster Heap is not available
    ///
    /// Implementations shall change the value of this field to reflect changes in the allocation of clusters in the Cluster Heap or shall change it to FFh.
    ///
    /// Implementations shall not include this field when computing its respective Main Boot or Backup Boot region checksum. When referring to the Backup Boot Sector, implementations shall treat this field as stale.
    pub percent_in_use: u8,

    reserved: [u8; 7],

    /// The `boot_code` field shall contain boot-strapping instructions.
    ///
    /// Implementations may populate this field with the CPU instructions necessary for boot-strapping a computer system.
    ///
    /// Implementations which don't provide boot-strapping instructions shall initialize each byte in this field to F4h (the halt instruction for CPUs common in personal computers) as part of their format operation.
    pub boot_code: [u8; 390],

    /// The `boot_signature` field shall describe whether the intent of a given sector is for it to be a Boot Sector or not.
    ///
    /// The valid value for this field is `AA55h`. Any other value in this field invalidates its respective Boot Sector. Implementations should verify the contents of this field prior to depending on any other field in its respective Boot Sector.
    pub boot_signature: [u8; 2],
}
