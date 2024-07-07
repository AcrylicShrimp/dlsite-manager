<script lang="ts">
  import type { Setting } from "@app/types/setting";

  import PrimaryButton from "@app/lib/buttons/PrimaryButton.svelte";
  import SecondaryButton from "@app/lib/buttons/SecondaryButton.svelte";

  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let defaultRootDir: string;

  onMount(async () => {
    const setting = await invoke<Setting>("setting_get");
    defaultRootDir = setting.download_root_dir;

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
    });
  }
</script>

<h1 class="text-center">設定</h1>
<span class="block h-8" />
<section>
  <div>
    <label>
      <p>既定のルートディレクトリ</p>
      <div class="pl-2 pt-1 flex flex-row items-center justify-stretch">
        <input
          type="text"
          placeholder="パス"
          bind:value={defaultRootDir}
          class="px-2 py-1 w-full text-0/5 disabled:text-3/5 bg-4/5 disabled:bg-4/5/20 rounded"
        />
        <span class="inline-block w-4" />
        <SecondaryButton on:click={browse}>参照</SecondaryButton>
      </div>
    </label>
  </div>
  <span class="block h-16" />
  <div class="flex flex-row items-center justify-center">
    <SecondaryButton on:click={close}></SecondaryButton>
    <span class="inline-block w-4" />
    <PrimaryButton on:click={save}>保存</PrimaryButton>
  </div>
</section>
