<script lang="ts">
  import type { AuditEvent } from "$lib/model/types";
  import { shortDate } from "$lib/utils/format";
  import { auditDetail, auditOutcomeLabel } from "$lib/utils/jobs";

  let {
    events = [],
    loading = false,
  }: {
    events?: AuditEvent[];
    loading?: boolean;
  } = $props();
</script>

{#if loading}
  <div class="empty-state">Loading</div>
{:else if events.length === 0}
  <div class="empty-state">No audit events</div>
{:else}
  <div class="audit-list" aria-label="Recent audit events">
    {#each events as event, index (`${event.at}-${event.operation}-${index}`)}
      <article class="audit-row" data-level={event.level} data-outcome={event.outcome}>
        <div>
          <div class="audit-title">
            <span>{event.operation}</span>
            <strong>{auditOutcomeLabel(event.outcome)}</strong>
          </div>
          <div class="audit-detail">{auditDetail(event)}</div>
        </div>
        <time datetime={event.at}>{shortDate(event.at)}</time>
      </article>
    {/each}
  </div>
{/if}

<style>
  .audit-list {
    display: grid;
    flex: 1 1 auto;
    gap: 0;
    align-content: start;
    grid-auto-rows: max-content;
    min-height: 0;
    padding-right: 4px;
    overflow: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
  }

  .audit-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 14px;
    align-items: center;
    min-height: 58px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }

  .audit-row:last-child {
    border-bottom: 0;
  }

  .audit-row > div {
    min-width: 0;
  }

  .audit-title {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
  }

  .audit-title span {
    color: var(--text);
    font-size: 13px;
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .audit-title strong {
    flex: 0 0 auto;
    color: var(--muted);
    font-size: 12px;
    font-weight: 650;
  }

  .audit-row[data-outcome="succeeded"] .audit-title strong {
    color: var(--accent);
  }

  .audit-row[data-outcome="failed"] .audit-title strong {
    color: var(--danger);
  }

  .audit-row[data-outcome="cancelled"] .audit-title strong {
    color: #d8a62d;
  }

  .audit-detail {
    margin-top: 2px;
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  time {
    color: var(--text-subtle);
    font-size: 12px;
    white-space: nowrap;
  }

  .empty-state {
    padding: 36px 14px;
    color: var(--muted);
    text-align: center;
  }

  @media (max-width: 720px) {
    .audit-row {
      grid-template-columns: 1fr;
      gap: 4px;
    }
  }
</style>
