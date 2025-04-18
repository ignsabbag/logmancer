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
        let mut file_lock = self.log_file.write().unwrap();
        let start_pos = file_lock.index.last().unwrap();
        let end_pos = min(file_lock.mmap.len(), *start_pos + max_bytes);
        let end_reached = file_lock.mmap.len() <= *start_pos + max_bytes;

        let mut new_index = Vec::new();
        let content = &file_lock.mmap[*start_pos..end_pos];
        for (pos, byte) in content.iter().enumerate() {
            if *byte == b'\n' {
                new_index.push(start_pos + pos + 1);
            }
        }
        file_lock.index.extend(new_index);
        Ok(end_reached)
    }
}