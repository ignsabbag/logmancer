#[macro_use]
mod print_utils;

use crate::print_utils::{HighlightKind, split_highlighted_segments};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{Attribute, Color, Print, PrintStyledContent, Stylize},
    terminal,
};
use log::{LevelFilter, debug, error};
use logmancer_core::{LogReader, PageSearchResult, SearchDisplayStatus};
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
                page_result
                    .search
                    .as_ref()
                    .map(|search| format_search_status(&search.display_status()))
                    .unwrap_or_else(|| "OFF".to_string())
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
                render_line_row(
                    i + 2,
                    line.number,
                    line.text.trim_end(),
                    left_offset,
                    columns as usize,
                    page_result.search.as_ref(),
                )?;
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

fn render_line_row(
    row: usize,
    line_number: usize,
    line_text: &str,
    left_offset: usize,
    columns: usize,
    search: Option<&PageSearchResult>,
) -> std::io::Result<()> {
    let is_current_line = search
        .and_then(|state| state.current.as_ref())
        .is_some_and(|current| current.line_index + 1 == line_number);
    let current_marker = if is_current_line { " <" } else { "" };
    let content_width = columns.saturating_sub(left_offset + 2 + current_marker.len());
    let visible_text = trunc_str(line_text, content_width);
    let spans = search
        .map(|state| collect_line_spans(state, line_number, visible_text.len()))
        .unwrap_or_default();
    let segments = split_highlighted_segments(visible_text, &spans);

    execute!(
        stdout(),
        cursor::MoveTo(0, row as u16),
        terminal::Clear(terminal::ClearType::UntilNewLine),
        Print(format!("{line_number:<left_offset$}| "))
    )?;

    for segment in segments {
        match segment.kind {
            HighlightKind::Plain => execute!(stdout(), Print(segment.text))?,
            HighlightKind::Match => execute!(
                stdout(),
                PrintStyledContent(segment.text.black().on(Color::Yellow))
            )?,
            HighlightKind::CurrentMatch => execute!(
                stdout(),
                PrintStyledContent(
                    segment
                        .text
                        .black()
                        .on(Color::DarkYellow)
                        .attribute(Attribute::Bold)
                        .attribute(Attribute::Underlined)
                )
            )?,
        }
    }

    if is_current_line {
        execute!(
            stdout(),
            PrintStyledContent(current_marker.with(Color::Cyan).attribute(Attribute::Bold))
        )?;
    }

    execute!(stdout(), cursor::MoveTo(0, row as u16))?;
    Ok(())
}

fn collect_line_spans(
    search: &PageSearchResult,
    line_number: usize,
    visible_len: usize,
) -> Vec<(usize, usize, bool)> {
    search
        .page_matches
        .iter()
        .filter(|search_match| search_match.line_index + 1 == line_number)
        .map(|search_match| {
            (
                search_match.start.min(visible_len),
                search_match.end.min(visible_len),
                search.current.as_ref() == Some(search_match),
            )
        })
        .collect()
}

fn format_search_status(status: &SearchDisplayStatus) -> String {
    let mut text = if status.total_matches == 0 {
        if status.is_indexing {
            format!("{} no matches yet", status.query)
        } else {
            format!("{} no matches", status.query)
        }
    } else if let Some(current_match_index) = status.current_match_index {
        format!(
            "{} {}/{}",
            status.query, current_match_index, status.total_matches
        )
    } else {
        format!("{} {} matches", status.query, status.total_matches)
    };

    if status.is_indexing {
        text.push_str(" searching...");
    }

    text
}

fn trunc_str(s: &str, max_len: usize) -> &str {
    if max_len == 0 {
        return "";
    }

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

#[cfg(test)]
mod tests {
    use super::{collect_line_spans, format_search_status, trunc_str};
    use logmancer_core::{PageSearchResult, SearchDisplayStatus, SearchMatch};

    #[test]
    fn trunc_str_returns_empty_when_width_is_zero() {
        assert_eq!(trunc_str("foo", 0), "");
    }

    #[test]
    fn collect_line_spans_marks_current_match_and_clamps_to_visible_text() {
        let current = SearchMatch {
            line_index: 4,
            start: 6,
            end: 20,
            ordinal: 1,
        };
        let search = PageSearchResult {
            query: "foo".to_string(),
            total_matches: 2,
            total_matches_final: true,
            is_indexing: false,
            first: None,
            current: Some(current.clone()),
            page_matches: vec![
                SearchMatch {
                    line_index: 4,
                    start: 0,
                    end: 3,
                    ordinal: 0,
                },
                current,
            ],
        };

        assert_eq!(
            collect_line_spans(&search, 5, 8),
            vec![(0, 3, false), (6, 8, true)]
        );
    }

    #[test]
    fn format_search_status_shows_current_and_total_matches() {
        assert_eq!(
            format_search_status(&SearchDisplayStatus {
                query: "error".to_string(),
                current_match_index: Some(3),
                total_matches: 27,
                total_matches_final: true,
                is_indexing: false,
            }),
            "error 3/27"
        );
    }

    #[test]
    fn format_search_status_shows_no_matches_while_indexing() {
        assert_eq!(
            format_search_status(&SearchDisplayStatus {
                query: "error".to_string(),
                current_match_index: None,
                total_matches: 0,
                total_matches_final: false,
                is_indexing: true,
            }),
            "error no matches yet searching..."
        );
    }
}
