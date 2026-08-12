<script lang="ts">
  import SettingsView from "$lib/features/settings/SettingsView.svelte";

  let {
    viewState = "ready",
  }: {
    viewState?: "ready" | "loading" | "saving";
  } = $props();

  let libraryRoot = $state("/Users/example/Library/DLsite Manager Collection");
  let downloadRoot = $state("/Users/example/Downloads/DLsite Staging");
  let lastAction = $state("No action yet");
</script>

<main class="story-surface">
  <SettingsView
    bind:libraryRoot
    bind:downloadRoot
    loading={viewState === "loading"}
    saving={viewState === "saving"}
    appInfo={{ name: "DLsite Manager", version: "3.0.0", identifier: "com.acrylicshrimp.dlsite-manager", tauriVersion: "2.8.5" }}
    appInfoLoading={viewState === "loading"}
    onReload={() => (lastAction = "Reload")}
    onChooseDirectory={(kind) => (lastAction = `Browse ${kind}`)}
    onUseDefaultDownloadRoot={() => { downloadRoot = "/Users/example/Downloads"; lastAction = "Use default"; }}
    onSave={(event) => { event.preventDefault(); lastAction = `Save ${libraryRoot}`; }}
    onOpenGitHub={() => (lastAction = "Open GitHub")}
    onOpenDlsite={() => (lastAction = "Open DLsite")}
  />
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface { display:grid; gap:12px; width:min(1120px,calc(100vw - 48px)); margin:24px auto; }
  p { margin:0; color:var(--text-subtle); font-size:12px; text-align:right; }
  @media (max-width:720px) { .story-surface { width:calc(100vw - 24px); margin:12px auto; } }
</style>
