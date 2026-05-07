use dm_archive::{extract_archive_plan, plan_archive_handling, ArchiveExtractOptions, ArchivePlan};
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[test]
fn live_extracts_downloaded_legacy_split_rar_fixture() -> TestResult {
    let Some(fixture_dir) = live_fixture_dir() else {
        return Ok(());
    };

    let temp_root = test_dir("legacy-split-rar");
    let input_dir = temp_root.join("input");
    let output_dir = temp_root.join("output");
    fs::create_dir_all(&input_dir)?;
    fs::create_dir_all(&output_dir)?;

    let copied_parts = copy_fixture_parts(&fixture_dir, &input_dir)?;
    let plan = plan_archive_handling(copied_parts);

    assert!(
        matches!(plan, ArchivePlan::LegacySplitRar { .. }),
        "fixture directory must contain a legacy split-RAR set"
    );

    let extraction = extract_archive_plan(&plan, &output_dir, ArchiveExtractOptions::default())?;

    assert!(!extraction.extracted_paths.is_empty());
    assert!(!extraction.removed_sources.is_empty());

    fs::remove_dir_all(temp_root)?;

    Ok(())
}

fn live_fixture_dir() -> Option<PathBuf> {
    load_dotenv();

    if env_value("DMSITE_ARCHIVE_TEST_LEGACY_SPLIT_RAR").as_deref() != Some("1") {
        return None;
    }

    env_value("DMSITE_ARCHIVE_TEST_LEGACY_SPLIT_RAR_DIR").map(resolve_fixture_dir)
}

fn copy_fixture_parts(source_dir: &Path, input_dir: &Path) -> TestResult<Vec<PathBuf>> {
    let mut source_paths = fs::read_dir(source_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;

    source_paths.retain(|path| path.is_file());
    source_paths.sort_by(|left, right| left.file_name().cmp(&right.file_name()));

    let mut copied_paths = Vec::new();

    for source_path in source_paths {
        let Some(file_name) = source_path.file_name() else {
            continue;
        };

        let copied_path = input_dir.join(file_name);
        fs::copy(&source_path, &copied_path)?;
        copied_paths.push(copied_path);
    }

    Ok(copied_paths)
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
}

fn resolve_fixture_dir(path: String) -> PathBuf {
    let path = PathBuf::from(path);

    if path.is_absolute() {
        return path;
    }

    repo_root().join(path)
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or(manifest_dir)
}

fn test_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = env::temp_dir().join(format!("dm-archive-{name}-{}-{unique}", std::process::id()));

    fs::create_dir_all(&dir).unwrap();
    dir
}
