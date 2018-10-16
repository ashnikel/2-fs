use std::io;
use std::path::Path;
use std::mem::size_of;
use std::cmp::min;

use util::SliceExt;
use mbr::MasterBootRecord;
use vfat::{Shared, Cluster, File, Dir, Entry, FatEntry, Error, Status};
use vfat::{BiosParameterBlock, CachedDevice, Partition};
use traits::{FileSystem, BlockDevice};

#[derive(Debug)]
pub struct VFat {
    device: CachedDevice,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    root_dir_cluster: Cluster,
}

impl VFat {
    pub fn from<T>(mut device: T) -> Result<Shared<VFat>, Error>
        where T: BlockDevice + 'static
    {
        let mbr = MasterBootRecord::from(&mut device)?;
        let sector = mbr.first_fat32()?.sector();
        let ebpb = BiosParameterBlock::from(&mut device, sector)?;

        let partition = Partition {
            start: sector,
            sector_size: ebpb.bytes_per_sector() as u64,
        };

        let cache_device = CachedDevice::new(device, partition);

        Ok(Shared::new(VFat {
            device: cache_device,
            bytes_per_sector: ebpb.bytes_per_sector(),
            sectors_per_cluster: ebpb.sectors_per_cluster(),
            sectors_per_fat: ebpb.sectors_per_fat(),
            fat_start_sector: ebpb.fat_start_sector(),
            data_start_sector: ebpb.data_start_sector(),
            root_dir_cluster: Cluster::from(ebpb.root_dir_cluster())
        }))
    }

    /// A method to read from an offset of a cluster into a buffer.
    fn read_cluster(
        &mut self,
        cluster: Cluster,
        offset: usize,
        buf: &mut [u8],
    ) -> io::Result<usize> {

        let first_sector_of_cluster = self.data_start_sector
            + cluster.data_index()? as u64 * self.sectors_per_cluster as u64;
        let last_sector_of_cluster = first_sector_of_cluster
            + self.sectors_per_cluster as u64;

        let start_sector = first_sector_of_cluster + offset as u64;

        let buf_size_in_sectors = buf.len() as u64 / self.bytes_per_sector as u64;
        let last_sector_to_read =
            min(last_sector_of_cluster, start_sector + buf_size_in_sectors);

        let mut read = 0;
        for sec in start_sector .. last_sector_to_read {
            read += self.device.read_sector(sec, &mut buf[read..])?;
        }

        Ok(read)
    }

    /// A method to read all of the clusters chained from a starting cluster
    /// into a vector.
    fn read_chain(
        &mut self,
        start: Cluster,
        buf: &mut Vec<u8>
    ) -> io::Result<usize> {

        let mut cluster = start;
        let mut read = 0;

        while let Status::Data(next_cluster) = self.fat_entry(cluster)?.status() {
            read += self.read_cluster(cluster, 0, &mut buf[read..])?;
            cluster = next_cluster;
        }

        match self.fat_entry(cluster)?.status() {
            Status::Eoc(eoc) => {
                read += self.read_cluster(cluster, 0, &mut buf[read..])?;
            },
            Status::Free  => return Err(io::Error::new(io::ErrorKind::Other,
                "can't read from free sector")),
            Status::Bad => return Err(io::Error::new(io::ErrorKind::Other,
                "can't read from bad sector")),
            Status::Reserved => return Err(io::Error::new(io::ErrorKind::Other,
                "can't read from reserved sector")),
            Status::Data(next_cluster) => unreachable!(),
        }

        Ok(read)
    }


    /// A method to return a reference to a `FatEntry` for a cluster where the
    /// reference points directly into a cached sector.
    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> {
        let cluster_index = cluster.fat_index() as usize;
        let fat_entries_per_sector = self.bytes_per_sector as usize
                                     / ::std::mem::size_of::<FatEntry>();

        let sector_of_fat_entry = cluster_index / fat_entries_per_sector;

        let sector = self.device.get(self.fat_start_sector
                                     + sector_of_fat_entry as u64)?;
        let fat_entries: &[FatEntry] = unsafe { sector.cast() };

        let fat_entry_index_in_sector = cluster_index % fat_entries_per_sector;
        Ok(&fat_entries[fat_entry_index_in_sector])
    }
}

impl<'a> FileSystem for &'a Shared<VFat> {
    type File = ::traits::Dummy;
    type Dir = ::traits::Dummy;
    type Entry = ::traits::Dummy;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        unimplemented!("FileSystem::open()")
    }

    fn create_file<P: AsRef<Path>>(self, _path: P) -> io::Result<Self::File> {
        unimplemented!("read only file system")
    }

    fn create_dir<P>(self, _path: P, _parents: bool) -> io::Result<Self::Dir>
        where P: AsRef<Path>
    {
        unimplemented!("read only file system")
    }

    fn rename<P, Q>(self, _from: P, _to: Q) -> io::Result<()>
        where P: AsRef<Path>, Q: AsRef<Path>
    {
        unimplemented!("read only file system")
    }

    fn remove<P: AsRef<Path>>(self, _path: P, _children: bool) -> io::Result<()> {
        unimplemented!("read only file system")
    }
}
