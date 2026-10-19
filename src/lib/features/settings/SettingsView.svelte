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
  import { DEFAULT_LANGUAGE_PRIORITY } from "$lib/model/constants";
  import {
    languageCodeLabel,
    languageDisplayName,
  } from "$lib/utils/language-display-name";
  import unicodeLicense from "$lib/data/cldr-48.2.3/LICENSE?raw";

  let {
    libraryRoot = $bindable(""),
    downloadRoot = $bindable(""),
    languagePriority = $bindable([...DEFAULT_LANGUAGE_PRIORITY]),
    languageDiscoveryLoading = false,
    languageDiscoveryError = "",
    loading = false,
    saving = false,
    appInfo = null,
    appInfoLoading = false,
    updatePhase = "idle",
    updateProgressMessage = "",
    onReload,
    onMoveLanguage,
    onRetryLanguages,
    onChooseDirectory,
    onUseDefaultDownloadRoot,
    onSave,
    onOpenGitHub,
    onOpenDlsite,
    onCheckForUpdates,
  }: {
    libraryRoot?: string;
    downloadRoot?: string;
    languagePriority?: string[];
    languageDiscoveryLoading?: boolean;
    languageDiscoveryError?: string;
    loading?: boolean;
    saving?: boolean;
    appInfo?: AppInfo | null;
    appInfoLoading?: boolean;
    updatePhase?: "idle" | "checking" | "downloading" | "installing";
    updateProgressMessage?: string;
    onReload: () => void;
    onMoveLanguage?: (code: string, direction: -1 | 1) => void;
    onRetryLanguages?: () => void;
    onChooseDirectory: (kind: "library" | "download") => void;
    onUseDefaultDownloadRoot: () => void;
    onSave: (event: SubmitEvent) => void;
    onOpenGitHub: () => void;
    onOpenDlsite: () => void;
    onCheckForUpdates: () => void;
  } = $props();

  const busy = $derived(loading || saving || languageDiscoveryLoading);

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
          { value: "names", label: "Names" },
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
    </form>{:else if tab === "names"}<form
      onsubmit={onSave}
      aria-label="Name language settings"
      class="dm:flex dm:min-w-0 dm:flex-col dm:gap-5"
    >
      <div class="dm:max-w-2xl dm:text-sm dm:text-draft-dim">
        <p class="dm:mt-0">
          Use the same language order for work titles and maker names. The first
          available nonempty name is shown.
        </p>
        <p class="dm:mb-0">
          Search always covers every language. Additional languages appear here
          when found in cached names; discovering one does not change your saved
          order.
        </p>
      </div>
      {#if languageDiscoveryError}
        <div
          role="alert"
          class="dm:flex dm:flex-wrap dm:items-center dm:gap-3 dm:text-sm dm:text-draft-error"
        >
          <span
            >Could not load additional languages. Saved and default languages
            remain available. {languageDiscoveryError}</span
          >
          <Button disabled={busy} onclick={onRetryLanguages}
            >Retry languages</Button
          >
        </div>
      {/if}
      {#if languageDiscoveryLoading}<p
          role="status"
          class="dm:m-0 dm:text-sm dm:text-draft-dim"
        >
          Loading additional languages…
        </p>{/if}
      <ol
        aria-label="Name language priority"
        class="dm:m-0 dm:flex dm:max-w-3xl dm:list-decimal dm:flex-col dm:gap-2 dm:pl-6"
      >
        {#each languagePriority as code, index (code)}
          {@const name = languageDisplayName(code)}
          <li class="dm:min-w-0 dm:pl-1">
            <div
              class="dm:flex dm:min-w-0 dm:items-center dm:justify-between dm:gap-3 dm:rounded-md dm:border dm:border-solid dm:border-draft-line dm:bg-draft-surface dm:p-3"
            >
              <div class="dm:min-w-0 dm:break-all dm:text-sm">
                <span>{name}</span>
                {#if name !== code}<code
                    class="dm:mt-1 dm:block dm:text-xs dm:text-draft-dim"
                    >{code}</code
                  >{/if}
              </div>
              <div class="dm:flex dm:shrink-0 dm:gap-1">
                <Button
                  disabled={busy || index === 0}
                  aria-label={`Move ${languageCodeLabel(code)} up`}
                  onclick={() => onMoveLanguage?.(code, -1)}>↑</Button
                >
                <Button
                  disabled={busy || index === languagePriority.length - 1}
                  aria-label={`Move ${languageCodeLabel(code)} down`}
                  onclick={() => onMoveLanguage?.(code, 1)}>↓</Button
                >
              </div>
            </div>
          </li>
        {/each}
      </ol>
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
        ><UiButton variant="secondary" onclick={onOpenDlsite}>DLsite ↗</UiButton
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
      <Disclosure title="Language name data">
        <p class="dm:text-sm dm:text-draft-dim">
          Language, territory and script names: Unicode CLDR 48.2.3.
        </p>
        <pre
          class="dm:max-h-80 dm:overflow-auto dm:whitespace-pre-wrap dm:break-words dm:text-xs dm:text-draft-dim">{unicodeLicense}</pre>
      </Disclosure>
    </section>{/if}
</div>
