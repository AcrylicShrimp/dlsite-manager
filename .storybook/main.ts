import type { StorybookConfig } from "@storybook/sveltekit";

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.@(js|ts|svelte)"],
  addons: ["@storybook/addon-a11y"],
  framework: {
    name: "@storybook/sveltekit",
    options: {},
  },
  staticDirs: ["../static"],
};

export default config;
