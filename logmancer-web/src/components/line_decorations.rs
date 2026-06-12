use logmancer_core::PageSearchResult;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DecorationKind {
    SearchMatch,
    SearchCurrent,
}

impl DecorationKind {
    fn precedence(self) -> u8 {
        match self {
            Self::SearchMatch => 10,
            Self::SearchCurrent => 40,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LineDecoration {
    pub start: usize,
    pub end: usize,
    pub kind: DecorationKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RenderSegment<'a> {
    pub text: &'a str,
    pub kind: Option<DecorationKind>,
}

pub(crate) fn split_line_segments<'a>(
    line: &'a str,
    decorations: &[LineDecoration],
) -> Vec<RenderSegment<'a>> {
    let valid_decorations = decorations
        .iter()
        .copied()
        .filter(|decoration| {
            decoration.start < decoration.end
                && decoration.end <= line.len()
                && line.is_char_boundary(decoration.start)
                && line.is_char_boundary(decoration.end)
        })
        .collect::<Vec<_>>();

    if valid_decorations.is_empty() {
        return vec![RenderSegment {
            text: line,
            kind: None,
        }];
    }

    let mut split_points = vec![0, line.len()];
    for decoration in &valid_decorations {
        split_points.push(decoration.start);
        split_points.push(decoration.end);
    }
    split_points.sort_unstable();
    split_points.dedup();

    let mut segments = Vec::new();
    for window in split_points.windows(2) {
        let start = window[0];
        let end = window[1];
        if start == end {
            continue;
        }

        let selected = valid_decorations
            .iter()
            .enumerate()
            .filter(|(_, decoration)| decoration.start <= start && end <= decoration.end)
            .fold(
                None,
                |selected: Option<(usize, DecorationKind)>, (index, decoration)| match selected {
                    Some((_, kind)) if kind.precedence() >= decoration.kind.precedence() => {
                        selected
                    }
                    _ => Some((index, decoration.kind)),
                },
            );

        segments.push((
            selected.map(|(_, kind)| kind),
            selected.map(|(index, _)| index),
            start,
            end,
        ));
    }

    segments
        .into_iter()
        .fold(Vec::new(), |mut merged, (kind, source, start, end)| {
            if let Some((last_kind, last_source, _, last_end)) = merged.last_mut() {
                if *last_kind == kind && *last_source == source {
                    *last_end = end;
                    return merged;
                }
            }
            merged.push((kind, source, start, end));
            merged
        })
        .into_iter()
        .map(|(kind, _, start, end)| RenderSegment {
            text: &line[start..end],
            kind,
        })
        .collect()
}

pub(crate) fn search_decorations_by_line(
    search: &PageSearchResult,
) -> HashMap<usize, Vec<LineDecoration>> {
    let mut decorations_by_line = HashMap::new();

    for search_match in &search.page_matches {
        decorations_by_line
            .entry(search_match.line_index + 1)
            .or_insert_with(Vec::new)
            .push(LineDecoration {
                start: search_match.start,
                end: search_match.end,
                kind: if search.current.as_ref() == Some(search_match) {
                    DecorationKind::SearchCurrent
                } else {
                    DecorationKind::SearchMatch
                },
            });
    }

    decorations_by_line
}

#[cfg(test)]
mod tests {
    use super::{
        search_decorations_by_line, split_line_segments, DecorationKind, LineDecoration,
        RenderSegment,
    };
    use logmancer_core::{PageSearchResult, SearchMatch};

    fn segment<'a>(text: &'a str, kind: Option<DecorationKind>) -> RenderSegment<'a> {
        RenderSegment { text, kind }
    }

    fn decoration(start: usize, end: usize, kind: DecorationKind) -> LineDecoration {
        LineDecoration { start, end, kind }
    }

    fn search_match(line_index: usize, start: usize, end: usize, ordinal: usize) -> SearchMatch {
        SearchMatch {
            line_index,
            start,
            end,
            ordinal,
        }
    }

    fn page_search_result(
        current: Option<SearchMatch>,
        page_matches: Vec<SearchMatch>,
    ) -> PageSearchResult {
        PageSearchResult {
            query: "foo".to_string(),
            total_matches: page_matches.len(),
            total_matches_final: true,
            is_indexing: false,
            first: page_matches.first().cloned(),
            current,
            page_matches,
        }
    }

    #[test]
    fn split_line_segments_handles_adjacent_overlap_and_stable_order() {
        assert_eq!(
            split_line_segments(
                "foobar",
                &[
                    decoration(0, 3, DecorationKind::SearchMatch),
                    decoration(3, 6, DecorationKind::SearchCurrent),
                ],
            ),
            vec![
                segment("foo", Some(DecorationKind::SearchMatch)),
                segment("bar", Some(DecorationKind::SearchCurrent)),
            ]
        );

        assert_eq!(
            split_line_segments(
                "abcdef",
                &[
                    decoration(0, 6, DecorationKind::SearchMatch),
                    decoration(2, 4, DecorationKind::SearchCurrent),
                ],
            ),
            vec![
                segment("ab", Some(DecorationKind::SearchMatch)),
                segment("cd", Some(DecorationKind::SearchCurrent)),
                segment("ef", Some(DecorationKind::SearchMatch)),
            ]
        );

        assert_eq!(
            split_line_segments(
                "abcdef",
                &[
                    decoration(1, 4, DecorationKind::SearchMatch),
                    decoration(2, 5, DecorationKind::SearchMatch),
                ],
            ),
            vec![
                segment("a", None),
                segment("bcd", Some(DecorationKind::SearchMatch)),
                segment("e", Some(DecorationKind::SearchMatch)),
                segment("f", None),
            ]
        );
    }

    #[test]
    fn split_line_segments_ignores_invalid_spans_without_panicking() {
        assert_eq!(
            split_line_segments(
                "éclair",
                &[
                    decoration(0, 0, DecorationKind::SearchMatch),
                    decoration(5, 3, DecorationKind::SearchMatch),
                    decoration(0, 20, DecorationKind::SearchMatch),
                    decoration(1, 3, DecorationKind::SearchCurrent),
                    decoration(2, 4, DecorationKind::SearchMatch),
                ],
            ),
            vec![
                segment("é", None),
                segment("cl", Some(DecorationKind::SearchMatch)),
                segment("air", None),
            ]
        );

        assert_eq!(
            split_line_segments("plain", &[]),
            vec![segment("plain", None)]
        );
    }

    #[test]
    fn split_line_segments_matches_existing_multiple_occurrence_expectation() {
        let segments = split_line_segments(
            "foo bar foo",
            &[
                decoration(0, 3, DecorationKind::SearchMatch),
                decoration(8, 11, DecorationKind::SearchCurrent),
            ],
        );

        assert_eq!(
            segments,
            vec![
                segment("foo", Some(DecorationKind::SearchMatch)),
                segment(" bar ", None),
                segment("foo", Some(DecorationKind::SearchCurrent)),
            ]
        );
        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.text)
                .collect::<String>(),
            "foo bar foo"
        );
    }

    #[test]
    fn search_decorations_by_line_converts_current_and_non_current_matches() {
        let current = search_match(2, 4, 9, 1);
        let search = page_search_result(
            Some(current.clone()),
            vec![search_match(2, 0, 3, 0), current, search_match(3, 1, 2, 2)],
        );
        let grouped = search_decorations_by_line(&search);

        assert_eq!(
            grouped.get(&3),
            Some(&vec![
                decoration(0, 3, DecorationKind::SearchMatch),
                decoration(4, 9, DecorationKind::SearchCurrent),
            ])
        );
    }

    #[test]
    fn search_decorations_by_line_groups_typed_decorations_once() {
        let current = search_match(2, 4, 9, 1);
        let search = page_search_result(
            Some(current.clone()),
            vec![search_match(2, 0, 3, 0), current, search_match(3, 1, 2, 2)],
        );

        let grouped = search_decorations_by_line(&search);

        assert_eq!(
            grouped.get(&3),
            Some(&vec![
                decoration(0, 3, DecorationKind::SearchMatch),
                decoration(4, 9, DecorationKind::SearchCurrent),
            ])
        );
        assert_eq!(
            grouped.get(&4),
            Some(&vec![decoration(1, 2, DecorationKind::SearchMatch)])
        );
    }
}
