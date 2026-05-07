use dm_api::{
    ContentQuery, Credentials, DlsiteClient, DlsiteClientConfig, DownloadByteRange,
    DownloadResolution, WorkId,
};
use std::{env, error::Error, path::PathBuf};
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    load_dotenv();

    let username = env::var("DMSITE_API_TEST_USERNAME")?;
    let password = env::var("DMSITE_API_TEST_PASSWORD")?;
    let cases = [
        (
            "direct",
            optional_work_id("DMSITE_API_TEST_DIRECT_DOWNLOAD_WORK_ID"),
        ),
        (
            "split",
            optional_work_id("DMSITE_API_TEST_SPLIT_DOWNLOAD_WORK_ID"),
        ),
        (
            "serial",
            optional_work_id("DMSITE_API_TEST_SERIAL_REQUIRED_WORK_ID"),
        ),
    ];

    let client = DlsiteClient::new(DlsiteClientConfig::default())?;
    client.login(&Credentials::new(username, password)).await?;
    let count = client.content_count(ContentQuery::default()).await?;

    println!(
        "content/count: user={}, production={}, page_limit={:?}, concurrency={:?}",
        count.user, count.production, count.page_limit, count.concurrency
    );

    for (label, work_id) in cases {
        let Some(work_id) = work_id else {
            continue;
        };

        println!("case={label} work_id={work_id}");

        let probe = client.probe_download(&work_id).await?;
        print_raw("api/v3/download", &probe.initial);
        println!("classification={:?}", probe.resolution);

        match probe.resolution {
            DownloadResolution::Direct { stream_request } => {
                let mut stream = client
                    .open_download_stream(&stream_request, Some(DownloadByteRange::first_byte()))
                    .await?;
                println!(
                    "direct stream: status={}, content_length={:?}, headers={}",
                    stream.status(),
                    stream.content_length(),
                    format_header_summary(stream.headers())
                );
                println!(
                    "direct first_chunk={}",
                    stream.next_chunk().await?.is_some()
                );
            }
            DownloadResolution::Split { location } => {
                let raw = client.raw_get_with_body_limit(location, 200_000).await?;
                print_raw("follow-up", &raw);
                print_body_observations(raw.body_snippet.as_deref().unwrap_or_default(), &work_id);

                let page = client.split_download_page(raw.url.clone()).await?;
                println!("split parts={}", page.parts.len());
                for part in page.parts.iter().take(10) {
                    println!(
                        "  part number={} url={}",
                        part.number,
                        sanitize_url(&part.stream_request.url)
                    );
                }
            }
            DownloadResolution::SerialRequired { location } => {
                let raw = client.raw_get_with_body_limit(location, 200_000).await?;
                print_raw("follow-up", &raw);
                print_body_observations(raw.body_snippet.as_deref().unwrap_or_default(), &work_id);

                let page = client.serial_download_page(raw.url.clone()).await?;
                println!(
                    "serial download url={}",
                    sanitize_url(&page.stream_request.url)
                );
            }
            DownloadResolution::UnknownRedirect { location } => {
                let raw = client.raw_get_with_body_limit(location, 200_000).await?;
                print_raw("follow-up", &raw);
                print_body_observations(raw.body_snippet.as_deref().unwrap_or_default(), &work_id);
            }
            DownloadResolution::Unavailable { reason } => {
                println!("unavailable={reason:?}");
            }
        }
    }

    Ok(())
}

fn print_raw(label: &str, raw: &dm_api::raw::RawResponse) {
    println!(
        "{label}: status={}, url={}, location={:?}, content_type={:?}",
        raw.status,
        sanitize_url(&raw.url),
        raw.location.as_ref().map(sanitize_url),
        raw.content_type
    );
}

fn print_body_observations(body: &str, work_id: &WorkId) {
    println!(
        "body observations: chars={}, has_form={}, has_serial={}, has_split={}, has_download={}, has_product_id={}, has_number={}, has_work_id={}",
        body.chars().count(),
        body.contains("<form"),
        body.to_ascii_lowercase().contains("serial"),
        body.to_ascii_lowercase().contains("split"),
        body.contains("/download"),
        body.contains("product_id"),
        body.contains("number"),
        body.contains(work_id.as_ref())
    );

    let candidates = extract_quoted_candidates(body, work_id.as_ref());
    if !candidates.is_empty() {
        println!("quoted candidates:");
        for candidate in candidates.into_iter().take(30) {
            println!("  {}", sanitize_url_string(&candidate));
        }
    }

    print_keyword_contexts(body, work_id.as_ref());
}

fn extract_quoted_candidates(body: &str, work_id: &str) -> Vec<String> {
    let mut values = Vec::new();

    for quote in ['"', '\''] {
        let mut start = None;

        for (index, value) in body.char_indices() {
            if value != quote {
                continue;
            }

            if let Some(open) = start.take() {
                let value = &body[open..index];

                if is_candidate(value, work_id) {
                    values.push(value.to_owned());
                }
            } else {
                start = Some(index + value.len_utf8());
            }
        }
    }

    values.sort();
    values.dedup();
    values
}

fn is_candidate(value: &str, work_id: &str) -> bool {
    value.contains(work_id)
        || value.contains("/home/")
        || value.contains("/api/")
        || value.contains("download")
        || value.contains("serial")
        || value.contains("split")
        || value.contains("product_id")
}

fn print_keyword_contexts(body: &str, work_id: &str) {
    for keyword in [
        work_id, "/home/", "/api/", "download", "serial", "split", "number",
    ] {
        let mut rest = body;
        let mut offset = 0;
        let mut printed = 0;

        while let Some(index) = rest.find(keyword) {
            let absolute = offset + index;
            let start = body[..absolute]
                .char_indices()
                .rev()
                .nth(120)
                .map(|(index, _)| index)
                .unwrap_or(0);
            let end = body[absolute..]
                .char_indices()
                .nth(keyword.chars().count() + 120)
                .map(|(index, _)| absolute + index)
                .unwrap_or(body.len());
            let context = body[start..end]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");

            println!(
                "context keyword={}: {}",
                keyword.escape_debug(),
                sanitize_url_string(&context).escape_debug()
            );

            printed += 1;
            if printed >= 4 {
                break;
            }

            let advance = index + keyword.len();
            offset += advance;
            rest = &rest[advance..];
        }
    }
}

fn format_header_summary(headers: std::collections::BTreeMap<String, String>) -> String {
    [
        "content-type",
        "content-length",
        "content-range",
        "content-disposition",
    ]
    .into_iter()
    .filter_map(|key| {
        headers
            .get(key)
            .map(|value| format!("{key}={}", value.escape_debug()))
    })
    .collect::<Vec<_>>()
    .join(", ")
}

fn sanitize_url(url: &Url) -> String {
    let mut sanitized = url.clone();
    sanitized.set_query(None);
    sanitized.set_fragment(None);
    sanitized.to_string()
}

fn sanitize_url_string(value: &str) -> String {
    if let Ok(url) = Url::parse(value) {
        return sanitize_url(&url);
    }

    value.split('?').next().unwrap_or(value).to_owned()
}

fn optional_work_id(key: &str) -> Option<WorkId> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .map(WorkId::from)
}

fn load_dotenv() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dotenvy::from_path(manifest_dir.join(".env")).ok();
}
