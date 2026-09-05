import { describe, expect, it, vi } from "vitest";
import { LibraryQueryController, RequestGeneration } from "./library-query-controller";
import type { ProductListRequest } from "$lib/api/tauri";
import type { ProductListPage } from "$lib/model/types";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function query(overrides: Partial<ProductListRequest> = {}): ProductListRequest {
  return { search: null, accountIds: [], typeGroups: [], ageCategories: [], sourceGroups: [], makerNames: [], customTagNames: [], excludedCustomTagNames: [], sort: "latestPurchaseDesc", limit: 100, offset: 0, ...overrides };
}
const page = (totalCount: number): ProductListPage => ({ products: [], totalCount });
function fixture() {
  const port = { listProducts: vi.fn(), listProductFilterFacets: vi.fn().mockResolvedValue({ makers: [], customTags: [] }) };
  const view = { page: vi.fn(), facets: vi.fn(), loading: vi.fn(), error: vi.fn() };
  return { port, view, controller: new LibraryQueryController(port, view) };
}

describe("Library query lifecycle", () => {
  it("retains last known content when an authoritative terminal-event refresh fails", async () => {
    const { controller, port, view } = fixture();
    port.listProducts.mockResolvedValueOnce(page(1)).mockRejectedValueOnce(new Error("refresh unavailable"));
    await controller.load(query());
    controller.invalidate();
    await controller.load(query(), true);
    expect(view.page).toHaveBeenCalledExactlyOnceWith(page(1), 0);
    expect(view.error).toHaveBeenCalledTimes(1);
    expect(view.loading).toHaveBeenLastCalledWith(false);
  });

  it("clamps an emptied page after a mutation changes filter membership", async () => {
    const { controller, port, view } = fixture();
    port.listProducts.mockResolvedValueOnce(page(0));
    await controller.load(query({ customTagNames: ["removed"], offset: 200 }), true);
    expect(port.listProducts).toHaveBeenCalledTimes(1);
    expect(view.page).toHaveBeenCalledExactlyOnceWith(page(0), 0);
    expect(port.listProductFilterFacets).toHaveBeenCalledWith(query({ customTagNames: ["removed"], offset: 0 }));
  });

  for (const order of ["old-first", "new-first"]) {
    it(`rejects stale page/finally after filter change: ${order}`, async () => {
      const { controller, port, view } = fixture();
      const a = deferred<ProductListPage>(), b = deferred<ProductListPage>();
      port.listProducts.mockReturnValueOnce(a.promise).mockReturnValueOnce(b.promise);
      const first = controller.load(query({ search: "old" }));
      const second = controller.load(query({ search: "new" }));
      if (order === "old-first") {
        a.resolve(page(10)); await first;
        expect(view.page).not.toHaveBeenCalled();
        expect(view.loading).not.toHaveBeenCalledWith(false);
      }
      b.resolve(page(20)); await second;
      a.resolve(page(10)); await first;
      expect(view.page).toHaveBeenCalledExactlyOnceWith(page(20), 0);
      expect(view.loading).toHaveBeenLastCalledWith(false);
    });
  }

  it("invalidates old responses and errors after download/tag mutation", async () => {
    const { controller, port, view } = fixture();
    const a = deferred<ProductListPage>();
    port.listProducts.mockReturnValueOnce(a.promise).mockResolvedValueOnce(page(2));
    const first = controller.load(query({}));
    controller.invalidate();
    await controller.load(query({}));
    a.reject(new Error("obsolete query")); await first;
    expect(view.error).not.toHaveBeenCalled();
    expect(view.page).toHaveBeenCalledExactlyOnceWith(page(2), 0);
  });

  it("keeps clamp retry on its immutable request and rejects it after reset", async () => {
    const { controller, port, view } = fixture();
    const clamp = deferred<ProductListPage>();
    port.listProducts.mockResolvedValueOnce(page(2)).mockReturnValueOnce(clamp.promise).mockResolvedValueOnce(page(8));
    const input = query({ search: "old", makerNames: ["original"], limit: 100, offset: 200 });
    const first = controller.load(input, true);
    input.makerNames[0] = "mutated";
    await Promise.resolve();
    expect(port.listProducts).toHaveBeenNthCalledWith(2, { ...input, makerNames: ["original"], offset: 0 });
    await controller.load(query({ search: "reset", offset: 0 }));
    clamp.resolve(page(2)); await first;
    expect(view.page).toHaveBeenCalledExactlyOnceWith(page(8), 0);
  });

  it("ignores stale facets and clears facets on a current failure", async () => {
    const { controller, port, view } = fixture();
    const facets = deferred<{ makers: []; customTags: [] }>();
    port.listProducts.mockResolvedValue(page(2));
    port.listProductFilterFacets.mockReturnValueOnce(facets.promise).mockRejectedValueOnce(new Error("current facets"));
    const first = controller.load(query({ search: "old" }));
    await Promise.resolve();
    await controller.load(query({ search: "new" }));
    facets.resolve({ makers: [], customTags: [] }); await first;
    expect(view.facets).toHaveBeenCalledTimes(2); // each current page clears facets
    expect(view.error).toHaveBeenCalledTimes(1);
  });

  it("invalidates detail requests on close, mutation, or work switching", () => {
    const detail = new RequestGeneration();
    const old = detail.invalidate();
    const next = detail.invalidate();
    expect(detail.current(old)).toBe(false);
    expect(detail.current(next)).toBe(true);
    detail.invalidate();
    expect(detail.current(next)).toBe(false);
  });
});
