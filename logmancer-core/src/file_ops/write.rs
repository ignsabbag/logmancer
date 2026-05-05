use crate::models::log_file::LogFile;
use memmap2::Mmap;
use regex::Regex;
use std::cmp::min;
use std::fs::{metadata, File};
use std::io;
use std::path::Path;
use std::sync::{Arc, RwLock};

const INDEX_MAX_BYTES: usize = 1024 * 1024; // 1MB
const INDEX_MAX_LINES: usize = 1000;

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

    /// Indexes lines up to a maximum of INDEX_MAX_BYTES bytes. Returns false unless the end of the file is reached.
    pub fn index_lines(&mut self) -> io::Result<bool> {
        let mut file_lock = self.log_file.write().unwrap();
        
        let start_pos = *file_lock.index.last().unwrap();
        let end_pos = min(file_lock.mmap.len(), start_pos + INDEX_MAX_BYTES);
        let end_reached = file_lock.mmap.len() <= start_pos + INDEX_MAX_BYTES;
        let content = &file_lock.mmap[start_pos..end_pos];

        let mut index = Vec::new();
        for (pos, byte) in content.iter().enumerate() {
            if *byte == b'\n' {
                index.push(start_pos + pos + 1);
            }
        }
        file_lock.index.extend(index);
        Ok(end_reached)
    }

    /// Sets the regex pattern and resets the filter index if a pattern is provided.
    pub fn filter(&mut self, pattern: Option<String>) -> io::Result<()> {
        if let Some(pat) = pattern {
            let mut file_lock = self.log_file.write().unwrap();
            file_lock.regex = Some(pat);
            file_lock.filter.clear();
        }
        Ok(())
    }

    /// Indexes filtered lines up to a maximun of INDEX_MAX_LINES lines. Returns false unless the end of the file is reached.
    pub fn index_filter(&mut self) -> io::Result<bool> {
        let mut file_lock = self.log_file.write().unwrap();

        let pattern = match &file_lock.regex {
            Some(pat) => pat,
            None => return Ok(true),
        };
        let re = Regex::new(pattern).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        let start_line = file_lock.filter.len();
        let total_lines = file_lock.index.len();
        let end_line = min(total_lines - 1, start_line + INDEX_MAX_LINES);

        for i in start_line..end_line {
            let start_pos = file_lock.index[i];
            let end_pos = file_lock.index[i + 1];
            let line = &file_lock.mmap[start_pos..end_pos];
            let mut match_filter = false;
            if let Ok(text) = std::str::from_utf8(line)
                && re.is_match(text)
            {
                match_filter = true;
            }
            file_lock.filter.push(match_filter);
        }

        Ok(end_line == total_lines - 1)
    }
}
