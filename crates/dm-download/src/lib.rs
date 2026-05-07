use dm_api::{
    DlsiteClient, DownloadByteRange, DownloadFileKind, DownloadStream, DownloadStreamRequest,
    WorkId,
};
use std::{
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
};
use url::Url;

pub const DEFAULT_MAX_RETRIES: u32 = 3;

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
    #[error("download ended before expected size; expected {expected} bytes, got {actual}")]
    IncompleteDownload { expected: u64, actual: u64 },
    #[error("download exceeded expected size; expected {expected} bytes, got {actual}")]
    SizeExceeded { expected: u64, actual: u64 },
    #[error("download stream error: {0}")]
    Stream(String),
    #[error("dlsite api error")]
    Api(#[from] dm_api::DmApiError),
    #[error("I/O error")]
    Io(#[from] std::io::Error),
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

        let mut stream = match source.open_range(bytes_written).await {
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

            let chunk = match stream.next_chunk().await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
        time::{SystemTime, UNIX_EPOCH},
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
}
