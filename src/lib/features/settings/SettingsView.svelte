<script lang="ts">
  import PageHeader from "$lib/components/workspace/PageHeader.svelte";
  import FormActions from "$lib/components/workspace/FormActions.svelte";
  import Button from "$lib/components/workspace/Button.svelte";
  import UiButton from "$lib/components/ui/Button.svelte";
  import Choices from "$lib/components/workspace/ChoiceGroup.svelte";
  import Mark from "$lib/components/workspace/AppMark.svelte";
  import Disclosure from "$lib/components/workspace/Disclosure.svelte";
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

  let tab = $state("storage");
</script>

<div class="settings-layout dm:min-w-0">
  <PageHeader title="Settings">
    {#snippet tabs()}
      <Choices
        variant="tabs"
        label="Settings section"
        value={tab}
        onchange={(v) => (tab = v)}
        options={[
          { value: "storage", label: "Storage" },
          { value: "about", label: "About & updates" },
        ]}
      />
    {/snippet}
  </PageHeader>
  {#if tab === "storage"}<form
      onsubmit={onSave}
      class="dm:flex dm:min-w-0 dm:flex-col dm:gap-6"
    >
      <Field id="library-root" label="Library folder"
        ><div class="dm:flex dm:flex-wrap dm:gap-2">
          <div class="dm:min-w-0 dm:flex-1">
            <TextInput
              id="library-root"
              disabled={busy}
              bind:value={libraryRoot}
            />
          </div>
          <UiButton
            variant="secondary"
            responsiveWidth="auto"
            disabled={busy}
            onclick={() => onChooseDirectory("library")}>Browse</UiButton
          >
        </div></Field
      >
      <Field id="download-root" label="Download staging folder"
        ><div class="dm:flex dm:flex-wrap dm:gap-2">
          <div class="dm:min-w-0 dm:flex-1">
            <TextInput
              id="download-root"
              disabled={busy}
              bind:value={downloadRoot}
            />
          </div>
          <UiButton
            variant="secondary"
            responsiveWidth="auto"
            disabled={busy}
            onclick={() => onChooseDirectory("download")}>Browse</UiButton
          >
        </div>
        <div>
          <Button
            variant="text"
            disabled={busy}
            onclick={onUseDefaultDownloadRoot}
            >Use system Downloads folder</Button
          >
        </div></Field
      >
      <FormActions>
        <Button disabled={busy} onclick={onReload}>Revert changes</Button>
        <Button variant="primary" type="submit" disabled={busy}
          >{saving ? "Saving…" : "Save changes"}</Button
        >
      </FormActions>
    </form>{:else}<section aria-label="About" class="dm:min-w-0 dm:py-2">
      <div class="dm:flex dm:items-center dm:gap-3">
        <Mark size="large" />
        <div class="dm:min-w-0">
          <h2 class="dm:m-0 dm:text-lg dm:font-semibold">
            {appInfoValue(appInfo?.name, appInfoLoading)}
          </h2>
          <p class="dm:mt-1 dm:mb-0 dm:text-sm dm:text-draft-dim">
            Version {appInfoValue(appInfo?.version, appInfoLoading)}
          </p>
        </div>
      </div>
      <div class="dm:my-5">
        <UpdatePanel
          phase={updatePhase}
          message={updateProgressMessage}
          onCheck={onCheckForUpdates}
        />
      </div>
      <div
        class="dm:mb-5 dm:flex dm:flex-wrap dm:items-center dm:gap-3 dm:border-0 dm:border-t dm:border-solid dm:border-draft-line dm:pt-4"
      >
        <UiButton variant="secondary" onclick={onOpenGitHub}>GitHub ↗</UiButton
        ><UiButton variant="secondary" onclick={onOpenDlsite}
          >DLsite ↗</UiButton
        ><span class="dm:text-xs dm:text-draft-dim">MIT License</span>
      </div>
      <Disclosure title="Application details"
        ><dl
          class="dm:grid dm:grid-cols-[auto_minmax(0,1fr)] dm:gap-3 dm:text-sm"
        >
          <dt>Identifier</dt>
          <dd class="dm:m-0 dm:wrap-anywhere">
            {appInfoValue(appInfo?.identifier, appInfoLoading)}
          </dd>
          <dt>Tauri</dt>
          <dd class="dm:m-0">
            {appInfoValue(appInfo?.tauriVersion, appInfoLoading)}
          </dd>
        </dl></Disclosure
      >
    </section>{/if}
</div>
