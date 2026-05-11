use dm_api::{ContentQuery, Credentials, DlsiteClient, DlsiteClientConfig, WorkId};
use serde_json::{Map, Value};
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

    if raw_probe_enabled() {
        let raw = client
            .raw_works_batch_with_body_limit(&work_ids, raw_body_limit())
            .await?;
        print_raw_works_probe(&raw)?;
    }

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

fn raw_probe_enabled() -> bool {
    env::var("DMSITE_API_PROBE_WORKS_RAW")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn raw_body_limit() -> usize {
    env::var("DMSITE_API_PROBE_WORKS_BODY_LIMIT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2_000_000)
}

fn print_raw_works_probe(raw: &dm_api::raw::RawResponse) -> Result<(), Box<dyn Error>> {
    println!(
        "raw content/works: status={}, content_type={:?}, location={:?}",
        raw.status, raw.content_type, raw.location
    );

    let Some(body) = raw.body_snippet.as_deref() else {
        println!("  raw_body=none");
        return Ok(());
    };

    println!("  raw_body_chars={}", body.chars().count());
    let literal_hits = serial_literal_hits(body);
    if literal_hits.is_empty() {
        println!("  serial_like_literals=none");
    } else {
        println!("  serial_like_literals={}", literal_hits.join(","));
    }

    let value = serde_json::from_str::<Value>(body)?;
    let Some(works) = value.get("works").and_then(Value::as_array) else {
        println!("  raw_works=missing");
        return Ok(());
    };

    for work in works {
        let work_id = work
            .get("workno")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        println!("  raw_work_id={work_id}");

        let top_keys = object_keys(work);
        println!("    raw_top_keys={}", top_keys.join(","));

        let mut serial_like_paths = Vec::new();
        collect_key_matches(
            work,
            "$",
            &["serial", "license", "activation", "product_key"],
            &mut serial_like_paths,
        );

        if serial_like_paths.is_empty() {
            println!("    serial_like_paths=none");
        } else {
            println!("    serial_like_paths={}", serial_like_paths.join(","));
        }
    }

    Ok(())
}

fn serial_literal_hits(body: &str) -> Vec<&'static str> {
    let lower_body = body.to_ascii_lowercase();
    ["serial", "license", "activation", "product_key", "シリアル"]
        .into_iter()
        .filter(|needle| {
            if needle.is_ascii() {
                lower_body.contains(*needle)
            } else {
                body.contains(*needle)
            }
        })
        .collect()
}

fn object_keys(value: &Value) -> Vec<&str> {
    let Some(object) = value.as_object() else {
        return Vec::new();
    };

    let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    keys
}

fn collect_key_matches(value: &Value, path: &str, needles: &[&str], matches_out: &mut Vec<String>) {
    match value {
        Value::Object(object) => collect_object_key_matches(object, path, needles, matches_out),
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_key_matches(item, &format!("{path}[{index}]"), needles, matches_out);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn collect_object_key_matches(
    object: &Map<String, Value>,
    path: &str,
    needles: &[&str],
    matches_out: &mut Vec<String>,
) {
    for (key, value) in object {
        let child_path = format!("{path}.{key}");
        let lower_key = key.to_ascii_lowercase();
        if needles.iter().any(|needle| lower_key.contains(needle)) {
            matches_out.push(child_path.clone());
        }

        collect_key_matches(value, &child_path, needles, matches_out);
    }
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
