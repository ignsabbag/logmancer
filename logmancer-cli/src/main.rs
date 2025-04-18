#[macro_use]
mod print_utils;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute, terminal,
};
use log::{debug, error, LevelFilter};
use logmancer_core::LogReader;
use std::env;
use std::fs::{OpenOptions};
use std::io::stdout;
use std::{process, time};

fn main() -> std::io::Result<()> {
    setup_logging().expect("Failed to initialize logging");

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <ruta del archivo>", args[0]);
        process::exit(1);
    }
    let filepath = &args[1];

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

    let mut stdout = stdout();
    let mut page_size: usize;
    let mut current_offset: usize = 0;
    let mut follow_mode = false;
    let mut end_reached = false;

    loop {

        let (columns, rows) = terminal::size()?;
        if rows > 2 && columns > 7 {
            page_size = (rows.saturating_sub(2) - 2) as usize;
        } else {
            // In debug, sometimes columns and rows are 0
            continue;
        }

        let result = if end_reached {
            reader.tail(page_size, follow_mode)
        } else {
            reader.read_page(current_offset, page_size)
        };
        let page_result = match result {
            Ok(page_result) => {
                current_offset = page_result.start_line;
                page_result
            },
            Err(e) => {
                error!("Error reading file: {}", e);
                break;
            }
        };
        end_reached = current_offset + page_size >= page_result.total_lines;
        let index = page_result.indexing_progress * 100.0;

        // Header
        execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All))?;
        print_row!(0, "File: {} | Follow Mode: {} | Total Lines: {} ({:.2}% indexed)",
            &args[1], if follow_mode { "ON" } else { "OFF" }, page_result.total_lines, index);
        print_row!(1, "{}", "-".repeat(columns as usize));

        // Lines
        for (i, line) in page_result.lines.iter().enumerate() {
            print_row!(i + 2, "{:<5}{} {}", current_offset + i, "|",
                trunc_str(line.trim_end(), (columns - 7) as usize));
        }

        let polling = (end_reached && follow_mode) || index < 100.0;
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
                        current_offset = 0;
                    }
                    KeyCode::Char('G') => {
                        end_reached = true;
                    }
                    KeyCode::Down => {
                        if !end_reached {
                            current_offset += 1;
                        }
                    }
                    KeyCode::Up => {
                        end_reached = false;
                        current_offset = current_offset.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        if !end_reached {
                            current_offset += page_size;
                        }
                    }
                    KeyCode::PageUp => {
                        end_reached = false;
                        current_offset = current_offset.saturating_sub(page_size);
                    }
                    _ => {}
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
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