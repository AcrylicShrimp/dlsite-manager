import type { Meta, StoryObj } from "@storybook/sveltekit";
import WorkspaceHeader from "$lib/components/WorkspaceHeader.svelte";

const meta = {
  title: "Shell/Workspace Header",
  component: WorkspaceHeader,
  tags: ["autodocs"],
  args: {
    eyebrow: "Collection",
    title: "Library",
  },
} satisfies Meta<typeof WorkspaceHeader>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Library: Story = {};

export const Activity: Story = {
  args: { eyebrow: "Jobs", title: "Activity" },
};
