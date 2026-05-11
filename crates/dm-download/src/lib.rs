use dm_api::{
    DlsiteClient, DownloadByteRange, DownloadFile, DownloadFileKind, DownloadPlan, DownloadStream,
    DownloadStreamRequest, WorkId,
};
use dm_archive::{ArchiveExtractOptions, ArchiveExtraction, ArchivePlan};
use std::{
    collections::BTreeMap,
    future::Future,
    path::{Component, Path, PathBuf},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
    time::{sleep, Duration},
};
use url::Url;

pub const DEFAULT_MAX_RETRIES: u32 = 3;
const CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadJobRequest {
    pub work_id: WorkId,
    pub target_root: PathBuf,
    pub unpack_policy: UnpackPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnpackPolicy {
    KeepArchives,
    UnpackWhenRecognized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadProgress {
    pub phase: DownloadPhase,
    pub file_index: Option<usize>,
    pub file_kind: Option<DownloadFileKind>,
    pub bytes_received: u64,
    pub bytes_total: Option<u64>,
}

impl DownloadProgress {
    pub fn percentage(&self) -> Option<u8> {
        let total = self.bytes_total?;

        if total == 0 {
            return None;
        }

        let percentage = self.bytes_received.saturating_mul(100) / total;
        Some(u8::try_from(percentage.min(100)).expect("percentage is capped at 100"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadFileRequest {
    pub file_index: usize,
    pub file_kind: DownloadFileKind,
    pub target_dir: PathBuf,
    pub file_name: String,
    pub expected_size: Option<u64>,
    pub max_retries: u32,
}

impl DownloadFileRequest {
    pub fn new(
        file_index: usize,
        file_kind: DownloadFileKind,
        target_dir: impl Into<PathBuf>,
        file_name: impl Into<String>,
    ) -> Self {
        Self {
            file_index,
            file_kind,
            target_dir: target_dir.into(),
            file_name: file_name.into(),
            expected_size: None,
            max_retries: DEFAULT_MAX_RETRIES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedFile {
    pub file_name: String,
    pub path: PathBuf,
    pub bytes_written: u64,
    pub resumed_from: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadFileMetadata {
    pub file_index: usize,
    pub file_kind: DownloadFileKind,
    pub file_name: String,
    pub expected_size: Option<u64>,
    pub final_url: Url,
}

impl DownloadFileMetadata {
    pub fn to_file_request(&self, target_dir: impl Into<PathBuf>) -> DownloadFileRequest {
        let mut request = DownloadFileRequest::new(
            self.file_index,
            self.file_kind.clone(),
            target_dir,
            self.file_name.clone(),
        );
        request.expected_size = self.expected_size;
        request
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedWork {
    pub work_id: WorkId,
    pub target_dir: PathBuf,
    pub files: Vec<DownloadedFile>,
    pub archive_extraction: Option<ArchiveExtraction>,
}

pub type DownloadOpenFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<Box<dyn DownloadByteStream + Send + 'a>, DownloadError>>
            + Send
            + 'a,
    >,
>;

pub type DownloadChunkFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>, DownloadError>> + Send + 'a>>;

pub trait RangedDownloadSource {
    fn open_range<'a>(&'a mut self, start: u64) -> DownloadOpenFuture<'a>;
}

pub trait DownloadByteStream {
    fn next_chunk<'a>(&'a mut self) -> DownloadChunkFuture<'a>;
}

#[derive(Clone)]
pub struct DlsiteDownloadSource {
    client: DlsiteClient,
    stream_request: DownloadStreamRequest,
}

impl DlsiteDownloadSource {
    pub fn new(client: DlsiteClient, stream_request: DownloadStreamRequest) -> Self {
        Self {
            client,
            stream_request,
        }
    }
}

impl RangedDownloadSource for DlsiteDownloadSource {
    fn open_range<'a>(&'a mut self, start: u64) -> DownloadOpenFuture<'a> {
        Box::pin(async move {
            let stream = self
                .client
                .open_download_stream(
                    &self.stream_request,
                    Some(DownloadByteRange::from_start(start)),
                )
                .await?;
            let stream: Box<dyn DownloadByteStream + Send + 'a> =
                Box::new(DlsiteDownloadByteStream { stream });

            Ok(stream)
        })
    }
}

struct DlsiteDownloadByteStream {
    stream: DownloadStream,
}

impl DownloadByteStream for DlsiteDownloadByteStream {
    fn next_chunk<'a>(&'a mut self) -> DownloadChunkFuture<'a> {
        Box::pin(async move {
            self.stream
                .next_chunk()
                .await
                .map(|chunk| chunk.map(|chunk| chunk.to_vec()))
                .map_err(|err| DownloadError::Stream(err.to_string()))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadPhase {
    ResolvingPlan,
    ProbingMetadata,
    Downloading,
    Finalizing,
    Unpacking,
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub async fn cancelled(&self) {
        while !self.is_cancelled() {
            sleep(CANCELLATION_POLL_INTERVAL).await;
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("download was cancelled")]
    Cancelled,
    #[error("download file name is not known yet for file index {file_index}")]
    FileNameUnknown { file_index: usize },
    #[error("invalid download file name: {file_name}")]
    InvalidFileName { file_name: String },
    #[error("download target already exists: {path}")]
    TargetAlreadyExists { path: PathBuf },
    #[error("download job work id {job_work_id} does not match plan work id {plan_work_id}")]
    PlanWorkMismatch {
        job_work_id: WorkId,
        plan_work_id: WorkId,
    },
    #[error("download ended before expected size; expected {expected} bytes, got {actual}")]
    IncompleteDownload { expected: u64, actual: u64 },
    #[error("download exceeded expected size; expected {expected} bytes, got {actual}")]
    SizeExceeded { expected: u64, actual: u64 },
    #[error("download stream error: {0}")]
    Stream(String),
    #[error("dlsite api error")]
    Api(#[from] dm_api::DmApiError),
    #[error("archive error")]
    Archive(#[from] dm_archive::ArchiveError),
    #[error("I/O error")]
    Io(#[from] std::io::Error),
}

pub async fn download_work_files<F>(
    client: DlsiteClient,
    job: &DownloadJobRequest,
    plan: &DownloadPlan,
    cancellation: &CancellationToken,
    mut on_progress: F,
) -> Result<DownloadedWork, DownloadError>
where
    F: FnMut(DownloadProgress),
{
    if job.work_id.as_ref() != plan.work_id.as_ref() {
        return Err(DownloadError::PlanWorkMismatch {
            job_work_id: job.work_id.clone(),
            plan_work_id: plan.work_id.clone(),
        });
    }

    let target_dir = job.target_root.join(job.work_id.as_ref());
    let mut file_metadata = Vec::with_capacity(plan.files.len());
    let mut downloaded_files = Vec::with_capacity(plan.files.len());

    for (file_index, file) in plan.files.iter().enumerate() {
        if cancellation.is_cancelled() {
            return Err(DownloadError::Cancelled);
        }

        on_progress(DownloadProgress {
            phase: DownloadPhase::ProbingMetadata,
            file_index: Some(file_index),
            file_kind: Some(file.kind.clone()),
            bytes_received: 0,
            bytes_total: None,
        });

        let metadata = cancellable(
            cancellation,
            probe_download_file_metadata(&client, file_index, file),
        )
        .await?;
        file_metadata.push((metadata, file.stream_request.clone()));
    }

    let aggregate_bytes_total = total_expected_size(
        file_metadata
            .iter()
            .map(|(metadata, _stream_request)| metadata),
    );
    let mut completed_bytes = 0;

    for (metadata, stream_request) in file_metadata {
        if cancellation.is_cancelled() {
            return Err(DownloadError::Cancelled);
        }

        let request = metadata.to_file_request(target_dir.clone());
        let mut source = DlsiteDownloadSource::new(client.clone(), stream_request);
        let file_offset = completed_bytes;
        let mut aggregate_progress = |progress| {
            on_progress(aggregate_file_progress(
                progress,
                file_offset,
                aggregate_bytes_total,
            ));
        };
        let downloaded =
            download_file(&mut source, &request, cancellation, &mut aggregate_progress).await?;

        completed_bytes = completed_bytes.saturating_add(downloaded.bytes_written);
        on_progress(DownloadProgress {
            phase: DownloadPhase::Downloading,
            file_index: Some(metadata.file_index),
            file_kind: Some(metadata.file_kind.clone()),
            bytes_received: completed_bytes,
            bytes_total: aggregate_bytes_total,
        });

        downloaded_files.push(downloaded);
    }

    let archive_plan = plan_downloaded_archive(&downloaded_files);
    if job.unpack_policy == UnpackPolicy::UnpackWhenRecognized
        && matches!(
            archive_plan,
            ArchivePlan::SingleZip { .. } | ArchivePlan::LegacySplitRar { .. }
        )
    {
        on_progress(DownloadProgress {
            phase: DownloadPhase::Unpacking,
            file_index: None,
            file_kind: None,
            bytes_received: completed_bytes,
            bytes_total: aggregate_bytes_total,
        });
    }

    if cancellation.is_cancelled() {
        return Err(DownloadError::Cancelled);
    }

    let archive_extraction = unpack_downloaded_archive_plan(
        archive_plan,
        &target_dir,
        job.unpack_policy,
        ArchiveExtractOptions::default(),
    )?;

    if cancellation.is_cancelled() {
        return Err(DownloadError::Cancelled);
    }

    Ok(DownloadedWork {
        work_id: job.work_id.clone(),
        target_dir,
        files: downloaded_files,
        archive_extraction,
    })
}

pub fn unpack_downloaded_files(
    files: &[DownloadedFile],
    target_dir: impl AsRef<Path>,
    unpack_policy: UnpackPolicy,
    options: ArchiveExtractOptions,
) -> Result<Option<ArchiveExtraction>, DownloadError> {
    let archive_plan = plan_downloaded_archive(files);
    unpack_downloaded_archive_plan(archive_plan, target_dir, unpack_policy, options)
}

fn plan_downloaded_archive(files: &[DownloadedFile]) -> ArchivePlan {
    dm_archive::plan_archive_handling(
        files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>(),
    )
}

fn unpack_downloaded_archive_plan(
    archive_plan: ArchivePlan,
    target_dir: impl AsRef<Path>,
    unpack_policy: UnpackPolicy,
    options: ArchiveExtractOptions,
) -> Result<Option<ArchiveExtraction>, DownloadError> {
    if unpack_policy == UnpackPolicy::KeepArchives {
        return Ok(None);
    }

    match archive_plan {
        ArchivePlan::SingleZip { .. } | ArchivePlan::LegacySplitRar { .. } => {
            dm_archive::extract_archive_plan(&archive_plan, target_dir, options)
                .map(Some)
                .map_err(Into::into)
        }
        ArchivePlan::KeepArchives { .. } => Ok(None),
    }
}

fn total_expected_size<'a>(
    metadata: impl IntoIterator<Item = &'a DownloadFileMetadata>,
) -> Option<u64> {
    metadata.into_iter().try_fold(0u64, |total, metadata| {
        Some(total.saturating_add(metadata.expected_size?))
    })
}

fn aggregate_file_progress(
    mut progress: DownloadProgress,
    completed_before_file: u64,
    aggregate_bytes_total: Option<u64>,
) -> DownloadProgress {
    if progress.phase == DownloadPhase::Downloading {
        progress.bytes_received = completed_before_file.saturating_add(progress.bytes_received);
        progress.bytes_total = aggregate_bytes_total;
    }

    progress
}

pub async fn probe_download_file_metadata(
    client: &DlsiteClient,
    file_index: usize,
    file: &DownloadFile,
) -> Result<DownloadFileMetadata, DownloadError> {
    let stream = client
        .open_download_stream(&file.stream_request, Some(DownloadByteRange::first_byte()))
        .await?;
    let headers = stream.headers();
    let final_url = stream.url().clone();
    let file_name = file_name_from_download_url(&final_url)
        .ok_or(DownloadError::FileNameUnknown { file_index })?;
    let expected_size =
        header_value(&headers, "content-range").and_then(total_size_from_content_range);

    Ok(DownloadFileMetadata {
        file_index,
        file_kind: file.kind.clone(),
        file_name,
        expected_size,
        final_url,
    })
}

pub async fn download_file<S, F>(
    source: &mut S,
    request: &DownloadFileRequest,
    cancellation: &CancellationToken,
    mut on_progress: F,
) -> Result<DownloadedFile, DownloadError>
where
    S: RangedDownloadSource + Send,
    F: FnMut(DownloadProgress),
{
    validate_file_name(&request.file_name)?;

    let target_path = request.target_dir.join(&request.file_name);
    let staging_dir = staging_dir_for(&request.target_dir);
    let staging_path = staging_dir.join(&request.file_name);

    if target_path.try_exists()? {
        let existing_size = fs::metadata(&target_path).await?.len();

        if request
            .expected_size
            .is_some_and(|expected_size| existing_size == expected_size)
        {
            return Ok(DownloadedFile {
                file_name: request.file_name.clone(),
                path: target_path,
                bytes_written: existing_size,
                resumed_from: existing_size,
            });
        }

        return Err(DownloadError::TargetAlreadyExists { path: target_path });
    }

    fs::create_dir_all(&staging_dir).await?;

    let mut resumed_from = file_size_if_exists(&staging_path).await?;

    if let Some(expected_size) = request.expected_size {
        if resumed_from > expected_size {
            fs::remove_file(&staging_path).await?;
            resumed_from = 0;
        }
    }

    let mut bytes_written = resumed_from;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&staging_path)
        .await?;
    let mut retries = 0;

    'download: loop {
        if cancellation.is_cancelled() {
            file.flush().await.ok();
            return Err(DownloadError::Cancelled);
        }

        if request
            .expected_size
            .is_some_and(|expected_size| bytes_written == expected_size)
        {
            break;
        }

        let mut stream = match cancellable(cancellation, source.open_range(bytes_written)).await {
            Ok(stream) => stream,
            Err(_err) if retries < request.max_retries => {
                retries += 1;
                continue;
            }
            Err(err) => return Err(err),
        };

        loop {
            if cancellation.is_cancelled() {
                file.flush().await.ok();
                return Err(DownloadError::Cancelled);
            }

            let chunk = match cancellable(cancellation, stream.next_chunk()).await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(_err) if retries < request.max_retries => {
                    retries += 1;
                    continue 'download;
                }
                Err(err) => return Err(err),
            };

            if chunk.is_empty() {
                continue;
            }

            let next_bytes_written = bytes_written + chunk.len() as u64;

            if let Some(expected) = request.expected_size {
                if next_bytes_written > expected {
                    return Err(DownloadError::SizeExceeded {
                        expected,
                        actual: next_bytes_written,
                    });
                }
            }

            file.write_all(&chunk).await?;
            bytes_written = next_bytes_written;

            on_progress(DownloadProgress {
                phase: DownloadPhase::Downloading,
                file_index: Some(request.file_index),
                file_kind: Some(request.file_kind.clone()),
                bytes_received: bytes_written,
                bytes_total: request.expected_size,
            });
        }

        if let Some(expected) = request.expected_size {
            if bytes_written < expected {
                if retries < request.max_retries {
                    retries += 1;
                    continue;
                }

                return Err(DownloadError::IncompleteDownload {
                    expected,
                    actual: bytes_written,
                });
            }
        }

        break;
    }

    file.flush().await?;
    drop(file);

    if cancellation.is_cancelled() {
        return Err(DownloadError::Cancelled);
    }

    if target_path.try_exists()? {
        return Err(DownloadError::TargetAlreadyExists { path: target_path });
    }

    fs::rename(&staging_path, &target_path).await?;
    fs::remove_dir(&staging_dir).await.ok();

    Ok(DownloadedFile {
        file_name: request.file_name.clone(),
        path: target_path,
        bytes_written,
        resumed_from,
    })
}

async fn cancellable<T, F>(cancellation: &CancellationToken, future: F) -> Result<T, DownloadError>
where
    F: Future<Output = Result<T, DownloadError>>,
{
    if cancellation.is_cancelled() {
        return Err(DownloadError::Cancelled);
    }

    tokio::select! {
        _ = cancellation.cancelled() => Err(DownloadError::Cancelled),
        result = future => result,
    }
}

pub fn staging_dir_for(target_dir: &Path) -> PathBuf {
    target_dir.join(".dm-download")
}

fn validate_file_name(file_name: &str) -> Result<(), DownloadError> {
    let path = Path::new(file_name);

    if file_name.is_empty()
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains('\0')
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        return Err(DownloadError::InvalidFileName {
            file_name: file_name.to_owned(),
        });
    }

    Ok(())
}

async fn file_size_if_exists(path: &Path) -> Result<u64, DownloadError> {
    match fs::metadata(path).await {
        Ok(metadata) => Ok(metadata.len()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(err) => Err(err.into()),
    }
}

pub fn file_name_from_download_url(url: &Url) -> Option<String> {
    let mut segments = url.path_segments()?;

    while let Some(segment) = segments.next() {
        if segment != "file" {
            continue;
        }

        let file_name = segments.next()?;

        if file_name.is_empty() {
            return None;
        }

        return Some(file_name.to_owned());
    }

    None
}

pub fn total_size_from_content_range(content_range: &str) -> Option<u64> {
    let (_, size) = content_range.rsplit_once('/')?;

    if size == "*" {
        return None;
    }

    size.parse().ok()
}

fn header_value<'a>(headers: &'a BTreeMap<String, String>, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::VecDeque,
        io::Write,
        sync::{Arc, Mutex},
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn reports_progress_percentage_when_total_is_known() {
        let progress = DownloadProgress {
            phase: DownloadPhase::Downloading,
            file_index: Some(0),
            file_kind: Some(DownloadFileKind::Direct),
            bytes_received: 25,
            bytes_total: Some(100),
        };

        assert_eq!(progress.percentage(), Some(25));
    }

    #[test]
    fn caps_progress_percentage_at_one_hundred() {
        let progress = DownloadProgress {
            phase: DownloadPhase::Downloading,
            file_index: Some(0),
            file_kind: Some(DownloadFileKind::Direct),
            bytes_received: 150,
            bytes_total: Some(100),
        };

        assert_eq!(progress.percentage(), Some(100));
    }

    #[test]
    fn omits_progress_percentage_when_total_is_unknown_or_zero() {
        let mut progress = DownloadProgress {
            phase: DownloadPhase::Downloading,
            file_index: Some(0),
            file_kind: Some(DownloadFileKind::Direct),
            bytes_received: 25,
            bytes_total: None,
        };

        assert_eq!(progress.percentage(), None);

        progress.bytes_total = Some(0);
        assert_eq!(progress.percentage(), None);
    }

    #[test]
    fn aggregates_known_multi_file_progress() {
        let metadata = vec![
            download_metadata(0, DownloadFileKind::SplitPart { number: 1 }, Some(100)),
            download_metadata(1, DownloadFileKind::SplitPart { number: 2 }, Some(250)),
        ];

        assert_eq!(total_expected_size(metadata.iter()), Some(350));

        let progress = aggregate_file_progress(
            DownloadProgress {
                phase: DownloadPhase::Downloading,
                file_index: Some(1),
                file_kind: Some(DownloadFileKind::SplitPart { number: 2 }),
                bytes_received: 75,
                bytes_total: Some(250),
            },
            100,
            total_expected_size(metadata.iter()),
        );

        assert_eq!(progress.bytes_received, 175);
        assert_eq!(progress.bytes_total, Some(350));
        assert_eq!(progress.percentage(), Some(50));
    }

    #[test]
    fn omits_aggregate_total_when_any_file_size_is_unknown() {
        let metadata = vec![
            download_metadata(0, DownloadFileKind::SplitPart { number: 1 }, Some(100)),
            download_metadata(1, DownloadFileKind::SplitPart { number: 2 }, None),
        ];

        assert_eq!(total_expected_size(metadata.iter()), None);
    }

    #[test]
    fn aggregate_progress_keeps_non_download_phases_unchanged() {
        let progress = DownloadProgress {
            phase: DownloadPhase::Unpacking,
            file_index: None,
            file_kind: None,
            bytes_received: 100,
            bytes_total: Some(100),
        };

        assert_eq!(
            aggregate_file_progress(progress.clone(), 300, Some(500)),
            progress
        );
    }

    #[test]
    fn cancellation_token_can_be_shared() {
        let token = CancellationToken::new();
        let cloned = token.clone();

        assert!(!token.is_cancelled());
        cloned.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn extracts_file_name_from_dlsite_download_url() {
        let url = Url::parse(
            "https://download.dlsite.com/get/=/type/work/domain/doujin/dir/RJ322000/file/RJ321841.part1.exe/_/20210426172436",
        )
        .unwrap();

        assert_eq!(
            file_name_from_download_url(&url),
            Some("RJ321841.part1.exe".to_owned())
        );
    }

    #[test]
    fn does_not_extract_file_name_from_unrecognized_url() {
        let url = Url::parse("https://example.com/download/RJ321841.zip").unwrap();

        assert_eq!(file_name_from_download_url(&url), None);
    }

    #[test]
    fn extracts_total_size_from_content_range() {
        assert_eq!(
            total_size_from_content_range("bytes 0-0/204264274"),
            Some(204264274)
        );
        assert_eq!(
            total_size_from_content_range("bytes */204264274"),
            Some(204264274)
        );
        assert_eq!(total_size_from_content_range("bytes 0-0/*"), None);
        assert_eq!(total_size_from_content_range("invalid"), None);
    }

    #[test]
    fn converts_metadata_to_file_request() {
        let metadata = DownloadFileMetadata {
            file_index: 2,
            file_kind: DownloadFileKind::SplitPart { number: 3 },
            file_name: "RJ123456.part3.rar".to_owned(),
            expected_size: Some(42),
            final_url: Url::parse("https://download.dlsite.com/get/=/file/RJ123456.part3.rar/_/1")
                .unwrap(),
        };

        let request = metadata.to_file_request("/tmp/work");

        assert_eq!(request.file_index, 2);
        assert_eq!(request.file_kind, DownloadFileKind::SplitPart { number: 3 });
        assert_eq!(request.target_dir, PathBuf::from("/tmp/work"));
        assert_eq!(request.file_name, "RJ123456.part3.rar");
        assert_eq!(request.expected_size, Some(42));
    }

    #[test]
    fn finds_headers_case_insensitively() {
        let headers = BTreeMap::from([("Content-Range".to_owned(), "bytes 0-0/42".to_owned())]);

        assert_eq!(
            header_value(&headers, "content-range"),
            Some("bytes 0-0/42")
        );
    }

    fn download_metadata(
        file_index: usize,
        file_kind: DownloadFileKind,
        expected_size: Option<u64>,
    ) -> DownloadFileMetadata {
        DownloadFileMetadata {
            file_index,
            file_kind,
            file_name: format!("part{file_index}.bin"),
            expected_size,
            final_url: Url::parse(&format!(
                "https://download.example.test/file/part{file_index}.bin/_/1"
            ))
            .expect("url"),
        }
    }

    #[tokio::test]
    async fn downloads_file_through_staging_path() {
        let dir = test_dir("fresh");
        let mut source = ScriptedSource::new(vec![Ok(vec![b"abc".to_vec(), b"def".to_vec()])]);
        let request = request(&dir, "RJ123456.zip", Some(6));
        let mut progress = Vec::new();

        let downloaded = download_file(&mut source, &request, &CancellationToken::new(), |event| {
            progress.push(event.bytes_received)
        })
        .await
        .unwrap();

        assert_eq!(downloaded.bytes_written, 6);
        assert_eq!(downloaded.resumed_from, 0);
        assert_eq!(std::fs::read(dir.join("RJ123456.zip")).unwrap(), b"abcdef");
        assert!(!staging_dir_for(&dir).exists());
        assert_eq!(source.starts(), vec![0]);
        assert_eq!(progress, vec![3, 6]);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn resumes_from_existing_staging_file() {
        let dir = test_dir("resume");
        let staging_dir = staging_dir_for(&dir);
        std::fs::create_dir_all(&staging_dir).unwrap();
        std::fs::write(staging_dir.join("RJ123456.zip"), b"abc").unwrap();

        let mut source = ScriptedSource::new(vec![Ok(vec![b"def".to_vec()])]);
        let request = request(&dir, "RJ123456.zip", Some(6));

        let downloaded = download_file(&mut source, &request, &CancellationToken::new(), |_| {})
            .await
            .unwrap();

        assert_eq!(downloaded.bytes_written, 6);
        assert_eq!(downloaded.resumed_from, 3);
        assert_eq!(std::fs::read(dir.join("RJ123456.zip")).unwrap(), b"abcdef");
        assert_eq!(source.starts(), vec![3]);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn reuses_existing_finalized_file_when_size_matches() {
        let dir = test_dir("existing-final");
        std::fs::write(dir.join("RJ123456.zip"), b"abcdef").unwrap();
        let mut source = ScriptedSource::new(vec![]);
        let request = request(&dir, "RJ123456.zip", Some(6));

        let downloaded = download_file(&mut source, &request, &CancellationToken::new(), |_| {})
            .await
            .unwrap();

        assert_eq!(downloaded.bytes_written, 6);
        assert_eq!(downloaded.resumed_from, 6);
        assert_eq!(source.starts(), Vec::<u64>::new());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn retries_after_stream_failure_from_current_offset() {
        let dir = test_dir("retry");
        let mut source =
            ScriptedSource::new(vec![Ok(vec![b"abc".to_vec()]), Ok(vec![b"def".to_vec()])])
                .with_fail_after_chunks(0, 1);
        let mut request = request(&dir, "RJ123456.zip", Some(6));
        request.max_retries = 1;

        let downloaded = download_file(&mut source, &request, &CancellationToken::new(), |_| {})
            .await
            .unwrap();

        assert_eq!(downloaded.bytes_written, 6);
        assert_eq!(std::fs::read(dir.join("RJ123456.zip")).unwrap(), b"abcdef");
        assert_eq!(source.starts(), vec![0, 3]);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn rejects_path_traversal_file_names() {
        let dir = test_dir("invalid-file-name");
        let mut source = ScriptedSource::new(vec![]);
        let request = request(&dir, "../RJ123456.zip", Some(0));

        let err = download_file(&mut source, &request, &CancellationToken::new(), |_| {})
            .await
            .unwrap_err();

        assert!(matches!(err, DownloadError::InvalidFileName { .. }));

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn cancellation_leaves_staging_file_for_resume() {
        let dir = test_dir("cancel");
        let mut source = ScriptedSource::new(vec![Ok(vec![b"abc".to_vec(), b"def".to_vec()])]);
        let request = request(&dir, "RJ123456.zip", Some(6));
        let cancellation = CancellationToken::new();
        let cancellation_on_progress = cancellation.clone();

        let err = download_file(&mut source, &request, &cancellation, |_| {
            cancellation_on_progress.cancel();
        })
        .await
        .unwrap_err();

        assert!(matches!(err, DownloadError::Cancelled));
        assert_eq!(
            std::fs::read(staging_dir_for(&dir).join("RJ123456.zip")).unwrap(),
            b"abc"
        );
        assert!(!dir.join("RJ123456.zip").exists());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn cancellation_interrupts_pending_stream_read() {
        let dir = test_dir("cancel-pending-stream");
        let mut source = PendingStreamSource;
        let request = request(&dir, "RJ123456.zip", Some(6));
        let cancellation = CancellationToken::new();
        let cancellation_trigger = cancellation.clone();

        let download = download_file(&mut source, &request, &cancellation, |_| {});
        let cancel = async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            cancellation_trigger.cancel();
        };
        let (result, ()) = tokio::join!(download, cancel);
        let err = result.unwrap_err();

        assert!(matches!(err, DownloadError::Cancelled));
        assert!(staging_dir_for(&dir).join("RJ123456.zip").exists());
        assert!(!dir.join("RJ123456.zip").exists());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unpacks_single_zip_when_policy_requests_it() {
        let dir = test_dir("unpack-zip");
        let archive = dir.join("RJ123456.zip");
        write_zip(&archive, &[("RJ123456/readme.txt", b"hello".as_slice())]);
        let downloaded = DownloadedFile {
            file_name: "RJ123456.zip".to_owned(),
            path: archive.clone(),
            bytes_written: std::fs::metadata(&archive).unwrap().len(),
            resumed_from: 0,
        };

        let extraction = unpack_downloaded_files(
            &[downloaded],
            &dir,
            UnpackPolicy::UnpackWhenRecognized,
            ArchiveExtractOptions::default(),
        )
        .unwrap()
        .unwrap();

        assert_eq!(std::fs::read(dir.join("readme.txt")).unwrap(), b"hello");
        assert_eq!(extraction.removed_sources, vec![archive.clone()]);
        assert!(!archive.exists());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn propagates_legacy_split_rar_errors_and_preserves_sources() {
        let dir = test_dir("invalid-split-rar");
        let first_part = dir.join("RJ123456.part1.exe");
        let second_part = dir.join("RJ123456.part2.rar");
        std::fs::write(&first_part, b"part1").unwrap();
        std::fs::write(&second_part, b"part2").unwrap();
        let files = [
            DownloadedFile {
                file_name: "RJ123456.part1.exe".to_owned(),
                path: first_part.clone(),
                bytes_written: 5,
                resumed_from: 0,
            },
            DownloadedFile {
                file_name: "RJ123456.part2.rar".to_owned(),
                path: second_part.clone(),
                bytes_written: 5,
                resumed_from: 0,
            },
        ];

        let err = unpack_downloaded_files(
            &files,
            &dir,
            UnpackPolicy::UnpackWhenRecognized,
            ArchiveExtractOptions::default(),
        )
        .unwrap_err();

        assert!(matches!(err, DownloadError::Archive(_)));
        assert!(first_part.exists());
        assert!(second_part.exists());

        std::fs::remove_dir_all(dir).unwrap();
    }

    fn request(dir: &Path, file_name: &str, expected_size: Option<u64>) -> DownloadFileRequest {
        let mut request =
            DownloadFileRequest::new(0, DownloadFileKind::Direct, dir, file_name.to_owned());
        request.expected_size = expected_size;
        request
    }

    fn test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "dm-download-{name}-{}-{unique}",
            std::process::id()
        ));

        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (name, content) in entries {
            zip.start_file(name, options).unwrap();
            zip.write_all(content).unwrap();
        }

        zip.finish().unwrap();
    }

    struct ScriptedSource {
        openings: VecDeque<Result<Vec<Vec<u8>>, DownloadError>>,
        starts: Arc<Mutex<Vec<u64>>>,
        fail_after: Option<(usize, usize)>,
        open_count: usize,
    }

    impl ScriptedSource {
        fn new(openings: Vec<Result<Vec<Vec<u8>>, DownloadError>>) -> Self {
            Self {
                openings: VecDeque::from(openings),
                starts: Arc::new(Mutex::new(Vec::new())),
                fail_after: None,
                open_count: 0,
            }
        }

        fn with_fail_after_chunks(mut self, open_index: usize, chunks: usize) -> Self {
            self.fail_after = Some((open_index, chunks));
            self
        }

        fn starts(&self) -> Vec<u64> {
            self.starts.lock().unwrap().clone()
        }
    }

    impl RangedDownloadSource for ScriptedSource {
        fn open_range<'a>(&'a mut self, start: u64) -> DownloadOpenFuture<'a> {
            Box::pin(async move {
                self.starts.lock().unwrap().push(start);
                let open_index = self.open_count;
                self.open_count += 1;
                let chunks = self
                    .openings
                    .pop_front()
                    .unwrap_or_else(|| Err(DownloadError::Stream("unexpected open".to_owned())))?;
                let fail_after = self
                    .fail_after
                    .and_then(|(index, chunks)| (index == open_index).then_some(chunks));

                let stream: Box<dyn DownloadByteStream + Send + 'a> = Box::new(ScriptedStream {
                    chunks: VecDeque::from(chunks),
                    fail_after,
                    emitted_chunks: 0,
                });

                Ok(stream)
            })
        }
    }

    struct ScriptedStream {
        chunks: VecDeque<Vec<u8>>,
        fail_after: Option<usize>,
        emitted_chunks: usize,
    }

    impl DownloadByteStream for ScriptedStream {
        fn next_chunk<'a>(&'a mut self) -> DownloadChunkFuture<'a> {
            Box::pin(async move {
                if self.fail_after == Some(self.emitted_chunks) {
                    self.fail_after = None;
                    return Err(DownloadError::Stream("scripted failure".to_owned()));
                }

                let chunk = self.chunks.pop_front();

                if chunk.is_some() {
                    self.emitted_chunks += 1;
                }

                Ok(chunk)
            })
        }
    }

    struct PendingStreamSource;

    impl RangedDownloadSource for PendingStreamSource {
        fn open_range<'a>(&'a mut self, _start: u64) -> DownloadOpenFuture<'a> {
            Box::pin(async move {
                let stream: Box<dyn DownloadByteStream + Send + 'a> = Box::new(PendingStream);
                Ok(stream)
            })
        }
    }

    struct PendingStream;

    impl DownloadByteStream for PendingStream {
        fn next_chunk<'a>(&'a mut self) -> DownloadChunkFuture<'a> {
            Box::pin(async move {
                std::future::pending::<Result<Option<Vec<u8>>, DownloadError>>().await
            })
        }
    }
}
