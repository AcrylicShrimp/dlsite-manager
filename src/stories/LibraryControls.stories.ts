import type { Meta, StoryObj } from "@storybook/sveltekit";
import LibraryControlsStory from "./harnesses/LibraryControlsStory.svelte";

const meta = {
  title: "Library/Controls",
  component: LibraryControlsStory,
  parameters: { layout: "fullscreen" },
  args: {
    initialSearch: "",
    filtersOpen: false,
    searchDisabled: false,
    reloadDisabled: false,
    syncDisabled: false,
    bulkDisabled: false,
    bulkLabel: "Download 24 Results",
  },
} satisfies Meta<typeof LibraryControlsStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Ready: Story = {};

export const SearchAndFilters: Story = {
  args: {
    initialSearch: "voice drama",
    filtersOpen: true,
    bulkLabel: "Download 8 Results",
  },
};

export const Busy: Story = {
  args: {
    initialSearch: "RJ01553954",
    searchDisabled: true,
    reloadDisabled: true,
    syncDisabled: true,
    bulkDisabled: true,
    bulkLabel: "Planning…",
  },
};
