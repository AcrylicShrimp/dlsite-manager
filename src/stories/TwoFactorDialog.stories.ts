import type { Meta, StoryObj } from "@storybook/sveltekit";
import { fn } from "storybook/test";
import TwoFactorDialog from "$lib/components/TwoFactorDialog.svelte";

const meta = {
  title: "Feedback/Two-Factor Dialog",
  component: TwoFactorDialog,
  tags: ["autodocs"],
  parameters: { layout: "fullscreen" },
  args: {
    onSubmit: fn(),
    onCancel: fn(),
    submitting: false,
    request: {
      requestId: "two-factor-1",
      accountId: "account-a",
      accountLabel: "Main account",
      attempt: 1,
      previousCodeRejected: false,
      jobId: "job-1",
    },
  },
} satisfies Meta<typeof TwoFactorDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

/** The first prompt a job raises: no attempt hint, no rejection notice. */
export const InitialPrompt: Story = {};

/** DLsite sent the job back to the challenge, so the code was wrong or expired. */
export const RejectedCode: Story = {
  args: {
    request: {
      requestId: "two-factor-2",
      accountId: "account-a",
      accountLabel: "Main account",
      attempt: 2,
      previousCodeRejected: true,
      jobId: "job-1",
    },
  },
};

/** The code is in flight: the field and both actions are held until the job answers. */
export const Submitting: Story = {
  args: { submitting: true },
};

/** A long label at 390px, where the actions become a two-column grid. */
export const NarrowViewport: Story = {
  args: {
    request: {
      requestId: "two-factor-3",
      accountId: "account-b",
      accountLabel: "サークル専用アカウント (two-factor enabled)",
      attempt: 1,
      previousCodeRejected: false,
      jobId: "job-2",
    },
  },
  parameters: {
    viewport: {
      options: {
        narrow: { name: "Narrow", styles: { width: "390px", height: "844px" } },
      },
    },
  },
  globals: {
    viewport: { value: "narrow", isRotated: false },
  },
};
