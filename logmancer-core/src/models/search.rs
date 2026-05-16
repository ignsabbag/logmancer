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
    pub current: Option<SearchMatch>,
    pub page_matches: Vec<SearchMatch>,
}

#[derive(Clone, Debug, Default)]
pub struct SearchState {
    pub session: Option<SearchSession>,
}

#[derive(Clone, Debug)]
pub struct SearchSession {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub current_ordinal: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SearchStatus {
    pub query: Option<String>,
    pub total_matches: usize,
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
                total_matches: session.matches.len(),
                current: session.current_match().cloned(),
            },
            None => SearchStatus {
                query: None,
                total_matches: 0,
                current: None,
            },
        }
    }
}

impl SearchSession {
    pub fn new(query: String, matches: Vec<SearchMatch>) -> Self {
        let current_ordinal = if matches.is_empty() { None } else { Some(0) };
        Self {
            query,
            matches,
            current_ordinal,
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
