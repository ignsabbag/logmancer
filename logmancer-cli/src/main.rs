#[macro_use]
mod print_utils;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute, terminal,
    style::{Print}
};
use log::{debug, error, LevelFilter};
use logmancer_core::LogReader;
use std::env;
use std::fs::{OpenOptions};
use std::io::{stdout, Write};
use std::{process, time};

fn main() -> std::io::Result<()> {
    setup_logging().expect("Failed to initialize logging");

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <ruta del archivo>", args[0]);
        process::exit(1);
    }
    let filepath = &args[1];

    execute!(stdout(), terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    std::panic::set_hook(Box::new(|panic_info| {
        error!("{}", panic_info);
        terminal::disable_raw_mode().unwrap();
        process::exit(1);
    }));

    let mut reader = match LogReader::new(filepath.to_string()) {
        Ok(r) => r,
        Err(e) => {
            error!("Error opening file: {}", e);
            process::exit(1);
        }
    };

    let mut page_size: usize = 20;
    let mut page_first_line: usize = 0;
    let mut last_page_result = None;

    let mut last_dimensions = (0, 0);
    let mut follow_mode = false;
    let mut end_reached = false;

    loop {

        let (columns, rows) = terminal::size()?;
        if rows <= 2 || columns <= 8 {
            continue;
        }

        let new_page_size = rows.saturating_sub(2) as usize;
        let dimensions_changed = (columns, rows) != last_dimensions;
        if dimensions_changed {
            page_size = new_page_size;
            last_dimensions = (columns, rows);
        }

        let result = if end_reached {
            reader.tail(page_size, follow_mode)
        } else {
            reader.read_page(page_first_line, page_size)
        };
        let page_result = match result {
            Ok(page_result) => {
                page_first_line = page_result.start_line;
                page_result
            },
            Err(e) => {
                error!("Error reading file: {}", e);
                break;
            }
        };

        end_reached = page_first_line + page_size >= page_result.total_lines;
        let indexing_progress = page_result.indexing_progress * 100.0;

        if last_page_result.as_ref() != Some(&page_result) || dimensions_changed {
            let indexed = if indexing_progress < 100.0 {
                format!(" ({:.2}% indexed)", indexing_progress)
            } else {
                "".to_owned()
            };

            // Header
            print_row!(0, "File: {} | Follow Mode: {} | Total Lines: {}{}",
                &args[1], if follow_mode { "ON" } else { "OFF" }, page_result.total_lines, indexed);
            print_row!(1, "{}", "-".repeat(columns as usize));

            // Lines
            let last_line = page_result.start_line + page_size;
            let left_offset = last_line.to_string().len() + 1;
            for (i, line) in page_result.lines.iter().enumerate() {
                print_row!(i + 2, "{:<left_offset$}{} {}", page_first_line + i, "|",
                    trunc_str(line.trim_end(), columns as usize - left_offset - 2));
            }

            last_page_result = Some(page_result);
        }
        stdout().flush()?;

        let polling = (end_reached && follow_mode) || indexing_progress < 100.0;
        let event = if polling {
            if event::poll(time::Duration::from_millis(1000))? {
                Some(event::read()?)
            } else {
                None
            }
        } else {
            Some(event::read()?)
        };
        if let Some(evt) = event {
            if let Event::Key(key_event) = evt {
                match key_event.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('f') | KeyCode::Char('F') => follow_mode = !follow_mode,
                    KeyCode::Char('g') => {
                        end_reached = false;
                        page_first_line = 0;
                    }
                    KeyCode::Char('G') => {
                        end_reached = true;
                    }
                    KeyCode::Down => {
                        if !end_reached {
                            page_first_line += 1;
                        }
                    }
                    KeyCode::Up => {
                        end_reached = false;
                        page_first_line = page_first_line.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        if !end_reached {
                            page_first_line += page_size;
                        }
                    }
                    KeyCode::PageUp => {
                        end_reached = false;
                        page_first_line = page_first_line.saturating_sub(page_size);
                    }
                    _ => {}
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    execute!(stdout(), terminal::LeaveAlternateScreen)?;
    Ok(())
}

fn trunc_str(s: &str, max_len: usize) -> &str {
    if s.chars().count() > max_len {
        let mut end = 0;
        for (i, _) in s.char_indices().take(max_len) {
            end = i;
        }
        &s[..=end]
    } else {
        s
    }
}

fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("logmancer.log")?;

    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)
        .target(env_logger::Target::Pipe(Box::new(file)))
        .init();

    debug!("Log initialized");

    Ok(())
}