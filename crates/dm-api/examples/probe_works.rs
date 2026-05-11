use dm_api::{ContentQuery, Credentials, DlsiteClient, DlsiteClientConfig, WorkId};
use serde_json::Value;
use std::{collections::BTreeMap, env, error::Error, path::PathBuf};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    load_dotenv();

    let username = env::var("DMSITE_API_TEST_USERNAME")?;
    let password = env::var("DMSITE_API_TEST_PASSWORD")?;
    let work_ids = work_ids();

    if work_ids.is_empty() {
        println!("no work IDs configured");
        return Ok(());
    }

    let client = DlsiteClient::new(DlsiteClientConfig::default())?;
    client.login(&Credentials::new(username, password)).await?;
    let count = client.content_count(ContentQuery::default()).await?;
    println!(
        "content/count: user={}, production={}, page_limit={:?}, concurrency={:?}",
        count.user, count.production, count.page_limit, count.concurrency
    );

    for work in client.works(&work_ids).await? {
        println!("work_id={}", work.id);
        println!(
            "  known: work_type={}, age_category={}, content_size={:?}",
            work.work_kind.code,
            serde_json::to_value(&work.age_category)?
                .as_str()
                .unwrap_or("unknown"),
            work.content_size
        );

        let mut extra_keys = work.extra.keys().map(String::as_str).collect::<Vec<_>>();
        extra_keys.sort_unstable();
        println!("  extra_keys={}", extra_keys.join(","));

        for (key, value) in size_like_values(&work.extra) {
            println!("  candidate {key}={value}");
        }
    }

    Ok(())
}

fn work_ids() -> Vec<WorkId> {
    if let Some(value) = env::var("DMSITE_API_TEST_BATCH_WORK_IDS")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .take(10)
            .map(WorkId::from)
            .collect();
    }

    [
        "DMSITE_API_TEST_OWNED_WORK_ID",
        "DMSITE_API_TEST_DIRECT_DOWNLOAD_WORK_ID",
        "DMSITE_API_TEST_SPLIT_DOWNLOAD_WORK_ID",
        "DMSITE_API_TEST_SERIAL_REQUIRED_WORK_ID",
    ]
    .into_iter()
    .filter_map(|key| env::var(key).ok())
    .map(|value| value.trim().to_owned())
    .filter(|value| !value.is_empty())
    .map(WorkId::from)
    .collect()
}

fn size_like_values(extra: &BTreeMap<String, Value>) -> Vec<(&str, String)> {
    let mut values = Vec::new();

    for (key, value) in extra {
        if is_size_like_key(key) {
            values.push((key.as_str(), compact_value(value)));
        }
    }

    values.sort_by(|left, right| left.0.cmp(right.0));
    values
}

fn is_size_like_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("size") || key.contains("byte") || key.contains("file")
}

fn compact_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => "null".to_owned(),
        Value::Array(items) => format!("array(len={})", items.len()),
        Value::Object(object) => format!("object(keys={})", object.len()),
    }
}

fn load_dotenv() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [manifest_dir.join(".env")];

    for candidate in candidates {
        if candidate.exists() {
            let _ = dotenvy::from_path(candidate);
        }
    }
}
