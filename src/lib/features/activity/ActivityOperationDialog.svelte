<script lang="ts">
  import { exportDiagnostics, operationEvents } from "$lib/api/diagnostics";
  import Button from "$lib/components/workspace/Button.svelte";
  import Disclosure from "$lib/components/workspace/Disclosure.svelte";
  import Icon from "$lib/components/workspace/Icon.svelte";
  import Modal from "$lib/components/workspace/Modal.svelte";
  import type { AuditEvent } from "$lib/model/types";
  import type { DiagnosticTarget } from "$lib/utils/diagnostic-message";
  import { shortDate } from "$lib/utils/format";
  import { auditDetail, auditOutcomeLabel } from "$lib/utils/jobs";

  let { target, onClose }: { target: DiagnosticTarget; onClose: () => void } = $props();
  let events = $state<AuditEvent[]>([]);
  let loading = $state(true);
  let loadError = $state(false);
  let copying = $state(false);
  let copied = $state(false);
  let copyFailed = $state(false);
  let exporting = $state(false);
  let exportStatus = $state("");
  const reference = $derived(`[diagnostic:${target.runId}:${target.operationId}]`);

  $effect(() => {
    const { operationId, runId } = target;
    let active = true;
    loading = true;
    loadError = false;
    events = [];
    void operationEvents(operationId).then((result) => {
      if (active) events = result.filter((event) => event.runId === runId);
    }).catch(() => {
      if (active) loadError = true;
    }).finally(() => {
      if (active) loading = false;
    });
    return () => { active = false; };
  });

  async function copyReference() {
    copying = true;
    copied = false;
    copyFailed = false;
    try {
      await navigator.clipboard.writeText(reference);
      copied = true;
    } catch {
      copyFailed = true;
    } finally {
      copying = false;
    }
  }
  async function save() {
    exporting = true;
    exportStatus = "";
    try {
      const path = await exportDiagnostics(target.runId, target.operationId);
      if (path) exportStatus = `Saved to ${path}`;
    } catch {
      exportStatus = "Could not export diagnostics. Try again or copy the diagnostic ID.";
    } finally {
      exporting = false;
    }
  }
</script>

<Modal title="Activity details" {onClose}>
  <p class="dm:mt-0 dm:wrap-anywhere">{target.message}</p>
  {#if loading}
    <p role="status" class="dm:text-draft-dim">Loading activity…</p>
  {:else if loadError}
    <p role="status" class="dm:text-draft-dim">Activity could not be loaded. You can still export diagnostics or copy the diagnostic ID.</p>
  {:else if !events.length}
    <p role="status" class="dm:text-draft-dim">This operation is no longer in recent activity. Try exporting diagnostics for the saved records.</p>
  {:else}
    <ol aria-label="Operation activity" class="dm:m-0 dm:list-none dm:p-0">
      {#each events as event}
        <li class="dm:border-0 dm:border-t dm:border-solid dm:border-draft-line dm:py-3">
          <div class="dm:flex dm:flex-wrap dm:items-baseline dm:justify-between dm:gap-2">
            <strong class="dm:wrap-anywhere">{event.operation}</strong>
            <span class={event.outcome === "failed" ? "dm:text-draft-error" : "dm:text-draft-dim"}>{auditOutcomeLabel(event.outcome)}</span>
          </div>
          <time class="dm:text-xs dm:text-draft-dim">{shortDate(event.at)}</time>
          <p class="dm:mb-0 dm:mt-1 dm:wrap-anywhere dm:text-sm">{auditDetail(event)}</p>
          {#if Object.keys(event.details).length}
            <Disclosure title="Event details" nested>
              <pre class="dm:m-0 dm:whitespace-pre-wrap dm:wrap-anywhere dm:text-xs">{JSON.stringify(event.details, null, 2)}</pre>
            </Disclosure>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
  <Disclosure title="Diagnostic IDs" nested>
    <dl class="dm:m-0 dm:grid dm:gap-2 dm:text-xs">
      <dt class="dm:text-draft-dim">Run</dt>
      <dd class="dm:m-0 dm:select-text dm:wrap-anywhere dm:font-mono">{target.runId}</dd>
      <dt class="dm:text-draft-dim">Operation</dt>
      <dd class="dm:m-0 dm:select-text dm:wrap-anywhere dm:font-mono">{target.operationId}</dd>
    </dl>
  </Disclosure>
  {#if copyFailed}
    <p role="status" class="dm:text-sm dm:text-draft-dim">Could not copy. Select the diagnostic ID below.</p>
    <textarea aria-label="Diagnostic ID" readonly value={reference} rows="3"
      class="dm:box-border dm:w-full dm:rounded-md dm:border dm:border-solid dm:border-draft-line dm:bg-draft-input dm:p-3 dm:text-xs dm:text-draft-ink dm:draft-focus-control"></textarea>
  {/if}
  {#if exportStatus}<p role="status" class="dm:wrap-anywhere dm:text-sm dm:text-draft-dim">{exportStatus}</p>{/if}
  {#snippet actions()}
    <Button disabled={copying} onclick={copyReference}><Icon name={copied ? "check" : "copy"} />{copied ? "Copied" : "Copy diagnostic ID"}</Button>
    <Button disabled={exporting} onclick={save}><Icon name="downloads" />{exporting ? "Exporting…" : "Export diagnostics"}</Button>
  {/snippet}
</Modal>
