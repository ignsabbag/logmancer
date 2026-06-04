#[macro_export]
macro_rules! print_row {
    ($row:expr, $($arg:tt)*) => {
        execute!(
            stdout(),
            cursor::MoveTo(0, $row as u16),
            terminal::Clear(terminal::ClearType::UntilNewLine),
            Print(format!($($arg)*)),
            cursor::MoveTo(0, $row as u16)
        )?;
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HighlightKind {
    Plain,
    Match,
    CurrentMatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HighlightSegment<'a> {
    pub text: &'a str,
    pub kind: HighlightKind,
}

pub fn split_highlighted_segments<'a>(
    line: &'a str,
    spans: &[(usize, usize, bool)],
) -> Vec<HighlightSegment<'a>> {
    if spans.is_empty() {
        return vec![HighlightSegment {
            text: line,
            kind: HighlightKind::Plain,
        }];
    }

    let mut segments = Vec::new();
    let mut cursor = 0usize;

    for (start, end, is_current) in spans {
        let start = (*start).min(line.len());
        let end = (*end).min(line.len());

        if cursor < start {
            segments.push(HighlightSegment {
                text: &line[cursor..start],
                kind: HighlightKind::Plain,
            });
        }

        if start < end {
            segments.push(HighlightSegment {
                text: &line[start..end],
                kind: if *is_current {
                    HighlightKind::CurrentMatch
                } else {
                    HighlightKind::Match
                },
            });
        }

        cursor = end;
    }

    if cursor < line.len() {
        segments.push(HighlightSegment {
            text: &line[cursor..],
            kind: HighlightKind::Plain,
        });
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::{HighlightKind, HighlightSegment, split_highlighted_segments};

    #[test]
    fn split_highlighted_segments_marks_current_and_secondary_matches() {
        let segments = split_highlighted_segments("foo bar baz", &[(0, 3, false), (8, 11, true)]);

        assert_eq!(
            segments,
            vec![
                HighlightSegment {
                    text: "foo",
                    kind: HighlightKind::Match,
                },
                HighlightSegment {
                    text: " bar ",
                    kind: HighlightKind::Plain,
                },
                HighlightSegment {
                    text: "baz",
                    kind: HighlightKind::CurrentMatch,
                },
            ]
        );
    }

    #[test]
    fn split_highlighted_segments_clamps_spans_to_visible_prefix() {
        let segments = split_highlighted_segments("foobar", &[(3, 12, false)]);

        assert_eq!(
            segments,
            vec![
                HighlightSegment {
                    text: "foo",
                    kind: HighlightKind::Plain,
                },
                HighlightSegment {
                    text: "bar",
                    kind: HighlightKind::Match,
                },
            ]
        );
    }
}
