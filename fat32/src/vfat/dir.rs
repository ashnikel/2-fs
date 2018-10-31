use std::ffi::OsStr;
use std::char::{decode_utf16, REPLACEMENT_CHARACTER};
use std::borrow::Cow;
use std::io;

use traits;
use util::VecExt;
use vfat::{Cluster, Entry, File, Shared, VFat};
use vfat::{Attributes, Date, Metadata, Time, Timestamp};

#[derive(Debug)]
pub struct Dir {
    name: String,
    cluster: Cluster,
    vfat: Shared<VFat>,
    metadata: Metadata,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    name: [u8; 8],
    ext: [u8; 3],
    attr: u8,
    reserved: u8,
    ctime_fine: u8,
    ctime: u16,
    cdate: u16,
    adate: u16,
    cluster_hi: u16,
    mtime: u16,
    mdate: u16,
    cluster_lo: u16,
    size: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    seq_number: u8,
    name1: [u16; 5],
    attr: u8,
    lfn_type: u8,
    checksum: u8,
    name2: [u16; 6],
    zero_pad: u16,
    name3: [u16; 2],
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    id: u8, // 0x00 - end of dir, 0xE5 - deleted/unused dir, other - regular dir or LFN
    unknown1: [u8; 10],
    attr: u8,
    unknown2: [u8; 20],
}

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

pub struct EntryIter {
    entries: Vec<VFatDirEntry>,
    index: usize,
    vfat: Shared<VFat>,
}

impl VFatUnknownDirEntry {
    pub fn is_deleted(&self) -> bool {
        self.id == 0xE5
    }

    pub fn is_end(&self) -> bool {
        self.id == 0x00
    }

    pub fn is_dir(&self) -> bool {
        !self.is_deleted() && !self.is_end()
    }

    pub fn is_lfn(&self) -> bool {
        self.is_dir() && self.attr == 0x0F
    }

    pub fn is_regular(&self) -> bool {
        self.is_dir() && !self.is_lfn()
    }
}

impl VFatLfnDirEntry {
    pub fn is_last(&self) -> bool {
        self.seq_number & (1 << 6) != 0
    }

    pub fn is_deleted(&self) -> bool {
        self.seq_number == 0xE5
    }
}

impl Iterator for EntryIter {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        let mut unknown_entry = unsafe { self.entries[self.index].unknown };

        while !unknown_entry.is_end() {
            if unknown_entry.is_deleted() {
                self.index += 1;
                unknown_entry = unsafe { self.entries[self.index].unknown };
                continue;
            }

            // 13 (5+6+2) characters in LFN entry. Up to 20 LFN entries can be chained.
            let mut lfn_name = [0u16; 13 * 20];
            let mut lfn_found = false;

            while unknown_entry.is_lfn() {
                let lfn = unsafe { self.entries[self.index].long_filename };
                if !lfn.is_deleted() {
                    lfn_found = true;
                    let pos = ((lfn.seq_number & 0b11111) as usize - 1) * 13;
                    lfn_name[pos..pos + 5].copy_from_slice(&lfn.name1);
                    lfn_name[pos + 5..pos + 11].copy_from_slice(&lfn.name2);
                    lfn_name[pos + 11..pos + 13].copy_from_slice(&lfn.name3);
                }
                self.index += 1;
                unknown_entry = unsafe { self.entries[self.index].unknown };
            }

            //TODO regular entry

            let name = if lfn_found {
                // File name can be terminated using 0x0000 or 0xFFFF
                decode_utf16(
                    lfn_name
                        .iter()
                        .take_while(|x| **x != 0x0000 && **x != 0xFFFF)
                        .cloned(),
                ).map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
                    .collect::<String>()
            } else {
                String::new()
            };
        }
        None
    }
}

impl Dir {
    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry> {
        unimplemented!("Dir::find()")
    }
}

// impl traits::Dir for Dir {
//     /// The type of entry stored in this directory.
//     type Entry = Entry;

//     /// An type that is an iterator over the entries in this directory.
//     type Iter = EntryIter;

//     /// Returns an interator over the entries in this directory.
//     fn entries(&self) -> io::Result<Self::Iter> {
//         let mut buf = Vec::new();
//         self.vfat.borrow_mut().read_chain(self.cluster, &mut buf)?;
//     }
// }
