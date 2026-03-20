<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    listPortForwardSessions,
    stopPortForwardSession,
    startPortForward,
    getPortForwardProfiles,
    savePortForwardProfiles,
    type PortForwardSession,
    type PortForwardProfile,
  } from '$lib/api';
  import { isConnected } from '$lib/stores';

  let sessions = $state<PortForwardSession[]>([]);
  let profiles = $state<PortForwardProfile[]>([]);
  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let loading = $state(true);
  let activeTab = $state<'sessions' | 'profiles'>('sessions');
  let showAddProfile = $state(false);
  let newProfile = $state<PortForwardProfile>({
    name: '',
    namespace: 'default',
    pod: '',
    localPort: 8080,
    remotePort: 80,
  });

  function formatDuration(startedAt: string): string {
    const start = new Date(startedAt);
    const now = new Date();
    const diffSec = Math.floor((now.getTime() - start.getTime()) / 1000);
    if (diffSec < 60) return `${diffSec}s`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m`;
    const diffHours = Math.floor(diffMin / 60);
    return `${diffHours}h ${diffMin % 60}m`;
  }

  async function loadSessions() {
    if (!$isConnected) {
      loading = false;
      sessions = [];
      return;
    }
    sessions = await listPortForwardSessions();
    loading = false;
  }

  async function loadProfiles() {
    profiles = await getPortForwardProfiles();
  }

  async function handleStop(sessionId: string) {
    try {
      await stopPortForwardSession(sessionId);
      await loadSessions();
    } catch (e) {
      console.error('Failed to stop port-forward:', e);
    }
  }

  async function handleStartProfile(profile: PortForwardProfile) {
    try {
      const response = await startPortForward(
        profile.namespace,
        profile.pod,
        profile.localPort,
        profile.remotePort
      );
      console.log(`Started profile "${profile.name}": session ${response.session_id}`);
      await loadSessions();
    } catch (e) {
      console.error(`Failed to start profile "${profile.name}":`, e);
    }
  }

  async function handleSaveProfile() {
    if (!newProfile.name || !newProfile.pod) {
      alert('Please provide a name and pod');
      return;
    }
    const updated = [...profiles, { ...newProfile }];
    await savePortForwardProfiles(updated);
    profiles = updated;
    showAddProfile = false;
    newProfile = {
      name: '',
      namespace: 'default',
      pod: '',
      localPort: 8080,
      remotePort: 80,
    };
  }

  async function handleDeleteProfile(index: number) {
    const updated = profiles.filter((_, i) => i !== index);
    await savePortForwardProfiles(updated);
    profiles = updated;
  }

  $effect(() => {
    void $isConnected;
    loadSessions();
  });

  onMount(() => {
    loadProfiles();
    refreshTimer = setInterval(loadSessions, 2000);
  });

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
  });
</script>

<div class="connectivity-page">
  <header>
    <h1>Service Connectivity</h1>
    <div class="controls">
      <button type="button" onclick={loadSessions} aria-label="Refresh sessions">
        <span class="refresh-icon">↻</span> Refresh
      </button>
    </div>
  </header>

  <div class="tabs">
    <button
      type="button"
      class="tab"
      class:active={activeTab === 'sessions'}
      onclick={() => (activeTab = 'sessions')}
    >
      Active Sessions ({sessions.length})
    </button>
    <button
      type="button"
      class="tab"
      class:active={activeTab === 'profiles'}
      onclick={() => (activeTab = 'profiles')}
    >
      Saved Profiles ({profiles.length})
    </button>
  </div>

  {#if activeTab === 'sessions'}
    {#if !$isConnected && !loading}
      <div class="not-connected">
        <p>🔌 Not connected to a cluster</p>
        <p class="hint">Select a context from the header to connect.</p>
      </div>
    {:else if loading}
      <div class="loading">Loading sessions...</div>
    {:else if sessions.length === 0}
      <div class="empty-state">
        <p>No active port-forward sessions</p>
        <p class="hint">Start a port-forward from a pod detail page or saved profile.</p>
      </div>
    {:else}
      <div class="sessions-grid">
        {#each sessions as session (session.id)}
          <div class="session-card">
            <div class="session-header">
              <span class="session-id">{session.id}</span>
              <span class="status status-{session.status.toLowerCase()}">{session.status}</span>
            </div>
            <div class="session-body">
              <div class="session-row">
                <span class="label">Pod:</span>
                <span class="value">
                  <a href="/pods/{session.namespace}/{session.pod}">{session.pod}</a>
                </span>
              </div>
              <div class="session-row">
                <span class="label">Namespace:</span>
                <span class="value">{session.namespace}</span>
              </div>
              <div class="session-row">
                <span class="label">Forward:</span>
                <span class="value port-mapping">
                  localhost:{session.local_port} → {session.pod}:{session.remote_port}
                </span>
              </div>
              <div class="session-row">
                <span class="label">Duration:</span>
                <span class="value">{formatDuration(session.started_at)}</span>
              </div>
            </div>
            <div class="session-actions">
              {#if session.status === 'Active'}
                <button type="button" class="stop-btn" onclick={() => handleStop(session.id)}>
                  Stop
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {:else}
    <div class="profiles-section">
      <div class="profiles-header">
        <button type="button" class="add-btn" onclick={() => (showAddProfile = true)}>
          + Add Profile
        </button>
      </div>

      {#if showAddProfile}
        <div class="profile-form">
          <h3>New Port Forward Profile</h3>
          <label>
            Profile Name
            <input type="text" bind:value={newProfile.name} placeholder="e.g., My App Dev" />
          </label>
          <label>
            Namespace
            <input type="text" bind:value={newProfile.namespace} />
          </label>
          <label>
            Pod Name
            <input type="text" bind:value={newProfile.pod} placeholder="e.g., my-app-pod" />
          </label>
          <div class="port-row">
            <label>
              Local Port
              <input type="number" bind:value={newProfile.localPort} min="0" max="65535" />
            </label>
            <span class="arrow">→</span>
            <label>
              Remote Port
              <input type="number" bind:value={newProfile.remotePort} min="1" max="65535" />
            </label>
          </div>
          <div class="form-actions">
            <button type="button" class="cancel-btn" onclick={() => (showAddProfile = false)}>
              Cancel
            </button>
            <button type="button" class="save-btn" onclick={handleSaveProfile}>Save</button>
          </div>
        </div>
      {/if}

      {#if profiles.length === 0}
        <div class="empty-state">
          <p>No saved profiles</p>
          <p class="hint">Create a profile to quickly start port-forwards.</p>
        </div>
      {:else}
        <div class="profiles-grid">
          {#each profiles as profile, index (profile.name)}
            <div class="profile-card">
              <div class="profile-header">
                <span class="profile-name">{profile.name}</span>
              </div>
              <div class="profile-body">
                <div class="profile-row">
                  <span class="label">Pod:</span>
                  <span class="value">{profile.pod}</span>
                </div>
                <div class="profile-row">
                  <span class="label">Namespace:</span>
                  <span class="value">{profile.namespace}</span>
                </div>
                <div class="profile-row">
                  <span class="label">Forward:</span>
                  <span class="value port-mapping">
                    localhost:{profile.localPort} → {profile.pod}:{profile.remotePort}
                  </span>
                </div>
              </div>
              <div class="profile-actions">
                <button
                  type="button"
                  class="start-btn"
                  onclick={() => handleStartProfile(profile)}
                  disabled={!$isConnected}
                >
                  Start
                </button>
                <button
                  type="button"
                  class="delete-btn"
                  onclick={() => handleDeleteProfile(index)}
                >
                  Delete
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .connectivity-page {
    padding: 1rem;
    color: #e0e0e0;
    background: #0f0f23;
    min-height: 100vh;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }

  h1 {
    font-size: 1.5rem;
    margin: 0;
  }

  .controls button {
    background: #1a73e8;
    color: white;
    border: none;
    padding: 0.375rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }

  .controls button:hover {
    background: #1565c0;
  }

  .refresh-icon {
    display: inline-block;
  }

  .not-connected,
  .loading,
  .empty-state {
    text-align: center;
    padding: 3rem 1rem;
    color: #757575;
  }

  .hint {
    font-size: 0.875rem;
    color: #616161;
    margin-top: 0.5rem;
  }

  .sessions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
    gap: 1rem;
  }

  .session-card {
    background: #1c1c2e;
    border: 1px solid #2a2a3e;
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .session-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid #2a2a3e;
  }

  .session-id {
    font-family: monospace;
    font-size: 0.9rem;
    color: #4fc3f7;
  }

  .status {
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
  }

  .status-active {
    background: rgba(76, 175, 80, 0.2);
    color: #4caf50;
  }

  .status-stopping {
    background: rgba(255, 152, 0, 0.2);
    color: #ff9800;
  }

  .status-stopped {
    background: rgba(158, 158, 158, 0.2);
    color: #9e9e9e;
  }

  .session-body {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .session-row {
    display: flex;
    gap: 0.5rem;
    font-size: 0.85rem;
  }

  .label {
    color: #8b949e;
    min-width: 90px;
  }

  .value {
    color: #e0e0e0;
  }

  .value a {
    color: #58a6ff;
    text-decoration: none;
  }

  .value a:hover {
    text-decoration: underline;
  }

  .port-mapping {
    font-family: monospace;
    background: #161b22;
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
  }

  .session-actions {
    display: flex;
    justify-content: flex-end;
    padding-top: 0.5rem;
    border-top: 1px solid #2a2a3e;
  }

  .stop-btn {
    background: #d32f2f;
    color: white;
    border: none;
    padding: 0.4rem 0.8rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .stop-btn:hover {
    background: #b71c1c;
  }

  .tabs {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1.5rem;
    border-bottom: 2px solid #2a2a3e;
  }

  .tab {
    background: transparent;
    border: none;
    color: #8b949e;
    padding: 0.75rem 1rem;
    cursor: pointer;
    font-size: 0.9rem;
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
  }

  .tab:hover {
    color: #e0e0e0;
  }

  .tab.active {
    color: #4fc3f7;
    border-bottom-color: #4fc3f7;
  }

  .profiles-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .profiles-header {
    display: flex;
    justify-content: flex-end;
  }

  .add-btn {
    background: #238636;
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .add-btn:hover {
    background: #2ea043;
  }

  .profile-form {
    background: #1c1c2e;
    border: 1px solid #2a2a3e;
    border-radius: 8px;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .profile-form h3 {
    margin: 0 0 0.5rem;
    color: #4fc3f7;
  }

  .profile-form label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.85rem;
    color: #8b949e;
  }

  .profile-form input {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 4px;
    color: #e0e0e0;
    padding: 0.5rem;
    font-size: 0.9rem;
  }

  .profile-form input:focus {
    outline: none;
    border-color: #58a6ff;
  }

  .profile-form .port-row {
    display: flex;
    align-items: flex-end;
    gap: 0.75rem;
  }

  .profile-form .arrow {
    font-size: 1.5rem;
    color: #58a6ff;
    padding-bottom: 0.35rem;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid #2a2a3e;
  }

  .cancel-btn {
    background: transparent;
    color: #8b949e;
    border: 1px solid #30363d;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
  }

  .cancel-btn:hover {
    background: #21262d;
  }

  .save-btn {
    background: #238636;
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
  }

  .save-btn:hover {
    background: #2ea043;
  }

  .profiles-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
    gap: 1rem;
  }

  .profile-card {
    background: #1c1c2e;
    border: 1px solid #2a2a3e;
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .profile-header {
    padding-bottom: 0.5rem;
    border-bottom: 1px solid #2a2a3e;
  }

  .profile-name {
    font-weight: 600;
    font-size: 1rem;
    color: #4fc3f7;
  }

  .profile-body {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .profile-row {
    display: flex;
    gap: 0.5rem;
    font-size: 0.85rem;
  }

  .profile-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid #2a2a3e;
  }

  .start-btn {
    background: #238636;
    color: white;
    border: none;
    padding: 0.4rem 0.8rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .start-btn:hover:not(:disabled) {
    background: #2ea043;
  }

  .start-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .delete-btn {
    background: #d32f2f;
    color: white;
    border: none;
    padding: 0.4rem 0.8rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .delete-btn:hover {
    background: #b71c1c;
  }
</style>
