export interface DownloadProgress {
  product_id: string;
  progress: number;
}

export interface DownloadComplete {
  product_id: string;
  downloaded_path: string;
}
