import type { Account } from "./account";

export interface Product {
  id: number;
  account: Account;
  product: DLsiteProduct;
}

export interface DLsiteProduct {
  id: string;
  ty: DLsiteProductType;
  age: DLsiteProductAge;
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

export type DLsiteProductType =
  | "Unknown"
  | "Adult"
  | "Doujinsji"
  | "Software"
  | "Game"
  | "Action"
  | "Adventure"
  | "AudioMaterial"
  | "Comic"
  | "DigitalNovel"
  | "Other"
  | "OtherGame"
  | "Illust"
  | "ImageMaterial"
  | "Manga"
  | "Anime"
  | "Music"
  | "Novel"
  | "Puzzle"
  | "Quiz"
  | "RolePlaying"
  | "Gekiga"
  | "Simulation"
  | "Voice"
  | "Shooter"
  | "Tabletop"
  | "Utility"
  | "Typing"
  | "SexualNovel";
export type DLsiteProductAge = "All" | "R15" | "R18";
