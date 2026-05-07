use dm_api::{
    ContentQuery, Credentials, DlsiteClient, DlsiteClientConfig, DownloadResolution, RangeStart,
    WorkId,
};
use std::{env, error::Error, path::PathBuf};

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

    let works = client.works(&ids).await?;
    assert!(!works.is_empty());

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

    let raw = client.raw_download_probe(&work_id).await?;
    assert_ne!(raw.status, 0);

    let resolution = client.resolve_download(&work_id).await?;

    if let DownloadResolution::Direct { stream_request } = resolution {
        let stream = client
            .open_download_stream(&stream_request, Some(RangeStart(0)))
            .await?;
        assert!(stream.status().is_success());
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

fn load_dotenv() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dotenvy::from_path(manifest_dir.join(".env")).ok();
}
