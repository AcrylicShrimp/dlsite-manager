import type { ProductCreditFieldDefinition, ProductTypeCodeDetail } from "./types";

export const GITHUB_URL = "https://github.com/AcrylicShrimp/dlsite-manager";
export const DLSITE_URL = "https://www.dlsite.com/";

export const TYPE_FILTERS = [
  ["audio", "Audio"],
  ["video", "Video"],
  ["game", "Game"],
  ["image", "Image / Comic"],
  ["other", "Other"],
] as const;

export const AGE_FILTERS = [
  ["all", "All Ages"],
  ["r15", "R-15"],
  ["r18", "R-18"],
] as const;

export const SORT_OPTIONS = [
  ["latestPurchaseDesc", "Latest Purchase"],
  ["publishedAtDesc", "Published"],
  ["titleAsc", "Title"],
] as const;

export const creditFieldDefinitions = [
  { key: "maker", label: "Maker" },
  { key: "voice", label: "CV" },
  { key: "illust", label: "Illust" },
  { key: "scenario", label: "Scenario" },
  { key: "creator", label: "Creator" },
  { key: "music", label: "Music" },
  { key: "other", label: "Other" },
] as const satisfies readonly ProductCreditFieldDefinition[];

export const productTypeCodeDetails: Record<string, ProductTypeCodeDetail> = {
  ACN: { label: "Action", tone: "game", group: "Game", description: "Action game" },
  ADL: { label: "Adult", tone: "image", group: "Image / Comic", description: "Adult work" },
  ADV: { label: "Adventure", tone: "game", group: "Game", description: "Adventure game" },
  AMT: {
    label: "Audio Material",
    tone: "audio",
    group: "Audio",
    description: "Audio material or sound assets",
  },
  COM: { label: "Comic", tone: "image", group: "Image / Comic", description: "Comic" },
  DNV: {
    label: "Digital Novel",
    tone: "image",
    group: "Image / Comic",
    description: "Digital novel or reading work",
  },
  DOH: {
    label: "Doujinshi",
    tone: "image",
    group: "Image / Comic",
    description: "Doujinshi or self-published book",
  },
  ET3: { label: "Other", tone: "other", group: "Other", description: "Miscellaneous product" },
  ETC: {
    label: "Other Game",
    tone: "game",
    group: "Game",
    description: "Game without a narrower type",
  },
  GAM: { label: "Game", tone: "game", group: "Game", description: "General game" },
  ICG: {
    label: "Illustration",
    tone: "image",
    group: "Image / Comic",
    description: "Illustration or CG collection",
  },
  IMT: {
    label: "Image Material",
    tone: "image",
    group: "Image / Comic",
    description: "Image material or visual assets",
  },
  KSV: {
    label: "Visual Novel",
    tone: "image",
    group: "Image / Comic",
    description: "Visual novel",
  },
  MNG: { label: "Manga", tone: "image", group: "Image / Comic", description: "Manga" },
  MOV: { label: "Anime", tone: "video", group: "Video", description: "Anime or video" },
  MUS: { label: "Music", tone: "audio", group: "Audio", description: "Music" },
  NRE: {
    label: "Novel",
    tone: "image",
    group: "Image / Comic",
    description: "Novel or text work",
  },
  PZL: { label: "Puzzle", tone: "game", group: "Game", description: "Puzzle game" },
  QIZ: { label: "Quiz", tone: "game", group: "Game", description: "Quiz game" },
  RPG: { label: "RPG", tone: "game", group: "Game", description: "Role-playing game" },
  SCM: {
    label: "Gekiga",
    tone: "image",
    group: "Image / Comic",
    description: "Gekiga or dramatic comic",
  },
  SLN: { label: "Simulation", tone: "game", group: "Game", description: "Simulation game" },
  SOF: { label: "Software", tone: "other", group: "Other", description: "Software product" },
  SOU: { label: "Voice", tone: "audio", group: "Audio", description: "Voice/audio work" },
  STG: { label: "Shooter", tone: "game", group: "Game", description: "Shooter game" },
  TBL: { label: "Tabletop", tone: "game", group: "Game", description: "Tabletop game" },
  TOL: { label: "Utility", tone: "other", group: "Other", description: "Utility tool or app" },
  TYP: { label: "Typing", tone: "game", group: "Game", description: "Typing game" },
  VCM: {
    label: "Voice Comic",
    tone: "voice-comic",
    group: "Image / Comic",
    description: "Comic with voice/audio presentation",
  },
};
