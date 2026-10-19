<script lang="ts">
  import SettingsView from "$lib/features/settings/SettingsView.svelte";
  import { DEFAULT_LANGUAGE_PRIORITY } from "$lib/model/constants";
  import {
    buildLanguagePriority,
    languagePriorityPatch,
    moveLanguagePriority,
  } from "$lib/utils/name-language-priority";

  let {
    viewState = "ready",
    languagePreset = "default",
  }: {
    viewState?:
      | "ready"
      | "loading"
      | "saving"
      | "checking"
      | "downloading"
      | "installing"
      | "discoveryFailure"
      | "discovering";
    languagePreset?: "default" | "japanese" | "unknown" | "subset" | "long";
  } = $props();

  let libraryRoot = $state("/Users/example/Library/DLsite Manager Collection");
  let downloadRoot = $state("/Users/example/Downloads/DLsite Staging");
  let lastAction = $state("No action yet");
  let savedPriority = $state([...DEFAULT_LANGUAGE_PRIORITY]);
  let observed = $state<string[]>([]);
  let languagePriority = $state([...DEFAULT_LANGUAGE_PRIORITY]);
  let baseline = $state([...DEFAULT_LANGUAGE_PRIORITY]);
  let discoveryFailed = $state(false);
  $effect(() => {
    const saved =
      languagePreset === "subset"
        ? ["ja_JP"]
        : languagePreset === "japanese"
          ? [
              "ja_JP",
              ...DEFAULT_LANGUAGE_PRIORITY.filter((code) => code !== "ja_JP"),
            ]
          : [...DEFAULT_LANGUAGE_PRIORITY];
    const discovered =
      languagePreset === "unknown"
        ? ["pt_BR", "xx_XX", "zh_Hant"]
        : languagePreset === "long"
          ? [`x_${"unrecognized-language-code-".repeat(8)}`, "pt_BR"]
          : languagePreset === "subset"
            ? ["aa_AA"]
            : ["pt_BR"];
    const draft = buildLanguagePriority(saved, discovered);
    savedPriority = saved;
    observed = discovered;
    languagePriority = draft;
    baseline = [...draft];
  });
  $effect(() => {
    discoveryFailed = viewState === "discoveryFailure";
  });
</script>

<main class="story-surface">
  <SettingsView
    bind:libraryRoot
    bind:downloadRoot
    bind:languagePriority
    languageDiscoveryError={discoveryFailed
      ? "Simulated discovery failure"
      : ""}
    languageDiscoveryLoading={viewState === "discovering"}
    loading={viewState === "loading"}
    saving={viewState === "saving"}
    appInfo={{
      name: "DLsite Manager",
      version: "3.2.2",
      identifier: "com.acrylicshrimp.dlsite-manager",
      tauriVersion: "2.8.5",
    }}
    appInfoLoading={viewState === "loading"}
    updatePhase={viewState === "checking" ||
    viewState === "downloading" ||
    viewState === "installing"
      ? viewState
      : "idle"}
    updateProgressMessage={viewState === "checking"
      ? "Checking for updates"
      : viewState === "downloading"
        ? "Downloading 3.2.3 68%"
        : viewState === "installing"
          ? "Installing 3.2.3"
          : ""}
    onReload={() => {
      languagePriority = buildLanguagePriority(savedPriority, observed);
      baseline = [...languagePriority];
      lastAction = "Reverted changes";
    }}
    onMoveLanguage={(code, direction) => {
      languagePriority = moveLanguagePriority(
        languagePriority,
        code,
        direction,
      );
    }}
    onRetryLanguages={() => {
      discoveryFailed = false;
      lastAction = "Language discovery retried";
    }}
    onChooseDirectory={(kind) => (lastAction = `Browse ${kind}`)}
    onUseDefaultDownloadRoot={() => {
      downloadRoot = "/Users/example/Downloads";
      lastAction = "Use default";
    }}
    onSave={(event) => {
      event.preventDefault();
      const patch = languagePriorityPatch(languagePriority, baseline);
      lastAction = JSON.stringify({ libraryRoot, downloadRoot, ...patch });
      if (patch.languagePriority) savedPriority = [...patch.languagePriority];
      languagePriority = buildLanguagePriority(savedPriority, observed);
      baseline = [...languagePriority];
    }}
    onOpenGitHub={() => (lastAction = "Open GitHub")}
    onOpenDlsite={() => (lastAction = "Open DLsite")}
    onCheckForUpdates={() => (lastAction = "Check for updates")}
  />
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface {
    display: grid;
    gap: 12px;
    width: min(1120px, calc(100vw - 48px));
    margin: 24px auto;
  }
  p {
    margin: 0;
    color: var(--text-subtle);
    font-size: 12px;
    text-align: right;
  }
  @media (max-width: 720px) {
    .story-surface {
      width: calc(100vw - 24px);
      margin: 12px auto;
    }
  }
</style>
