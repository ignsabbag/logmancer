#[macro_use]
mod print_utils;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::Print,
    terminal,
};
use log::{LevelFilter, debug, error};
use logmancer_core::LogReader;
use std::env;
use std::fs::OpenOptions;
use std::io::{Write, stdout};
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
        error!("{panic_info}");
        terminal::disable_raw_mode().unwrap();
        process::exit(1);
    }));

    let mut reader = match LogReader::new(filepath.to_string()) {
        Ok(r) => r,
        Err(e) => {
            error!("Error opening file: {e}");
            process::exit(1);
        }
    };

    let mut page_size: usize = 20;
    let mut page_first_line: usize = 0;
    let mut last_page_result = None;

    let mut last_dimensions = (0, 0);
    let mut follow_mode = false;
    let mut end_reached = false;
    let mut search_input_mode = false;
    let mut search_prompt = String::new();
    let mut ui_dirty = true;

    loop {
        let (columns, rows) = terminal::size()?;
        if rows <= 3 || columns <= 8 {
            continue;
        }

        let new_page_size = rows.saturating_sub(3) as usize;
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
            }
            Err(e) => {
                error!("Error reading file: {e}");
                break;
            }
        };

        end_reached = page_first_line + page_size >= page_result.total_lines;
        let indexing_progress = page_result.indexing_progress * 100.0;

        if last_page_result.as_ref() != Some(&page_result) || dimensions_changed || ui_dirty {
            let indexed = if indexing_progress < 100.0 {
                format!(" ({indexing_progress:.2}% indexed)")
            } else {
                "".to_owned()
            };

            // Header
            print_row!(
                0,
                "File: {} | Follow Mode: {} | Total Lines: {}{} | Search: {}",
                &args[1],
                if follow_mode { "ON" } else { "OFF" },
                page_result.total_lines,
                indexed,
                page_result.search.as_ref().map_or_else(
                    || "OFF".to_string(),
                    |search| {
                        let phase = if search.is_indexing {
                            " (searching...)"
                        } else {
                            ""
                        };
                        format!("{}{}", search.query, phase)
                    },
                )
            );
            print_row!(1, "{}", "-".repeat(columns as usize));

            // Lines
            let last_line = page_result
                .lines
                .last()
                .map(|line| line.number)
                .unwrap_or(page_result.start_line + page_size);
            let left_offset = last_line.to_string().len() + 1;
            for (i, line) in page_result.lines.iter().enumerate() {
                print_row!(
                    i + 2,
                    "{:<left_offset$}{} {}{}",
                    line.number,
                    "|",
                    trunc_str(line.text.trim_end(), columns as usize - left_offset - 2),
                    page_result
                        .search
                        .as_ref()
                        .and_then(|search| search.current.as_ref())
                        .filter(|current| current.line_index + 1 == line.number)
                        .map(|_| " <")
                        .unwrap_or("")
                );
            }

            print_row!(
                rows as usize - 1,
                "{}",
                if search_input_mode {
                    format!("/{}", search_prompt)
                } else {
                    "".to_string()
                }
            );

            last_page_result = Some(page_result);
            ui_dirty = false;
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
        if let Some(Event::Key(key_event)) = event {
            if search_input_mode {
                match key_event.code {
                    KeyCode::Enter => {
                        search_input_mode = false;
                        let search_query = search_prompt.trim().to_string();
                        ui_dirty = true;

                        if search_query.is_empty() {
                            reader.clear_search();
                            last_page_result = None;
                        } else if let Ok(page) =
                            reader.apply_search(search_query.clone(), page_size)
                        {
                            page_first_line = page.start_line;
                            end_reached = page_first_line + page_size >= page.total_lines;
                            last_page_result = None;
                        }
                    }
                    KeyCode::Esc => {
                        search_input_mode = false;
                        search_prompt.clear();
                        ui_dirty = true;
                    }
                    KeyCode::Backspace => {
                        search_prompt.pop();
                        ui_dirty = true;
                    }
                    KeyCode::Char(c) => {
                        search_prompt.push(c);
                        ui_dirty = true;
                    }
                    _ => {}
                }
                continue;
            }
            match key_event.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('/') => {
                    search_input_mode = true;
                    search_prompt.clear();
                    ui_dirty = true;
                }
                KeyCode::Char('n') => {
                    if let Ok(page) = reader.search_next(page_size) {
                        page_first_line = page.start_line;
                        end_reached = page_first_line + page_size >= page.total_lines;
                        last_page_result = None;
                    }
                }
                KeyCode::Char('N') => {
                    if let Ok(page) = reader.search_previous(page_size) {
                        page_first_line = page.start_line;
                        end_reached = page_first_line + page_size >= page.total_lines;
                        last_page_result = None;
                    }
                }
                KeyCode::Char('f') | KeyCode::Char('F') => follow_mode = !follow_mode,
                KeyCode::Char('g') => {
                    end_reached = false;
                    page_first_line = 0;
                }
                KeyCode::Char('G') => {
                    end_reached = true;
                }
                KeyCode::Down if !end_reached => {
                    page_first_line += 1;
                }
                KeyCode::Up => {
                    end_reached = false;
                    page_first_line = page_first_line.saturating_sub(1);
                }
                KeyCode::PageDown if !end_reached => {
                    page_first_line += page_size;
                }
                KeyCode::PageUp => {
                    end_reached = false;
                    page_first_line = page_first_line.saturating_sub(page_size);
                }
                _ => {}
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
        .append(true)
        .open("logmancer.log")?;

    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)
        .target(env_logger::Target::Pipe(Box::new(file)))
        .init();

    debug!("Log initialized");

    Ok(())
}
