import type { Account } from "./account";

export interface Product {
  id: number;
  account: Account;
  product: DLsiteProduct;
}

export interface DLsiteProduct {
  id: string;
  ty: string;
  age: string;
  title: DLsiteProductLocalizedString;
  group: DLsiteProductGroup;
  icon: DLsiteProductIcon;
  registered_at?: string;
  upgraded_at?: string;
  purchased_at: string;
}

export interface DLsiteProductLocalizedString {
  japanese?: string;
  english?: string;
  korean?: string;
  taiwanese?: string;
  chinese?: string;
}

export interface DLsiteProductGroup {
  id: string;
  name: DLsiteProductLocalizedString;
}

export interface DLsiteProductIcon {
  main: string;
  small: string;
}
