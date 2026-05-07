use dm_api::{DownloadFileKind, WorkId};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use url::Url;

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
    #[error("dlsite api error")]
    Api(#[from] dm_api::DmApiError),
    #[error("I/O error")]
    Io(#[from] std::io::Error),
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
}
