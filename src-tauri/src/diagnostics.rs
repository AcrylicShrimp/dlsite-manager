use dm_audit::{AuditEvent, AuditLogger, ExportScope};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

pub fn environment() -> BTreeMap<String, OsString> {
    dm_audit::ENVIRONMENT_KEYS
        .iter()
        .filter_map(|key| std::env::var_os(key).map(|v| ((*key).into(), v)))
        .collect()
}
pub fn initialize(app: &tauri::App) -> AuditLogger {
    use tauri::Manager;
    let path = app
        .path()
        .app_log_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("dlsite-manager-diagnostics-unavailable"));
    let logger = AuditLogger::new(path).expect("in-memory diagnostics initialization");
    logger.startup_metadata(
        env!("CARGO_PKG_VERSION"),
        option_env!("DMSITE_BUILD_REVISION"),
        &std::env::current_exe().unwrap_or_default(),
        &std::env::current_dir().unwrap_or_default(),
        logger.environment(&environment()),
    );
    platform_metadata(&logger);
    logger
}
#[tauri::command]
pub fn diagnostic_summary(logger: State<'_, AuditLogger>) -> Value {
    logger.summary()
}
#[tauri::command]
pub fn diagnostic_runs(
    logger: State<'_, AuditLogger>,
) -> Result<Vec<dm_audit::RunSummary>, String> {
    logger
        .retained_runs()
        .map_err(|_| "Unable to read retained runs".into())
}
#[tauri::command]
pub fn diagnostic_operation(
    logger: State<'_, AuditLogger>,
    operation_id: String,
) -> Vec<AuditEvent> {
    logger.operation_events(&operation_id)
}
#[tauri::command]
pub async fn export_diagnostics(
    app: AppHandle,
    logger: State<'_, AuditLogger>,
    scope: ExportScope,
) -> Result<Option<String>, String> {
    let logger = logger.inner().clone();
    let (send, recv) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Export diagnostics — environment and operation records; no account secrets")
        .set_file_name(format!("dlsite-manager-{}.zip", logger.run_id()))
        .add_filter("Diagnostics", &["zip"])
        .save_file(move |path| {
            let _ = send.send(path);
        });
    let Some(path) = recv.await.map_err(|_| "Save dialog unavailable")? else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|_| "Only local file destinations are supported")?;
    let saved = path.to_string_lossy().into_owned();
    tauri::async_runtime::spawn_blocking(move||logger.export(&path,scope)).await.map_err(|_|"Diagnostic export task failed")?
        .map_err(|_|"Cannot save diagnostics. Choose a new filename in a writable directory, or copy the support summary.")?;
    Ok(Some(saved))
}

#[derive(Default)]
pub struct FrontendReports(Mutex<(Option<Instant>, u32, Vec<String>)>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrontendFailure {
    client_id: String,
    command: String,
    category: String,
    #[serde(default)]
    repeat_count: u32,
    outcome: Option<dm_audit::AuditOutcome>,
    duration_ms: Option<u64>,
}
#[tauri::command]
pub fn report_frontend_failure(
    logger: State<'_, AuditLogger>,
    reports: State<'_, FrontendReports>,
    failure: FrontendFailure,
) -> Result<(), String> {
    if failure.client_id.len() > 64 || failure.command.len() > 96 || failure.category.len() > 64 {
        return Err("Invalid diagnostic report".into());
    }
    let mut reports = reports.0.lock().unwrap_or_else(|e| e.into_inner());
    if reports
        .0
        .is_none_or(|t| t.elapsed() >= Duration::from_secs(1))
    {
        reports.0 = Some(Instant::now());
        reports.1 = 0;
    }
    if reports.2.contains(&failure.client_id) {
        return Ok(());
    }
    if reports.1 >= 10 {
        return Err("Diagnostic report rate limited".into());
    }
    reports.1 += 1;
    reports.2.push(failure.client_id.clone());
    if reports.2.len() > 256 {
        reports.2.remove(0);
    }
    // The core validates command vocabulary and IDs; no frontend message/payload is accepted.
    let mut event = match failure
        .outcome
        .filter(|_| failure.command.starts_with("native."))
    {
        Some(dm_audit::AuditOutcome::Succeeded) => AuditEvent::succeeded(failure.command, ""),
        Some(dm_audit::AuditOutcome::Cancelled) => AuditEvent::cancelled(failure.command, ""),
        _ => AuditEvent::failed(failure.command, ""),
    };
    event.audience = "support".into();
    event.duration_ms = failure.duration_ms.map(|v| v.min(86400000));
    event.details = json!({"clientId":failure.client_id,"failureKind":if event.outcome==dm_audit::AuditOutcome::Failed{Some(if failure.category=="transport"{"transport"}else{"frontend"})}else{None},"repeatCount":failure.repeat_count.min(10000)});
    logger.record_now(event);
    Ok(())
}

fn platform_metadata(logger: &AuditLogger) {
    #[cfg(target_os = "linux")]
    {
        use std::io::Read;
        fn limited(path: &str) -> String {
            let mut value = String::new();
            if let Ok(file) = std::fs::File::open(path) {
                let _ = file.take(16384).read_to_string(&mut value);
            }
            value
        }
        let release = limited("/etc/os-release");
        let field = |name: &str| {
            release
                .lines()
                .find_map(|line| line.strip_prefix(name))
                .map(|s| s.trim_matches('"'))
        };
        let kernel = limited("/proc/sys/kernel/osrelease");
        let version = kernel.trim().split('-').next();
        logger.platform_metadata(version, field("ID="), field("VERSION_ID="));
    }
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("/usr/bin/sw_vers")
            .arg("-productVersion")
            .output()
            .ok();
        let version = output
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok());
        logger.platform_metadata(version.as_deref().map(str::trim), None, None);
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    logger.platform_metadata(None, None, None);
}
