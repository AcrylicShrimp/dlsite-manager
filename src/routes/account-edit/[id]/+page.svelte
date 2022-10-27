<script lang="ts">
  import type { PageData } from "./$types";
  import PrimaryButton from "@app/lib/buttons/PrimaryButton.svelte";
  import SecondaryButton from "@app/lib/buttons/SecondaryButton.svelte";
  import LabeledInput from "@app/lib/inputs/LabeledInput.svelte";

  import { invoke } from "@tauri-apps/api/tauri";
  import { onMount } from "svelte";

  let id: number;
  let username: string;
  let password: string;
  let memo: string;
  let isTesting: boolean = false;
  let testResult: null | number = null;
  export let data: PageData;

  onMount(async () => {
    id = data.account.id;
    username = data.account.username;
    password = data.account.password;
    memo = data.account.memo ?? "";

    await invoke("show_window");
  });

  async function save() {
    await invoke("account_management_update_account", {
      account: {
        id,
        username,
        password,
        memo,
      },
    });
  }

  async function test() {
    isTesting = true;
    testResult = await invoke<number>("account_management_test_account", {
      username,
      password,
    });
    isTesting = false;
  }
</script>

<h1 class="text-center">Edit Account</h1>
<span class="block h-4" />
<section>
  <div>
    <LabeledInput
      label="Username"
      placeholder="Username"
      bind:value={username}
      disabled={isTesting}
    />
    <span class="block h-4" />
    <LabeledInput
      label="Password"
      placeholder="Password"
      bind:value={password}
      disabled={isTesting}
    />
    <span class="block h-4" />
    <LabeledInput
      label="Memo (optional)"
      placeholder="Memo"
      bind:value={memo}
      disabled={isTesting}
    />
  </div>
  <span class="block h-8" />
  <p
    class={"text-center text-sm" +
      (isTesting || testResult === null
        ? " text-3/5"
        : testResult < 0
        ? " text-error"
        : " text-ok")}
  >
    {#if isTesting}
      Testing...
    {:else if testResult === null}
      Not tested
    {:else if testResult < 0}
      Test failed
    {:else}
      Test success, {testResult} product(s) detected.
    {/if}
  </p>
  <span class="block h-4" />
  <div class="flex flex-row items-center justify-center">
    <SecondaryButton on:click={test} disabled={isTesting}>Test</SecondaryButton>
    <span class="inline-block w-4" />
    <PrimaryButton on:click={save} disabled={isTesting}>Save</PrimaryButton>
  </div>
</section>
