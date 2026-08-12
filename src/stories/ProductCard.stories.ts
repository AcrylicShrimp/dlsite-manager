import type { Meta, StoryObj } from "@storybook/sveltekit";
import type { Product } from "$lib/model/types";
import ProductCardStory from "./harnesses/ProductCardStory.svelte";

const thumbnail =
  "data:image/svg+xml," +
  encodeURIComponent(`
    <svg xmlns="http://www.w3.org/2000/svg" width="240" height="240" viewBox="0 0 240 240">
      <defs>
        <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
          <stop stop-color="#24332d"/>
          <stop offset="1" stop-color="#15191c"/>
        </linearGradient>
      </defs>
      <rect width="240" height="240" rx="18" fill="url(#g)"/>
      <circle cx="120" cy="96" r="48" fill="#95c29b" fill-opacity=".18"/>
      <path d="M82 132c10-30 66-30 76 0" fill="none" stroke="#95c29b" stroke-width="8" stroke-linecap="round"/>
      <text x="120" y="196" text-anchor="middle" fill="#edf2f6" font-family="system-ui" font-size="20" font-weight="700">RJ SAMPLE</text>
    </svg>
  `);

const emptyDownload = {
  status: "notDownloaded" as const,
  localPath: null,
  stagingPath: null,
  unpackPolicy: null,
  bytesReceived: 0,
  bytesTotal: null,
  errorCode: null,
  errorMessage: null,
  startedAt: null,
  completedAt: null,
  updatedAt: null,
};

const ownedProduct: Product = {
  workId: "RJ01553954",
  title: "A Long Evening at the Observatory — Binaural Voice Drama",
  makerName: "North Window Studio",
  workType: "SOU",
  ageCategory: "all",
  thumbnailUrl: thumbnail,
  publishedAt: "2026-05-04T00:00:00Z",
  updatedAt: "2026-05-18T00:00:00Z",
  earliestPurchasedAt: "2026-05-20T08:30:00Z",
  latestPurchasedAt: "2026-05-21T14:10:00Z",
  creditGroups: [
    { kind: "voiceActor", label: "Voice", names: ["Akari Example"] },
    { kind: "scenario", label: "Scenario", names: ["M. Hoshino"] },
    { kind: "illustration", label: "Illustration", names: ["Studio K"] },
    { kind: "music", label: "Music", names: ["Night Signal"] },
  ],
  customTags: [{ name: "Favorites" }, { name: "Sleep" }],
  download: emptyDownload,
  owners: [
    { accountId: "primary", label: "Primary", purchasedAt: "2026-05-20T08:30:00Z" },
    { accountId: "archive", label: "Archive", purchasedAt: "2026-05-21T14:10:00Z" },
  ],
};

const localProduct: Product = {
  ...ownedProduct,
  workId: "RJ01234567",
  title: "Imported local work with partially available metadata",
  makerName: null,
  workType: "COM",
  ageCategory: "r18",
  thumbnailUrl: null,
  creditGroups: [],
  customTags: [{ name: "Imported" }],
  download: {
    ...emptyDownload,
    status: "downloaded",
    localPath: "/Library/RJ01234567",
  },
  owners: [{ accountId: "__local__", label: "Local", purchasedAt: null }],
};

const meta = {
  title: "Library/Product Card",
  component: ProductCardStory,
  parameters: { layout: "fullscreen" },
  args: {
    product: ownedProduct,
    downloadLabel: "Download",
    downloadTitle: "Download this work",
    downloadDisabled: false,
    detailLoading: false,
    menuOpen: false,
  },
} satisfies Meta<typeof ProductCardStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const OwnedAudio: Story = {};

export const LocalOnly: Story = {
  args: {
    product: localProduct,
    downloadLabel: "Open",
    downloadTitle: "Open /Library/RJ01234567",
  },
};

export const Downloaded: Story = {
  args: {
    product: {
      ...ownedProduct,
      download: {
        ...emptyDownload,
        status: "downloaded",
        localPath: "/Library/RJ01553954",
        bytesReceived: 482_000_000,
        bytesTotal: 482_000_000,
      },
    },
    downloadLabel: "Open",
    downloadTitle: "Open /Library/RJ01553954",
  },
};

export const Downloading: Story = {
  args: {
    product: {
      ...ownedProduct,
      download: {
        ...emptyDownload,
        status: "downloading",
        bytesReceived: 214_000_000,
        bytesTotal: 482_000_000,
      },
    },
    downloadLabel: "Downloading",
    downloadTitle: "Downloading",
    downloadDisabled: true,
  },
};
