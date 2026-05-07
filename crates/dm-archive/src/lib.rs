use std::path::{Path, PathBuf};

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

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
