<script lang="ts">
  import AccountEditor from "$lib/features/accounts/AccountEditor.svelte";

  let {
    editorState = "new",
  }: {
    editorState?: "new" | "editing" | "saving";
  } = $props();

  let label = $state("");
  let loginName = $state("");
  let password = $state("");
  let lastAction = $state("No action yet");

  $effect(() => {
    label = editorState === "new" ? "" : "Primary DLsite account";
    loginName = editorState === "new" ? "" : "primary@example.test";
    password = "";
  });
</script>

<main class="story-surface">
  <AccountEditor
    editing={editorState !== "new"}
    saving={editorState === "saving"}
    bind:label
    bind:loginName
    bind:password
    onReset={() => (lastAction = "New account")}
    onSave={(event) => { event.preventDefault(); lastAction = `Submit ${label}`; }}
  />
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface { display:grid; gap:12px; width:min(420px,calc(100vw - 48px)); margin:24px auto; }
  p { margin:0; color:var(--text-subtle); font-size:12px; text-align:right; }
  @media (max-width:720px) { .story-surface { width:calc(100vw - 24px); margin:12px auto; } }
</style>
