<script lang="ts">
  import { onMount } from "svelte";
  import type { View } from "$lib/model/types";
  import { views, scenarios, type Scenario } from "../fixtures/redesign";
  import TwoFactorPreview from "./TwoFactorPreview.svelte";
  import CurrentWorkspace from "./CurrentWorkspace.svelte";
  import DraftWorkspace from "./DraftWorkspace.svelte";
  let {
    variant = "draft",
    initialView = "activity",
    initialScenario = "populated",
  }: {
    variant?: "current" | "draft";
    initialView?: View;
    initialScenario?: Scenario;
  } = $props();
  let view = $state<View>("activity");
  let scenario = $state<Scenario>("populated");
  $effect(() => {
    view = initialView;
    scenario = initialScenario;
  });
  function navigate(next: View) {
    view = next;
    window.parent.postMessage(
      { type: "dm-preview-navigation", view: next },
      window.location.origin,
    );
  }
  onMount(() => {
    function receive(e: MessageEvent) {
      if (
        e.origin !== window.location.origin ||
        e.source !== window.parent ||
        e.data?.type !== "dm-preview-state"
      )
        return;
      if (views.includes(e.data.view)) view = e.data.view;
      if (scenarios.includes(e.data.scenario)) scenario = e.data.scenario;
    }
    window.addEventListener("message", receive);
    window.parent.postMessage(
      { type: "dm-preview-ready" },
      window.location.origin,
    );
    return () => window.removeEventListener("message", receive);
  });
</script>

{#if variant === "current"}<CurrentWorkspace
    {view}
    {scenario}
    onNavigate={navigate}
  />{:else}<DraftWorkspace {view} {scenario} onNavigate={navigate} />{/if}

{#key scenario}
  {#if scenario === "mfa" || scenario === "mfa-rejected" || scenario === "mfa-submitting"}
    <TwoFactorPreview {variant} previewState={scenario} />
  {/if}
{/key}
