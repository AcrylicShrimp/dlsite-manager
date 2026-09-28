<script lang="ts">
  import { onMount, untrack } from "svelte";
  import UiButton from "$lib/components/ui/Button.svelte";
  import {
    diagnosticRuns,
    exportDiagnostics,
    getDiagnosticSummary,
    operationEvents,
    supportSummary,
  } from "$lib/api/diagnostics";
  import type { AuditEvent } from "$lib/model/types";
  let {
    events = [],
    initialOperationId = "",
  }: { events?: AuditEvent[]; initialOperationId?: string } = $props();
  let runs = $state<{ runId: string; current: boolean }[]>([]);
  let runId = $state("");
  let operationId = $state("");
  let health = $state("Checking diagnostics…");
  let busy = $state(false);
  let status = $state("");
  let savedPath = $state("");
  let summary = $state("");
  let detail = $state<AuditEvent[]>([]);
  const operations = $derived([
    ...new Map(
      events.filter((e) => e.operationId).map((e) => [e.operationId!, e]),
    ).values(),
  ]);
  async function refresh() {
    try {
      const value = await getDiagnosticSummary();
      health =
        value.health.state === "available" &&
        value.health.droppedCritical + value.health.droppedRoutine === 0
          ? ""
          : `Logging: ${value.health.state} · Dropped: ${value.health.droppedCritical + value.health.droppedRoutine} · Older records removed from memory: ${value.health.ringEvicted}`;
      runs = await diagnosticRuns();
    } catch {
      health =
        "Diagnostics storage unavailable. You can still copy a support summary.";
    }
  }
  onMount(() => {
    void refresh();
  });
  $effect(() => {
    const id = initialOperationId;
    untrack(() => {
      operationId = id;
      void inspect();
    });
  });
  async function save() {
    busy = true;
    status = "";
    try {
      const path = await exportDiagnostics(runId, operationId);
      if (path) {
        savedPath = path;
        status = `Saved to ${path}`;
      }
    } catch {
      status =
        "Could not save diagnostics. Try a new filename in a writable folder, or copy the summary below.";
    } finally {
      busy = false;
      await refresh();
    }
  }
  async function copyPath() {
    try {
      await navigator.clipboard.writeText(savedPath);
      status = "Saved path copied.";
    } catch {
      status = `Saved to ${savedPath}`;
    }
  }
  async function copy() {
    summary = await supportSummary();
    try {
      await navigator.clipboard.writeText(summary);
      status = "Support summary copied.";
    } catch {
      status = "Copy the support summary from the text field below.";
    }
  }
  async function inspect() {
    if (!operationId) {
      detail = [];
      return;
    }
    try {
      detail = await operationEvents(operationId);
    } catch {
      status = "Operation details unavailable. Try the support summary.";
    }
  }
</script>

<section class="diagnostics" aria-label="Diagnostics">
  <div class="actions">
    <strong>Diagnostics</strong>
    <UiButton size="small" disabled={busy} onclick={save}
      >Export diagnostics</UiButton
    >
    <UiButton size="small" variant="secondary" onclick={copy}
      >Copy support summary</UiButton
    >
    <UiButton size="small" variant="secondary" onclick={refresh}
      >Refresh</UiButton
    >
  </div>
  {#if health}<p role="status">{health}</p>{/if}
  <div class="actions">
    <label
      >Run <select
        bind:value={runId}
        onchange={() => {
          operationId = "";
          detail = [];
        }}
      >
        <option value="">Current run</option>
        {#each runs.filter((r) => !r.current) as run}<option value={run.runId}
            >{run.runId}</option
          >{/each}
      </select></label
    >
    {#if !runId}
      <label
        >Operation <select bind:value={operationId} onchange={inspect}>
          <option value="">All recent operations</option>
          {#each operations as event}<option value={event.operationId!}
              >{event.operation} · {event.outcome} · {event.at}</option
            >{/each}
        </select></label
      >
    {/if}
  </div>
  {#if operationId}<pre aria-label="Operation details">{JSON.stringify(
        detail,
        null,
        2,
      )}</pre>{/if}
  {#if status}<p role="status">{status}</p>{/if}
  {#if savedPath}<UiButton size="small" variant="secondary" onclick={copyPath}
      >Copy saved path</UiButton
    >{/if}
  {#if summary}<textarea
      aria-label="Support summary"
      readonly
      value={summary}
      rows="6"></textarea>{/if}
</section>

<style>
  .diagnostics {
    padding: 0;
    min-width: 0;
  }
  .actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
  }
  p {
    color: var(--muted);
    font-size: 12px;
    overflow-wrap: anywhere;
    margin: 8px 0;
  }
  label {
    font-size: 12px;
    max-width: 100%;
  }
  select {
    background: var(--panel);
    color: var(--text);
    border: 1px solid var(--border);
    max-width: 100%;
  }
  textarea,
  pre {
    width: 100%;
    box-sizing: border-box;
    max-height: 180px;
    overflow: auto;
    background: var(--panel);
    color: var(--text);
    font-size: 12px;
  }
</style>
