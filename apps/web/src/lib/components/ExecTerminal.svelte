<script lang="ts">
  import { execCommand } from '$lib/api';

  let { namespace, pod, containers = [] }: {
    namespace: string;
    pod: string;
    containers: string[];
  } = $props();

  let selectedContainer = $state(containers[0] ?? '');
  let command = $state('');
  let history: Array<{ cmd: string; stdout: string; stderr: string; success: boolean }> = $state([]);
  let running = $state(false);
  let outputEl: HTMLDivElement | undefined = $state(undefined);

  async function runCommand() {
    if (!command.trim() || running) return;
    running = true;
    const cmd = command;
    command = '';

    try {
      const result = await execCommand(
        namespace,
        pod,
        selectedContainer || undefined,
        ['sh', '-c', cmd],
      );
      history = [...history, { cmd, ...result }];
    } catch (e) {
      history = [
        ...history,
        { cmd, stdout: '', stderr: e instanceof Error ? e.message : 'Exec failed', success: false },
      ];
    } finally {
      running = false;
      // Scroll to bottom after output renders
      requestAnimationFrame(() => {
        outputEl?.scrollTo({ top: outputEl.scrollHeight });
      });
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      runCommand();
    }
  }

  function clearHistory() {
    history = [];
  }
</script>

<div class="exec-terminal">
  <div class="terminal-toolbar">
    {#if containers.length > 1}
      <div class="container-selector">
        <label for="exec-container">Container:</label>
        <select id="exec-container" bind:value={selectedContainer}>
          {#each containers as c}
            <option value={c}>{c}</option>
          {/each}
        </select>
      </div>
    {:else if containers.length === 1}
      <span class="container-label">Container: {containers[0]}</span>
    {/if}
    {#if history.length > 0}
      <button class="clear-btn" onclick={clearHistory}>Clear</button>
    {/if}
  </div>

  <div class="terminal-output" bind:this={outputEl}>
    {#if history.length === 0 && !running}
      <div class="placeholder">
        Type a command to execute in the container. Commands run non-interactively via <code>sh -c</code>.
      </div>
    {/if}
    {#each history as entry}
      <div class="entry">
        <div class="prompt">$ {entry.cmd}</div>
        {#if entry.stdout}<pre class="stdout">{entry.stdout}</pre>{/if}
        {#if entry.stderr}<pre class="stderr">{entry.stderr}</pre>{/if}
        {#if !entry.success && !entry.stderr}
          <pre class="stderr">Command failed</pre>
        {/if}
      </div>
    {/each}
    {#if running}<div class="running">Running…</div>{/if}
  </div>

  <div class="input-line">
    <span class="prompt-char">$</span>
    <input
      type="text"
      bind:value={command}
      onkeydown={handleKeydown}
      placeholder="Enter command…"
      disabled={running}
    />
  </div>
</div>

<style>
  .exec-terminal {
    display: flex;
    flex-direction: column;
    background: #0d1117;
    border: 1px solid #21262d;
    border-radius: 6px;
    overflow: hidden;
    font-family: 'Cascadia Code', 'Fira Code', 'JetBrains Mono', monospace;
    font-size: 0.8rem;
    line-height: 1.5;
  }

  .terminal-toolbar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    background: #161b22;
    border-bottom: 1px solid #21262d;
    font-size: 0.75rem;
  }

  .container-selector {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .container-selector label {
    color: #8b949e;
  }

  .container-selector select {
    background: #0d1117;
    color: #e0e0e0;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0.2rem 0.4rem;
    font-size: 0.75rem;
    font-family: inherit;
  }

  .container-label {
    color: #8b949e;
  }

  .clear-btn {
    margin-left: auto;
    background: transparent;
    color: #8b949e;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0.15rem 0.5rem;
    font-size: 0.7rem;
    cursor: pointer;
  }
  .clear-btn:hover {
    color: #e0e0e0;
    border-color: #484f58;
  }

  .terminal-output {
    flex: 1;
    min-height: 200px;
    max-height: 50vh;
    overflow-y: auto;
    padding: 0.75rem;
  }

  .placeholder {
    color: #484f58;
    font-style: italic;
  }
  .placeholder code {
    color: #6e7681;
    background: #161b22;
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }

  .entry {
    margin-bottom: 0.5rem;
  }

  .prompt {
    color: #58a6ff;
    font-weight: 600;
  }

  .stdout {
    color: #3fb950;
    margin: 0;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .stderr {
    color: #f85149;
    margin: 0;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .running {
    color: #d29922;
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .input-line {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-top: 1px solid #21262d;
    background: #161b22;
  }

  .prompt-char {
    color: #3fb950;
    font-weight: 700;
  }

  .input-line input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #e0e0e0;
    font-family: inherit;
    font-size: inherit;
    caret-color: #3fb950;
  }

  .input-line input::placeholder {
    color: #484f58;
  }

  .input-line input:disabled {
    opacity: 0.5;
  }
</style>
