<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type AppSettings = {
    libraryRoot: string | null;
    downloadRoot: string | null;
  };

  let libraryRoot = $state("");
  let downloadRoot = $state("");
  let loading = $state(true);
  let saving = $state(false);
  let status = $state("");
  let error = $state("");

  onMount(() => {
    void loadSettings();
  });

  async function loadSettings() {
    loading = true;
    error = "";

    try {
      const settings = await invoke<AppSettings>("get_settings");
      libraryRoot = settings.libraryRoot ?? "";
      downloadRoot = settings.downloadRoot ?? "";
      status = "";
    } catch (err) {
      error = errorMessage(err);
    } finally {
      loading = false;
    }
  }

  async function saveSettings(event: Event) {
    event.preventDefault();
    saving = true;
    error = "";
    status = "";

    try {
      const settings = await invoke<AppSettings>("save_settings", {
        settings: {
          libraryRoot: valueOrNull(libraryRoot),
          downloadRoot: valueOrNull(downloadRoot),
        },
      });
      libraryRoot = settings.libraryRoot ?? "";
      downloadRoot = settings.downloadRoot ?? "";
      status = "Saved";
    } catch (err) {
      error = errorMessage(err);
    } finally {
      saving = false;
    }
  }

  function valueOrNull(value: string) {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
  }

  function errorMessage(err: unknown) {
    return err instanceof Error ? err.message : String(err);
  }
</script>

<svelte:head>
  <title>dlsite-manager</title>
</svelte:head>

<main class="app-shell">
  <aside class="sidebar" aria-label="Primary">
    <div class="brand">dlsite-manager</div>
    <nav>
      <a class="nav-item active" href="/">Settings</a>
    </nav>
  </aside>

  <section class="workspace">
    <header class="workspace-header">
      <div>
        <p class="eyebrow">Application</p>
        <h1>Settings</h1>
      </div>
      <button class="secondary" type="button" onclick={loadSettings} disabled={loading || saving}>
        Reload
      </button>
    </header>

    <form class="settings-panel" onsubmit={saveSettings}>
      <label>
        <span>Library root</span>
        <input
          type="text"
          autocomplete="off"
          spellcheck="false"
          bind:value={libraryRoot}
          disabled={loading || saving}
        />
      </label>

      <label>
        <span>Download root</span>
        <input
          type="text"
          autocomplete="off"
          spellcheck="false"
          bind:value={downloadRoot}
          disabled={loading || saving}
        />
      </label>

      <div class="actions">
        <p class:error={Boolean(error)} aria-live="polite">{error || status}</p>
        <button type="submit" disabled={loading || saving}>{saving ? "Saving" : "Save"}</button>
      </div>
    </form>
  </section>
</main>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    color: #1f2933;
    background: #eef2f5;
    font-family:
      Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    font-size: 15px;
    letter-spacing: 0;
  }

  :global(button),
  :global(input) {
    font: inherit;
    letter-spacing: 0;
  }

  .app-shell {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    min-height: 100vh;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 24px 18px;
    border-right: 1px solid #d4dde5;
    background: #17212b;
    color: #f8fafc;
  }

  .brand {
    font-size: 16px;
    font-weight: 700;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .nav-item {
    display: block;
    padding: 9px 10px;
    border-radius: 6px;
    color: #cbd5df;
    text-decoration: none;
  }

  .nav-item.active {
    background: #2a3947;
    color: #ffffff;
  }

  .workspace {
    min-width: 0;
    padding: 32px;
  }

  .workspace-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 22px;
  }

  .eyebrow {
    margin: 0 0 4px;
    color: #667787;
    font-size: 13px;
    font-weight: 600;
  }

  h1 {
    margin: 0;
    color: #111827;
    font-size: 28px;
    font-weight: 700;
  }

  .settings-panel {
    display: grid;
    gap: 18px;
    max-width: 760px;
    padding: 22px;
    border: 1px solid #d8e0e7;
    border-radius: 8px;
    background: #ffffff;
    box-shadow: 0 1px 2px rgb(15 23 42 / 8%);
  }

  label {
    display: grid;
    gap: 8px;
  }

  label span {
    color: #334155;
    font-size: 13px;
    font-weight: 650;
  }

  input {
    width: 100%;
    min-width: 0;
    height: 40px;
    padding: 0 11px;
    border: 1px solid #cbd5df;
    border-radius: 6px;
    color: #111827;
    background: #fbfcfd;
  }

  input:focus {
    border-color: #38658f;
    outline: 2px solid rgb(56 101 143 / 16%);
  }

  input:disabled {
    color: #7b8794;
    background: #f4f7f9;
  }

  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    min-height: 40px;
  }

  .actions p {
    margin: 0;
    min-width: 0;
    color: #2f6f4f;
    overflow-wrap: anywhere;
  }

  .actions p.error {
    color: #b42318;
  }

  button {
    min-width: 88px;
    height: 38px;
    padding: 0 14px;
    border: 1px solid #203142;
    border-radius: 6px;
    color: #ffffff;
    background: #203142;
    cursor: pointer;
  }

  button.secondary {
    min-width: 82px;
    border-color: #c5d0da;
    color: #1f2933;
    background: #ffffff;
  }

  button:disabled {
    cursor: default;
    opacity: 0.58;
  }

  @media (max-width: 720px) {
    .app-shell {
      grid-template-columns: 1fr;
    }

    .sidebar {
      padding: 14px 16px;
      border-right: 0;
      border-bottom: 1px solid #d4dde5;
    }

    .workspace {
      padding: 20px 16px;
    }

    .workspace-header,
    .actions {
      align-items: stretch;
      flex-direction: column;
    }

    button,
    button.secondary {
      width: 100%;
    }
  }
</style>
