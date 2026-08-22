import type { Meta, StoryObj } from "@storybook/sveltekit";
import { fn } from "storybook/test";
import ToastStack from "$lib/components/ToastStack.svelte";

const meta = {
  title: "Feedback/Toast Stack",
  component: ToastStack,
  tags: ["autodocs"],
  parameters: { layout: "fullscreen" },
  args: {
    onDismiss: fn(),
    toasts: [
      { id: "sync", kind: "success", message: "Account sync completed." },
      { id: "info", kind: "info", message: "RJ01553954 was added to the download queue." },
      {
        id: "error",
        kind: "error",
        message: "RJ01105050 failed while checking the archive. Open Activity for details.",
      },
    ],
  },
} satisfies Meta<typeof ToastStack>;

export default meta;
type Story = StoryObj<typeof meta>;

export const MixedStates: Story = {};

export const LongMessage: Story = {
  args: {
    toasts: [
      {
        id: "long",
        kind: "error",
        message:
          "RJ01553954 could not be finalized because the configured library folder is no longer available. Check Settings and try again.",
      },
    ],
  },
};
