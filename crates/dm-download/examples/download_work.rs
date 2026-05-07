use dm_api::{Credentials, DlsiteClient, DlsiteClientConfig, WorkId};
use dm_download::{
    download_work_files, probe_download_file_metadata, CancellationToken, DownloadJobRequest,
    DownloadPhase, UnpackPolicy,
};
use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
};

type ExampleResult<T> = Result<T, Box<dyn Error>>;

struct RealDownloadEnv {
    username: String,
    password: String,
    work_ids: Vec<WorkId>,
    target_root: PathBuf,
    max_total_bytes: Option<u64>,
    unpack_policy: UnpackPolicy,
}

impl RealDownloadEnv {
    fn load() -> ExampleResult<Self> {
        load_dotenv();

        let username = required_env(
            "DMSITE_DOWNLOAD_USERNAME",
            &["DMSITE_DOWNLOAD_TEST_USERNAME", "DMSITE_API_TEST_USERNAME"],
        )?;
        let password = required_env(
            "DMSITE_DOWNLOAD_PASSWORD",
            &["DMSITE_DOWNLOAD_TEST_PASSWORD", "DMSITE_API_TEST_PASSWORD"],
        )?;
        let work_ids = load_work_ids()?;
        let target_root = env_value("DMSITE_DOWNLOAD_TARGET_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(default_target_root);
        let max_total_bytes = match env_value("DMSITE_DOWNLOAD_MAX_TOTAL_BYTES") {
            Some(value) => {
                let parsed = value.parse::<u64>().map_err(|err| {
                    format!("invalid DMSITE_DOWNLOAD_MAX_TOTAL_BYTES={value}: {err}")
                })?;
                (parsed > 0).then_some(parsed)
            }
            None => Some(10 * 1024 * 1024),
        };
        let unpack_policy = match env_value("DMSITE_DOWNLOAD_UNPACK").as_deref() {
            Some("1") | Some("true") | Some("TRUE") => UnpackPolicy::UnpackWhenRecognized,
            _ => UnpackPolicy::KeepArchives,
        };

        Ok(Self {
            username,
            password,
            work_ids,
            target_root,
            max_total_bytes,
            unpack_policy,
        })
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExampleResult<()> {
    let env = RealDownloadEnv::load()?;
    let target_root = absolutize_target_root(&env.target_root)?;
    let client = DlsiteClient::new(DlsiteClientConfig::default())?;

    println!("logging in");
    client
        .login(&Credentials::new(
            env.username.clone(),
            env.password.clone(),
        ))
        .await?;

    for work_id in &env.work_ids {
        download_one_work(&client, work_id, &target_root, &env).await?;
    }

    Ok(())
}

async fn download_one_work(
    client: &DlsiteClient,
    work_id: &WorkId,
    target_root: &Path,
    env: &RealDownloadEnv,
) -> ExampleResult<()> {
    println!("resolving download plan for {}", work_id.as_ref());
    let plan = client.download_plan(work_id).await?;
    println!(
        "plan: {} file(s), {} serial number(s)",
        plan.files.len(),
        plan.serial_numbers.len()
    );

    let mut total_size = 0_u64;
    let mut all_sizes_known = true;

    for (file_index, file) in plan.files.iter().enumerate() {
        let metadata = probe_download_file_metadata(&client, file_index, file).await?;

        match metadata.expected_size {
            Some(size) => total_size += size,
            None => all_sizes_known = false,
        }

        println!(
            "file {}: {:?} {} {}",
            file_index + 1,
            metadata.file_kind,
            metadata.file_name,
            metadata
                .expected_size
                .map(format_bytes)
                .unwrap_or_else(|| "unknown size".to_owned())
        );
    }

    if let Some(max_total_bytes) = env.max_total_bytes {
        if !all_sizes_known {
            return Err(format!(
                "refusing to download because at least one file size is unknown and DMSITE_DOWNLOAD_MAX_TOTAL_BYTES is set to {}",
                format_bytes(max_total_bytes)
            )
            .into());
        }

        if total_size > max_total_bytes {
            return Err(format!(
                "refusing to download {}; total {} exceeds DMSITE_DOWNLOAD_MAX_TOTAL_BYTES={}",
                work_id.as_ref(),
                format_bytes(total_size),
                format_bytes(max_total_bytes)
            )
            .into());
        }
    }

    println!("target root: {}", target_root.display());

    let job = DownloadJobRequest {
        work_id: work_id.clone(),
        target_root: target_root.to_path_buf(),
        unpack_policy: env.unpack_policy,
    };
    let cancellation = CancellationToken::new();
    let mut last_reported = vec![None; plan.files.len()];
    let downloaded = download_work_files(client.clone(), &job, &plan, &cancellation, |progress| {
        if progress.phase != DownloadPhase::Downloading {
            return;
        }

        let Some(file_index) = progress.file_index else {
            return;
        };
        let Some(percentage) = progress.percentage() else {
            return;
        };

        let bucket = percentage / 10 * 10;
        let should_report = match last_reported.get(file_index).and_then(|value| *value) {
            Some(previous) => bucket > previous,
            None => true,
        };

        if should_report {
            if let Some(previous) = last_reported.get_mut(file_index) {
                *previous = Some(bucket);
            }

            println!("file {}: {}%", file_index + 1, bucket);
        }
    })
    .await?;

    for file in &downloaded.files {
        println!(
            "downloaded: {} {}",
            file.path.display(),
            format_bytes(file.bytes_written)
        );
    }

    if let Some(extraction) = downloaded.archive_extraction {
        println!(
            "extracted: {} item(s), removed {} archive file(s)",
            extraction.extracted_paths.len(),
            extraction.removed_sources.len()
        );
    }

    Ok(())
}

fn load_work_ids() -> ExampleResult<Vec<WorkId>> {
    if let Some(value) = env_value("DMSITE_DOWNLOAD_WORK_IDS") {
        let work_ids = parse_work_ids(&value);

        if !work_ids.is_empty() {
            return Ok(work_ids);
        }
    }

    let work_id = required_env(
        "DMSITE_DOWNLOAD_WORK_ID",
        &[
            "DMSITE_DOWNLOAD_TEST_WORK_ID",
            "DMSITE_API_TEST_SERIAL_REQUIRED_WORK_ID",
            "DMSITE_API_TEST_DIRECT_DOWNLOAD_WORK_ID",
        ],
    )?;

    Ok(vec![WorkId::from(work_id)])
}

fn parse_work_ids(value: &str) -> Vec<WorkId> {
    value
        .split(|character: char| character == ',' || character.is_whitespace())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| WorkId::from(value.to_owned()))
        .collect()
}

fn required_env(key: &str, fallbacks: &[&str]) -> ExampleResult<String> {
    env_value(key)
        .or_else(|| fallbacks.iter().find_map(|fallback| env_value(fallback)))
        .ok_or_else(|| format!("missing required env var {key}").into())
}

fn env_value(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn load_dotenv() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| manifest_dir.clone());

    load_dotenv_file(&manifest_dir.join(".env"));
    load_dotenv_file(&manifest_dir.join("../dm-api/.env"));
    load_dotenv_file(&repo_root.join(".env"));
}

fn load_dotenv_file(path: &Path) {
    let Ok(iter) = dotenvy::from_path_iter(path) else {
        return;
    };

    for item in iter.flatten() {
        let (key, value) = item;

        if value.trim().is_empty() || env::var_os(&key).is_some() {
            continue;
        }

        env::set_var(key, value);
    }
}

fn default_target_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(|repo_root| repo_root.join(".dlsite-downloads"))
        .unwrap_or_else(|| PathBuf::from(".dlsite-downloads"))
}

fn absolutize_target_root(path: &Path) -> ExampleResult<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    Ok(env::current_dir()?.join(path))
}

fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}
