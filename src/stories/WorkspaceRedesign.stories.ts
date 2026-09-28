import type { Meta, StoryObj } from "@storybook/sveltekit";
import { scenarios } from "./fixtures/redesign";
import WorkspacePreview from "./redesign/WorkspacePreview.svelte";

const meta = {
  title: "Redesign/Workspace",
  component: WorkspacePreview,
  parameters: { layout: "fullscreen" },
  args: {
    variant: "draft",
    initialView: "activity",
    initialScenario: "populated",
  },
  argTypes: {
    variant: { control: "radio", options: ["current", "draft"] },
    initialView: {
      control: "select",
      options: ["library", "downloads", "activity", "accounts", "settings"],
    },
    initialScenario: {
      control: "select",
      options: [...scenarios],
    },
  },
} satisfies Meta<typeof WorkspacePreview>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Current: Story = { args: { variant: "current" } };
export const Draft: Story = {};

export const TwoFactorRequired: Story = {
  args: { initialView: "accounts", initialScenario: "mfa" },
};
export const TwoFactorRejected: Story = {
  args: { initialView: "accounts", initialScenario: "mfa-rejected" },
};
export const TwoFactorSubmitting: Story = {
  args: { initialView: "accounts", initialScenario: "mfa-submitting" },
};
