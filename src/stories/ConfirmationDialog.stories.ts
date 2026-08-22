import type { Meta, StoryObj } from "@storybook/sveltekit";
import { fn } from "storybook/test";
import ConfirmationDialog from "$lib/components/ConfirmationDialog.svelte";

const meta = {
  title: "Feedback/Confirmation Dialog",
  component: ConfirmationDialog,
  tags: ["autodocs"],
  parameters: { layout: "fullscreen" },
  args: {
    onClose: fn(),
    dialog: {
      eyebrow: "Confirm action",
      title: "Download this work?",
      message: "The archive will be downloaded to staging and finalized in your library.",
      confirmLabel: "Download",
      cancelLabel: "Cancel",
      tone: "default",
    },
  },
} satisfies Meta<typeof ConfirmationDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Danger: Story = {
  args: {
    dialog: {
      eyebrow: "Delete downloaded files",
      title: "Remove RJ01553954 from this device?",
      message:
        "This removes the managed library folder and any resumable staging files. Product metadata stays in the Library.",
      confirmLabel: "Delete files",
      cancelLabel: "Keep files",
      tone: "danger",
    },
  },
};
