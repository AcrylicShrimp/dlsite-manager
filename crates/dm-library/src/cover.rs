//! Fetch a cached work's cover without account credentials. The adapter chooses the destination.
use std::{path::Path, time::Duration};

const MAX_COVER_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum CoverError {
    #[error("cover request failed")]
    Request(#[from] reqwest::Error),
    #[error("unsupported cover image")]
    Unsupported,
    #[error("cover image exceeds 20 MiB")]
    TooLarge,
    #[error("could not save cover image")]
    Io(#[from] std::io::Error),
}

pub struct CoverImage {
    bytes: Vec<u8>,
    pub extension: &'static str,
}

impl CoverImage {
    pub async fn save(&self, path: &Path) -> Result<(), CoverError> {
        tokio::fs::write(path, &self.bytes).await?;
        Ok(())
    }
}

pub async fn fetch_cover(url: &str) -> Result<CoverImage, CoverError> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(Duration::from_secs(30))
        .build()?;
    let mut response = client.get(url).send().await?.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|n| n > MAX_COVER_BYTES as u64)
    {
        return Err(CoverError::TooLarge);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > MAX_COVER_BYTES {
            return Err(CoverError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    let extension = image_extension(&bytes).ok_or(CoverError::Unsupported)?;
    Ok(CoverImage { bytes, extension })
}

fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\xff\xd8\xff") {
        Some("jpg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("webp")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uses_image_bytes_not_an_untrusted_content_type() {
        assert_eq!(image_extension(b"\xff\xd8\xffrest"), Some("jpg"));
        assert_eq!(image_extension(b"\x89PNG\r\n\x1a\nrest"), Some("png"));
        assert_eq!(image_extension(b"GIF89arest"), Some("gif"));
        assert_eq!(image_extension(b"RIFFxxxxWEBPrest"), Some("webp"));
        assert_eq!(image_extension(b"<html>login required</html>"), None);
        assert_eq!(image_extension(b"RIFF"), None);
    }
}
