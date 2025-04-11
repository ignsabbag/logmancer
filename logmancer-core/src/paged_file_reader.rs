use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use regex::Regex;

pub struct PageResult {
    pub lines: Vec<String>,
    pub start_line: usize,
    pub total_lines: usize,
}

pub struct PagedFileReader {
    file: File,
    reader: BufReader<File>,
    index: Option<(Vec<u64>, usize)>, // (index, granularity)
}

impl PagedFileReader {

    pub fn from(path: &str) -> io::Result<Self> {
        let metadata = std::fs::metadata(Path::new(path))?;
        let bytes = metadata.len();
        let file_size = bytes / (1024 * 1024); //MB

        if file_size < 2 {
            Self::new(path)
        } else if file_size < 10 {
            Self::indexed(path, 100)
        } else if file_size < 100 {
            Self::indexed(path, 500)
        } else {
            Self::indexed(path, 1000)
        }
    }

    /// New `PagedFileReader` without indexes
    pub fn new(path: &str) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file.try_clone()?);
        Ok(PagedFileReader {
            file,
            reader,
            index: None,
        })
    }

    /// Creates a new `PagedFileReader` with indexes.
    /// `granularity` indicates how many lines an offset is saved.
    pub fn indexed(path: &str, granularity: usize) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut reader = BufReader::new(file.try_clone()?);
        let mut index = Vec::new();

        let mut pos = reader.stream_position()?;
        let mut line_count = 0;
        index.push(pos);

        loop {
            let mut buf = String::new();
            let bytes_read = reader.read_line(&mut buf)?;
            if bytes_read == 0 {
                break;
            }
            line_count += 1;
            pos += bytes_read as u64;
            if line_count % granularity == 0 {
                index.push(pos);
            }
        }

        file.seek(SeekFrom::Start(0))?;
        let reader = BufReader::new(file.try_clone()?);

        Ok(PagedFileReader {
            file,
            reader,
            index: Some((index, granularity)),
        })
    }

    /// Reads a page from the file, starting at `start_line` and reading up to `max_lines` lines.
    /// If the index has been built, it is used to perform a fast jump.
    pub fn read_page(&mut self, start_line: usize, max_lines: usize) -> io::Result<PageResult> {
        if let Some((ref index, granularity)) = self.index {
            let block = start_line / granularity;
            if block < index.len() {
                let offset = index[block];
                self.file.seek(SeekFrom::Start(offset))?;
                self.reader = BufReader::new(self.file.try_clone()?);
                for _ in 0..(start_line - block * granularity) {
                    let mut dummy = String::new();
                    if self.reader.read_line(&mut dummy)? == 0 {
                        break;
                    }
                }
            } else {
                return Ok(PageResult {
                    lines: vec![],
                    start_line,
                    total_lines: start_line,
                });
            }
        } else {
            self.file.seek(SeekFrom::Start(0))?;
            self.reader = BufReader::new(self.file.try_clone()?);
            for _ in 0..start_line {
                let mut dummy = String::new();
                if self.reader.read_line(&mut dummy)? == 0 {
                    break;
                }
            }
        }

        let mut result_lines = Vec::with_capacity(max_lines);
        for _ in 0..max_lines {
            let mut line = String::new();
            let bytes = self.reader.read_line(&mut line)?;
            if bytes == 0 {
                break;
            }
            result_lines.push(line.trim_end().to_owned());
        }

        let total_lines = self.count_lines()?; // TODO: Improve
        Ok(PageResult {
            lines: result_lines,
            start_line,
            total_lines,
        })
    }

    pub fn read_filter_page(
        &mut self,
        start_line: usize,
        max_lines: usize,
        filter_pattern: &str,
    ) -> io::Result<Vec<String>> {
        let re = Regex::new(filter_pattern).unwrap(); // TODO: Validate regex
        let mut filtered_lines = Vec::with_capacity(max_lines);
        let mut match_count = 0;
        self.file.seek(SeekFrom::Start(0))?;
        self.reader = BufReader::new(self.file.try_clone()?);
        for line_result in self.reader.by_ref().lines() {
            let line = line_result?;
            if re.is_match(&line) {
                if match_count >= start_line && filtered_lines.len() < max_lines {
                    filtered_lines.push(line);
                }
                match_count += 1;
                if filtered_lines.len() >= max_lines {
                    break;
                }
            }
        }
        Ok(filtered_lines)
    }

    /// Returns the last `max_lines` lines in the file.
    /// This method implicitly reindexes the file as it traverses it.
    pub fn tail(&mut self, max_lines: usize) -> io::Result<PageResult> {
        self.file.seek(SeekFrom::Start(0))?;
        self.reader = BufReader::new(self.file.try_clone()?);
        let mut total_lines = 0;
        let mut tail_lines = Vec::new();

        for line_result in self.reader.by_ref().lines() {
            let line = line_result?;
            total_lines += 1;
            if tail_lines.len() == max_lines {
                tail_lines.remove(0);
            }
            tail_lines.push(line.trim_end().to_owned());
        }

        let start_line = if total_lines >= tail_lines.len() {
            total_lines - tail_lines.len()
        } else {
            0
        };

        // TODO: Update index if it exists

        self.file.seek(SeekFrom::Start(0))?;
        self.reader = BufReader::new(self.file.try_clone()?);

        Ok(PageResult {
            lines: tail_lines,
            start_line,
            total_lines,
        })
    }

    /// Auxiliar para contar el total de líneas del archivo.
    fn count_lines(&mut self) -> io::Result<usize> {
        self.file.seek(SeekFrom::Start(0))?;
        self.reader = BufReader::new(self.file.try_clone()?);
        let mut count = 0;
        for line in self.reader.by_ref().lines() {
            line?;
            count += 1;
        }
        self.file.seek(SeekFrom::Start(0))?;
        self.reader = BufReader::new(self.file.try_clone()?);
        Ok(count)
    }
}