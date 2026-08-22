import type { Meta, StoryObj } from "@storybook/sveltekit";
import AccountsViewStory from "./harnesses/AccountsViewStory.svelte";

const meta = {
  title: "Accounts/View",
  component: AccountsViewStory,
  parameters: { layout: "fullscreen" },
  args: { viewState: "populated" },
} satisfies Meta<typeof AccountsViewStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Populated: Story = {};
export const Loading: Story = { args: { viewState: "loading" } };
export const Empty: Story = { args: { viewState: "empty" } };
