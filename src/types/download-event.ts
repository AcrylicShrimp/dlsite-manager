import type { ProductDownload } from "./product";

export interface DownloadProgress {
  product_id: string;
  progress: number;
}

export interface DownloadComplete {
  product_id: string;
  download: ProductDownload;
}
