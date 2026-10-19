import type { Meta, StoryObj } from "@storybook/sveltekit";
import SettingsViewStory from "./harnesses/SettingsViewStory.svelte";
import { tick } from "svelte";

const meta = {
  title: "Settings/View",
  component: SettingsViewStory,
  parameters: { layout: "fullscreen" },
  args: { viewState: "ready" },
} satisfies Meta<typeof SettingsViewStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Ready: Story = {};
export const Loading: Story = { args: { viewState: "loading" } };
export const Saving: Story = { args: { viewState: "saving" } };
export const CheckingForUpdates: Story = { args: { viewState: "checking" } };
export const DownloadingUpdate: Story = { args: { viewState: "downloading" } };
export const InstallingUpdate: Story = { args: { viewState: "installing" } };

const showNames: Story["play"] = async ({ canvasElement }) => {
  const button = [...canvasElement.querySelectorAll("button")].find(
    (item) => item.textContent?.trim() === "Names",
  );
  button?.click();
  await tick();
};
export const NameLanguages: Story = { play: showNames };
export const JapaneseFirst: Story = {
  args: { languagePreset: "japanese" },
  play: showNames,
};
export const UnknownLanguages: Story = {
  args: { languagePreset: "unknown" },
  play: showNames,
};
export const SavedSubset: Story = {
  args: { languagePreset: "subset" },
  play: showNames,
};
export const DiscoveryFailure: Story = {
  args: { viewState: "discoveryFailure" },
  play: showNames,
};
export const LoadingLanguages: Story = {
  args: { viewState: "discovering" },
  play: showNames,
};
export const SavingLanguages: Story = {
  args: { viewState: "saving" },
  play: showNames,
};
export const LongLanguageLabels: Story = {
  args: { languagePreset: "long" },
  play: showNames,
};
