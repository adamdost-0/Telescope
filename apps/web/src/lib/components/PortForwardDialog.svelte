<script lang="ts">
  let {
    open = false,
    podName = '',
    namespace = '',
    availablePorts = [] as number[],
    onforward,
    oncancel,
  }: {
    open: boolean;
    podName?: string;
    namespace?: string;
    availablePorts?: number[];
    onforward?: (localPort: number, remotePort: number) => void;
    oncancel?: () => void;
  } = $props();

  let localPort = $state(8080);
  let remotePort = $state(0);
  let customRemotePort = $state(80);
  let useCustom = $state(false);

  $effect(() => {
    if (availablePorts.length > 0) {
      remotePort = availablePorts[0];
      useCustom = false;
    } else {
      useCustom = true;
    }
  });

  let effectiveRemotePort = $derived(useCustom ? customRemotePort : remotePort);
</script>

{#if open}
  <div class="overlay" role="presentation" onclick={oncancel}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <h3>Port Forward — {podName}</h3>
      <p class="subtitle">{namespace}</p>

      <div class="port-row">
        <label>
          Local Port
          <input type="number" bind:value={localPort} min="0" max="65535" />
        </label>
        <span class="arrow">→</span>
        <label>
          Remote Port
          {#if availablePorts.length > 0}
            <select
              bind:value={remotePort}
              onchange={() => {
                useCustom = false;
              }}
            >
              {#each availablePorts as p}
                <option value={p}>{p}</option>
              {/each}
            </select>
            <button class="link-btn" onclick={() => (useCustom = !useCustom)}>
              {useCustom ? 'Use listed' : 'Custom'}
            </button>
          {/if}
          {#if useCustom || availablePorts.length === 0}
            <input type="number" bind:value={customRemotePort} min="1" max="65535" />
          {/if}
        </label>
      </div>

      <p class="hint">
        localhost:{localPort || '?'} → {podName}:{effectiveRemotePort}
      </p>

      <div class="actions">
        <button class="cancel" onclick={oncancel}>Cancel</button>
        <button
          class="confirm"
          onclick={() => onforward?.(localPort, effectiveRemotePort)}
        >
          Start Forward
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .dialog {
    background: #1c1c2e;
    border: 1px solid #2a2a3e;
    border-radius: 8px;
    padding: 1.5rem;
    min-width: 360px;
    max-width: 480px;
    color: #e0e0e0;
  }

  h3 {
    margin: 0 0 0.25rem;
    font-size: 1.1rem;
    color: #4fc3f7;
  }

  .subtitle {
    margin: 0 0 1rem;
    font-size: 0.8rem;
    color: #6e7681;
  }

  .port-row {
    display: flex;
    align-items: flex-end;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }

  .arrow {
    font-size: 1.5rem;
    color: #58a6ff;
    padding-bottom: 0.35rem;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.8rem;
    color: #8b949e;
  }

  input[type='number'],
  select {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 4px;
    color: #e0e0e0;
    padding: 0.4rem 0.5rem;
    font-size: 0.9rem;
    font-family: monospace;
    width: 100px;
  }

  input[type='number']:focus,
  select:focus {
    outline: none;
    border-color: #58a6ff;
  }

  .link-btn {
    background: none;
    border: none;
    color: #58a6ff;
    font-size: 0.75rem;
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
  }

  .hint {
    font-family: monospace;
    font-size: 0.8rem;
    color: #8b949e;
    background: #161b22;
    padding: 0.4rem 0.6rem;
    border-radius: 4px;
    margin-bottom: 1rem;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  button {
    padding: 0.4rem 1rem;
    border-radius: 6px;
    font-size: 0.85rem;
    cursor: pointer;
    border: 1px solid transparent;
  }

  .cancel {
    background: transparent;
    color: #8b949e;
    border-color: #30363d;
  }
  .cancel:hover {
    background: #21262d;
  }

  .confirm {
    background: #238636;
    color: #fff;
    border-color: #2ea043;
  }
  .confirm:hover {
    background: #2ea043;
  }
</style>
