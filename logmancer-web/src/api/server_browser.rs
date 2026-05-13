use crate::api::commons::{
    ApiError, OpenServerFileResponse, ServerBrowserEntry, ServerBrowserListRequest,
    ServerBrowserListResponse, ServerBrowserOpenRequest, ServerBrowserStatusResponse,
};
use crate::api::config::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Clone)]
pub struct ServerFileRoot {
    pub canonical_path: PathBuf,
}

impl ServerFileRoot {
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var("LOGMANCER_SERVER_FILE_ROOT").ok()?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        let canonical_path = std::fs::canonicalize(trimmed).ok()?;
        if !canonical_path.is_dir() {
            return None;
        }

        Some(Self { canonical_path })
    }
}

pub async fn server_browser_status(State(app_state): State<AppState>) -> impl IntoResponse {
    let enabled = app_state.server_file_root.is_some();
    let message = if enabled {
        None
    } else {
        Some("Server browsing is not configured on this deployment.".to_string())
    };

    (
        StatusCode::OK,
        Json(ServerBrowserStatusResponse { enabled, message }),
    )
        .into_response()
}

pub async fn server_browser_list(
    State(app_state): State<AppState>,
    Json(payload): Json<ServerBrowserListRequest>,
) -> impl IntoResponse {
    let Some(root) = app_state.server_file_root.as_ref() else {
        return api_error(
            StatusCode::FORBIDDEN,
            "server_browser_disabled",
            "Server browser is unavailable.",
        );
    };

    let resolved = match resolve_root_bound_path(root, &payload.path) {
        Ok(path) => path,
        Err((status, code, message)) => return api_error(status, code, message),
    };

    if !resolved.is_dir() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "not_directory",
            "Requested path is not a directory.",
        );
    }

    match list_directory(root, &resolved) {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(_) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "open_failed",
            "Could not list directory.",
        ),
    }
}

pub async fn server_browser_open(
    State(app_state): State<AppState>,
    Json(payload): Json<ServerBrowserOpenRequest>,
) -> impl IntoResponse {
    let Some(root) = app_state.server_file_root.as_ref() else {
        return api_error(
            StatusCode::FORBIDDEN,
            "server_browser_disabled",
            "Server browser is unavailable.",
        );
    };

    let resolved = match resolve_root_bound_path(root, &payload.path) {
        Ok(path) => path,
        Err((status, code, message)) => return api_error(status, code, message),
    };

    if !resolved.is_file() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "not_file",
            "Requested path is not a file.",
        );
    }

    if !is_text_readable(&resolved) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "not_text_readable",
            "Requested file is not a readable text file.",
        );
    }

    let open_target = resolved.to_string_lossy().to_string();
    match app_state.registry.clone().open_file(&open_target) {
        Ok(file_id) => (
            StatusCode::CREATED,
            Json(OpenServerFileResponse { file_id }),
        )
            .into_response(),
        Err(_) => api_error(
            StatusCode::BAD_REQUEST,
            "open_failed",
            "Could not open file.",
        ),
    }
}

fn list_directory(root: &ServerFileRoot, dir_path: &Path) -> Result<ServerBrowserListResponse, ()> {
    let mut entries = Vec::new();

    for entry in std::fs::read_dir(dir_path).map_err(|_| ())? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let canonical = match std::fs::canonicalize(entry.path()) {
            Ok(path) => path,
            Err(_) => continue,
        };

        if !canonical.starts_with(&root.canonical_path) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let rel = match canonical.strip_prefix(&root.canonical_path) {
            Ok(rel) => rel,
            Err(_) => continue,
        };

        entries.push(ServerBrowserEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: rel.to_string_lossy().replace('\\', "/"),
            entry_type: if metadata.is_dir() {
                "directory"
            } else {
                "file"
            }
            .to_string(),
            size: if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            },
            modified: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs().to_string()),
        });
    }

    entries.sort_by(|a, b| {
        let type_order = match (a.entry_type.as_str(), b.entry_type.as_str()) {
            ("directory", "file") => std::cmp::Ordering::Less,
            ("file", "directory") => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        };
        if type_order != std::cmp::Ordering::Equal {
            return type_order;
        }
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });

    let current = dir_path
        .strip_prefix(&root.canonical_path)
        .map_err(|_| ())?
        .to_string_lossy()
        .replace('\\', "/");

    Ok(ServerBrowserListResponse {
        can_go_up: !current.is_empty(),
        current_path: current,
        entries,
    })
}

fn resolve_root_bound_path(
    root: &ServerFileRoot,
    token: &str,
) -> Result<PathBuf, (StatusCode, &'static str, &'static str)> {
    let trimmed = token.trim();
    let requested = if trimmed.is_empty() {
        PathBuf::new()
    } else {
        PathBuf::from(trimmed)
    };

    if requested.is_absolute() {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid_path",
            "Invalid path token.",
        ));
    }

    for component in requested.components() {
        match component {
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "invalid_path",
                    "Invalid path token.",
                ));
            }
            _ => {}
        }
    }

    let joined = root.canonical_path.join(&requested);
    let canonical = joined.canonicalize().map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            "not_found",
            "Requested path was not found.",
        )
    })?;

    if !canonical.starts_with(&root.canonical_path) {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid_path",
            "Invalid path token.",
        ));
    }

    Ok(canonical)
}

fn is_text_readable(path: &Path) -> bool {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let probe_len = bytes.len().min(8192);
    let probe = &bytes[..probe_len];
    if probe.contains(&0) {
        return false;
    }

    std::str::from_utf8(probe).is_ok()
}

fn api_error(status: StatusCode, code: &str, message: &str) -> axum::response::Response {
    (
        status,
        Json(ApiError {
            code: code.to_string(),
            message: message.to_string(),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn mk_root() -> (tempfile::TempDir, ServerFileRoot) {
        let dir = tempfile::tempdir().unwrap();
        let root = ServerFileRoot {
            canonical_path: std::fs::canonicalize(dir.path()).unwrap(),
        };
        (dir, root)
    }

    #[test]
    fn rejects_parent_traversal() {
        let (_dir, root) = mk_root();
        let result = resolve_root_bound_path(&root, "../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_absolute_path() {
        let (_dir, root) = mk_root();
        let result = resolve_root_bound_path(&root, "/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_symlink_escape() {
        let (_dir, root) = mk_root();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("outside.log"), "hello").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            outside.path().join("outside.log"),
            root.canonical_path.join("link.log"),
        )
        .unwrap();

        let result = resolve_root_bound_path(&root, "link.log");
        assert!(result.is_err());
    }

    #[test]
    fn list_sorts_directories_first_then_alpha() {
        let (_dir, root) = mk_root();
        std::fs::create_dir_all(root.canonical_path.join("zdir")).unwrap();
        std::fs::create_dir_all(root.canonical_path.join("adir")).unwrap();
        std::fs::write(root.canonical_path.join("z.log"), "z").unwrap();
        std::fs::write(root.canonical_path.join("a.log"), "a").unwrap();

        let response = list_directory(&root, &root.canonical_path).unwrap();
        let names: Vec<String> = response.entries.into_iter().map(|e| e.name).collect();
        assert_eq!(names, vec!["adir", "zdir", "a.log", "z.log"]);
    }

    #[test]
    fn list_populates_metadata_and_can_go_up() {
        let (_dir, root) = mk_root();
        std::fs::create_dir_all(root.canonical_path.join("child")).unwrap();
        let file_path = root.canonical_path.join("child").join("app.log");
        std::fs::write(&file_path, "abc").unwrap();

        let response = list_directory(&root, &root.canonical_path.join("child")).unwrap();
        assert!(response.can_go_up);
        assert_eq!(response.current_path, "child");
        assert_eq!(response.entries.len(), 1);
        assert_eq!(response.entries[0].entry_type, "file");
        assert_eq!(response.entries[0].size, Some(3));
        assert!(response.entries[0].modified.is_some());
    }

    #[test]
    fn text_readable_accepts_utf8_text() {
        let (_dir, root) = mk_root();
        let path = root.canonical_path.join("ok.log");
        std::fs::write(&path, "hola").unwrap();
        assert!(is_text_readable(&path));
    }

    #[test]
    fn text_readable_rejects_binary_with_nul() {
        let (_dir, root) = mk_root();
        let path = root.canonical_path.join("bad.bin");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&[0, 159, 146, 150]).unwrap();
        assert!(!is_text_readable(&path));
    }
}
