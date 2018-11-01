use std::fmt;

use traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(pub u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(pub u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    pub attr: Attributes,
    pub created: Timestamp,
    pub accessed: Timestamp,
    pub modified: Timestamp,
}

impl traits::Timestamp for Timestamp {
    /// The calendar year.
    ///
    /// The year is not offset. 2009 is 2009.
    fn year(&self) -> usize {
        ((self.date.0 & 0b1111_1110_0000_0000) >> 9) as usize + 1980
    }

    /// The calendar month, starting at 1 for January. Always in range [1, 12].
    ///
    /// January is 1, Feburary is 2, ..., December is 12.
    fn month(&self) -> u8 {
        ((self.date.0 & 0b0000_0001_1110_0000) >> 5) as u8 + 1
    }

    /// The calendar day, starting at 1. Always in range [1, 31].
    fn day(&self) -> u8 {
        (self.date.0 & 0b0000_0000_0001_1111) as u8 + 1
    }

    /// The 24-hour hour. Always in range [0, 24).
    fn hour(&self) -> u8 {
        ((self.time.0 & 0b1111_1000_0000_0000) >> 11) as u8
    }

    /// The minute. Always in range [0, 60).
    fn minute(&self) -> u8 {
        ((self.time.0 & 0b0000_0111_1110_0000) >> 5) as u8
    }

    /// The second. Always in range [0, 60).
    fn second(&self) -> u8 {
        (self.time.0 & 0b0000_0000_0001_1111) as u8 * 2
    }
}

impl traits::Metadata for Metadata {
    /// Type corresponding to a point in time.
    type Timestamp = Timestamp;

    /// Whether the associated entry is read only.
    fn read_only(&self) -> bool {
        self.attr.0 & 0x01 == 0x01
    }

    /// Whether the entry should be "hidden" from directory traversals.
    fn hidden(&self) -> bool {
        self.attr.0 & 0x02 == 0x02
    }

    /// The timestamp when the entry was created.
    fn created(&self) -> Self::Timestamp {
        self.created
    }

    /// The timestamp for the entry's last access.
    fn accessed(&self) -> Self::Timestamp {
        self.accessed
    }

    /// The timestamp for the entry's last modification.
    fn modified(&self) -> Self::Timestamp {
        self.modified
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use traits::{Metadata, Timestamp};

        let r = if self.read_only() { 'R' } else { '-' };
        let h = if self.hidden() { 'H' } else { '-' };
        write!(
            f,
            "{}{} {}.{}.{} {}:{}:{}",
            r,
            h,
            self.created().day(),
            self.created().month(),
            self.created().year(),
            self.created().hour(),
            self.created().minute(),
            self.created().second()
        )
    }
}
