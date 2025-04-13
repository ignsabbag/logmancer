use memmap2::Mmap;
use std::fs::{metadata, File};
use std::io::{self, BufRead};
use std::path::Path;

pub struct PageResult {
    pub lines: Vec<String>,
    pub start_line: usize,
    pub total_lines: usize,
}

pub struct PagedFileReader {
    path: String,
    size: u64,
    mmap: Mmap,
    line_offsets: Vec<usize>
}

impl PagedFileReader {

    pub fn from(path: String) -> io::Result<Self> {
        let size = metadata(Path::new(&path))?.len();
        let file = File::open(&path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let mut line_offsets: Vec<usize> = Vec::new();

        // TODO: crossbeam-channel
        line_offsets.push(0);
        line_offsets.extend(Self::get_offsets(&mmap, 0));

        Ok(PagedFileReader { path, size, mmap, line_offsets })
    }

    /// Reads a page from the file, starting at `start_line` and reading up to `max_lines` lines.
    pub fn read_page(&mut self, start_line: usize, max_lines: usize) -> io::Result<PageResult> {
        let mut result_lines = Vec::with_capacity(max_lines);
        for current_line in start_line..start_line + max_lines {
            if let Some(line) = self.read_line(current_line) {
                result_lines.push(line);
            } else {
                break;
            }
        }
        Ok(PageResult {
            lines: result_lines,
            start_line,
            total_lines: self.line_offsets.len(),
        })
    }

    // Reads the last `max_lines` lines from the file. If `follow` is true the file is reloaded
    pub fn tail(&mut self, max_lines: usize, follow: bool) -> io::Result<PageResult> {
        if follow && self.file_changed()? {
            self.update_offsets()?;
        }
        let mut result_lines = Vec::with_capacity(max_lines);
        let start_line = self.line_offsets.len() - max_lines;
        let mut line_number = start_line;
        while let Some(line) = self.read_line(line_number) {
            result_lines.push(line);
            line_number += 1;
        }
        Ok(PageResult {
            lines: result_lines,
            start_line,
            total_lines: self.line_offsets.len(),
        })
    }

    fn file_changed(&mut self) -> io::Result<bool> {
        let current_size = metadata(Path::new(&self.path))?.len();
        Ok(current_size > self.size)
    }

    /// Reads the line number `line_number` from the file.
    pub fn read_line(&self, line_number: usize) -> Option<String> {
        if line_number + 1 >= self.line_offsets.len() {
            return None;
        }

        let line_start = self.line_offsets[line_number];
        let line_end = self.line_offsets[line_number + 1];
        let bytes = &self.mmap[line_start..line_end];

        Some(String::from_utf8_lossy(bytes).trim_end().to_owned())
    }

    fn update_offsets(&mut self) -> io::Result<()> {
        let file = File::open(&self.path)?;
        self.size = file.metadata()?.len();
        self.mmap = unsafe { Mmap::map(&file)? };
        if let Some(start_pos) = self.line_offsets.last() {
            let new_content = &self.mmap[start_pos.clone()..];
            let new_offsets = Self::get_offsets(new_content, *start_pos);
            self.line_offsets.extend(new_offsets);
        }
        Ok(())
    }

    fn get_offsets<T: PartialEq<u8>>(array: &[T], base_offset: usize) -> Vec<usize> {
        let mut line_offsets = Vec::new();
        for (pos, byte) in array.iter().enumerate() {
            if *byte == b'\n' {
                line_offsets.push(base_offset + pos + 1);
            }
        }
        line_offsets
    }
}