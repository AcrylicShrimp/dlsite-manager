<script lang="ts">
  import PrimaryButton from "@app/lib/buttons/PrimaryButton.svelte";
  import SecondaryButton from "@app/lib/buttons/SecondaryButton.svelte";
  import LabeledInput from "@app/lib/inputs/LabeledInput.svelte";

  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let username: string;
  let password: string;
  let memo: string;
  let isTesting: boolean = false;
  let testResult: null | number = null;

  onMount(async () => {
    await invoke("show_window");
  });

  async function add() {
    await invoke("account_management_add_account", {
      account: {
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

<h1 class="text-center">Add Account</h1>
<span class="block h-4" />
<section>
  <div>
    <LabeledInput
      label="ユーザー名"
      placeholder="ユーザー名"
      bind:value={username}
      disabled={isTesting}
    />
    <span class="block h-4" />
    <LabeledInput
      label="パスワード"
      placeholder="パスワード"
      bind:value={password}
      disabled={isTesting}
    />
    <span class="block h-4" />
    <LabeledInput
      label="メモ (任意)"
      placeholder="メモ"
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
      テストが成功しました。{testResult}件の商品が検出されました。
    {/if}
  </p>
  <span class="block h-4" />
  <div class="flex flex-row items-center justify-center">
    <SecondaryButton on:click={test} disabled={isTesting}>テスト</SecondaryButton>
    <span class="inline-block w-4" />
    <PrimaryButton on:click={add} disabled={isTesting}>追加</PrimaryButton>
  </div>
</section>
