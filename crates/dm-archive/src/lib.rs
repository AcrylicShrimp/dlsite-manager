use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveExtractOptions {
    pub flatten_single_root: bool,
    pub remove_sources: bool,
}

impl Default for ArchiveExtractOptions {
    fn default() -> Self {
        Self {
            flatten_single_root: true,
            remove_sources: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveExtraction {
    pub output_dir: PathBuf,
    pub extracted_paths: Vec<PathBuf>,
    pub removed_sources: Vec<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("I/O error")]
    Io(#[from] io::Error),
    #[error("ZIP error")]
    Zip(#[from] zip::result::ZipError),
    #[error("unsafe archive entry path: {entry}")]
    UnsafeArchiveEntry { entry: String },
    #[error("archive extraction target already exists: {path}")]
    TargetAlreadyExists { path: PathBuf },
    #[error("archive plan is not extractable yet: {kind}")]
    UnsupportedPlan { kind: &'static str },
}

pub type Result<T> = std::result::Result<T, ArchiveError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchivePlan {
    KeepArchives {
        files: Vec<PathBuf>,
    },
    SingleZip {
        archive: PathBuf,
    },
    LegacySplitRar {
        first_part: PathBuf,
        parts: Vec<PathBuf>,
    },
}

impl ArchivePlan {
    pub fn is_unpackable(&self) -> bool {
        !matches!(self, Self::KeepArchives { .. })
    }

    pub fn source_files(&self) -> Vec<&Path> {
        match self {
            Self::KeepArchives { files } => files.iter().map(PathBuf::as_path).collect(),
            Self::SingleZip { archive } => vec![archive.as_path()],
            Self::LegacySplitRar { parts, .. } => parts.iter().map(PathBuf::as_path).collect(),
        }
    }
}

pub fn plan_archive_handling(files: impl IntoIterator<Item = PathBuf>) -> ArchivePlan {
    let files = files.into_iter().collect::<Vec<_>>();

    match files.as_slice() {
        [archive] if has_extension(archive, "zip") => ArchivePlan::SingleZip {
            archive: archive.clone(),
        },
        [first_part, ..] if has_extension(first_part, "exe") => ArchivePlan::LegacySplitRar {
            first_part: first_part.clone(),
            parts: files,
        },
        _ => ArchivePlan::KeepArchives { files },
    }
}

pub fn extract_archive_plan(
    plan: &ArchivePlan,
    output_dir: impl AsRef<Path>,
    options: ArchiveExtractOptions,
) -> Result<ArchiveExtraction> {
    match plan {
        ArchivePlan::KeepArchives { .. } => Ok(ArchiveExtraction {
            output_dir: output_dir.as_ref().to_owned(),
            extracted_paths: Vec::new(),
            removed_sources: Vec::new(),
        }),
        ArchivePlan::SingleZip { archive } => extract_single_zip(archive, output_dir, options),
        ArchivePlan::LegacySplitRar { .. } => Err(ArchiveError::UnsupportedPlan {
            kind: "legacy_split_rar",
        }),
    }
}

pub fn extract_single_zip(
    archive: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    options: ArchiveExtractOptions,
) -> Result<ArchiveExtraction> {
    let archive = archive.as_ref();
    let output_dir = output_dir.as_ref();

    fs::create_dir_all(output_dir)?;

    let staging_dir = create_staging_dir(output_dir)?;
    let result = extract_single_zip_inner(archive, output_dir, &staging_dir, options);

    if result.is_err() {
        fs::remove_dir_all(&staging_dir).ok();
    }

    result
}

fn extract_single_zip_inner(
    archive: &Path,
    output_dir: &Path,
    staging_dir: &Path,
    options: ArchiveExtractOptions,
) -> Result<ArchiveExtraction> {
    let archive_file = fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(archive_file)?;

    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let entry_path = safe_archive_entry_path(staging_dir, entry.name())?;

        if entry.is_dir() {
            fs::create_dir_all(&entry_path)?;
            continue;
        }

        if let Some(parent) = entry_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output_file = fs::File::create(&entry_path)?;
        io::copy(&mut entry, &mut output_file)?;
    }

    let content_root = content_root(staging_dir, options.flatten_single_root)?;
    let extracted_paths = move_extracted_contents(&content_root, output_dir)?;

    fs::remove_dir_all(staging_dir).ok();

    let removed_sources = if options.remove_sources {
        fs::remove_file(archive)?;
        vec![archive.to_owned()]
    } else {
        Vec::new()
    };

    Ok(ArchiveExtraction {
        output_dir: output_dir.to_owned(),
        extracted_paths,
        removed_sources,
    })
}

fn content_root(staging_dir: &Path, flatten_single_root: bool) -> Result<PathBuf> {
    if !flatten_single_root {
        return Ok(staging_dir.to_owned());
    }

    let entries = fs::read_dir(staging_dir)?.collect::<io::Result<Vec<_>>>()?;

    if entries.len() == 1 && entries[0].file_type()?.is_dir() {
        return Ok(entries[0].path());
    }

    Ok(staging_dir.to_owned())
}

fn move_extracted_contents(content_root: &Path, output_dir: &Path) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(content_root)?.collect::<io::Result<Vec<_>>>()?;
    let mut extracted_paths = Vec::with_capacity(entries.len());

    for entry in entries {
        let target_path = output_dir.join(entry.file_name());

        if target_path.try_exists()? {
            return Err(ArchiveError::TargetAlreadyExists { path: target_path });
        }

        fs::rename(entry.path(), &target_path)?;
        extracted_paths.push(target_path);
    }

    Ok(extracted_paths)
}

fn safe_archive_entry_path(base: &Path, entry: &str) -> Result<PathBuf> {
    if entry.contains('\\') || entry.contains('\0') {
        return Err(ArchiveError::UnsafeArchiveEntry {
            entry: entry.to_owned(),
        });
    }

    let path = Path::new(entry);
    let mut relative = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            _ => {
                return Err(ArchiveError::UnsafeArchiveEntry {
                    entry: entry.to_owned(),
                });
            }
        }
    }

    if relative.as_os_str().is_empty() {
        return Err(ArchiveError::UnsafeArchiveEntry {
            entry: entry.to_owned(),
        });
    }

    Ok(base.join(relative))
}

fn create_staging_dir(output_dir: &Path) -> Result<PathBuf> {
    for index in 0..1000 {
        let staging_dir = output_dir.join(format!(".dm-archive-{index}"));

        match fs::create_dir(&staging_dir) {
            Ok(()) => return Ok(staging_dir),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err.into()),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "unable to allocate archive staging directory",
    )
    .into())
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn keeps_empty_sets() {
        assert_eq!(
            plan_archive_handling(Vec::<PathBuf>::new()),
            ArchivePlan::KeepArchives { files: Vec::new() }
        );
    }

    #[test]
    fn detects_single_zip_case_insensitively() {
        assert_eq!(
            plan_archive_handling([PathBuf::from("RJ123456.ZIP")]),
            ArchivePlan::SingleZip {
                archive: PathBuf::from("RJ123456.ZIP")
            }
        );
    }

    #[test]
    fn detects_legacy_split_rar_from_first_exe() {
        assert_eq!(
            plan_archive_handling([
                PathBuf::from("RJ123456.exe"),
                PathBuf::from("RJ123456.r00"),
                PathBuf::from("RJ123456.r01"),
            ]),
            ArchivePlan::LegacySplitRar {
                first_part: PathBuf::from("RJ123456.exe"),
                parts: vec![
                    PathBuf::from("RJ123456.exe"),
                    PathBuf::from("RJ123456.r00"),
                    PathBuf::from("RJ123456.r01"),
                ],
            }
        );
    }

    #[test]
    fn keeps_unrecognized_files() {
        assert_eq!(
            plan_archive_handling([PathBuf::from("readme.txt")]),
            ArchivePlan::KeepArchives {
                files: vec![PathBuf::from("readme.txt")]
            }
        );
    }

    #[test]
    fn keeps_multiple_files_when_first_file_is_not_exe() {
        assert_eq!(
            plan_archive_handling([PathBuf::from("part1.bin"), PathBuf::from("part2.bin")]),
            ArchivePlan::KeepArchives {
                files: vec![PathBuf::from("part1.bin"), PathBuf::from("part2.bin")]
            }
        );
    }

    #[test]
    fn extracts_single_zip_and_flattens_single_root() {
        let dir = test_dir("single-zip");
        let archive = dir.join("RJ123456.zip");
        write_zip(&archive, &[("RJ123456/readme.txt", b"hello".as_slice())]);

        let extraction =
            extract_single_zip(&archive, &dir, ArchiveExtractOptions::default()).unwrap();

        assert_eq!(std::fs::read(dir.join("readme.txt")).unwrap(), b"hello");
        assert!(!archive.exists());
        assert_eq!(extraction.extracted_paths, vec![dir.join("readme.txt")]);
        assert_eq!(extraction.removed_sources, vec![archive]);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn can_preserve_single_root_directory() {
        let dir = test_dir("preserve-root");
        let archive = dir.join("RJ123456.zip");
        write_zip(&archive, &[("RJ123456/readme.txt", b"hello".as_slice())]);

        extract_single_zip(
            &archive,
            &dir,
            ArchiveExtractOptions {
                flatten_single_root: false,
                remove_sources: false,
            },
        )
        .unwrap();

        assert_eq!(
            std::fs::read(dir.join("RJ123456").join("readme.txt")).unwrap(),
            b"hello"
        );
        assert!(archive.exists());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_zip_entries_that_escape_output_dir() {
        let dir = test_dir("unsafe-entry");
        let archive = dir.join("RJ123456.zip");
        write_zip(&archive, &[("../evil.txt", b"evil".as_slice())]);

        let err = extract_single_zip(&archive, &dir, ArchiveExtractOptions::default()).unwrap_err();

        assert!(matches!(err, ArchiveError::UnsafeArchiveEntry { .. }));
        assert!(!dir.parent().unwrap().join("evil.txt").exists());
        assert!(!dir.join(".dm-archive-0").exists());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_windows_style_escape_entries() {
        let dir = test_dir("unsafe-backslash");
        let archive = dir.join("RJ123456.zip");
        write_zip(&archive, &[("..\\evil.txt", b"evil".as_slice())]);

        let err = extract_single_zip(&archive, &dir, ArchiveExtractOptions::default()).unwrap_err();

        assert!(matches!(err, ArchiveError::UnsafeArchiveEntry { .. }));

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn does_not_overwrite_existing_output_files() {
        let dir = test_dir("target-exists");
        let archive = dir.join("RJ123456.zip");
        std::fs::write(dir.join("readme.txt"), b"existing").unwrap();
        write_zip(&archive, &[("readme.txt", b"new".as_slice())]);

        let err = extract_single_zip(
            &archive,
            &dir,
            ArchiveExtractOptions {
                flatten_single_root: true,
                remove_sources: false,
            },
        )
        .unwrap_err();

        assert!(matches!(err, ArchiveError::TargetAlreadyExists { .. }));
        assert_eq!(std::fs::read(dir.join("readme.txt")).unwrap(), b"existing");

        std::fs::remove_dir_all(dir).unwrap();
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (name, content) in entries {
            zip.start_file(name, options).unwrap();
            zip.write_all(content).unwrap();
        }

        zip.finish().unwrap();
    }

    fn test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("dm-archive-{name}-{}-{unique}", std::process::id()));

        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
