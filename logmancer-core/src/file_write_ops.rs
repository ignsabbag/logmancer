use crate::log_file::LogFile;
use memmap2::Mmap;
use std::cmp::min;
use std::fs::{metadata, File};
use std::io;
use std::path::Path;
use std::sync::{Arc, RwLock};

pub struct FileWriteOps {
    log_file: Arc<RwLock<LogFile>>
}

impl FileWriteOps {

    pub fn new(log_file: Arc<RwLock<LogFile>>) -> Self {
        FileWriteOps {log_file}
    }

    /// Checks the file size and resets mmap and size
    pub fn reload(&mut self) -> io::Result<()> {
        let mut file_lock = self.log_file.write().unwrap();
        let current_size = metadata(Path::new(&file_lock.path))?.len();
        if current_size > file_lock.size {
            let file = File::open(&file_lock.path)?;
            file_lock.mmap = unsafe { Mmap::map(&file)? };
            file_lock.size = current_size;
            Ok(())
        } else if current_size < file_lock.size {
            Err(io::Error::new(io::ErrorKind::InvalidData, "File changed"))
        } else {
            Ok(())
        }
    }

    /// Indexes the next `max_bytes` bytes. Returns true unless the end of the file is reached.
    pub fn index_max(&mut self, max_bytes: usize) -> io::Result<bool> {
        let (gap, end_reached) = self.index(max_bytes)?;
        self.log_file.write().unwrap().index.extend(gap);
        Ok(end_reached)
    }

    fn index(&self, max_bytes: usize) -> io::Result<(Vec<usize>, bool)> {
        let read_lock = self.log_file.write().unwrap();
        let start_pos = read_lock.index.last().unwrap().clone();
        let end_pos = min(read_lock.mmap.len(), start_pos + max_bytes);
        let end_reached = read_lock.mmap.len() <= start_pos + max_bytes;
        let content = &read_lock.mmap[start_pos..end_pos];

        let mut index = Vec::new();
        for (pos, byte) in content.iter().enumerate() {
            if *byte == b'\n' {
                index.push(start_pos + pos + 1);
            }
        }
        Ok((index, end_reached))
    }
}