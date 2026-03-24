import type { ComponentType } from 'svelte';
import { ACTION_ICONS, type ActionIconName, getActionIcon } from './action-icons';
import { RESOURCE_ICONS, type ResourceIconName, getResourceIcon } from './resource-icons';

export type IconName = ActionIconName | ResourceIconName;

const REGISTRY: Record<IconName, ComponentType> = {
  ...ACTION_ICONS,
  ...RESOURCE_ICONS,
};

export function getIcon(name: IconName | string): ComponentType | undefined {
  // try strict match first
  if (name in REGISTRY) return REGISTRY[name as IconName];

  // fallback: map resource icons via helper
  const resourceFallback = getResourceIcon(name);
  if (resourceFallback) return resourceFallback;

  return undefined;
}

export { ACTION_ICONS, RESOURCE_ICONS, getActionIcon, getResourceIcon };
export type { ActionIconName, ResourceIconName };
