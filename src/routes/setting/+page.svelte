<script lang="ts">
  import type { PageData } from "./$types";

  import DragDropList from "@app/lib/lists/DragDropList.svelte";
  import PrimaryButton from "@app/lib/buttons/PrimaryButton.svelte";
  import SecondaryButton from "@app/lib/buttons/SecondaryButton.svelte";
  import type { DLsiteProductLocalizedString } from "@app/types/product";

  import { invoke } from "@tauri-apps/api/tauri";
  import { onMount } from "svelte";

  interface Language {
    id: string;
    text: string;
  }

  export let data: PageData;
  let defaultRootDir: string;
  let languages: Language[] = [];

  onMount(async () => {
    defaultRootDir = data.setting.download_root_dir;
    languages = agmentLanguage(data.display_language_setting.languages);

    await invoke("show_window");
  });

  async function browse() {
    defaultRootDir =
      (await invoke("setting_browse_default_root_directory")) ?? defaultRootDir;
  }

  async function close() {
    await invoke("setting_close");
  }

  async function save() {
    await invoke("setting_save_and_close", {
      setting: {
        download_root_dir: defaultRootDir,
      },
      displayLanguageSetting: {
        languages: deagmentLanguage(languages),
      },
    });
  }

  function agmentLanguage(
    languages: (keyof DLsiteProductLocalizedString)[]
  ): Language[] {
    const displayNames: Record<keyof DLsiteProductLocalizedString, string> = {
      japanese: "Japanese",
      english: "English",
      korean: "Korean",
      taiwanese: "Simplified Chinese",
      chinese: "Traditional Chinese",
    };
    return languages.map((language) => ({
      id: language,
      text: displayNames[language],
    }));
  }

  function deagmentLanguage(languages: Language[]): string[] {
    return languages.map((language) => language.id);
  }
</script>

<h1 class="text-center">Settings</h1>
<span class="block h-8" />
<section>
  <div>
    <label>
      <p>Default Root Directory</p>
      <div class="pl-2 pt-1 flex flex-row items-center justify-stretch">
        <input
          type="text"
          placeholder="Path"
          bind:value={defaultRootDir}
          class="px-2 py-1 w-full text-0/5 disabled:text-3/5 bg-4/5 disabled:bg-4/5/20 rounded"
        />
        <span class="inline-block w-4" />
        <SecondaryButton on:click={browse}>Browse</SecondaryButton>
      </div>
    </label>
  </div>
  <div class="mt-8">
    <p>
      Display Language <span class="text-3/5">(higher takes precedence)</span>
    </p>
    <div class="pl-2 pt-2">
      <DragDropList bind:data={languages} />
    </div>
  </div>
  <span class="block h-16" />
  <div class="flex flex-row items-center justify-center">
    <SecondaryButton on:click={close}>Cancel</SecondaryButton>
    <span class="inline-block w-4" />
    <PrimaryButton on:click={save}>Save</PrimaryButton>
  </div>
</section>

<style lang="postcss">
  :global(.dragdroplist > div.item) {
    @apply !border !border-4/5 !text-1/5 !bg-3/5 !rounded;
  }

  :global(.dragdroplist > .list > div.item) {
    @apply border border-4/5 text-0/5 bg-4/5 rounded;
  }
</style>
