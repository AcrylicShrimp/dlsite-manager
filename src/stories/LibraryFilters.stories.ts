import type { Meta, StoryObj } from "@storybook/sveltekit";
import LibraryFiltersStory from "./harnesses/LibraryFiltersStory.svelte";

const meta = {
  title: "Library/Filters",
  component: LibraryFiltersStory,
  parameters: { layout: "fullscreen" },
  args: { active: false },
} satisfies Meta<typeof LibraryFiltersStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const ActiveSelections: Story = {
  args: { active: true },
};
