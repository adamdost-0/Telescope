import type { ComponentType } from 'svelte';
import {
  IconActivity,
  IconAnchor,
  IconArchive,
  IconBox,
  IconBoxMultiple,
  IconChartLine,
  IconChartPie,
  IconClipboardList,
  IconCompass,
  IconDatabase,
  IconDoorEnter,
  IconFolders,
  IconInfinity,
  IconLink,
  IconListDetails,
  IconPuzzle,
  IconRulerMeasure,
  IconRuler,
  IconServer,
  IconServerBolt,
  IconServerCog,
  IconShieldCheck,
  IconShieldLock,
  IconShield,
  IconTags,
  IconTopologyStar3,
  IconWorld,
  IconClock,
  IconPlus,
  IconRocket,
  IconSettings,
} from '@tabler/icons-svelte';

/**
 * Resource icon keys are shared across Sidebar, SearchPalette, Overview cards, etc.
 * Keep keys lower-case and dash-separated to match routes.
 */
export type ResourceIconName =
  | 'overview'
  | 'namespaces'
  | 'create'
  | 'nodes'
  | 'node-pools'
  | 'priorityclasses'
  | 'events'
  | 'pods'
  | 'deployments'
  | 'statefulsets'
  | 'daemonsets'
  | 'replicasets'
  | 'jobs'
  | 'cronjobs'
  | 'hpas'
  | 'poddisruptionbudgets'
  | 'services'
  | 'ingresses'
  | 'networkpolicies'
  | 'endpointslices'
  | 'configmaps'
  | 'secrets'
  | 'resourcequotas'
  | 'limitranges'
  | 'pvcs'
  | 'persistentvolumes'
  | 'storageclasses'
  | 'helm'
  | 'crds'
  | 'clusterroles'
  | 'clusterrolebindings'
  | 'settings'
  | 'addons'
  | 'auth'
  | 'default';

export const RESOURCE_ICONS: Record<ResourceIconName, ComponentType> = {
  overview: IconChartPie,
  namespaces: IconFolders,
  create: IconPlus,
  nodes: IconServer,
  'node-pools': IconServerBolt,
  priorityclasses: IconTags,
  events: IconActivity,
  pods: IconBox,
  deployments: IconRocket,
  statefulsets: IconArchive,
  daemonsets: IconInfinity,
  replicasets: IconTopologyStar3,
  jobs: IconServerCog,
  cronjobs: IconClock,
  hpas: IconChartLine,
  poddisruptionbudgets: IconShield,
  services: IconWorld,
  ingresses: IconDoorEnter,
  networkpolicies: IconShieldLock,
  endpointslices: IconCompass,
  configmaps: IconClipboardList,
  secrets: IconShieldCheck,
  resourcequotas: IconRulerMeasure,
  limitranges: IconRuler,
  pvcs: IconDatabase,
  persistentvolumes: IconArchive,
  storageclasses: IconTags,
  helm: IconAnchor,
  crds: IconPuzzle,
  clusterroles: IconListDetails,
  clusterrolebindings: IconLink,
  settings: IconSettings,
  addons: IconPuzzle,
  auth: IconShieldLock,
  default: IconBoxMultiple,
};


export function getResourceIcon(name: ResourceIconName | string): ComponentType {
  const key = name as ResourceIconName;
  return RESOURCE_ICONS[key] ?? RESOURCE_ICONS.default;
}
