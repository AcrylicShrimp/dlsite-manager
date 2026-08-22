import type { Meta, StoryObj } from "@storybook/sveltekit";
import LibraryViewStory from "./harnesses/LibraryViewStory.svelte";

const meta = {
  title: "Library/View",
  component: LibraryViewStory,
  parameters: { layout: "fullscreen" },
  args: { viewState: "populated", initialFiltersOpen: false },
} satisfies Meta<typeof LibraryViewStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Populated: Story = {};
export const WithFilters: Story = { args: { initialFiltersOpen: true } };
export const Loading: Story = { args: { viewState: "loading" } };
export const Empty: Story = { args: { viewState: "empty" } };
