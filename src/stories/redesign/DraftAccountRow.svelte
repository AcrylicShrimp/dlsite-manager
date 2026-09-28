<script lang="ts">
  import type { Account } from "$lib/model/types";
  import Button from "./DraftButton.svelte";
  let { account, onEdit }: { account: Account; onEdit: () => void } = $props();
</script>

<article
  class="account-row dm:flex dm:items-center dm:gap-3 dm:border-0 dm:border-b dm:border-solid dm:border-draft-line dm:px-4 dm:py-4"
>
  <span
    class="dm:grid dm:size-10 dm:shrink-0 dm:place-items-center dm:rounded-lg dm:bg-draft-selected dm:text-draft-accent dm:text-lg"
    >{account.label.slice(0, 1)}</span
  >
  <div
    class="dm:flex dm:min-w-0 dm:flex-1 dm:flex-col dm:gap-1 dm:wrap-anywhere"
  >
    <strong class="dm:text-base dm:font-semibold">{account.label}</strong><span
      class="dm:text-xs dm:text-draft-dim"
      >{account.loginName ?? "No saved credentials"}</span
    ><small
      class={`dm:text-xs ${account.enabled ? "dm:text-draft-accent" : "dm:text-draft-dim"}`}
      >{account.enabled ? "Connected · DLsite" : "Disabled"}</small
    >
  </div>
  <Button onclick={onEdit}>Edit</Button>
</article>
