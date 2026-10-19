use super::*;
use dm_storage::{AppSettingsPatch, LanguagePriority};

#[tokio::test]
async fn paginated_candidates_and_bulk_selection_keep_one_priority_snapshot() -> Result<()> {
    let library = migrated_library().await?;
    library.save_account(save_account_request(false)).await?;
    let count = BULK_DOWNLOAD_PAGE_LIMIT + 5;
    let works = (0..count)
        .map(|i| {
            let mut source = work(
                &format!("RJ{i:06}"),
                "unused",
                "Circle",
                "2026-01-01T00:00:00Z",
            );
            source.name = BTreeMap::from([
                (Language::English, format!("A{i:04}")),
                (Language::Japanese, format!("Z{:04}", count - i)),
                (Language::Unknown("pt_BR".into()), format!("P{i:04}")),
            ]);
            cached_work_from_api(source, "2026-10-19")
        })
        .collect::<Result<Vec<_>>>()?;
    let owned = works
        .iter()
        .map(|w| AccountWork {
            work_id: w.work_id.clone(),
            purchased_at: None,
        })
        .collect();
    library
        .storage
        .commit_account_sync(&AccountSyncCommit {
            sync_run_id: "names-sync".into(),
            account_id: "account-a".into(),
            started_at: "2026-10-19".into(),
            completed_at: "2026-10-19".into(),
            works,
            account_works: owned,
        })
        .await?;
    let query = ProductListQuery {
        sort: ProductSort::TitleAsc,
        maker_keys: vec!["id:maker-Circle".into()],
        limit: 1,
        ..Default::default()
    };
    let resolved = library.resolved_query(&query).await?;
    let mut pages = BulkProductPages::new(resolved);
    let first = pages.next(&library.storage).await?.expect("first page");
    assert_eq!(first.products.len(), BULK_DOWNLOAD_PAGE_LIMIT as usize);
    assert_eq!(first.products[0].work_id, "RJ000000");
    library
        .storage
        .save_app_settings_patch(&AppSettingsPatch {
            language_priority: Some(LanguagePriority::new(vec!["ja_JP".into()])?),
            ..Default::default()
        })
        .await?;
    let second = pages.next(&library.storage).await?.expect("second page");
    assert_eq!(second.products.len(), 5);
    assert_eq!(second.products[0].work_id, "RJ000500");
    assert!(pages.next(&library.storage).await?.is_none());
    let ids: BTreeSet<_> = first
        .products
        .iter()
        .chain(&second.products)
        .map(|p| &p.work_id)
        .collect();
    assert_eq!(ids.len(), count as usize);
    assert_eq!(
        library.list_products(&query).await?.products[0].work_id,
        "RJ000504"
    );

    let candidates = library.select_bulk_candidates(&query, false).await?;
    let selection = library
        .bulk_download_selection(&query, false, None, None)
        .await?;
    assert_eq!(candidates.total_count, u64::from(count));
    assert_eq!(candidates.work_ids.len(), count as usize);
    assert_eq!(
        candidates.work_ids,
        selection
            .items
            .iter()
            .map(|item| item.work_id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        library.product_filter_facets(&query).await?.makers[0].count,
        u64::from(count)
    );
    library
        .storage
        .save_app_settings_patch(&AppSettingsPatch {
            language_priority: Some(LanguagePriority::new(vec!["pt_BR".into()])?),
            ..Default::default()
        })
        .await?;
    assert_eq!(library.product_detail("RJ000000").await?.title, "P0000");
    assert_eq!(
        library.available_name_languages().await?,
        vec!["en_US", "ja_JP", "pt_BR"]
    );
    Ok(())
}
