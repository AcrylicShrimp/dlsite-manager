<script lang="ts">
  import "./draft.css";
  import Button from "./DraftButton.svelte";
  import Choices from "./DraftChoiceGroup.svelte";
  import Row from "./DraftListRow.svelte";
  import Tag from "./DraftTagChip.svelte";
  import Badges from "./DraftWorkBadges.svelte";
  import Icon from "./DraftIcon.svelte";
  import Disclosure from "./DraftDisclosure.svelte";
  import SearchField from "./DraftSearchField.svelte";
  let query = $state("");
  let filter = $state("all");
  let tab = $state("history");
  let message = $state("");
</script>

<div
  class="dm:box-border dm:min-h-screen dm:bg-draft-background dm:p-6 dm:text-draft-ink dm:text-base"
>
  <div class="dm:mx-auto dm:flex dm:max-w-3xl dm:flex-col dm:gap-6">
    <h1 class="dm:m-0 dm:text-xl">Shared controls</h1>
    <div class="dm:flex dm:flex-wrap dm:gap-2">
      <Button variant="primary" onclick={() => (message = "Primary clicked")}
        >Primary</Button
      >
      <Button onclick={() => (message = "Secondary clicked")}>Secondary</Button>
      <Button variant="text" onclick={() => (message = "Text action clicked")}
        >Text action <Icon name="arrow" /></Button
      >
      <Button
        variant="icon"
        aria-label="Refresh"
        title="Refresh"
        onclick={() => (message = "Refresh clicked")}
        ><Icon name="refresh" /></Button
      >
      <Button disabled>Disabled</Button>
    </div>
    <Choices
      variant="tabs"
      label="Example tabs"
      value={tab}
      onchange={(value) => (tab = value)}
      options={[
        { value: "history", label: "Work history" },
        { value: "logs", label: "Application logs" },
      ]}
    />
    <Choices
      label="Example filter"
      value={filter}
      onchange={(value) => (filter = value)}
      options={[
        { value: "all", label: "All" },
        { value: "error", label: "Errors" },
      ]}
    />
    <div>
      <Row onclick={() => (message = "First row clicked")}
        >{#snippet leading()}<Icon name="refresh" />{/snippet}<strong
          >Sync Secondary purchases</strong
        ><span class="dm:text-xs dm:text-draft-dim">42 of 120 works loaded</span
        >{#snippet trailing()}<span>Syncing</span>{/snippet}</Row
      >
      <Row onclick={() => (message = "Long row clicked")}
        >{#snippet leading()}<Icon name="info" />{/snippet}<strong
          >A longer title that wraps naturally in a narrow window without losing
          its inner padding</strong
        ><span class="dm:text-xs dm:text-draft-dim"
          >The entire padded row is clickable.</span
        >{#snippet trailing()}<span class="dm:text-draft-error">Failed</span
          >{/snippet}</Row
      >
    </div>
    <div class="dm:flex dm:flex-wrap dm:gap-2">
      <Tag
        name="Long-form listening"
        oncopy={() => (message = "Tag copied")}
        onremove={() => (message = "Tag removed")}
      /><Tag
        name="긴 한글 태그"
        oncopy={() => (message = "Tag copied")}
        onremove={() => (message = "Tag removed")}
      />
    </div>
    <SearchField
      bind:value={query}
      label="Search controls"
      placeholder="Search…"
      clearable
    />
    <Disclosure title="Metadata section">
      <p class="dm:m-0 dm:text-sm dm:text-draft-dim">Section content</p>
    </Disclosure>
    <div class="dm:flex dm:flex-wrap dm:gap-3">
      <Badges workType="SOU" ageCategory="all" /><Badges
        workType="COM"
        ageCategory="r15"
      /><Badges workType="RPG" ageCategory="r18" />
    </div>
    <p role="status" class="dm:m-0 dm:text-sm dm:text-draft-dim">{message}</p>
  </div>
</div>
