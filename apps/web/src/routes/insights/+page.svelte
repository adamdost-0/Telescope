<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    clearAiInsightsHistory,
    generateAiInsights,
    getAiInsightsSettings,
    listAiInsightsHistory,
    onApiError,
    testAiInsightsConnection,
  } from '$lib/api';
  import { formatAiInsightsScope } from '$lib/tauri-commands';
  import { isConnected } from '$lib/stores';
  import type {
    AiInsightsAuthMode,
    AiInsightsConnectionTestResult,
    AiInsightsHistoryEntry,
    AiInsightsResponse,
  } from '$lib/tauri-commands';

  const AI_INSIGHTS_COMMANDS = new Set([
    'test_ai_insights_connection',
    'generate_ai_insights',
    'list_ai_insights_history',
    'clear_ai_insights_history',
  ]);

  let insights = $state<AiInsightsResponse | null>(null);
  let history = $state<AiInsightsHistoryEntry[]>([]);
  let selectedHistoryCreatedAt = $state<string | null>(null);
  let connectionResult = $state<AiInsightsConnectionTestResult | null>(null);
  let loadingHistory = $state(true);
  let testingConnection = $state(false);
  let generating = $state(false);
  let clearingHistory = $state(false);
  let aiInsightsAuthMode = $state<AiInsightsAuthMode>('azureLogin');
  let apiKeyInput = $state('');
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);
  let unsubApiError: (() => void) | null = null;

  let sortedHistory = $derived.by(() => (
    [...history].sort((a, b) => b.createdAt.localeCompare(a.createdAt))
  ));

  function formatTimestamp(createdAt: string): string {
    const date = new Date(createdAt);
    if (Number.isNaN(date.getTime())) return createdAt;
    return date.toLocaleString();
  }

  function getSessionApiKey(): string | undefined {
    if (aiInsightsAuthMode !== 'apiKey') return undefined;
    const value = apiKeyInput.trim();
    return value.length > 0 ? value : undefined;
  }

  function sanitizeMessage(message: string): string {
    if (apiKeyInput.length === 0) return message;
    const redacted = message.split(apiKeyInput).join('[redacted]');
    const trimmedApiKey = apiKeyInput.trim();
    if (trimmedApiKey.length === 0 || trimmedApiKey === apiKeyInput) {
      return redacted;
    }
    return redacted.split(trimmedApiKey).join('[redacted]');
  }

  function toMessage(errorValue: unknown, fallback: string): string {
    const message = errorValue instanceof Error ? errorValue.message : fallback;
    return sanitizeMessage(message);
  }

  $effect(() => {
    if (aiInsightsAuthMode !== 'apiKey' && apiKeyInput.length > 0) {
      apiKeyInput = '';
    }
  });

  function selectHistoryEntry(entry: AiInsightsHistoryEntry) {
    insights = entry.response;
    selectedHistoryCreatedAt = entry.createdAt;
    status = `Loaded insights from ${formatTimestamp(entry.createdAt)}.`;
    error = null;
  }

  async function loadHistory() {
    loadingHistory = true;
    try {
      const entries = await listAiInsightsHistory();
      const orderedEntries = [...entries].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
      history = orderedEntries;

      if (orderedEntries.length === 0) {
        selectedHistoryCreatedAt = null;
        return;
      }

      if (selectedHistoryCreatedAt) {
        const selected = orderedEntries.find((entry) => entry.createdAt === selectedHistoryCreatedAt);
        if (selected) {
          insights = selected.response;
          return;
        }
      }

      insights = orderedEntries[0].response;
      selectedHistoryCreatedAt = orderedEntries[0].createdAt;
    } catch (e) {
      error = toMessage(e, 'Failed to load AI Insights history.');
    } finally {
      loadingHistory = false;
    }
  }

  async function handleTestConnection() {
    if (!$isConnected) return;
    testingConnection = true;
    error = null;
    status = null;
    try {
      connectionResult = await testAiInsightsConnection(getSessionApiKey());
      status = 'AI Insights connection test succeeded.';
    } catch (e) {
      error = toMessage(e, 'Failed to test AI Insights connection.');
    } finally {
      testingConnection = false;
    }
  }

  async function handleGenerate() {
    if (!$isConnected) return;
    generating = true;
    error = null;
    status = null;
    try {
      insights = await generateAiInsights(getSessionApiKey());
      selectedHistoryCreatedAt = null;
      status = 'Generated AI Insights.';
      await loadHistory();
    } catch (e) {
      error = toMessage(e, 'Failed to generate AI Insights.');
    } finally {
      generating = false;
    }
  }

  async function handleClearHistory() {
    if (history.length === 0) return;
    clearingHistory = true;
    error = null;
    status = null;
    try {
      await clearAiInsightsHistory();
      history = [];
      insights = null;
      selectedHistoryCreatedAt = null;
      status = 'Cleared AI Insights history.';
    } catch (e) {
      error = toMessage(e, 'Failed to clear AI Insights history.');
    } finally {
      clearingHistory = false;
    }
  }

  onMount(() => {
    unsubApiError = onApiError(({ command, message }) => {
      if (!AI_INSIGHTS_COMMANDS.has(command)) return;
      if (!error) {
        error = sanitizeMessage(`${command}: ${message}`);
      }
    });
    void (async () => {
      try {
        const settings = await getAiInsightsSettings();
        aiInsightsAuthMode = settings.authMode;
      } catch (e) {
        error = toMessage(e, 'Failed to load AI Insights settings.');
      }
    })();
    void loadHistory();
  });

  onDestroy(() => {
    apiKeyInput = '';
    unsubApiError?.();
  });
</script>

<div class="insights-page">
  <header class="page-header">
    <div>
      <h1>AI Insights</h1>
      <p class="subtitle">Generate deterministic cluster analysis with summary, risks, observations, recommendations, and references.</p>
    </div>
  </header>

  {#if !$isConnected}
    <p class="notice" role="status">
      Connect to a cluster to test or generate insights. History remains available while disconnected.
    </p>
  {/if}

  {#if status}
    <p class="status" role="status">{status}</p>
  {/if}
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <section class="controls" aria-label="AI Insights controls">
    {#if aiInsightsAuthMode === 'apiKey'}
      <label class="api-key-field">
        <span>API key (session only)</span>
        <input
          type="password"
          bind:value={apiKeyInput}
          placeholder="Enter API key for this session"
          autocomplete="off"
          spellcheck={false}
        />
        <span class="api-key-hint">Stored in memory only and cleared when this view closes.</span>
      </label>
    {/if}

    <div class="control-buttons">
      <button type="button" onclick={handleTestConnection} disabled={!$isConnected || testingConnection || generating}>
        {testingConnection ? 'Testing connection…' : 'Test connection'}
      </button>
      <button type="button" class="primary" onclick={handleGenerate} disabled={!$isConnected || generating || testingConnection}>
        {generating ? 'Generating…' : 'Generate insights'}
      </button>
    </div>

    {#if connectionResult}
      <dl class="connection-result">
        <div>
          <dt>Endpoint</dt>
          <dd>{connectionResult.normalizedEndpoint}</dd>
        </div>
        <div>
          <dt>Chat URL</dt>
          <dd>{connectionResult.chatCompletionsUrl}</dd>
        </div>
        <div>
          <dt>Model</dt>
          <dd>{connectionResult.model}</dd>
        </div>
      </dl>
    {/if}
  </section>

  <div class="content-grid">
    <section class="results" aria-label="AI Insights response">
      <section class="result-section">
        <h2>Summary</h2>
        {#if insights}
          <p>{insights.summary || 'No summary available.'}</p>
        {:else}
          <p class="empty">Generate insights to populate this section.</p>
        {/if}
      </section>

      <section class="result-section">
        <h2>Risks</h2>
        {#if insights && insights.risks.length > 0}
          <ul>
            {#each insights.risks as risk}
              <li>
                <div class="row-heading">
                  <strong>{risk.title}</strong>
                  <span class={`impact impact-${risk.impact}`}>{risk.impact}</span>
                </div>
                <p>{risk.detail}</p>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty">No risks available.</p>
        {/if}
      </section>

      <section class="result-section">
        <h2>Observations</h2>
        {#if insights && insights.observations.length > 0}
          <ul>
            {#each insights.observations as observation}
              <li>
                <div class="row-heading">
                  <strong>{observation.area}</strong>
                </div>
                <p>{observation.detail}</p>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty">No observations available.</p>
        {/if}
      </section>

      <section class="result-section">
        <h2>Recommendations</h2>
        {#if insights && insights.recommendations.length > 0}
          <ul>
            {#each insights.recommendations as recommendation}
              <li>
                <div class="row-heading">
                  <strong>{recommendation.action}</strong>
                  <span class="confidence">Confidence {(recommendation.confidence * 100).toFixed(0)}%</span>
                </div>
                <p>{recommendation.rationale}</p>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty">No recommendations available.</p>
        {/if}
      </section>

      <section class="result-section">
        <h2>References</h2>
        {#if insights && insights.references.length > 0}
          <ul>
            {#each insights.references as reference}
              <li class="reference-row">
                <span>{reference.kind}</span>
                <span>{reference.name}</span>
                <span>{reference.namespace ?? 'cluster-scope'}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty">No references available.</p>
        {/if}
      </section>
    </section>

    <aside class="history" aria-label="AI Insights history">
      <div class="history-header">
        <h2>History</h2>
        <button type="button" class="clear-btn" onclick={handleClearHistory} disabled={clearingHistory || history.length === 0}>
          {clearingHistory ? 'Clearing…' : 'Clear all'}
        </button>
      </div>

      {#if loadingHistory}
        <p class="empty">Loading history…</p>
      {:else if sortedHistory.length === 0}
        <p class="empty">No history entries yet.</p>
      {:else}
        <ul class="history-list">
          {#each sortedHistory as entry}
            <li>
              <button
                type="button"
                class="history-entry"
                class:selected={selectedHistoryCreatedAt === entry.createdAt}
                onclick={() => selectHistoryEntry(entry)}
              >
                <span class="history-time">{formatTimestamp(entry.createdAt)}</span>
                <span class="history-scope">{formatAiInsightsScope(entry.scope)}</span>
                <span class="history-summary">{entry.response.summary || 'No summary available.'}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </aside>
  </div>
</div>

<style>
  .insights-page {
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .page-header h1 {
    margin: 0;
    font-size: 1.5rem;
  }
  .subtitle {
    margin: 0.35rem 0 0;
    color: var(--text-secondary);
  }
  .notice,
  .status,
  .error {
    margin: 0;
    padding: 0.75rem 0.9rem;
    border-radius: 8px;
    border: 1px solid var(--border);
  }
  .notice {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }
  .status {
    background: color-mix(in srgb, var(--success) 15%, transparent);
    color: var(--text-primary);
  }
  .error {
    background: color-mix(in srgb, var(--error) 12%, transparent);
    color: var(--text-primary);
  }
  .controls {
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 0.9rem;
    background: var(--bg-secondary);
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }
  .control-buttons {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .api-key-field {
    display: grid;
    gap: 0.35rem;
  }
  .api-key-field span {
    font-size: 0.82rem;
    color: var(--text-secondary);
  }
  .api-key-hint {
    font-size: 0.75rem;
  }
  .api-key-field input {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    padding: 0.45rem 0.6rem;
    font: inherit;
  }
  button {
    border: 1px solid var(--border);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border-radius: 6px;
    padding: 0.45rem 0.8rem;
    cursor: pointer;
    font: inherit;
  }
  button.primary {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }
  button:hover:not(:disabled) {
    filter: brightness(1.06);
  }
  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .connection-result {
    margin: 0;
    display: grid;
    gap: 0.6rem;
  }
  .connection-result div {
    display: grid;
    gap: 0.2rem;
  }
  .connection-result dt {
    color: var(--text-secondary);
    font-size: 0.8rem;
  }
  .connection-result dd {
    margin: 0;
    font-size: 0.9rem;
    word-break: break-word;
  }
  .content-grid {
    display: grid;
    gap: 1rem;
    grid-template-columns: 2fr 1fr;
    align-items: start;
  }
  .results {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }
  .result-section {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-secondary);
    padding: 0.85rem;
  }
  .result-section h2 {
    margin: 0 0 0.65rem;
    font-size: 1rem;
  }
  .result-section p {
    margin: 0;
    color: var(--text-primary);
  }
  .result-section ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }
  .result-section li {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.65rem;
    background: var(--bg-tertiary);
  }
  .row-heading {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.4rem;
  }
  .impact {
    text-transform: uppercase;
    font-size: 0.72rem;
    letter-spacing: 0.03em;
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.1rem 0.5rem;
  }
  .impact-low {
    color: var(--success);
  }
  .impact-medium {
    color: var(--warning);
  }
  .impact-high {
    color: var(--error);
  }
  .confidence {
    color: var(--text-secondary);
    font-size: 0.82rem;
  }
  .reference-row {
    display: grid;
    gap: 0.5rem;
    grid-template-columns: 1fr 1.3fr 1fr;
    align-items: center;
  }
  .history {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-secondary);
    padding: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .history-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
  }
  .history-header h2 {
    margin: 0;
    font-size: 1rem;
  }
  .clear-btn {
    font-size: 0.85rem;
  }
  .history-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 480px;
    overflow-y: auto;
  }
  .history-entry {
    width: 100%;
    text-align: left;
    display: grid;
    gap: 0.25rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.6rem;
  }
  .history-entry.selected {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .history-time {
    color: var(--text-secondary);
    font-size: 0.75rem;
  }
  .history-scope {
    color: var(--text-primary);
    font-size: 0.82rem;
    font-weight: 600;
  }
  .history-summary {
    color: var(--text-secondary);
    font-size: 0.8rem;
  }
  .empty {
    color: var(--text-secondary);
  }
  @media (max-width: 980px) {
    .content-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
