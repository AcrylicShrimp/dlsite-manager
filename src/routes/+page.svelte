<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

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

  type View = "library" | "accounts" | "activity" | "settings";

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
  let status = $state("");
  let error = $state("");

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

  async function loadInitial() {
    await Promise.all([loadSettings(), loadAccounts(), loadProducts(), loadJobs()]);
  }

  async function loadSettings() {
    settingsLoading = true;
    error = "";

    try {
      const settings = await invoke<AppSettings>("get_settings");
      libraryRoot = settings.libraryRoot ?? "";
      downloadRoot = settings.downloadRoot ?? "";
    } catch (err) {
      error = errorMessage(err);
    } finally {
      settingsLoading = false;
    }
  }

  async function saveSettings(event: Event) {
    event.preventDefault();
    settingsSaving = true;
    error = "";
    status = "";

    try {
      const settings = await invoke<AppSettings>("save_settings", {
        settings: {
          libraryRoot: valueOrNull(libraryRoot),
          downloadRoot: valueOrNull(downloadRoot),
        },
      });
      libraryRoot = settings.libraryRoot ?? "";
      downloadRoot = settings.downloadRoot ?? "";
      status = "Settings saved";
    } catch (err) {
      error = errorMessage(err);
    } finally {
      settingsSaving = false;
    }
  }

  async function loadAccounts() {
    accountsLoading = true;
    error = "";

    try {
      accounts = await invoke<Account[]>("list_accounts");
      if (selectedAccountId && !accounts.some((account) => account.id === selectedAccountId)) {
        selectedAccountId = "";
      }
    } catch (err) {
      error = errorMessage(err);
    } finally {
      accountsLoading = false;
    }
  }

  async function saveAccount(event: Event) {
    event.preventDefault();
    accountSaving = true;
    error = "";
    status = "";

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
      status = editingAccountId ? "Account updated" : "Account added";
      editAccount(account);
      accountPassword = "";
      await loadAccounts();
    } catch (err) {
      error = errorMessage(err);
    } finally {
      accountSaving = false;
    }
  }

  async function setAccountEnabled(account: Account, enabled: boolean) {
    error = "";
    status = "";

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
      error = errorMessage(err);
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
    error = "";

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
      error = errorMessage(err);
    } finally {
      productsLoading = false;
    }
  }

  async function loadJobs() {
    jobsLoading = true;
    error = "";

    try {
      jobs = await invoke<JobSnapshot[]>("list_jobs");
    } catch (err) {
      error = errorMessage(err);
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

  async function syncAccount(account: Account) {
    error = "";
    status = "";

    try {
      const response = await invoke<StartJobResponse>("start_account_sync", {
        request: {
          accountId: account.id,
          password: editingAccountId === account.id ? valueOrNull(accountPassword) : null,
        },
      });
      status = "Sync queued";
      jobMessages = {
        ...jobMessages,
        [response.jobId]: "Sync queued",
      };
      accountPassword = "";
      await loadJobs();
    } catch (err) {
      error = errorMessage(err);
    }
  }

  async function syncEnabledAccounts() {
    const enabledAccounts = accounts.filter(
      (account) => account.enabled && !activeAccountSyncJob(account.id),
    );

    for (const account of enabledAccounts) {
      await syncAccount(account);
      if (error) {
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
    error = "";
    status = "";

    try {
      await invoke("cancel_job", {
        request: {
          jobId: job.id,
        },
      });
      status = "Cancellation requested";
      await loadJobs();
    } catch (err) {
      error = errorMessage(err);
    }
  }

  async function clearFinishedJobs() {
    error = "";
    status = "";

    try {
      await invoke("clear_finished_jobs");
      await loadJobs();
    } catch (err) {
      error = errorMessage(err);
    }
  }

  function phaseLabel(account: Account) {
    const activeJob = activeAccountSyncJob(account.id);

    if (activeJob) {
      return jobLabel(activeJob);
    }

    const latestJob = latestAccountSyncJob(account.id);

    if (latestJob?.status === "failed") {
      return "Sync failed";
    }

    if (latestJob?.status === "cancelled") {
      return "Sync cancelled";
    }

    if (account.lastSyncAt) {
      return shortDate(account.lastSyncAt);
    }

    return "Not synced";
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

    {#if error || status}
      <p class:error={Boolean(error)} class="status-line" aria-live="polite">{error || status}</p>
    {/if}

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
              <article class="product-row">
                <div class="thumb" aria-hidden="true">
                  {#if product.thumbnailUrl}
                    <img src={product.thumbnailUrl} alt="" loading="lazy" />
                  {:else}
                    <span>?</span>
                  {/if}
                </div>
                <div class="product-main">
                  <div class="product-title">{product.title}</div>
                  <div class="product-meta">
                    <span>{product.workId}</span>
                    {#if product.makerName}
                      <span>{product.makerName}</span>
                    {/if}
                    {#if product.workType}
                      <span>{product.workType}</span>
                    {/if}
                    {#if ageLabel(product.ageCategory)}
                      <span>{ageLabel(product.ageCategory)}</span>
                    {/if}
                  </div>
                </div>
                <div class="owner-list" aria-label="Owners">
                  {#each product.owners as owner (owner.accountId)}
                    <span>{owner.label}</span>
                  {/each}
                </div>
                <div class="date-cell">{shortDate(product.latestPurchasedAt)}</div>
              </article>
            {/each}
          </div>
        {/if}
      </section>
    {:else if activeView === "accounts"}
      <div class="accounts-layout">
        <section class="accounts-panel account-editor" aria-label="Account editor">
          <div class="panel-title">
            <h2>{editingAccountId ? "Edit account" : "Add account"}</h2>
            <button class="secondary small" type="button" onclick={resetAccountForm} disabled={accountSaving}>
              New
            </button>
          </div>
          <form class="account-form" onsubmit={saveAccount}>
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
            <label class="check-row">
              <input type="checkbox" bind:checked={accountRememberPassword} disabled={accountSaving} />
              <span>Remember password</span>
            </label>
            <label class="check-row">
              <input type="checkbox" bind:checked={accountEnabled} disabled={accountSaving} />
              <span>Enabled</span>
            </label>
            <button type="submit" disabled={accountSaving}>
              {editingAccountId ? "Update" : "Add"}
            </button>
          </form>
        </section>

        <section class="accounts-panel" aria-label="Accounts">
          <div class="panel-title">
            <h2>Accounts</h2>
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
          <div class="account-list">
            {#if accountsLoading}
              <div class="empty-state compact">Loading</div>
            {:else if accounts.length === 0}
              <div class="empty-state compact">No accounts</div>
            {:else}
              {#each accounts as account (account.id)}
                {@const activeSyncJob = activeAccountSyncJob(account.id)}
                <article class="account-row" class:disabled={!account.enabled}>
                  <button class="account-name" type="button" onclick={() => editAccount(account)}>
                    <span>{account.label}</span>
                    <small>{phaseLabel(account)}</small>
                  </button>
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
</main>

<style>
  :global(*) {
    box-sizing: border-box;
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
    min-height: 100vh;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 24px 18px;
    border-right: 1px solid var(--border);
    background: #111417;
    color: var(--text);
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
    min-width: 0;
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

  .status-line {
    min-height: 22px;
    margin: 0 0 14px;
    color: var(--accent);
    overflow-wrap: anywhere;
  }

  .status-line.error {
    color: var(--danger);
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

  .product-area {
    min-width: 0;
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
    grid-template-columns: minmax(280px, 360px) minmax(0, 1fr);
    gap: 18px;
    align-items: start;
  }

  .account-editor {
    position: sticky;
    top: 28px;
  }

  .toolbar {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) 170px 130px 160px auto auto auto;
    gap: 10px;
    padding: 14px;
    border-bottom: 1px solid var(--border);
    background: var(--panel-soft);
  }

  .list-header {
    display: flex;
    justify-content: flex-end;
    padding: 9px 14px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 13px;
  }

  .product-table {
    display: grid;
  }

  .product-row {
    display: grid;
    grid-template-columns: 54px minmax(0, 1fr) minmax(110px, 190px) 150px;
    gap: 12px;
    align-items: center;
    min-height: 78px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }

  .product-row:hover {
    background: var(--panel-soft);
  }

  .product-row:last-child {
    border-bottom: 0;
  }

  .thumb {
    width: 48px;
    height: 48px;
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
    min-width: 0;
  }

  .product-title {
    color: var(--text);
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .product-meta,
  .owner-list,
  .date-cell,
  .account-name small {
    color: var(--muted);
    font-size: 12px;
  }

  .product-meta {
    display: flex;
    gap: 9px;
    margin-top: 4px;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
  }

  .owner-list {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    justify-content: flex-end;
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

  .date-cell {
    text-align: right;
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
    gap: 8px;
    align-items: center;
    padding: 9px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
  }

  .account-row.disabled {
    opacity: 0.62;
  }

  .account-name {
    display: grid;
    justify-items: start;
    min-width: 0;
    height: auto;
    min-height: 38px;
    padding: 0;
    border: 0;
    color: inherit;
    background: transparent;
  }

  .account-name span {
    max-width: 100%;
    color: var(--text);
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

    .product-row {
      grid-template-columns: 54px minmax(0, 1fr);
    }

    .owner-list,
    .date-cell {
      grid-column: 2;
      justify-content: flex-start;
      text-align: left;
    }
  }

  @media (max-width: 720px) {
    .app-shell {
      grid-template-columns: 1fr;
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

    .job-row {
      grid-template-columns: 1fr;
    }

    button,
    button.secondary {
      width: 100%;
    }
  }
</style>
