<script lang="ts">
  import AppShell from "$lib/components/AppShell.svelte";
  import type { View } from "$lib/model/types";

  let { initialView = "library" }: { initialView?: View } = $props();
  let activeView = $state<View>("library");

  $effect(() => {
    activeView = initialView;
  });

  const viewNotes: Record<View, string> = {
    library: "Browse 248 cached works across two enabled accounts.",
    downloads: "Track active downloads and cooperative cancellation.",
    accounts: "Manage DLsite sources and their saved credentials.",
    activity: "Review recent jobs and support-oriented audit events.",
    settings: "Choose managed library and download staging folders.",
  };
</script>

<AppShell {activeView} onNavigate={(view) => (activeView = view)}>
  <section class="shell-preview" aria-label={`${activeView} preview`}>
    <div class="preview-heading">
      <div>
        <span>Selected workspace</span>
        <strong>{activeView}</strong>
      </div>
      <span class="state-pill">Ready</span>
    </div>
    <p>{viewNotes[activeView]}</p>
    <div class="preview-grid">
      <article>
        <span>Visible</span>
        <strong>{activeView === "library" ? "1–100" : "12"}</strong>
      </article>
      <article>
        <span>Total</span>
        <strong>{activeView === "library" ? "248" : "24"}</strong>
      </article>
      <article>
        <span>Status</span>
        <strong>Up to date</strong>
      </article>
    </div>
  </section>
</AppShell>

<style>
  .shell-preview {
    display: grid;
    gap: 18px;
    min-width: 0;
    min-height: 260px;
    padding: 20px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--panel);
    box-shadow: 0 16px 40px rgb(0 0 0 / 18%);
  }

  .preview-heading {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: start;
  }

  .preview-heading div {
    display: grid;
    gap: 4px;
  }

  span,
  p {
    color: var(--muted);
  }

  .preview-heading span,
  article span {
    font-size: 12px;
    font-weight: 650;
  }

  .preview-heading strong {
    color: var(--text-strong);
    font-size: 20px;
    text-transform: capitalize;
  }

  .state-pill {
    padding: 5px 8px;
    border: 1px solid rgb(149 194 155 / 24%);
    border-radius: 999px;
    color: var(--accent);
    background: var(--accent-muted);
  }

  p {
    max-width: 580px;
    margin: 0;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }

  .preview-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
    align-self: end;
  }

  article {
    display: grid;
    gap: 5px;
    padding: 13px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--panel-soft);
  }

  article strong {
    color: var(--text);
    font-size: 15px;
  }

  @media (max-width: 720px) {
    .preview-heading {
      align-items: flex-start;
      flex-direction: column;
    }

    .preview-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
