# Telescope — Current UX Report

> **Author:** Adam the Architect · Design Review Panel Input Document
> **Scope:** Desktop-only Tauri v2 Kubernetes IDE (v1.0.0)
> **Source:** Static analysis of apps/web frontend codebase

---

## 1. Executive Summary

Telescope is a **desktop-only Kubernetes IDE** built with SvelteKit 5 (runes) inside a Tauri v2 shell. All data flows through Tauri IPC — there is no HTTP backend, no browser mode, and no server-side rendering.

The UX model is a **sidebar-driven resource browser** with a persistent app shell comprising a 48 px header, a collapsible 220 px sidebar, and a scrollable main content area. A command-palette search (Cmd+K), Vim-inspired g-prefix shortcuts, and a production-safety banner round out the interaction surface. Azure AKS features are conditionally injected into navigation and dashboard views when an AKS cluster is detected.

**Key UX characteristics:**
- ~39 routable pages covering 28+ Kubernetes resource types, Helm releases, CRDs, and Azure AKS node pools.
- GitHub-inspired dark-first design system using CSS custom properties (12 semantic tokens), with a full light theme variant.
- Keyboard-first power-user orientation: global shortcuts, command palette, g-prefix navigation.
- Production-safety rails: pattern-matched context detection, red warning banner, PROD badges, type-to-confirm destructive dialogs.
- Minimal Azure surface: a single conditional sidebar section ("Node Pools") and AKS-aware dashboard cards.

---

## 2. Navigation System

### 2.1 Sidebar Structure

The sidebar organises resources into **8 logical sections** (9 when Azure is active):

| # | Section | Items | Representative Resources |
|---|---------|------:|--------------------------|
| 1 | **Cluster** | 6 | Overview, Namespaces, Create, Nodes, Priority Classes, Events |
| 2 | **Azure** *(conditional)* | 1 | Node Pools |
| 3 | **Workloads** | 8 | Pods, Deployments, StatefulSets, DaemonSets, Jobs, CronJobs, HPAs, PDBs |
| 4 | **Network** | 4 | Services, Ingresses, NetworkPolicies, EndpointSlices |
| 5 | **Config** | 4 | ConfigMaps, Secrets, ResourceQuotas, LimitRanges |
| 6 | **Storage** | 3 | PVCs, PVs, Storage Classes |
| 7 | **Helm** | 1 | Releases |
| 8 | **Custom Resources** | 1 | CRDs |
| 9 | **System** | 1 | Settings |

**Collapse behaviour:** Toggles between 220 px (icons + labels, grouped by section headers) and 48 px (icon-only, flat list). Transition is 0.2 s ease. Collapse state is component-local (Svelte $state(false)) and does **not** persist across sessions.

**Active-state logic:** Exact match for /overview; prefix match for all other routes via page.url.pathname.startsWith(item.href). Active items receive --bg-hover background and --accent text colour.

**Disabled-state logic:** When disconnected (!isConnected), every item except /, /overview, and /settings is disabled (opacity 0.4, pointer-events: none, aria-disabled, tooltip: "Connect to a cluster first").

### 2.2 Conditional Azure Section

The Azure section is derived reactively:

```ts
const sections = $derived.by((): NavSection[] => {
  if (!$isAks || !$isConnected) return baseSections;
  // Insert Azure section after Cluster
  return [baseSections[0], azureSection, ...baseSections.slice(1)];
});
```

**Detection mechanism:** isAks is a derived store that pattern-matches the cluster API server URL against .azmk8s.io. No Azure SDK call is required — it is a pure string check.

**Observation:** The Azure section currently contains a single item ("Node Pools" pointing to /nodes) that shares its href with the existing "Nodes" item in the Cluster section. This creates a **duplicate route target** — two sidebar entries resolving to the same page — which may confuse users about the distinction between Kubernetes nodes and AKS node pools.

### 2.3 Accessibility

- nav element with aria-label="Resource navigation" and distinct labels for expanded/collapsed states.
- Disabled items are excluded from tab order (tabindex={-1}).
- No skip-link to main content (acceptable for a desktop app where the sidebar is always visible and tab-navigable).

---

## 3. Interaction Design

### 3.1 App Shell Layout

```
+--------------------------------------------------+
|  [Prod Banner — conditional, red, full-width]    |
+--------------------------------------------------+
|  Telescope    [Status] [Context v] [theme] [?]   |  <- 48 px header
+-----------+--------------------------------------+
|           |                                      |
|  Sidebar  |         main content                 |
|  220 px   |         flex: 1, overflow-y: auto    |
|           |         padding: 1rem                |
|           |                                      |
+-----------+--------------------------------------+
```

- **Production banner:** Rendered above the header when isProduction is true. Red (--danger-bg), white text, uppercase "PRODUCTION". Triggered by configurable pattern matching against the active kubeconfig context name.
- **Header contents:** Brand mark, spacer, ConnectionStatus (7-state indicator with pulse animations), ContextSwitcher (context dropdown + namespace dropdown + disconnect button + auth/AKS badges + server URL), ThemeToggle (sun/moon), Help button (?).
- **Main area:** Renders the matched SvelteKit route. Pages follow two dominant patterns: *list page* (FilterBar, count, ResourceTable) and *detail page* (Breadcrumbs, Tabs, content panels including YAML editor).

### 3.2 Global Keyboard Shortcuts

| Shortcut | Action | Notes |
|----------|--------|-------|
| Ctrl/Cmd + K | Toggle search palette | Prevented on input/textarea/select focus |
| ? | Toggle shortcut help | Prevented on input focus |
| Escape | Close topmost overlay | Help, search palette |
| g then o | Go to Overview | 1000 ms timeout for second key |
| g then p | Go to Pods | |
| g then d | Go to Deployments | |
| g then s | Go to Services | |
| g then n | Go to Nodes | |
| g then e | Go to Events | |
| g then h | Go to Helm | |
| g then c | Go to CRDs | |
| g then t | Go to Settings | |

The g-prefix system is Vim-inspired: pressing g starts a 1-second window during which a second key selects the destination. Shortcuts are suppressed when an input, textarea, or select element has focus.

### 3.3 Search / Command Palette

- **Trigger:** Ctrl/Cmd + K or future sidebar button.
- **Overlay:** Fixed, centred, min(560px, 90vw) wide, 420 px max-height, z-index: 100 with 60% black backdrop.
- **Search:** 200 ms debounced, case-insensitive, matches resource name and kind against the local cache.
- **Results:** Grouped by kind with emoji icon headers (Pod, Deployment, etc.). Each result shows name (left, truncated) and namespace badge (right).
- **Keyboard:** Up/Down to navigate, Enter to select, Escape to close. Full role="listbox" / role="option" ARIA semantics.

### 3.4 Connection State Feedback

The ConnectionStatus component in the header provides 7 discrete states:

| State | Indicator | Animation |
|-------|-----------|-----------|
| Ready | Green dot | — |
| Syncing | Blue dot | pulse 1.5 s |
| Degraded | Orange dot | — |
| Error | Red dot | — |
| Backoff | Orange dot | pulse 2 s |
| Connecting | Blue dot | pulse 1.5 s |
| Disconnected | Grey dot | — |

Announced to screen readers via role="status" and aria-live="polite".

---

## 4. Information Architecture

### 4.1 Route Hierarchy

```
/                                 <- Cluster picker (landing, lists kubeconfig contexts)
+-- /overview                     <- Dashboard (resource counts, pod status, events, AKS cards)
+-- /pods                         <- Pod list
|   +-- /pods/[ns]/[name]        <- Pod detail (Summary | Logs | Exec | Events | YAML)
+-- /nodes                        <- Node list (AKS-aware pool grouping)
|   +-- /nodes/[name]            <- Node detail
+-- /events                       <- Cluster event browser (filterable by type/namespace)
+-- /namespaces                   <- Namespace management (create/delete)
+-- /helm                         <- Helm release list
|   +-- /helm/[ns]/[name]        <- Helm release detail
+-- /crds                         <- CRD list
|   +-- /crds/[group]/[kind]     <- CRD instance list
+-- /create                       <- Resource creation (template selector + YAML editor)
+-- /settings                     <- Preferences (Appearance, Cluster, Safety, Azure, About)
+-- /azure/
|   +-- /azure/node-pools        <- AKS node pool management (1298 lines)
+-- /resources/
    +-- /resources/deployments    <- Generic resource list (repeated for ~20 types)
    +-- /resources/services
    +-- /resources/configmaps
    +-- /resources/secrets
    +-- /resources/statefulsets
    +-- /resources/daemonsets
    +-- /resources/jobs
    +-- /resources/cronjobs
    +-- /resources/hpas
    +-- /resources/pvcs
    +-- /resources/persistentvolumes
    +-- /resources/storageclasses
    +-- /resources/ingresses
    +-- /resources/networkpolicies
    +-- /resources/endpointslices
    +-- /resources/resourcequotas
    +-- /resources/limitranges
    +-- /resources/priorityclasses
    +-- /resources/poddisruptionbudgets
    +-- /resources/webhooks
    +-- /resources/roles
    +-- /resources/rolebindings
    +-- /resources/serviceaccounts
        +-- /resources/[kind]/[ns]/[name]  <- Generic resource detail
```

### 4.2 Page Patterns

**List pages** follow a consistent template:
1. Header row: title, namespace selector (if scoped), last-updated timestamp, refresh button.
2. FilterBar for text search.
3. Resource count badge.
4. ResourceTable with type-specific columns (e.g., Deployments show Ready / Up-to-date / Available).
5. Empty state when disconnected; loading skeleton while fetching; error message with retry.

**Detail pages** use a tabbed layout:
- **Summary** — metadata, status, labels, annotations, type-specific sections.
- **Events** — associated cluster events.
- **Logs** (pods only) — streaming log viewer with container selector.
- **Exec** (pods only) — interactive terminal session.
- **YAML** — full resource definition with syntax-highlighted editor, Apply, Dry-run, and dirty-state indicator.
- Action buttons: Delete (with confirmation), Scale (Deployments/StatefulSets), Restart/Rollout, Port-Forward.

### 4.3 Data Loading

- **No SSR:** ssr = false, prerender = false — entirely client-side via Tauri IPC.
- **Auto-refresh:** Configurable interval (5-300 s, default varies by page). Stale-data warning if data is > 30 s old (overview page).
- **State:** Svelte 5 runes ($state, $derived, $effect) replace legacy $: reactivity.

---

## 5. Visual System

### 5.1 Design Tokens

The entire colour system is encoded in 12 CSS custom properties, scoped to [data-theme] on html:

| Token | Dark | Light | Semantic Role |
|-------|------|-------|---------------|
| --bg-primary | #0f0f23 | #ffffff | Page canvas |
| --bg-secondary | #0d1117 | #f6f8fa | Sidebar, header, cards |
| --bg-tertiary | #161b22 | #f0f2f5 | Modals, panels |
| --bg-hover | #1f2937 | #e8eaed | Interactive hover |
| --border | #21262d | #d0d7de | Dividers, outlines |
| --text-primary | #e0e0e0 | #1f2328 | Body text |
| --text-secondary | #8b949e | #656d76 | Labels, secondary copy |
| --text-muted | #484f58 | #8c959f | Hints, timestamps |
| --accent | #58a6ff | #0969da | Links, focus, active state |
| --success | #66bb6a | #1a7f37 | Ready / healthy |
| --warning | #ffa726 | #9a6700 | Caution / degraded |
| --error | #ef5350 | #cf222e | Errors, danger text |

**Aesthetic:** GitHub-inspired dark palette. The dark theme primary (#0f0f23) is slightly warmer than GitHub's #0d1117, giving Telescope a distinct but familiar feel.

### 5.2 Typography

- **System font stack:** -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif.
- **Monospace:** 'SF Mono', 'Fira Code', monospace (used in shortcut keys, YAML editor, log viewer).
- **Scale:** 0.65 rem to 1.25 rem (no design-token abstraction for sizes — raw rem values throughout).
- **Weights:** 500 (active nav items), 600 (section headers), 700 (brand).

### 5.3 Spacing and Shape

- **Spacing:** Ad-hoc rem values (0.25, 0.375, 0.5, 0.75, 1, 1.25) — no named spacing scale.
- **Border radius:** 3 px, 4 px, 6 px, 12 px — inconsistent, component-specific.
- **Z-index:** Two layers: header at 10, modals/overlays at 100.

### 5.4 Theme Switching

- Attribute-based: document.documentElement.setAttribute('data-theme', 'dark' | 'light').
- Persisted to localStorage (telescope-theme) and Tauri preferences API.
- Falls back to prefers-color-scheme media query on first launch.
- Toggle via sun/moon button in the header.

### 5.5 Iconography

- **Navigation icons:** Unicode emoji (various resource-type emoji) — not an icon font or SVG sprite.
- **Connection-state dots:** Inline-styled span elements with hard-coded hex colours.
- **Implications:** Emoji rendering varies across platforms (Windows vs macOS). No icon theming or size consistency guarantees.

---

## 6. Azure Integration Analysis

### 6.1 Detection

AKS detection is **URL-based, client-side, zero-cost:**

```ts
export const isAks = derived(clusterServerUrl, ($url) =>
  $url ? isAksCluster($url) : false
);
// isAksCluster checks for `.azmk8s.io` in the server URL
```

No Azure SDK token exchange or ARM call is required to activate Azure features. This is a deliberate design choice: Azure UI surface lights up instantly and does not block cluster connection.

### 6.2 Current Azure UX Surface

| Touchpoint | Location | Behaviour |
|------------|----------|-----------|
| **Sidebar section** | Between Cluster and Workloads | Single item: "Node Pools" pointing to /nodes |
| **Overview dashboard** | /overview (1482 lines) | AKS-specific cards: cluster details, upgrade availability, maintenance config |
| **Node pool page** | /azure/node-pools (1298 lines) | Full node pool list with VM size, OS, mode, ready count |
| **Settings panel** | /settings Azure section | Cloud environment selector (Commercial/Gov/Secret/TS), subscription/RG/cluster manual override, auto-detect |
| **Context switcher** | Header | Blue "AKS" badge when AKS detected |
| **Resource detail** | /resources/[kind]/[ns]/[name] | AzureIdentitySection component for service principal metadata |

### 6.3 Critical Observation: Route Aliasing

The sidebar "Node Pools" entry points to /nodes — the **same route** as the Cluster Nodes entry. Meanwhile, a dedicated /azure/node-pools route exists with 1298 lines of AKS-specific node pool management UI. This appears to be a **wiring bug** or an intentional simplification that under-surfaces the richer Azure page.

### 6.4 UX Gap Analysis

1. **Minimal navigation presence:** Azure is represented by a single sidebar item. The rich functionality in /azure/node-pools, the overview AKS cards, and the settings Azure section are not discoverable from the sidebar alone.
2. **No Azure breadcrumb trail:** There is no /azure parent route or layout — /azure/node-pools exists as an orphan route with no grouping context.
3. **Missing from command palette:** The g-prefix shortcuts have no Azure binding (e.g., g then a for Azure).
4. **No progressive disclosure:** Azure features are binary (shown/hidden). There is no onboarding hint, empty state, or "Connect Azure" CTA for non-AKS clusters that could still benefit from Azure context.
5. **Settings-only deep config:** Azure cloud selection, subscription ID, and resource group are buried in Settings with no contextual link from the Azure sidebar section or the overview dashboard.

---

## Appendix A: Component Inventory

| Component | Purpose |
|-----------|---------|
| AppHeader | Top bar: brand, status, context, theme, help |
| Sidebar | Primary nav (8-9 sections, collapse toggle) |
| SearchPalette | Cmd+K command palette |
| ShortcutHelp | ? keyboard shortcut reference |
| ConnectionStatus | 7-state cluster connection indicator |
| ContextSwitcher | Kubeconfig context + namespace dropdowns |
| ThemeToggle | Dark/light theme switcher |
| Breadcrumbs | Detail-page breadcrumb trail |
| FilterBar | Debounced text filter input |
| ResourceTable | Generic resource list table |
| Tabs | Tabbed content navigation |
| YamlEditor | Syntax-highlighted YAML editor with apply/dry-run |
| LogViewer | Streaming pod log viewer |
| ExecTerminal | Interactive pod exec terminal |
| PortForwardDialog | Port-forward configuration |
| ScaleDialog | Replica count adjustment |
| ConfirmDialog | Destructive action confirmation |
| ErrorMessage | Error display with retry |
| LoadingSkeleton | Loading placeholder |
| Sparkline | Inline metric sparklines |
| AzureIdentitySection | AKS service principal info |
| NodePoolHeader | AKS node pool group header |

## Appendix B: Accessibility Summary

| Feature | Status |
|---------|--------|
| Semantic landmarks (nav, main, header) | Implemented |
| ARIA labels on interactive regions | Implemented |
| Focus-visible outlines (2 px accent ring) | Global |
| Modal focus trapping / aria-modal | Search, Help |
| role="status" / aria-live for connection state | Implemented |
| Disabled-state ARIA (aria-disabled, tabindex) | Sidebar, context |
| Skip-link to main content | Not present (desktop-only, acceptable) |
| Screen-reader-only text for emoji icons | Not present |
| Colour contrast (WCAG AA) | Likely passes for primary text; not audited for muted tokens |
