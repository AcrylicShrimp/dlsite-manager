<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import * as native from "$lib/api/native";
  import * as commands from "$lib/api/tauri";
  import AppShell from "$lib/components/AppShell.svelte";
  import BulkDownloadDialogView from "$lib/components/BulkDownloadDialog.svelte";
  import ConfirmationDialogView from "$lib/components/ConfirmationDialog.svelte";
  import ToastStack from "$lib/components/ToastStack.svelte";
  import TwoFactorDialog from "$lib/components/TwoFactorDialog.svelte";
  import { JobController } from "$lib/controllers/job-controller.svelte";
  import AccountsView from "$lib/features/accounts/AccountsView.svelte";
  import ActivityView from "$lib/features/activity/ActivityView.svelte";
  import DownloadsView from "$lib/features/downloads/DownloadsView.svelte";
  import LibraryView from "$lib/features/library/LibraryView.svelte";
  import ProductActionMenuView from "$lib/features/library/ProductActionMenu.svelte";
  import ProductDetailDialog from "$lib/features/library/ProductDetailDialog.svelte";
  import ProductImagePreviewView from "$lib/features/library/ProductImagePreview.svelte";
  import SettingsView from "$lib/features/settings/SettingsView.svelte";
  import { DLSITE_URL, GITHUB_URL } from "$lib/model/constants";
  import {
    errorMessage,
    formatBytes,
    shortDate,
    valueOrNull,
  } from "$lib/utils/format";
  import {
    activeJobDetail,
    bulkDownloadResult,
    isActiveJob,
    isDownloadQueueJob,
    isTerminalJob,
    jobAccountId,
    jobLabel,
    jobOutputBoolean,
    jobOutputNumber,
    jobOutputString,
    jobWorkId,
    metadataNumber,
  } from "$lib/utils/jobs";
  import {
    ageTooltip,
    creditTextForKind,
  } from "$lib/utils/products";
  import type {
    Account,
    AppInfo,
    AuditEvent,
    BulkDownloadDialog,
    BulkWorkDownloadPreview,
    ChipTooltip,
    ConfirmationDialog,
    JobEvent,
    JobSnapshot,
    Product,
    ProductActionMenu,
    ProductCreditField,
    ProductCustomTag,
    ProductDetail,
    ProductDownload,
    ProductFilterFacets,
    ProductImagePreview,
    StartWorkDownloadOptions,
    TwoFactorClosed,
    TwoFactorRequest,
    Toast,
    ToastKind,
    View,
  } from "$lib/model/types";

  const PRODUCT_PAGE_SIZE = 100;

  type ProductLoadOptions = {
    resetPage?: boolean;
    clampInvalidPage?: boolean;
  };

  let activeView = $state<View>("library");

  let libraryRoot = $state("");
  let downloadRoot = $state("");
  let settingsLoading = $state(true);
  let settingsSaving = $state(false);
  let appInfo = $state<AppInfo | null>(null);
  let appInfoLoading = $state(true);
  let updatePhase = $state<"idle" | "checking" | "downloading" | "installing">("idle");
  let updateProgressMessage = $state("");

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
  let productPageIndex = $state(0);
  let bulkDownloadPlanning = $state(false);
  let productSearch = $state("");
  let selectedAccountIds = $state<string[]>([]);
  let selectedProductTypes = $state<string[]>([]);
  let selectedAgeCategories = $state<string[]>([]);
  let selectedProductSources = $state<string[]>([]);
  let selectedMakerNames = $state<string[]>([]);
  let selectedCustomTagNames = $state<string[]>([]);
  let excludedCustomTagNames = $state<string[]>([]);
  let productFilterFacets = $state<ProductFilterFacets>({ makers: [], customTags: [] });
  let productSort = $state("latestPurchaseDesc");
  let libraryFiltersOpen = $state(false);

  const jobController = new JobController();
  let auditEvents = $state<AuditEvent[]>([]);
  let auditLoading = $state(true);
  let auditLogDir = $state("");
  let toasts = $state<Toast[]>([]);
  let productImagePreview = $state<ProductImagePreview | null>(null);
  let productActionMenu = $state<ProductActionMenu | null>(null);
  let productDetail = $state<ProductDetail | null>(null);
  let productDetailLoadingWorkId = $state<string | null>(null);
  let customTagInput = $state("");
  let chipTooltip = $state<ChipTooltip | null>(null);
  let bulkDownloadDialog = $state<BulkDownloadDialog | null>(null);
  let confirmationDialog = $state<ConfirmationDialog | null>(null);
  let twoFactorQueue = $state<TwoFactorRequest[]>([]);
  let twoFactorSubmitting = $state(false);

  let toastSequence = 0;
  let bulkDownloadDialogResolve: ((confirmed: boolean) => void) | null = null;
  let confirmationDialogResolve: ((confirmed: boolean) => void) | null = null;
  const toastTimers = new Map<string, ReturnType<typeof setTimeout>>();

  onMount(() => {
    void loadInitial();

    const unlisteners: (() => void)[] = [];
    let disposed = false;

    const register = (pending: Promise<() => void>) => {
      void pending.then((cleanup) => {
        if (disposed) {
          cleanup();
        } else {
          unlisteners.push(cleanup);
        }
      });
    };

    register(jobController.listen(handleJobEvent));
    register(native.listenToTwoFactorRequests(queueTwoFactorRequest));
    register(
      native.listenToTwoFactorClosures((closed) => {
        // The job stopped waiting (timeout, cancellation, or another window answered).
        dropTwoFactorRequest(closed.requestId);
      }),
    );

    return () => {
      disposed = true;

      for (const unlisten of unlisteners) {
        unlisten();
      }
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
      appInfo = await native.getAppInfo();
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
      const settings = await commands.getSettings();
      const defaultDownloadRoot = await systemDownloadRoot();
      libraryRoot = settings.libraryRoot ?? "";
      downloadRoot = settings.downloadRoot ?? defaultDownloadRoot;
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      settingsLoading = false;
    }
  }

  async function checkForUpdates() {
    if (updatePhase !== "idle") {
      return;
    }

    updatePhase = "checking";
    updateProgressMessage = "Checking for updates";

    try {
      const version = await native.downloadAndInstallAvailableUpdate((progress) => {
        updatePhase = progress.phase;

        if (progress.phase === "installing") {
          updateProgressMessage = `Installing ${progress.version}`;
        } else if (progress.contentLength && progress.contentLength > 0) {
          const percent = Math.min(100, Math.floor((progress.downloadedBytes / progress.contentLength) * 100));
          updateProgressMessage = `Downloading ${progress.version} ${percent}%`;
        } else {
          updateProgressMessage = `Downloading ${progress.version} ${formatBytes(progress.downloadedBytes)}`;
        }
      });

      if (!version) {
        updateProgressMessage = "";
        notifyInfo("dlsite-manager is up to date");
        return;
      }

      updateProgressMessage = `Installed ${version}. Relaunching`;
      notifySuccess(`Installed update ${version}. Relaunching`);
      await native.relaunchApp();
    } catch (err) {
      updateProgressMessage = "";
      notifyError(`Update failed: ${errorMessage(err)}`);
    } finally {
      updatePhase = "idle";
    }
  }

  async function saveSettings(event: Event) {
    event.preventDefault();
    settingsSaving = true;

    try {
      const settings = await commands.saveSettings({
        libraryRoot: valueOrNull(libraryRoot),
        downloadRoot: valueOrNull(downloadRoot),
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
      const selected = await native.chooseDirectory({
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
      return await native.getSystemDownloadDirectory();
    } catch {
      return "";
    }
  }

  async function loadAccounts() {
    accountsLoading = true;

    try {
      accounts = await commands.listAccounts();
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
      const account = await commands.saveAccount({
        id: editingAccountId,
        label: accountLabel,
        loginName: valueOrNull(accountLoginName),
        password: valueOrNull(accountPassword),
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
      await commands.setAccountEnabled(account.id, enabled);
      await loadAccounts();
      await loadProducts({ clampInvalidPage: true });
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
      const report = await commands.removeAccount(account.id);

      notifySuccess(`Removed ${report.label}`);

      if (editingAccountId === account.id) {
        resetAccountForm();
      }

      selectedAccountIds = selectedAccountIds.filter((accountId) => accountId !== account.id);

      await Promise.all([loadAccounts(), loadProducts({ resetPage: true })]);
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

  async function loadProducts(options: ProductLoadOptions = {}) {
    if (options.resetPage) {
      productPageIndex = 0;
    }

    productsLoading = true;

    try {
      let request = productListRequest();
      let page = await commands.listProducts(request);

      if (options.clampInvalidPage) {
        const pageIndex = clampedProductPageIndex(page.totalCount);

        if (pageIndex !== productPageIndex) {
          productPageIndex = pageIndex;

          if (page.totalCount > 0) {
            request = productListRequest();
            page = await commands.listProducts(request);
          }
        }
      }

      products = page.products;
      totalProducts = page.totalCount;
      await loadProductFilterFacets(request);
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      productsLoading = false;
    }
  }

  async function loadProductFilterFacets(request = productListRequest()) {
    productFilterFacets = await commands.listProductFilterFacets(request);
  }

  function productListRequest(): commands.ProductListRequest {
    return {
      search: valueOrNull(productSearch),
      accountIds: selectedAccountIds,
      typeGroups: selectedProductTypes,
      ageCategories: selectedAgeCategories,
      sourceGroups: selectedProductSources,
      makerNames: selectedMakerNames,
      customTagNames: selectedCustomTagNames,
      excludedCustomTagNames,
      sort: productSort,
      limit: PRODUCT_PAGE_SIZE,
      offset: productPageOffset(),
    };
  }

  function productPageOffset() {
    return productPageIndex * PRODUCT_PAGE_SIZE;
  }

  function productPageCount() {
    return Math.max(1, Math.ceil(totalProducts / PRODUCT_PAGE_SIZE));
  }

  function clampedProductPageIndex(totalCount: number) {
    return Math.min(
      productPageIndex,
      Math.max(1, Math.ceil(totalCount / PRODUCT_PAGE_SIZE)) - 1,
    );
  }

  function productTotalLabel() {
    return `${totalProducts} ${totalProducts === 1 ? "product" : "products"}`;
  }

  function productRangeLabel() {
    if (productsLoading) {
      return totalProducts > 0 ? `Loading ${productTotalLabel()}` : "Loading products";
    }

    if (totalProducts === 0 || products.length === 0) {
      return productTotalLabel();
    }

    const start = Math.min(productPageOffset() + 1, totalProducts);
    const end = Math.min(productPageOffset() + products.length, totalProducts);

    return `${start}-${end} of ${productTotalLabel()}`;
  }

  function productPageLabel() {
    return `Page ${Math.min(productPageIndex + 1, productPageCount())} of ${productPageCount()}`;
  }

  function hasPreviousProductPage() {
    return productPageIndex > 0;
  }

  function hasNextProductPage() {
    return productPageIndex < productPageCount() - 1;
  }

  async function reloadProducts() {
    await loadProducts({ clampInvalidPage: true });
  }

  async function goToPreviousProductPage() {
    if (productsLoading || !hasPreviousProductPage()) {
      return;
    }

    productPageIndex = Math.max(0, productPageIndex - 1);
    await loadProducts({ clampInvalidPage: true });
  }

  async function goToNextProductPage() {
    if (productsLoading || !hasNextProductPage()) {
      return;
    }

    productPageIndex = Math.min(productPageCount() - 1, productPageIndex + 1);
    await loadProducts({ clampInvalidPage: true });
  }

  function productBulkRequest(): commands.BulkWorkDownloadRequest {
    return {
      search: valueOrNull(productSearch),
      accountIds: selectedAccountIds,
      typeGroups: selectedProductTypes,
      ageCategories: selectedAgeCategories,
      sourceGroups: selectedProductSources,
      makerNames: selectedMakerNames,
      customTagNames: selectedCustomTagNames,
      excludedCustomTagNames,
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
    await loadProducts({ resetPage: true });
  }

  async function toggleProductTypeFilter(typeGroup: string) {
    selectedProductTypes = toggleFilterValue(selectedProductTypes, typeGroup);
    await loadProducts({ resetPage: true });
  }

  async function toggleAgeFilter(ageCategory: string) {
    selectedAgeCategories = toggleFilterValue(selectedAgeCategories, ageCategory);
    await loadProducts({ resetPage: true });
  }

  async function toggleProductSourceFilter(sourceGroup: string) {
    selectedProductSources = toggleFilterValue(selectedProductSources, sourceGroup);
    await loadProducts({ resetPage: true });
  }

  async function toggleMakerFilter(makerName: string) {
    selectedMakerNames = toggleFilterValue(selectedMakerNames, makerName);
    await loadProducts({ resetPage: true });
  }

  function customTagFilterState(tagName: string) {
    if (selectedCustomTagNames.includes(tagName)) {
      return "include";
    }

    if (excludedCustomTagNames.includes(tagName)) {
      return "exclude";
    }

    return "none";
  }

  async function cycleCustomTagFilter(tagName: string) {
    const state = customTagFilterState(tagName);

    if (state === "none") {
      selectedCustomTagNames = [...selectedCustomTagNames, tagName];
      excludedCustomTagNames = excludedCustomTagNames.filter((name) => name !== tagName);
    } else if (state === "include") {
      selectedCustomTagNames = selectedCustomTagNames.filter((name) => name !== tagName);
      excludedCustomTagNames = [...excludedCustomTagNames, tagName];
    } else {
      excludedCustomTagNames = excludedCustomTagNames.filter((name) => name !== tagName);
    }

    await loadProducts({ resetPage: true });
  }

  async function clearAccountFilters() {
    selectedAccountIds = [];
    await loadProducts({ resetPage: true });
  }

  async function clearTypeFilters() {
    selectedProductTypes = [];
    await loadProducts({ resetPage: true });
  }

  async function clearAgeFilters() {
    selectedAgeCategories = [];
    await loadProducts({ resetPage: true });
  }

  async function clearSourceFilters() {
    selectedProductSources = [];
    await loadProducts({ resetPage: true });
  }

  async function clearMakerFilters() {
    selectedMakerNames = [];
    await loadProducts({ resetPage: true });
  }

  async function clearCustomTagFilters() {
    selectedCustomTagNames = [];
    excludedCustomTagNames = [];
    await loadProducts({ resetPage: true });
  }

  async function setProductSort(sort: string) {
    productSort = sort;
    await loadProducts({ resetPage: true });
  }

  async function resetLibraryFilters() {
    productSearch = "";
    selectedAccountIds = [];
    selectedProductTypes = [];
    selectedAgeCategories = [];
    selectedProductSources = [];
    selectedMakerNames = [];
    selectedCustomTagNames = [];
    excludedCustomTagNames = [];
    productSort = "latestPurchaseDesc";
    await loadProducts({ resetPage: true });
  }

  async function loadJobs() {
    try {
      await jobController.load();
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function handleJobEvent(event: JobEvent) {
    if (event.kind === "accountSync" && isTerminalJob(event.snapshot)) {
      await Promise.all([
        loadAccounts(),
        loadProducts({ clampInvalidPage: true }),
        loadAuditEvents(),
      ]);
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

  function patchProductCustomTags(workId: string, customTags: ProductCustomTag[]) {
    products = products.map((product) =>
      product.workId === workId
        ? {
            ...product,
            customTags,
          }
        : product,
    );

    if (productDetail?.workId === workId) {
      productDetail = {
        ...productDetail,
        customTags,
      };
    }
  }

  function parseCustomTagInput(value: string) {
    const tags: string[] = [];
    const seen = new Set<string>();

    for (const part of value.split(/[,\n]/)) {
      const normalized = part.trim().replace(/\s+/g, " ");

      if (!normalized) {
        continue;
      }

      const key = normalized.toLowerCase();
      if (!seen.has(key)) {
        seen.add(key);
        tags.push(normalized);
      }
    }

    return tags;
  }

  async function saveProductCustomTags(workId: string, names: string[]) {
    const customTags = await commands.setProductCustomTags(workId, names);

    patchProductCustomTags(workId, customTags);
    await loadProductFilterFacets();

    return customTags;
  }

  async function addProductDetailCustomTags() {
    if (!productDetail) {
      return;
    }

    const additions = parseCustomTagInput(customTagInput);

    if (additions.length === 0) {
      return;
    }

    const nextNames = [...productDetail.customTags.map((tag) => tag.name), ...additions];

    try {
      const customTags = await saveProductCustomTags(productDetail.workId, nextNames);
      customTagInput = "";
      notifySuccess(
        `Saved ${customTags.length} custom tag${customTags.length === 1 ? "" : "s"} for ${productDetail.workId}`,
      );
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function removeProductDetailCustomTag(tagName: string) {
    if (!productDetail) {
      return;
    }

    const nextNames = productDetail.customTags
      .map((tag) => tag.name)
      .filter((name) => name !== tagName);

    try {
      await saveProductCustomTags(productDetail.workId, nextNames);
      notifySuccess(`Removed custom tag ${tagName} from ${productDetail.workId}`);
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function searchProducts() {
    await loadProducts({ resetPage: true });
  }

  async function copyWorkId(workId: string) {
    try {
      await navigator.clipboard.writeText(workId);
      notifySuccess(`Copied work ID ${workId}`);
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function copyCreditField(field: ProductCreditField, workId?: string) {
    if (field.missing) {
      return;
    }

    try {
      await navigator.clipboard.writeText(field.value);
      notifySuccess(workId ? `Copied ${field.label} for ${workId}` : `Copied ${field.label}`);
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function copyText(label: string, value: string | null | undefined, workId?: string) {
    const normalized = value?.trim();

    if (!normalized) {
      return;
    }

    try {
      await navigator.clipboard.writeText(normalized);
      notifySuccess(workId ? `Copied ${label} for ${workId}` : `Copied ${label}`);
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function openProductDetail(product: Product) {
    closeProductActionMenu();
    productDetailLoadingWorkId = product.workId;

    try {
      productDetail = await commands.getProductDetail(product.workId);
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      productDetailLoadingWorkId = null;
    }
  }

  function closeProductDetail() {
    productDetail = null;
    customTagInput = "";
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
      customTags: detail.customTags,
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
      const response = await commands.startAccountSync(
        account.id,
        editingAccountId === account.id ? valueOrNull(accountPassword) : null,
      );
      notifyInfo("Sync queued");
      jobController.setMessage(response.jobId, "Sync queued");
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
      const response = await commands.startWorkDownload({
        workId: product.workId,
        accountId: downloadAccountId(),
        password: null,
        unpackPolicy: options.unpackPolicy ?? "unpackWhenRecognized",
        replaceExisting: options.replaceExisting ?? false,
      });
      const queuedMessage = options.queuedMessage ?? "Download queued";
      notifyInfo(`${queuedMessage} for ${product.workId}`);
      jobController.setMessage(response.jobId, queuedMessage);
      await loadJobs();
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function startBulkWorkDownload() {
    bulkDownloadPlanning = true;

    try {
      const preview = await commands.previewBulkWorkDownload(productBulkRequest());

      if (preview.requestedCount === 0) {
        await showBulkDownloadDialog(preview, "notice");
        return;
      }

      const confirmed = await showBulkDownloadDialog(preview, "confirm");

      if (!confirmed) {
        return;
      }

      const response = await commands.startBulkWorkDownload(productBulkRequest());
      notifyInfo("Bulk download queued");
      jobController.setMessage(response.jobId, "Bulk download queued");
      await loadJobs();
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      bulkDownloadPlanning = false;
    }
  }

  async function openDownloadedProduct(product: Product) {
    if (!product.download.localPath) {
      return;
    }

    try {
      await commands.openWorkDownload(product.workId);
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
      const download = await commands.deleteWorkDownload(product.workId);
      notifySuccess(`Deleted download for ${product.workId}`);
      setProductDownload(product.workId, download);
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function markProductDownloaded(product: Product) {
    closeProductActionMenu();

    try {
      const fallbackRoot = libraryRoot.trim() || (await systemDownloadRoot());
      const selected = await native.chooseDirectory({
        canCreateDirectories: false,
        defaultPath: fallbackRoot || undefined,
        title: `Choose local folder for ${product.workId}`,
      });

      if (!selected) {
        return;
      }

      const download = await commands.markWorkDownloaded(product.workId, selected);
      notifySuccess(`Marked ${product.workId} as downloaded`);
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

  async function cancelJob(job: JobSnapshot) {
    try {
      await commands.cancelJob(job.id);
      const workId = jobWorkId(job) ?? jobOutputString(job, "workId");
      notifyInfo(workId ? `Cancellation requested for ${workId}` : "Cancellation requested");
      await loadJobs();
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function clearFinishedJobs() {
    try {
      await commands.clearFinishedJobs();
      await loadJobs();
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function loadAuditEvents() {
    auditLoading = true;

    try {
      auditEvents = await commands.listAuditEvents(80);
    } catch (err) {
      notifyError(errorMessage(err));
    } finally {
      auditLoading = false;
    }
  }

  async function loadAuditLogDir() {
    try {
      const result = await commands.getAuditLogDir();
      auditLogDir = result.path;
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function openAuditLogDir() {
    try {
      await commands.openAuditLogDir();
    } catch (err) {
      notifyError(errorMessage(err));
    }
  }

  async function openExternalUrl(url: string, label: string) {
    try {
      await native.openExternalUrl(url);
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

  function syncingAccountCount() {
    return accounts.filter((account) => activeAccountSyncJob(account.id)).length;
  }

  function accountSyncJobs(accountId: string) {
    return jobController.jobs.filter(
      (job) => job.kind === "accountSync" && jobAccountId(job) === accountId,
    );
  }

  function activeAccountSyncJob(accountId: string) {
    return [...accountSyncJobs(accountId)].reverse().find(isActiveJob) ?? null;
  }

  function latestAccountSyncJob(accountId: string) {
    return [...accountSyncJobs(accountId)].reverse()[0] ?? null;
  }

  function workDownloadJobs(workId: string) {
    return jobController.jobs.filter(
      (job) => job.kind === "workDownload" && jobWorkId(job) === workId,
    );
  }

  function activeWorkDownloadJob(workId: string) {
    return [...workDownloadJobs(workId)].reverse().find(isActiveJob) ?? null;
  }

  function activeBulkDownloadPlanningJob() {
    return (
      [...jobController.jobs]
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
    return [...jobController.jobs].reverse().slice(0, limit);
  }

  function visibleDownloadJobs(limit = 50) {
    return [...currentDownloadJobs()].reverse().slice(0, limit);
  }

  function currentDownloadJobs() {
    return jobController.jobs.filter((job) => isDownloadQueueJob(job) && isActiveJob(job));
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

  function hasSyncableEnabledAccount() {
    return accounts.some((account) => account.enabled && !activeAccountSyncJob(account.id));
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

    return (
      jobController.messages[job.id] ??
      shortDate(job.finishedAt ?? job.startedAt ?? job.createdAt)
    );
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

  const activeTwoFactorRequest = $derived(twoFactorQueue[0] ?? null);

  function queueTwoFactorRequest(request: TwoFactorRequest) {
    // A retry for the same job replaces its earlier request rather than stacking behind it.
    twoFactorQueue = [
      ...twoFactorQueue.filter((queued) => queued.jobId !== request.jobId),
      request,
    ];
  }

  function dropTwoFactorRequest(requestId: string) {
    twoFactorQueue = twoFactorQueue.filter((queued) => queued.requestId !== requestId);

    if (twoFactorQueue.length === 0) {
      twoFactorSubmitting = false;
    }
  }

  async function submitTwoFactorCode(code: string) {
    const request = activeTwoFactorRequest;

    if (!request || twoFactorSubmitting) {
      return;
    }

    twoFactorSubmitting = true;

    try {
      await commands.submitTwoFactorCode(request.requestId, code);
      dropTwoFactorRequest(request.requestId);
    } catch (error) {
      notifyError(errorMessage(error));
    } finally {
      twoFactorSubmitting = false;
    }
  }

  async function cancelTwoFactor() {
    const request = activeTwoFactorRequest;

    if (!request) {
      return;
    }

    dropTwoFactorRequest(request.requestId);

    try {
      await commands.cancelTwoFactor(request.requestId);
    } catch (error) {
      notifyError(errorMessage(error));
    }
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

</script>

<svelte:head>
  <title>dlsite-manager</title>
</svelte:head>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

<AppShell {activeView} onNavigate={(view) => (activeView = view)}>

    {#if activeView === "library"}
      <LibraryView
        {products}
        loading={productsLoading}
        bind:search={productSearch}
        filtersOpen={libraryFiltersOpen}
        {accounts}
        facets={productFilterFacets}
        sort={productSort}
        {selectedAccountIds}
        selectedSources={selectedProductSources}
        selectedAges={selectedAgeCategories}
        selectedTypes={selectedProductTypes}
        selectedMakers={selectedMakerNames}
        selectedCustomTags={selectedCustomTagNames}
        excludedCustomTags={excludedCustomTagNames}
        rangeLabel={productRangeLabel()}
        pageLabel={productPageLabel()}
        previousDisabled={productsLoading || !hasPreviousProductPage()}
        nextDisabled={productsLoading || !hasNextProductPage()}
        syncDisabled={accountsLoading || jobController.loading || !hasSyncableEnabledAccount()}
        bulkDisabled={bulkDownloadPlanning || productsLoading || jobController.loading || totalProducts === 0}
        bulkLabel={bulkDownloadButtonLabel()}
        detailLoadingWorkId={productDetailLoadingWorkId}
        openMenuWorkId={productActionMenu?.workId ?? null}
        getDownloadLabel={(product) => productDownloadActionLabel(product, activeWorkDownloadJob(product.workId))}
        getDownloadTitle={(product) => productDownloadActionTitle(product, activeWorkDownloadJob(product.workId))}
        getDownloadDisabled={(product) => productDownloadActionDisabled(product, activeWorkDownloadJob(product.workId))}
        onSearch={searchProducts}
        onReset={resetLibraryFilters}
        onToggleFilters={() => (libraryFiltersOpen = !libraryFiltersOpen)}
        onReload={reloadProducts}
        onSync={syncEnabledAccounts}
        onBulkDownload={startBulkWorkDownload}
        onSetSort={setProductSort}
        onClearAccounts={clearAccountFilters}
        onToggleAccount={toggleAccountFilter}
        onClearSources={clearSourceFilters}
        onToggleSource={toggleProductSourceFilter}
        onClearAges={clearAgeFilters}
        onToggleAge={toggleAgeFilter}
        onClearTypes={clearTypeFilters}
        onToggleType={toggleProductTypeFilter}
        onClearMakers={clearMakerFilters}
        onToggleMaker={toggleMakerFilter}
        onClearCustomTags={clearCustomTagFilters}
        onCycleCustomTag={cycleCustomTagFilter}
        onPreviousPage={goToPreviousProductPage}
        onNextPage={goToNextProductPage}
        onPreview={openProductImage}
        onOpenDetails={openProductDetail}
        onCopyWorkId={copyWorkId}
        onCopyCredit={copyCreditField}
        onShowTooltip={showChipTooltip}
        onMoveTooltip={moveChipTooltip}
        onHideTooltip={hideChipTooltip}
        onOpenDlsite={openDlsiteProductPage}
        onDownload={runProductDownloadAction}
        onToggleMenu={toggleProductActionMenu}
      />
    {:else if activeView === "downloads"}
      <DownloadsView
        jobs={visibleDownloadJobs()}
        loading={jobController.loading}
        queuedCount={queuedDownloadJobCount()}
        runningCount={runningDownloadJobCount()}
        getTitle={downloadQueueTitle}
        getDetail={jobDetail}
        onReload={loadJobs}
        onCancel={cancelJob}
      />
    {:else if activeView === "accounts"}
      <AccountsView
        {accounts}
        loading={accountsLoading}
        saving={accountSaving}
        jobsLoading={jobController.loading}
        {editingAccountId}
        bind:label={accountLabel}
        bind:loginName={accountLoginName}
        bind:password={accountPassword}
        syncingCount={syncingAccountCount()}
        syncAllDisabled={!hasSyncableEnabledAccount()}
        getActiveSyncJob={activeAccountSyncJob}
        getStatusLabel={accountStatusLabel}
        getStatusTone={accountStatusTone}
        onReload={loadAccounts}
        onSyncAll={syncEnabledAccounts}
        onToggleEnabled={setAccountEnabled}
        onEdit={editAccount}
        onSync={syncAccount}
        onCancelSync={cancelAccountSync}
        onRemove={removeAccount}
        onReset={resetAccountForm}
        onSave={saveAccount}
      />
    {:else if activeView === "activity"}
      <ActivityView
        jobs={visibleJobs()}
        jobLoading={jobController.loading}
        auditEvents={visibleAuditEvents()}
        {auditLoading}
        {auditLogDir}
        getJobTitle={jobAccountLabel}
        getJobDetail={jobDetail}
        onReloadJobs={loadJobs}
        onClearJobs={clearFinishedJobs}
        onCancelJob={cancelJob}
        onOpenAuditFolder={openAuditLogDir}
        onReloadAudit={loadAuditEvents}
      />
    {:else}
      <SettingsView
        bind:libraryRoot
        bind:downloadRoot
        loading={settingsLoading}
        saving={settingsSaving}
        {appInfo}
        {appInfoLoading}
        {updatePhase}
        {updateProgressMessage}
        onReload={loadSettings}
        onChooseDirectory={chooseSettingsDirectory}
        onUseDefaultDownloadRoot={useDefaultDownloadRoot}
        onSave={saveSettings}
        onOpenGitHub={() => openExternalUrl(GITHUB_URL, "GitHub")}
        onOpenDlsite={() => openExternalUrl(DLSITE_URL, "DLsite")}
        onCheckForUpdates={checkForUpdates}
      />
    {/if}

  <ConfirmationDialogView dialog={confirmationDialog} onClose={closeConfirmationDialog} />

  <TwoFactorDialog
    request={activeTwoFactorRequest}
    submitting={twoFactorSubmitting}
    onSubmit={(code) => void submitTwoFactorCode(code)}
    onCancel={() => void cancelTwoFactor()}
  />
  <BulkDownloadDialogView dialog={bulkDownloadDialog} onClose={closeBulkDownloadDialog} />
  <ProductDetailDialog
    detail={productDetail}
    bind:customTagInput
    onClose={closeProductDetail}
    onPreview={openProductImageFromDetail}
    onCopyText={copyText}
    onCopyWorkId={copyWorkId}
    onCopyCredit={copyCreditField}
    onOpenDlsite={openDlsiteProductPage}
    onAddTags={addProductDetailCustomTags}
    onRemoveTag={removeProductDetailCustomTag}
  />

  {#if productImagePreview}
    <ProductImagePreviewView preview={productImagePreview} onClose={closeProductImage} />
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
      <ProductActionMenuView
        workId={menuProduct.workId}
        downloadStatus={menuProduct.download.status}
        busy={!!menuDownloadJob}
        left={productActionMenu.left}
        top={productActionMenu.top}
        onClose={closeProductActionMenu}
        onDownloadArchives={() => downloadProductArchivesOnly(menuProduct)}
        onMarkDownloaded={() => markProductDownloaded(menuProduct)}
        onRedownload={() => redownloadProduct(menuProduct)}
        onDeleteDownload={() => deleteDownloadedProduct(menuProduct)}
      />
    {/if}
  {/if}

  <ToastStack {toasts} onDismiss={dismissToast} />
</AppShell>

<style>
  :global(html),
  :global(body) {
    height: 100%;
    overflow: hidden;
  }

  :global(body) {
    margin: 0;
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

</style>
