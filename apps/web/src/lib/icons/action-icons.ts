import type { ComponentType } from 'svelte';
import {
  IconAlertCircle,
  IconAlertTriangle,
  IconArrowBackUp,
  IconArrowsRightLeft,
  IconBulb,
  IconCertificate,
  IconCheck,
  IconClipboard,
  IconFileAlert,
  IconFileInfo,
  IconFlask,
  IconKey,
  IconLock,
  IconLockOpen,
  IconPlugOff,
  IconPlus,
  IconRefresh,
  IconRestore,
  IconScale,
  IconSettings,
  IconShieldCheck,
  IconTerminal2,
  IconTrash,
  IconEye,
  IconEyeOff,
  IconPalette,
  IconSearch,
} from '@tabler/icons-svelte';

/**
 * UI action icon keys. Add new keys here instead of sprinkling icons across components.
 */
export type ActionIconName =
  | 'dry-run'
  | 'apply'
  | 'reset'
  | 'create'
  | 'reload'
  | 'settings'
  | 'prod-warning'
  | 'error'
  | 'suggestion'
  | 'helm-lock'
  | 'helm-unlock'
  | 'auth-exec'
  | 'auth-token'
  | 'auth-cert'
  | 'auth-oidc'
  | 'diagnostic-warning'
  | 'diagnostic-info'
  | 'rollback'
  | 'show-values'
  | 'hide-values'
  | 'theme'
  | 'search'
  | 'disconnect'
  | 'copy'
  | 'delete'
  | 'scale'
  | 'port-forward';

export const ACTION_ICONS: Record<ActionIconName, ComponentType> = {
  'dry-run': IconFlask,
  apply: IconCheck,
  reset: IconArrowBackUp,
  create: IconPlus,
  reload: IconRefresh,
  settings: IconSettings,
  'prod-warning': IconAlertTriangle,
  error: IconAlertCircle,
  suggestion: IconBulb,
  'helm-lock': IconLock,
  'helm-unlock': IconLockOpen,
  'auth-exec': IconTerminal2,
  'auth-token': IconKey,
  'auth-cert': IconCertificate,
  'auth-oidc': IconShieldCheck,
  'diagnostic-warning': IconFileAlert,
  'diagnostic-info': IconFileInfo,
  rollback: IconRestore,
  'show-values': IconEye,
  'hide-values': IconEyeOff,
  theme: IconPalette,
  search: IconSearch,
  disconnect: IconPlugOff,
  copy: IconClipboard,
  delete: IconTrash,
  scale: IconScale,
  'port-forward': IconArrowsRightLeft,
};

export function getActionIcon(name: ActionIconName): ComponentType {
  return ACTION_ICONS[name];
}
