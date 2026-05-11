# dlsite-manager

Desktop manager for DLsite works and local libraries.

`dlsite-manager` v3 is a fresh Tauri 2 rewrite focused on DLsite Play v3 syncing, a unified multi-account library, resumable downloads, local-only work management, custom tags, and support-friendly audit logs.

This is an unofficial project and is not affiliated with DLsite.

![Library view](docs/assets/readme/library.png)

## Download

Get the latest build from the [GitHub releases page](https://github.com/AcrylicShrimp/dlsite-manager/releases/latest).

The current published binary is a macOS Apple Silicon DMG. Other platforms can be built from source until additional packaging is added.

## Features

- Sync purchased works through the DLsite Play v3 API.
- Manage multiple DLsite accounts as one unified library.
- Search and filter by title, work ID, maker, credits, account source, ownership source, age class, work type, and custom tags.
- Add custom tags and include or exclude them in Library filters.
- Scan existing local folders and track local-only / not-owned works.
- Download direct archives, legacy split archives, and serial-required products.
- Resume interrupted downloads from staging files.
- Cancel active downloads and inspect current download jobs from a dedicated Downloads page.
- Open downloaded works, product folders, DLsite product pages, and support log folders from the app.
- Record app operations and detailed failures in file-backed audit logs.

## Screenshots

| Product detail | Downloads |
| --- | --- |
| ![Product detail panel](docs/assets/readme/product-detail.png) | ![Downloads page](docs/assets/readme/downloads.png) |

| Accounts | Settings |
| --- | --- |
| ![Accounts page](docs/assets/readme/accounts.png) | ![Settings page](docs/assets/readme/settings.png) |

![Activity and audit log page](docs/assets/readme/activity.png)

## Basic Usage

1. Open **Accounts** and add a DLsite account.
2. Run **Sync** to cache purchased works.
3. Use **Library** to search, filter, tag, inspect, download, or open works.
4. Use **Downloads** to watch active downloads and cancel running jobs.
5. Use **Settings** to choose the final library folder and download staging folder.
6. Use **Activity** when you need recent job history or audit logs for troubleshooting.

## Storage And Credentials

The app stores library metadata in SQLite under the application data directory. Downloaded works are stored in the configured library folder, while partial downloads and fetched archives use the configured staging folder so interrupted downloads can resume.

Saved account credentials are kept in the app credential store, separate from the SQLite database. Do not share passwords, cookies, or serial numbers in bug reports.

Audit logs are written to the app log directory. When reporting a bug, include the app version, affected work ID, steps to reproduce, and relevant audit log entries.

## Supported Download Paths

The v3 download path currently targets DLsite archive downloads:

- normal single-archive products
- serial-required products
- legacy split archives
- downloaded folder opening and deletion
- re-download with confirmation
- local folder import / manual downloaded-state marking

Browser-reader-only workflows, including manga download behavior, still need separate research and are not part of the current v3 release.

## Development

Prerequisites:

- Rust stable
- Node.js and pnpm
- Tauri 2 system prerequisites for your platform

Common commands:

```sh
pnpm install
pnpm check
cargo test --workspace
pnpm tauri dev
pnpm tauri build
```

Live DLsite API/download tests are env-gated. Use the `.env.example` files in the relevant crates when validating those surfaces:

- `crates/dm-api/.env.example`
- `crates/dm-download/.env.example`
- `crates/dm-archive/.env.example`

## Architecture

The app keeps DLsite/domain behavior in reusable Rust crates and leaves the Tauri layer as a thin adapter.

- `crates/dm-api` - DLsite Play v3 API, authentication, metadata, download-plan discovery, authenticated byte streams
- `crates/dm-download` - resumable file downloads and progress reporting
- `crates/dm-archive` - archive classification and extraction
- `crates/dm-storage` - SQLite persistence and SQLx migrations
- `crates/dm-library` - application services that combine API, storage, credentials, downloads, and local-library behavior
- `crates/dm-jobs` - background job snapshots, progress, and cancellation
- `crates/dm-audit` - file-backed audit log records
- `crates/dm-credentials` - app credential storage
- `src-tauri` - Tauri commands, events, and desktop integration
- `src/routes` - Svelte UI

## License

MIT
