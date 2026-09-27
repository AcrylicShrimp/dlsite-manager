use super::*;
use std::{collections::BTreeMap, ffi::OsString, fs, io::Read};

fn archive(logger: &AuditLogger, destination: &Path) -> BTreeMap<String, String> {
    logger.export(destination, ExportScope::default()).unwrap();
    let mut zip = zip::ZipArchive::new(fs::File::open(destination).unwrap()).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut file = zip.by_index(i).unwrap();
            let name = file.name().to_owned();
            let mut data = String::new();
            file.read_to_string(&mut data).unwrap();
            (name, data)
        })
        .collect()
}
fn events(data: &str) -> Vec<AuditEvent> {
    data.lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn secrets_are_absent_from_actual_files_and_export() {
    let dir = tempfile::tempdir().unwrap();
    let logger = AuditLogger::new(dir.path()).unwrap();
    let private = "PRIVATE-SEED-비밀-123";
    let environment = BTreeMap::from([
        (
            "PATH".into(),
            OsString::from(format!("/usr/bin:/home/{private}/bin::/usr/bin")),
        ),
        ("COOKIE_SECRET".into(), OsString::from(private)),
        (
            "DBUS_SESSION_BUS_ADDRESS".into(),
            OsString::from(format!("unix:path=/run/{private}")),
        ),
    ]);
    logger.startup_metadata(
        "3.3.0",
        Some("abcdef123"),
        Path::new(private),
        Path::new(private),
        logger.environment(&environment),
    );
    let mut event=AuditEvent::failed(private,private).with_error(Some(private),private).with_details(json!({"bodySnippet":private,"accountId":private,"path":private,"errorDetails":{"message":private,"apiErrorKind":"unexpected_status","httpStatus":503},"newField":private}));
    event.stage = private.into();
    event.component = private.into();
    event.at = private.into();
    event.operation_id = Some(private.into());
    logger.record_now(event);
    assert!(logger.flush(Duration::from_secs(2)));
    let contents = archive(&logger, &dir.path().join("support.zip"));
    for data in contents.values() {
        assert!(!data.contains(private), "{data}");
        assert!(!data.contains("COOKIE_SECRET"));
    }
    let recorded = events(&contents["events.jsonl"]);
    assert_eq!(recorded[0].details["errorDetails"]["httpStatus"], 503);
    for entry in fs::read_dir(dir.path().join("diagnostics").join(logger.run_id())).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some() {
            assert!(!String::from_utf8_lossy(&fs::read(path).unwrap()).contains(private));
        }
    }
}

#[test]
fn unavailable_store_still_exports_memory_and_never_clobbers_destination() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("not-a-directory");
    fs::write(&root, "existing").unwrap();
    let logger = AuditLogger::new(&root).unwrap();
    logger.record_now(AuditEvent::failed("work.open", ""));
    let destination = dir.path().join("support.zip");
    let data = archive(&logger, &destination);
    assert_eq!(events(&data["events.jsonl"]).len(), 1);
    let manifest: Value = serde_json::from_str(&data["manifest.json"]).unwrap();
    assert_eq!(manifest["diskUnavailable"], true);
    assert_eq!(manifest["flushComplete"], false);
    let before = fs::read(&destination).unwrap();
    assert!(logger.export(&destination, ExportScope::default()).is_err());
    assert_eq!(fs::read(destination).unwrap(), before);
    assert_eq!(fs::read_to_string(root).unwrap(), "existing");
}

#[test]
fn environments_preserve_order_relationships_and_bound_unknown_values() {
    let dir = tempfile::tempdir().unwrap();
    let logger = AuditLogger::new(dir.path()).unwrap();
    let values = BTreeMap::from([
        ("APPDIR".into(), OsString::from("/private/App Dir")),
        (
            "PATH".into(),
            OsString::from(
                [
                    "/private/App Dir/bin",
                    "",
                    "/usr/bin",
                    "/private/App Dir/bin",
                ]
                .join(if cfg!(windows) { ";" } else { ":" }),
            ),
        ),
        ("LANG".into(), OsString::new()),
    ]);
    let env = logger.environment(&values);
    let entries = env.variables["PATH"]["detail"]["entries"]
        .as_array()
        .unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0], entries[3]);
    assert_eq!(entries[0]["origin"], "appImage");
    assert_eq!(entries[1]["origin"], "empty");
    assert_eq!(entries[2]["reference"], "/usr/bin");
    assert_eq!(env.variables["LANG"]["state"], "empty");
    assert_eq!(env.variables["LD_PRELOAD"]["state"], "unset");
    logger.startup_metadata(
        "3.3.0",
        None,
        Path::new("/private/App Dir/app"),
        Path::new("/private"),
        env.clone(),
    );
    let mut changed = values.clone();
    changed.remove("APPDIR");
    logger.opener_environment(Path::new("/private"), logger.environment(&changed));
    let inner = logger.owner.shared.lock();
    let event = inner.ring.back().unwrap();
    assert_eq!(
        event.details["parentEnvironment"]["variables"]["APPDIR"]["state"],
        "unset"
    );
    assert_eq!(event.details["startupEnvironmentChanged"], true);
    assert_eq!(event.details["childEnvironment"], "not_observed");
    assert_eq!(event.details["desktopCompletion"], "not_observed");
    assert!(serde_json::to_vec(event).unwrap().len() < EVENT_BYTES);
}

#[tokio::test]
async fn concurrent_scopes_and_command_errors_keep_their_own_parents() {
    let dir = tempfile::tempdir().unwrap();
    let logger = AuditLogger::new(dir.path()).unwrap();
    async fn run(logger: AuditLogger) -> String {
        let parent = logger.operation("start_work_download");
        let id = parent.id().to_owned();
        let result: std::result::Result<(), String> = parent
            .command(async {
                tokio::task::yield_now().await;
                let mut child = logger.job_operation("job", &format!("job-{}", Uuid::new_v4()));
                child
                    .scope(async {
                        record_current_failure("io", json!({"osCode":13}));
                        tokio::task::yield_now().await;
                    })
                    .await;
                child.finish(AuditOutcome::Failed, json!({}));
                Err("visible error".into())
            })
            .await;
        assert!(result.unwrap_err().contains(&id));
        id
    }
    let (a, b) = tokio::join!(run(logger.clone()), run(logger.clone()));
    assert_ne!(a, b);
    let inner = logger.owner.shared.lock();
    let children: Vec<_> = inner
        .ring
        .iter()
        .filter(|e| e.operation == "job" && e.kind == "started")
        .collect();
    assert_eq!(children.len(), 2);
    for child in children {
        assert!([&a, &b].contains(&child.parent_operation_id.as_ref().unwrap()));
        let cause = inner
            .ring
            .iter()
            .find(|e| e.stage == "cause" && e.operation_id == child.operation_id)
            .unwrap();
        assert_eq!(cause.parent_operation_id, child.parent_operation_id);
        assert_eq!(cause.job_id, child.job_id);
        assert_eq!(
            inner
                .ring
                .iter()
                .filter(|e| e.operation_id == child.operation_id && e.kind == "finished")
                .count(),
            1
        );
    }
}

#[test]
fn queue_and_ring_limits_drop_truthfully_without_blocking_disk() {
    let dir = tempfile::tempdir().unwrap();
    let logger = AuditLogger::new(dir.path()).unwrap();
    assert!(logger.flush(Duration::from_secs(2)));
    let mut inner = logger.owner.shared.lock();
    for _ in 0..5000 {
        logger.submit(&mut inner, AuditEvent::succeeded("runtime", ""));
    }
    assert!(inner.health.dropped_routine > 0);
    assert!(inner.health.ring_evicted > 0);
    let before = inner.queue_bytes;
    logger.submit(&mut inner, AuditEvent::failed("work.open", ""));
    assert!(inner.queue_bytes > before);
    assert!(inner.queue_bytes <= QUEUE_BYTES);
    assert!(inner.ring_bytes <= RING_BYTES);
}

#[test]
fn export_merges_disk_and_memory_once_and_reports_malformed_tail() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let logger = AuditLogger::new(dir.path()).unwrap();
    for _ in 0..20 {
        logger.record_now(AuditEvent::failed("work.open", ""));
    }
    assert!(logger.flush(Duration::from_secs(2)));
    let path = dir
        .path()
        .join("diagnostics")
        .join(logger.run_id())
        .join("events-9999.jsonl");
    fs::File::create(path)
        .unwrap()
        .write_all(b"{incomplete")
        .unwrap();
    let data = archive(&logger, &dir.path().join("support.zip"));
    let all = events(&data["events.jsonl"]);
    assert_eq!(all.len(), 20);
    assert!(all.windows(2).all(|w| w[1].sequence == w[0].sequence + 1));
    let manifest: Value = serde_json::from_str(&data["manifest.json"]).unwrap();
    assert_eq!(manifest["malformedRecords"], 1);
    assert_eq!(manifest["missingRangeCount"], 0);
}

#[test]
fn storage_recovers_after_initial_directory_failure() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    fs::write(&root, "").unwrap();
    let logger = AuditLogger::new(&root).unwrap();
    logger.record_now(AuditEvent::failed("work.open", ""));
    assert!(!logger.flush(Duration::from_secs(2)));
    fs::remove_file(&root).unwrap();
    logger.record_now(AuditEvent::failed("work.open", ""));
    assert!(logger.flush(Duration::from_secs(2)));
    assert_eq!(logger.health().last_written_sequence, 2);
    assert_eq!(logger.health().state, "degraded");
    let data = archive(&logger, &dir.path().join("support.zip"));
    assert_eq!(events(&data["events.jsonl"]).len(), 2);
    let manifest: Value = serde_json::from_str(&data["manifest.json"]).unwrap();
    assert_eq!(manifest["flushComplete"], true);
    assert_eq!(manifest["health"]["state"], "degraded");
    assert!(manifest["health"]["droppedCritical"].as_u64().unwrap() > 0);
}

#[cfg(unix)]
#[test]
fn export_does_not_follow_segment_symlinks_or_include_legacy_logs() {
    let dir = tempfile::tempdir().unwrap();
    let logger = AuditLogger::new(dir.path()).unwrap();
    assert!(logger.flush(Duration::from_secs(2)));
    let secret = dir.path().join("runtime.json");
    fs::write(&secret, "SECRET-OUTSIDE-STORE").unwrap();
    std::os::unix::fs::symlink(
        &secret,
        dir.path()
            .join("diagnostics")
            .join(logger.run_id())
            .join("events-9999.jsonl"),
    )
    .unwrap();
    let data = archive(&logger, &dir.path().join("support.zip"));
    assert!(data.values().all(|v| !v.contains("SECRET-OUTSIDE-STORE")));
    assert_eq!(data.len(), 3);
}

#[test]
fn environment_limits_keep_an_ordered_prefix_and_report_omissions() {
    let dir = tempfile::tempdir().unwrap();
    let logger = AuditLogger::new(dir.path()).unwrap();
    let paths = (0..200)
        .map(|i| format!("/private/path-{i}"))
        .collect::<Vec<_>>()
        .join(if cfg!(windows) { ";" } else { ":" });
    let values = BTreeMap::from([("PATH".into(), OsString::from(paths))]);
    let env = logger.environment(&values);
    assert!(serde_json::to_vec(&env.variables).unwrap().len() <= 6 * 1024);
    assert!(
        env.variables["PATH"]["detail"]["omittedEntries"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(!env.variables["PATH"]["detail"]["entries"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn safe_tracing_layer_drops_messages_debug_and_third_party_targets() {
    use tracing_subscriber::prelude::*;
    let dir = tempfile::tempdir().unwrap();
    let logger = AuditLogger::new(dir.path()).unwrap();
    let subscriber = tracing_subscriber::registry().with(DiagnosticLayer(logger.clone()));
    tracing::subscriber::with_default(subscriber, || {
        tracing::error!(target:"third_party",message="PRIVATE-SECRET",httpStatus=503u64);
        tracing::error!(target:"dm_diagnostic",message="PRIVATE-SECRET",error=?"PRIVATE-SECRET",kind="login",httpStatus=503u64);
    });
    let inner = logger.owner.shared.lock();
    assert_eq!(inner.ring.len(), 1);
    assert_eq!(inner.ring[0].details["httpStatus"], 503);
    assert!(!serde_json::to_string(&inner.ring)
        .unwrap()
        .contains("PRIVATE-SECRET"));
}

#[cfg(unix)]
#[test]
fn export_rejects_a_symlinked_store_root_and_keeps_ring_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("diagnostics")).unwrap();
    let logger = AuditLogger::new(dir.path()).unwrap();
    logger.record_now(AuditEvent::failed("work.open", ""));
    let data = archive(&logger, &dir.path().join("support.zip"));
    assert_eq!(events(&data["events.jsonl"]).len(), 1);
    let manifest: Value = serde_json::from_str(&data["manifest.json"]).unwrap();
    assert_eq!(manifest["diskUnavailable"], true);
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}
