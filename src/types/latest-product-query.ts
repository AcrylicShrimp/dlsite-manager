import type { DLsiteProductDownloadState, ProductQuery } from "./product";

export interface LatestProductQuery {
  query: ProductQuery;
  download?: DLsiteProductDownloadState;
}
