use std::cmp::min;
use std::io;
use std::mem::size_of;
use std::path::{Component, Path};

use mbr::MasterBootRecord;
use traits::{BlockDevice, FileSystem};
use util::SliceExt;
use vfat::{BiosParameterBlock, CachedDevice, Partition};
use vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Shared, Status};

#[derive(Debug)]
pub struct VFat {
    device: CachedDevice,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    pub root_dir_cluster: Cluster,
}

impl VFat {
    pub fn from<T>(mut device: T) -> Result<Shared<VFat>, Error>
    where
        T: BlockDevice + 'static,
    {
        let mbr = MasterBootRecord::from(&mut device)?;
        let sector = mbr.first_fat32()?.sector();
        let ebpb = BiosParameterBlock::from(&mut device, sector)?;
        let fat_start_sector = sector + ebpb.sectors_reserved as u64;
        let data_start_sector =
            fat_start_sector + ebpb.fats_number as u64 * ebpb.sectors_per_fat as u64;

        let partition = Partition {
            start: sector,
            sector_size: ebpb.bytes_per_sector as u64,
        };

        let cache_device = CachedDevice::new(device, partition);

        Ok(Shared::new(VFat {
            device: cache_device,
            bytes_per_sector: ebpb.bytes_per_sector,
            sectors_per_cluster: ebpb.sectors_per_cluster,
            sectors_per_fat: ebpb.sectors_per_fat,
            fat_start_sector: sector + ebpb.sectors_reserved as u64,
            data_start_sector,
            root_dir_cluster: Cluster::from(ebpb.root_dir_cluster),
        }))
    }

    /// A method to read from an offset of a cluster into a buffer.
    pub fn read_cluster(
        &mut self,
        cluster: Cluster,
        offset: usize,
        buf: &mut [u8],
    ) -> io::Result<usize> {
        let first_sector_of_cluster =
            self.data_start_sector + cluster.data_index()? as u64 * self.sectors_per_cluster as u64;
        let last_sector_of_cluster = first_sector_of_cluster + self.sectors_per_cluster as u64;

        let start_sector = first_sector_of_cluster + offset as u64;

        let buf_size_in_sectors = buf.len() as u64 / self.bytes_per_sector as u64;
        let last_sector_to_read = min(last_sector_of_cluster, start_sector + buf_size_in_sectors);

        let mut read = 0;
        for sec in start_sector..last_sector_to_read {
            read += self.device.read_sector(sec, &mut buf[read..])?;
        }

        Ok(read)
    }

    /// A method to read all of the clusters chained from a starting cluster
    /// into a vector.
    pub fn read_chain(&mut self, start: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
        let mut cluster = start;
        let mut read = 0;

        while let Status::Data(next_cluster) = self.fat_entry(cluster)?.status() {
            let buf_len = buf.len();
            buf.resize(
                buf_len + self.bytes_per_sector as usize * self.sectors_per_cluster as usize,
                0,
            );
            read += self.read_cluster(cluster, 0, &mut buf[read..])?;
            cluster = next_cluster;
        }

        match self.fat_entry(cluster)?.status() {
            Status::Eoc(_eoc) => {
                let buf_len = buf.len();
                buf.resize(
                    buf_len + self.bytes_per_sector as usize * self.sectors_per_cluster as usize,
                    0,
                );
                read += self.read_cluster(cluster, 0, &mut buf[read..])?;
            }
            Status::Free => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "can't read from free sector",
                ))
            }
            Status::Bad => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "can't read from bad sector",
                ))
            }
            Status::Reserved => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "can't read from reserved sector",
                ))
            }
            Status::Data(_next_cluster) => unreachable!(),
        }

        Ok(read)
    }

    /// A method to return a reference to a `FatEntry` for a cluster where the
    /// reference points directly into a cached sector.
    pub fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> {
        let cluster_index = cluster.fat_index() as usize;
        let fat_entries_per_sector = self.bytes_per_sector as usize / size_of::<FatEntry>();

        let sector_of_fat_entry = cluster_index / fat_entries_per_sector;

        let sector = self
            .device
            .get(self.fat_start_sector + sector_of_fat_entry as u64)?;
        let fat_entries: &[FatEntry] = unsafe { sector.cast() };

        let fat_entry_index_in_sector = cluster_index % fat_entries_per_sector;
        Ok(&fat_entries[fat_entry_index_in_sector])
    }
}

impl<'a> FileSystem for &'a Shared<VFat> {
    type File = File;
    type Dir = Dir;
    type Entry = Entry;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        use traits::Entry;
        use vfat::Entry as VFatEntry;

        let mut cur_dir = VFatEntry::Dir(Dir::root(self.clone()));

        for comp in path.as_ref().components() {
            match comp {
                Component::Normal(name) => {
                    cur_dir = cur_dir
                        .as_dir()
                        .ok_or(io::Error::new(io::ErrorKind::NotFound, "File not found"))?
                        .find(name)?
                }
                Component::RootDir => {}
                Component::CurDir => unimplemented!("CurDir"),
                Component::ParentDir => unimplemented!("ParentDir"),
                Component::Prefix(_) => unimplemented!("Prefix"),
            }
        }
        Ok(cur_dir)
    }

    fn create_file<P: AsRef<Path>>(self, _path: P) -> io::Result<Self::File> {
        unimplemented!("read only file system")
    }

    fn create_dir<P>(self, _path: P, _parents: bool) -> io::Result<Self::Dir>
    where
        P: AsRef<Path>,
    {
        unimplemented!("read only file system")
    }

    fn rename<P, Q>(self, _from: P, _to: Q) -> io::Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        unimplemented!("read only file system")
    }

    fn remove<P: AsRef<Path>>(self, _path: P, _children: bool) -> io::Result<()> {
        unimplemented!("read only file system")
    }
}
