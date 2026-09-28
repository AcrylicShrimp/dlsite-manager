<script lang="ts">
  import { ageLabel, ageTone, productTypeFromCode } from "$lib/utils/products";
  let {
    workType,
    ageCategory,
  }: { workType: string | null; ageCategory: string | null } = $props();
  const type = $derived(productTypeFromCode(workType));
  const kinds: Record<string, string> = {
    audio:
      "dm:bg-kind-audio-bg dm:border-kind-audio-border dm:text-kind-audio-text",
    image:
      "dm:bg-kind-image-bg dm:border-kind-image-border dm:text-kind-image-text",
    video:
      "dm:bg-kind-video-bg dm:border-kind-video-border dm:text-kind-video-text",
    game: "dm:bg-kind-game-bg dm:border-kind-game-border dm:text-kind-game-text",
    "voice-comic":
      "dm:bg-kind-voice-comic-bg dm:border-kind-voice-comic-border dm:text-kind-voice-comic-text",
    other:
      "dm:bg-kind-other-bg dm:border-kind-other-border dm:text-kind-other-text",
  };
  const ages: Record<string, string> = {
    all: "dm:bg-age-all-bg dm:border-age-all-border dm:text-age-all-text",
    r15: "dm:bg-age-r15-bg dm:border-age-r15-border dm:text-age-r15-text",
    r18: "dm:bg-age-r18-bg dm:border-age-r18-border dm:text-age-r18-text",
    unknown: kinds.other,
  };
  const badge = "dm:px-1.5 dm:py-0.5 dm:rounded dm:border dm:border-solid";
</script>

<span
  class="work-badges dm:inline-flex dm:flex-wrap dm:gap-1 dm:text-[10px] dm:font-semibold dm:leading-normal dm:tracking-normal"
>
  <span
    class={`badge kind ${badge} ${kinds[type.tone] ?? kinds.other}`}
    data-tone={type.tone}
    title={type.tooltip}>{type.label}</span
  >
  <span
    class={`badge age ${badge} ${ages[ageTone(ageCategory)]}`}
    data-age={ageTone(ageCategory)}
    title="Age rating">{ageLabel(ageCategory) || "Age unknown"}</span
  >
</span>
