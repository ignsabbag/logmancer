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
