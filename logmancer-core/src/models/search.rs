use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SearchMatch {
    pub line_index: usize,
    pub start: usize,
    pub end: usize,
    pub ordinal: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PageSearchResult {
    pub query: String,
    pub total_matches: usize,
    pub total_matches_final: bool,
    pub is_indexing: bool,
    pub first: Option<SearchMatch>,
    pub current: Option<SearchMatch>,
    pub page_matches: Vec<SearchMatch>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SearchDisplayStatus {
    pub query: String,
    pub current_match_index: Option<usize>,
    pub total_matches: usize,
    pub total_matches_final: bool,
    pub is_indexing: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SearchState {
    pub session: Option<SearchSession>,
}

#[derive(Clone, Debug)]
pub struct SearchSession {
    pub generation: u64,
    pub query: String,
    pub origin_line: usize,
    pub phase: SearchPhase,
    pub total_matches_final: bool,
    pub matches: Vec<SearchMatch>,
    pub first_match: Option<SearchMatch>,
    pub current_ordinal: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum SearchPhase {
    #[default]
    Ready,
    Indexing,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SearchStatus {
    pub query: Option<String>,
    pub generation: u64,
    pub origin_line: Option<usize>,
    pub phase: SearchPhase,
    pub is_ready: bool,
    pub total_matches: usize,
    pub total_matches_final: bool,
    pub first: Option<SearchMatch>,
    pub current: Option<SearchMatch>,
}

impl PageSearchResult {
    pub fn display_status(&self) -> SearchDisplayStatus {
        SearchDisplayStatus {
            query: self.query.clone(),
            current_match_index: self.current.as_ref().map(|current| current.ordinal + 1),
            total_matches: self.total_matches,
            total_matches_final: self.total_matches_final,
            is_indexing: self.is_indexing,
        }
    }
}

impl SearchStatus {
    pub fn display_status(&self) -> Option<SearchDisplayStatus> {
        self.query.as_ref().map(|query| SearchDisplayStatus {
            query: query.clone(),
            current_match_index: self.current.as_ref().map(|current| current.ordinal + 1),
            total_matches: self.total_matches,
            total_matches_final: self.total_matches_final,
            is_indexing: !self.is_ready,
        })
    }
}

impl SearchState {
    pub fn clear(&mut self) {
        self.session = None;
    }

    pub fn status(&self) -> SearchStatus {
        match &self.session {
            Some(session) => SearchStatus {
                query: Some(session.query.clone()),
                generation: session.generation,
                origin_line: Some(session.origin_line),
                phase: session.phase.clone(),
                is_ready: matches!(session.phase, SearchPhase::Ready),
                total_matches: session.matches.len(),
                total_matches_final: session.total_matches_final,
                first: session.first_match.clone(),
                current: session.current_match().cloned(),
            },
            None => SearchStatus {
                query: None,
                generation: 0,
                origin_line: None,
                phase: SearchPhase::Ready,
                is_ready: true,
                total_matches: 0,
                total_matches_final: true,
                first: None,
                current: None,
            },
        }
    }
}

impl SearchSession {
    pub fn indexing(generation: u64, query: String, origin_line: usize) -> Self {
        Self {
            generation,
            query,
            origin_line,
            phase: SearchPhase::Indexing,
            total_matches_final: false,
            matches: Vec::new(),
            first_match: None,
            current_ordinal: None,
        }
    }

    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.current_ordinal.and_then(|idx| self.matches.get(idx))
    }

    pub fn next(&mut self) {
        if self.matches.is_empty() {
            self.current_ordinal = None;
            return;
        }
        let current = self.current_ordinal.unwrap_or(0);
        self.current_ordinal = Some((current + 1) % self.matches.len());
    }

    pub fn previous(&mut self) {
        if self.matches.is_empty() {
            self.current_ordinal = None;
            return;
        }
        let current = self.current_ordinal.unwrap_or(0);
        self.current_ordinal = Some((current + self.matches.len() - 1) % self.matches.len());
    }
}

#[cfg(test)]
mod tests {
    use super::{PageSearchResult, SearchDisplayStatus, SearchMatch, SearchPhase, SearchStatus};

    #[test]
    fn page_search_display_status_uses_one_based_current_match_index() {
        let search = PageSearchResult {
            query: "error".to_string(),
            total_matches: 27,
            total_matches_final: false,
            is_indexing: true,
            first: None,
            current: Some(SearchMatch {
                line_index: 12,
                start: 0,
                end: 5,
                ordinal: 2,
            }),
            page_matches: Vec::new(),
        };

        assert_eq!(
            search.display_status(),
            SearchDisplayStatus {
                query: "error".to_string(),
                current_match_index: Some(3),
                total_matches: 27,
                total_matches_final: false,
                is_indexing: true,
            }
        );
    }

    #[test]
    fn search_status_display_status_is_absent_without_query() {
        let status = SearchStatus {
            query: None,
            generation: 0,
            origin_line: None,
            phase: SearchPhase::Ready,
            is_ready: true,
            total_matches: 0,
            total_matches_final: true,
            first: None,
            current: None,
        };

        assert_eq!(status.display_status(), None);
    }

    #[test]
    fn search_status_display_status_maps_readiness_to_indexing() {
        let status = SearchStatus {
            query: Some("error".to_string()),
            generation: 1,
            origin_line: Some(0),
            phase: SearchPhase::Indexing,
            is_ready: false,
            total_matches: 0,
            total_matches_final: false,
            first: None,
            current: None,
        };

        assert_eq!(
            status.display_status(),
            Some(SearchDisplayStatus {
                query: "error".to_string(),
                current_match_index: None,
                total_matches: 0,
                total_matches_final: false,
                is_indexing: true,
            })
        );
    }
}
