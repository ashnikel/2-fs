use std::ffi::OsStr;
use std::char::{decode_utf16, REPLACEMENT_CHARACTER};
// use std::borrow::Cow;
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
    attr: Attributes,
    reserved: u8,
    ctime_fine: u8,
    ctime: Time,
    cdate: Date,
    adate: Date,
    cluster_hi: u16,
    mtime: Time,
    mdate: Date,
    cluster_lo: u16,
    size: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    seq_number: u8,
    name1: [u16; 5],
    attr: Attributes,
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

    pub fn is_lfn(&self) -> bool {
        self.attr == 0x0F
    }
}

impl VFatRegularDirEntry {
    pub fn metadata(&self) -> Metadata {
        Metadata {
            attr: self.attr,
            created: Timestamp {
                date: self.cdate,
                time: self.ctime,
            },
            accessed: Timestamp {
                date: self.adate,
                time: Time(0),
            },
            modified: Timestamp {
                date: self.mdate,
                time: self.mtime,
            },
        }
    }

    pub fn cluster(&self) -> Cluster {
        let cluster = ((self.cluster_hi as u32) << 16) | self.cluster_lo as u32;
        Cluster::from(cluster)
    }

    pub fn is_dir(&self) -> bool {
        self.attr.0 as u8 & 0x10 != 0
    }
}

impl VFatLfnDirEntry {
    pub fn is_deleted(&self) -> bool {
        self.seq_number == 0xE5
    }
}

pub fn ucs_2_to_string(arr: &[u16]) -> String {
    // File name in LFN entry can be terminated using 0x0000 or 0xFFFF
    decode_utf16(
        arr.iter()
            .take_while(|x| **x != 0x0000 && **x != 0xFFFF)
            .cloned(),
    ).map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
        .collect::<String>()
}

pub fn ascii_to_string(arr: &[u8]) -> Option<String> {
    let s = arr.iter()
        .take_while(|x| **x != 0x00 && **x != 0x20)
        .map(|&c| c as char)
        .collect::<String>();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

impl Iterator for EntryIter {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        let mut unknown_entry = unsafe { self.entries[self.index].unknown };

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

        while !unknown_entry.is_end() {
            if unknown_entry.is_deleted() {
                self.index += 1;
                unknown_entry = unsafe { self.entries[self.index].unknown };
                continue;
            }

            let regular = unsafe { self.entries[self.index].regular };

            let name = if lfn_found {
                ucs_2_to_string(&lfn_name)
            } else {
                match ascii_to_string(&regular.ext) {
                    None => ascii_to_string(&regular.name).unwrap(),
                    Some(ext) => {
                        let mut s = ascii_to_string(&regular.name).unwrap();
                        s.push('.');
                        s.push_str(&ext);
                        s
                    }
                }
            };

            let metadata = regular.metadata();
            let cluster = regular.cluster();

            if regular.is_dir() {
                return Some(Entry::Dir(Dir {
                    name,
                    cluster,
                    vfat: self.vfat.clone(),
                    metadata,
                }));
            } else {
                return Some(Entry::File(File {
                    name,
                    cluster,
                    vfat: self.vfat.clone(),
                    metadata,
                    size: regular.size,
                }));
            }
        }

        None
    }
}

impl Dir {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn root(vfat: Shared<VFat>) -> Dir {
        Dir {
            name: String::from("/"),
            cluster: vfat.borrow().root_dir_cluster,
            vfat: vfat.clone(),
            metadata: Metadata::default(),
        }
    }

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
        let name = name.as_ref().to_str().ok_or(io::Error::new(io::ErrorKind::InvalidInput, "Invalid UTF-8"))?;

        use traits::{Dir, Entry};
        for entry in self.entries()? {
            if entry.name().eq_ignore_ascii_case(name) {
                return Ok(entry);
            }
        }

        Err(io::Error::new(io::ErrorKind::NotFound, "Entry not found"))
    }
}

impl traits::Dir for Dir {
    /// The type of entry stored in this directory.
    type Entry = Entry;

    /// An type that is an iterator over the entries in this directory.
    type Iter = EntryIter;

    /// Returns an interator over the entries in this directory.
    fn entries(&self) -> io::Result<Self::Iter> {
        let mut buf = Vec::new();
        self.vfat.borrow_mut().read_chain(self.cluster, &mut buf)?;
        Ok(EntryIter {
            entries: unsafe { buf.cast() },
            index: 0,
            vfat: self.vfat.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_to_string() {
        let arr: [u8; 8] = [0x48, 0x45, 0x4C, 0x4C, 0x4F, 0x20, 0x4F, 0x00];
        assert_eq!(ascii_to_string(&arr), Some("HELLO".to_string()));

        let arr: [u8; 8] = [0x00, 0x45, 0x4C, 0x4C, 0x4F, 0x20, 0x00, 0x00];
        assert_eq!(ascii_to_string(&arr), None);
    }

    #[test]
    fn test_ucs_2_to_string() {
        let arr = [
            0xD834, 0x041F, 0x0440, 0x0438, 0x0432, 0x0435, 0x0442, 0xDD1E, 0x0000, 0x0072
        ];
        assert_eq!(ucs_2_to_string(&arr), "�Привет�".to_string());
    }
}
