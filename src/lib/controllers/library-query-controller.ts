import type { ProductListRequest } from "$lib/api/tauri";
import type { ProductListPage, ProductFilterFacets } from "$lib/model/types";

export class RequestGeneration {
  private generation = 0;
  invalidate() { return ++this.generation; }
  current(generation: number) { return generation === this.generation; }
}

type QueryPort = {
  listProducts(request: ProductListRequest): Promise<ProductListPage>;
  listProductFilterFacets(request: ProductListRequest): Promise<ProductFilterFacets>;
};
type QueryView = {
  loading(value: boolean): void;
  page(value: ProductListPage, pageIndex: number): void;
  facets(value: ProductFilterFacets): void;
  error(error: unknown): void;
};

export class LibraryQueryController extends RequestGeneration {
  constructor(private port: QueryPort, private view: QueryView) { super(); }

  async load(input: ProductListRequest, clamp = false) {
    const generation = this.invalidate();
    // Route inputs can be Svelte proxies. Copy all filter arrays before the first await.
    let request: ProductListRequest = JSON.parse(JSON.stringify(input));
    this.view.loading(true);
    try {
      let page = await this.port.listProducts(request);
      if (!this.current(generation)) return;
      const limit = request.limit ?? 100;
      let pageIndex = Math.floor((request.offset ?? 0) / limit);
      if (clamp) {
        const clamped = Math.min(pageIndex, Math.max(0, Math.ceil(page.totalCount / limit) - 1));
        if (clamped !== pageIndex) {
          pageIndex = clamped;
          request = { ...request, offset: pageIndex * limit };
          if (page.totalCount > 0) page = await this.port.listProducts(request);
          if (!this.current(generation)) return;
        }
      }
      this.view.page(page, pageIndex);
      // Clear old facets if the current facet refresh fails.
      this.view.facets({ makers: [], customTags: [] });
      const facets = await this.port.listProductFilterFacets(request);
      if (this.current(generation)) this.view.facets(facets);
    } catch (error) {
      if (this.current(generation)) this.view.error(error);
    } finally {
      if (this.current(generation)) this.view.loading(false);
    }
  }
}
