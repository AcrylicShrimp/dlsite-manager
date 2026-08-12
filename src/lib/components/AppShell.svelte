<script lang="ts">
  import type { Snippet } from "svelte";
  import type { View } from "$lib/model/types";
  import SidebarNav from "$lib/components/SidebarNav.svelte";
  import WorkspaceHeader from "$lib/components/WorkspaceHeader.svelte";

  const VIEW_COPY: Record<View, { eyebrow: string; title: string }> = {
    library: { eyebrow: "Collection", title: "Library" },
    downloads: { eyebrow: "Queue", title: "Downloads" },
    accounts: { eyebrow: "Sources", title: "Accounts" },
    activity: { eyebrow: "Jobs", title: "Activity" },
    settings: { eyebrow: "Application", title: "Settings" },
  };

  let {
    activeView,
    onNavigate,
    children,
  }: {
    activeView: View;
    onNavigate: (view: View) => void;
    children?: Snippet;
  } = $props();

  let viewCopy = $derived(VIEW_COPY[activeView]);
</script>

<main class="app-shell">
  <SidebarNav {activeView} {onNavigate} />
  <section class:library-workspace={activeView === "library"} class="workspace">
    <WorkspaceHeader eyebrow={viewCopy.eyebrow} title={viewCopy.title} />
    {@render children?.()}
  </section>
</main>

<style>
  .app-shell {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    height: 100vh;
    min-height: 100vh;
    overflow: hidden;
  }

  .workspace {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    padding: 28px;
    overflow: hidden;
  }

  .workspace.library-workspace {
    padding-top: 0;
    overflow: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
  }

  .workspace.library-workspace :global(.workspace-header) {
    padding-top: 28px;
  }

  @media (max-width: 720px) {
    .app-shell {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto minmax(0, 1fr);
    }

    .workspace {
      padding: 20px 16px;
    }

    .workspace.library-workspace :global(.workspace-header) {
      padding-top: 20px;
    }
  }
</style>
