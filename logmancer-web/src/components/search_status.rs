use logmancer_core::{PageResult, SearchDisplayStatus};

pub fn format_page_search_status(page: &PageResult) -> String {
    page.search
        .as_ref()
        .map(|search| format_search_status(&search.display_status()))
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::format_search_status;
    use logmancer_core::SearchDisplayStatus;

    #[test]
    fn format_search_status_shows_match_position() {
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
    fn format_search_status_shows_no_matches() {
        assert_eq!(
            format_search_status(&SearchDisplayStatus {
                query: "error".to_string(),
                current_match_index: None,
                total_matches: 0,
                total_matches_final: true,
                is_indexing: false,
            }),
            "error no matches"
        );
    }
}
