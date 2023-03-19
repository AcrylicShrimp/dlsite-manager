import type { DLsiteProductLocalizedString } from "./product";

export interface Setting {
  download_root_dir: string;
}

export interface DisplayLanguageSetting {
  languages: (keyof DLsiteProductLocalizedString)[];
}
