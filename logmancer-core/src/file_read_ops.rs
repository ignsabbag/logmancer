use crate::log_file::LogFile;
use std::io;
use std::sync::RwLockReadGuard;

pub struct FileReadOps<'a> {
    log_file: RwLockReadGuard<'a, LogFile>
}

const LINE_MAX_BYTES: usize = 10 * 1024;

impl<'a> FileReadOps<'a> {

    pub fn new(log_file: RwLockReadGuard<'a, LogFile>) -> Self {
        FileReadOps {log_file}
    }

    /// Reads the line number `line_number` from the file.
    pub fn read_line(&self, line_number: usize) -> io::Result<String> {
        if line_number >= self.log_file.index.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Unexpected end of file"));
        }

        let start_pos = self.log_file.index[line_number];
        let line_size = if line_number + 1 == self.log_file.index.len() {
            self.log_file.mmap.len() // Last line
        } else {
            self.log_file.index[line_number + 1]
        }.saturating_sub(start_pos);
        let end_pos =
            line_size.min(LINE_MAX_BYTES)        // Max line size validation
                .saturating_add(start_pos)       // End position
                .min(self.log_file.mmap.len());         // Max file size validation

        let bytes = &self.log_file.mmap[start_pos..end_pos];

        Ok(String::from_utf8_lossy(bytes).trim_end().to_owned())
    }

    /// Returns the total number of lines indexed.
    /// This may not be the total number of lines in the file if indexing is in progress.
    pub fn total_lines(&self) -> io::Result<usize> {
        Ok(self.log_file.index.len())
    }

    pub fn indexing_progress(&self) -> io::Result<f64> {
        let file_size = self.log_file.mmap.len();
        if file_size == 0 {
            return Ok(1.0);
        }
        let indexed = *self.log_file.index.last().unwrap();
        Ok(indexed as f64 / file_size as f64)
    }
}