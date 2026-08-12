<script lang="ts">
  import type { View } from "$lib/model/types";

  type NavItem = {
    label: string;
    view: View;
    path: string;
  };

  const mainItems: NavItem[] = [
    {
      label: "Library",
      view: "library",
      path: "M4 6.5A2.5 2.5 0 0 1 6.5 4H20v15H6.5A2.5 2.5 0 0 1 4 16.5v-10Zm0 10A2.5 2.5 0 0 1 6.5 14H20M8 8h8",
    },
    {
      label: "Downloads",
      view: "downloads",
      path: "M12 3v12m0 0 5-5m-5 5-5-5M5 20h14",
    },
    {
      label: "Accounts",
      view: "accounts",
      path: "M16 19v-1.5A3.5 3.5 0 0 0 12.5 14h-5A3.5 3.5 0 0 0 4 17.5V19m12-9a3 3 0 1 0 0-6m4 15v-1.5a3.5 3.5 0 0 0-2.1-3.2M10 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z",
    },
    {
      label: "Settings",
      view: "settings",
      path: "M12 15.25A3.25 3.25 0 1 0 12 8.75a3.25 3.25 0 0 0 0 6.5Zm7-3.25 2-1-2-3.46-2.22.14a7.8 7.8 0 0 0-1.56-.9L14.23 4h-4.46l-.99 2.78c-.55.25-1.07.55-1.56.9L5 7.54 3 11l2 1-2 1 2 3.46 2.22-.14c.49.35 1.01.65 1.56.9L9.77 20h4.46l.99-2.78c.55-.25 1.07-.55 1.56-.9l2.22.14L21 13l-2-1Z",
    },
  ];

  const utilityItems: NavItem[] = [
    {
      label: "Activity",
      view: "activity",
      path: "M4 19V5m0 14h16M8 16v-5m4 5V7m4 9v-3",
    },
  ];

  let {
    activeView,
    onNavigate,
  }: {
    activeView: View;
    onNavigate: (view: View) => void;
  } = $props();
</script>

<aside class="sidebar" aria-label="Primary">
  <div class="brand">
    <span class="brand-mark" aria-hidden="true">dm</span>
    <span>dlsite-manager</span>
  </div>

  <nav class="main-nav" aria-label="Main">
    {#each mainItems as item (item.view)}
      <button
        class:active={activeView === item.view}
        type="button"
        aria-current={activeView === item.view ? "page" : undefined}
        onclick={() => onNavigate(item.view)}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d={item.path} /></svg>
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>

  <nav class="utility-nav" aria-label="Utility">
    {#each utilityItems as item (item.view)}
      <button
        class:active={activeView === item.view}
        type="button"
        aria-current={activeView === item.view ? "page" : undefined}
        onclick={() => onNavigate(item.view)}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d={item.path} /></svg>
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 24px;
    min-width: 0;
    padding: 22px 16px 18px;
    border-right: 1px solid var(--border);
    color: var(--text);
    background: #111417;
    overflow: auto;
  }

  .brand {
    display: flex;
    gap: 10px;
    align-items: center;
    min-height: 34px;
    padding: 0 6px;
    color: var(--text-strong);
    font-size: 15px;
    font-weight: 720;
  }

  .brand-mark {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 1px solid rgb(149 194 155 / 38%);
    border-radius: 8px;
    color: var(--accent);
    background: var(--accent-muted);
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .utility-nav {
    margin-top: auto;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  nav button {
    display: flex;
    gap: 10px;
    align-items: center;
    width: 100%;
    min-width: 0;
    height: 40px;
    padding: 0 11px;
    border: 1px solid transparent;
    border-radius: 7px;
    color: var(--muted);
    background: transparent;
    font: inherit;
    font-size: 14px;
    font-weight: 620;
    text-align: left;
    cursor: pointer;
    transition:
      border-color 120ms ease,
      color 120ms ease,
      background-color 120ms ease;
  }

  nav button:hover {
    color: var(--text);
    background: rgb(255 255 255 / 3.5%);
  }

  nav button.active {
    border-color: rgb(149 194 155 / 20%);
    color: var(--text-strong);
    background: var(--accent-muted);
  }

  nav button:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  svg {
    flex: 0 0 auto;
    width: 17px;
    height: 17px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 1.7;
  }

  @media (max-width: 720px) {
    .sidebar {
      gap: 10px;
      padding: 11px 12px 10px;
      border-right: 0;
      border-bottom: 1px solid var(--border);
      overflow: visible;
    }

    .brand {
      min-height: 30px;
      padding: 0 4px;
    }

    .brand-mark {
      width: 27px;
      height: 27px;
      border-radius: 7px;
    }

    nav {
      flex-direction: row;
      gap: 4px;
    }

    .main-nav {
      overflow-x: auto;
      scrollbar-width: none;
    }

    .main-nav::-webkit-scrollbar {
      display: none;
    }

    .utility-nav {
      margin-top: 0;
      padding-top: 0;
      border-top: 0;
    }

    nav button {
      flex: 1 0 auto;
      justify-content: center;
      width: auto;
      height: 36px;
      padding: 0 10px;
      font-size: 13px;
    }

    nav button svg {
      width: 15px;
      height: 15px;
    }
  }
</style>
