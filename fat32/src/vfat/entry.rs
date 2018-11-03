use traits;
use vfat::{Cluster, Dir, File, Metadata};

// TODO: You may need to change this definition.
#[derive(Debug)]
pub enum Entry {
    File(File),
    Dir(Dir),
}

// TODO: Implement any useful helper methods on `Entry`.

impl traits::Entry for Entry {
    type File = File;
    type Dir = Dir;
    type Metadata = Metadata;

    /// The name of the file or directory corresponding to this entry.
    fn name(&self) -> &str {
        match self {
            &Entry::Dir(ref d) => d.name(),
            &Entry::File(ref f) => f.name(),
        }
    }

    /// The metadata associated with the entry.
    fn metadata(&self) -> &Self::Metadata {
        match self {
            &Entry::Dir(ref d) => d.metadata(),
            &Entry::File(ref f) => f.metadata(),
        }
    }

    /// If `self` is a file, returns `Some` of a reference to the file.
    /// Otherwise returns `None`.
    fn as_file(&self) -> Option<&Self::File> {
        match self {
            &Entry::File(ref f) => Some(f),
            _ => None,
        }
    }

    /// If `self` is a directory, returns `Some` of a reference to the
    /// directory. Otherwise returns `None`.
    fn as_dir(&self) -> Option<&Self::Dir> {
        match self {
            &Entry::Dir(ref d) => Some(d),
            _ => None,
        }
    }

    /// If `self` is a file, returns `Some` of the file. Otherwise returns
    /// `None`.
    fn into_file(self) -> Option<Self::File> {
        match self {
            Entry::File(f) => Some(f),
            _ => None,
        }
    }

    /// If `self` is a directory, returns `Some` of the directory. Otherwise
    /// returns `None`.
    fn into_dir(self) -> Option<Self::Dir> {
        match self {
            Entry::Dir(d) => Some(d),
            _ => None,
        }
    }
}
