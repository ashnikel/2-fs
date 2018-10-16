use std::fmt;

use traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    // FIXME: Fill me in.
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
    fn day(&self) ->u8 {
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

// FIXME: Implement `traits::Metadata` for `Metadata`.

// FIXME: Implement `fmt::Display` (to your liking) for `Metadata`.
