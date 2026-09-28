<script lang="ts">
  import { onMount } from "svelte";
  import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
  import App from "../../routes/+page.svelte";
  import { makeFixture } from "../fixtures/redesign";
  import {
    makeDetail,
    emptyFilters,
    queryProducts,
    facetsFor,
  } from "../redesign/library-preview";
  import { isActiveJob } from "$lib/utils/jobs";
  import type {
    ProductListRequest,
    SaveAccountRequest,
    StartWorkDownloadRequest,
  } from "$lib/api/tauri";
  import type { Account, Product, JobSnapshot } from "$lib/model/types";
  let ready = $state(false);
  onMount(() => {
    const fixture = makeFixture("long");
    let products: Product[] = Array.from({ length: 3 }, (_, batch) =>
      fixture.products.map((p, i) => ({
        ...p,
        workId: `RJ${15000000 + batch * 72 + i}`,
        title: batch ? `${p.title} · Collection ${batch + 1}` : p.title,
      })),
    ).flat();
    let accounts = fixture.accounts.map((a) => ({ ...a }));
    let jobs = fixture.jobs.map((j) => ({
      ...j,
      title:
        j.kind === "accountSync" ? `Sync ${j.metadata.accountId}` : j.title,
    }));
    let settings = {
      libraryRoot: "/tmp/dlsite-library",
      downloadRoot: "/tmp/dlsite-downloads",
    };
    function filters(request: ProductListRequest) {
      return {
        ...emptyFilters(),
        accounts: request.accountIds,
        types: request.typeGroups,
        ages: request.ageCategories,
        sources: request.sourceGroups,
        makers: request.makerNames,
        tags: request.customTagNames,
        excludedTags: request.excludedCustomTagNames,
        sort: request.sort as ReturnType<typeof emptyFilters>["sort"],
      };
    }
    mockIPC(
      (command, payload) => {
        const args = payload as Record<string, unknown>;
        if (command === "report_frontend_failure") return;
        if (command === "plugin:app|name") return "DLsite Manager";
        if (command === "plugin:app|version") return "3.4.0";
        if (command === "plugin:app|tauri_version") return "2.11.5";
        if (command === "plugin:app|identifier")
          return "com.acrylicshrimp.dlsite-manager";
        if (command === "plugin:path|resolve_directory")
          return "/tmp/Downloads";
        if (command === "get_settings") return settings;
        if (command === "save_settings") {
          settings = args.settings as typeof settings;
          return settings;
        }
        if (command === "list_accounts") return accounts;
        if (
          command === "list_products" ||
          command === "list_product_filter_facets"
        ) {
          const request = args.request as ProductListRequest;
          if (command === "list_product_filter_facets")
            return facetsFor(products, request.search ?? "", filters(request));
          const matched = queryProducts(
            products,
            request.search ?? "",
            filters(request),
          );
          return {
            products: matched.slice(
              request.offset,
              request.offset + request.limit,
            ),
            totalCount: matched.length,
          };
        }
        if (command === "get_product_detail") {
          const product = products.find(
            (p) => p.workId === (args.request as { workId: string }).workId,
          );
          if (!product) throw new Error("Work not found");
          return makeDetail(product);
        }
        if (command === "set_product_custom_tags") {
          const request = args.request as { workId: string; tags: string[] };
          const tags = request.tags.map((name) => ({ name }));
          products = products.map((p) =>
            p.workId === request.workId ? { ...p, customTags: tags } : p,
          );
          return tags;
        }
        if (command === "save_account") {
          const request = args.request as SaveAccountRequest;
          if (request.label === "fail")
            throw new Error("Could not save account (simulated)");
          const previous = accounts.find((a) => a.id === request.id);
          const account = {
            ...(previous ?? fixture.accounts[0]),
            id: request.id ?? crypto.randomUUID(),
            label: request.label,
            loginName: request.loginName,
            hasCredential:
              Boolean(request.password) || Boolean(previous?.hasCredential),
          } satisfies Account;
          accounts = [...accounts.filter((a) => a.id !== account.id), account];
          return account;
        }
        if (command === "set_account_enabled") {
          const request = args.request as {
            accountId: string;
            enabled: boolean;
          };
          accounts = accounts.map((a) =>
            a.id === request.accountId ? { ...a, enabled: request.enabled } : a,
          );
          return;
        }
        if (command === "remove_account") {
          const request = args.request as { accountId: string };
          accounts = accounts.filter((a) => a.id !== request.accountId);
          return {
            accountId: request.accountId,
            label: "Removed account",
            credentialDeleted: false,
          };
        }
        if (command === "list_jobs") return jobs;
        if (command === "cancel_job") {
          const job = jobs.find(
            (j) => j.id === (args.request as { jobId: string }).jobId,
          )!;
          job.status = "cancelling";
          return { outcome: "requested", snapshot: job };
        }
        if (command === "clear_finished_jobs") {
          const count = jobs.length;
          jobs = jobs.filter(isActiveJob);
          return { removedCount: count - jobs.length };
        }
        if (
          command === "start_work_download" ||
          command === "start_account_sync"
        ) {
          const request = args.request as StartWorkDownloadRequest;
          const job: JobSnapshot = {
            ...fixture.jobs[0],
            id: crypto.randomUUID(),
            status: "queued",
            phase: null,
            progress: null,
            kind:
              command === "start_account_sync" ? "accountSync" : "workDownload",
            metadata: { workId: request.workId, accountId: request.accountId },
          };
          jobs.push(job);
          return { jobId: job.id };
        }
        if (command === "preview_bulk_work_download")
          return {
            requestedCount: 12,
            plannedCount: 12,
            skippedDownloadedCount: 0,
            skippedQueuedCount: 0,
            failedCount: 0,
            totalCount: 12,
            knownExpectedBytes: 1024,
            totalExpectedBytes: 1024,
            unknownSizeCount: 0,
          };
        if (command === "start_bulk_work_download")
          return { jobId: "bulk-preview" };
        if (command === "list_audit_events") return fixture.events;
        if (command === "get_audit_log_dir")
          return { path: "/tmp/dlsite-logs" };
        if (command === "diagnostic_summary")
          return {
            runId: "run-story",
            health: {
              state: "available",
              droppedCritical: 0,
              droppedRoutine: 0,
              ringEvicted: 0,
            },
          };
        if (command === "diagnostic_runs")
          return [{ runId: "run-story", current: true }];
        if (command === "diagnostic_operation")
          return fixture.events.filter(
            (e) => e.operationId === args.operationId,
          );
        if (command === "export_diagnostics")
          return "/tmp/dlsite-diagnostics.zip";
        if (command === "save_product_cover") return true;
        if (command === "plugin:dialog|open") return null;
        if (command === "plugin:updater|check") return null;
        if (
          [
            "open_external_url",
            "open_work_download",
            "open_audit_log_dir",
            "submit_two_factor_code",
            "cancel_two_factor",
          ].includes(command)
        )
          return;
        throw new Error(`Unimplemented Storybook command: ${command}`);
      },
      { shouldMockEvents: true },
    );
    ready = true;
    return () => clearMocks();
  });
</script>

{#if ready}<App />{/if}
