<script lang="ts">
  import type { ComponentType } from 'svelte';
  import { getIcon, type IconName } from '$lib/icons';

  const props = $props<{
    name?: IconName | string;
    component?: ComponentType;
    size?: number | string;
    stroke?: number;
    class?: string;
    ariaLabel?: string;
  }>();

  const iconName = $derived(() => props.name);
  const iconComponent = $derived(() => props.component ?? (iconName ? getIcon(iconName) : undefined));
const ResolvedIcon = $derived(() => iconComponent);
const size = $derived(() => props.size ?? 18);
const stroke = $derived(() => props.stroke ?? 1.75);
const className = $derived(() => props.class ?? '');
const ariaLabel = $derived(() => props.ariaLabel);
const ariaHidden = $derived(() => (ariaLabel ? undefined : true));
</script>

{#if ResolvedIcon}
  <ResolvedIcon
    class={`icon ${className}`}
    size={size}
    stroke={stroke}
    aria-label={ariaLabel}
    aria-hidden={ariaHidden}
    focusable="false"
    role={ariaLabel ? 'img' : undefined}
  />
{:else}
  {#if name}
    <span class={`icon icon-fallback ${className}`} aria-label={ariaLabel ?? name} aria-hidden={ariaHidden}>{name}</span>
  {:else}
    <span class={`icon icon-fallback ${className}`} aria-hidden="true">?</span>
  {/if}
{/if}

<style>
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    vertical-align: middle;
    line-height: 1;
    color: inherit;
  }
  .icon :global(svg) {
    display: block;
  }
  .icon-fallback {
    font-size: 0.95em;
  }
</style>
