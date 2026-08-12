import type { Meta, StoryObj } from "@storybook/sveltekit";
import AccountSourceRowStory from "./harnesses/AccountSourceRowStory.svelte";

const meta = {
  title: "Accounts/Source Row",
  component: AccountSourceRowStory,
  parameters: { layout: "fullscreen" },
  args: { rowState: "selected" },
} satisfies Meta<typeof AccountSourceRowStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Selected: Story = {};
export const Syncing: Story = { args: { rowState: "syncing" } };
export const Disabled: Story = { args: { rowState: "disabled" } };
