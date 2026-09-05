//! Destination-side installation. SQL and job lifecycle stay with Library.
use super::*;
use dm_storage::DownloadFinalization;
use std::io;

pub(super) const PREFIX: &str = ".dm-finalize-";

pub(super) fn checked_child(path: &Path, roots: &[&Path]) -> Result<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(LibraryError::DownloadPathOutsideRoots(path.to_owned()));
    }
    // Reject symlinks even when they currently point inside the permitted root.
    for ancestor in path.ancestors() {
        if let Ok(meta) = std::fs::symlink_metadata(ancestor) {
            if meta.file_type().is_symlink() {
                return Err(LibraryError::DownloadPathOutsideRoots(path.to_owned()));
            }
        }
    }
    let allowed = roots
        .iter()
        .filter_map(|root| root.canonicalize().ok())
        .collect::<Vec<_>>();
    if !path_is_download_child_of_any_root(path, &allowed) {
        return Err(LibraryError::DownloadPathOutsideRoots(path.to_owned()));
    }
    Ok(path.to_owned())
}

pub(super) fn paths(
    work_id: &str,
    library_root: &Path,
    download_root: &Path,
) -> Result<(PathBuf, PathBuf)> {
    if detect_work_ids_in_text(work_id).as_slice() != [WorkId::from(work_id.to_owned())] {
        return Err(LibraryError::DownloadPathOutsideRoots(PathBuf::from(
            work_id,
        )));
    }
    std::fs::create_dir_all(library_root)?;
    std::fs::create_dir_all(download_root)?;
    let library_root = library_root.canonicalize()?;
    let download_root = download_root.canonicalize()?;
    let staging = checked_child(&download_root.join(work_id), &[&download_root])?;
    let final_path = checked_child(&library_root.join(work_id), &[&library_root])?;
    if staging.starts_with(&final_path) || final_path.starts_with(&staging) {
        return Err(LibraryError::DownloadPathOutsideRoots(staging));
    }
    Ok((staging, final_path))
}

pub(super) struct Installation {
    pub record: DownloadFinalization,
}

impl Installation {
    pub fn new(
        work_id: &str,
        staging: &Path,
        final_path: &Path,
        old: Option<&Path>,
        replace: bool,
    ) -> Result<Self> {
        let old = old.filter(|path| path.is_dir()).map(Path::to_owned);
        if final_path.try_exists()? && (!replace || old.as_deref().is_some_and(|p| p != final_path))
        {
            return Err(LibraryError::DownloadTargetExists(final_path.to_owned()));
        }
        let old = old.or_else(|| final_path.is_dir().then(|| final_path.to_owned()));
        if let Some(old) = &old {
            if !replace {
                return Err(LibraryError::DownloadTargetExists(old.clone()));
            }
            checked_child(old, &[final_path.parent().unwrap()])?;
            if old.starts_with(staging) || staging.starts_with(old) {
                return Err(LibraryError::DownloadPathOutsideRoots(old.clone()));
            }
        }
        let operation_id = Uuid::new_v4().to_string();
        let temporary = final_path
            .parent()
            .unwrap()
            .join(format!("{PREFIX}{operation_id}"));
        Ok(Self {
            record: DownloadFinalization {
                work_id: work_id.to_owned(),
                operation_id,
                staging_path: staging.to_string_lossy().into_owned(),
                final_path: final_path.to_string_lossy().into_owned(),
                old_path: old.map(|p| p.to_string_lossy().into_owned()),
                temporary_path: temporary.to_string_lossy().into_owned(),
                committed: false,
            },
        })
    }

    fn temporary(&self) -> &Path {
        Path::new(&self.record.temporary_path)
    }
    fn payload(&self) -> PathBuf {
        self.temporary().join("payload")
    }
    fn backup(&self) -> PathBuf {
        self.temporary().join("backup")
    }

    pub fn validate(&self, roots: &[&Path]) -> Result<()> {
        let r = &self.record;
        let final_path = checked_child(Path::new(&r.final_path), roots)?;
        let staging = checked_child(Path::new(&r.staging_path), roots)?;
        let temporary = checked_child(self.temporary(), roots)?;
        if Uuid::parse_str(&r.operation_id).is_err()
            || temporary
                != final_path
                    .parent()
                    .unwrap()
                    .join(format!("{PREFIX}{}", r.operation_id))
            || staging.starts_with(&final_path)
            || final_path.starts_with(&staging)
            || staging.starts_with(&temporary)
            || temporary.starts_with(&staging)
        {
            return Err(self.required("invalid recovery paths"));
        }
        for child in [self.payload(), self.backup()] {
            checked_child(&child, roots)?;
        }
        if let Some(old) = &r.old_path {
            let old = checked_child(Path::new(old), roots)?;
            if old.starts_with(&temporary)
                || temporary.starts_with(&old)
                || old.starts_with(&staging)
                || staging.starts_with(&old)
            {
                return Err(self.required("overlapping recovery paths"));
            }
        }
        Ok(())
    }

    pub async fn prepare(&self) -> Result<()> {
        self.prepare_with_rename_error(None).await
    }

    // A narrow seam for deterministic cross-device/error tests without extra mounts.
    async fn prepare_with_rename_error(&self, rename_error: Option<io::ErrorKind>) -> Result<()> {
        tokio::fs::create_dir(self.temporary()).await?;
        tokio::fs::write(self.temporary().join("owner"), &self.record.operation_id).await?;
        let renamed = match rename_error {
            Some(kind) => Err(io::Error::from(kind)),
            None => tokio::fs::rename(&self.record.staging_path, self.payload()).await,
        };
        match renamed {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
                let source = PathBuf::from(&self.record.staging_path);
                let destination = self.payload();
                tokio::task::spawn_blocking(move || copy_tree(&source, &destination))
                    .await
                    .map_err(|error| io::Error::other(error.to_string()))??;
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }

    pub async fn install(&self) -> Result<()> {
        if let Some(old) = &self.record.old_path {
            tokio::fs::rename(old, self.backup()).await?;
        }
        if Path::new(&self.record.final_path).try_exists()? {
            return Err(self.required("destination appeared during installation"));
        }
        tokio::fs::rename(self.payload(), &self.record.final_path).await?;
        Ok(())
    }

    fn owned(&self) -> Result<()> {
        let marker = self.temporary().join("owner");
        if !std::fs::symlink_metadata(&marker)?.file_type().is_file()
            || std::fs::read_to_string(marker)? != self.record.operation_id
        {
            return Err(self.required("recovery ownership marker does not match"));
        }
        Ok(())
    }

    fn marker_missing(&self) -> Result<bool> {
        match std::fs::symlink_metadata(self.temporary().join("owner")) {
            Ok(_) => Ok(false),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(true),
            Err(error) => Err(error.into()),
        }
    }

    fn check_rollback_complete(&self) -> Result<()> {
        if !Path::new(&self.record.staging_path).is_dir()
            || self
                .record
                .old_path
                .as_ref()
                .is_some_and(|p| !Path::new(p).is_dir())
            || (self.record.old_path.as_deref() != Some(&self.record.final_path)
                && Path::new(&self.record.final_path).try_exists()?)
        {
            return Err(self.required("missing or unexpected recovery material"));
        }
        Ok(())
    }

    async fn remove_incomplete_initialization(&self) -> Result<bool> {
        if self.record.committed {
            return Ok(false);
        }
        let marker = self.temporary().join("owner");
        if !std::fs::symlink_metadata(&marker)?.file_type().is_file() {
            return Ok(false);
        }
        let contents = std::fs::read(&marker)?;
        let operation_id = self.record.operation_id.as_bytes();
        if contents.len() >= operation_id.len() || !operation_id.starts_with(&contents) {
            return Ok(false);
        }
        // Preparation cannot move/copy payload until the full marker write succeeds.
        // A strict prefix alone is insufficient: require intact pre-install paths and
        // only this regular marker in the validated recorded temporary directory.
        self.check_rollback_complete()?;
        for entry in std::fs::read_dir(self.temporary())? {
            if entry?.file_name() != "owner" {
                return Err(self.required("incomplete owner beside unexpected recovery material"));
            }
        }
        tokio::fs::remove_file(marker).await?;
        tokio::fs::remove_dir(self.temporary()).await?;
        Ok(true)
    }

    fn check_temporary_entries(&self) -> Result<()> {
        for entry in std::fs::read_dir(self.temporary())? {
            let name = entry?.file_name();
            if name != "owner" && name != "payload" && name != "backup" {
                return Err(self.required("unexpected file in recovery directory"));
            }
        }
        Ok(())
    }

    /// Restores old content. New data is returned to staging when possible; a partial
    /// cross-device copy is retained under the owned temporary path for inspection.
    pub async fn rollback(&self) -> Result<()> {
        if !self.temporary().try_exists()? {
            self.check_rollback_complete()?;
            return Ok(());
        }
        if self.marker_missing()? {
            // The validated recorded directory can be empty before marker creation or
            // after retirement. Never recursively remove unowned contents.
            self.check_rollback_complete()?;
            tokio::fs::remove_dir(self.temporary()).await?;
            return Ok(());
        }
        if self.remove_incomplete_initialization().await? {
            return Ok(());
        }
        self.owned()?;
        self.check_temporary_entries()?;
        let final_path = Path::new(&self.record.final_path);
        let backup = self.backup();
        let payload = self.payload();
        if backup.try_exists()? {
            let old = self
                .record
                .old_path
                .as_ref()
                .ok_or_else(|| self.required("unexpected backup"))?;
            if final_path.try_exists()? {
                if payload.try_exists()? {
                    return Err(self.required("both payload and destination exist"));
                }
                tokio::fs::rename(final_path, &payload).await?;
            }
            if Path::new(old).try_exists()? {
                return Err(self.required("old path is occupied"));
            }
            tokio::fs::rename(&backup, old).await?;
        } else if self.record.old_path.is_none()
            && !payload.try_exists()?
            && final_path.try_exists()?
        {
            tokio::fs::rename(final_path, &payload).await?;
        } else if self
            .record
            .old_path
            .as_ref()
            .is_some_and(|p| !Path::new(p).is_dir())
        {
            return Err(self.required("old content and backup are missing"));
        }
        if payload.try_exists()? && !Path::new(&self.record.staging_path).try_exists()? {
            tokio::fs::rename(&payload, &self.record.staging_path).await?;
        }
        // Keep partial/new data if a cross-device source still exists. Never delete it
        // as part of rollback, nor infer ownership from the prefix alone.
        if !payload.try_exists()? {
            tokio::fs::remove_file(self.temporary().join("owner")).await?;
            tokio::fs::remove_dir(self.temporary()).await?;
        }
        Ok(())
    }

    pub async fn cleanup(&self) -> Result<()> {
        let temporary_exists = self.temporary().try_exists()?;
        if !temporary_exists || self.marker_missing()? {
            if !Path::new(&self.record.final_path).is_dir()
                || Path::new(&self.record.staging_path).try_exists()?
            {
                return Err(self.required("committed cleanup is not complete"));
            }
            if temporary_exists {
                tokio::fs::remove_dir(self.temporary()).await?;
            }
            return Ok(());
        }
        self.owned()?;
        self.check_temporary_entries()?;
        if !Path::new(&self.record.final_path).is_dir() || self.payload().try_exists()? {
            return Err(self.required("committed installation is not intact"));
        }
        if self.backup().try_exists()? {
            tokio::fs::remove_dir_all(self.backup()).await?;
        }
        if Path::new(&self.record.staging_path).try_exists()? {
            tokio::fs::remove_dir_all(&self.record.staging_path).await?;
        }
        tokio::fs::remove_file(self.temporary().join("owner")).await?;
        tokio::fs::remove_dir(self.temporary()).await?;
        Ok(())
    }

    pub fn required(&self, reason: &str) -> LibraryError {
        LibraryError::RecoveryRequired(format!(
            "{reason}; retained paths: {}, {}, {}",
            self.record.temporary_path, self.record.staging_path, self.record.final_path
        ))
    }
}

fn copy_tree(source: &Path, destination: &Path) -> io::Result<()> {
    std::fs::create_dir(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            let expected = entry.metadata()?.len();
            if std::fs::copy(entry.path(), target)? != expected {
                return Err(io::Error::other("source changed during copy"));
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "unsupported entry in download payload",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dm_credentials::InMemoryCredentialStore;

    struct Fixture {
        root: PathBuf,
        library: Library,
        installation: Installation,
        old: WorkDownloadState,
        update: WorkDownloadUpdate,
    }

    impl Fixture {
        async fn new() -> Result<Self> {
            let root =
                std::env::temp_dir().join(format!("dm-finalization-test-{}", Uuid::new_v4()));
            std::fs::create_dir(&root)?;
            let root = root.canonicalize()?;
            let (staging, final_path) =
                paths("RJ000001", &root.join("library"), &root.join("staging"))?;
            std::fs::create_dir(&staging)?;
            std::fs::create_dir(&final_path)?;
            std::fs::write(staging.join("content"), b"new")?;
            std::fs::write(final_path.join("content"), b"old")?;
            let storage = Storage::open(root.join("library.sqlite")).await?;
            storage.run_migrations().await?;
            let mut update = WorkDownloadUpdate {
                work_id: "RJ000001".to_owned(),
                status: WorkDownloadStatus::Downloaded,
                local_path: Some(final_path.to_string_lossy().into_owned()),
                staging_path: None,
                unpack_policy: "manual".to_owned(),
                bytes_received: 3,
                bytes_total: Some(3),
                error_code: None,
                error_message: None,
                started_at: Some("old".to_owned()),
                completed_at: Some("old".to_owned()),
                updated_at: "old".to_owned(),
            };
            storage
                .import_local_work_downloads_with_metadata(
                    &[LocalWorkDownloadImport {
                        work: cached_work_from_local_folder("RJ000001", "old", "old")?,
                        download: update.clone(),
                    }],
                    &[],
                )
                .await?;
            let old = storage.work_download_state("RJ000001").await?;
            update.completed_at = Some("new".to_owned());
            update.updated_at = "new".to_owned();
            let library = Library::new(storage, Arc::new(InMemoryCredentialStore::new()));
            let installation =
                Installation::new("RJ000001", &staging, &final_path, Some(&final_path), true)?;
            library
                .storage
                .begin_download_finalization(&installation.record)
                .await?;
            Ok(Self {
                root,
                library,
                installation,
                old,
                update,
            })
        }

        async fn reopen(&mut self) -> Result<()> {
            self.library = Library::new(
                Storage::open(self.root.join("library.sqlite")).await?,
                Arc::new(InMemoryCredentialStore::new()),
            );
            Ok(())
        }

        async fn recover(&self) -> Result<()> {
            self.library
                .recover_download(
                    "RJ000001",
                    &[&self.root.join("library"), &self.root.join("staging")],
                )
                .await
        }

        fn recovered_old(&self) -> WorkDownloadState {
            let mut old = self.old.clone();
            old.staging_path = Some(self.installation.record.staging_path.clone());
            old
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[tokio::test]
    async fn restart_at_every_swap_and_commit_boundary_preserves_bytes_and_rows() -> Result<()> {
        for boundary in 0..=4 {
            let mut f = Fixture::new().await?;
            if boundary >= 1 {
                f.installation.prepare().await?;
            }
            if boundary >= 2 {
                tokio::fs::rename(
                    f.installation.record.old_path.as_ref().unwrap(),
                    f.installation.backup(),
                )
                .await?;
            }
            if boundary >= 3 {
                tokio::fs::rename(f.installation.payload(), &f.installation.record.final_path)
                    .await?;
            }
            if boundary >= 4 {
                f.library
                    .storage
                    .commit_download_finalization(&f.installation.record.operation_id, &f.update)
                    .await?;
            }
            f.reopen().await?;
            f.recover().await?;
            let state = f.library.storage.work_download_state("RJ000001").await?;
            let bytes =
                tokio::fs::read(Path::new(&f.installation.record.final_path).join("content"))
                    .await?;
            if boundary < 4 {
                assert_eq!(state, f.recovered_old(), "boundary {boundary}");
                assert_eq!(bytes, b"old");
            } else {
                assert_eq!(state.completed_at.as_deref(), Some("new"));
                assert_eq!(bytes, b"new");
            }
            assert!(f
                .library
                .storage
                .download_finalization("RJ000001")
                .await?
                .is_none());
        }
        Ok(())
    }

    #[tokio::test]
    async fn cross_device_copy_and_unrelated_rename_failure_take_distinct_paths() -> Result<()> {
        let f = Fixture::new().await?;
        f.installation
            .prepare_with_rename_error(Some(io::ErrorKind::CrossesDevices))
            .await?;
        assert!(Path::new(&f.installation.record.staging_path).is_dir());
        f.installation.install().await?;
        f.library
            .storage
            .commit_download_finalization(&f.installation.record.operation_id, &f.update)
            .await?;
        f.recover().await?;
        assert!(!Path::new(&f.installation.record.staging_path).exists());
        assert_eq!(
            std::fs::read(Path::new(&f.installation.record.final_path).join("content"))?,
            b"new"
        );
        let g = Fixture::new().await?;
        assert!(g
            .installation
            .prepare_with_rename_error(Some(io::ErrorKind::PermissionDenied))
            .await
            .is_err());
        assert!(!g.installation.payload().exists());
        g.recover().await?;
        assert_eq!(
            g.library.storage.work_download_state("RJ000001").await?,
            g.recovered_old()
        );
        Ok(())
    }

    #[tokio::test]
    async fn interrupted_delete_recovers_before_removing_and_manual_mark_blocks_without_staging_authority(
    ) -> Result<()> {
        let mut f = Fixture::new().await?;
        f.installation.prepare().await?;
        tokio::fs::rename(
            f.installation.record.old_path.as_ref().unwrap(),
            f.installation.backup(),
        )
        .await?;
        f.reopen().await?;
        let marked = f
            .library
            .mark_work_downloaded(WorkDownloadMarkRequest::new(
                "RJ000001",
                &f.root.join("library"),
                Path::new(&f.installation.record.final_path),
            ))
            .await;
        assert!(matches!(marked, Err(LibraryError::RecoveryRequired(_))));
        assert_eq!(
            f.library.storage.work_download_state("RJ000001").await?,
            f.old
        );
        let state = f
            .library
            .remove_work_download(WorkDownloadRemovalRequest::new(
                "RJ000001",
                &f.root.join("library"),
                &f.root.join("staging"),
            ))
            .await?;
        assert_eq!(state.status, WorkDownloadStatus::NotDownloaded);
        assert!(!f.installation.backup().exists());
        assert!(!Path::new(&f.installation.record.final_path).exists());
        assert!(!Path::new(&f.installation.record.staging_path).exists());
        Ok(())
    }

    #[tokio::test]
    async fn restart_with_empty_missing_marker_at_initialization_and_retirement() -> Result<()> {
        for boundary in 0..3 {
            let mut f = Fixture::new().await?;
            if boundary == 0 {
                // Process stopped between create_dir and the initial marker write.
                std::fs::create_dir(f.installation.temporary())?;
            } else {
                f.installation.prepare().await?;
                if boundary == 1 {
                    // Rollback returned the payload, then removed its marker.
                    tokio::fs::rename(
                        f.installation.payload(),
                        &f.installation.record.staging_path,
                    )
                    .await?;
                } else {
                    f.installation.install().await?;
                    f.library
                        .storage
                        .commit_download_finalization(
                            &f.installation.record.operation_id,
                            &f.update,
                        )
                        .await?;
                    std::fs::remove_dir_all(f.installation.backup())?;
                }
                std::fs::remove_file(f.installation.temporary().join("owner"))?;
            }
            // Recover filesystem state, then restart before intent can be cleared.
            if boundary == 2 {
                f.installation.cleanup().await?;
            } else {
                f.installation.rollback().await?;
            }
            for _ in 0..2 {
                f.reopen().await?;
                f.recover().await?;
                let state = f.library.storage.work_download_state("RJ000001").await?;
                assert_eq!(
                    state,
                    if boundary == 2 {
                        f.update.clone().into()
                    } else {
                        f.recovered_old()
                    }
                );
                assert_eq!(
                    std::fs::read(Path::new(&f.installation.record.final_path).join("content"))?,
                    if boundary == 2 { b"new" } else { b"old" }
                );
                assert!(!f.installation.temporary().exists());
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn restart_recovers_every_strict_initial_owner_prefix() -> Result<()> {
        for prefix_len in 0..Uuid::nil().to_string().len() {
            let mut f = Fixture::new().await?;
            std::fs::create_dir(f.installation.temporary())?;
            std::fs::write(
                f.installation.temporary().join("owner"),
                &f.installation.record.operation_id[..prefix_len],
            )?;
            for _ in 0..2 {
                f.reopen().await?;
                f.recover().await?;
                assert!(!f.installation.temporary().exists(), "prefix {prefix_len}");
                assert_eq!(
                    f.library.storage.work_download_state("RJ000001").await?,
                    f.recovered_old()
                );
                assert_eq!(
                    std::fs::read(Path::new(&f.installation.record.final_path).join("content"))?,
                    b"old"
                );
                assert_eq!(
                    std::fs::read(Path::new(&f.installation.record.staging_path).join("content"))?,
                    b"new"
                );
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn missing_or_incomplete_marker_with_ambiguous_material_stays_blocked() -> Result<()> {
        for case in 0..11 {
            let mut f = Fixture::new().await?;
            std::fs::create_dir(f.installation.temporary())?;
            let marker = f.installation.temporary().join("owner");
            if [0, 1, 4, 5, 6, 10].contains(&case) {
                std::fs::write(&marker, &f.installation.record.operation_id[..8])?;
            }
            match case {
                0 => std::fs::create_dir(f.installation.payload())?,
                1 => std::fs::create_dir(f.installation.backup())?,
                2 => std::fs::write(f.installation.temporary().join("owner"), b"wrong-operation")?,
                3 => std::fs::create_dir(f.installation.payload())?,
                4 => std::fs::write(f.installation.temporary().join("unexpected"), b"retain")?,
                5 => std::fs::rename(
                    &f.installation.record.staging_path,
                    f.root.join("retained-staging"),
                )?,
                6 => std::fs::rename(
                    &f.installation.record.final_path,
                    f.root.join("retained-old"),
                )?,
                7 => std::fs::write(
                    &marker,
                    format!("{}extra", f.installation.record.operation_id),
                )?,
                8 => std::fs::create_dir(&marker)?,
                9 => std::fs::write(f.installation.temporary().join("unexpected"), b"retain")?,
                10 => {
                    // A fresh operation requires an absent final path before installation.
                    f.library
                        .storage
                        .clear_download_finalization(
                            "RJ000001",
                            &f.installation.record.operation_id,
                        )
                        .await?;
                    f.installation.record.old_path = None;
                    f.library
                        .storage
                        .begin_download_finalization(&f.installation.record)
                        .await?;
                }
                _ => unreachable!(),
            }
            for _ in 0..2 {
                f.reopen().await?;
                assert!(matches!(
                    f.recover().await,
                    Err(LibraryError::RecoveryRequired(_))
                ));
                assert!(f.installation.temporary().is_dir());
                if [0, 1, 4, 5, 6, 10].contains(&case) {
                    assert_eq!(
                        std::fs::read_to_string(&marker)?,
                        &f.installation.record.operation_id[..8]
                    );
                }
                assert!(f
                    .library
                    .storage
                    .download_finalization("RJ000001")
                    .await?
                    .is_some());
                assert_eq!(
                    f.library.storage.work_download_state("RJ000001").await?,
                    f.old
                );
                let old = if case == 6 {
                    f.root.join("retained-old")
                } else {
                    PathBuf::from(&f.installation.record.final_path)
                };
                assert_eq!(std::fs::read(old.join("content"))?, b"old");
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn committed_incomplete_owner_is_never_treated_as_initialization() -> Result<()> {
        for prefix_len in [0, 8, 35] {
            let mut f = Fixture::new().await?;
            f.installation.prepare().await?;
            f.installation.install().await?;
            f.library
                .storage
                .commit_download_finalization(&f.installation.record.operation_id, &f.update)
                .await?;
            let marker = f.installation.temporary().join("owner");
            std::fs::write(&marker, &f.installation.record.operation_id[..prefix_len])?;
            for _ in 0..2 {
                f.reopen().await?;
                assert!(matches!(
                    f.recover().await,
                    Err(LibraryError::RecoveryRequired(_))
                ));
                assert_eq!(
                    std::fs::read_to_string(&marker)?,
                    &f.installation.record.operation_id[..prefix_len]
                );
                assert_eq!(
                    std::fs::read(f.installation.backup().join("content"))?,
                    b"old"
                );
                assert_eq!(
                    std::fs::read(Path::new(&f.installation.record.final_path).join("content"))?,
                    b"new"
                );
                assert!(
                    f.library
                        .storage
                        .download_finalization("RJ000001")
                        .await?
                        .unwrap()
                        .committed
                );
                assert_eq!(
                    f.library.storage.work_download_state("RJ000001").await?,
                    f.update.clone().into()
                );
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn incomplete_owner_symlink_is_retained() -> Result<()> {
        let mut f = Fixture::new().await?;
        std::fs::create_dir(f.installation.temporary())?;
        let target = f.root.join("external-marker");
        let marker = f.installation.temporary().join("owner");
        std::fs::write(&target, &f.installation.record.operation_id[..8])?;
        std::os::unix::fs::symlink(&target, &marker)?;
        for _ in 0..2 {
            f.reopen().await?;
            assert!(matches!(
                f.recover().await,
                Err(LibraryError::RecoveryRequired(_))
            ));
            assert!(std::fs::symlink_metadata(&marker)?.file_type().is_symlink());
            assert_eq!(
                std::fs::read_to_string(&target)?,
                &f.installation.record.operation_id[..8]
            );
            assert_eq!(
                f.library.storage.work_download_state("RJ000001").await?,
                f.old
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn normal_rollback_then_restart_and_delete_removes_restored_staging() -> Result<()> {
        let mut f = Fixture::new().await?;
        assert!(f.old.staging_path.is_none());
        f.installation.prepare().await?;
        f.installation.install().await?;
        f.recover().await?;
        f.reopen().await?;
        f.library
            .remove_work_download(WorkDownloadRemovalRequest::new(
                "RJ000001",
                &f.root.join("library"),
                &f.root.join("staging"),
            ))
            .await?;
        assert!(!Path::new(&f.installation.record.staging_path).exists());
        assert!(!Path::new(&f.installation.record.final_path).exists());
        assert!(f
            .library
            .storage
            .download_finalization("RJ000001")
            .await?
            .is_none());
        assert_eq!(
            f.library
                .storage
                .work_download_state("RJ000001")
                .await?
                .status,
            WorkDownloadStatus::NotDownloaded
        );
        Ok(())
    }

    #[tokio::test]
    async fn recovery_keeps_intent_until_restored_staging_is_durably_tracked() -> Result<()> {
        for removing in [false, true] {
            for boundary in 0..4 {
                let mut f = Fixture::new().await?;
                assert!(f.old.staging_path.is_none());
                f.installation.prepare().await?;
                f.installation.install().await?;
                if boundary < 2 {
                    let mut tx = f.library.storage.begin_write().await?;
                    tx.execute(if boundary == 0 {
                    "CREATE TRIGGER reject_handoff BEFORE UPDATE ON work_downloads BEGIN SELECT RAISE(ABORT, 'injected tracking failure'); END"
                } else {
                    "CREATE TRIGGER reject_handoff BEFORE DELETE ON download_finalizations BEGIN SELECT RAISE(ABORT, 'injected intent clearing failure'); END"
                }).await?;
                    tx.commit().await?;
                }
                f.reopen().await?;
                let roots: [&Path; 2] = [&f.root.join("library"), &f.root.join("staging")];
                // Stop after the handoff, before deleting either tracked content path.
                let result = f
                    .library
                    .recover_download_inner("RJ000001", &roots, removing)
                    .await;
                assert_eq!(result.is_err(), boundary < 2);
                let mut expected = f.old.clone();
                if boundary > 0 {
                    expected.staging_path = Some(f.installation.record.staging_path.clone());
                }
                assert_eq!(
                    f.library.storage.work_download_state("RJ000001").await?,
                    expected
                );
                assert_eq!(
                    f.library
                        .storage
                        .download_finalization("RJ000001")
                        .await?
                        .is_some(),
                    boundary < 2
                );
                assert_eq!(
                    std::fs::read(Path::new(&f.installation.record.staging_path).join("content"))?,
                    b"new"
                );
                assert_eq!(
                    std::fs::read(Path::new(&f.installation.record.final_path).join("content"))?,
                    b"old"
                );
                if boundary == 3 {
                    // Delete stopped after removing local content but before staging.
                    std::fs::remove_dir_all(&f.installation.record.final_path)?;
                }

                f.reopen().await?;
                if boundary < 2 {
                    // Repeated failed recovery must remain safe with the original intent.
                    assert!(f
                        .library
                        .recover_download_inner("RJ000001", &roots, removing)
                        .await
                        .is_err());
                    let mut tx = f.library.storage.begin_write().await?;
                    tx.execute("DROP TRIGGER reject_handoff").await?;
                    tx.commit().await?;
                }
                for _ in 0..2 {
                    f.reopen().await?;
                    let state = f
                        .library
                        .remove_work_download(WorkDownloadRemovalRequest::new(
                            "RJ000001", roots[0], roots[1],
                        ))
                        .await?;
                    assert_eq!(state.status, WorkDownloadStatus::NotDownloaded);
                    assert!(!Path::new(&f.installation.record.staging_path).exists());
                    assert!(!Path::new(&f.installation.record.final_path).exists());
                    assert!(f
                        .library
                        .storage
                        .download_finalization("RJ000001")
                        .await?
                        .is_none());
                }
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn removal_handoff_accounts_for_a_different_previously_tracked_staging_path() -> Result<()>
    {
        let mut f = Fixture::new().await?;
        let previous_staging = f.root.join("staging/previous");
        std::fs::create_dir(&previous_staging)?;
        std::fs::write(previous_staging.join("content"), b"previous staging")?;
        let mut previous_update = f.update.clone();
        previous_update.staging_path = Some(previous_staging.to_string_lossy().into_owned());
        f.library
            .storage
            .save_work_download(&previous_update)
            .await?;
        f.installation.prepare().await?;
        f.installation.install().await?;
        let mut tx = f.library.storage.begin_write().await?;
        tx.execute("CREATE TRIGGER reject_tracking BEFORE UPDATE ON work_downloads BEGIN SELECT RAISE(ABORT, 'injected tracking failure'); END").await?;
        tx.commit().await?;
        f.reopen().await?;
        let roots: [&Path; 2] = [&f.root.join("library"), &f.root.join("staging")];
        for _ in 0..2 {
            assert!(matches!(
                f.recover().await,
                Err(LibraryError::RecoveryRequired(_))
            ));
            assert_eq!(
                std::fs::read(previous_staging.join("content"))?,
                b"previous staging"
            );
            assert_eq!(
                f.library.storage.work_download_state("RJ000001").await?,
                previous_update.clone().into()
            );
            assert!(f
                .library
                .storage
                .download_finalization("RJ000001")
                .await?
                .is_some());
            f.reopen().await?;
        }
        assert!(f
            .library
            .recover_download_inner("RJ000001", &roots, true)
            .await
            .is_err());
        assert!(!previous_staging.exists());
        assert_eq!(
            f.library.storage.work_download_state("RJ000001").await?,
            previous_update.into()
        );
        assert!(f
            .library
            .storage
            .download_finalization("RJ000001")
            .await?
            .is_some());
        f.reopen().await?;
        let mut tx = f.library.storage.begin_write().await?;
        tx.execute("DROP TRIGGER reject_tracking").await?;
        tx.commit().await?;
        f.library
            .recover_download_inner("RJ000001", &roots, true)
            .await?;
        assert!(!previous_staging.exists());
        f.reopen().await?;
        f.library
            .remove_work_download(WorkDownloadRemovalRequest::new(
                "RJ000001", roots[0], roots[1],
            ))
            .await?;
        assert!(!Path::new(&f.installation.record.staging_path).exists());
        assert!(!Path::new(&f.installation.record.final_path).exists());
        Ok(())
    }

    #[tokio::test]
    async fn ambiguous_recovery_blocks_delete_and_scan_without_changing_the_row() -> Result<()> {
        let mut f = Fixture::new().await?;
        f.installation.prepare().await?;
        std::fs::write(
            f.installation.temporary().join("owner"),
            b"not-this-operation",
        )?;
        f.reopen().await?;
        assert!(f
            .library
            .remove_work_download(WorkDownloadRemovalRequest::new(
                "RJ000001",
                &f.root.join("library"),
                &f.root.join("staging")
            ))
            .await
            .is_err());
        let report = f
            .library
            .import_local_work_downloads(LocalWorkImportRequest::new(&f.root.join("library")))
            .await?;
        assert_eq!(report.imported_count, 0);
        assert_eq!(report.recovery_errors.len(), 1);
        assert_eq!(
            f.library.storage.work_download_state("RJ000001").await?,
            f.old
        );
        assert_eq!(
            std::fs::read(Path::new(&f.installation.record.final_path).join("content"))?,
            b"old"
        );
        Ok(())
    }

    #[tokio::test]
    async fn cleanup_failure_keeps_committed_content_and_recovery_record() -> Result<()> {
        let f = Fixture::new().await?;
        f.installation.prepare().await?;
        f.installation.install().await?;
        f.library
            .storage
            .commit_download_finalization(&f.installation.record.operation_id, &f.update)
            .await?;
        std::fs::write(f.installation.temporary().join("unexpected"), b"retain")?;
        assert!(f.recover().await.is_err());
        assert!(
            f.library
                .storage
                .download_finalization("RJ000001")
                .await?
                .unwrap()
                .committed
        );
        assert_eq!(
            f.library
                .storage
                .work_download_state("RJ000001")
                .await?
                .completed_at
                .as_deref(),
            Some("new")
        );
        assert_eq!(
            std::fs::read(Path::new(&f.installation.record.final_path).join("content"))?,
            b"new"
        );
        assert!(f.installation.backup().exists());
        Ok(())
    }

    #[tokio::test]
    async fn wrong_commit_identity_rolls_back_the_entire_sql_transaction() -> Result<()> {
        let f = Fixture::new().await?;
        assert!(f
            .library
            .storage
            .commit_download_finalization("wrong-id", &f.update)
            .await
            .is_err());
        assert_eq!(
            f.library.storage.work_download_state("RJ000001").await?,
            f.old
        );
        assert!(
            !f.library
                .storage
                .download_finalization("RJ000001")
                .await?
                .unwrap()
                .committed
        );
        Ok(())
    }

    #[test]
    fn overlapping_and_traversing_roots_are_rejected() {
        let root =
            std::env::temp_dir().join(format!("dm-finalization-path-test-{}", Uuid::new_v4()));
        assert!(paths("../RJ000001", &root, &root).is_err());
        assert!(paths("RJ000001", &root, &root).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn same_work_mutation_waits_for_the_core_guard() -> Result<()> {
        let f = Fixture::new().await?;
        let lock = f.library.work_lock("RJ000001");
        let guard = lock.lock().await;
        let library_root = f.root.join("library");
        let staging_root = f.root.join("staging");
        let mut remove = Box::pin(
            f.library
                .remove_work_download(WorkDownloadRemovalRequest::new(
                    "RJ000001",
                    &library_root,
                    &staging_root,
                )),
        );
        assert!(tokio::time::timeout(Duration::from_millis(10), &mut remove)
            .await
            .is_err());
        assert_eq!(
            f.library.storage.work_download_state("RJ000001").await?,
            f.old
        );
        drop(guard);
        assert_eq!(remove.await?.status, WorkDownloadStatus::NotDownloaded);
        Ok(())
    }

    #[tokio::test]
    async fn install_collision_retains_backup_until_explicit_recovery() -> Result<()> {
        let f = Fixture::new().await?;
        f.installation.prepare().await?;
        // Simulate rename failure by occupying the backup target before the swap.
        std::fs::write(f.installation.backup(), b"unexpected")?;
        assert!(f.installation.install().await.is_err());
        assert_eq!(
            std::fs::read(Path::new(&f.installation.record.final_path).join("content"))?,
            b"old"
        );
        assert!(f.recover().await.is_err());
        assert_eq!(
            f.library.storage.work_download_state("RJ000001").await?,
            f.old
        );
        assert!(f.installation.payload().exists());
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unsupported_copy_entry_fails_without_touching_old_content() -> Result<()> {
        let f = Fixture::new().await?;
        std::os::unix::fs::symlink(
            "content",
            Path::new(&f.installation.record.staging_path).join("link"),
        )?;
        assert!(f
            .installation
            .prepare_with_rename_error(Some(io::ErrorKind::CrossesDevices))
            .await
            .is_err());
        f.recover().await?;
        assert_eq!(
            f.library.storage.work_download_state("RJ000001").await?,
            f.recovered_old()
        );
        assert_eq!(
            std::fs::read(Path::new(&f.installation.record.final_path).join("content"))?,
            b"old"
        );
        assert!(f.installation.payload().exists()); // retained partial copy, never installed
        Ok(())
    }
}
