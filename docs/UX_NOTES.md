# Telescope — UX Notes (Draft)

> ⚠️ **Status: Aspirational** — Only `/`, `/clusters`, and `/explore` routes exist. All other screens described below are planned, not implemented.

## Navigation
- Left rail: Clusters • Explore • Workloads • Helm • Observability • Settings
- Top bar: cluster switcher, namespace selector, search, connection status
- Main: list/detail split; tabs in detail view

## Must-have screens
- Cluster list + quick health
- Cluster overview (cheap "what's wrong")
- Resource explorer (GVR-driven) with virtualized tables
- Workload detail tabs: Summary • Pods • Logs • Exec • YAML • Events
- Logs viewer: ring buffer, pause, search, container selector
- Exec: dockable terminal sessions
- Helm: releases list + detail (values/manifest/history/diff)

## Performance rules
- Virtualize every list.
- Watch streams over polling; stop watches when not visible.
- Lazy-load data per tab; bounded logs.
