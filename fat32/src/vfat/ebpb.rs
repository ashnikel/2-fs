use std::{fmt, mem};

use traits::BlockDevice;
use vfat::Error;

#[repr(C, packed)]
pub struct BiosParameterBlock {
    // BPB
    jmp: [u8; 3],
    oem_id: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    sectors_reserved: u16,
    fats_number: u8,
    max_dir_entries: u16,
    logical_sectors_small: u16,
    fat_id: u8,
    sectors_per_fat16: u16,
    sectors_per_track: u16,
    heads: u16,
    hidden_sectors: u32,
    logical_sectors_big: u32,
    // EBPB
    sectors_per_fat: u32,
    flags: u16,
    fat_ver: u16,
    root_dir_cluster: u32,
    fsinfo_sector: u16,
    backup_boot_sector: u16,
    reserved: [u8; 12],
    drive_number: u8,
    reserved2: u8,
    signature: u8,
    volume_serial: u32,
    volume_label: [u8; 11],
    system_id: [u8; 8],
    boot_code: [u8; 420],
    bootable_signature: u16,
}

const EBPB_SIZE: usize = mem::size_of::<BiosParameterBlock>();

impl BiosParameterBlock {
    /// Reads the FAT32 extended BIOS parameter block from sector `sector` of
    /// device `device`.
    ///
    /// # Errors
    ///
    /// If the EBPB signature is invalid, returns an error of `BadSignature`.
    pub fn from<T: BlockDevice>(mut device: T, sector: u64) -> Result<BiosParameterBlock, Error> {
        let mut buf = [0u8; EBPB_SIZE];
        let _ebpb_size = device.read_sector(sector, &mut buf)?;
        let ebpb: BiosParameterBlock = unsafe { mem::transmute(buf) };

        if ebpb.signature != 0x28 && ebpb.signature != 0x29 {
            return Err(Error::BadSignature);
        }

        Ok(ebpb)
    }

    pub fn bytes_per_sector(&self) -> u16 {
        self.bytes_per_sector
    }

    pub fn sectors_per_cluster(&self) -> u8 {
        self.sectors_per_cluster
    }

    pub fn sectors_per_fat(&self) -> u32 {
        self.sectors_per_fat
    }

    pub fn fat_start_sector(&self) -> u64 {
        self.sectors_reserved as u64
    }

    pub fn data_start_sector(&self) -> u64 {
        self.fat_start_sector() + self.fats_number as u64 * self.sectors_per_fat() as u64
    }

    pub fn root_dir_cluster(&self) -> u32 {
        self.root_dir_cluster
    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BiosParameterBlock")
            .field("jmp", &self.jmp)
            .field("oem_id", &self.oem_id)
            .field("bytes_per_sector", &self.bytes_per_sector)
            .field("sectors_per_cluster", &self.sectors_per_cluster)
            .field("sectors_reserved", &self.sectors_reserved)
            .field("fats_number", &self.fats_number)
            .field("max_dir_entries", &self.max_dir_entries)
            .field("logical_sectors_small", &self.logical_sectors_small)
            .field("fat_id", &self.fat_id)
            .field("sectors_per_fat16", &self.sectors_per_fat16)
            .field("sectors_per_track", &self.sectors_per_track)
            .field("heads", &self.heads)
            .field("hidden_sectors", &self.hidden_sectors)
            .field("logical_sectors_big", &self.logical_sectors_big)
            .field("sectors_per_fat", &self.sectors_per_fat)
            .field("flags", &self.flags)
            .field("fat_ver", &self.fat_ver)
            .field("root_dir_cluster", &self.root_dir_cluster)
            .field("fsinfo_sector", &self.fsinfo_sector)
            .field("backup_boot_sector", &self.backup_boot_sector)
            .field("reserved", &"<reserved>")
            .field("drive_number", &self.drive_number)
            .field("reserved2", &self.reserved2)
            .field("signature", &self.signature)
            .field("volume_serial", &self.volume_serial)
            .field("volume_label", &self.volume_label)
            .field("system_id", &self.system_id)
            .field("boot_code", &"<boot_code>")
            .field("bootable_signature", &self.bootable_signature)
            .finish()
    }
}
