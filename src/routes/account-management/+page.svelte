<script lang="ts">
  import type { Account } from "@app/types/account";

  import SmallButton from "@app/lib/buttons/SmallButton.svelte";
  import SmallRedButton from "@app/lib/buttons/SmallRedButton.svelte";

  import { invoke } from "@tauri-apps/api/core";
  import { getCurrent } from "@tauri-apps/api/window";
  import { onMount } from "svelte";

  let accounts: Account[] = [];

  onMount(async () => {
    accounts = await invoke<Account[]>("account_management_list_accounts");

    const appWindow = getCurrent();
    await appWindow.listen<Account>("add-account", (event) => {
      accounts = [...accounts, event.payload];
    });
    await appWindow.listen<Account>("edit-account", (event) => {
      for (const account of accounts) {
        if (account.id !== event.payload.id) continue;
        account.username = event.payload.username;
        account.password = event.payload.password;
        account.memo = event.payload.memo;
        break;
      }

      accounts = [...accounts];
    });
    await appWindow.listen<number>("remove-account", (event) => {
      const index = accounts.findIndex(
        (account) => account.id === event.payload
      );

      if (index < 0) return;

      accounts.splice(index, 1);
      accounts = [...accounts];
    });

    await invoke("show_window");
  });

  async function add(): Promise<void> {
    await invoke("spawn_window_account_add");
  }

  async function edit(account: Account): Promise<void> {
    await invoke("spawn_window_account_edit", {
      accountId: account.id,
    });
  }

  async function remove(account: Account): Promise<void> {
    await invoke("account_management_remove_account", {
      accountId: account.id,
    });
  }
</script>

<h1 class="text-center">アカウントの管理</h1>
<span class="block h-8" />
<section>
  <div class="flex flex-row items-center justify-end">
    <SmallRedButton on:click={add}>追加</SmallRedButton>
  </div>
  <span class="block h-2" />
  <div>
    {#each accounts as account, index (account)}
      <div
        class="p-1 pl-2 border border-1/5 rounded flex flex-row items-center justify-start"
      >
        <p class="text-4/5 truncate">
          {account.username}
          {#if account.memo}
            <span class="w-1" />
            <span class="text-sm text-4/5/50 truncate">({account.memo})</span>
          {/if}
        </p>
        <span class="flex-1" />
        <SmallButton on:click={() => edit(account)}>編集</SmallButton>
        <span class="flex-none block w-1" />
        <SmallRedButton on:click={() => remove(account)}>削除</SmallRedButton>
      </div>
      {#if index < accounts.length - 1}
        <span class="block h-2" />
      {/if}
    {:else}
      <div class="h-[200px] flex flex-col items-center justify-center">
        <p class="text-xl text-2/5 select-none">
          アカウントを登録していないようですね。
        </p>
      </div>
    {/each}
  </div>
</section>
