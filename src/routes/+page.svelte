<script lang="ts">
  import { getIdentifier, getName, getTauriVersion, getVersion } from "@tauri-apps/api/app";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { downloadDir } from "@tauri-apps/api/path";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onDestroy, onMount } from "svelte";
  import {
    AGE_FILTERS,
    DLSITE_URL,
    GITHUB_URL,
    SORT_OPTIONS,
    TYPE_FILTERS,
    creditFieldDefinitions,
    productTypeCodeDetails,
  } from "$lib/model/constants";
  import {
    appInfoValue,
    bulkDownloadExpectedBytesLabel,
    detailDate,
    detailValue,
    downloadStatusLabel,
    errorMessage,
    formatBytes,
    shortDate,
    textVariantsLabel,
    valueOrNull,
  } from "$lib/utils/format";
  import type {
    Account,
    AccountRemovalReport,
    AppInfo,
    AppSettings,
    AuditEvent,
    AuditOutcome,
    BulkDownloadDialog,
    BulkFailedWork,
    BulkSucceededWork,
    BulkWorkDownloadPreview,
    ChipTooltip,
    ConfirmationDialog,
    JobEvent,
    JobSnapshot,
    LocalWorkImportReport,
    Product,
    ProductActionMenu,
    ProductCreditField,
    ProductCreditGroup,
    ProductDetail,
    ProductDownload,
    ProductFilterFacets,
    ProductImagePreview,
    ProductListPage,
    ProductTypeInfo,
    StartJobResponse,
    StartWorkDownloadOptions,
    Toast,
    ToastKind,
    View,
  } from "$lib/model/types";

  let activeView = $state<View>("library");

  let libraryRoot = $state("");
  let downloadRoot = $state("");
  let settingsLoading = $state(true);
  let settingsSaving = $state(false);
  let appInfo = $state<AppInfo | null>(null);
  let appInfoLoading = $state(true);

  let accounts = $state<Account[]>([]);
  let accountsLoading = $state(true);
  let accountSaving = $state(false);
  let editingAccountId = $state<string | null>(null);
  let accountLabel = $state("");
  let accountLoginName = $state("");
  let accountPassword = $state("");

  let products = $state<Product[]>([]);
  let totalProducts = $state(0);
  let productsLoading = $state(true);
  let bulkDownloadPlanning = $state(false);
  let localScanRunning = $state(false);
  let productSearch = $state("");
  let selectedAccountIds = $state<string[]>([]);
  let selectedProductTypes = $state<string[]>([]);
  let selectedAgeCategories = $state<string[]>([]);
  let selectedMakerNames = $state<string[]>([]);
  let productFilterFacets = $state<ProductFilterFacets>({ makers: [] });
  let productSort = $state("latestPurchaseDesc");
  let libraryFiltersOpen = $state(false);

  let jobs = $state<JobSnapshot[]>([]);
  let jobsLoading = $state(true);
  let jobMessages = $state<Record<string, string>>({});
  let auditEvents = $state<AuditEvent[]>([]);
  let auditLoading = $state(true);
  let auditLogDir = $state("");
  let toasts = $state<Toast[]>([]);
  let productImagePreview = $state<ProductImagePreview | null>(null);
  let productActionMenu = $state<ProductActionMenu | null>(null);
  let productDetail = $state<ProductDetail | null>(null);
  let productDetailLoadingWorkId = $state<string | null>(null);
  let chipTooltip = $state<ChipTooltip | null>(null);
  let bulkDownloadDialog = $state<BulkDownloadDialog | null>(null);
  let confirmationDialog = $state<ConfirmationDialog | null>(null);

  let toastSequence = 0;
  let bulkDownloadDialogResolve: ((confirmed: boolean) => void) | null = null;
  let confirmationDialogResolve: ((confirmed: boolean) => void) | null = null;
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

    if (bulkDownloadDialogResolve) {
      bulkDownloadDialogResolve(false);
      bulkDownloadDialogResolve = null;
    }

    if (confirmationDialogResolve) {
      confirmationDialogResolve(false);
      confirmationDialogResolve = null;
    }
  });

  async function loadInitial() {
    await Promise.all([
      loadSettings(),
      loadAppInfo(),
      loadAccounts(),
      loadProducts(),
      loadJobs(),
      loadAuditLogDir(),
      loadAuditEvents(),
    ]);
  }

  async function loadAppInfo() {
    appInfoLoading = true;

    try {
      const [name, version, identifier, tauriVersion] = await Promise.all([
        getName(),
        getVersion(),
        getIdentifier(),
        getTauriVersion(),
      ]);

      appInfo = {
        name,
        version,
        identifier,
        tauriVersion,
      };
    } catch (err) {
      appInfo = null;
      notifyError(errorMessage(err));
    } finally {
      appInfoLoading = false;
    }
  }

  async function loadSettings() {
    settingsLoading = true;

    try {
      const settings = await invoke<AppSettings>("get_settings");
      const defaultDownloadRoot = await systemDownloadRoot();
      libraryRoot = settings.libraryRoot ?? "";
      downloadRoot = settings.downloadRoot ?? defaultDownloadRoot;
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
      const defaultDownloadRoot = await systemDownloadRoot();
      libraryRoot = settings.libraryRoot ?? "";
      downloadRoot = settings.downloadRoot ?? defaultDownloadRoot;
      notifySuccess("Settings saved");
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      settingsSaving = false;
    }
  }

  async function chooseSettingsDirectory(kind: "library" | "download") {
    try {
      const fallbackRoot = await systemDownloadRoot();
      const currentRoot = kind === "library" ? libraryRoot : downloadRoot;
      const selected = await openDialog({
        directory: true,
        multiple: false,
        canCreateDirectories: true,
        defaultPath: currentRoot.trim() || fallbackRoot || undefined,
        title: kind === "library" ? "Choose library folder" : "Choose download staging folder",
      });

      if (!selected) {
        return;
      }

      if (kind === "library") {
        libraryRoot = selected;
      } else {
        downloadRoot = selected;
      }
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function useDefaultDownloadRoot() {
    const root = await systemDownloadRoot();

    if (root) {
      downloadRoot = root;
    }
  }

  async function systemDownloadRoot() {
    try {
      return await downloadDir();
    } catch {
      return "";
    }
  }

  async function loadAccounts() {
    accountsLoading = true;

    try {
      accounts = await invoke<Account[]>("list_accounts");
      selectedAccountIds = selectedAccountIds.filter((accountId) =>
        accounts.some((account) => account.id === accountId),
      );
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

  async function removeAccount(account: Account) {
    const confirmed = await showConfirmationDialog({
      eyebrow: "Account source",
      title: "Remove account?",
      message: `Remove ${account.label}. Its saved credential and ownership source will be deleted. Cached product metadata and downloaded local folders are kept.`,
      confirmLabel: "Remove Account",
      cancelLabel: "Cancel",
      tone: "danger",
    });

    if (!confirmed) {
      return;
    }

    try {
      const report = await invoke<AccountRemovalReport>("remove_account", {
        request: {
          accountId: account.id,
        },
      });

      notifySuccess(`Removed ${report.label}`);

      if (editingAccountId === account.id) {
        resetAccountForm();
      }

      selectedAccountIds = selectedAccountIds.filter((accountId) => accountId !== account.id);

      await Promise.all([loadAccounts(), loadProducts()]);
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  function editAccount(account: Account) {
    editingAccountId = account.id;
    accountLabel = account.label;
    accountLoginName = account.loginName ?? "";
    accountPassword = "";
  }

  function resetAccountForm() {
    editingAccountId = null;
    accountLabel = "";
    accountLoginName = "";
    accountPassword = "";
  }

  async function loadProducts() {
    productsLoading = true;

    try {
      const request = productListRequest();
      const page = await invoke<ProductListPage>("list_products", {
        request,
      });
      const facets = await invoke<ProductFilterFacets>("list_product_filter_facets", {
        request,
      });
      products = page.products;
      totalProducts = page.totalCount;
      productFilterFacets = facets;
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      productsLoading = false;
    }
  }

  function productListRequest() {
    return {
      search: valueOrNull(productSearch),
      accountIds: selectedAccountIds,
      typeGroups: selectedProductTypes,
      ageCategories: selectedAgeCategories,
      makerNames: selectedMakerNames,
      sort: productSort,
      limit: 100,
      offset: 0,
    };
  }

  function productBulkRequest() {
    return {
      search: valueOrNull(productSearch),
      accountIds: selectedAccountIds,
      typeGroups: selectedProductTypes,
      ageCategories: selectedAgeCategories,
      makerNames: selectedMakerNames,
      sort: productSort,
      unpackPolicy: "unpackWhenRecognized",
      skipDownloaded: true,
    };
  }

  function downloadAccountId() {
    return selectedAccountIds.length === 1 ? selectedAccountIds[0] : null;
  }

  function toggleFilterValue(values: string[], value: string) {
    return values.includes(value)
      ? values.filter((candidate) => candidate !== value)
      : [...values, value];
  }

  async function toggleAccountFilter(accountId: string) {
    selectedAccountIds = toggleFilterValue(selectedAccountIds, accountId);
    await loadProducts();
  }

  async function toggleProductTypeFilter(typeGroup: string) {
    selectedProductTypes = toggleFilterValue(selectedProductTypes, typeGroup);
    await loadProducts();
  }

  async function toggleAgeFilter(ageCategory: string) {
    selectedAgeCategories = toggleFilterValue(selectedAgeCategories, ageCategory);
    await loadProducts();
  }

  async function toggleMakerFilter(makerName: string) {
    selectedMakerNames = toggleFilterValue(selectedMakerNames, makerName);
    await loadProducts();
  }

  async function clearAccountFilters() {
    selectedAccountIds = [];
    await loadProducts();
  }

  async function clearTypeFilters() {
    selectedProductTypes = [];
    await loadProducts();
  }

  async function clearAgeFilters() {
    selectedAgeCategories = [];
    await loadProducts();
  }

  async function clearMakerFilters() {
    selectedMakerNames = [];
    await loadProducts();
  }

  async function setProductSort(sort: string) {
    productSort = sort;
    await loadProducts();
  }

  async function resetLibraryFilters() {
    productSearch = "";
    selectedAccountIds = [];
    selectedProductTypes = [];
    selectedAgeCategories = [];
    selectedMakerNames = [];
    productSort = "latestPurchaseDesc";
    await loadProducts();
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
      await Promise.all([loadAccounts(), loadProducts(), loadAuditEvents()]);
    }

    if (
      (event.kind === "workDownload" || event.kind === "bulkWorkDownload") &&
      isTerminalJob(event.snapshot)
    ) {
      applyDownloadJobResult(event.snapshot);
      await loadAuditEvents();
    }
  }

  function applyDownloadJobResult(job: JobSnapshot) {
    if (job.kind === "workDownload") {
      applySingleDownloadJobResult(job);
      return;
    }

    if (job.kind === "bulkWorkDownload") {
      applyBulkDownloadJobResult(job);
    }
  }

  function applySingleDownloadJobResult(job: JobSnapshot) {
    if (jobOutputBoolean(job, "skippedQueued")) {
      return;
    }

    const workId = jobWorkId(job) ?? jobOutputString(job, "workId");

    if (!workId) {
      return;
    }

    if (job.status === "succeeded") {
      patchProductDownload(workId, {
        status: "downloaded",
        localPath: jobOutputString(job, "localPath"),
        errorCode: null,
        errorMessage: null,
        completedAt: job.finishedAt,
        updatedAt: job.finishedAt ?? new Date().toISOString(),
      });
      return;
    }

    patchProductDownload(workId, {
      status: job.status === "cancelled" ? "cancelled" : "failed",
      errorCode: job.error?.code ?? null,
      errorMessage: job.error?.message ?? null,
      updatedAt: job.finishedAt ?? new Date().toISOString(),
    });
  }

  function applyBulkDownloadJobResult(job: JobSnapshot) {
    const result = bulkDownloadResult(job);

    for (const success of result.succeededWorks) {
      patchProductDownload(success.workId, {
        status: "downloaded",
        localPath: success.localPath,
        errorCode: null,
        errorMessage: null,
        completedAt: job.finishedAt,
        updatedAt: job.finishedAt ?? new Date().toISOString(),
      });
    }

    for (const failure of result.failedWorks) {
      patchProductDownload(failure.workId, {
        status: job.status === "cancelled" ? "cancelled" : "failed",
        errorCode: failure.errorCode ?? job.error?.code ?? null,
        errorMessage: failure.errorMessage ?? job.error?.message ?? null,
        updatedAt: job.finishedAt ?? new Date().toISOString(),
      });
    }
  }

  function patchProductDownload(workId: string, patch: Partial<ProductDownload>) {
    products = products.map((product) => {
      if (product.workId !== workId) {
        return product;
      }

      return {
        ...product,
        download: {
          ...product.download,
          ...patch,
        },
      };
    });

    if (productDetail?.workId === workId) {
      productDetail = {
        ...productDetail,
        download: {
          ...productDetail.download,
          ...patch,
        },
      };
    }
  }

  function setProductDownload(workId: string, download: ProductDownload) {
    products = products.map((product) =>
      product.workId === workId
        ? {
            ...product,
            download,
        }
        : product,
    );

    if (productDetail?.workId === workId) {
      productDetail = {
        ...productDetail,
        download,
      };
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

  async function copyText(label: string, value: string | null | undefined) {
    const normalized = value?.trim();

    if (!normalized) {
      return;
    }

    try {
      await navigator.clipboard.writeText(normalized);
      notifySuccess(`Copied ${label}`);
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function openProductDetail(product: Product) {
    closeProductActionMenu();
    productDetailLoadingWorkId = product.workId;

    try {
      productDetail = await invoke<ProductDetail>("get_product_detail", {
        request: {
          workId: product.workId,
        },
      });
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      productDetailLoadingWorkId = null;
    }
  }

  function closeProductDetail() {
    productDetail = null;
  }

  function openProductImage(product: Product) {
    if (!product.thumbnailUrl) {
      return;
    }

    productImagePreview = {
      url: product.thumbnailUrl,
      title: product.title,
      workId: product.workId,
    };
  }

  function openProductImageFromDetail(detail: ProductDetail) {
    openProductImage({
      workId: detail.workId,
      title: detail.title,
      makerName: detail.makerName,
      workType: detail.workType,
      ageCategory: detail.ageCategory,
      thumbnailUrl: detail.thumbnailUrl,
      publishedAt: detail.publishedAt,
      updatedAt: detail.updatedAt,
      earliestPurchasedAt: detail.earliestPurchasedAt,
      latestPurchasedAt: detail.latestPurchasedAt,
      creditGroups: detail.creditGroups,
      download: detail.download,
      owners: detail.owners,
    });
  }

  function closeProductImage() {
    productImagePreview = null;
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key !== "Escape") {
      return;
    }

    if (confirmationDialog) {
      closeConfirmationDialog(false);
      return;
    }

    if (bulkDownloadDialog) {
      closeBulkDownloadDialog(false);
      return;
    }

    if (productActionMenu) {
      closeProductActionMenu();
    }

    if (productImagePreview) {
      closeProductImage();
      return;
    }

    if (productDetail) {
      closeProductDetail();
    }
  }

  function handleWindowClick() {
    closeProductActionMenu();
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

  async function startWorkDownload(product: Product, options: StartWorkDownloadOptions = {}) {
    if (activeWorkDownloadJob(product.workId)) {
      return;
    }

    try {
      const response = await invoke<StartJobResponse>("start_work_download", {
        request: {
          workId: product.workId,
          accountId: downloadAccountId(),
          password: null,
          unpackPolicy: options.unpackPolicy ?? "unpackWhenRecognized",
          replaceExisting: options.replaceExisting ?? false,
        },
      });
      const queuedMessage = options.queuedMessage ?? "Download queued";
      notifyInfo(queuedMessage);
      jobMessages = {
        ...jobMessages,
        [response.jobId]: queuedMessage,
      };
      await loadJobs();
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function startBulkWorkDownload() {
    bulkDownloadPlanning = true;

    try {
      const preview = await invoke<BulkWorkDownloadPreview>("preview_bulk_work_download", {
        request: productBulkRequest(),
      });

      if (preview.requestedCount === 0) {
        await showBulkDownloadDialog(preview, "notice");
        return;
      }

      const confirmed = await showBulkDownloadDialog(preview, "confirm");

      if (!confirmed) {
        return;
      }

      const response = await invoke<StartJobResponse>("start_bulk_work_download", {
        request: productBulkRequest(),
      });
      notifyInfo("Bulk download queued");
      jobMessages = {
        ...jobMessages,
        [response.jobId]: "Bulk download queued",
      };
      await loadJobs();
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      bulkDownloadPlanning = false;
    }
  }

  async function scanLocalWorkDownloads() {
    if (localScanRunning) {
      return;
    }

    localScanRunning = true;

    try {
      const report = await invoke<LocalWorkImportReport>("scan_local_work_downloads");
      const skipped =
        report.skippedNoId +
        report.skippedAmbiguous +
        report.skippedNonUtf8 +
        report.skippedExisting;
      const suffix = skipped > 0 ? `, skipped ${skipped}` : "";
      notifySuccess(`Imported ${report.importedCount} local folders${suffix}`);
      await loadProducts();
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      localScanRunning = false;
    }
  }

  async function openDownloadedProduct(product: Product) {
    if (!product.download.localPath) {
      return;
    }

    try {
      await invoke("open_work_download", {
        request: {
          workId: product.workId,
        },
      });
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function downloadProductArchivesOnly(product: Product) {
    closeProductActionMenu();
    await startWorkDownload(product, {
      unpackPolicy: "keepArchives",
      queuedMessage: "Archive-only download queued",
    });
  }

  async function redownloadProduct(product: Product) {
    closeProductActionMenu();

    const confirmed = await showConfirmationDialog({
      eyebrow: "Re-download",
      title: `Re-download ${product.workId}?`,
      message:
        "This will replace the local folder after the new download completes. Any changes inside that folder will be removed.",
      confirmLabel: "Re-download",
      cancelLabel: "Cancel",
      tone: "danger",
    });

    if (!confirmed) {
      return;
    }

    await startWorkDownload(product, {
      unpackPolicy:
        product.download.unpackPolicy === "keep_archives"
          ? "keepArchives"
          : "unpackWhenRecognized",
      replaceExisting: true,
      queuedMessage: "Re-download queued",
    });
  }

  async function deleteDownloadedProduct(product: Product) {
    closeProductActionMenu();

    const confirmed = await showConfirmationDialog({
      eyebrow: "Delete Download",
      title: `Delete downloaded files for ${product.workId}?`,
      message:
        "This removes the local downloaded folder and any staging files. Cached ownership stays intact, so you can download it again later.",
      confirmLabel: "Delete Download",
      cancelLabel: "Cancel",
      tone: "danger",
    });

    if (!confirmed) {
      return;
    }

    try {
      const download = await invoke<ProductDownload>("delete_work_download", {
        request: {
          workId: product.workId,
        },
      });
      notifySuccess("Download deleted");
      setProductDownload(product.workId, download);
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function markProductDownloaded(product: Product) {
    closeProductActionMenu();

    try {
      const fallbackRoot = libraryRoot.trim() || (await systemDownloadRoot());
      const selected = await openDialog({
        directory: true,
        multiple: false,
        canCreateDirectories: false,
        defaultPath: fallbackRoot || undefined,
        title: `Choose local folder for ${product.workId}`,
      });

      if (!selected) {
        return;
      }

      const download = await invoke<ProductDownload>("mark_work_downloaded", {
        request: {
          workId: product.workId,
          localPath: selected,
        },
      });
      notifySuccess("Marked as downloaded");
      setProductDownload(product.workId, download);
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  function toggleProductActionMenu(product: Product, event: MouseEvent) {
    event.stopPropagation();

    if (productActionMenu?.workId === product.workId) {
      closeProductActionMenu();
      return;
    }

    const target = event.currentTarget;
    if (!(target instanceof HTMLElement)) {
      return;
    }

    const rect = target.getBoundingClientRect();
    const menuWidth = 220;
    const menuHeight = 150;
    productActionMenu = {
      workId: product.workId,
      left: Math.max(12, Math.min(rect.right - menuWidth, window.innerWidth - menuWidth - 12)),
      top: Math.max(12, Math.min(rect.bottom + 6, window.innerHeight - menuHeight - 12)),
    };
  }

  function closeProductActionMenu() {
    productActionMenu = null;
  }

  function productActionMenuProduct() {
    return productActionMenu
      ? products.find((product) => product.workId === productActionMenu?.workId) ?? null
      : null;
  }

  function productHasDownloadRecord(product: Product) {
    return product.download.status !== "notDownloaded";
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

  async function loadAuditEvents() {
    auditLoading = true;

    try {
      auditEvents = await invoke<AuditEvent[]>("list_audit_events", {
        request: {
          limit: 80,
        },
      });
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      auditLoading = false;
    }
  }

  async function loadAuditLogDir() {
    try {
      const result = await invoke<{ path: string }>("get_audit_log_dir");
      auditLogDir = result.path;
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function openAuditLogDir() {
    try {
      await invoke("open_audit_log_dir");
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function openExternalUrl(url: string, label: string) {
    try {
      await openUrl(url);
    } catch (err) {
      notifyError(`Failed to open ${label}: ${errorMessage(err)}`);
    }
  }

  function dlsiteProductPageUrl(workId: string) {
    return `https://www.dlsite.com/home/work/=/product_id/${encodeURIComponent(workId)}.html`;
  }

  async function openDlsiteProductPage(workId: string) {
    await openExternalUrl(dlsiteProductPageUrl(workId), `${workId} on DLsite`);
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

  function workDownloadJobs(workId: string) {
    return jobs.filter((job) => job.kind === "workDownload" && jobWorkId(job) === workId);
  }

  function activeWorkDownloadJob(workId: string) {
    return [...workDownloadJobs(workId)].reverse().find(isActiveJob) ?? null;
  }

  function activeBulkDownloadPlanningJob() {
    return (
      [...jobs]
        .reverse()
        .find((job) => job.kind === "bulkWorkDownloadPreview" && isActiveJob(job)) ?? null
    );
  }

  function bulkDownloadButtonLabel() {
    if (!bulkDownloadPlanning) {
      return "Bulk Download";
    }

    const planningJob = activeBulkDownloadPlanningJob();

    return planningJob ? jobLabel(planningJob) : "Planning";
  }

  function visibleJobs(limit = 20) {
    return [...jobs].reverse().slice(0, limit);
  }

  function visibleDownloadJobs(limit = 50) {
    return [...currentDownloadJobs()].reverse().slice(0, limit);
  }

  function isDownloadQueueJob(job: JobSnapshot) {
    return job.kind === "workDownload" || job.kind === "bulkWorkDownload";
  }

  function currentDownloadJobs() {
    return jobs.filter((job) => isDownloadQueueJob(job) && isActiveJob(job));
  }

  function queuedDownloadJobCount() {
    return currentDownloadJobs().filter((job) => job.status === "queued").length;
  }

  function runningDownloadJobCount() {
    return currentDownloadJobs().filter((job) => job.status === "running").length;
  }

  function visibleAuditEvents(limit = 30) {
    return auditEvents.slice(0, limit);
  }

  function auditOutcomeLabel(outcome: AuditOutcome) {
    switch (outcome) {
      case "queued":
        return "Queued";
      case "succeeded":
        return "Succeeded";
      case "failed":
        return "Failed";
      case "cancelled":
        return "Cancelled";
    }
  }

  function auditDetail(event: AuditEvent) {
    return event.errorMessage ?? event.message;
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

  function jobWorkId(job: JobSnapshot) {
    const workId = job.metadata.workId;
    return typeof workId === "string" ? workId : null;
  }

  function jobAccountLabel(job: JobSnapshot) {
    if (job.kind === "bulkWorkDownload") {
      return "Bulk download";
    }

    if (job.kind === "bulkWorkDownloadPreview") {
      return "Bulk planning";
    }

    if (job.kind === "workDownload") {
      return jobWorkId(job) ?? job.title;
    }

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
      if (job.kind === "workDownload") {
        return "Downloaded";
      }

      if (job.kind === "bulkWorkDownload") {
        const succeededCount = jobOutputNumber(job, "succeededCount");
        return typeof succeededCount === "number"
          ? `Downloaded ${succeededCount} works`
          : "Downloaded";
      }

      if (job.kind === "bulkWorkDownloadPreview") {
        const plannedCount = jobOutputNumber(job, "plannedCount");
        const failedCount = jobOutputNumber(job, "failedCount");

        if (
          typeof plannedCount === "number" &&
          typeof failedCount === "number" &&
          failedCount > 0
        ) {
          return `Planned ${plannedCount}, ${failedCount} failed`;
        }

        return typeof plannedCount === "number" ? `Planned ${plannedCount} works` : "Planned";
      }

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
      case "loadingProducts":
        return "Preparing downloads";
      case "bulkDownloading":
        return bulkDownloadJobProgressLabel(job);
      case "bulkPlanning":
        return bulkDownloadPlanningJobProgressLabel(job);
      case "committing":
        return "Saving cache";
      case "completed":
        return "Completing";
      case "resolvingDownload":
        return "Resolving download";
      case "probingDownload":
        return job.kind === "workDownload" ? downloadJobProgressLabel(job) : "Resolving download";
      case "downloading":
        return downloadJobProgressLabel(job);
      case "unpacking":
        return "Decompressing";
      case "finalizing":
        return "Finalizing";
      default:
        if (job.kind === "bulkWorkDownload") {
          return "Downloading results";
        }

        if (job.kind === "bulkWorkDownloadPreview") {
          return "Planning";
        }

        return job.kind === "workDownload" ? "Downloading" : "Syncing";
    }
  }

  function jobDetail(job: JobSnapshot) {
    if (job.error?.message) {
      return job.error.message;
    }

    if (isActiveJob(job)) {
      const activeDetail = activeJobDetail(job);

      if (activeDetail) {
        return activeDetail;
      }
    }

    return jobMessages[job.id] ?? shortDate(job.finishedAt ?? job.startedAt ?? job.createdAt);
  }

  function activeJobDetail(job: JobSnapshot) {
    if (job.kind === "workDownload") {
      return activeWorkDownloadDetail(job);
    }

    if (job.kind === "bulkWorkDownload") {
      return activeBulkDownloadDetail(job);
    }

    return null;
  }

  function activeWorkDownloadDetail(job: JobSnapshot) {
    if (job.status === "queued") {
      return "Waiting to start";
    }

    switch (job.phase) {
      case "loggingIn":
        return "Signing in";
      case "resolvingDownload":
        return "Resolving download files";
      case "probingDownload":
      case "downloading":
        return downloadProgressDetail(job);
      case "unpacking":
        return "Decompressing archive";
      case "finalizing":
        return "Moving files into the library";
      case "completed":
        return "Completing";
      default:
        return "Preparing download";
    }
  }

  function activeBulkDownloadDetail(job: JobSnapshot) {
    if (job.status === "queued") {
      return "Waiting to start";
    }

    if (job.phase === "bulkDownloading" && job.progress?.unit === "items") {
      const current = job.progress.current ?? 0;
      const total = job.progress.total;

      return typeof total === "number" && total > 0
        ? `${current} of ${total} products processed`
        : "Processing products";
    }

    return "Preparing bulk download";
  }

  function downloadQueueTitle(job: JobSnapshot) {
    if (job.kind === "bulkWorkDownload") {
      const requested = jobOutputNumber(job, "requestedCount") ?? metadataNumber(job, "reservedCount");
      return typeof requested === "number" && requested > 0
        ? `Bulk download (${requested} works)`
        : "Bulk download";
    }

    const workId = jobWorkId(job) ?? jobOutputString(job, "workId");
    const product = workId ? products.find((item) => item.workId === workId) : null;
    return product?.title ?? workId ?? job.title;
  }

  function downloadQueueSubtitle(job: JobSnapshot) {
    if (job.kind === "workDownload") {
      const workId = jobWorkId(job) ?? jobOutputString(job, "workId");
      return workId ? `Single work ${workId}` : "Single work download";
    }

    const skippedDownloaded =
      jobOutputNumber(job, "skippedDownloadedCount") ?? metadataNumber(job, "skippedDownloadedCount");
    const skippedQueued =
      jobOutputNumber(job, "skippedQueuedCount") ?? metadataNumber(job, "skippedQueuedCount");
    const parts = [
      typeof skippedDownloaded === "number" ? `${skippedDownloaded} already downloaded` : null,
      typeof skippedQueued === "number" ? `${skippedQueued} already queued` : null,
    ].filter((part): part is string => part !== null);

    return parts.length > 0 ? parts.join(", ") : "Current Library filters";
  }

  function downloadQueueKindLabel(job: JobSnapshot) {
    if (job.kind === "bulkWorkDownload") {
      return "Bulk";
    }

    return "Work";
  }

  function downloadQueueTime(job: JobSnapshot) {
    return shortDate(job.finishedAt ?? job.startedAt ?? job.createdAt);
  }

  function downloadQueueProgressPercent(job: JobSnapshot) {
    const current = job.progress?.current;
    const total = job.progress?.total;

    if (typeof current !== "number" || typeof total !== "number" || total <= 0) {
      return null;
    }

    return Math.min(100, Math.max(0, Math.floor((current * 100) / total)));
  }

  function metadataNumber(job: JobSnapshot, key: string) {
    const value = job.metadata[key];
    return typeof value === "number" ? value : null;
  }

  function jobOutputString(job: JobSnapshot, key: string) {
    const value = job.output?.[key];
    return typeof value === "string" ? value : null;
  }

  function jobOutputBoolean(job: JobSnapshot, key: string) {
    const value = job.output?.[key];
    return typeof value === "boolean" ? value : false;
  }

  function jobOutputNumber(job: JobSnapshot, key: string) {
    const value = job.output?.[key];
    return typeof value === "number" ? value : null;
  }

  function bulkDownloadResult(job: JobSnapshot) {
    const source = job.output ?? recordValue(job.error?.details.bulkDownload);

    return {
      succeededWorks: parseBulkSucceededWorks(source?.succeededWorks),
      failedWorks: parseBulkFailedWorks(source?.failedWorks),
    };
  }

  function parseBulkSucceededWorks(value: unknown): BulkSucceededWork[] {
    if (!Array.isArray(value)) {
      return [];
    }

    return value
      .map((item) => {
        const record = recordValue(item);

        if (!record || typeof record.workId !== "string") {
          return null;
        }

        return {
          workId: record.workId,
          localPath: typeof record.localPath === "string" ? record.localPath : null,
          fileCount: typeof record.fileCount === "number" ? record.fileCount : null,
          archiveExtracted:
            typeof record.archiveExtracted === "boolean" ? record.archiveExtracted : null,
        };
      })
      .filter((item): item is BulkSucceededWork => item !== null);
  }

  function parseBulkFailedWorks(value: unknown): BulkFailedWork[] {
    if (!Array.isArray(value)) {
      return [];
    }

    return value
      .map((item) => {
        const record = recordValue(item);

        if (!record || typeof record.workId !== "string") {
          return null;
        }

        return {
          workId: record.workId,
          errorCode: typeof record.errorCode === "string" ? record.errorCode : null,
          errorMessage: typeof record.errorMessage === "string" ? record.errorMessage : null,
        };
      })
      .filter((item): item is BulkFailedWork => item !== null);
  }

  function recordValue(value: unknown): Record<string, unknown> | null {
    return value && typeof value === "object" && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : null;
  }

  function downloadJobProgressLabel(job: JobSnapshot) {
    if (job.progress?.unit !== "bytes") {
      return "Downloading";
    }

    const current = job.progress.current ?? 0;
    const total = job.progress.total;

    if (typeof total === "number" && total > 0) {
      return `Downloading ${Math.min(100, Math.floor((current * 100) / total))}%`;
    }

    return "Downloading";
  }

  function downloadProgressDetail(job: JobSnapshot) {
    if (job.progress?.unit !== "bytes") {
      return "Downloading files";
    }

    const current = job.progress.current ?? 0;
    const total = job.progress.total;

    if (typeof total === "number" && total > 0) {
      return `${formatBytes(current)} of ${formatBytes(total)}`;
    }

    return `${formatBytes(current)} downloaded`;
  }

  function bulkDownloadJobProgressLabel(job: JobSnapshot) {
    if (job.progress?.unit !== "items") {
      return "Downloading results";
    }

    const current = job.progress.current ?? 0;
    const total = job.progress.total;

    if (typeof total === "number" && total > 0) {
      return `Downloading ${current}/${total}`;
    }

    return "Downloading results";
  }

  function bulkDownloadPlanningJobProgressLabel(job: JobSnapshot) {
    if (job.progress?.unit !== "items") {
      return "Planning";
    }

    const current = job.progress.current ?? 0;
    const total = job.progress.total;

    if (typeof total === "number" && total > 0) {
      return `Planning ${current}/${total}`;
    }

    return "Planning";
  }

  function showBulkDownloadDialog(
    preview: BulkWorkDownloadPreview,
    kind: BulkDownloadDialog["kind"],
  ) {
    if (bulkDownloadDialogResolve) {
      bulkDownloadDialogResolve(false);
    }

    return new Promise<boolean>((resolve) => {
      bulkDownloadDialogResolve = resolve;
      bulkDownloadDialog = { kind, preview };
    });
  }

  function closeBulkDownloadDialog(confirmed = false) {
    const resolve = bulkDownloadDialogResolve;

    bulkDownloadDialogResolve = null;
    bulkDownloadDialog = null;
    resolve?.(confirmed);
  }

  function showConfirmationDialog(dialog: ConfirmationDialog) {
    if (confirmationDialogResolve) {
      confirmationDialogResolve(false);
    }

    return new Promise<boolean>((resolve) => {
      confirmationDialogResolve = resolve;
      confirmationDialog = dialog;
    });
  }

  function closeConfirmationDialog(confirmed = false) {
    const resolve = confirmationDialogResolve;

    confirmationDialogResolve = null;
    confirmationDialog = null;
    resolve?.(confirmed);
  }

  function productDownloadActionLabel(product: Product, job: JobSnapshot | null) {
    if (job) {
      if (job.status === "queued") {
        return "Queued";
      }

      if (job.status === "cancelling") {
        return "Cancelling";
      }

      return jobLabel(job);
    }

    switch (product.download.status) {
      case "downloaded":
        return "Open";
      case "failed":
      case "cancelled":
      case "downloading":
        return "Retry";
      default:
        return "Download";
    }
  }

  function productDownloadActionTitle(product: Product, job: JobSnapshot | null) {
    if (job) {
      return jobLabel(job);
    }

    if (product.download.status === "downloaded" && product.download.localPath) {
      return `Open ${product.download.localPath}`;
    }

    if (product.download.errorMessage) {
      return product.download.errorMessage;
    }

    return "Download this work";
  }

  function productDownloadActionDisabled(product: Product, job: JobSnapshot | null) {
    return !!job || (product.download.status === "downloaded" && !product.download.localPath);
  }

  async function runProductDownloadAction(product: Product) {
    if (product.download.status === "downloaded") {
      await openDownloadedProduct(product);
      return;
    }

    await startWorkDownload(product);
  }

  function productType(product: Product): ProductTypeInfo {
    return productTypeFromCode(product.workType);
  }

  function productTypeFromCode(workType: string | null): ProductTypeInfo {
    const raw = workType?.trim() || "";
    const upper = raw.toUpperCase();
    const knownType = productTypeCodeDetails[upper];

    if (knownType) {
      return {
        label: knownType.label,
        tone: knownType.tone,
        tooltip: `${knownType.label}: ${knownType.description}. DLsite code ${upper}.`,
      };
    }

    const normalized = raw.toLowerCase().replace(/[\s_-]+/g, "");

    if (matchesAny(normalized, ["voicecomic", "vcomic"])) {
      return productTypeFallback(
        raw,
        "Voice Comic",
        "voice-comic",
        "Comic with voice/audio presentation",
      );
    }

    if (matchesAny(normalized, ["sou", "audio", "voice", "asmr", "music", "sound"])) {
      return productTypeFallback(raw, "Audio", "audio", "Audio-like product type");
    }

    if (matchesAny(normalized, ["mov", "movie", "video", "anime"])) {
      return productTypeFallback(raw, "Video", "video", "Video-like product type");
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
      return productTypeFallback(raw, "Game", "game", "Game-like product type");
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
      return productTypeFallback(
        raw,
        "Image / Comic",
        "image",
        "Image, comic, manga, or reading-material product type",
      );
    }

    if (matchesAny(normalized, ["software", "tool", "utility", "etc", "other"])) {
      return productTypeFallback(raw, "Other", "other", "Tool or other product type");
    }

    return {
      label: raw || "Other",
      tone: "other",
      tooltip: raw
        ? `Unrecognized product type from DLsite: ${raw}.`
        : "Product type is not available from DLsite.",
    };
  }

  function productTypeFallback(
    raw: string,
    fallbackLabel: string,
    tone: string,
    description: string,
  ): ProductTypeInfo {
    const label = raw || fallbackLabel;

    return {
      label,
      tone,
      tooltip: raw ? `${label}: ${description}.` : `${fallbackLabel}: ${description}.`,
    };
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
        return "All Ages";
      case "r15":
        return "R-15";
      case "r18":
        return "R-18";
      default:
        return "";
    }
  }

  function ageTooltip(value: string | null) {
    switch (value) {
      case "all":
        return "DLsite rating: all ages.";
      case "r15":
        return "DLsite rating: R-15.";
      case "r18":
        return "DLsite rating: R-18.";
      default:
        return "DLsite rating is unknown.";
    }
  }

  function showChipTooltip(text: string, event: MouseEvent) {
    moveChipTooltip(text, event);
  }

  function moveChipTooltip(text: string, event: MouseEvent) {
    const maxWidth = 320;
    const left = Math.max(12, Math.min(event.clientX + 12, window.innerWidth - maxWidth - 12));
    const top = Math.max(12, Math.min(event.clientY + 14, window.innerHeight - 54));
    chipTooltip = { text, left, top };
  }

  function hideChipTooltip() {
    chipTooltip = null;
  }

  function creditText(group: ProductCreditGroup) {
    return group.names.join(", ");
  }

  function productCreditFields(product: { makerName: string | null; creditGroups: ProductCreditGroup[] }): ProductCreditField[] {
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

  function creditTextForKind(product: { creditGroups: ProductCreditGroup[] }, kind: string) {
    const group = product.creditGroups?.find((item) => item.kind === kind);
    return group ? creditText(group).trim() : "";
  }

  function creditTooltip(field: ProductCreditField) {
    return field.missing ? `${field.label}: Not available` : `${field.label}: ${field.value}`;
  }

  function detailTags(detail: ProductDetail) {
    return detail.tags.filter((tag) => !tag.class.endsWith("_by"));
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
      case "downloads":
        return "Queue";
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
      case "downloads":
        return "Downloads";
      case "accounts":
        return "Accounts";
      case "activity":
        return "Activity";
      case "settings":
        return "Settings";
    }
  }

</script>

<svelte:head>
  <title>dlsite-manager</title>
</svelte:head>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

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
        class:active={activeView === "downloads"}
        type="button"
        onclick={() => (activeView = "downloads")}
      >
        Downloads
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

  <section class="workspace" class:library-workspace={activeView === "library"}>
    <header class="workspace-header">
      <div>
        <p class="eyebrow">{viewEyebrow(activeView)}</p>
        <h1>{viewTitle(activeView)}</h1>
      </div>
    </header>

    {#if activeView === "library"}
      <section class="product-area" aria-label="Library">
        <div class="library-controls">
          <form class="library-search-panel" onsubmit={searchProducts}>
            <div class="library-search-row">
              <input
                type="search"
                autocomplete="off"
                spellcheck="false"
                placeholder="Search title, maker, credit, tag, work ID"
                bind:value={productSearch}
              />
              <button type="submit" disabled={productsLoading}>Search</button>
              <button class="secondary" type="button" onclick={resetLibraryFilters}>
                Reset
              </button>
              <button
                class="secondary filter-fold-button"
                type="button"
                aria-expanded={libraryFiltersOpen}
                aria-controls="library-filter-grid"
                onclick={() => (libraryFiltersOpen = !libraryFiltersOpen)}
              >
                {libraryFiltersOpen ? "Hide Filters" : "Show Filters"}
              </button>
            </div>
          </form>

          <div class="library-actions-panel" aria-label="Library actions">
            <div class="library-action-group">
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
            </div>
            <div class="library-action-group">
              <button
                class="secondary"
                type="button"
                onclick={scanLocalWorkDownloads}
                disabled={localScanRunning || productsLoading}
              >
                {localScanRunning ? "Scanning" : "Scan Local"}
              </button>
              <button
                class="secondary download-results-button"
                type="button"
                onclick={startBulkWorkDownload}
                disabled={bulkDownloadPlanning || productsLoading || jobsLoading || totalProducts === 0}
              >
                {bulkDownloadButtonLabel()}
              </button>
            </div>
          </div>
        </div>

        {#if libraryFiltersOpen}
          <div id="library-filter-grid" class="library-filter-panel filter-grid">
            <div class="filter-group sort-filter">
              <span>Sort</span>
              <div class="toggle-row">
                {#each SORT_OPTIONS as [value, label] (value)}
                  <button
                    class:active={productSort === value}
                    type="button"
                    onclick={() => setProductSort(value)}
                  >
                    <span class="filter-chip-label">{label}</span>
                  </button>
                {/each}
              </div>
            </div>

            <div class="filter-group">
              <span>Accounts</span>
              <div class="toggle-row">
                <button
                  class:active={selectedAccountIds.length === 0}
                  type="button"
                  onclick={clearAccountFilters}
                >
                  <span class="filter-chip-label">All</span>
                </button>
                {#each accounts as account (account.id)}
                  <button
                    class:active={selectedAccountIds.includes(account.id)}
                    type="button"
                    title={account.loginName ?? account.label}
                    onclick={() => toggleAccountFilter(account.id)}
                  >
                    <span class="filter-chip-label">{account.label}</span>
                  </button>
                {/each}
              </div>
            </div>

            <div class="filter-group">
              <span>Age</span>
              <div class="toggle-row">
                <button
                  class:active={selectedAgeCategories.length === 0}
                  type="button"
                  onclick={clearAgeFilters}
                >
                  <span class="filter-chip-label">Any</span>
                </button>
                {#each AGE_FILTERS as [value, label] (value)}
                  <button
                    class:active={selectedAgeCategories.includes(value)}
                    type="button"
                    onclick={() => toggleAgeFilter(value)}
                  >
                    <span class="filter-chip-label">{label}</span>
                  </button>
                {/each}
              </div>
            </div>

            <div class="filter-group">
              <span>Type</span>
              <div class="toggle-row">
                <button
                  class:active={selectedProductTypes.length === 0}
                  type="button"
                  onclick={clearTypeFilters}
                >
                  <span class="filter-chip-label">Any</span>
                </button>
                {#each TYPE_FILTERS as [value, label] (value)}
                  <button
                    class:active={selectedProductTypes.includes(value)}
                    type="button"
                    onclick={() => toggleProductTypeFilter(value)}
                  >
                    <span class="filter-chip-label">{label}</span>
                  </button>
                {/each}
              </div>
            </div>

            <div class="filter-group maker-filter">
              <span>Makers</span>
              <div class="toggle-row">
                <button
                  class:active={selectedMakerNames.length === 0}
                  type="button"
                  onclick={clearMakerFilters}
                >
                  <span class="filter-chip-label">Any</span>
                </button>
                {#each productFilterFacets.makers as maker (maker.name)}
                  <button
                    class:active={selectedMakerNames.includes(maker.name)}
                    type="button"
                    title={`${maker.name} (${maker.count})`}
                    onclick={() => toggleMakerFilter(maker.name)}
                  >
                    <span class="filter-chip-label">{maker.name}</span>
                    <small>{maker.count}</small>
                  </button>
                {/each}
              </div>
            </div>
          </div>
        {/if}

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
              {@const downloadJob = activeWorkDownloadJob(product.workId)}
              <article class="product-card" data-tone={typeInfo.tone}>
                <div class="type-belt" aria-hidden="true"></div>
                {#if product.thumbnailUrl}
                  <button
                    class="thumb"
                    type="button"
                    title={`Preview ${product.title}`}
                    aria-label={`Preview image for ${product.title}`}
                    onclick={(event) => {
                      event.stopPropagation();
                      openProductImage(product);
                    }}
                  >
                    <img src={product.thumbnailUrl} alt="" loading="lazy" />
                  </button>
                {:else}
                  <div class="thumb" aria-hidden="true">
                    <span>?</span>
                  </div>
                {/if}
                <div class="product-main">
                  <div class="product-title-row">
                    <button
                      class="product-title"
                      type="button"
                      title={`Open details for ${product.title}`}
                      disabled={productDetailLoadingWorkId === product.workId}
                      onclick={() => openProductDetail(product)}
                    >
                      {product.title}
                    </button>
                    <button
                      class="work-id"
                      type="button"
                      title={`Copy ${product.workId}`}
                      onclick={(event) => {
                        event.stopPropagation();
                        void copyWorkId(product.workId);
                      }}
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
                        onclick={(event) => {
                          event.stopPropagation();
                          void copyCreditField(field);
                        }}
                      >
                        <span class="credit-label">{field.label}</span>
                        <span class:missing={field.missing} class="credit-value">{field.value}</span>
                      </button>
                    {/each}
                  </div>
                  <div class="labeled-row" aria-label="Classifications">
                    <span class="credit-label">Tags</span>
                    <div class="chip-row">
                      <span
                        class="chip type-chip"
                        role="note"
                        aria-label={typeInfo.tooltip}
                        onmouseenter={(event) => showChipTooltip(typeInfo.tooltip, event)}
                        onmousemove={(event) => moveChipTooltip(typeInfo.tooltip, event)}
                        onmouseleave={hideChipTooltip}
                      >
                        {typeInfo.label}
                      </span>
                      {#if ageLabel(product.ageCategory)}
                        <span
                          class="chip age-chip"
                          role="note"
                          data-age={ageTone(product.ageCategory)}
                          aria-label={ageTooltip(product.ageCategory)}
                          onmouseenter={(event) =>
                            showChipTooltip(ageTooltip(product.ageCategory), event)}
                          onmousemove={(event) =>
                            moveChipTooltip(ageTooltip(product.ageCategory), event)}
                          onmouseleave={hideChipTooltip}
                        >
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
                      <button
                        class="secondary small"
                        type="button"
                        title={`Open ${product.workId} on DLsite`}
                        onclick={(event) => {
                          event.stopPropagation();
                          void openDlsiteProductPage(product.workId);
                        }}
                      >
                        DLsite
                      </button>
                      <button
                        class="small"
                        type="button"
                        title={productDownloadActionTitle(product, downloadJob)}
                        disabled={productDownloadActionDisabled(product, downloadJob)}
                        onclick={(event) => {
                          event.stopPropagation();
                          void runProductDownloadAction(product);
                        }}
                      >
                        {productDownloadActionLabel(product, downloadJob)}
                      </button>
                      <button
                        class="secondary small menu-button"
                        type="button"
                        title="More actions"
                        aria-expanded={productActionMenu?.workId === product.workId}
                        onclick={(event) => {
                          event.stopPropagation();
                          toggleProductActionMenu(product, event);
                        }}
                      >
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
    {:else if activeView === "downloads"}
      <section class="downloads-panel" aria-label="Downloads">
        <div class="panel-title download-panel-title">
          <div>
            <h2>Download queue</h2>
            <p>Currently queued and running downloads</p>
          </div>
          <div class="panel-actions">
            <button class="secondary small" type="button" onclick={loadJobs} disabled={jobsLoading}>
              Reload
            </button>
          </div>
        </div>

        <div class="download-summary-strip" aria-label="Download queue summary">
          <div class="download-stat">
            <span>{visibleDownloadJobs().length}</span>
            <small>Current</small>
          </div>
          <div class="download-stat">
            <span>{queuedDownloadJobCount()}</span>
            <small>Queued</small>
          </div>
          <div class="download-stat">
            <span>{runningDownloadJobCount()}</span>
            <small>Running</small>
          </div>
        </div>

        {#if jobsLoading}
          <div class="empty-state">Loading</div>
        {:else if visibleDownloadJobs().length === 0}
          <div class="empty-state">No active downloads</div>
        {:else}
          <div class="download-queue-list" aria-label="Download jobs">
            {#each visibleDownloadJobs() as job (job.id)}
              {@const progressPercent = downloadQueueProgressPercent(job)}
              <article
                class="download-queue-row"
                class:failed={job.status === "failed"}
                data-status={job.status}
              >
                <div class="download-queue-main">
                  <span>{downloadQueueKindLabel(job)}</span>
                  <h2 title={downloadQueueTitle(job)}>{downloadQueueTitle(job)}</h2>
                  <p title={downloadQueueSubtitle(job)}>{downloadQueueSubtitle(job)}</p>
                </div>

                <div class="download-queue-state">
                  <div>
                    <strong class:active={isActiveJob(job)}>{jobLabel(job)}</strong>
                    <small>{jobDetail(job)}</small>
                  </div>
                  {#if progressPercent !== null}
                    <div class="download-progress-track" aria-label={`Progress ${progressPercent}%`}>
                      <span style={`width: ${progressPercent}%`}></span>
                    </div>
                  {/if}
                </div>

                <time datetime={job.finishedAt ?? job.startedAt ?? job.createdAt}>
                  {downloadQueueTime(job)}
                </time>

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
                Sync All
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
                  <button
                    class:disabled={!account.enabled}
                    class="account-enabled-pill"
                    type="button"
                    title={account.enabled ? "Disable account" : "Enable account"}
                    aria-label={account.enabled ? `Disable ${account.label}` : `Enable ${account.label}`}
                    onclick={() => setAccountEnabled(account, !account.enabled)}
                    disabled={Boolean(activeSyncJob)}
                  >
                    {accountEnabledLabel(account)}
                  </button>
                  <button class="account-name" type="button" onclick={() => editAccount(account)}>
                    <span class="account-identity">
                      <span title={account.label}>{account.label}</span>
                      <small title={accountLoginLabel(account)}>{accountLoginLabel(account)}</small>
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
                    <button
                      class="secondary small"
                      type="button"
                      title="Update saved credential"
                      onclick={() => editAccount(account)}
                    >
                      Update Credential
                    </button>
                    <button
                      class="secondary small"
                      type="button"
                      title="Remove account source"
                      onclick={() => removeAccount(account)}
                      disabled={Boolean(activeSyncJob)}
                    >
                      Remove
                    </button>
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
      <div class="activity-layout">
        <section class="activity-panel" aria-label="Jobs">
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

        <section class="activity-panel" aria-label="Audit log">
          <div class="panel-title">
            <div>
              <h2>Audit log</h2>
              <p>{auditLogDir || "App log directory"}</p>
            </div>
            <div class="panel-actions">
              <button
                class="secondary small"
                type="button"
                onclick={openAuditLogDir}
                disabled={!auditLogDir}
              >
                Open Folder
              </button>
              <button
                class="secondary small"
                type="button"
                onclick={loadAuditEvents}
                disabled={auditLoading}
              >
                Reload
              </button>
            </div>
          </div>

          {#if auditLoading}
            <div class="empty-state">Loading</div>
          {:else if visibleAuditEvents().length === 0}
            <div class="empty-state">No audit events</div>
          {:else}
            <div class="audit-list">
              {#each visibleAuditEvents() as event, index (`${event.at}-${event.operation}-${index}`)}
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
        </section>
      </div>
    {:else}
      <div class="settings-layout">
        <form class="settings-panel" onsubmit={saveSettings}>
          <div class="panel-title">
            <div>
              <h2>Storage paths</h2>
              <p>Library is the final managed collection. Download staging keeps resumable partial files and fetched archives.</p>
            </div>
            <button
              class="secondary small"
              type="button"
              onclick={loadSettings}
              disabled={settingsLoading || settingsSaving}
            >
              Reload
            </button>
          </div>

          <div class="settings-field">
            <label for="library-root">
              <span>Library folder</span>
              <small>Final location for managed works after download and unpacking.</small>
            </label>
            <div class="path-control">
              <input
                id="library-root"
                type="text"
                autocomplete="off"
                spellcheck="false"
                bind:value={libraryRoot}
                disabled={settingsLoading || settingsSaving}
              />
              <button
                class="secondary small"
                type="button"
                onclick={() => chooseSettingsDirectory("library")}
                disabled={settingsLoading || settingsSaving}
              >
                Browse
              </button>
            </div>
          </div>

          <div class="settings-field">
            <label for="download-root">
              <span>Download staging folder</span>
              <small>Working folder for partial downloads, retries, and fetched archives. Defaults to your system Downloads folder.</small>
            </label>
            <div class="path-control">
              <input
                id="download-root"
                type="text"
                autocomplete="off"
                spellcheck="false"
                bind:value={downloadRoot}
                disabled={settingsLoading || settingsSaving}
              />
              <button
                class="secondary small"
                type="button"
                onclick={() => chooseSettingsDirectory("download")}
                disabled={settingsLoading || settingsSaving}
              >
                Browse
              </button>
              <button
                class="secondary small"
                type="button"
                onclick={useDefaultDownloadRoot}
                disabled={settingsLoading || settingsSaving}
              >
                Use Default
              </button>
            </div>
          </div>

          <div class="actions">
            <span></span>
            <button type="submit" disabled={settingsLoading || settingsSaving}>
              {settingsSaving ? "Saving" : "Save"}
            </button>
          </div>
        </form>

        <section class="settings-panel about-panel" aria-label="About">
          <div class="panel-title">
            <h2>About</h2>
            <div class="panel-actions about-actions">
              <button
                class="secondary small"
                type="button"
                onclick={() => openExternalUrl(GITHUB_URL, "GitHub")}
              >
                GitHub
              </button>
              <button
                class="secondary small"
                type="button"
                onclick={() => openExternalUrl(DLSITE_URL, "DLsite")}
              >
                DLsite
              </button>
            </div>
          </div>
          <dl class="about-grid">
            <dt>Application</dt>
            <dd>{appInfoValue(appInfo?.name, appInfoLoading)}</dd>

            <dt>Version</dt>
            <dd>{appInfoValue(appInfo?.version, appInfoLoading)}</dd>

            <dt>Identifier</dt>
            <dd>{appInfoValue(appInfo?.identifier, appInfoLoading)}</dd>

            <dt>Tauri</dt>
            <dd>{appInfoValue(appInfo?.tauriVersion, appInfoLoading)}</dd>
          </dl>
        </section>
      </div>
    {/if}
  </section>

  {#if confirmationDialog}
    <div
      class="confirmation-dialog-layer"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="confirmation-dialog-title"
      aria-describedby="confirmation-dialog-message"
    >
      <button
        class="confirmation-dialog-backdrop"
        type="button"
        aria-label="Close confirmation dialog"
        onclick={() => closeConfirmationDialog(false)}
      ></button>
      <section class:danger={confirmationDialog.tone === "danger"} class="confirmation-dialog-panel">
        <div class="confirmation-dialog-heading">
          <div>
            <p>{confirmationDialog.eyebrow}</p>
            <h2 id="confirmation-dialog-title">{confirmationDialog.title}</h2>
          </div>
          <button
            class="confirmation-dialog-close"
            type="button"
            aria-label="Close confirmation dialog"
            onclick={() => closeConfirmationDialog(false)}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        <p id="confirmation-dialog-message" class="confirmation-dialog-message">
          {confirmationDialog.message}
        </p>

        <div class="confirmation-dialog-actions">
          <button class="secondary" type="button" onclick={() => closeConfirmationDialog(false)}>
            {confirmationDialog.cancelLabel}
          </button>
          <button
            class:danger-action={confirmationDialog.tone === "danger"}
            type="button"
            onclick={() => closeConfirmationDialog(true)}
          >
            {confirmationDialog.confirmLabel}
          </button>
        </div>
      </section>
    </div>
  {/if}

  {#if bulkDownloadDialog}
    <div
      class="bulk-dialog-layer"
      role={bulkDownloadDialog.kind === "notice" ? "alertdialog" : "dialog"}
      aria-modal="true"
      aria-labelledby="bulk-dialog-title"
    >
      <button
        class="bulk-dialog-backdrop"
        type="button"
        aria-label="Close bulk download dialog"
        onclick={() => closeBulkDownloadDialog(false)}
      ></button>
      <section class="bulk-dialog-panel">
        <div class="bulk-dialog-heading">
          <div>
            <p>Bulk Download</p>
            <h2 id="bulk-dialog-title">
              {bulkDownloadDialog.kind === "notice" ? "No products to download" : "Start bulk download?"}
            </h2>
          </div>
          <button
            class="bulk-dialog-close"
            type="button"
            aria-label="Close bulk download dialog"
            onclick={() => closeBulkDownloadDialog(false)}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="bulk-dialog-summary" aria-label="Bulk download plan">
          <div>
            <span>Products to download</span>
            <strong>{bulkDownloadDialog.preview.requestedCount}</strong>
          </div>
          <div>
            <span>Checked products</span>
            <strong>{bulkDownloadDialog.preview.plannedCount}</strong>
          </div>
          <div>
            <span>Already downloaded</span>
            <strong>{bulkDownloadDialog.preview.skippedDownloadedCount}</strong>
          </div>
          <div>
            <span>Already queued</span>
            <strong>{bulkDownloadDialog.preview.skippedQueuedCount}</strong>
          </div>
          <div class="wide">
            <span>Expected total download</span>
            <strong>{bulkDownloadExpectedBytesLabel(bulkDownloadDialog.preview)}</strong>
          </div>
        </div>

        {#if bulkDownloadDialog.preview.failedCount > 0}
          <p class="bulk-dialog-warning">
            {bulkDownloadDialog.preview.failedCount} product(s) could not be checked before download. They will still be attempted and may fail.
          </p>
        {/if}

        {#if bulkDownloadDialog.kind === "notice"}
          <p class="bulk-dialog-note">
            Matching products were already downloaded, already queued, or unavailable for this action.
          </p>
        {/if}

        <div class="bulk-dialog-actions">
          {#if bulkDownloadDialog.kind === "notice"}
            <button type="button" onclick={() => closeBulkDownloadDialog(false)}>Close</button>
          {:else}
            <button class="secondary" type="button" onclick={() => closeBulkDownloadDialog(false)}>
              Cancel
            </button>
            <button type="button" onclick={() => closeBulkDownloadDialog(true)}>
              Start Download
            </button>
          {/if}
        </div>
      </section>
    </div>
  {/if}

  {#if productDetail}
    {@const detail = productDetail}
    {@const detailTypeInfo = productTypeFromCode(detail.workType)}
    {@const genericTags = detailTags(detail)}
    <div
      class="product-detail"
      role="dialog"
      aria-modal="true"
      aria-labelledby="product-detail-title"
      tabindex="-1"
    >
      <button
        class="product-detail-backdrop"
        type="button"
        aria-label="Close product detail"
        onclick={closeProductDetail}
      ></button>
      <section class="product-detail-panel" data-tone={detailTypeInfo.tone}>
        <div class="product-detail-belt" aria-hidden="true"></div>
        <div class="product-detail-heading">
          {#if detail.thumbnailUrl}
            <button
              class="detail-thumb"
              type="button"
              aria-label={`Preview image for ${detail.title}`}
              onclick={() => openProductImageFromDetail(detail)}
            >
              <img src={detail.thumbnailUrl} alt="" />
            </button>
          {:else}
            <div class="detail-thumb missing-thumb" aria-hidden="true">?</div>
          {/if}

          <div class="product-detail-title-block">
            <p>Product detail</p>
            <button
              id="product-detail-title"
              class="product-detail-title-copy"
              type="button"
              title={detail.titleVariants.length > 0 ? textVariantsLabel(detail.titleVariants) : `Copy ${detail.title}`}
              onclick={() => copyText("title", detail.title)}
            >
              {detail.title}
            </button>
            <button
              class="link-button"
              type="button"
              onclick={() => openDlsiteProductPage(detail.workId)}
            >
              Open on DLsite
            </button>
          </div>

          <button
            class="work-id detail-work-id"
            type="button"
            title={`Copy ${detail.workId}`}
            onclick={() => copyWorkId(detail.workId)}
          >
            {detail.workId}
          </button>

          <button
            class="image-preview-close"
            type="button"
            aria-label="Close product detail"
            onclick={closeProductDetail}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="product-detail-body">
          <div class="detail-column">
            <section class="detail-section">
              <h3>Identity</h3>
              <div class="detail-grid">
                <div>
                  <span>Maker</span>
                  <button type="button" onclick={() => copyText("maker", detail.makerName)}>
                    {detailValue(detail.makerName)}
                  </button>
                </div>
                <div>
                  <span>Maker ID</span>
                  <button type="button" onclick={() => copyText("maker ID", detail.makerId)}>
                    {detailValue(detail.makerId)}
                  </button>
                </div>
                <div>
                  <span>Type</span>
                  <span>{detailTypeInfo.label}</span>
                </div>
                <div>
                  <span>Age</span>
                  <span>{ageLabel(detail.ageCategory) || "-"}</span>
                </div>
                <div>
                  <span>Size</span>
                  <span>{detail.contentSizeBytes ? formatBytes(detail.contentSizeBytes) : "-"}</span>
                </div>
                <div>
                  <span>Last detail sync</span>
                  <span>{detailDate(detail.lastDetailSyncAt)}</span>
                </div>
              </div>
            </section>

            <section class="detail-section">
              <h3>Ownership</h3>
              <div class="detail-chip-list">
                {#each detail.owners as owner (owner.accountId)}
                  <span title={owner.purchasedAt ? `${owner.label}: ${shortDate(owner.purchasedAt)}` : owner.label}>
                    {owner.label}
                  </span>
                {/each}
              </div>
            </section>

            <section class="detail-section">
              <h3>Dates</h3>
              <div class="detail-grid">
                <div>
                  <span>Registered</span>
                  <span>{detailDate(detail.registeredAt)}</span>
                </div>
                <div>
                  <span>Published</span>
                  <span>{detailDate(detail.publishedAt)}</span>
                </div>
                <div>
                  <span>Updated</span>
                  <span>{detailDate(detail.updatedAt)}</span>
                </div>
                <div>
                  <span>Latest Purchase</span>
                  <span>{detailDate(detail.latestPurchasedAt)}</span>
                </div>
              </div>
            </section>
          </div>

          <div class="detail-column">
            <section class="detail-section">
              <h3>Credits</h3>
              <div class="detail-credit-list">
                {#each productCreditFields(detail) as field (field.key)}
                  <button
                    type="button"
                    disabled={field.missing}
                    title={creditTooltip(field)}
                    onclick={() => copyCreditField(field)}
                  >
                    <span>{field.label}</span>
                    <strong class:missing={field.missing}>{field.value}</strong>
                  </button>
                {/each}
              </div>
            </section>

            <section class="detail-section">
              <h3>Download</h3>
              <div class="detail-grid">
                <div>
                  <span>Status</span>
                  <span>{downloadStatusLabel(detail.download.status)}</span>
                </div>
                <div>
                  <span>Policy</span>
                  <span>{detailValue(detail.download.unpackPolicy)}</span>
                </div>
                <div class="wide">
                  <span>Local path</span>
                  <button type="button" onclick={() => copyText("local path", detail.download.localPath)}>
                    {detailValue(detail.download.localPath)}
                  </button>
                </div>
                {#if detail.download.errorMessage}
                  <div class="wide">
                    <span>Error</span>
                    <span>{detail.download.errorMessage}</span>
                  </div>
                {/if}
              </div>
            </section>

            <section class="detail-section">
              <h3>Tags</h3>
              {#if genericTags.length > 0}
                <div class="detail-chip-list">
                  {#each genericTags as tag (`${tag.class}:${tag.name}`)}
                    <span title={tag.class}>{tag.name}</span>
                  {/each}
                </div>
              {:else}
                <p class="detail-muted">No tags cached</p>
              {/if}
            </section>
          </div>
        </div>
      </section>
    </div>
  {/if}

  {#if productImagePreview}
    <div
      class="image-preview"
      role="dialog"
      aria-modal="true"
      aria-labelledby="image-preview-title"
      tabindex="-1"
    >
      <button
        class="image-preview-backdrop"
        type="button"
        aria-label="Close image preview"
        onclick={closeProductImage}
      ></button>
      <div class="image-preview-panel">
        <div class="image-preview-heading">
          <div>
            <h2 id="image-preview-title">{productImagePreview.title}</h2>
            <p>{productImagePreview.workId}</p>
          </div>
          <button
            class="image-preview-close"
            type="button"
            aria-label="Close image preview"
            onclick={closeProductImage}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M18 6 6 18M6 6l12 12" />
            </svg>
          </button>
        </div>
        <div class="image-preview-frame">
          <img src={productImagePreview.url} alt="" />
        </div>
      </div>
    </div>
  {/if}

  {#if chipTooltip}
    <div
      class="chip-tooltip"
      role="tooltip"
      style={`left: ${chipTooltip.left}px; top: ${chipTooltip.top}px;`}
    >
      {chipTooltip.text}
    </div>
  {/if}

  {#if productActionMenu}
    {@const menuProduct = productActionMenuProduct()}
    {#if menuProduct}
      {@const menuDownloadJob = activeWorkDownloadJob(menuProduct.workId)}
      <div
        class="product-action-menu"
        role="menu"
        tabindex="-1"
        aria-label={`Actions for ${menuProduct.workId}`}
        style={`left: ${productActionMenu.left}px; top: ${productActionMenu.top}px;`}
        onclick={(event) => event.stopPropagation()}
        onkeydown={(event) => {
          if (event.key === "Escape") {
            closeProductActionMenu();
          }
        }}
      >
        {#if menuProduct.download.status !== "downloaded"}
          <button
            type="button"
            role="menuitem"
            disabled={!!menuDownloadJob}
            onclick={() => downloadProductArchivesOnly(menuProduct)}
          >
            Download Archives Only
          </button>
          <button
            type="button"
            role="menuitem"
            disabled={!!menuDownloadJob}
            onclick={() => markProductDownloaded(menuProduct)}
          >
            Mark as Downloaded
          </button>
        {/if}
        {#if menuProduct.download.status === "downloaded"}
          <button
            class="danger"
            type="button"
            role="menuitem"
            disabled={!!menuDownloadJob}
            onclick={() => redownloadProduct(menuProduct)}
          >
            Re-download
          </button>
        {/if}
        {#if productHasDownloadRecord(menuProduct)}
          <button
            class="danger"
            type="button"
            role="menuitem"
            disabled={!!menuDownloadJob}
            onclick={() => deleteDownloadedProduct(menuProduct)}
          >
            Delete Download
          </button>
        {/if}
      </div>
    {/if}
  {/if}

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

  .workspace.library-workspace {
    overflow: auto;
    padding-top: 0;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
  }

  .workspace.library-workspace .workspace-header {
    padding-top: 28px;
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
  .downloads-panel,
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
    z-index: 120;
    display: grid;
    gap: 8px;
    width: min(380px, calc(100vw - 36px));
    pointer-events: none;
  }

  .chip-tooltip {
    position: fixed;
    z-index: 50;
    max-width: 320px;
    padding: 7px 9px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: var(--text);
    background: color-mix(in srgb, var(--panel-raised) 94%, black);
    box-shadow: 0 12px 28px rgb(0 0 0 / 34%);
    font-size: 12px;
    font-weight: 600;
    line-height: 1.35;
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

  .confirmation-dialog-layer {
    position: fixed;
    z-index: 60;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
  }

  .confirmation-dialog-backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: rgb(0 0 0 / 68%);
    cursor: default;
  }

  .confirmation-dialog-panel {
    position: relative;
    z-index: 1;
    display: grid;
    gap: 16px;
    width: min(520px, calc(100vw - 40px));
    padding: 18px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 24px 64px rgb(0 0 0 / 52%);
  }

  .confirmation-dialog-panel.danger {
    border-color: rgb(248 113 113 / 42%);
  }

  .confirmation-dialog-heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
  }

  .confirmation-dialog-heading p,
  .confirmation-dialog-message {
    margin: 0;
  }

  .confirmation-dialog-heading p {
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .confirmation-dialog-heading h2 {
    margin-top: 2px;
    font-size: 20px;
    line-height: 1.2;
  }

  .confirmation-dialog-close {
    width: 34px;
    min-width: 34px;
    height: 34px;
    padding: 0;
    border-color: var(--border-strong);
    color: var(--muted);
    background: var(--panel-raised);
  }

  .confirmation-dialog-close:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .confirmation-dialog-close svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.35;
  }

  .confirmation-dialog-message {
    color: var(--text);
    font-size: 14px;
    line-height: 1.5;
  }

  .confirmation-dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  button.danger-action {
    border-color: #fca5a5;
    color: #1b0707;
    background: #fca5a5;
  }

  button.danger-action:hover:not(:disabled) {
    border-color: #fecaca;
    background: #fecaca;
  }

  .bulk-dialog-layer {
    position: fixed;
    z-index: 50;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
  }

  .bulk-dialog-backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: rgb(0 0 0 / 68%);
    cursor: default;
  }

  .bulk-dialog-panel {
    position: relative;
    z-index: 1;
    display: grid;
    gap: 16px;
    width: min(560px, calc(100vw - 40px));
    max-height: calc(100vh - 48px);
    padding: 18px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 24px 64px rgb(0 0 0 / 52%);
    overflow: auto;
  }

  .bulk-dialog-heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
  }

  .bulk-dialog-heading p,
  .bulk-dialog-note,
  .bulk-dialog-warning {
    margin: 0;
  }

  .bulk-dialog-heading p {
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .bulk-dialog-heading h2 {
    margin-top: 2px;
    font-size: 20px;
  }

  .bulk-dialog-close {
    width: 34px;
    min-width: 34px;
    height: 34px;
    padding: 0;
    border-color: var(--border-strong);
    color: var(--muted);
    background: var(--panel-raised);
  }

  .bulk-dialog-close:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .bulk-dialog-close svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.35;
  }

  .bulk-dialog-summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--border);
    overflow: hidden;
  }

  .bulk-dialog-summary div {
    display: grid;
    gap: 4px;
    padding: 11px 12px;
    background: var(--panel-soft);
  }

  .bulk-dialog-summary .wide {
    grid-column: 1 / -1;
  }

  .bulk-dialog-summary span {
    color: var(--muted);
    font-size: 12px;
    font-weight: 650;
  }

  .bulk-dialog-summary strong {
    min-width: 0;
    color: var(--text-strong);
    font-size: 17px;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }

  .bulk-dialog-warning {
    padding: 10px 12px;
    border: 1px solid rgb(248 113 113 / 36%);
    border-radius: 8px;
    color: #fecaca;
    background: rgb(248 113 113 / 11%);
    font-size: 13px;
    line-height: 1.45;
  }

  .bulk-dialog-note {
    color: var(--muted);
    font-size: 13px;
    line-height: 1.45;
  }

  .bulk-dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .product-detail {
    position: fixed;
    z-index: 45;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 28px;
  }

  .product-detail-backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: rgb(0 0 0 / 70%);
    cursor: default;
  }

  .product-detail-panel {
    --type-color: #6b7177;
    --type-soft: rgb(107 113 119 / 18%);

    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: 5px minmax(0, 1fr);
    width: min(980px, 94vw);
    max-height: 90vh;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 24px 64px rgb(0 0 0 / 52%);
    overflow: hidden;
  }

  .product-detail-panel[data-tone="audio"] {
    --type-color: #d8a62d;
    --type-soft: rgb(216 166 45 / 17%);
  }

  .product-detail-panel[data-tone="video"] {
    --type-color: #d64b92;
    --type-soft: rgb(214 75 146 / 17%);
  }

  .product-detail-panel[data-tone="voice-comic"] {
    --type-color: #55bfe6;
    --type-soft: rgb(85 191 230 / 16%);
  }

  .product-detail-panel[data-tone="game"] {
    --type-color: #9863df;
    --type-soft: rgb(152 99 223 / 17%);
  }

  .product-detail-panel[data-tone="image"] {
    --type-color: #4fb85b;
    --type-soft: rgb(79 184 91 / 16%);
  }

  .product-detail-belt {
    grid-row: 1 / 3;
    background: var(--type-color);
  }

  .product-detail-heading {
    display: grid;
    grid-template-columns: 120px minmax(0, 1fr) auto auto;
    gap: 14px;
    align-items: start;
    min-width: 0;
    padding: 16px;
    border-bottom: 1px solid var(--border);
  }

  .detail-thumb {
    width: 120px;
    height: 120px;
    min-width: 0;
    padding: 0;
    border-color: var(--border-strong);
    border-radius: 6px;
    background: var(--panel-raised);
    overflow: hidden;
  }

  .detail-thumb:hover {
    border-color: var(--type-color);
  }

  .detail-thumb img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .missing-thumb {
    display: grid;
    place-items: center;
    color: var(--text-subtle);
    font-weight: 700;
  }

  .product-detail-title-block {
    display: grid;
    align-content: start;
    gap: 6px;
    min-width: 0;
  }

  .product-detail-title-block p {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .product-detail-title-copy {
    display: block;
    width: 100%;
    height: auto;
    min-width: 0;
    min-height: 0;
    padding: 0;
    border: 0;
    color: var(--text-strong);
    background: transparent;
    font-size: 22px;
    font-weight: 700;
    line-height: 1.24;
    text-align: left;
    overflow-wrap: anywhere;
  }

  .product-detail-title-copy:hover:not(:disabled) {
    color: var(--accent);
    background: transparent;
  }

  .product-detail-title-copy:focus-visible {
    outline: 2px solid var(--accent-muted);
    outline-offset: 2px;
  }

  .link-button {
    justify-self: start;
    min-height: 26px;
    padding: 0;
    border: 0;
    color: var(--accent);
    background: transparent;
    font-size: 12px;
    font-weight: 650;
  }

  .link-button:hover {
    color: var(--text-strong);
    background: transparent;
  }

  .detail-work-id {
    align-self: start;
  }

  .product-detail-body {
    display: grid;
    grid-template-columns: minmax(0, 0.95fr) minmax(0, 1.05fr);
    align-items: start;
    gap: 12px;
    min-height: 0;
    max-height: calc(90vh - 154px);
    padding: 16px;
    overflow: auto;
  }

  .detail-column {
    display: grid;
    align-content: start;
    gap: 12px;
    min-width: 0;
  }

  .detail-section {
    min-width: 0;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
  }

  .detail-section h3 {
    margin: 0 0 10px;
    color: var(--text-strong);
    font-size: 13px;
    font-weight: 700;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(132px, 1fr));
    gap: 12px;
  }

  .detail-grid div {
    display: grid;
    align-content: start;
    gap: 4px;
    min-width: 0;
  }

  .detail-credit-list button {
    display: grid;
    grid-template-columns: 104px minmax(0, 1fr);
    gap: 8px;
    align-items: baseline;
    min-width: 0;
  }

  .detail-grid .wide {
    grid-column: 1 / -1;
  }

  .detail-grid span:first-child,
  .detail-credit-list span {
    color: var(--text-subtle);
    font-size: 12px;
    font-weight: 700;
  }

  .detail-grid span:last-child,
  .detail-grid button,
  .detail-credit-list strong {
    min-width: 0;
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .detail-grid button,
  .detail-credit-list button {
    height: auto;
    min-height: 0;
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: 3px;
    color: inherit;
    background: transparent;
    text-align: left;
  }

  .detail-grid button {
    justify-self: start;
    max-width: 100%;
  }

  .detail-credit-list button {
    width: 100%;
  }

  .detail-grid button:hover:not(:disabled),
  .detail-credit-list button:hover:not(:disabled) strong {
    color: var(--text-strong);
  }

  .detail-credit-list {
    display: grid;
    gap: 12px;
  }

  .detail-credit-list strong.missing {
    color: var(--text-subtle);
    opacity: 0.72;
  }

  .detail-chip-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }

  .detail-chip-list span {
    max-width: 100%;
    padding: 4px 8px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--text);
    background: var(--panel-raised);
    font-size: 12px;
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail-muted {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }

  .image-preview {
    position: fixed;
    z-index: 90;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 28px;
  }

  .image-preview-backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: rgb(0 0 0 / 70%);
    cursor: default;
  }

  .image-preview-panel {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 14px;
    width: min(920px, 92vw);
    max-height: 88vh;
    padding: 16px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 24px 64px rgb(0 0 0 / 52%);
  }

  .image-preview-heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
  }

  .image-preview-heading h2 {
    margin: 0;
    color: var(--text-strong);
    font-size: 17px;
    line-height: 1.25;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .image-preview-heading p {
    margin: 4px 0 0;
    color: var(--muted);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    font-size: 12px;
  }

  .image-preview-close {
    width: 34px;
    min-width: 34px;
    height: 34px;
    padding: 0;
    border-color: var(--border-strong);
    color: var(--muted);
    background: var(--panel-raised);
  }

  .image-preview-close:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .image-preview-close svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.35;
  }

  .image-preview-frame {
    display: grid;
    place-items: center;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    overflow: hidden;
  }

  .image-preview-frame img {
    display: block;
    max-width: 100%;
    max-height: calc(88vh - 110px);
    object-fit: contain;
  }

  .product-area {
    display: flex;
    flex: 0 0 auto;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: visible;
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
    width: 100%;
    min-width: 0;
  }

  .settings-layout {
    display: grid;
    flex: 1 1 auto;
    align-content: start;
    gap: 14px;
    width: 100%;
    min-width: 0;
    min-height: 0;
    overflow: auto;
    scrollbar-gutter: stable;
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

  .library-controls {
    position: sticky;
    top: 0;
    z-index: 30;
    display: grid;
    flex: 0 0 auto;
    gap: 1px;
    border-bottom: 1px solid var(--border);
    background: var(--border);
    border-radius: 7px 7px 0 0;
    box-shadow: 0 14px 26px rgb(0 0 0 / 22%);
    overflow: hidden;
  }

  .library-search-panel,
  .library-filter-panel,
  .library-actions-panel {
    min-width: 0;
    padding: 14px;
    background: var(--panel-soft);
  }

  .library-search-panel,
  .library-filter-panel {
    display: grid;
    gap: 10px;
  }

  .library-filter-panel {
    flex: 0 0 auto;
    border-bottom: 1px solid var(--border);
  }

  .library-search-row {
    display: grid;
    grid-template-columns: minmax(260px, 1fr) auto auto auto;
    gap: 10px;
    align-items: center;
  }

  .filter-fold-button {
    min-width: 112px;
  }

  .filter-grid {
    display: grid;
    gap: 10px;
  }

  .filter-group {
    display: grid;
    grid-template-columns: 78px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    min-width: 0;
  }

  .filter-group > span {
    padding-top: 6px;
    color: var(--text-subtle);
    font-size: 12px;
    font-weight: 700;
  }

  .toggle-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }

  .maker-filter .toggle-row {
    align-items: flex-start;
  }

  .toggle-row button {
    justify-content: flex-start;
    min-width: 0;
    max-width: 210px;
    height: 30px;
    padding: 0 10px;
    border-color: var(--border-strong);
    color: var(--muted);
    background: var(--field);
    font-size: 12px;
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .toggle-row button.active {
    border-color: var(--accent);
    color: var(--text-strong);
    background: var(--accent-muted);
  }

  .filter-chip-label {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .toggle-row button small {
    flex: 0 0 auto;
    margin-left: 6px;
    color: var(--text-subtle);
    font-size: 11px;
    font-weight: 700;
  }

  .library-actions-panel {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
    width: 100%;
    padding-block: 10px;
  }

  .library-action-group {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    min-width: 0;
  }

  .download-results-button {
    min-width: 128px;
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
    flex: 0 0 auto;
    min-height: 0;
    overflow: visible;
    overflow-anchor: none;
    overscroll-behavior: contain;
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
    contain: layout paint;
    overflow: hidden;
    overflow-anchor: none;
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

  .product-card[data-tone="voice-comic"] {
    --type-color: #55bfe6;
    --type-soft: rgb(85 191 230 / 16%);
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
    display: block;
    width: var(--thumb-size);
    height: var(--thumb-size);
    min-width: 0;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: inherit;
    background: var(--panel-raised);
    cursor: pointer;
    overflow: hidden;
  }

  .thumb:hover {
    border-color: var(--type-color);
  }

  .thumb:focus-visible {
    border-color: var(--type-color);
    outline: 2px solid var(--type-soft);
    outline-offset: 2px;
  }

  .thumb[aria-hidden="true"] {
    cursor: default;
  }

  .thumb[aria-hidden="true"]:hover {
    border-color: var(--border-strong);
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
    display: block;
    width: 100%;
    min-height: 0;
    min-width: 0;
    height: auto;
    padding: 0;
    border: 0;
    color: var(--text-strong);
    background: transparent;
    font-size: 17px;
    font-weight: 700;
    line-height: 1.25;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .product-title:hover:not(:disabled) {
    color: var(--accent);
    background: transparent;
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

  .product-action-menu {
    position: fixed;
    z-index: 80;
    display: grid;
    width: 220px;
    gap: 4px;
    padding: 6px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-raised);
    box-shadow: 0 18px 40px rgb(0 0 0 / 38%);
  }

  .product-action-menu button {
    justify-content: flex-start;
    width: 100%;
    min-height: 34px;
    padding: 0 10px;
    border: 0;
    color: var(--text);
    background: transparent;
    font-size: 13px;
    text-align: left;
  }

  .product-action-menu button:hover:not(:disabled) {
    background: var(--panel-soft);
  }

  .product-action-menu button.danger {
    color: var(--danger);
  }

  .product-action-menu button.danger:hover:not(:disabled) {
    background: rgb(248 113 113 / 11%);
  }

  .panel-title {
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 14px;
  }

  .panel-title > div {
    min-width: 0;
  }

  .panel-title p {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.35;
  }

  .account-list {
    display: grid;
    gap: 8px;
  }

  .account-row {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) minmax(340px, 0.88fr);
    grid-template-rows: auto auto auto;
    gap: 10px 30px;
    align-items: start;
    min-height: 132px;
    padding: 14px 16px;
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
    grid-row: 2 / 4;
    align-self: start;
    justify-content: stretch;
    justify-items: stretch;
    width: 100%;
    min-width: 0;
    height: auto;
    min-height: 0;
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
    gap: 3px;
    justify-self: start;
    width: min(360px, 100%);
    min-width: 0;
  }

  .account-identity span {
    max-width: 100%;
    color: var(--text);
    font-size: 15px;
    font-weight: 650;
    line-height: 1.1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .account-identity small {
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .account-enabled-pill {
    display: inline-flex;
    align-items: center;
    grid-column: 1;
    grid-row: 1;
    justify-content: flex-start;
    justify-self: start;
    gap: 6px;
    min-width: 114px;
    height: auto;
    min-height: 28px;
    padding: 3px 10px;
    border: 1px solid rgb(112 165 120 / 58%);
    border-radius: 5px;
    color: var(--accent);
    background: var(--accent-muted);
    font-size: 12px;
    font-weight: 650;
    line-height: 1.1;
    cursor: pointer;
  }

  .account-enabled-pill::before {
    content: "";
    width: 9px;
    height: 9px;
    border: 2px solid currentColor;
    border-radius: 999px;
    background: rgb(160 198 164 / 24%);
  }

  .account-enabled-pill.disabled {
    border-color: var(--border-strong);
    color: var(--text-subtle);
    background: var(--field-disabled);
  }

  .account-enabled-pill:disabled {
    cursor: default;
  }

  .account-meta-grid {
    display: grid;
    grid-column: 2;
    grid-row: 2;
    align-self: start;
    grid-template-columns: 1fr;
    gap: 7px;
    min-width: 0;
  }

  .account-meta-grid div {
    display: grid;
    grid-template-columns: minmax(92px, 0.42fr) minmax(170px, 1fr);
    gap: 16px;
    align-items: baseline;
    min-width: 0;
  }

  .account-meta-grid span {
    color: var(--muted);
    font-size: 13px;
    font-weight: 600;
  }

  .account-meta-grid strong {
    min-width: 0;
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .account-actions {
    grid-column: 2;
    grid-row: 3;
    align-self: end;
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  .job-list {
    display: grid;
    gap: 7px;
  }

  .job-list.large {
    flex: 1 1 auto;
    gap: 0;
    min-height: 0;
    padding-right: 4px;
    overflow: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
  }

  .downloads-panel {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
    min-height: 0;
    padding: 18px;
    overflow: hidden;
  }

  .download-panel-title {
    flex: 0 0 auto;
  }

  .download-panel-title > div {
    min-width: 0;
  }

  .download-panel-title p {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12px;
  }

  .download-summary-strip {
    display: grid;
    flex: 0 0 auto;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--border);
    overflow: hidden;
  }

  .download-stat {
    display: grid;
    gap: 2px;
    padding: 10px 12px;
    background: var(--panel-soft);
  }

  .download-stat span {
    color: var(--text-strong);
    font-size: 18px;
    font-weight: 700;
    line-height: 1.1;
  }

  .download-stat small {
    color: var(--muted);
    font-size: 12px;
    font-weight: 650;
  }

  .download-queue-list {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-height: 0;
    padding-right: 4px;
    overflow: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
  }

  .download-queue-row {
    display: grid;
    flex: 0 0 auto;
    grid-template-columns: minmax(220px, 1.2fr) minmax(220px, 0.8fr) minmax(150px, auto) auto;
    gap: 14px;
    align-items: center;
    min-height: 76px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }

  .download-queue-row:last-child {
    border-bottom: 0;
  }

  .download-queue-row.failed h2 {
    color: var(--danger);
  }

  .download-queue-main,
  .download-queue-state {
    min-width: 0;
  }

  .download-queue-main span {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--muted);
    background: var(--panel-soft);
    font-size: 11px;
    font-weight: 700;
    line-height: 1;
  }

  .download-queue-main h2 {
    margin-top: 7px;
    color: var(--text-strong);
    font-size: 15px;
    line-height: 1.25;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .download-queue-main p,
  .download-queue-state small,
  .download-queue-row time {
    color: var(--muted);
    font-size: 12px;
  }

  .download-queue-main p {
    margin: 3px 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .download-queue-state {
    display: grid;
    gap: 8px;
  }

  .download-queue-state > div:first-child {
    display: grid;
    gap: 2px;
  }

  .download-queue-state strong {
    color: var(--text);
    font-size: 13px;
    font-weight: 700;
  }

  .download-queue-state strong.active {
    color: var(--accent);
  }

  .download-progress-track {
    height: 6px;
    border-radius: 999px;
    background: var(--field);
    overflow: hidden;
  }

  .download-progress-track span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
  }

  .download-queue-row time {
    text-align: right;
    white-space: nowrap;
  }

  .activity-layout {
    display: grid;
    flex: 1 1 auto;
    grid-template-rows: minmax(120px, 0.42fr) minmax(0, 1fr);
    gap: 18px;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .activity-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .activity-panel > .panel-title {
    flex: 0 0 auto;
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

  .audit-list {
    display: grid;
    flex: 1 1 auto;
    gap: 0;
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

  .audit-row time {
    color: var(--text-subtle);
    font-size: 12px;
    white-space: nowrap;
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

  label small {
    color: var(--muted);
    font-size: 12px;
    line-height: 1.35;
  }

  .settings-field {
    display: grid;
    gap: 8px;
  }

  .about-panel {
    gap: 10px;
  }

  .about-grid {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    column-gap: 18px;
    row-gap: 8px;
    margin: 0;
    font-size: 13px;
  }

  .about-grid dt {
    color: var(--muted);
    font-weight: 650;
  }

  .about-grid dd {
    min-width: 0;
    margin: 0;
    color: var(--text);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .path-control {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: 8px;
    align-items: center;
  }

  input {
    width: 100%;
    min-width: 0;
    height: 38px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: var(--text);
    background: var(--field);
  }

  input:focus {
    border-color: var(--accent-strong);
    outline: 2px solid var(--accent-muted);
  }

  input:disabled {
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
    .download-queue-row {
      grid-template-columns: minmax(0, 1fr) minmax(180px, 0.7fr) auto;
    }

    .download-queue-row button {
      grid-column: 1 / -1;
      justify-self: start;
    }

    .account-row {
      grid-template-columns: minmax(0, 1fr);
    }

    .account-name {
      grid-column: 1;
      grid-row: 2;
    }

    .account-meta-grid {
      grid-column: 1 / -1;
      grid-row: 3;
    }

    .account-actions {
      grid-column: 1 / -1;
      grid-row: 4;
      justify-content: flex-start;
    }

    .library-controls {
      grid-template-columns: 1fr;
    }

    .library-actions-panel {
      justify-content: flex-start;
      width: 100%;
    }
  }

  @media (max-width: 980px) {
    .accounts-layout {
      grid-template-columns: 1fr;
    }

    .account-editor {
      position: static;
    }

    .library-search-row {
      grid-template-columns: minmax(0, 1fr) auto auto;
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

    .product-detail-heading {
      grid-template-columns: 86px minmax(0, 1fr) auto;
      gap: 12px;
    }

    .detail-thumb {
      width: 86px;
      height: 86px;
    }

    .detail-work-id {
      grid-column: 2;
      grid-row: 2;
      justify-self: start;
    }

    .product-detail-body {
      grid-template-columns: 1fr;
      max-height: calc(90vh - 124px);
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

    .library-search-row,
    .filter-group {
      grid-template-columns: 1fr;
    }

    .toggle-row button,
    .library-actions-panel button {
      flex: 1 1 130px;
    }

    .path-control {
      grid-template-columns: 1fr;
    }

    .account-summary-strip {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .download-summary-strip {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .download-queue-row {
      grid-template-columns: 1fr;
    }

    .download-queue-row time {
      text-align: left;
    }

    .account-row,
    .account-meta-grid {
      grid-template-columns: 1fr;
    }

    .account-enabled-pill,
    .account-name,
    .account-meta-grid,
    .account-actions {
      grid-column: 1;
      grid-row: auto;
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

    .product-detail {
      padding: 16px;
    }

    .product-detail-heading {
      grid-template-columns: 72px minmax(0, 1fr) auto;
      padding: 12px;
    }

    .product-detail-title-copy {
      font-size: 18px;
    }

    .detail-thumb {
      width: 72px;
      height: 72px;
    }

    .product-detail-body {
      max-height: calc(90vh - 108px);
      padding: 12px;
    }

    .detail-grid {
      grid-template-columns: 1fr;
    }

    .detail-credit-list button {
      grid-template-columns: 82px minmax(0, 1fr);
    }

    button,
    button.secondary {
      width: 100%;
    }

    .product-actions button,
    .product-actions button.secondary,
    .account-enabled-pill,
    .account-actions button,
    .account-actions button.secondary,
    .download-queue-row button,
    .download-queue-row button.secondary,
    .detail-grid button,
    .detail-credit-list button,
    .link-button,
    .path-control button,
    .path-control button.secondary,
    .work-id {
      width: auto;
    }
  }
</style>
