<script lang="ts">
  import { onMount } from "svelte";
  import type { View } from "$lib/model/types";
  import { views, labels, type Scenario } from "../fixtures/redesign";
  let view = $state<View>("activity");
  let scenario = $state<Scenario>("populated");
  let mode = $state("split");
  let variant = $state("draft");
  let size = $state("800");
  let actualSize = $state(false);
  let available = $state(1400);
  let stage: HTMLDivElement;
  let currentFrame = $state<HTMLIFrameElement>();
  let draftFrame = $state<HTMLIFrameElement>();
  const width = $derived(Number(size));
  const height = $derived(size === "1200" ? 800 : size === "390" ? 700 : 600);
  const slotWidth = $derived(
    mode === "split" ? (available - 18) / 2 : available,
  );
  const scale = $derived(
    actualSize ? 1 : Math.min(1, Math.max(240, slotWidth) / width),
  );
  const frameWidth = $derived(width * scale);
  const frameHeight = $derived(height * scale);
  function broadcast() {
    for (const frame of [currentFrame, draftFrame])
      frame?.contentWindow?.postMessage(
        { type: "dm-preview-state", view, scenario },
        window.location.origin,
      );
  }
  $effect(() => {
    view;
    scenario;
    broadcast();
  });
  onMount(() => {
    const observer = new ResizeObserver(
      (entries) => (available = entries[0].contentRect.width),
    );
    observer.observe(stage);
    function receive(e: MessageEvent) {
      if (
        e.origin !== window.location.origin ||
        ![currentFrame?.contentWindow, draftFrame?.contentWindow].includes(
          e.source as Window,
        )
      )
        return;
      if (e.data?.type === "dm-preview-ready") broadcast();
      if (
        e.data?.type === "dm-preview-navigation" &&
        views.includes(e.data.view)
      )
        view = e.data.view;
    }
    window.addEventListener("message", receive);
    return () => {
      observer.disconnect();
      window.removeEventListener("message", receive);
    };
  });
</script>

<main class="comparison">
  <header>
    <div>
      <span class="eyebrow">WORKSPACE STUDY / 01</span>
      <h1>실제 구현과 승인한 디자인.</h1>
      <p>동일한 데이터와 창 크기. 메뉴 이동은 양쪽에 함께 적용됩니다.</p>
    </div>
    <span class="preview-badge"
      >Storybook prototype · 실제 데이터 변경 없음</span
    >
  </header>
  <div class="controls">
    <div class="view-picker" aria-label="비교 화면">
      {#each views as item}<button
          class:active={view === item}
          aria-pressed={view === item}
          onclick={() => (view = item)}>{labels[item]}</button
        >{/each}
    </div>
    <div class="options">
      <label
        >데이터<select aria-label="데이터" bind:value={scenario}
          ><option value="populated">기본 · 오류 포함</option><option
            value="long">긴 목록</option
          ><option value="empty">빈 화면</option><option value="mfa"
            >MFA · 코드 입력</option
          ><option value="mfa-rejected">MFA · 코드 거절</option><option
            value="mfa-submitting">MFA · 확인 중</option
          ></select
        ></label
      ><label
        >창 크기<select aria-label="창 크기" bind:value={size}
          ><option value="800">800 × 600</option><option value="1200"
            >1200 × 800</option
          ><option value="390">390 × 700</option></select
        ></label
      >
      <div class="mode-picker" aria-label="비교 방식">
        <button
          class:active={mode === "split"}
          aria-pressed={mode === "split"}
          onclick={() => (mode = "split")}>나란히</button
        ><button
          class:active={mode === "toggle"}
          aria-pressed={mode === "toggle"}
          onclick={() => (mode = "toggle")}>같은 위치에서 전환</button
        >
      </div>
      <label class="scale-control"
        ><input type="checkbox" bind:checked={actualSize} />원본 크기</label
      >
    </div>
  </div>
  <div class="comparison-meta">
    <span
      >{width} × {height} px · {Math.round(scale * 100)}% 표시 · 두 화면의 배율
      동일</span
    >{#if mode === "toggle"}<div class="variant-picker">
        <button
          class:active={variant === "current"}
          aria-pressed={variant === "current"}
          onclick={() => (variant = "current")}>A · Cutover 구현</button
        ><button
          class:active={variant === "draft"}
          aria-pressed={variant === "draft"}
          onclick={() => (variant = "draft")}>B · 승인한 초안</button
        >
      </div>{/if}
  </div>
  <div class="stage" bind:this={stage}>
    <div
      class="frames"
      class:split={mode === "split"}
      style:width={mode === "split"
        ? `${frameWidth * 2 + 18}px`
        : `${frameWidth}px`}
    >
      {#each ["current", "draft"] as item}<section
          hidden={mode === "toggle" && variant !== item}
          style:width={`${frameWidth}px`}
        >
          <div class="frame-label">
            <strong
              >{item === "current"
                ? "A · Cutover 구현"
                : "B · 승인한 초안"}</strong
            ><span
              >{item === "current"
                ? "현재 작업 트리의 실제 컴포넌트"
                : "목록 중심 · 본문 스크롤 하나"}</span
            >
          </div>
          <div
            class="frame-viewport"
            style:width={`${frameWidth}px`}
            style:height={`${frameHeight}px`}
          >
            {#if item === "current"}<iframe
                bind:this={currentFrame}
                title="Cutover 구현"
                src="./iframe.html?id=redesign-workspace--current&viewMode=story"
                style:width={`${width}px`}
                style:height={`${height}px`}
                style:transform={`scale(${scale})`}
              ></iframe>
            {:else}<iframe
                bind:this={draftFrame}
                title="승인한 초안"
                src="./iframe.html?id=redesign-workspace--draft&viewMode=story"
                style:width={`${width}px`}
                style:height={`${height}px`}
                style:transform={`scale(${scale})`}
              ></iframe>{/if}
          </div>
        </section>{/each}
    </div>
  </div>
  <footer>
    <p>
      살펴볼 흐름: 목록 위·여백에서 스크롤 → 항목 상세 → 닫고 같은 위치로 복귀.
      Downloads의 실패 안내 → Activity → 관련 진단 내보내기.
    </p>
    <p>
      초안의 검색·필터·탭·대화상자·대기열·계정 편집은 메모리에서 동작합니다.
      Cutover 구현의 업무 동작은 알림으로 표시하며, 네이티브 호출은 모의
      응답입니다.
    </p>
    <a
      href="./iframe.html?id=redesign-workspace--draft&viewMode=story"
      target="_blank"
      rel="noreferrer">승인한 초안 전체 창으로 열기 ↗</a
    >
  </footer>
</main>

<style>
  .comparison {
    min-height: 100vh;
    background: #0d1011;
    color: #e9eee9;
    padding: 26px 28px;
    font-size: 12px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 22px;
  }
  .eyebrow {
    color: #a9c5a4;
    font-size: 9px;
    letter-spacing: 2px;
  }
  h1 {
    margin: 8px 0 6px;
    font-size: 23px;
    letter-spacing: -0.5px;
    font-weight: 600;
  }
  p {
    margin: 0;
    color: #9da7a0;
    font-size: 11px;
    line-height: 1.8;
  }
  .preview-badge {
    color: #9da7a0;
    border: 1px solid #303936;
    border-radius: 20px;
    padding: 6px 11px;
    font-size: 10px;
  }
  .controls {
    display: flex;
    flex-direction: column;
    gap: 14px;
    border-top: 1px solid #2a3330;
    border-bottom: 1px solid #2a3330;
    padding: 15px 0;
  }
  .view-picker,
  .options,
  .mode-picker,
  .variant-picker {
    display: flex;
    gap: 5px;
    align-items: center;
    flex-wrap: wrap;
  }
  .options {
    gap: 16px;
  }
  label {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #9da7a0;
    font-size: 11px;
  }
  button,
  select {
    border: 1px solid transparent;
    border-radius: 5px;
    padding: 7px 11px;
    background: #1a211e;
    color: #aab4ae;
    cursor: pointer;
    font: inherit;
    font-size: 11px;
  }
  .view-picker button {
    background: none;
    font-size: 12px;
  }
  button.active {
    color: #e1ecdd;
    background: #2c3c30;
    border-color: #465b4a;
  }
  select {
    border-color: #344039;
  }
  .mode-picker {
    margin-left: auto;
  }
  .scale-control {
    margin-left: 4px;
  }
  input {
    accent-color: #adc9a6;
  }
  button:focus-visible,
  select:focus-visible,
  a:focus-visible {
    outline: 2px solid #adc9a6;
    outline-offset: 3px;
  }
  .comparison-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    min-height: 56px;
    color: #8e9c93;
    font-size: 10px;
  }
  .stage {
    width: 100%;
    overflow-x: auto;
    padding-bottom: 6px;
  }
  .frames {
    display: flex;
    gap: 18px;
    margin: auto;
  }
  .frames:not(.split) {
    display: block;
  }
  .frame-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin: 0 0 10px;
    font-size: 11px;
  }
  .frame-label strong {
    font-weight: 550;
  }
  .frame-label span {
    font-size: 9px;
    color: #93a097;
  }
  .frame-viewport {
    position: relative;
    overflow: hidden;
    outline: 1px solid #344039;
    border-radius: 8px;
  }
  iframe {
    display: block;
    border: 0;
    transform-origin: top left;
    background: #171b1c;
  }
  section[hidden] {
    display: none;
  }
  footer {
    margin-top: 22px;
    display: grid;
    gap: 4px;
  }
  footer a {
    color: #b1cbaa;
    font-size: 11px;
    justify-self: start;
    margin-top: 8px;
    text-decoration: none;
  }
  @media (max-width: 700px) {
    .comparison {
      padding: 18px 14px;
    }
    header {
      align-items: start;
      flex-direction: column;
      gap: 10px;
    }
    .preview-badge {
      font-size: 9px;
    }
    .mode-picker {
      margin-left: 0;
    }
    .options {
      gap: 10px;
    }
    .frame-label span {
      display: none;
    }
    .comparison-meta {
      flex-wrap: wrap;
      padding: 10px 0;
    }
  }
</style>
