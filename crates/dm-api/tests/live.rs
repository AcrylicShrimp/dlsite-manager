use dm_api::{
    ContentQuery, Credentials, DlsiteClient, DlsiteClientConfig, DmApiError, DownloadByteRange,
    DownloadFileKind, DownloadPlan, DownloadResolution, WorkId,
};
use std::{collections::BTreeSet, env, error::Error, path::PathBuf};

type TestResult = Result<(), Box<dyn Error>>;

struct LiveEnv {
    username: String,
    password: String,
    owned_work_id: Option<WorkId>,
    direct_download_work_id: Option<WorkId>,
    split_download_work_id: Option<WorkId>,
    serial_required_work_id: Option<WorkId>,
    batch_work_ids: Vec<WorkId>,
}

impl LiveEnv {
    fn load() -> Option<Self> {
        load_dotenv();

        if env::var("DMSITE_API_TEST_LIVE").ok().as_deref() != Some("1") {
            return None;
        }

        Some(Self {
            username: env::var("DMSITE_API_TEST_USERNAME").ok()?,
            password: env::var("DMSITE_API_TEST_PASSWORD").ok()?,
            owned_work_id: optional_work_id("DMSITE_API_TEST_OWNED_WORK_ID"),
            direct_download_work_id: optional_work_id("DMSITE_API_TEST_DIRECT_DOWNLOAD_WORK_ID"),
            split_download_work_id: optional_work_id("DMSITE_API_TEST_SPLIT_DOWNLOAD_WORK_ID"),
            serial_required_work_id: optional_work_id("DMSITE_API_TEST_SERIAL_REQUIRED_WORK_ID"),
            batch_work_ids: env::var("DMSITE_API_TEST_BATCH_WORK_IDS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(WorkId::from)
                .collect(),
        })
    }

    fn credentials(&self) -> Credentials {
        Credentials::new(self.username.clone(), self.password.clone())
    }
}

#[tokio::test]
async fn live_login_count_and_session_reuse() -> TestResult {
    let Some(env) = LiveEnv::load() else {
        return Ok(());
    };

    let client = DlsiteClient::new(DlsiteClientConfig::default())?;
    let snapshot = client.login(&env.credentials()).await?;
    let count = client.content_count(ContentQuery::default()).await?;

    assert!(count.user > 0);
    assert!(count.page_limit.is_some_and(|limit| limit > 0));
    assert!(count.concurrency.is_some_and(|concurrency| concurrency > 0));

    let reused = DlsiteClient::new(DlsiteClientConfig::default())?;
    reused.import_session(&snapshot)?;
    let reused_count = reused.content_count(ContentQuery::default()).await?;

    assert_eq!(count.user, reused_count.user);

    Ok(())
}

#[tokio::test]
async fn live_sales_and_works() -> TestResult {
    let Some(env) = LiveEnv::load() else {
        return Ok(());
    };

    let client = DlsiteClient::new(DlsiteClientConfig::default())?;
    client.login(&env.credentials()).await?;

    let count = client.content_count(ContentQuery::default()).await?;
    let sales = client.sales(ContentQuery::default()).await?;
    assert!(!sales.is_empty());

    let ids = if env.batch_work_ids.is_empty() {
        sales
            .iter()
            .take(usize::min(50, sales.len()))
            .map(|purchase| purchase.id.clone())
            .collect::<Vec<_>>()
    } else {
        env.batch_work_ids
    };
    let requested_ids = ids
        .iter()
        .map(|id| id.as_ref().to_owned())
        .collect::<BTreeSet<_>>();

    let works = match client.works(&ids).await {
        Ok(works) => works,
        Err(err) => {
            let diagnostic = diagnose_works_failure(&client, &ids, count.page_limit.unwrap_or(50))
                .await
                .unwrap_or_else(|diagnostic_err| {
                    format!("failed to diagnose works failure: {diagnostic_err}")
                });
            panic!("failed to load work details: {err}\n{diagnostic}");
        }
    };
    assert!(!works.is_empty());
    let returned_ids = works
        .iter()
        .map(|work| work.id.as_ref().to_owned())
        .collect::<BTreeSet<_>>();
    assert!(returned_ids.is_subset(&requested_ids));

    Ok(())
}

#[tokio::test]
async fn live_download_resolution_probe() -> TestResult {
    let Some(env) = LiveEnv::load() else {
        return Ok(());
    };

    let Some(work_id) = env
        .direct_download_work_id
        .clone()
        .or(env.split_download_work_id.clone())
        .or(env.serial_required_work_id.clone())
        .or(env.owned_work_id.clone())
    else {
        return Ok(());
    };

    let client = DlsiteClient::new(DlsiteClientConfig::default())?;
    client.login(&env.credentials()).await?;

    let probe = client.probe_download(&work_id).await?;
    assert_ne!(probe.initial.status, 0);

    if let DownloadResolution::Direct { stream_request } = probe.resolution {
        let mut stream = client
            .open_download_stream(&stream_request, Some(DownloadByteRange::first_byte()))
            .await?;
        assert!(stream.status().is_success());
        assert!(stream.next_chunk().await?.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn live_download_resolution_pinned_cases() -> TestResult {
    let Some(env) = LiveEnv::load() else {
        return Ok(());
    };

    let client = DlsiteClient::new(DlsiteClientConfig::default())?;
    client.login(&env.credentials()).await?;

    if let Some(work_id) = env.direct_download_work_id {
        let resolution = client.resolve_download(&work_id).await?;
        let DownloadResolution::Direct { stream_request } = &resolution else {
            panic!("expected {work_id} to resolve to a direct download, got {resolution:?}");
        };
        assert!(client
            .optional_serial_download_page(&work_id)
            .await?
            .is_none());

        let mut stream = client
            .open_download_stream(stream_request, Some(DownloadByteRange::first_byte()))
            .await?;
        assert!(stream.status().is_success());
        assert!(stream.next_chunk().await?.is_some());

        let plan = client.download_plan(&work_id).await?;
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.files[0].kind, DownloadFileKind::Direct);
        assert!(plan.serial_numbers.is_empty());
        assert_plan_files_are_streamable(&client, &plan).await?;
    }

    if let Some(work_id) = env.split_download_work_id {
        let resolution = client.resolve_download(&work_id).await?;
        let DownloadResolution::Split { location } = &resolution else {
            panic!("expected {work_id} to resolve to a split download, got {resolution:?}");
        };

        let page = client.split_download_page(location.clone()).await?;
        assert!(!page.parts.is_empty());

        let mut stream = client
            .open_download_stream(
                &page.parts[0].stream_request,
                Some(DownloadByteRange::first_byte()),
            )
            .await?;
        assert!(stream.status().is_success());
        assert!(stream.next_chunk().await?.is_some());

        let plan = client.download_plan(&work_id).await?;
        assert!(!plan.files.is_empty());
        assert!(plan.serial_numbers.is_empty());
        assert!(plan
            .files
            .iter()
            .all(|file| matches!(file.kind, DownloadFileKind::SplitPart { .. })));
        assert_plan_files_are_streamable(&client, &plan).await?;
    }

    if let Some(work_id) = env.serial_required_work_id {
        let resolution = client.resolve_download(&work_id).await?;
        let DownloadResolution::SerialRequired { location } = &resolution else {
            panic!("expected {work_id} to require a serial number, got {resolution:?}");
        };

        let page = client.serial_download_page(location.clone()).await?;
        assert!(!page.serial_numbers.is_empty());
        let mut stream = client
            .open_download_stream(&page.stream_request, Some(DownloadByteRange::first_byte()))
            .await?;
        assert!(stream.status().is_success());
        assert!(stream.next_chunk().await?.is_some());

        let plan = client.download_plan(&work_id).await?;
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.files[0].kind, DownloadFileKind::Direct);
        assert!(!plan.serial_numbers.is_empty());
        assert_plan_files_are_streamable(&client, &plan).await?;
    }

    Ok(())
}

async fn assert_plan_files_are_streamable(
    client: &DlsiteClient,
    plan: &DownloadPlan,
) -> TestResult {
    assert!(!plan.files.is_empty());

    for file in &plan.files {
        let mut stream = client
            .open_download_stream(&file.stream_request, Some(DownloadByteRange::first_byte()))
            .await?;

        assert!(
            stream.status().is_success(),
            "expected {:?} stream to succeed, got {}",
            file.kind,
            stream.status()
        );

        if let Some(content_length) = stream.content_length() {
            assert!(
                content_length > 0,
                "expected {:?} stream content length to be positive",
                file.kind
            );
        }

        let first_chunk = stream.next_chunk().await?;
        assert!(
            first_chunk.as_ref().is_some_and(|chunk| !chunk.is_empty()),
            "expected {:?} stream to return bytes",
            file.kind
        );
    }

    Ok(())
}

fn optional_work_id(key: &str) -> Option<WorkId> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .map(WorkId::from)
}

async fn diagnose_works_failure(
    client: &DlsiteClient,
    ids: &[WorkId],
    page_limit: usize,
) -> Result<String, DmApiError> {
    let chunk_size = usize::max(1, page_limit);

    for (chunk_index, chunk) in ids.chunks(chunk_size).enumerate() {
        if let Err(err) = client.works_batch(chunk).await {
            let mut details = format!(
                "first failing content/works chunk: index={chunk_index}, size={}, ids={}",
                chunk.len(),
                format_work_ids(chunk)
            );

            if chunk.len() > 1 {
                let bad_items = diagnose_bad_work_items(client, chunk).await?;
                if !bad_items.is_empty() {
                    details.push_str(&format!(
                        "\nindividual IDs rejected by content/works: {}",
                        bad_items
                            .iter()
                            .map(|id| format_env_value(id))
                            .collect::<Vec<_>>()
                            .join(",")
                    ));
                }
            }

            details.push_str(&format!("\nchunk error: {err}"));
            return Ok(details);
        }
    }

    Ok("all chunks passed during diagnostics; the original failure may be transient".to_owned())
}

async fn diagnose_bad_work_items(
    client: &DlsiteClient,
    ids: &[WorkId],
) -> Result<Vec<String>, DmApiError> {
    let mut rejected = Vec::new();

    for id in ids {
        if client.works_batch(std::slice::from_ref(id)).await.is_err() {
            rejected.push(id.as_ref().to_owned());
        }
    }

    Ok(rejected)
}

fn format_work_ids(ids: &[WorkId]) -> String {
    ids.iter()
        .map(|id| format_env_value(id.as_ref()))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_env_value(value: &str) -> String {
    value.escape_debug().to_string()
}

fn load_dotenv() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dotenvy::from_path(manifest_dir.join(".env")).ok();
}
