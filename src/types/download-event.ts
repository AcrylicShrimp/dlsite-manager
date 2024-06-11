export interface DownloadProgress {
  product_id: string;
  progress: number;
  decompressing: boolean;
}

export interface DownloadComplete {
  product_id: string;
  downloaded_path: string;
}
