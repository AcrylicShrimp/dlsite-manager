<script lang="ts">
  import { tick, untrack } from "svelte";
  import type { Snippet } from "svelte";
  import type { JobSnapshot, View } from "$lib/model/types";
  import { isActiveJob, isDownloadQueueJob } from "$lib/utils/jobs";
  import Sidebar from "./workspace/Sidebar.svelte";
  import BackToTop from "./workspace/BackToTop.svelte";
  let {
    activeView,
    onNavigate,
    children,
    jobs = [],
    version = "",
  }: {
    activeView: View;
    onNavigate: (view: View) => void;
    children?: Snippet;
    jobs?: JobSnapshot[];
    version?: string;
  } = $props();
  let scroller: HTMLElement;
  let scrollTop = $state(0);
  const positions: Partial<Record<View, number>> = {};
  const downloads = $derived(
    jobs.filter((j) => isDownloadQueueJob(j) && isActiveJob(j)),
  );
  const running = $derived(downloads.filter((j) => j.status !== "queued"));
  const failures = $derived(
    jobs.filter((j) => isDownloadQueueJob(j) && j.status === "failed").length,
  );
  $effect(() => {
    const view = activeView;
    void tick().then(() =>
      untrack(() => {
        if (scroller) {
          scroller.scrollTop = positions[view] ?? 0;
          scrollTop = scroller.scrollTop;
        }
      }),
    );
  });
</script>

<div
  class="app-shell dm:grid dm:h-dvh dm:grid-cols-[194px_minmax(0,1fr)] dm:overflow-hidden dm:bg-draft-background dm:text-draft-ink dm:text-base dm:max-[620px]:grid-cols-1 dm:max-[620px]:grid-rows-[auto_minmax(0,1fr)]"
>
  <Sidebar
    view={activeView}
    {version}
    {running}
    queuedCount={downloads.filter((j) => j.status === "queued").length}
    activeCount={downloads.length}
    failureCount={failures}
    {onNavigate}
    onRunning={() => onNavigate("downloads")}
    onFailure={() => onNavigate("activity")}
  />
  <main
    bind:this={scroller}
    tabindex="-1"
    class="workspace dm:outline-none dm:min-h-0 dm:min-w-0 dm:overflow-auto dm:px-6 dm:py-7 dm:[scrollbar-gutter:stable] dm:max-[620px]:px-4 dm:max-[620px]:py-5"
    onscroll={() => {
      scrollTop = scroller.scrollTop;
      positions[activeView] = scrollTop;
    }}
  >
    {@render children?.()}
  </main>
</div>
{#if scrollTop > 300}<BackToTop
    target={() => scroller}
    class="dm:fixed dm:bottom-6 dm:right-6 dm:z-20"
  />{/if}
