use std::cmp::min;
use std::io::{self, SeekFrom};

use traits;
use vfat::{Cluster, Metadata, Shared, VFat};

#[derive(Debug)]
pub struct File {
    pub name: String,
    pub cluster: Cluster,
    pub vfat: Shared<VFat>,
    pub metadata: Metadata,
    pub size: usize,
    pub read_ptr: usize,
}

impl File {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }
}

// FIXME: Implement `traits::File` (and its supertraits) for `File`.
impl traits::File for File {
    /// Writes any buffered data to disk.
    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }

    /// Returns the size of the file in bytes.
    fn size(&self) -> u64 {
        self.size as u64
    }
}

impl io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.size == 0 {
            return Ok(0);
        }

        let mut buf_vec = Vec::new();
        self.vfat
            .borrow_mut()
            .read_chain(self.cluster, &mut buf_vec)?;
        let left_to_read = self.size - self.read_ptr;
        let bytes_to_copy = min(left_to_read, buf.len());

        buf[..bytes_to_copy]
            .copy_from_slice(&buf_vec[self.read_ptr..self.read_ptr + bytes_to_copy]);
        self.read_ptr += bytes_to_copy;

        Ok(bytes_to_copy)
    }
}

impl io::Write for File {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        unimplemented!()
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Seek for File {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        unimplemented!("File::seek()")
    }
}
