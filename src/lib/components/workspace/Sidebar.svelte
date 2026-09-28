<script lang="ts">
  import type { JobSnapshot, View } from "$lib/model/types";
  import { views, labels } from "$lib/model/navigation";
  import { downloadQueueProgressPercent as progress } from "$lib/utils/jobs";
  import Button from "$lib/components/workspace/Button.svelte";
  import AppMark from "$lib/components/workspace/AppMark.svelte";
  import Icon from "$lib/components/workspace/Icon.svelte";
  import Progress from "$lib/components/workspace/Progress.svelte";
  let {
    view,
    version = "",
    running,
    queuedCount,
    activeCount,
    failureCount,
    onNavigate,
    onRunning,
    onFailure,
  }: {
    view: View;
    version?: string;
    running: JobSnapshot[];
    queuedCount: number;
    activeCount: number;
    failureCount: number;
    onNavigate: (view: View) => void;
    onRunning: () => void;
    onFailure: () => void;
  } = $props();
</script>

<aside
  class="sidebar dm:flex dm:min-h-0 dm:flex-col dm:gap-3 dm:border-0 dm:border-r dm:border-solid dm:border-draft-line-subtle dm:bg-draft-sidebar dm:px-3.5 dm:pt-6 dm:pb-4 dm:text-base dm:max-[620px]:border-r-0 dm:max-[620px]:border-b dm:max-[620px]:pt-3 dm:max-[620px]:pb-2"
  aria-label="Primary"
>
  <a
    href="#library"
    onclick={(event) => {
      event.preventDefault();
      onNavigate("library");
    }}
    class="dm:flex dm:items-center dm:gap-2.5 dm:text-draft-ink dm:no-underline dm:font-semibold dm:draft-focus-text"
  >
    <AppMark /><span>dlsite-manager</span>
  </a>
  <nav aria-label="Main" class="dm:grid dm:gap-1 dm:max-[620px]:flex">
    {#each views as item}<Button
        variant="navigation"
        aria-current={view === item ? "page" : undefined}
        onclick={() => onNavigate(item)}
      >
        <Icon name={item} /><span>{labels[item]}</span>
        {#if item === "downloads" && activeCount}<small
            class="dm:ml-auto dm:rounded dm:bg-draft-selected-strong dm:px-1.5 dm:text-xs dm:text-draft-selected-text dm:max-[620px]:hidden"
            >{activeCount}</small
          >{/if}
        {#if item === "activity" && failureCount}<span
            class="dm:ml-auto dm:size-1.5 dm:rounded-full dm:bg-draft-error dm:max-[620px]:hidden"
            aria-label={`${failureCount} failed job`}
          ></span>{/if}
      </Button>{/each}
  </nav>
  <div
    class="queue-overview dm:mt-auto dm:border-0 dm:border-t dm:border-solid dm:border-draft-line-subtle dm:pt-3 dm:max-[620px]:hidden"
    aria-label="All downloads summary"
  >
    <Button
      variant="sidebar"
      class="queue-heading"
      onclick={() => onNavigate("downloads")}
      ><span class="dm:flex dm:w-full dm:items-center dm:justify-between"
        ><span class="dm:text-[9px] dm:tracking-widest">DOWNLOADS</span><Icon
          name="arrow"
        /></span
      ></Button
    >
    <p class="dm:mx-0 dm:mt-2 dm:mb-3 dm:px-2.5 dm:text-xs dm:text-draft-dim">
      <strong class="dm:text-lg dm:text-draft-ink">{running.length}</strong>
      active <span class="dm:mx-1.5">·</span><strong
        class="dm:text-lg dm:text-draft-ink">{queuedCount}</strong
      > queued
    </p>
    {#each running.slice(0, 2) as job}<Button
        variant="sidebar"
        class="mini-job"
        onclick={onRunning}
        title={job.title}
        ><span class="dm:flex dm:w-full dm:min-w-0 dm:flex-col dm:gap-1.5"
          ><span class="dm:flex dm:justify-between dm:gap-2 dm:text-[10px]"
            ><span class="dm:truncate">{job.title}</span><b
              class="dm:shrink-0 dm:font-medium dm:text-draft-accent"
              >{progress(job) === null ? "…" : `${progress(job)}%`}</b
            ></span
          ><Progress
            compact
            value={progress(job)}
            label={`${job.title} progress`}
          /></span
        ></Button
      >{/each}
    {#if running.length > 2}<Button variant="sidebar" onclick={onRunning}
        >+{running.length - 2} more active</Button
      >{/if}
    {#if !activeCount}<p
        class="dm:mx-0 dm:my-2 dm:px-2.5 dm:text-xs dm:text-draft-dim"
      >
        No active downloads
      </p>{/if}
    {#if failureCount}<Button
        variant="sidebar"
        tone="error"
        class="failure-link"
        onclick={onFailure}
        ><span
          class="dm:flex dm:w-full dm:items-center dm:justify-between dm:gap-1 dm:text-[10px]"
          ><span>{failureCount} download needs attention</span><span>→</span
          ></span
        ></Button
      >{/if}
  </div>
  <span
    class="version dm:flex dm:justify-between dm:text-[9px] dm:text-draft-dim dm:max-[620px]:hidden"
    >DLsite Manager <span>{version}</span></span
  >
</aside>
