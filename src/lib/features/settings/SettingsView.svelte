<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import Field from "$lib/components/ui/Field.svelte";
  import TextInput from "$lib/components/ui/TextInput.svelte";
  import UpdatePanel from "$lib/features/settings/UpdatePanel.svelte";
  import type { AppInfo } from "$lib/model/types";
  import { appInfoValue } from "$lib/utils/format";

  let {
    libraryRoot = $bindable(""),
    downloadRoot = $bindable(""),
    loading = false,
    saving = false,
    appInfo = null,
    appInfoLoading = false,
    updatePhase = "idle",
    updateProgressMessage = "",
    onReload,
    onChooseDirectory,
    onUseDefaultDownloadRoot,
    onSave,
    onOpenGitHub,
    onOpenDlsite,
    onCheckForUpdates,
  }: {
    libraryRoot?: string;
    downloadRoot?: string;
    loading?: boolean;
    saving?: boolean;
    appInfo?: AppInfo | null;
    appInfoLoading?: boolean;
    updatePhase?: "idle" | "checking" | "downloading" | "installing";
    updateProgressMessage?: string;
    onReload: () => void;
    onChooseDirectory: (kind: "library" | "download") => void;
    onUseDefaultDownloadRoot: () => void;
    onSave: (event: SubmitEvent) => void;
    onOpenGitHub: () => void;
    onOpenDlsite: () => void;
    onCheckForUpdates: () => void;
  } = $props();

  const busy = $derived(loading || saving);
</script>

<div class="settings-layout">
  <form class="settings-panel" onsubmit={onSave}>
    <div class="panel-title">
      <div>
        <h2>Storage paths</h2>
        <p>Library is the final managed collection. Download staging keeps resumable partial files and fetched archives.</p>
      </div>
      <UiButton variant="secondary" size="small" disabled={busy} onclick={onReload}>Reload</UiButton>
    </div>

    <Field id="library-root" label="Library folder" help="Final location for managed works after download and unpacking.">
      <div class="path-control">
        <TextInput id="library-root" disabled={busy} bind:value={libraryRoot} />
        <UiButton variant="secondary" size="small" disabled={busy} onclick={() => onChooseDirectory("library")}>
          Browse
        </UiButton>
      </div>
    </Field>

    <Field
      id="download-root"
      label="Download staging folder"
      help="Working folder for partial downloads, retries, and fetched archives. Defaults to your system Downloads folder."
    >
      <div class="path-control download-path-control">
        <TextInput id="download-root" disabled={busy} bind:value={downloadRoot} />
        <UiButton variant="secondary" size="small" disabled={busy} onclick={() => onChooseDirectory("download")}>
          Browse
        </UiButton>
        <UiButton variant="secondary" size="small" disabled={busy} onclick={onUseDefaultDownloadRoot}>
          Use Default
        </UiButton>
      </div>
    </Field>

    <div class="actions">
      <span></span>
      <UiButton type="submit" disabled={busy}>{saving ? "Saving" : "Save"}</UiButton>
    </div>
  </form>

  <section class="settings-panel about-panel" aria-label="About">
    <div class="panel-title">
      <h2>About</h2>
      <div class="panel-actions">
        <UiButton variant="secondary" size="small" onclick={onOpenGitHub}>GitHub</UiButton>
        <UiButton variant="secondary" size="small" onclick={onOpenDlsite}>DLsite</UiButton>
      </div>
    </div>
    <dl class="about-grid">
      <dt>Application</dt><dd>{appInfoValue(appInfo?.name, appInfoLoading)}</dd>
      <dt>Version</dt><dd>{appInfoValue(appInfo?.version, appInfoLoading)}</dd>
      <dt>Identifier</dt><dd>{appInfoValue(appInfo?.identifier, appInfoLoading)}</dd>
      <dt>Tauri</dt><dd>{appInfoValue(appInfo?.tauriVersion, appInfoLoading)}</dd>
    </dl>

    <UpdatePanel phase={updatePhase} message={updateProgressMessage} onCheck={onCheckForUpdates} />
  </section>
</div>

<style>
  .settings-layout {
    display: grid;
    flex: 1 1 auto;
    align-content: start;
    gap: 14px;
    width: 100%;
    min-width: 0;
    min-height: 0;
    overflow: auto;
    scrollbar-gutter: stable;
  }

  .settings-panel {
    display: grid;
    gap: 14px;
    width: 100%;
    min-width: 0;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 16px 40px rgb(0 0 0 / 18%);
  }

  .about-panel { gap: 10px; }

  .panel-title,
  .panel-actions,
  .actions {
    display: flex;
    align-items: center;
  }

  .panel-title,
  .actions { justify-content: space-between; gap: 10px; }
  .panel-title { margin-bottom: 0; }
  .panel-title > div { min-width: 0; }
  .panel-actions { gap: 8px; }

  h2 { margin: 0; color: var(--text-strong); font-size: 17px; font-weight: 700; }
  p { margin: 4px 0 0; color: var(--muted); font-size: 12px; line-height: 1.35; }

  .path-control { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 8px; align-items: center; }
  .download-path-control { grid-template-columns: minmax(0, 1fr) auto auto; }

  .about-grid {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    column-gap: 18px;
    row-gap: 8px;
    margin: 0;
    font-size: 13px;
  }

  .about-grid dt { color: var(--muted); font-weight: 650; }
  .about-grid dd {
    min-width: 0;
    margin: 0;
    color: var(--text);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 720px) {
    .panel-title, .panel-actions, .actions { align-items: stretch; flex-direction: column; }
    .path-control, .download-path-control { grid-template-columns: 1fr; }
  }
</style>
