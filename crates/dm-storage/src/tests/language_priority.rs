use super::*;

#[tokio::test]
async fn patch_preserves_omitted_priority_and_repairs_invalid_stored_value_atomically() -> Result<()>
{
    let dir = tempfile::tempdir().expect("temporary settings database");
    let database = dir.path().join("settings.sqlite");
    let storage = Storage::open(&database).await?;
    storage.run_migrations().await?;
    assert_eq!(
        storage.app_settings().await?.language_priority.codes(),
        &DEFAULT_LANGUAGE_PRIORITY
    );
    let priority = LanguagePriority::new(vec!["ja_JP".into(), "pt_BR".into()])?;
    storage
        .save_app_settings_patch(&AppSettingsPatch {
            library_root: Some("/library".into()),
            download_root: Some("/downloads".into()),
            language_priority: Some(priority.clone()),
        })
        .await?;
    let saved = storage
        .save_app_settings_patch(&AppSettingsPatch {
            library_root: Some("/new-library".into()),
            download_root: None,
            language_priority: None,
        })
        .await?;
    assert_eq!(saved.language_priority, priority);
    assert_eq!(storage.app_settings().await?, saved);
    assert!(LanguagePriority::new(vec!["ja_JP".into(), "ja_JP".into()]).is_err());
    assert!(LanguagePriority::new(vec![" ".into()]).is_err());
    assert_eq!(storage.app_settings().await?, saved);

    sqlx::query("UPDATE app_settings SET value = 'not JSON' WHERE key = ?")
        .bind(LANGUAGE_PRIORITY_KEY)
        .execute(&storage.pool)
        .await?;
    assert!(matches!(
        storage.app_settings().await,
        Err(StorageError::InvalidStoredValue { .. })
    ));
    let result = storage
        .save_app_settings_patch(&AppSettingsPatch {
            library_root: Some("/must-not-be-written".into()),
            ..Default::default()
        })
        .await;
    assert!(matches!(
        result,
        Err(StorageError::InvalidStoredValue { .. })
    ));
    let root: String = sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(LIBRARY_ROOT_KEY)
        .fetch_one(&storage.pool)
        .await?;
    assert_eq!(root, "/new-library");
    let repaired = storage
        .save_app_settings_patch(&AppSettingsPatch {
            library_root: Some(root),
            language_priority: Some(priority),
            ..Default::default()
        })
        .await?;
    assert_eq!(storage.app_settings().await?, repaired);
    storage.pool.close().await;
    let reopened = Storage::open(&database).await?;
    reopened.run_migrations().await?;
    assert_eq!(reopened.app_settings().await?, repaired);
    Ok(())
}

#[tokio::test]
async fn priority_changes_display_sort_and_facets_without_changing_search_or_maker_identity(
) -> Result<()> {
    let storage = migrated_storage().await?;
    storage
        .save_account(&account("account-a", "Account A"))
        .await?;
    let works = vec![
        CachedWork {
            title_json:
                r#"{"en_US":"Zulu","ja_JP":"Alpha","pt_BR":"Primeiro","ko_KR":"검색 이름"}"#.into(),
            maker_id: Some(" RG1 ".into()),
            maker_json: Some(
                r#"{"en_US":"Circle One","ja_JP":"制作元一","pt_BR":"Círculo"}"#.into(),
            ),
            ..work("RJ000001", "unused", "unused", "2026-01-01")
        },
        CachedWork {
            title_json: r#"{"en_US":"Alpha","ja_JP":"Zulu"}"#.into(),
            maker_id: Some("RG1".into()),
            maker_json: Some(r#"{"en_US":"Circle One","ja_JP":"制作元一"}"#.into()),
            ..work("RJ000002", "unused", "unused", "2026-01-01")
        },
        CachedWork {
            title_json: r#"{"en_US":"Middle","aa_AA":"Unlisted title"}"#.into(),
            maker_id: Some("  ".into()),
            maker_json: Some(r#"{"en_US":"Fallback Circle","ja_JP":"別の制作元"}"#.into()),
            ..work("RJ000003", "unused", "unused", "2026-01-01")
        },
        CachedWork {
            title_json: "broken".into(),
            maker_id: None,
            maker_json: Some("[]".into()),
            ..work("RJ000004", "unused", "unused", "2026-01-01")
        },
    ];
    let owned = works
        .iter()
        .map(|w| account_work(&w.work_id, "2026-01-01"))
        .collect();
    storage
        .commit_account_sync(&sync_commit("account-a", "sync-a", works, owned))
        .await?;
    let mut query = ProductListQuery {
        sort: ProductSort::TitleAsc,
        limit: 1,
        ..Default::default()
    };
    assert_eq!(
        storage.list_products(&query).await?.products[0].work_id,
        "RJ000002"
    );
    let english_facets = storage.product_filter_facets(&query).await?;
    assert_eq!(english_facets.makers[0].key, "id:RG1");
    assert_eq!(english_facets.makers[0].count, 2);
    assert_eq!(english_facets.makers[1].key, "name:Fallback Circle");
    query.language_priority = LanguagePriority::new(vec!["ja_JP".into()])?;
    assert_eq!(
        storage.list_products(&query).await?.products[0].work_id,
        "RJ000001"
    );
    query.offset = 1;
    assert_eq!(
        storage.list_products(&query).await?.products[0].work_id,
        "RJ000004"
    );
    let japanese_facets = storage.product_filter_facets(&query).await?;
    assert_eq!(japanese_facets.makers[0].key, "id:RG1");
    assert_eq!(japanese_facets.makers[0].name, "制作元一");
    assert_eq!(japanese_facets.makers[1].key, "name:Fallback Circle");
    assert_eq!(japanese_facets.makers[1].name, "別の制作元");
    query.maker_keys = vec!["id:RG1".into()];
    query.offset = 0;
    query.limit = 100;
    assert_eq!(storage.list_products(&query).await?.total_count, 2);
    query.language_priority = LanguagePriority::new(vec!["pt_BR".into()])?;
    let detail = storage
        .product_detail("RJ000001", &query.language_priority)
        .await?;
    assert_eq!(detail.title, "Primeiro");
    assert_eq!(detail.maker_name.as_deref(), Some("Círculo"));
    assert_eq!(
        storage.product_filter_facets(&query).await?.makers[0].name,
        "Círculo"
    );
    assert_eq!(storage.list_products(&query).await?.total_count, 2);
    for search in ["검색 이름", "制作元一", "Zulu"] {
        query.search = Some(search.into());
        assert!(storage.list_products(&query).await?.total_count > 0);
    }
    assert_eq!(
        storage.available_name_languages().await?,
        vec!["aa_AA", "en_US", "ja_JP", "ko_KR", "pt_BR"]
    );
    Ok(())
}

#[tokio::test]
async fn large_unknown_priority_uses_one_json_parameter_and_unlisted_code_fallback() -> Result<()> {
    let storage = migrated_storage().await?;
    storage
        .save_account(&account("account-a", "Account A"))
        .await?;
    storage
        .commit_account_sync(&sync_commit(
            "account-a",
            "sync-a",
            vec![CachedWork {
                title_json:
                    r#"{"zz_ZZ":"Last fallback","aa_AA":"First fallback","pt_BR":"Prioritized"}"#
                        .into(),
                maker_json: None,
                ..work("RJ000001", "unused", "unused", "2026-01-01")
            }],
            vec![account_work("RJ000001", "2026-01-01")],
        ))
        .await?;
    let mut codes: Vec<_> = (0..1500).map(|i| format!("unknown_{i:04}")).collect();
    codes.push("pt_BR".into());
    codes.push("long".repeat(300));
    let priority = LanguagePriority::new(codes)?;
    let saved = storage
        .save_app_settings_patch(&AppSettingsPatch {
            language_priority: Some(priority.clone()),
            ..Default::default()
        })
        .await?;
    assert_eq!(saved, storage.app_settings().await?);
    let query = ProductListQuery {
        language_priority: priority,
        ..Default::default()
    };
    assert_eq!(
        storage.list_products(&query).await?.products[0].title,
        "Prioritized"
    );
    assert_eq!(
        storage
            .product_detail("RJ000001", &LanguagePriority::new(vec![])?)
            .await?
            .title,
        "First fallback"
    );
    assert_eq!(
        storage.available_name_languages().await?,
        vec!["aa_AA", "pt_BR", "zz_ZZ"]
    );
    Ok(())
}
