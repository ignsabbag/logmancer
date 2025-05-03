use memmap2::Mmap;
use std::fs::File;
use std::io;

/// Holds mmap and index of the file. It's no thread safe.
pub struct LogFile {
    pub path: String,
    pub index: Vec<usize>,
    pub mmap: Mmap,
    pub size: u64
}

impl LogFile {

    pub fn new(path: String) -> io::Result<LogFile> {
        let file = File::open(&path)?;
        let mut index = Vec::<usize>::new();
        index.push(0);
        Ok(LogFile {
            path,
            index,
            mmap: unsafe { Mmap::map(&file)? },
            size: file.metadata()?.len()
        })
    }

}