import type { Meta, StoryObj } from "@storybook/sveltekit";
import ButtonStory from "./harnesses/ButtonStory.svelte";

const meta = {
  title: "Foundations/Button",
  component: ButtonStory,
  tags: ["autodocs"],
  args: {
    label: "Sync library",
    variant: "primary",
    size: "normal",
    disabled: false,
  },
  argTypes: {
    variant: { control: "select", options: ["primary", "secondary", "danger"] },
    size: { control: "select", options: ["normal", "small"] },
  },
} satisfies Meta<typeof ButtonStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};

export const Secondary: Story = {
  args: { label: "Reload", variant: "secondary" },
};

export const Danger: Story = {
  args: { label: "Delete local files", variant: "danger" },
};

export const Compact: Story = {
  args: { label: "Copy ID", variant: "secondary", size: "small" },
};

export const Disabled: Story = {
  args: { label: "Downloading", disabled: true },
};
