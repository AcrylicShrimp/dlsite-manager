<script lang="ts">
  import type { PageData } from "./$types";
  import type { Account } from "@app/types/account";

  import SmallButton from "@app/lib/buttons/SmallButton.svelte";
  import SmallRedButton from "@app/lib/buttons/SmallRedButton.svelte";

  import { invoke } from "@tauri-apps/api/tauri";
  import { appWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";

  export let data: PageData;
  let accounts: Account[] = [];

  onMount(async () => {
    accounts = data.accounts;

    const unlistens = await Promise.all([
      appWindow.listen<Account>("add-account", (event) => {
        accounts = [...accounts, event.payload];
      }),
      appWindow.listen<Account>("edit-account", (event) => {
        for (const account of accounts) {
          if (account.id !== event.payload.id) continue;
          account.username = event.payload.username;
          account.password = event.payload.password;
          account.memo = event.payload.memo;
          account.created_at = event.payload.created_at;
          account.updated_at = event.payload.updated_at;
          break;
        }

        accounts = [...accounts];
      }),
      appWindow.listen<number>("remove-account", (event) => {
        const index = accounts.findIndex(
          (account) => account.id === event.payload
        );

        if (index < 0) return;

        accounts.splice(index, 1);
        accounts = [...accounts];
      }),
    ]);

    await invoke("show_window");

    return () => {
      for (const unlisten of unlistens) unlisten();
    };
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

<h1 class="text-center">Account Management</h1>
<span class="block h-8" />
<section>
  <div class="flex flex-row items-center justify-end">
    <SmallRedButton on:click={add}>Add</SmallRedButton>
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
        <SmallButton on:click={() => edit(account)}>Edit</SmallButton>
        <span class="flex-none block w-1" />
        <SmallRedButton on:click={() => remove(account)}>Remove</SmallRedButton>
      </div>
      {#if index < accounts.length - 1}
        <span class="block h-2" />
      {/if}
    {:else}
      <div class="h-[200px] flex flex-col items-center justify-center">
        <p class="text-xl text-2/5 select-none">
          It seems that you haven't any account.
        </p>
      </div>
    {/each}
  </div>
</section>
