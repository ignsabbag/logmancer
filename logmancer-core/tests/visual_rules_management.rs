#![cfg(feature = "native-persistence")]

use logmancer_core::{
    AtomicFileReplacer, LineStyleIntent, LogRegistry, ManagedVisualRule, NativeVisualRulesStore,
    SaveOutcome, StoreCommit, ValidationSeverity, VisualColor, VisualMatcher, VisualRulesEnvelope,
    VisualRulesError, VisualRulesManager, VisualRulesStore,
};
use std::fs::File;
use std::io::{BufRead, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

fn style(foreground: Option<&str>, background: Option<&str>) -> LineStyleIntent {
    LineStyleIntent {
        foreground: foreground.map(|value| VisualColor(value.to_string())),
        background: background.map(|value| VisualColor(value.to_string())),
    }
}

fn red_style() -> Option<LineStyleIntent> {
    Some(style(Some("red"), Some("default")))
}

fn rule(pattern: &str) -> ManagedVisualRule {
    ManagedVisualRule {
        name: Some("important".to_string()),
        enabled: true,
        matcher: VisualMatcher::Text(pattern.to_string()),
        case_sensitive: false,
        style: style(Some("red"), Some("default")),
    }
}

fn oversized_valid_envelope() -> VisualRulesEnvelope {
    let mut oversized_rule = rule(&"\u{1}".repeat(512));
    oversized_rule.name = Some("\u{1}".repeat(80));
    oversized_rule.enabled = false;
    let envelope = VisualRulesEnvelope::new(vec![oversized_rule; 100]);

    envelope
        .validate_for_save()
        .expect("character-count limits remain valid");
    assert!(
        serde_json::to_vec(&envelope)
            .expect("serialize oversized envelope")
            .len()
            > VisualRulesEnvelope::MAX_PERSISTED_SIZE
    );
    envelope
}

fn envelope_with_serialized_size(target: usize) -> VisualRulesEnvelope {
    let empty_rule = ManagedVisualRule {
        name: Some(String::new()),
        enabled: false,
        matcher: VisualMatcher::Text(String::new()),
        case_sensitive: false,
        style: style(Some("red"), Some("default")),
    };
    let mut envelope = VisualRulesEnvelope::new(vec![empty_rule; 100]);
    let baseline = serde_json::to_vec_pretty(&envelope)
        .expect("serialize baseline envelope")
        .len();
    let extra = target
        .checked_sub(baseline)
        .expect("target exceeds baseline");
    let mut escaped = extra / 6;
    let mut ascii = extra % 6;

    let mut fill = |capacity| {
        let escaped_count = escaped.min(capacity);
        escaped -= escaped_count;
        let ascii_count = ascii.min(capacity - escaped_count);
        ascii -= ascii_count;
        "\u{1}".repeat(escaped_count) + &"x".repeat(ascii_count)
    };
    for rule in &mut envelope.rules {
        rule.matcher = VisualMatcher::Text(fill(512));
        rule.name = Some(fill(80));
    }
    assert_eq!((escaped, ascii), (0, 0));
    assert!(envelope.validate_for_save().is_ok());
    let size = serde_json::to_vec_pretty(&envelope)
        .expect("serialize boundary envelope")
        .len();
    assert_eq!(size, target);
    envelope
}

#[test]
fn envelope_validation_enforces_limits_palette_and_enabled_conversion() {
    let envelope = VisualRulesEnvelope::new(vec![rule("ERROR")]);

    let validated = envelope.validate_for_save().expect("valid envelope");
    assert_eq!(validated.evaluator_rules.len(), 1);
    assert_eq!(
        validated.evaluator_rules[0].matcher,
        VisualMatcher::Text("ERROR".to_string())
    );

    let mut disabled = rule("[");
    disabled.enabled = false;
    disabled.matcher = VisualMatcher::Regex("[".to_string());
    let report = VisualRulesEnvelope::new(vec![disabled])
        .validate_for_load()
        .expect("disabled invalid regex is recoverable");
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == ValidationSeverity::Warning && diagnostic.message.contains("regex")
    }));
    assert!(report.evaluator_rules.is_empty());
}

#[test]
fn envelope_validation_rejects_limits_and_recovers_valid_siblings_in_order() {
    let mut bad = rule(&"x".repeat(513));
    bad.name = Some("n".repeat(81));
    bad.style = style(Some("not-a-palette-token"), None);
    let envelope = VisualRulesEnvelope::new(vec![rule("first"), bad, rule("last")]);

    let report = envelope
        .validate_for_load()
        .expect("entry failure is recoverable");
    assert_eq!(report.evaluator_rules.len(), 2);
    assert_eq!(
        report.evaluator_rules[0].matcher,
        VisualMatcher::Text("first".to_string())
    );
    assert_eq!(
        report.evaluator_rules[1].matcher,
        VisualMatcher::Text("last".to_string())
    );
    assert_eq!(report.diagnostics.len(), 1);

    let too_many =
        VisualRulesEnvelope::new((0..101).map(|index| rule(&index.to_string())).collect());
    assert!(too_many.validate_for_load().is_err());
}

fn temp_log_path(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("logmancer-{name}-{suffix}.log"))
}

fn wait_for_indexed_lines(registry: &LogRegistry, file_id: &str, expected: usize) {
    for _ in 0..20 {
        if registry
            .with_reader(file_id, |reader| reader.file_info())
            .expect("reader")
            .expect("file info")
            .total_lines
            >= expected
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("reader did not index {expected} lines");
}

#[test]
fn registry_readers_capture_global_snapshot_without_changing_filter_search_or_navigation() {
    let path = temp_log_path("visual-rules-snapshot");
    let mut file = File::create(&path).expect("create log");
    writeln!(file, "INFO boot").expect("write");
    writeln!(file, "ERROR disk").expect("write");
    writeln!(file, "WARN cache").expect("write");
    drop(file);

    let registry = LogRegistry::new();
    let first_id = registry
        .open_file(path.to_str().expect("path"))
        .expect("open first");
    registry
        .apply_visual_rules_memory(VisualRulesEnvelope::new(vec![rule("ERROR")]))
        .expect("apply first snapshot");

    wait_for_indexed_lines(&registry, &first_id, 3);

    let first_page = registry
        .with_reader(&first_id, |reader| reader.read_page(0, 3))
        .expect("first reader")
        .expect("read page");
    assert_eq!(
        first_page
            .lines
            .iter()
            .map(|line| line.number)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        first_page.lines[1].style,
        Some(style(Some("red"), Some("default")))
    );

    registry
        .apply_visual_rules_memory(VisualRulesEnvelope::new(vec![rule("WARN")]))
        .expect("swap snapshot");
    let second_id = registry
        .open_file(path.to_str().expect("path"))
        .expect("open second");
    wait_for_indexed_lines(&registry, &second_id, 3);
    let second_page = registry
        .with_reader(&second_id, |reader| reader.read_page(0, 3))
        .expect("second reader")
        .expect("read future reader");
    assert_eq!(second_page.lines[1].style, None);
    assert_eq!(
        second_page.lines[2].style,
        Some(style(Some("red"), Some("default")))
    );

    let (filtered, searched, status) = registry
        .with_reader(&first_id, |reader| {
            reader.filter("ERROR|WARN".to_string());
            let filtered = reader.read_filter(0, 3);
            let searched = reader.apply_search("WARN".to_string(), 3);
            let status = reader.search_status();
            (filtered, searched, status)
        })
        .expect("first reader");
    let filtered = filtered.expect("filtered read");
    assert_eq!(
        filtered
            .lines
            .iter()
            .map(|line| line.number)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    let searched = searched.expect("search");
    assert_eq!(
        searched
            .lines
            .iter()
            .map(|line| line.number)
            .collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
    assert_eq!(status.query.as_deref(), Some("WARN"));

    std::fs::remove_file(path).expect("remove log");
}

fn temp_config_path(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("logmancer-{name}-{suffix}.json"))
}

fn native_manager(path: &std::path::Path) -> std::sync::Arc<VisualRulesManager> {
    VisualRulesManager::with_store(std::sync::Arc::new(NativeVisualRulesStore::new(
        path.to_path_buf(),
    )))
}

struct ProcessPausingReplacer;

impl AtomicFileReplacer for ProcessPausingReplacer {
    fn save_new(&self, _path: &std::path::Path, _bytes: &[u8]) -> std::io::Result<StoreCommit> {
        unreachable!("the process proof replaces an existing file")
    }

    fn replace(&self, path: &std::path::Path, bytes: &[u8]) -> std::io::Result<StoreCommit> {
        println!("EVENT PUBLISH");
        std::io::stdout().flush()?;
        let mut release = String::new();
        std::io::stdin().read_line(&mut release)?;
        assert_eq!(release.trim(), "RELEASE");
        std::fs::write(path, bytes)?;
        Ok(StoreCommit::Committed)
    }
}

#[test]
fn native_store_process_worker() {
    let Some(path) = std::env::var_os("LOGMANCER_PROCESS_STORE_PATH") else {
        return;
    };
    let pattern = std::env::var("LOGMANCER_PROCESS_PATTERN").expect("worker pattern");
    let store = NativeVisualRulesStore::with_replacer(
        std::path::PathBuf::from(path),
        std::sync::Arc::new(ProcessPausingReplacer),
    );
    let manager = VisualRulesManager::with_store(std::sync::Arc::new(store));
    let state = manager.load().expect("worker load");

    println!("EVENT READY");
    std::io::stdout().flush().expect("flush ready event");
    let mut start = String::new();
    std::io::stdin().read_line(&mut start).expect("read start");
    assert_eq!(start.trim(), "START");

    let result = manager.replace(
        state.revision,
        VisualRulesEnvelope::new(vec![rule(&pattern)]),
    );
    let outcome = match result {
        Ok(_) => "COMMITTED",
        Err(error) if error.is_source_conflict() => "CONFLICT",
        Err(error) => panic!("unexpected worker error: {error}"),
    };
    println!("EVENT RESULT {outcome}");
    std::io::stdout().flush().expect("flush result event");
}

fn spawn_store_worker(
    id: usize,
    path: &std::path::Path,
    pattern: &str,
    events: Sender<(usize, String)>,
) -> (Child, ChildStdin) {
    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", "native_store_process_worker", "--nocapture"])
        .env("LOGMANCER_PROCESS_STORE_PATH", path)
        .env("LOGMANCER_PROCESS_PATTERN", pattern)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn store worker");
    let stdout = child.stdout.take().expect("worker stdout");
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout)
            .lines()
            .map_while(Result::ok)
        {
            if let Some(event) = line.strip_prefix("EVENT ") {
                events.send((id, event.to_string())).expect("send event");
            }
        }
    });
    let stdin = child.stdin.take().expect("worker stdin");
    (child, stdin)
}

fn process_event(events: &Receiver<(usize, String)>) -> (usize, String) {
    events
        .recv_timeout(Duration::from_secs(5))
        .expect("worker event before deadlock timeout")
}

#[test]
fn separate_processes_serialize_native_compare_and_commit() {
    let path = temp_config_path("visual-rules-process-lock");
    let source = serde_json::to_vec(&VisualRulesEnvelope::new(vec![rule("ERROR")]))
        .expect("serialize source");
    std::fs::write(&path, source).expect("write source");
    let (event_tx, event_rx) = mpsc::channel();
    let (mut first, mut first_stdin) = spawn_store_worker(0, &path, "WARN", event_tx.clone());
    let (mut second, mut second_stdin) = spawn_store_worker(1, &path, "INFO", event_tx);

    let mut ready = [false; 2];
    while !ready.iter().all(|value| *value) {
        let (id, event) = process_event(&event_rx);
        assert_eq!(event, "READY");
        ready[id] = true;
    }
    writeln!(first_stdin, "START").expect("start first worker");
    writeln!(second_stdin, "START").expect("start second worker");

    let (publisher, event) = process_event(&event_rx);
    assert_eq!(event, "PUBLISH");
    let mut publications = 1;
    let mut results = Vec::new();
    match event_rx.recv_timeout(Duration::from_secs(2)) {
        Ok((id, event)) => {
            assert_eq!(event, "PUBLISH");
            publications += 1;
            writeln!([&mut first_stdin, &mut second_stdin][id], "RELEASE")
                .expect("release second publisher");
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        Err(error) => panic!("worker event channel failed: {error}"),
    }
    writeln!([&mut first_stdin, &mut second_stdin][publisher], "RELEASE")
        .expect("release first publisher");

    while results.len() < 2 {
        let (id, event) = process_event(&event_rx);
        if event == "PUBLISH" {
            publications += 1;
            writeln!([&mut first_stdin, &mut second_stdin][id], "RELEASE")
                .expect("release publisher");
        } else if let Some(result) = event.strip_prefix("RESULT ") {
            results.push(result.to_string());
        } else {
            panic!("unexpected worker event: {event}");
        }
    }
    assert!(first.wait().expect("first worker status").success());
    assert!(second.wait().expect("second worker status").success());
    results.sort();
    assert_eq!(publications, 1);
    assert_eq!(results, ["COMMITTED", "CONFLICT"]);

    std::fs::remove_file(&path).expect("remove config");
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    std::fs::remove_file(lock_path).expect("remove lock file");
}

#[test]
fn native_persistence_upsert_creates_updates_and_preserves_source_conflicts() {
    let path = temp_config_path("visual-rules-persistence");
    let store = NativeVisualRulesStore::new(path.clone());
    let manager = VisualRulesManager::with_store(std::sync::Arc::new(store));

    let loaded = manager.load().expect("missing config loads safely");
    assert_eq!(loaded.revision, 1);
    assert!(loaded.envelope.rules.is_empty());

    let saved = manager
        .upsert(
            loaded.revision,
            VisualRulesEnvelope::new(vec![rule("ERROR")]),
        )
        .expect("first upsert");
    assert_eq!(saved.outcome, SaveOutcome::Committed);
    assert!(path.exists());

    let replaced = manager
        .upsert(saved.revision, VisualRulesEnvelope::new(vec![rule("WARN")]))
        .expect("upsert replaces an existing configuration with a backup");
    assert_eq!(replaced.outcome, SaveOutcome::Committed);
    let backup_prefix = path
        .file_stem()
        .and_then(|name| name.to_str())
        .expect("config file stem")
        .to_string();
    let backups: Vec<_> = std::fs::read_dir(path.parent().expect("parent"))
        .expect("read config directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| {
            candidate
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&backup_prefix) && name.ends_with(".bak"))
        })
        .collect();
    assert_eq!(backups.len(), 1);
    assert!(
        String::from_utf8(std::fs::read(&backups[0]).expect("read backup"))
            .expect("backup JSON")
            .contains("ERROR")
    );
    assert!(
        String::from_utf8(std::fs::read(&path).expect("read replacement"))
            .expect("replacement JSON")
            .contains("WARN")
    );

    std::fs::write(&path, "{\"schemaVersion\":1,\"rules\":[]}").expect("external write");
    let conflict = manager.upsert(
        replaced.revision,
        VisualRulesEnvelope::new(vec![rule("INFO")]),
    );
    assert!(
        conflict
            .expect_err("changed source conflicts")
            .is_source_conflict()
    );

    std::fs::remove_file(backups[0].clone()).expect("remove backup");
    std::fs::remove_file(path).expect("remove config");
}

#[test]
fn failed_publication_does_not_mutate_snapshot_and_warning_reconciles_commit() {
    let path = temp_config_path("visual-rules-failure");
    let store = NativeVisualRulesStore::new(path.clone());
    let manager = VisualRulesManager::with_store(std::sync::Arc::new(store));
    let initial = manager.load().expect("initial load");

    let blocked_path = path
        .parent()
        .expect("parent")
        .join("missing-parent")
        .join("rules.json");
    let blocked = VisualRulesManager::with_store(std::sync::Arc::new(NativeVisualRulesStore::new(
        blocked_path,
    )));
    let before = blocked.snapshot();
    let failure = blocked.save(0, VisualRulesEnvelope::new(vec![rule("ERROR")]));
    assert!(failure.expect_err("pre-publication write fails").is_io());
    assert_eq!(
        blocked.snapshot().evaluate("ERROR"),
        before.evaluate("ERROR")
    );

    let committed = manager
        .save(
            initial.revision,
            VisualRulesEnvelope::new(vec![rule("WARN")]),
        )
        .expect("save");
    assert_eq!(committed.outcome, SaveOutcome::Committed);
    assert_eq!(
        manager.snapshot().evaluate("WARN"),
        Some(style(Some("red"), Some("default")))
    );
    std::fs::remove_file(path).expect("remove config");
}

#[test]
fn envelope_load_failure_retains_the_last_good_snapshot_and_reports_a_diagnostic() {
    let path = temp_config_path("visual-rules-recovery");
    let manager = VisualRulesManager::with_store(std::sync::Arc::new(NativeVisualRulesStore::new(
        path.clone(),
    )));
    let initial = manager.load().expect("initial load");
    let saved = manager
        .save(
            initial.revision,
            VisualRulesEnvelope::new(vec![rule("ERROR")]),
        )
        .expect("first save");

    std::fs::write(&path, "not-json").expect("corrupt config");
    let recovered = manager.load().expect("malformed config recovers");
    assert_eq!(recovered.revision, saved.revision + 1);
    assert_eq!(
        manager.snapshot().evaluate("ERROR"),
        Some(style(Some("red"), Some("default")))
    );
    assert_eq!(recovered.diagnostics.len(), 1);
    assert!(recovered.diagnostics[0].message.contains("expected"));

    let repaired = manager
        .replace(
            recovered.revision,
            VisualRulesEnvelope::new(vec![rule("WARN")]),
        )
        .expect("replace repairs the observed malformed source");
    assert_eq!(repaired.outcome, SaveOutcome::Committed);
    assert_eq!(
        manager.snapshot().evaluate("WARN"),
        Some(style(Some("red"), Some("default")))
    );

    std::fs::remove_file(path).expect("remove config");
}

#[test]
fn save_rejects_more_than_fifty_enabled_rules_without_mutation() {
    let path = temp_config_path("visual-rules-enabled-limit");
    let manager = native_manager(&path);
    let initial = manager.load().expect("missing config loads");
    let error = manager
        .save(
            initial.revision,
            VisualRulesEnvelope::new(vec![rule("ERROR"); 51]),
        )
        .expect_err("enabled-rule limit is enforced on save");

    assert_eq!(
        error,
        VisualRulesError::Validation("at most 50 rules may be enabled".to_string())
    );
    assert!(!path.exists());
    assert_eq!(manager.state().revision, initial.revision);
    assert_eq!(manager.state().envelope, initial.envelope);
    assert_eq!(manager.snapshot().evaluate("ERROR"), None);
}

#[test]
fn unsupported_schema_load_preserves_last_good_state_and_allows_repair() {
    let path = temp_config_path("visual-rules-schema-recovery");
    let manager = native_manager(&path);
    let loaded = manager.load().expect("missing config loads");
    let saved = manager
        .save(
            loaded.revision,
            VisualRulesEnvelope::new(vec![rule("ERROR")]),
        )
        .expect("save last-known-good config");
    std::fs::write(&path, r#"{"schemaVersion":2,"rules":[]}"#).expect("write future schema");

    let recovered = manager.load().expect("unsupported schema recovers");
    assert_eq!(recovered.revision, saved.revision + 1);
    assert_eq!(recovered.envelope, manager.state().envelope);
    assert_eq!(recovered.envelope.rules, vec![rule("ERROR")]);
    assert_eq!(
        recovered.diagnostics[0].message,
        "unsupported schemaVersion"
    );
    assert_eq!(
        recovered.diagnostics[0].severity,
        ValidationSeverity::Warning
    );
    assert_eq!(manager.snapshot().evaluate("ERROR"), red_style());

    manager
        .replace(
            recovered.revision,
            VisualRulesEnvelope::new(vec![rule("WARN")]),
        )
        .expect("replace repairs the observed unsupported source");
    assert_eq!(manager.snapshot().evaluate("WARN"), red_style());
    std::fs::remove_file(path).expect("remove config");
}

#[test]
fn manager_enforces_exact_persisted_size_boundary() {
    let path = temp_config_path("visual-rules-exact-size");
    let manager = native_manager(&path);
    let loaded = manager.load().expect("missing config loads");
    let accepted = envelope_with_serialized_size(262_144);
    let saved = manager
        .save(loaded.revision, accepted.clone())
        .expect("exactly 256 KiB is accepted");
    let persisted = std::fs::read(&path).expect("read accepted boundary");
    assert_eq!(persisted.len(), 262_144);
    assert_eq!(manager.state().envelope, accepted);

    let rejected = envelope_with_serialized_size(262_145);
    let error = manager
        .replace(saved.revision, rejected)
        .expect_err("one byte over 256 KiB is rejected");
    assert_eq!(
        error,
        VisualRulesError::Validation("visual rules configuration exceeds 256 KiB".to_string())
    );
    assert_eq!(
        std::fs::read(&path).expect("read unchanged boundary"),
        persisted
    );
    assert_eq!(manager.state().revision, saved.revision);
    assert_eq!(manager.state().envelope, accepted);
    std::fs::remove_file(path).expect("remove config");
}

#[test]
fn over_count_loads_preserve_last_good_state_and_report_diagnostics() {
    for (name, invalid, expected) in [
        (
            "stored",
            VisualRulesEnvelope::new(
                (0..101)
                    .map(|index| {
                        let mut rule = rule(&index.to_string());
                        rule.enabled = false;
                        rule
                    })
                    .collect(),
            ),
            "at most 100 rules may be stored",
        ),
        (
            "enabled",
            VisualRulesEnvelope::new(vec![rule("WARN"); 51]),
            "at most 50 rules may be enabled",
        ),
    ] {
        let path = temp_config_path(name);
        let good = VisualRulesEnvelope::new(vec![rule("ERROR")]);
        std::fs::write(
            &path,
            serde_json::to_vec(&good).expect("serialize good config"),
        )
        .expect("write good config");
        let manager = native_manager(&path);
        let loaded = manager.load().expect("good config loads");
        std::fs::write(
            &path,
            serde_json::to_vec(&invalid).expect("serialize over-count config"),
        )
        .expect("write over-count config");

        let recovered = manager.load().expect("over-count config recovers");
        assert_eq!(recovered.revision, loaded.revision + 1);
        assert_eq!(recovered.envelope, good);
        assert_eq!(recovered.diagnostics[0].message, expected);
        assert_eq!(
            recovered.diagnostics[0].severity,
            ValidationSeverity::Warning
        );
        assert_eq!(manager.snapshot().evaluate("ERROR"), red_style());
        assert_eq!(manager.snapshot().evaluate("WARN"), None);
        std::fs::remove_file(path).expect("remove config");
    }
}

#[test]
fn stale_base_revision_leaves_manager_and_persistence_unchanged() {
    let path = temp_config_path("visual-rules-stale-revision");
    let manager = native_manager(&path);
    let loaded = manager.load().expect("missing config loads");
    let saved = manager
        .save(
            loaded.revision,
            VisualRulesEnvelope::new(vec![rule("ERROR")]),
        )
        .expect("save current revision");
    let before = manager.state();
    let persisted = std::fs::read(&path).expect("read current config");

    let error = manager
        .replace(
            loaded.revision,
            VisualRulesEnvelope::new(vec![rule("WARN")]),
        )
        .expect_err("stale revision conflicts");
    assert_eq!(error, VisualRulesError::RevisionConflict);
    assert_eq!(
        std::fs::read(&path).expect("read unchanged config"),
        persisted
    );
    assert_eq!(manager.state().revision, before.revision);
    assert_eq!(manager.state().envelope, before.envelope);
    assert_eq!(manager.state().diagnostics, before.diagnostics);
    assert_eq!(manager.snapshot().evaluate("ERROR"), red_style());
    assert_eq!(manager.snapshot().evaluate("WARN"), None);

    manager
        .replace(saved.revision, VisualRulesEnvelope::new(vec![rule("INFO")]))
        .expect("unchanged source remains replaceable at the current revision");
    std::fs::remove_file(path).expect("remove config");
}

#[test]
fn oversized_native_load_is_bounded_and_retains_the_last_good_state() {
    let path = temp_config_path("visual-rules-oversized-load");
    let initial_envelope = VisualRulesEnvelope::new(vec![rule("ERROR")]);
    std::fs::write(
        &path,
        serde_json::to_vec(&initial_envelope).expect("serialize initial envelope"),
    )
    .expect("write initial config");
    let store = std::sync::Arc::new(NativeVisualRulesStore::new(path.clone()));
    let manager = VisualRulesManager::with_store(store.clone());
    let initial = manager.load().expect("initial config loads");

    let oversized_bytes =
        serde_json::to_vec(&oversized_valid_envelope()).expect("serialize oversized envelope");
    std::fs::write(&path, oversized_bytes).expect("write oversized config");
    let bounded_source = store
        .read()
        .expect("read oversized source")
        .expect("source");
    assert_eq!(
        bounded_source.len(),
        VisualRulesEnvelope::MAX_PERSISTED_SIZE + 1
    );

    let recovered = manager.load().expect("oversized config recovers");
    assert_eq!(recovered.revision, initial.revision + 1);
    assert_eq!(recovered.envelope, initial_envelope);
    assert_eq!(recovered.diagnostics.len(), 1);
    assert!(recovered.diagnostics[0].message.contains("256 KiB"));
    assert_eq!(
        manager.snapshot().evaluate("ERROR"),
        Some(style(Some("red"), Some("default")))
    );

    std::fs::remove_file(path).expect("remove config");
}

#[test]
fn oversized_tail_change_conflicts_until_the_complete_source_is_reloaded() {
    let path = temp_config_path("visual-rules-oversized-tail-conflict");
    let initial_envelope = VisualRulesEnvelope::new(vec![rule("ERROR")]);
    std::fs::write(
        &path,
        serde_json::to_vec(&initial_envelope).expect("serialize initial envelope"),
    )
    .expect("write initial config");
    let manager = VisualRulesManager::with_store(std::sync::Arc::new(NativeVisualRulesStore::new(
        path.clone(),
    )));
    manager.load().expect("initial config loads");

    let mut oversized = vec![b'a'; VisualRulesEnvelope::MAX_PERSISTED_SIZE + 4096];
    std::fs::write(&path, &oversized).expect("write oversized config");
    let recovered = manager.load().expect("oversized config recovers");
    assert_eq!(recovered.envelope, initial_envelope);
    assert!(recovered.diagnostics[0].message.contains("256 KiB"));

    oversized[VisualRulesEnvelope::MAX_PERSISTED_SIZE + 1024] = b'b';
    std::fs::write(&path, &oversized).expect("mutate only the unread tail");
    let before_conflict = manager.state();
    let conflict = manager.replace(
        recovered.revision,
        VisualRulesEnvelope::new(vec![rule("WARN")]),
    );
    assert!(
        conflict
            .expect_err("tail-only source change conflicts")
            .is_source_conflict()
    );
    assert_eq!(
        std::fs::read(&path).expect("read mutated source"),
        oversized
    );
    assert_eq!(manager.state().revision, before_conflict.revision);
    assert_eq!(manager.state().envelope, before_conflict.envelope);
    assert_eq!(
        manager.snapshot().evaluate("ERROR"),
        Some(style(Some("red"), Some("default")))
    );

    let observed = manager.load().expect("changed oversized source reloads");
    let repaired = manager
        .replace(
            observed.revision,
            VisualRulesEnvelope::new(vec![rule("WARN")]),
        )
        .expect("unchanged oversized source can be repaired");
    assert_eq!(repaired.outcome, SaveOutcome::Committed);
    assert_eq!(
        std::fs::read(&path).expect("read repaired source"),
        serde_json::to_vec_pretty(&VisualRulesEnvelope::new(vec![rule("WARN")]))
            .expect("serialize repaired source")
    );
    assert_eq!(
        manager.snapshot().evaluate("WARN"),
        Some(style(Some("red"), Some("default")))
    );

    std::fs::remove_file(path).expect("remove config");
}

#[test]
fn oversized_serialized_save_and_replace_leave_state_and_source_unchanged() {
    let path = temp_config_path("visual-rules-oversized-save");
    let manager = VisualRulesManager::with_store(std::sync::Arc::new(NativeVisualRulesStore::new(
        path.clone(),
    )));
    let initial = manager.load().expect("missing config loads");
    let oversized = oversized_valid_envelope();

    let save_error = manager
        .save(initial.revision, oversized.clone())
        .expect_err("oversized first save is rejected");
    assert!(save_error.to_string().contains("256 KiB"));
    assert!(!path.exists());
    assert_eq!(manager.state().revision, initial.revision);
    assert_eq!(manager.state().envelope, initial.envelope);
    assert_eq!(manager.snapshot().evaluate("ERROR"), None);

    let saved = manager
        .save(
            initial.revision,
            VisualRulesEnvelope::new(vec![rule("ERROR")]),
        )
        .expect("valid save still uses the unchanged source and revision");
    let persisted = std::fs::read(&path).expect("read persisted config");
    let before_replace = manager.state();

    let replace_error = manager
        .replace(saved.revision, oversized)
        .expect_err("oversized replacement is rejected");
    assert!(replace_error.to_string().contains("256 KiB"));
    assert_eq!(
        std::fs::read(&path).expect("read unchanged config"),
        persisted
    );
    assert_eq!(manager.state().revision, before_replace.revision);
    assert_eq!(manager.state().envelope, before_replace.envelope);
    assert_eq!(
        manager.snapshot().evaluate("ERROR"),
        Some(style(Some("red"), Some("default")))
    );

    std::fs::remove_file(path).expect("remove config");
}

struct PausingReplacer {
    replacement_count: std::sync::atomic::AtomicUsize,
    first_replacement_started: std::sync::mpsc::Sender<()>,
    release_first_replacement: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}

impl AtomicFileReplacer for PausingReplacer {
    fn save_new(&self, _path: &std::path::Path, _bytes: &[u8]) -> std::io::Result<StoreCommit> {
        unreachable!("the test replaces an existing file")
    }

    fn replace(&self, path: &std::path::Path, bytes: &[u8]) -> std::io::Result<StoreCommit> {
        if self
            .replacement_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            == 0
        {
            self.first_replacement_started
                .send(())
                .expect("report first replacement");
            self.release_first_replacement
                .lock()
                .expect("release receiver lock")
                .recv_timeout(std::time::Duration::from_secs(5))
                .expect("first replacement was not released before timeout");
        }
        std::fs::write(path, bytes)?;
        Ok(StoreCommit::Committed)
    }
}

#[test]
fn separate_native_stores_serialize_concurrent_manager_replacements() {
    let path = temp_config_path("visual-rules-concurrent-replace");
    let source = serde_json::to_vec(&VisualRulesEnvelope::new(vec![rule("ERROR")]))
        .expect("serialize source");
    std::fs::write(&path, source).expect("write source");

    let (replacement_started_tx, replacement_started_rx) = std::sync::mpsc::channel();
    let (release_replacement_tx, release_replacement_rx) = std::sync::mpsc::channel();
    let replacer = std::sync::Arc::new(PausingReplacer {
        replacement_count: std::sync::atomic::AtomicUsize::new(0),
        first_replacement_started: replacement_started_tx,
        release_first_replacement: std::sync::Mutex::new(release_replacement_rx),
    });
    let first_store = NativeVisualRulesStore::with_replacer(path.clone(), replacer.clone());
    let second_store = NativeVisualRulesStore::with_replacer(path.clone(), replacer);
    let first = VisualRulesManager::with_store(std::sync::Arc::new(first_store));
    let second = VisualRulesManager::with_store(std::sync::Arc::new(second_store));
    let first_state = first.load().expect("first manager load");
    let second_state = second.load().expect("second manager load");

    let first_thread = std::thread::spawn(move || {
        first.replace(
            first_state.revision,
            VisualRulesEnvelope::new(vec![rule("WARN")]),
        )
    });
    replacement_started_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("first replacement did not reach the publication boundary");

    let (second_attempted_tx, second_attempted_rx) = std::sync::mpsc::channel();
    let second_thread = std::thread::spawn(move || {
        second_attempted_tx
            .send(())
            .expect("report second replacement attempt");
        second.replace(
            second_state.revision,
            VisualRulesEnvelope::new(vec![rule("INFO")]),
        )
    });
    second_attempted_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("second manager did not attempt replacement");
    release_replacement_tx
        .send(())
        .expect("release first replacement");

    let first_result = first_thread.join().expect("first manager thread");
    let second_result = second_thread.join().expect("second manager thread");

    assert_eq!(
        usize::from(first_result.is_ok()) + usize::from(second_result.is_ok()),
        1
    );
    assert!(
        first_result
            .err()
            .or_else(|| second_result.err())
            .expect("one stale source conflicts")
            .is_source_conflict()
    );
    assert_eq!(
        std::fs::read(&path).expect("read committed source"),
        serde_json::to_vec_pretty(&VisualRulesEnvelope::new(vec![rule("WARN")]))
            .expect("serialize committed replacement")
    );

    std::fs::remove_file(path).expect("remove config");
}

struct WarningStore {
    bytes: std::sync::Mutex<Option<Vec<u8>>>,
}

impl WarningStore {
    fn missing() -> Self {
        Self {
            bytes: std::sync::Mutex::new(None),
        }
    }
}

impl VisualRulesStore for WarningStore {
    fn read(&self) -> std::io::Result<Option<Vec<u8>>> {
        Ok(self.bytes.lock().expect("store lock").clone())
    }

    fn save_new(&self, bytes: &[u8]) -> std::io::Result<StoreCommit> {
        let mut stored = self.bytes.lock().expect("store lock");
        if stored.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "target appeared before publication",
            ));
        }
        *stored = Some(bytes.to_vec());
        Ok(StoreCommit::with_warning("parent directory sync failed"))
    }

    fn replace(&self, _bytes: &[u8]) -> std::io::Result<StoreCommit> {
        unreachable!("first save uses save_new")
    }

    fn compare_and_commit(
        &self,
        expected: Option<&[u8]>,
        bytes: &[u8],
        replace: bool,
    ) -> std::io::Result<StoreCommit> {
        if self.read()?.as_deref() != expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "source changed before publication",
            ));
        }
        if replace {
            self.replace(bytes)
        } else {
            self.save_new(bytes)
        }
    }
}

#[test]
fn post_publication_warning_commits_snapshot_revision_and_source_once() {
    let store = std::sync::Arc::new(WarningStore::missing());
    let manager = VisualRulesManager::with_store(store.clone());
    let initial = manager.load().expect("missing store loads");

    let result = manager
        .save(
            initial.revision,
            VisualRulesEnvelope::new(vec![rule("ERROR")]),
        )
        .expect("publication warning is committed");
    assert_eq!(
        result.outcome,
        SaveOutcome::CommittedWithWarning("parent directory sync failed".to_string())
    );
    assert_eq!(result.revision, initial.revision + 1);
    assert_eq!(
        manager.snapshot().evaluate("ERROR"),
        Some(style(Some("red"), Some("default")))
    );
    assert!(store.read().expect("stored bytes").is_some());

    let conflict = manager.save(
        result.revision,
        VisualRulesEnvelope::new(vec![rule("WARN")]),
    );
    assert!(
        conflict
            .expect_err("second first-save cannot clobber")
            .is_source_conflict()
    );
}

struct ConcurrentCreatorStore;

impl VisualRulesStore for ConcurrentCreatorStore {
    fn read(&self) -> std::io::Result<Option<Vec<u8>>> {
        Ok(None)
    }

    fn save_new(&self, _bytes: &[u8]) -> std::io::Result<StoreCommit> {
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "another process published first",
        ))
    }

    fn replace(&self, _bytes: &[u8]) -> std::io::Result<StoreCommit> {
        unreachable!("missing target must use no-clobber save")
    }

    fn compare_and_commit(
        &self,
        _expected: Option<&[u8]>,
        _bytes: &[u8],
        _replace: bool,
    ) -> std::io::Result<StoreCommit> {
        self.save_new(&[])
    }
}

#[test]
fn first_save_never_clobbers_a_concurrently_created_target() {
    let manager = VisualRulesManager::with_store(std::sync::Arc::new(ConcurrentCreatorStore));
    let initial = manager.load().expect("missing store loads");
    let before = manager.snapshot();

    let failure = manager.save(
        initial.revision,
        VisualRulesEnvelope::new(vec![rule("ERROR")]),
    );
    assert!(
        failure
            .expect_err("concurrent target conflicts")
            .is_source_conflict()
    );
    assert_eq!(manager.state().revision, initial.revision);
    assert_eq!(
        manager.snapshot().evaluate("ERROR"),
        before.evaluate("ERROR")
    );
}
