<script lang="ts">
  interface Tab {
    id: string;
    label: string;
  }

  let { tabs, activeTab = tabs[0]?.id, onchange }: {
    tabs: Tab[];
    activeTab?: string;
    onchange?: (tabId: string) => void;
  } = $props();

  function selectTab(id: string) {
    activeTab = id;
    onchange?.(id);
  }
</script>

<div class="tabs" role="tablist">
  {#each tabs as tab (tab.id)}
    <button
      role="tab"
      aria-selected={activeTab === tab.id}
      class:active={activeTab === tab.id}
      onclick={() => selectTab(tab.id)}
    >
      {tab.label}
    </button>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid #21262d;
    margin-bottom: 1rem;
  }
  button {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: #8b949e;
    padding: 0.5rem 1rem;
    cursor: pointer;
    font-size: 0.875rem;
    transition: color 0.15s, border-color 0.15s;
  }
  button:hover { color: #e0e0e0; }
  button.active {
    color: #58a6ff;
    border-bottom-color: #58a6ff;
  }
</style>
