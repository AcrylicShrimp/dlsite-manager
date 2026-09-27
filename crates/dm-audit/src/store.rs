use crate::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    time::SystemTime,
};

const SEGMENT_BYTES: u64 = 4 * 1024 * 1024;
const STORE_BYTES: u64 = 50 * 1024 * 1024;
const EXPORT_BYTES: usize = 20 * 1024 * 1024;
const RETENTION: Duration = Duration::from_secs(7 * 86400);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportScope {
    pub run_id: Option<String>,
    pub operation_id: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub run_id: String,
    pub current: bool,
}

fn io_busy() -> std::io::Error {
    std::io::Error::from(std::io::ErrorKind::WouldBlock)
}
fn regular(path: &Path) -> std::io::Result<bool> {
    Ok(fs::symlink_metadata(path)?.file_type().is_file())
}
fn directory(path: &Path) -> std::io::Result<()> {
    match fs::create_dir(path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e),
    }
    if !fs::symlink_metadata(path)?.file_type().is_dir() {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }
    Ok(())
}
fn lock_file(path: &Path) -> std::io::Result<File> {
    if fs::symlink_metadata(path).is_ok() && !regular(path)? {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.try_lock().map_err(|_| io_busy())?;
    Ok(file)
}
fn valid_run(value: &str) -> bool {
    value
        .strip_prefix("run-")
        .is_some_and(|s| Uuid::parse_str(s).is_ok())
}
fn segment(path: &Path) -> bool {
    path.file_name().and_then(|s| s.to_str()).is_some_and(|s| {
        s.starts_with("events-")
            && s.ends_with(".jsonl")
            && s[7..s.len() - 6].chars().all(|c| c.is_ascii_digit())
    })
}
fn run_dirs(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    if !fs::symlink_metadata(root)?.is_dir() {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }
    let mut dirs = Vec::new();
    for entry in fs::read_dir(root)?.take(4096) {
        let entry = entry?;
        if entry.file_type()?.is_dir() && entry.file_name().to_str().is_some_and(valid_run) {
            dirs.push(entry.path());
        }
    }
    Ok(dirs)
}

/// Called only under the store lock. Active runs are protected by an OS file lease.
fn reclaim(root: &Path, current: &Path, active: &Path, needed: u64) -> std::io::Result<()> {
    let mut total = 0;
    let mut candidates = Vec::new();
    for dir in run_dirs(root)? {
        let lease = if dir == current {
            None
        } else {
            lock_file(&dir.join("active.lock")).ok()
        };
        let deletable = dir == current || lease.is_some();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            let meta = entry.metadata()?;
            total += meta.len();
            if deletable && segment(&path) && path != active {
                candidates.push((
                    meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    path,
                    meta.len(),
                ));
            }
        }
    }
    candidates.sort_by_key(|v| v.0);
    for (modified, path, size) in candidates {
        if total + needed <= STORE_BYTES && modified.elapsed().unwrap_or_default() < RETENTION {
            continue;
        }
        fs::remove_file(path)?;
        total = total.saturating_sub(size);
    }
    // Remove only fully orphaned inactive run metadata/leases, never arbitrary files.
    for dir in run_dirs(root)? {
        if dir == current {
            continue;
        }
        let Ok(lease) = lock_file(&dir.join("active.lock")) else {
            continue;
        };
        let entries: Vec<_> = fs::read_dir(&dir)?.collect::<std::io::Result<Vec<_>>>()?;
        if entries.iter().any(|e| segment(&e.path())) {
            continue;
        }
        for name in ["run.json", "active.lock"] {
            let p = dir.join(name);
            if p.exists() && regular(&p)? {
                let size = p.metadata()?.len();
                if name == "active.lock" {
                    continue;
                }
                fs::remove_file(&p)?;
                total = total.saturating_sub(size);
            }
        }
        drop(lease);
        let _ = fs::remove_file(dir.join("active.lock"));
        let _ = fs::remove_dir(&dir);
    }
    if total + needed > STORE_BYTES {
        return Err(std::io::Error::from(std::io::ErrorKind::StorageFull));
    }
    Ok(())
}
struct Disk {
    root: PathBuf,
    run: PathBuf,
    _lease: File,
    file: File,
    path: PathBuf,
    index: u64,
    bytes: u64,
    date: String,
    metadata: Vec<u8>,
}
impl Disk {
    fn open(shared: &Shared) -> std::io::Result<Self> {
        fs::create_dir_all(&shared.root)?;
        let root = shared.root.join("diagnostics");
        directory(&root)?;
        let _guard = lock_file(&root.join("store.lock"))?;
        let run = root.join(&shared.run_id);
        directory(&run)?;
        let lease = lock_file(&run.join("active.lock"))?;
        let path = run.join("events-0001.jsonl");
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(Self {
            root,
            run,
            _lease: lease,
            file,
            path,
            index: 1,
            bytes: 0,
            date: Utc::now().format("%Y-%m-%d").to_string(),
            metadata: Vec::new(),
        })
    }
    fn append(&mut self, batch: &[(u64, Vec<u8>)], metadata: &Value) -> std::io::Result<()> {
        let _guard = lock_file(&self.root.join("store.lock"))?;
        let data = serde_json::to_vec(metadata)?;
        let meta_changed = data != self.metadata;
        let needed = batch.iter().map(|(_, b)| b.len() as u64).sum::<u64>()
            + if meta_changed { data.len() as u64 } else { 0 };
        reclaim(&self.root, &self.run, &self.path, needed)?;
        if meta_changed {
            let dest = self.run.join("run.json");
            if dest.exists() && !regular(&dest)? {
                return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
            }
            let mut tmp = tempfile::NamedTempFile::new_in(&self.run)?;
            tmp.write_all(&data)?;
            tmp.persist(&dest).map_err(|e| e.error)?;
            self.metadata = data;
        }
        for (_, bytes) in batch {
            let today = Utc::now().format("%Y-%m-%d").to_string();
            if self.bytes + bytes.len() as u64 > SEGMENT_BYTES || self.date != today {
                self.file.flush()?;
                self.index += 1;
                self.path = self.run.join(format!("events-{:04}.jsonl", self.index));
                self.file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&self.path)?;
                self.bytes = 0;
                self.date = today;
            }
            self.file.write_all(bytes)?;
            self.bytes += bytes.len() as u64;
        }
        self.file.flush()
    }
}

pub(crate) fn writer(shared: Arc<Shared>) {
    let mut disk = Disk::open(&shared)
        .inspect_err(|e| {
            let mut inner = shared.lock();
            inner.health.last_io_code = e.raw_os_error();
        })
        .ok();
    loop {
        let (batch, metadata, flush_ticket, stop) = {
            let mut inner = shared.lock();
            if inner.queue.is_empty() && !inner.stop && inner.flush_ack == inner.flush_requested {
                inner = shared
                    .wake
                    .wait_timeout(inner, Duration::from_secs(1))
                    .unwrap_or_else(|e| e.into_inner())
                    .0;
            }
            let batch: Vec<_> = inner.queue.drain(..).collect();
            inner.queue_bytes = 0;
            (
                batch,
                inner.metadata.clone(),
                inner.flush_requested,
                inner.stop,
            )
        };
        if disk.is_none() {
            disk = Disk::open(&shared)
                .inspect_err(|e| {
                    shared.lock().health.last_io_code = e.raw_os_error();
                })
                .ok();
        }
        let result = disk.as_mut().map(|d| {
            let start = Instant::now();
            loop {
                match d.append(&batch, &metadata) {
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && start.elapsed() < Duration::from_secs(1) =>
                    {
                        std::thread::sleep(Duration::from_millis(20))
                    }
                    result => break result,
                }
            }
        });
        let lost_critical = batch
            .iter()
            .filter(|(_, data)| {
                serde_json::from_slice::<AuditEvent>(data)
                    .is_ok_and(|e| e.level == AuditLevel::Error || e.kind == "finished")
            })
            .count() as u64;
        let succeeded = result.as_ref().is_some_and(|result| result.is_ok());
        let mut inner = shared.lock();
        match result {
            Some(Ok(())) => {
                inner.health.state =
                    if inner.health.dropped_critical + inner.health.dropped_routine > 0 {
                        "degraded"
                    } else {
                        "available"
                    };
                inner.health.last_io_code = None;
                if let Some((seq, _)) = batch.last() {
                    inner.health.last_written_sequence = *seq;
                }
            }
            Some(Err(e)) => {
                inner.health.state = "degraded";
                inner.health.last_io_code = e.raw_os_error();
                inner.health.dropped_critical += lost_critical;
                inner.health.dropped_routine += batch.len() as u64 - lost_critical;
            }
            None => {
                inner.health.state = "unavailable";
                inner.health.dropped_critical += lost_critical;
                inner.health.dropped_routine += batch.len() as u64 - lost_critical;
            }
        }
        if flush_ticket > inner.flush_ack {
            let first = inner.flush_ack + 1;
            inner
                .flush_results
                .push_back((first, flush_ticket, succeeded));
            while inner.flush_results.len() > 64 {
                inner.flush_results.pop_front();
            }
            inner.flush_ack = flush_ticket;
        }
        shared.wake.notify_all();
        if stop {
            break;
        }
    }
}

#[derive(Default)]
struct ScanStats {
    malformed: u64,
    unreadable: u64,
    limited: bool,
}

// Both passes share the store lease, a byte budget, and an intake cutoff.
fn scan_events(
    paths: &[PathBuf],
    run_id: &str,
    cutoff: Option<u64>,
    mut visit: impl FnMut(AuditEvent) -> Result<()>,
) -> Result<ScanStats> {
    let mut stats = ScanStats::default();
    let mut read_bytes = 0u64;
    for path in paths {
        if !regular(path).unwrap_or(false) {
            continue;
        }
        let file = match File::open(path) {
            Ok(file) => file,
            Err(_) => {
                stats.unreadable += 1;
                continue;
            }
        };
        let mut reader = BufReader::new(file);
        loop {
            let mut line = Vec::new();
            let count = match reader
                .by_ref()
                .take(EVENT_BYTES as u64 + 2)
                .read_until(b'\n', &mut line)
            {
                Ok(0) => break,
                Ok(count) => count,
                Err(_) => {
                    stats.unreadable += 1;
                    break;
                }
            };
            read_bytes += count as u64;
            if read_bytes > STORE_BYTES {
                stats.limited = true;
                return Ok(stats);
            }
            if line.last() != Some(&b'\n') || count > EVENT_BYTES + 1 {
                stats.malformed += 1;
                while line.last() != Some(&b'\n') {
                    line.clear();
                    match reader
                        .by_ref()
                        .take(EVENT_BYTES as u64 + 2)
                        .read_until(b'\n', &mut line)
                    {
                        Ok(0) => break,
                        Ok(count) => read_bytes += count as u64,
                        Err(_) => {
                            stats.unreadable += 1;
                            break;
                        }
                    }
                    if read_bytes > STORE_BYTES {
                        stats.limited = true;
                        return Ok(stats);
                    }
                }
                continue;
            }
            let Ok(event) = serde_json::from_slice::<AuditEvent>(&line) else {
                stats.malformed += 1;
                continue;
            };
            if event.schema_version != 1
                || event.run_id != run_id
                || cutoff.is_some_and(|end| event.sequence > end)
            {
                continue;
            }
            visit(event)?;
        }
    }
    Ok(stats)
}

// Keep graph metadata separately so unrelated event payloads cannot evict the selected scope.
#[derive(Default)]
struct OperationGraph {
    parents: BTreeMap<String, Option<String>>,
    incomplete: bool,
}
impl OperationGraph {
    fn add(&mut self, event: &AuditEvent) {
        let Some(id) = event
            .operation_id
            .as_deref()
            .filter(|id| crate::privacy::valid_id(id))
        else {
            return;
        };
        if self.parents.contains_key(id) {
            return;
        }
        if self.parents.len() >= 65536 {
            self.incomplete = true;
            return;
        }
        self.parents.insert(
            id.into(),
            event
                .parent_operation_id
                .clone()
                .filter(|id| crate::privacy::valid_id(id)),
        );
    }
    fn context(&self, selected: &str) -> BTreeSet<String> {
        let mut neighbors = BTreeMap::<&str, Vec<&str>>::new();
        for (id, parent) in &self.parents {
            if let Some(parent) = parent {
                neighbors.entry(id).or_default().push(parent);
                neighbors.entry(parent).or_default().push(id);
            }
        }
        let mut result = BTreeSet::new();
        let mut pending = vec![selected];
        while let Some(id) = pending.pop() {
            if result.insert(id.to_owned()) {
                if let Some(adjacent) = neighbors.get(id) {
                    pending.extend(adjacent.iter().copied());
                }
            }
        }
        result
    }
}

#[derive(Default)]
struct ExportEvents {
    records: BTreeMap<u64, AuditEvent>,
    priority: BTreeSet<(u8, u64)>,
    bytes: usize,
    omitted: u64,
    selected_found: bool,
}
impl ExportEvents {
    fn insert(&mut self, event: AuditEvent, selected: Option<&str>) -> Result<()> {
        self.selected_found |= selected.is_some() && event.operation_id.as_deref() == selected;
        if self.records.contains_key(&event.sequence) {
            return Ok(());
        }
        let rank = if selected.is_some() && event.operation_id.as_deref() == selected {
            2
        } else if event.level == AuditLevel::Error || event.kind == "finished" {
            1
        } else {
            0
        };
        self.bytes += serde_json::to_vec(&event)?.len() + 1;
        self.priority.insert((rank, event.sequence));
        self.records.insert(event.sequence, event);
        while self.bytes > EXPORT_BYTES - 128 * 1024 {
            let Some((_, sequence)) = self.priority.pop_first() else {
                break;
            };
            if let Some(event) = self.records.remove(&sequence) {
                self.bytes -= serde_json::to_vec(&event)?.len() + 1;
                self.omitted += 1;
            }
        }
        Ok(())
    }
}

impl AuditLogger {
    pub fn retained_runs(&self) -> Result<Vec<RunSummary>> {
        let mut runs = vec![RunSummary {
            run_id: self.run_id().into(),
            current: true,
        }];
        for dir in run_dirs(&self.log_dir().join("diagnostics"))? {
            let id = dir.file_name().and_then(|s| s.to_str()).unwrap_or_default();
            if id != self.run_id() {
                runs.push(RunSummary {
                    run_id: id.into(),
                    current: false,
                });
            }
        }
        Ok(runs)
    }
    pub fn export(&self, destination: &Path, scope: ExportScope) -> Result<()> {
        let _export = self
            .owner
            .shared
            .export
            .try_lock()
            .map_err(|_| AuditError::Unavailable)?;
        let run_id = scope.run_id.as_deref().unwrap_or(self.run_id());
        if !valid_run(run_id)
            || scope
                .operation_id
                .as_deref()
                .is_some_and(|s| !crate::privacy::valid_id(s))
        {
            return Err(AuditError::Unavailable);
        }
        let flushed = self.flush(Duration::from_secs(2));
        let (cutoff, mut metadata, ring, health) = {
            let inner = self.owner.shared.lock();
            (
                inner.health.last_sequence,
                inner.metadata.clone(),
                inner.ring.clone(),
                inner.health.clone(),
            )
        };
        let root = self.log_dir().join("diagnostics");
        let mut disk_unavailable = false;
        let mut metadata_missing = metadata
            .get("metadataUnavailable")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if run_id != self.run_id() {
            metadata = json!({"metadataUnavailable":true});
            metadata_missing = true;
        }
        let guard = if fs::symlink_metadata(&root).is_ok_and(|m| m.is_dir()) {
            lock_file(&root.join("store.lock")).ok()
        } else {
            None
        };
        let mut paths = Vec::new();
        if guard.is_some() {
            let dir = root.join(run_id);
            if fs::symlink_metadata(&dir).is_ok_and(|m| m.is_dir()) {
                let path = dir.join("run.json");
                if run_id != self.run_id()
                    && regular(&path).unwrap_or(false)
                    && path.metadata().is_ok_and(|m| m.len() <= 64 * 1024)
                {
                    if let Ok(data) = fs::read(&path) {
                        if let Ok(value) = serde_json::from_slice(&data) {
                            metadata = value;
                            metadata_missing = false;
                        }
                    }
                }
                match fs::read_dir(&dir) {
                    Ok(entries) => {
                        for entry in entries {
                            match entry {
                                Ok(entry) if segment(&entry.path()) => paths.push(entry.path()),
                                Ok(_) => (),
                                Err(_) => disk_unavailable = true,
                            }
                        }
                        paths.sort();
                    }
                    Err(_) => disk_unavailable = true,
                }
            } else {
                disk_unavailable = true;
            }
        } else {
            disk_unavailable = true;
        }
        let source_cutoff = (run_id == self.run_id()).then_some(cutoff);
        let ring = if run_id == self.run_id() {
            ring
        } else {
            VecDeque::new()
        };
        let mut graph = OperationGraph::default();
        let mut source_limited = false;
        let scope_ids = if let Some(id) = &scope.operation_id {
            for event in &ring {
                graph.add(event);
            }
            let stats = scan_events(&paths, run_id, source_cutoff, |event| {
                graph.add(&event);
                Ok(())
            })?;
            source_limited |= stats.limited;
            disk_unavailable |= stats.unreadable > 0;
            Some(graph.context(id))
        } else {
            None
        };
        let in_scope = |event: &AuditEvent| {
            scope_ids.as_ref().is_none_or(|ids| {
                event
                    .operation_id
                    .as_ref()
                    .is_some_and(|id| ids.contains(id))
            })
        };
        let mut selection = ExportEvents::default();
        // Ring records are authoritative for their intake identities and use the same budget.
        for event in ring {
            if event.sequence <= cutoff && in_scope(&event) {
                selection.insert(event, scope.operation_id.as_deref())?;
            }
        }
        let stats = scan_events(&paths, run_id, source_cutoff, |event| {
            if in_scope(&event) {
                selection.insert(event, scope.operation_id.as_deref())?;
            }
            Ok(())
        })?;
        source_limited |= stats.limited;
        disk_unavailable |= stats.unreadable > 0;
        let malformed = stats.malformed;
        drop(guard);
        let omitted = selection.omitted;
        let selected_found = selection.selected_found;
        let size = selection.bytes;
        let events = selection.records;
        let mut lines = Vec::with_capacity(size);
        let mut gaps = Vec::new();
        let mut previous = 0;
        let mut gap_count = 0;
        let mut unfinished = std::collections::BTreeSet::new();
        for event in events.values() {
            if event.sequence > previous + 1 {
                gap_count += 1;
                if gaps.len() < 256 {
                    gaps.push([previous + 1, event.sequence - 1]);
                }
            }
            previous = event.sequence;
            if let Some(id) = &event.operation_id {
                if event.kind == "started" {
                    unfinished.insert(id.clone());
                } else if event.kind == "finished" {
                    unfinished.remove(id);
                }
            }
            lines.extend(serde_json::to_vec(event)?);
            lines.push(b'\n');
        }
        if run_id == self.run_id() && previous < cutoff {
            gap_count += 1;
            if gaps.len() < 256 {
                gaps.push([previous + 1, cutoff]);
            }
        }
        let manifest = json!({"schemaVersion":1,"runId":run_id,"operationId":scope.operation_id,"cutoff":if run_id==self.run_id(){Some(cutoff)}else{None},"flushComplete":if run_id==self.run_id(){Some(flushed)}else{None},"health":if run_id==self.run_id(){Some(health)}else{None},"diskUnavailable":disk_unavailable,"metadataUnavailable":metadata_missing,"malformedRecords":malformed,"omittedRecords":omitted,"firstRetainedSequence":events.keys().next(),"lastRetainedSequence":events.keys().next_back(),"missingSequenceRanges":gaps,"missingRangeCount":gap_count,"incompleteOperations":unfinished.into_iter().take(256).collect::<Vec<_>>(),"historicalLogsExcluded":true,"limits":{"storeBytes":STORE_BYTES,"exportBytes":EXPORT_BYTES,"eventBytes":EVENT_BYTES,"retentionDays":7},"scopeFiltered":scope.operation_id.is_some(),"missingRangesIncludeScopeExclusions":scope.operation_id.is_some(),"selectedOperationFound":scope.operation_id.as_ref().map(|_|selected_found),"scopeContextIncomplete":graph.incomplete,"sourceReadLimited":source_limited});
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or(AuditError::Unavailable)?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
        {
            let mut archive = zip::ZipWriter::new(tmp.as_file_mut());
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, bytes) in [
                (
                    "manifest.json".into(),
                    serde_json::to_vec_pretty(&manifest)?,
                ),
                (
                    format!("runs/{run_id}.json"),
                    serde_json::to_vec_pretty(&metadata)?,
                ),
                ("events.jsonl".into(), lines),
            ] {
                archive.start_file(name, options)?;
                archive.write_all(&bytes)?;
            }
            archive.finish()?;
        }
        // A failed export never clobbers an existing destination.
        tmp.persist_noclobber(destination)
            .map_err(|e| AuditError::Io(e.error))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retention_protects_another_active_run_and_reclaims_closed_segments() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let active = root.join(format!("run-{}", Uuid::new_v4()));
        directory(&active).unwrap();
        let lease = lock_file(&active.join("active.lock")).unwrap();
        let protected = active.join("events-0001.jsonl");
        File::create(&protected)
            .unwrap()
            .set_len(STORE_BYTES)
            .unwrap();
        let own = root.join(format!("run-{}", Uuid::new_v4()));
        directory(&own).unwrap();
        let current = own.join("events-0002.jsonl");
        File::create(&current).unwrap();
        let closed = own.join("events-0001.jsonl");
        File::create(&closed).unwrap().set_len(1024).unwrap();
        assert!(reclaim(root, &own, &current, 1).is_err());
        assert!(protected.exists());
        assert!(!closed.exists());
        assert!(current.exists());
        drop(lease);
        reclaim(root, &own, &current, 1).unwrap();
        assert!(!protected.exists());
        assert!(current.exists());
    }
}

#[cfg(test)]
mod export_regressions {
    use super::*;
    #[test]
    fn selecting_old_operation_keeps_its_tree_before_budgeting_unrelated_events() {
        let dir = tempfile::tempdir().unwrap();
        let logger = AuditLogger::new(dir.path()).unwrap();
        let paths = (0..200)
            .map(|i| format!("/private/path-{i}"))
            .collect::<Vec<_>>()
            .join(if cfg!(windows) { ";" } else { ":" });
        logger.opener_environment(
            Path::new("/private"),
            logger.environment(&BTreeMap::from([("PATH".into(), paths.into())])),
        );
        assert!(logger.flush(Duration::from_secs(2)));
        let mut event = logger.owner.shared.lock().ring.back().unwrap().clone();
        let root = dir.path().join("diagnostics");
        let guard = lock_file(&root.join("store.lock")).unwrap();
        let run_id = format!("run-{}", Uuid::new_v4());
        let run = root.join(&run_id);
        directory(&run).unwrap();
        fs::write(
            run.join("run.json"),
            serde_json::to_vec(&json!({"schemaVersion":1,"runId":run_id})).unwrap(),
        )
        .unwrap();
        event.run_id = run_id.clone();
        let parent = format!("op-{}", Uuid::new_v4());
        let child = format!("op-{}", Uuid::new_v4());
        let mut file = File::create(run.join("events-0001.jsonl")).unwrap();
        let mut index = 1;
        let mut segment_bytes = 0usize;
        let mut total = 0usize;
        let mut sequence = 0;
        while total < EXPORT_BYTES + 2 * 1024 * 1024 {
            sequence += 1;
            event.sequence = sequence;
            event.operation_id = Some(if sequence <= 2 {
                parent.clone()
            } else if sequence <= 4 {
                child.clone()
            } else {
                format!("op-{}", Uuid::new_v4())
            });
            event.parent_operation_id = if (3..=4).contains(&sequence) {
                Some(parent.clone())
            } else {
                None
            };
            event.kind = if sequence == 1 || sequence == 3 {
                "started"
            } else {
                "finished"
            }
            .into();
            event.level = AuditLevel::Error;
            event.outcome = AuditOutcome::Failed;
            let mut bytes = serde_json::to_vec(&event).unwrap();
            bytes.push(b'\n');
            assert!(bytes.len() <= EVENT_BYTES);
            if segment_bytes + bytes.len() > SEGMENT_BYTES as usize {
                index += 1;
                file = File::create(run.join(format!("events-{index:04}.jsonl"))).unwrap();
                segment_bytes = 0;
            }
            file.write_all(&bytes).unwrap();
            segment_bytes += bytes.len();
            total += bytes.len();
        }
        drop(file);
        drop(guard);
        let destination = dir.path().join("selected.zip");
        logger
            .export(
                &destination,
                ExportScope {
                    run_id: Some(run_id),
                    operation_id: Some(child),
                },
            )
            .unwrap();
        let mut archive = zip::ZipArchive::new(File::open(destination).unwrap()).unwrap();
        let mut data = String::new();
        archive
            .by_name("events.jsonl")
            .unwrap()
            .read_to_string(&mut data)
            .unwrap();
        let events: Vec<AuditEvent> = data
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            events.iter().map(|e| e.sequence).collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        let mut data = String::new();
        archive
            .by_name("manifest.json")
            .unwrap()
            .read_to_string(&mut data)
            .unwrap();
        let manifest: Value = serde_json::from_str(&data).unwrap();
        assert_eq!(manifest["selectedOperationFound"], true);
        assert_eq!(manifest["scopeContextIncomplete"], false);
        assert_eq!(manifest["sourceReadLimited"], false);
    }
}
