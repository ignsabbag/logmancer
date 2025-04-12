#[macro_use]
mod print_utils;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute, terminal,
};
use std::env;
use std::io::stdout;
use std::{process, time};

use logmancer_core::{PagedFileReader, PageResult};

fn main() -> std::io::Result<()> {

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <ruta del archivo>", args[0]);
        process::exit(1);
    }
    let filepath = &args[1];

    let mut reader = match PagedFileReader::from(filepath.to_string()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error al abrir el archivo: {}", e);
            process::exit(1);
        }
    };

    terminal::enable_raw_mode()?;
    let mut stdout = stdout();

    let mut page_size: usize;
    let mut current_offset: usize = 0;
    let mut follow_mode = false; // false: navegación normal; true: modo tail (seguimiento)

    loop {

        let (columns, rows) = terminal::size()?;
        page_size = (rows.saturating_sub(2) - 2) as usize;

        execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(terminal::ClearType::All))?;

        let page_result: PageResult = if follow_mode {
            match reader.tail(page_size, false) {
                Ok(pr) => {
                    current_offset = pr.start_line;
                    pr
                },
                Err(e) => {
                    eprintln!("Error al leer tail: {}", e);
                    break;
                }
            }
        } else {
            match reader.read_page(current_offset, page_size) {
                Ok(pr) => pr,
                Err(e) => {
                    eprintln!("Error al leer la página: {}", e);
                    break;
                }
            }
        };

        // Header
        print_row!(0, "File: {} | Total Lines: {} | Follow Mode: {}",
            &args[1], page_result.total_lines, if follow_mode { "ON" } else { "OFF" });
        print_row!(1, "{}", "-".repeat(columns as usize));

        // Lines
        for (i, line) in page_result.lines.iter().enumerate() {
            print_row!(i + 2, "{:<5}{} {}", current_offset + i, "|",
                trunc_str(line.trim_end(), (columns - 7) as usize));
        }

        // let event = if follow_mode {
        //     if event::poll(time::Duration::from_millis(500))? {
        //         Some(event::read()?)
        //     } else {
        //         None
        //     }
        // } else {
        //     Some(event::read()?)
        // };
        let event = Some(event::read()?);
            if let Some(evt) = event {
            if let Event::Key(key_event) = evt {
                match key_event.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('f') | KeyCode::Char('F') => follow_mode = !follow_mode,
                    KeyCode::Down => {
                        let end_reached = current_offset + page_size >= page_result.total_lines;
                        if !follow_mode && !end_reached {
                            current_offset += 1;
                        }
                    }
                    KeyCode::Up => {
                        if follow_mode {
                            follow_mode = false;
                        } else if current_offset > 0 {
                            current_offset -= 1;
                        }
                    }
                    KeyCode::PageDown => {
                        let end_reached = current_offset + 2 * page_size >= page_result.total_lines;
                        if !follow_mode && !end_reached {
                            current_offset += page_size;
                        } else if end_reached {
                            current_offset = page_result.total_lines - page_size;
                        }
                    }
                    KeyCode::PageUp => {
                        if !follow_mode {
                            current_offset = current_offset.saturating_sub(page_size);
                        }
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