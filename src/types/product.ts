import type { Account } from "./account";

export interface Product {
  id: number;
  account: Account;
  product: DLsiteProduct;
  download?: ProductDownload;
}

export interface ProductDownload {
  id: number;
  path: string;
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

export enum DLsiteProductType {
  Unknown = "Unknown",
  Adult = "Adult",
  Doujinsji = "Doujinsji",
  Software = "Software",
  Game = "Game",
  Action = "Action",
  Adventure = "Adventure",
  AudioMaterial = "AudioMaterial",
  Comic = "Comic",
  DigitalNovel = "DigitalNovel",
  Other = "Other",
  OtherGame = "OtherGame",
  Illust = "Illust",
  ImageMaterial = "ImageMaterial",
  Manga = "Manga",
  Anime = "Anime",
  Music = "Music",
  Novel = "Novel",
  Puzzle = "Puzzle",
  Quiz = "Quiz",
  RolePlaying = "RolePlaying",
  Gekiga = "Gekiga",
  Simulation = "Simulation",
  Voice = "Voice",
  Shooter = "Shooter",
  Tabletop = "Tabletop",
  Utility = "Utility",
  Typing = "Typing",
  SexualNovel = "SexualNovel",
}

export enum DLsiteProductAge {
  All = "All",
  R15 = "R15",
  R18 = "R18",
}
