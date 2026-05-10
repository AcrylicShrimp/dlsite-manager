<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy, onMount } from "svelte";

  type AppSettings = {
    libraryRoot: string | null;
    downloadRoot: string | null;
  };

  type Account = {
    id: string;
    label: string;
    loginName: string | null;
    hasCredential: boolean;
    enabled: boolean;
    createdAt: string;
    updatedAt: string;
    lastLoginAt: string | null;
    lastSyncAt: string | null;
  };

  type ProductOwner = {
    accountId: string;
    label: string;
    purchasedAt: string | null;
  };

  type ProductCreditGroup = {
    kind: string;
    label: string;
    names: string[];
  };

  type Product = {
    workId: string;
    title: string;
    makerName: string | null;
    workType: string | null;
    ageCategory: string | null;
    thumbnailUrl: string | null;
    publishedAt: string | null;
    updatedAt: string | null;
    earliestPurchasedAt: string | null;
    latestPurchasedAt: string | null;
    creditGroups: ProductCreditGroup[];
    owners: ProductOwner[];
  };

  type ProductListPage = {
    totalCount: number;
    products: Product[];
  };

  type JobStatus = "queued" | "running" | "cancelling" | "succeeded" | "failed" | "cancelled";

  type JobProgress = {
    current: number | null;
    total: number | null;
    unit: string | null;
  };

  type JobFailure = {
    code: string | null;
    message: string;
    details: Record<string, unknown>;
  };

  type JobSnapshot = {
    id: string;
    kind: string;
    title: string;
    status: JobStatus;
    phase: string | null;
    progress: JobProgress | null;
    metadata: Record<string, unknown>;
    output: Record<string, unknown> | null;
    error: JobFailure | null;
    cancellable: boolean;
    createdAt: string;
    startedAt: string | null;
    finishedAt: string | null;
  };

  type JobEvent = {
    sequence: number;
    eventKind: string;
    jobId: string;
    kind: string;
    status: JobStatus;
    phase: string | null;
    progress: JobProgress | null;
    message: string | null;
    log: { message: string } | null;
    snapshot: JobSnapshot;
  };

  type StartJobResponse = {
    jobId: string;
  };

  type ToastKind = "success" | "error" | "info";

  type Toast = {
    id: string;
    kind: ToastKind;
    message: string;
  };

  type ProductCreditField = {
    key: string;
    label: string;
    value: string;
    missing: boolean;
  };

  type View = "library" | "accounts" | "activity" | "settings";

  const creditFieldDefinitions = [
    { key: "maker", label: "Maker" },
    { key: "voice", label: "CV" },
    { key: "illust", label: "Illust" },
    { key: "scenario", label: "Scenario" },
    { key: "creator", label: "Creator" },
    { key: "music", label: "Music" },
    { key: "other", label: "Other" },
  ] as const;

  let activeView = $state<View>("library");

  let libraryRoot = $state("");
  let downloadRoot = $state("");
  let settingsLoading = $state(true);
  let settingsSaving = $state(false);

  let accounts = $state<Account[]>([]);
  let accountsLoading = $state(true);
  let accountSaving = $state(false);
  let editingAccountId = $state<string | null>(null);
  let accountLabel = $state("");
  let accountLoginName = $state("");
  let accountPassword = $state("");
  let accountRememberPassword = $state(true);
  let accountEnabled = $state(true);

  let products = $state<Product[]>([]);
  let totalProducts = $state(0);
  let productsLoading = $state(true);
  let productSearch = $state("");
  let selectedAccountId = $state("");
  let selectedAgeCategory = $state("");
  let productSort = $state("titleAsc");

  let jobs = $state<JobSnapshot[]>([]);
  let jobsLoading = $state(true);
  let jobMessages = $state<Record<string, string>>({});
  let toasts = $state<Toast[]>([]);

  let toastSequence = 0;
  const toastTimers = new Map<string, ReturnType<typeof setTimeout>>();

  onMount(() => {
    void loadInitial();

    let unlisten: (() => void) | null = null;
    let disposed = false;

    void listen<JobEvent>("dm-job-event", (event) => {
      void handleJobEvent(event.payload);
    }).then((cleanup) => {
      if (disposed) {
        cleanup();
      } else {
        unlisten = cleanup;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  onDestroy(() => {
    for (const timer of toastTimers.values()) {
      clearTimeout(timer);
    }
    toastTimers.clear();
  });

  async function loadInitial() {
    await Promise.all([loadSettings(), loadAccounts(), loadProducts(), loadJobs()]);
  }

  async function loadSettings() {
    settingsLoading = true;

    try {
      const settings = await invoke<AppSettings>("get_settings");
      libraryRoot = settings.libraryRoot ?? "";
      downloadRoot = settings.downloadRoot ?? "";
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      settingsLoading = false;
    }
  }

  async function saveSettings(event: Event) {
    event.preventDefault();
    settingsSaving = true;

    try {
      const settings = await invoke<AppSettings>("save_settings", {
        settings: {
          libraryRoot: valueOrNull(libraryRoot),
          downloadRoot: valueOrNull(downloadRoot),
        },
      });
      libraryRoot = settings.libraryRoot ?? "";
      downloadRoot = settings.downloadRoot ?? "";
      notifySuccess("Settings saved");
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      settingsSaving = false;
    }
  }

  async function loadAccounts() {
    accountsLoading = true;

    try {
      accounts = await invoke<Account[]>("list_accounts");
      if (selectedAccountId && !accounts.some((account) => account.id === selectedAccountId)) {
        selectedAccountId = "";
      }
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      accountsLoading = false;
    }
  }

  async function saveAccount(event: Event) {
    event.preventDefault();
    accountSaving = true;

    try {
      const account = await invoke<Account>("save_account", {
        request: {
          id: editingAccountId,
          label: accountLabel,
          loginName: valueOrNull(accountLoginName),
          password: valueOrNull(accountPassword),
          rememberPassword: accountRememberPassword,
          enabled: accountEnabled,
        },
      });
      notifySuccess(editingAccountId ? "Account updated" : "Account added");
      editAccount(account);
      accountPassword = "";
      await loadAccounts();
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      accountSaving = false;
    }
  }

  async function setAccountEnabled(account: Account, enabled: boolean) {
    try {
      await invoke("set_account_enabled", {
        request: {
          accountId: account.id,
          enabled,
        },
      });
      await loadAccounts();
      await loadProducts();
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  function editAccount(account: Account) {
    editingAccountId = account.id;
    accountLabel = account.label;
    accountLoginName = account.loginName ?? "";
    accountPassword = "";
    accountRememberPassword = account.hasCredential;
    accountEnabled = account.enabled;
  }

  function resetAccountForm() {
    editingAccountId = null;
    accountLabel = "";
    accountLoginName = "";
    accountPassword = "";
    accountRememberPassword = true;
    accountEnabled = true;
  }

  async function loadProducts() {
    productsLoading = true;

    try {
      const page = await invoke<ProductListPage>("list_products", {
        request: {
          search: valueOrNull(productSearch),
          accountId: selectedAccountId || null,
          ageCategory: selectedAgeCategory || null,
          sort: productSort,
          limit: 100,
          offset: 0,
        },
      });
      products = page.products;
      totalProducts = page.totalCount;
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      productsLoading = false;
    }
  }

  async function loadJobs() {
    jobsLoading = true;

    try {
      jobs = await invoke<JobSnapshot[]>("list_jobs");
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      jobsLoading = false;
    }
  }

  async function handleJobEvent(event: JobEvent) {
    jobs = upsertJob(jobs, event.snapshot);

    if (event.message) {
      jobMessages = {
        ...jobMessages,
        [event.jobId]: event.message,
      };
    }

    if (event.kind === "accountSync" && isTerminalJob(event.snapshot)) {
      await Promise.all([loadAccounts(), loadProducts()]);
    }
  }

  async function searchProducts(event: Event) {
    event.preventDefault();
    await loadProducts();
  }

  async function copyWorkId(workId: string) {
    try {
      await navigator.clipboard.writeText(workId);
      notifySuccess(`Copied ${workId}`);
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function copyCreditField(field: ProductCreditField) {
    if (field.missing) {
      return;
    }

    try {
      await navigator.clipboard.writeText(field.value);
      notifySuccess(`Copied ${field.label}`);
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function syncAccount(account: Account): Promise<boolean> {
    try {
      const response = await invoke<StartJobResponse>("start_account_sync", {
        request: {
          accountId: account.id,
          password: editingAccountId === account.id ? valueOrNull(accountPassword) : null,
        },
      });
      notifyInfo("Sync queued");
      jobMessages = {
        ...jobMessages,
        [response.jobId]: "Sync queued",
      };
      accountPassword = "";
      await loadJobs();
      return true;
    } catch (err) {
      notifyError(errorMessage(err));
      return false;
    }
  }

  async function syncEnabledAccounts() {
    const enabledAccounts = accounts.filter(
      (account) => account.enabled && !activeAccountSyncJob(account.id),
    );

    for (const account of enabledAccounts) {
      const started = await syncAccount(account);
      if (!started) {
        break;
      }
    }
  }

  async function cancelAccountSync(account: Account) {
    const job = activeAccountSyncJob(account.id);

    if (!job) {
      return;
    }

    await cancelJob(job);
  }

  async function cancelJob(job: JobSnapshot) {
    try {
      await invoke("cancel_job", {
        request: {
          jobId: job.id,
        },
      });
      notifyInfo("Cancellation requested");
      await loadJobs();
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function clearFinishedJobs() {
    try {
      await invoke("clear_finished_jobs");
      await loadJobs();
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  function accountStatusLabel(account: Account) {
    const activeJob = activeAccountSyncJob(account.id);

    if (activeJob) {
      return jobLabel(activeJob);
    }

    if (!account.enabled) {
      return "Disabled";
    }

    const latestJob = latestAccountSyncJob(account.id);

    if (latestJob?.status === "failed") {
      return "Sync failed";
    }

    if (latestJob?.status === "cancelled") {
      return "Sync cancelled";
    }

    if (account.lastSyncAt) {
      return "Synced";
    }

    return "Not synced";
  }

  function accountStatusTone(account: Account) {
    const activeJob = activeAccountSyncJob(account.id);

    if (activeJob) {
      return "syncing";
    }

    if (!account.enabled) {
      return "disabled";
    }

    const latestJob = latestAccountSyncJob(account.id);

    if (latestJob?.status === "failed") {
      return "failed";
    }

    if (latestJob?.status === "cancelled") {
      return "warning";
    }

    if (account.lastSyncAt) {
      return "synced";
    }

    return "idle";
  }

  function accountLoginLabel(account: Account) {
    return account.loginName?.trim() || "No login name";
  }

  function accountLastSyncLabel(account: Account) {
    return account.lastSyncAt ? shortDate(account.lastSyncAt) : "Never synced";
  }

  function accountCredentialLabel(account: Account) {
    return account.hasCredential ? "Saved" : "Not saved";
  }

  function accountEnabledLabel(account: Account) {
    return account.enabled ? "Enabled" : "Disabled";
  }

  function enabledAccountCount() {
    return accounts.filter((account) => account.enabled).length;
  }

  function credentialedAccountCount() {
    return accounts.filter((account) => account.hasCredential).length;
  }

  function syncingAccountCount() {
    return accounts.filter((account) => activeAccountSyncJob(account.id)).length;
  }

  function upsertJob(currentJobs: JobSnapshot[], job: JobSnapshot) {
    const index = currentJobs.findIndex((item) => item.id === job.id);

    if (index === -1) {
      return [...currentJobs, job];
    }

    const next = currentJobs.slice();
    next[index] = job;
    return next;
  }

  function accountSyncJobs(accountId: string) {
    return jobs.filter((job) => job.kind === "accountSync" && jobAccountId(job) === accountId);
  }

  function activeAccountSyncJob(accountId: string) {
    return [...accountSyncJobs(accountId)].reverse().find(isActiveJob) ?? null;
  }

  function latestAccountSyncJob(accountId: string) {
    return [...accountSyncJobs(accountId)].reverse()[0] ?? null;
  }

  function visibleJobs(limit = 20) {
    return [...jobs].reverse().slice(0, limit);
  }

  function hasSyncableEnabledAccount() {
    return accounts.some((account) => account.enabled && !activeAccountSyncJob(account.id));
  }

  function isActiveJob(job: JobSnapshot) {
    return job.status === "queued" || job.status === "running" || job.status === "cancelling";
  }

  function isTerminalJob(job: JobSnapshot) {
    return job.status === "succeeded" || job.status === "failed" || job.status === "cancelled";
  }

  function jobAccountId(job: JobSnapshot) {
    const accountId = job.metadata.accountId;
    return typeof accountId === "string" ? accountId : null;
  }

  function jobAccountLabel(job: JobSnapshot) {
    const accountId = jobAccountId(job);
    const account = accounts.find((item) => item.id === accountId);
    return account?.label ?? accountId ?? job.title;
  }

  function jobLabel(job: JobSnapshot) {
    if (job.status === "queued") {
      return "Queued";
    }

    if (job.status === "cancelling") {
      return "Cancelling";
    }

    if (job.status === "failed") {
      return "Failed";
    }

    if (job.status === "cancelled") {
      return "Cancelled";
    }

    if (job.status === "succeeded") {
      const cachedCount = jobOutputNumber(job, "cachedWorkCount");
      return typeof cachedCount === "number" ? `Synced ${cachedCount} works` : "Synced";
    }

    switch (job.phase) {
      case "loggingIn":
        return "Signing in";
      case "loadingCount":
        return "Checking library";
      case "loadingPurchases":
        return "Loading purchases";
      case "loadingWorks":
        return `Loading ${job.progress?.total ?? 0} works`;
      case "committing":
        return "Saving cache";
      case "completed":
        return "Completing";
      default:
        return "Syncing";
    }
  }

  function jobDetail(job: JobSnapshot) {
    if (job.error?.message) {
      return job.error.message;
    }

    return jobMessages[job.id] ?? shortDate(job.finishedAt ?? job.startedAt ?? job.createdAt);
  }

  function jobOutputNumber(job: JobSnapshot, key: string) {
    const value = job.output?.[key];
    return typeof value === "number" ? value : null;
  }

  function productType(product: Product) {
    const raw = product.workType?.trim() || "";
    const upper = raw.toUpperCase();
    const normalized = raw.toLowerCase().replace(/[\s_-]+/g, "");

    if (matchesAny(normalized, ["sou", "audio", "voice", "asmr", "music", "sound"])) {
      return { label: upper === "SOU" ? "Voice" : raw || "Audio", tone: "audio" };
    }

    if (matchesAny(normalized, ["mov", "movie", "video", "anime", "voicecomic", "vcomic"])) {
      return { label: raw || "Video", tone: "video" };
    }

    if (
      matchesAny(normalized, [
        "gam",
        "game",
        "rpg",
        "adv",
        "action",
        "acn",
        "puzzle",
        "puz",
        "quiz",
        "simulation",
        "slg",
        "shooter",
        "stg",
        "tabletop",
        "typing",
      ])
    ) {
      return { label: raw || "Game", tone: "game" };
    }

    if (
      matchesAny(normalized, [
        "cg",
        "icg",
        "image",
        "illust",
        "comic",
        "com",
        "manga",
        "mng",
        "gekiga",
        "pdf",
        "novel",
        "digitalnovel",
        "book",
      ])
    ) {
      return { label: raw || "Comic", tone: "image" };
    }

    if (matchesAny(normalized, ["software", "tool", "utility", "etc", "other"])) {
      return { label: raw || "Other", tone: "other" };
    }

    return { label: raw || "Other", tone: "other" };
  }

  function matchesAny(value: string, needles: string[]) {
    return needles.some((needle) => value.includes(needle));
  }

  function ageTone(value: string | null) {
    switch (value) {
      case "all":
        return "all";
      case "r15":
        return "r15";
      case "r18":
        return "r18";
      default:
        return "unknown";
    }
  }

  function ageLabel(value: string | null) {
    switch (value) {
      case "all":
        return "All ages";
      case "r15":
        return "R-15";
      case "r18":
        return "R-18";
      default:
        return "";
    }
  }

  function creditText(group: ProductCreditGroup) {
    return group.names.join(", ");
  }

  function productCreditFields(product: Product): ProductCreditField[] {
    return creditFieldDefinitions.map((definition) => {
      const value =
        definition.key === "maker"
          ? product.makerName?.trim() || ""
          : creditTextForKind(product, definition.key);

      return {
        ...definition,
        value: value || "-",
        missing: !value,
      };
    });
  }

  function creditTextForKind(product: Product, kind: string) {
    const group = product.creditGroups?.find((item) => item.kind === kind);
    return group ? creditText(group).trim() : "";
  }

  function creditTooltip(field: ProductCreditField) {
    return field.missing ? `${field.label}: Not available` : `${field.label}: ${field.value}`;
  }

  function notifySuccess(message: string) {
    pushToast("success", message);
  }

  function notifyInfo(message: string) {
    pushToast("info", message);
  }

  function notifyError(message: string) {
    pushToast("error", message, 7000);
  }

  function pushToast(kind: ToastKind, message: string, duration = 3600) {
    const id = `toast-${Date.now()}-${toastSequence++}`;
    const toast = { id, kind, message };
    toasts = [toast, ...toasts].slice(0, 5);

    const timer = setTimeout(() => dismissToast(id), duration);
    toastTimers.set(id, timer);
    clearOrphanedToastTimers();
  }

  function dismissToast(id: string) {
    toasts = toasts.filter((toast) => toast.id !== id);

    const timer = toastTimers.get(id);
    if (timer) {
      clearTimeout(timer);
      toastTimers.delete(id);
    }
  }

  function clearOrphanedToastTimers() {
    const visibleToastIds = new Set(toasts.map((toast) => toast.id));

    for (const [id, timer] of toastTimers.entries()) {
      if (!visibleToastIds.has(id)) {
        clearTimeout(timer);
        toastTimers.delete(id);
      }
    }
  }

  function viewEyebrow(view: View) {
    switch (view) {
      case "library":
        return "Collection";
      case "accounts":
        return "Sources";
      case "activity":
        return "Jobs";
      case "settings":
        return "Application";
    }
  }

  function viewTitle(view: View) {
    switch (view) {
      case "library":
        return "Library";
      case "accounts":
        return "Accounts";
      case "activity":
        return "Activity";
      case "settings":
        return "Settings";
    }
  }

  function valueOrNull(value: string) {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
  }

  function shortDate(value: string | null) {
    if (!value) {
      return "";
    }

    return value.replace("T", " ").replace(/\.\d+Z$/, "Z");
  }

  function errorMessage(err: unknown) {
    return err instanceof Error ? err.message : String(err);
  }
</script>

<svelte:head>
  <title>dlsite-manager</title>
</svelte:head>

<main class="app-shell">
  <aside class="sidebar" aria-label="Primary">
    <div class="brand">dlsite-manager</div>
    <nav class="main-nav" aria-label="Main">
      <button
        class:active={activeView === "library"}
        type="button"
        onclick={() => (activeView = "library")}
      >
        Library
      </button>
      <button
        class:active={activeView === "accounts"}
        type="button"
        onclick={() => (activeView = "accounts")}
      >
        Accounts
      </button>
      <button
        class:active={activeView === "settings"}
        type="button"
        onclick={() => (activeView = "settings")}
      >
        Settings
      </button>
    </nav>
    <nav class="utility-nav" aria-label="Utility">
      <button
        class:active={activeView === "activity"}
        type="button"
        onclick={() => (activeView = "activity")}
      >
        Activity
      </button>
    </nav>
  </aside>

  <section class="workspace">
    <header class="workspace-header">
      <div>
        <p class="eyebrow">{viewEyebrow(activeView)}</p>
        <h1>{viewTitle(activeView)}</h1>
      </div>
    </header>

    {#if activeView === "library"}
      <section class="product-area" aria-label="Library">
        <form class="toolbar" onsubmit={searchProducts}>
          <input
            type="search"
            autocomplete="off"
            spellcheck="false"
            placeholder="Search title, maker, work ID"
            bind:value={productSearch}
          />
          <select bind:value={selectedAccountId} onchange={loadProducts}>
            <option value="">All accounts</option>
            {#each accounts as account (account.id)}
              <option value={account.id}>{account.label}</option>
            {/each}
          </select>
          <select bind:value={selectedAgeCategory} onchange={loadProducts}>
            <option value="">Any age</option>
            <option value="all">All ages</option>
            <option value="r15">R-15</option>
            <option value="r18">R-18</option>
          </select>
          <select bind:value={productSort} onchange={loadProducts}>
            <option value="titleAsc">Title</option>
            <option value="latestPurchaseDesc">Latest purchase</option>
            <option value="publishedAtDesc">Published</option>
          </select>
          <button type="submit" disabled={productsLoading}>Search</button>
          <button
            class="secondary"
            type="button"
            onclick={loadProducts}
            disabled={productsLoading}
          >
            Reload
          </button>
          <button
            type="button"
            onclick={syncEnabledAccounts}
            disabled={accountsLoading || jobsLoading || !hasSyncableEnabledAccount()}
          >
            Sync
          </button>
        </form>

        <div class="list-header">
          <span>{totalProducts} products</span>
        </div>

        {#if productsLoading}
          <div class="empty-state">Loading</div>
        {:else if products.length === 0}
          <div class="empty-state">No products</div>
        {:else}
          <div class="product-table" aria-label="Cached products">
            {#each products as product (product.workId)}
              {@const typeInfo = productType(product)}
              <article class="product-card" data-tone={typeInfo.tone}>
                <div class="type-belt" aria-hidden="true"></div>
                <div class="thumb" aria-hidden="true">
                  {#if product.thumbnailUrl}
                    <img src={product.thumbnailUrl} alt="" loading="lazy" />
                  {:else}
                    <span>?</span>
                  {/if}
                </div>
                <div class="product-main">
                  <div class="product-title-row">
                    <div class="product-title" title={product.title}>{product.title}</div>
                    <button
                      class="work-id"
                      type="button"
                      title={`Copy ${product.workId}`}
                      onclick={() => copyWorkId(product.workId)}
                    >
                      {product.workId}
                    </button>
                  </div>
                  <div class="product-meta">
                    {#each productCreditFields(product) as field (field.key)}
                      <button
                        class="credit-row"
                        type="button"
                        title={creditTooltip(field)}
                        aria-label={field.missing ? `${field.label} is not available` : `Copy ${field.label}: ${field.value}`}
                        disabled={field.missing}
                        onclick={() => copyCreditField(field)}
                      >
                        <span class="credit-label">{field.label}</span>
                        <span class:missing={field.missing} class="credit-value">{field.value}</span>
                      </button>
                    {/each}
                  </div>
                  <div class="labeled-row" aria-label="Classifications">
                    <span class="credit-label">Tags</span>
                    <div class="chip-row">
                      <span class="chip type-chip">{typeInfo.label}</span>
                      {#if ageLabel(product.ageCategory)}
                        <span class="chip age-chip" data-age={ageTone(product.ageCategory)}>
                          {ageLabel(product.ageCategory)}
                        </span>
                      {/if}
                    </div>
                  </div>
                  <div class="product-footer">
                    <div class="labeled-row owner-row" aria-label="Owners">
                      <span class="credit-label">Owned by</span>
                      <div class="owner-list">
                        {#each product.owners as owner (owner.accountId)}
                          <span title={owner.purchasedAt ? `${owner.label}: ${shortDate(owner.purchasedAt)}` : owner.label}>
                            {owner.label}
                          </span>
                        {/each}
                      </div>
                    </div>
                    <div class="product-actions" aria-label="Actions">
                      <button class="small" type="button" disabled title="Download handling is a later slice">
                        Download
                      </button>
                      <button class="secondary small menu-button" type="button" disabled title="More actions">
                        ...
                      </button>
                    </div>
                  </div>
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </section>
    {:else if activeView === "accounts"}
      <div class="accounts-layout">
        <section class="accounts-panel account-list-panel" aria-label="Accounts">
          <div class="panel-title account-panel-title">
            <div>
              <h2>Account sources</h2>
              <p>{enabledAccountCount()} enabled of {accounts.length}</p>
            </div>
            <div class="panel-actions">
              <button
                class="secondary small"
                type="button"
                onclick={loadAccounts}
                disabled={accountsLoading || accountSaving}
              >
                Reload
              </button>
              <button
                class="small"
                type="button"
                onclick={syncEnabledAccounts}
                disabled={accountsLoading || jobsLoading || !hasSyncableEnabledAccount()}
              >
                Sync all
              </button>
            </div>
          </div>

          <div class="account-summary-strip" aria-label="Account summary">
            <div class="account-stat">
              <span>{accounts.length}</span>
              <small>Total</small>
            </div>
            <div class="account-stat">
              <span>{enabledAccountCount()}</span>
              <small>Enabled</small>
            </div>
            <div class="account-stat">
              <span>{credentialedAccountCount()}</span>
              <small>Credentials</small>
            </div>
            <div class="account-stat">
              <span>{syncingAccountCount()}</span>
              <small>Syncing</small>
            </div>
          </div>

          <div class="account-list">
            {#if accountsLoading}
              <div class="empty-state compact">Loading</div>
            {:else if accounts.length === 0}
              <div class="empty-state compact">No accounts</div>
            {:else}
              {#each accounts as account (account.id)}
                {@const activeSyncJob = activeAccountSyncJob(account.id)}
                <article
                  class="account-row"
                  class:disabled={!account.enabled}
                  class:selected={editingAccountId === account.id}
                >
                  <button class="account-name" type="button" onclick={() => editAccount(account)}>
                    <span class="account-identity">
                      <span title={account.label}>{account.label}</span>
                      <small title={accountLoginLabel(account)}>{accountLoginLabel(account)}</small>
                      <span class:disabled={!account.enabled} class="account-enabled-pill">
                        {accountEnabledLabel(account)}
                      </span>
                    </span>
                  </button>
                  <div class="account-meta-grid">
                    <div>
                      <span>Status</span>
                      <strong class={`account-status-text ${accountStatusTone(account)}`} title={accountStatusLabel(account)}>
                        {accountStatusLabel(account)}
                      </strong>
                    </div>
                    <div>
                      <span>Credential</span>
                      <strong title={accountCredentialLabel(account)}>{accountCredentialLabel(account)}</strong>
                    </div>
                    <div>
                      <span>Last sync</span>
                      <strong title={accountLastSyncLabel(account)}>{accountLastSyncLabel(account)}</strong>
                    </div>
                  </div>
                  <div class="account-actions">
                    <button
                      class="secondary small"
                      type="button"
                      onclick={() => setAccountEnabled(account, !account.enabled)}
                      disabled={Boolean(activeSyncJob)}
                    >
                      {account.enabled ? "Disable" : "Enable"}
                    </button>
                    {#if activeSyncJob}
                      <button
                        class="secondary small"
                        type="button"
                        onclick={() => cancelAccountSync(account)}
                        disabled={!activeSyncJob.cancellable || activeSyncJob.status === "cancelling"}
                      >
                        Cancel
                      </button>
                    {:else}
                      <button
                        class="small"
                        type="button"
                        onclick={() => syncAccount(account)}
                        disabled={!account.enabled}
                      >
                        Sync
                      </button>
                    {/if}
                  </div>
                </article>
              {/each}
            {/if}
          </div>
        </section>

        <section class="accounts-panel account-editor" aria-label="Account editor">
          <div class="panel-title account-panel-title">
            <div>
              <h2>{editingAccountId ? "Account details" : "Add account"}</h2>
              <p>{editingAccountId ? "Editing selected source" : "New DLsite source"}</p>
            </div>
            <button class="secondary small" type="button" onclick={resetAccountForm} disabled={accountSaving}>
              New
            </button>
          </div>
          <form class="account-form" onsubmit={saveAccount}>
            <div class="account-form-grid">
              <label>
                <span>Label</span>
                <input type="text" autocomplete="off" bind:value={accountLabel} disabled={accountSaving} />
              </label>
              <label>
                <span>Login</span>
                <input
                  type="text"
                  autocomplete="username"
                  spellcheck="false"
                  bind:value={accountLoginName}
                  disabled={accountSaving}
                />
              </label>
              <label>
                <span>Password</span>
                <input
                  type="password"
                  autocomplete="current-password"
                  bind:value={accountPassword}
                  disabled={accountSaving}
                />
              </label>
            </div>
            <div class="account-option-grid">
              <label class="check-row">
                <input type="checkbox" bind:checked={accountRememberPassword} disabled={accountSaving} />
                <span>Remember password</span>
              </label>
              <label class="check-row">
                <input type="checkbox" bind:checked={accountEnabled} disabled={accountSaving} />
                <span>Enabled</span>
              </label>
            </div>
            <div class="actions account-form-actions">
              <span class="form-context">
                {editingAccountId ? "Update source" : "Create source"}
              </span>
              <button type="submit" disabled={accountSaving}>
                {editingAccountId ? "Save" : "Add"}
              </button>
            </div>
          </form>
        </section>
      </div>
    {:else if activeView === "activity"}
      <section class="activity-panel" aria-label="Activity">
        <div class="panel-title">
          <h2>Jobs</h2>
          <div class="panel-actions">
            <button class="secondary small" type="button" onclick={loadJobs} disabled={jobsLoading}>
              Reload
            </button>
            <button class="small" type="button" onclick={clearFinishedJobs} disabled={jobsLoading}>
              Clear
            </button>
          </div>
        </div>

        {#if jobsLoading}
          <div class="empty-state">Loading</div>
        {:else if visibleJobs().length === 0}
          <div class="empty-state">No jobs</div>
        {:else}
          <div class="job-list large">
            {#each visibleJobs() as job (job.id)}
              <article class="job-row" class:failed={job.status === "failed"}>
                <div>
                  <div class="job-title">{jobAccountLabel(job)}</div>
                  <div class="job-detail">{jobDetail(job)}</div>
                </div>
                <span class:active={isActiveJob(job)}>{jobLabel(job)}</span>
                {#if isActiveJob(job)}
                  <button
                    class="secondary small"
                    type="button"
                    onclick={() => cancelJob(job)}
                    disabled={!job.cancellable || job.status === "cancelling"}
                  >
                    Cancel
                  </button>
                {/if}
              </article>
            {/each}
          </div>
        {/if}
      </section>
    {:else}
      <form class="settings-panel" onsubmit={saveSettings}>
        <div class="panel-title">
          <h2>Paths</h2>
          <button
            class="secondary small"
            type="button"
            onclick={loadSettings}
            disabled={settingsLoading || settingsSaving}
          >
            Reload
          </button>
        </div>

        <label>
          <span>Library root</span>
          <input
            type="text"
            autocomplete="off"
            spellcheck="false"
            bind:value={libraryRoot}
            disabled={settingsLoading || settingsSaving}
          />
        </label>

        <label>
          <span>Download root</span>
          <input
            type="text"
            autocomplete="off"
            spellcheck="false"
            bind:value={downloadRoot}
            disabled={settingsLoading || settingsSaving}
          />
        </label>

        <div class="actions">
          <span></span>
          <button type="submit" disabled={settingsLoading || settingsSaving}>
            {settingsSaving ? "Saving" : "Save"}
          </button>
        </div>
      </form>
    {/if}
  </section>

  {#if toasts.length > 0}
    <section class="toast-stack" aria-label="Notifications" aria-live="polite">
      {#each toasts as toast (toast.id)}
        <article class="toast" class:error={toast.kind === "error"} class:success={toast.kind === "success"} role={toast.kind === "error" ? "alert" : "status"}>
          <div class="toast-marker" aria-hidden="true"></div>
          <p>{toast.message}</p>
          <button
            class="toast-close"
            type="button"
            aria-label="Dismiss notification"
            onclick={() => dismissToast(toast.id)}
          >
            <svg class="toast-close-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </article>
      {/each}
    </section>
  {/if}
</main>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(html),
  :global(body) {
    height: 100%;
    overflow: hidden;
  }

  :global(body) {
    --accent: #95c29b;
    --accent-strong: #6fa579;
    --accent-muted: rgb(149 194 155 / 16%);
    --bg: #101214;
    --border: #2c343a;
    --border-strong: #46515a;
    --danger: #f87171;
    --field: #121518;
    --field-disabled: #1a1f23;
    --muted: #9aa5ae;
    --panel: #181c20;
    --panel-raised: #20252a;
    --panel-soft: #15181b;
    --text: #edf2f6;
    --text-strong: #ffffff;
    --text-subtle: #707b85;

    margin: 0;
    color: var(--text);
    background: var(--bg);
    font-family:
      Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    font-size: 15px;
    letter-spacing: 0;
  }

  :global(button),
  :global(input),
  :global(select) {
    font: inherit;
    letter-spacing: 0;
  }

  .app-shell {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    height: 100vh;
    min-height: 100vh;
    overflow: hidden;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 24px 18px;
    border-right: 1px solid var(--border);
    background: #111417;
    color: var(--text);
    overflow: auto;
  }

  .brand {
    font-size: 16px;
    font-weight: 700;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .utility-nav {
    margin-top: auto;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  nav button {
    width: 100%;
    justify-content: flex-start;
    border-color: transparent;
    color: var(--muted);
    background: transparent;
  }

  nav button.active {
    border-color: var(--border);
    background: var(--accent-muted);
    color: var(--text-strong);
  }

  .workspace {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    padding: 28px;
  }

  .workspace-header,
  .actions,
  .panel-title,
  .panel-actions,
  .account-actions {
    display: flex;
    align-items: center;
  }

  .workspace-header {
    flex: 0 0 auto;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 14px;
  }

  .panel-actions,
  .account-actions {
    gap: 8px;
  }

  .eyebrow {
    margin: 0 0 4px;
    color: var(--muted);
    font-size: 13px;
    font-weight: 600;
  }

  h1,
  h2 {
    margin: 0;
    color: var(--text-strong);
    font-weight: 700;
  }

  h1 {
    font-size: 28px;
  }

  h2 {
    font-size: 17px;
  }

  .product-area,
  .accounts-panel,
  .activity-panel,
  .settings-panel {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 16px 40px rgb(0 0 0 / 18%);
  }

  .toast-stack {
    position: fixed;
    right: 18px;
    bottom: 18px;
    z-index: 40;
    display: grid;
    gap: 8px;
    width: min(380px, calc(100vw - 36px));
    pointer-events: none;
  }

  .toast {
    --toast-color: #8ab4e6;
    --toast-bg: rgb(138 180 230 / 12%);

    display: grid;
    grid-template-columns: 4px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    min-height: 48px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel-raised) 92%, black);
    box-shadow: 0 18px 42px rgb(0 0 0 / 38%);
    overflow: hidden;
    pointer-events: auto;
  }

  .toast.success {
    --toast-color: var(--accent);
    --toast-bg: var(--accent-muted);
  }

  .toast.error {
    --toast-color: var(--danger);
    --toast-bg: rgb(248 113 113 / 13%);
  }

  .toast-marker {
    align-self: stretch;
    background: var(--toast-color);
  }

  .toast p {
    min-width: 0;
    margin: 0;
    padding: 10px 0;
    color: var(--text);
    font-size: 13px;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .toast-close {
    width: 30px;
    min-width: 30px;
    height: 30px;
    margin-right: 8px;
    padding: 0;
    border-color: transparent;
    color: var(--muted);
    background: transparent;
  }

  .toast-close:hover {
    border-color: var(--border-strong);
    color: var(--text);
    background: var(--toast-bg);
  }

  .toast-close-icon {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.2;
  }

  .product-area {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .accounts-panel,
  .activity-panel,
  .settings-panel {
    padding: 18px;
  }

  .settings-panel,
  .account-form {
    display: grid;
    gap: 14px;
  }

  .settings-panel {
    max-width: 760px;
  }

  .accounts-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(320px, 420px);
    gap: 18px;
    align-items: start;
    min-height: 0;
    overflow: auto;
  }

  .account-list-panel {
    min-width: 0;
  }

  .account-editor {
    position: sticky;
    top: 28px;
  }

  .account-panel-title {
    align-items: flex-start;
  }

  .account-panel-title > div {
    min-width: 0;
  }

  .account-panel-title p {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12px;
  }

  .account-summary-strip {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 1px;
    margin-bottom: 14px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--border);
    overflow: hidden;
  }

  .account-stat {
    display: grid;
    gap: 2px;
    padding: 10px 12px;
    background: var(--panel-soft);
  }

  .account-stat span {
    color: var(--text-strong);
    font-size: 18px;
    font-weight: 700;
    line-height: 1;
  }

  .account-stat small,
  .form-context {
    color: var(--muted);
    font-size: 12px;
  }

  .account-form-grid {
    display: grid;
    gap: 14px;
  }

  .account-option-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .toolbar {
    display: grid;
    flex: 0 0 auto;
    grid-template-columns: minmax(220px, 1fr) 170px 130px 160px auto auto auto;
    gap: 10px;
    padding: 14px;
    border-bottom: 1px solid var(--border);
    background: var(--panel-soft);
  }

  .list-header {
    display: flex;
    flex: 0 0 auto;
    justify-content: flex-end;
    padding: 9px 14px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 13px;
  }

  .product-table {
    display: block;
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
  }

  .product-card {
    --type-color: #6b7177;
    --type-soft: rgb(107 113 119 / 18%);
    --meta-column-gap: clamp(8px, 1.15vw, 14px);
    --credit-label-width: clamp(60px, 4.1vw, 66px);
    --credit-gap: clamp(5px, 0.7vw, 7px);
    --meta-width: min(100%, clamp(520px, 48vw, 760px));
    --meta-grid-height: 74px;
    --row-height: 220px;
    --thumb-size: 112px;

    display: grid;
    grid-template-columns: 5px var(--thumb-size) minmax(0, 1fr);
    gap: 14px;
    align-items: start;
    height: var(--row-height);
    padding: 12px 14px 12px 0;
    border-bottom: 1px solid var(--border);
    overflow: hidden;
  }

  .product-card:hover {
    background: var(--panel-soft);
  }

  .product-card:last-child {
    border-bottom: 0;
  }

  .product-card[data-tone="audio"] {
    --type-color: #d8a62d;
    --type-soft: rgb(216 166 45 / 17%);
  }

  .product-card[data-tone="video"] {
    --type-color: #d64b92;
    --type-soft: rgb(214 75 146 / 17%);
  }

  .product-card[data-tone="game"] {
    --type-color: #9863df;
    --type-soft: rgb(152 99 223 / 17%);
  }

  .product-card[data-tone="image"] {
    --type-color: #4fb85b;
    --type-soft: rgb(79 184 91 / 16%);
  }

  .type-belt {
    align-self: stretch;
    width: 5px;
    border-radius: 0 6px 6px 0;
    background: var(--type-color);
  }

  .thumb {
    width: var(--thumb-size);
    height: var(--thumb-size);
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    background: var(--panel-raised);
    overflow: hidden;
  }

  .thumb img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .thumb span {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    color: var(--text-subtle);
    font-weight: 700;
  }

  .product-main {
    display: grid;
    grid-template-rows: auto var(--meta-grid-height) 24px 32px;
    gap: 9px;
    height: 100%;
    min-width: 0;
    overflow: hidden;
  }

  .product-title-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
    min-width: 0;
  }

  .product-title {
    min-width: 0;
    color: var(--text-strong);
    font-size: 17px;
    font-weight: 700;
    line-height: 1.25;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .work-id {
    min-width: 102px;
    height: 27px;
    padding: 0 8px;
    border-color: var(--border-strong);
    color: var(--muted);
    background: var(--field);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .work-id:hover {
    border-color: var(--type-color);
    color: var(--text);
  }

  .product-meta {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-template-rows: repeat(4, 15px);
    align-content: start;
    gap: 3px var(--meta-column-gap);
    justify-self: start;
    height: var(--meta-grid-height);
    width: var(--meta-width);
    min-width: 0;
    overflow: hidden;
  }

  .credit-row,
  .labeled-row {
    display: grid;
    grid-template-columns: var(--credit-label-width) minmax(0, 1fr);
    gap: var(--credit-gap);
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.2;
  }

  .credit-row {
    width: 100%;
    height: 15px;
    min-width: 0;
    min-height: 0;
    padding: 0;
    border: 0;
    border-radius: 3px;
    background: transparent;
    cursor: pointer;
    text-align: left;
  }

  .credit-row:hover:not(:disabled) .credit-value {
    color: var(--text);
  }

  .credit-row:focus-visible {
    outline: 2px solid var(--accent-muted);
    outline-offset: 2px;
  }

  .credit-row:disabled {
    cursor: default;
    opacity: 1;
  }

  .labeled-row {
    align-items: center;
    justify-self: start;
    width: var(--meta-width);
    min-height: 24px;
  }

  .credit-label {
    color: var(--text-subtle);
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .credit-value {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .credit-value.missing {
    color: var(--text-subtle);
    opacity: 0.72;
  }

  .chip-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
    max-height: 24px;
    overflow: hidden;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    min-height: 24px;
    max-width: 190px;
    padding: 2px 8px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--muted);
    background: var(--panel-raised);
    font-size: 12px;
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .type-chip {
    border-color: var(--type-color);
    color: var(--type-color);
    background: var(--type-soft);
  }

  .age-chip[data-age="all"] {
    border-color: rgb(112 165 120 / 58%);
    color: #9bc89f;
    background: rgb(112 165 120 / 14%);
  }

  .age-chip[data-age="r15"] {
    border-color: rgb(204 166 61 / 58%);
    color: #d2b56c;
    background: rgb(204 166 61 / 14%);
  }

  .age-chip[data-age="r18"] {
    border-color: rgb(185 64 64 / 62%);
    color: #d77b7b;
    background: rgb(185 64 64 / 16%);
  }

  .product-footer {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    height: 32px;
    min-width: 0;
  }

  .owner-list,
  .account-name small {
    color: var(--muted);
    font-size: 12px;
  }

  .owner-list {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 5px;
    min-width: 0;
    max-height: 24px;
    overflow: hidden;
  }

  .owner-list span {
    max-width: 150px;
    padding: 3px 7px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--text);
    background: var(--panel-raised);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .product-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .menu-button {
    min-width: 42px;
    padding: 0 10px;
  }

  .panel-title {
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 14px;
  }

  .account-list {
    display: grid;
    gap: 8px;
  }

  .account-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-rows: auto auto;
    gap: 10px 16px;
    align-items: center;
    min-height: 96px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
  }

  .account-row.selected {
    border-color: var(--accent);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .account-row.disabled {
    opacity: 0.62;
  }

  .account-name {
    display: grid;
    grid-column: 1;
    grid-row: 1;
    align-items: center;
    justify-items: stretch;
    min-width: 0;
    height: auto;
    min-height: 38px;
    padding: 0;
    border: 0;
    color: inherit;
    background: transparent;
    text-align: left;
  }

  .account-status-text.synced {
    color: var(--accent);
  }

  .account-status-text.syncing {
    color: #d8a62d;
  }

  .account-status-text.failed {
    color: var(--danger);
  }

  .account-status-text.warning {
    color: #d8a62d;
  }

  .account-identity {
    display: grid;
    gap: 5px;
    min-width: 0;
  }

  .account-identity span {
    max-width: 100%;
    color: var(--text);
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .account-enabled-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    justify-self: start;
    width: min(150px, 100%);
    min-height: 24px;
    padding: 2px 10px;
    border: 1px solid rgb(112 165 120 / 58%);
    border-radius: 999px;
    color: var(--accent);
    background: var(--accent-muted);
    font-size: 12px;
    font-weight: 650;
    line-height: 1.1;
  }

  .account-enabled-pill.disabled {
    border-color: var(--border-strong);
    color: var(--text-subtle);
    background: var(--field-disabled);
  }

  .account-meta-grid {
    display: grid;
    grid-column: 1;
    grid-row: 2;
    grid-template-columns: minmax(120px, 0.8fr) minmax(110px, 0.7fr) minmax(190px, 1.2fr);
    gap: 16px;
    min-width: 0;
  }

  .account-meta-grid div {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  .account-meta-grid span {
    color: var(--muted);
    font-size: 11px;
    font-weight: 650;
  }

  .account-meta-grid strong {
    min-width: 0;
    color: var(--text);
    font-size: 12px;
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .account-actions {
    grid-column: 2;
    grid-row: 1 / 3;
    align-self: center;
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  .job-list {
    display: grid;
    gap: 7px;
  }

  .job-list.large {
    gap: 0;
  }

  .job-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: 8px;
    align-items: center;
    min-height: 56px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }

  .job-row:last-child {
    border-bottom: 0;
  }

  .job-row.failed .job-title {
    color: var(--danger);
  }

  .job-title {
    color: var(--text);
    font-size: 13px;
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .job-detail {
    margin-top: 2px;
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .job-row > span {
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }

  .job-row > span.active {
    color: var(--accent);
    font-weight: 650;
  }

  label {
    display: grid;
    gap: 6px;
  }

  label span {
    color: var(--text);
    font-size: 13px;
    font-weight: 650;
  }

  .check-row {
    display: flex;
    align-items: center;
    gap: 9px;
  }

  .account-option-grid .check-row {
    min-height: 40px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--field);
  }

  .check-row input {
    width: 16px;
    height: 16px;
    accent-color: var(--accent-strong);
  }

  input,
  select {
    width: 100%;
    min-width: 0;
    height: 38px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: var(--text);
    background: var(--field);
  }

  input:focus,
  select:focus {
    border-color: var(--accent-strong);
    outline: 2px solid var(--accent-muted);
  }

  input:disabled,
  select:disabled {
    color: var(--text-subtle);
    background: var(--field-disabled);
  }

  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 84px;
    height: 38px;
    padding: 0 13px;
    border: 1px solid var(--accent);
    border-radius: 6px;
    color: #09110c;
    background: var(--accent);
    cursor: pointer;
  }

  button.secondary {
    border-color: var(--border-strong);
    color: var(--text);
    background: var(--panel-raised);
  }

  button.small {
    min-width: 62px;
    height: 32px;
    padding: 0 10px;
    font-size: 13px;
  }

  button:disabled {
    cursor: default;
    opacity: 0.58;
  }

  .empty-state {
    padding: 36px 14px;
    color: var(--muted);
    text-align: center;
  }

  .empty-state.compact {
    padding: 16px 8px;
  }

  .actions {
    justify-content: space-between;
    gap: 14px;
  }

  @media (max-width: 1220px) {
    .account-row {
      grid-template-columns: minmax(0, 1fr);
    }

    .account-meta-grid {
      grid-column: 1 / -1;
    }

    .account-actions {
      grid-column: 1 / -1;
      grid-row: auto;
      justify-content: flex-start;
    }
  }

  @media (max-width: 980px) {
    .accounts-layout {
      grid-template-columns: 1fr;
    }

    .account-editor {
      position: static;
    }

    .toolbar {
      grid-template-columns: 1fr 1fr;
    }

    .product-card {
      --meta-column-gap: 8px;
      --credit-label-width: 62px;
      --credit-gap: 6px;
      --meta-width: 100%;
      --meta-grid-height: 148px;
      --row-height: 270px;
      --thumb-size: 84px;

      gap: 12px;
    }

    .product-meta {
      grid-template-columns: 1fr;
      grid-template-rows: repeat(7, 15px);
    }

    .product-main {
      grid-template-rows: auto var(--meta-grid-height) 24px minmax(24px, auto);
    }

    .product-footer {
      grid-template-columns: 1fr;
      align-items: start;
    }

    .product-actions {
      justify-content: flex-start;
    }
  }

  @media (max-width: 720px) {
    .app-shell {
      grid-template-columns: 1fr;
      grid-template-rows: auto minmax(0, 1fr);
    }

    .sidebar {
      padding: 14px 16px;
      border-right: 0;
      border-bottom: 1px solid var(--border);
    }

    nav {
      flex-direction: row;
      flex-wrap: wrap;
    }

    nav button {
      flex: 1 1 130px;
    }

    .workspace {
      padding: 20px 16px;
    }

    .workspace-header,
    .actions,
    .panel-title,
    .panel-actions,
    .account-row {
      align-items: stretch;
      flex-direction: column;
    }

    .toolbar {
      grid-template-columns: 1fr;
    }

    .account-summary-strip {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .account-row,
    .account-meta-grid,
    .account-option-grid {
      grid-template-columns: 1fr;
    }

    .account-actions {
      justify-content: flex-start;
    }

    .product-card {
      --credit-label-width: 60px;
      --credit-gap: 5px;
      --row-height: 286px;
      --thumb-size: 72px;

      padding-right: 10px;
    }

    .product-title-row {
      grid-template-columns: 1fr;
      gap: 6px;
    }

    .work-id,
    .product-actions,
    .product-actions button,
    .product-actions button.secondary {
      width: auto;
    }

    .work-id {
      justify-self: start;
    }

    .job-row {
      grid-template-columns: 1fr;
    }

    button,
    button.secondary {
      width: 100%;
    }

    .product-actions button,
    .product-actions button.secondary,
    .work-id {
      width: auto;
    }
  }
</style>
