use std::{fmt, io, mem};

use traits::BlockDevice;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct CHS {
    head: u8,
    sector: u8,
    cylinder: u8,
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct PartitionEntry {
    boot: u8,
    chs_start: CHS,
    part_type: u8,
    chs_end: CHS,
    relative_sector: u32,
    total_sectors: u32,
}

/// The master boot record (MBR).
#[repr(C, packed)]
pub struct MasterBootRecord {
    bootstrap: [u8; 436],
    disk_id: [u8; 10],
    partition_table: [PartitionEntry; 4],
    signature: [u8; 2],
}

const MBR_SIZE: usize = mem::size_of::<MasterBootRecord>();

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl MasterBootRecord {
    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
        let mut buf = [0u8; MBR_SIZE];

        let mbr_size = match device.read_sector(0, &mut buf) {
            Ok(size) => size,
            Err(e) => return Err(Error::Io(e))
        };

        if mbr_size != MBR_SIZE {
            return Err(Error::Io(io::Error::new(io::ErrorKind::UnexpectedEof, "bad MBR size")));
        }

        let mbr: MasterBootRecord = unsafe { mem::transmute(buf) };

        if mbr.signature != [0x55, 0xAA] {
            return Err(Error::BadSignature);
        }

        for i in 0..mbr.partition_table.len() {
            if mbr.partition_table[i].boot != 0x00 &&
               mbr.partition_table[i].boot != 0x80 {
                   return Err(Error::UnknownBootIndicator(i as u8))
               }
        }

        Ok(mbr)
    }
}

impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!("MasterBootRecord::fmt()")
    }
}
