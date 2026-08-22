import type { Meta, StoryObj } from "@storybook/sveltekit";
import ActivityViewStory from "./harnesses/ActivityViewStory.svelte";

const meta = {
  title: "Activity/View",
  component: ActivityViewStory,
  parameters: { layout: "fullscreen" },
  args: { viewState: "populated", withAuditDirectory: true },
} satisfies Meta<typeof ActivityViewStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Populated: Story = {};
export const Loading: Story = { args: { viewState: "loading" } };
export const Empty: Story = { args: { viewState: "empty" } };
export const MissingAuditDirectory: Story = { args: { withAuditDirectory: false } };
