export interface Product {
  id: string;
  account_id?: number;
  ty: DLsiteProductType;
  age: DLsiteProductAge;
  title: string;
  thumbnail: string;
  group_id: string;
  group_name: string;
  registered_at?: string;
}

export interface ProductDownload {
  product_id: string;
  path: string;
}

export interface ProductQuery {
  query?: string;
  age?: DLsiteProductAge;
  ty?: DLsiteProductType;
  order_by_asc?: boolean;
}

export enum DLsiteProductType {
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
  VoiceComic = "VoiceComic",
  Unknown = "Unknown",
}

export enum DLsiteProductAge {
  All = "All",
  R15 = "R15",
  R18 = "R18",
  Unknown = "Unknown",
}

export enum DLsiteProductDownloadState {
  NotDownloaded = "NotDownloaded",
  Downloading = "Downloading",
  Downloaded = "Downloaded",
  DownloadingAndDownloaded = "DownloadingAndDownloaded",
}
