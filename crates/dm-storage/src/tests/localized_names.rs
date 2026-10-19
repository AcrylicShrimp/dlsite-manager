use super::*;

#[tokio::test]
async fn upgrades_legacy_names_and_preserves_related_records() -> Result<()> {
    let dir = tempfile::tempdir().expect("temporary database directory");
    let source = dir.path().join("legacy.sqlite");
    let copy = dir.path().join("upgrade.sqlite");
    let legacy = Storage::open(&source).await?;
    MIGRATOR.run_to(20260905000000, &legacy.pool).await?;
    legacy
        .save_account(&account("account-a", "Account A"))
        .await?;
    let variants = r#"{"en_US":"English title","ja_JP":"日本語の作品","ko_KR":"한국어 작품","pt_BR":"Título"}"#;
    for (id, title, json, maker, maker_json) in [
        (
            "RJ000001",
            "Conflicting old title",
            variants,
            Some("Old maker"),
            Some(r#"{"ja_JP":"サークル","en_US":"Circle"}"#),
        ),
        (
            "RJ000002",
            "Legacy title",
            "{invalid",
            Some("Legacy maker"),
            Some("[]"),
        ),
        (
            "RJ000003",
            "Empty map title",
            r#"{"en_US":"","bad":42}"#,
            None,
            None,
        ),
        ("RJ000004", "", "{}", None, None),
    ] {
        sqlx::query("INSERT INTO works (work_id, title, title_json, maker_name, maker_json, raw_json, last_detail_sync_at) VALUES (?, ?, ?, ?, ?, '{}', '2026-10-19T00:00:00Z')")
            .bind(id).bind(title).bind(json).bind(maker).bind(maker_json)
            .execute(&legacy.pool).await?;
    }
    sqlx::query("INSERT INTO account_works (account_id, work_id, first_seen_at, last_seen_at) VALUES ('account-a', 'RJ000001', '2026-01-01', '2026-01-01')")
        .execute(&legacy.pool).await?;
    sqlx::query("INSERT INTO work_custom_tags (work_id, name, normalized_name) VALUES ('RJ000001', 'Keep', 'keep')")
        .execute(&legacy.pool).await?;
    sqlx::query("INSERT INTO work_downloads (work_id, status, unpack_policy, bytes_received, updated_at) VALUES ('RJ000001', 'downloaded', 'manual', 100, '2026-01-01')")
        .execute(&legacy.pool).await?;
    sqlx::query("INSERT INTO download_finalizations (work_id, operation_id, staging_path, final_path, temporary_path) VALUES ('RJ000001', 'op-a', '/staging', '/final', '/temporary')")
        .execute(&legacy.pool).await?;
    legacy.pool.close().await;
    std::fs::copy(&source, &copy).expect("copy disposable pre-change database");

    let storage = Storage::open(&copy).await?;
    storage.run_migrations().await?;
    let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('works')")
        .fetch_all(&storage.pool)
        .await?;
    assert!(!columns
        .iter()
        .any(|name| name == "title" || name == "maker_name"));
    let indexes: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'works'",
    )
    .fetch_all(&storage.pool)
    .await?;
    assert!(!indexes
        .iter()
        .any(|name| name == "works_title_idx" || name == "works_maker_name_idx"));
    let retained: String =
        sqlx::query_scalar("SELECT title_json FROM works WHERE work_id = 'RJ000001'")
            .fetch_one(&storage.pool)
            .await?;
    assert_eq!(retained, variants);
    let detail = storage
        .product_detail("RJ000001", &LanguagePriority::default())
        .await?;
    assert_eq!(detail.title, "English title");
    assert_eq!(detail.title_variants.len(), 4);
    assert_eq!(detail.custom_tags[0].name, "Keep");
    assert_eq!(detail.download.status, WorkDownloadStatus::Downloaded);
    assert_eq!(detail.owners[0].account_id, "account-a");
    assert!(storage.download_finalization("RJ000001").await?.is_some());
    let repaired = storage
        .product_detail("RJ000002", &LanguagePriority::default())
        .await?;
    assert_eq!(
        repaired.title_variants,
        vec![ProductTextValue {
            language: "und".into(),
            value: "Legacy title".into()
        }]
    );
    assert_eq!(repaired.maker_names[0].language, "und");
    assert_eq!(
        storage
            .product_detail("RJ000003", &LanguagePriority::default())
            .await?
            .title,
        "Empty map title"
    );
    assert_eq!(
        storage
            .product_detail("RJ000004", &LanguagePriority::default())
            .await?
            .title,
        "RJ000004"
    );
    assert!(sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&storage.pool)
        .await?
        .is_empty());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM works")
        .fetch_one(&storage.pool)
        .await?;
    assert_eq!(count, 4);
    let sqlite_version: String = sqlx::query_scalar("SELECT sqlite_version()")
        .fetch_one(&storage.pool)
        .await?;
    eprintln!("upgraded disposable database using bundled SQLite {sqlite_version}");
    Ok(())
}

#[tokio::test]
async fn searches_all_name_values_and_selects_before_pagination() -> Result<()> {
    let storage = migrated_storage().await?;
    storage
        .save_account(&account("account-a", "Account A"))
        .await?;
    let first = CachedWork {
        title_json: r#"{"en_US":"Zulu","ja_JP":"日本語","ko_KR":"한국어","zh_CN":"中文","pt_BR":"Português"}"#.into(),
        maker_json: Some(r#"{"en_US":"Circle","ko_KR":"한국 제작자","zz_ZZ":"Unknown maker"}"#.into()),
        ..work("RJ000001", "ignored", "ignored", "2026-01-01")
    };
    let second = CachedWork {
        title_json: r#"{"en_US":"  ","ja_JP":"Alpha","bad":42}"#.into(),
        maker_json: Some("not json".into()),
        ..work("RJ000002", "ignored", "ignored", "2026-01-01")
    };
    storage
        .commit_account_sync(&sync_commit(
            "account-a",
            "sync-a",
            vec![first, second],
            vec![
                account_work("RJ000001", "2026-01-01"),
                account_work("RJ000002", "2026-01-01"),
            ],
        ))
        .await?;
    for search in [
        "日本語",
        "한국어",
        "中文",
        "Português",
        "한국 제작자",
        "Unknown maker",
    ] {
        let page = storage
            .list_products(&ProductListQuery {
                search: Some(search.into()),
                ..Default::default()
            })
            .await?;
        assert_eq!(page.total_count, 1, "search {search}");
        assert_eq!(page.products[0].work_id, "RJ000001");
    }
    for (offset, id, title) in [(0, "RJ000002", "Alpha"), (1, "RJ000001", "Zulu")] {
        let page = storage
            .list_products(&ProductListQuery {
                sort: ProductSort::TitleAsc,
                limit: 1,
                offset,
                ..Default::default()
            })
            .await?;
        assert_eq!(page.total_count, 2);
        assert_eq!(page.products[0].work_id, id);
        assert_eq!(page.products[0].title, title);
    }
    let detail = storage
        .product_detail("RJ000002", &LanguagePriority::default())
        .await?;
    assert_eq!(detail.title, "Alpha");
    assert!(detail.maker_name.is_none());
    assert_eq!(detail.title_variants.len(), 1);
    Ok(())
}
