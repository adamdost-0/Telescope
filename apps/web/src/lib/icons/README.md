# Icon System (`apps/web/src/lib/icons`)

Telescope uses a **central icon registry** backed by [Tabler Icons Svelte](https://github.com/tabler/tabler-icons) (MIT licensed).

## Usage

```svelte
<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
</script>

<button class="action" data-testid="create-button">
  <Icon name="create" size={18} ariaLabel="Create resource" />
  <span>Create</span>
</button>
```

- **Do not inline emoji** in components. Use the registry.
- Icons are **tree-shaken**: only imported components are bundled.
- Icons use `currentColor` so they inherit text color.
- Provide `aria-label` when the icon conveys meaning. Decorative icons should set `aria-hidden="true"` (default when no `ariaLabel` is passed).

## Files

- `Icon.svelte` — thin wrapper that renders a registered icon component with size/color props.
- `index.ts` — exports `getIcon`, `IconName`, and typed maps for **resource** and **action** icons.
- `resource-icons.ts` — mapping for Kubernetes resource kinds and UI routes.
- `action-icons.ts` — mapping for UI actions (dry-run, apply, reset, create, reload, settings, prod warning, auth badges, helm lock/unlock, diagnostics).
- `auth-icons.ts` — auth method badges (exec, token, cert, oidc)

## Adding a new icon

1. Choose a Tabler icon component (e.g., `IconFlask`, `IconRocket`).
2. Add it to the appropriate map in `action-icons.ts` or `resource-icons.ts`.
3. Re-export the `IconName` union via `index.ts`.
4. Use `<Icon name="yourKey" />` in components.

> Tip: keep icon keys lowercase, dash-separated (`pod`, `deployments`, `node-pools`, `helm`, `prod-warning`).
