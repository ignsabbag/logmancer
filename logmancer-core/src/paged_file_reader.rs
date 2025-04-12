use memmap2::Mmap;
use regex::Regex;
use std::fs::{metadata, File};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

pub struct PageResult {
    pub lines: Vec<String>,
    pub start_line: usize,
    pub total_lines: usize,
}

pub struct PagedFileReader {
    path: String,
    current_size: u64,
    mmap: Mmap,
    line_offsets: Vec<u64>
}

impl PagedFileReader {

    pub fn from(path: String) -> io::Result<Self> {
        let current_size = metadata(Path::new(&path))?.len();
        let file = File::open(&path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let mut line_offsets = Vec::new();

        // TODO: crossbeam-channel
        line_offsets.push(0);
        for (pos, byte) in mmap.iter().enumerate() {
            if *byte == b'\n' {
                line_offsets.push(pos as u64 + 1);
            }
        }

        Ok(PagedFileReader { path, current_size, mmap, line_offsets })
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
        // if self.current_size != metadata(Path::new(&self.path))?.len() {
        //    //TODO update offsets
        // }
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

    /// Reads the line number `line_number` from the file.
    pub fn read_line(&self, line_number: usize) -> Option<String> {
        if line_number + 1 >= self.line_offsets.len() {
            return None;
        }

        let line_start = self.line_offsets[line_number] as usize;
        let line_end = self.line_offsets[line_number + 1] as usize;
        let bytes = &self.mmap[line_start..line_end];

        Some(String::from_utf8_lossy(bytes).trim_end().to_owned())
    }
}