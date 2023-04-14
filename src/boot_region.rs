use bytemuck::{Pod, Zeroable};

/// The BytesPerSectorShift field shall describe the bytes per sector expressed as log2(N), where N is the number of bytes per sector. For example, for 512 bytes per sector, the value of this field is 9.
///
/// The valid range of values for this field shall be:
/// - At least 9 (sector size of 512 bytes), which is the smallest sector possible for an exFAT volume
/// - At most 12 (sector size of 4096 bytes), which is the memory page size of CPUs common in personal computers
pub const BYTES_PER_SECTOR_SHIFT: u8 = 9;
pub const BYTES_PER_SECTOR: u16 = 1 << BYTES_PER_SECTOR_SHIFT;

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct BootSector {
    pub jump_boot: [u8; 3],
    pub filesystem_name: [u8; 8],
    must_be_zero: [u8; 53],
    pub partition_offset: u64,
    pub volume_length: u64,
    pub fat_offset: u32,
    pub fat_length: u32,
    pub cluster_heap_offset: u32,
    pub cluster_count: u32,
    pub first_cluster_of_root_directory: u32,
    pub volume_serial_number: u32,
    pub filesystem_revision: u16,
    pub volume_flags: u16,
    pub bytes_per_sector_shift: u8, // 9..=12
    pub sectors_per_cluster_shift: u8, // 0..=25
    pub number_of_fats: u8, /// 1..=2
    pub drive_select: u8,
    pub percent_in_use: u8,
    reserved: [u8; 7],
    pub boot_code: [u8; 390],
    pub boot_signature: [u8; 2],
}
