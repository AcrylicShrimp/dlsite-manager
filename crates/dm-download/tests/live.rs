use dm_api::{Credentials, DlsiteClient, DlsiteClientConfig, WorkId};
use dm_download::{
    download_work_files, probe_download_file_metadata, CancellationToken, DownloadJobRequest,
    UnpackPolicy,
};
use std::{
    env,
    error::Error,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

type TestResult = Result<(), Box<dyn Error>>;

struct LiveEnv {
    username: String,
    password: String,
    work_id: WorkId,
    max_total_bytes: u64,
}

impl LiveEnv {
    fn load() -> Option<Self> {
        load_dotenv();

        if env::var("DMSITE_DOWNLOAD_TEST_LIVE").ok().as_deref() != Some("1") {
            return None;
        }

        Some(Self {
            username: env_value("DMSITE_DOWNLOAD_TEST_USERNAME")
                .or_else(|| env_value("DMSITE_API_TEST_USERNAME"))?,
            password: env_value("DMSITE_DOWNLOAD_TEST_PASSWORD")
                .or_else(|| env_value("DMSITE_API_TEST_PASSWORD"))?,
            work_id: env_value("DMSITE_DOWNLOAD_TEST_WORK_ID")
                .or_else(|| env_value("DMSITE_API_TEST_SERIAL_REQUIRED_WORK_ID"))
                .or_else(|| env_value("DMSITE_API_TEST_DIRECT_DOWNLOAD_WORK_ID"))
                .map(WorkId::from)?,
            max_total_bytes: env_value("DMSITE_DOWNLOAD_TEST_MAX_TOTAL_BYTES")
                .and_then(|value| value.parse().ok())
                .unwrap_or(10 * 1024 * 1024),
        })
    }
}

#[tokio::test]
async fn live_downloads_pinned_small_work_to_temp_dir() -> TestResult {
    let Some(env) = LiveEnv::load() else {
        return Ok(());
    };

    let client = DlsiteClient::new(DlsiteClientConfig::default())?;
    client
        .login(&Credentials::new(env.username, env.password))
        .await?;

    let plan = client.download_plan(&env.work_id).await?;
    let mut expected_sizes = Vec::new();

    for (file_index, file) in plan.files.iter().enumerate() {
        let metadata = probe_download_file_metadata(&client, file_index, file).await?;
        let Some(expected_size) = metadata.expected_size else {
            return Ok(());
        };

        expected_sizes.push(expected_size);
    }

    let total_size = expected_sizes.iter().sum::<u64>();

    if total_size > env.max_total_bytes {
        return Ok(());
    }

    let target_root = test_dir("live");
    let job = DownloadJobRequest {
        work_id: env.work_id,
        target_root: target_root.clone(),
        unpack_policy: UnpackPolicy::KeepArchives,
    };
    let downloaded =
        download_work_files(client, &job, &plan, &CancellationToken::new(), |_| {}).await?;

    assert_eq!(downloaded.files.len(), plan.files.len());

    for (file, expected_size) in downloaded.files.iter().zip(expected_sizes) {
        assert_eq!(std::fs::metadata(&file.path)?.len(), expected_size);
        assert!(file.path.starts_with(&downloaded.target_dir));
    }

    std::fs::remove_dir_all(target_root)?;

    Ok(())
}

fn env_value(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn load_dotenv() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dotenvy::from_path(manifest_dir.join(".env")).ok();
    dotenvy::from_path(manifest_dir.join("../dm-api/.env")).ok();
}

fn test_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = env::temp_dir().join(format!(
        "dm-download-{name}-{}-{unique}",
        std::process::id()
    ));

    std::fs::create_dir_all(&dir).unwrap();
    dir
}
