use crate::models::log_file::LogFile;
use std::io;
use std::sync::RwLockReadGuard;

pub struct FileReadOps<'a> {
    log_file: RwLockReadGuard<'a, LogFile>,
}

const LINE_MAX_BYTES: usize = 10 * 1024;

impl<'a> FileReadOps<'a> {
    pub fn new(log_file: RwLockReadGuard<'a, LogFile>) -> Self {
        FileReadOps { log_file }
    }

    pub fn file_path(&self) -> String {
        self.log_file.path.clone()
    }

    /// Reads the line number `line_number` from the file.
    pub fn read_line(&self, line_number: usize) -> io::Result<String> {
        if line_number >= self.log_file.index.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Unexpected end of file",
            ));
        }

        let start_pos = self.log_file.index[line_number];
        let line_size = if line_number + 1 == self.log_file.index.len() {
            self.log_file.mmap.len() // Last line
        } else {
            self.log_file.index[line_number + 1]
        }
        .saturating_sub(start_pos);
        let end_pos = line_size
            .min(LINE_MAX_BYTES) // Max line size validation
            .saturating_add(start_pos) // End position
            .min(self.log_file.mmap.len()); // Max file size validation

        let bytes = &self.log_file.mmap[start_pos..end_pos];

        Ok(String::from_utf8_lossy(bytes).trim_end().to_owned())
    }

    pub fn read_filter_line(&self, line_number: usize) -> io::Result<Option<String>> {
        if self.log_file.filter[line_number] {
            Ok(Some(self.read_line(line_number)?))
        } else {
            Ok(None)
        }
    }

    /// Returns the total number of lines indexed.
    /// This may not be the total number of lines in the file if indexing is in progress.
    pub fn total_lines(&self) -> io::Result<usize> {
        Ok(self.log_file.index.len())
    }

    /// Returns the total number of lines indexed.
    /// This may not be the total number of matches if filter indexing is in progress.
    pub fn filtered_lines(&self) -> io::Result<usize> {
        Ok(self
            .log_file
            .filter
            .iter()
            .filter(|matched| **matched)
            .count())
    }

    /// Returns how many source lines have already been processed by the filter worker.
    pub fn processed_filter_lines(&self) -> io::Result<usize> {
        Ok(self.log_file.filter.len())
    }

    pub fn indexing_progress(&self) -> io::Result<f64> {
        let file_size = self.log_file.mmap.len();
        if file_size == 0 {
            return Ok(1.0);
        }
        let indexed = *self.log_file.index.last().unwrap();
        Ok(indexed as f64 / file_size as f64)
    }

    pub fn filter_indexing_progress(&self) -> io::Result<f64> {
        let file_size = self.log_file.mmap.len();
        if file_size == 0 {
            return Ok(1.0);
        }
        let indexed = self.log_file.index[self.log_file.filter.len()];
        Ok(indexed as f64 / file_size as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::log_file::LogFile;
    use std::fs::File;
    use std::io::Write;
    use std::sync::RwLock;

    #[test]
    fn test_read_line() {
        // Prepara un archivo temporal
        let path = "test_log.txt";
        let mut file = File::create(path).unwrap();
        writeln!(file, "line1").unwrap();

        let log_file = LogFile::new(path.to_string()).unwrap();
        let lock = RwLock::new(log_file);
        let read_ops = FileReadOps::new(lock.read().unwrap());

        assert_eq!(read_ops.read_line(0).unwrap(), "line1");

        std::fs::remove_file(path).unwrap();
    }
}
