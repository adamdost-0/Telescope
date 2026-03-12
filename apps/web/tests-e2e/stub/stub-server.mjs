import http from 'node:http';

const corsHeaders = {
  'access-control-allow-origin': '*',
  'access-control-allow-methods': 'GET,POST,OPTIONS',
  'access-control-allow-headers': 'content-type'
};

function json(res, status, body) {
  const payload = JSON.stringify(body);
  res.writeHead(status, {
    ...corsHeaders,
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store',
    'content-length': Buffer.byteLength(payload)
  });
  res.end(payload);
}

function text(res, status, body) {
  res.writeHead(status, {
    ...corsHeaders,
    'content-type': 'text/plain; charset=utf-8',
    'cache-control': 'no-store'
  });
  res.end(body);
}

async function readJson(req) {
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  if (chunks.length === 0) return null;
  return JSON.parse(Buffer.concat(chunks).toString('utf-8'));
}

function resourceEntry(gvk, namespace, name, resource) {
  return {
    gvk,
    namespace,
    name,
    resource_version: '1',
    updated_at: '2025-03-12T08:00:00.000Z',
    content: JSON.stringify(resource)
  };
}

function eventEntry(namespace, name, event) {
  return {
    gvk: 'v1/Event',
    namespace,
    name,
    resource_version: '1',
    updated_at: event.lastTimestamp ?? event.firstTimestamp ?? '2025-03-12T08:00:00.000Z',
    content: JSON.stringify(event)
  };
}

const contexts = [
  {
    name: 'Local Dev Cluster',
    cluster_server: 'https://local-dev-cluster.example.invalid',
    namespace: 'default',
    is_active: true,
    auth_type: 'token'
  },
  {
    name: 'Staging AKS',
    cluster_server: 'https://staging-aks.hcp.westeurope.azmk8s.io:443',
    namespace: 'default',
    is_active: false,
    auth_type: 'exec'
  }
];

const namespaces = ['default', 'kube-system'];

const podFixtures = [
  resourceEntry('v1/Pod', 'default', 'telescope-api-7f6c9d4b7b-abcde', {
    metadata: {
      name: 'telescope-api-7f6c9d4b7b-abcde',
      namespace: 'default',
      creationTimestamp: '2025-01-01T00:00:00.000Z',
      labels: {
        app: 'telescope-api'
      }
    },
    spec: {
      containers: [
        { name: 'api', image: 'ghcr.io/aksoftcode/telescope-api:dev' }
      ]
    },
    status: {
      phase: 'Running',
      containerStatuses: [
        {
          name: 'api',
          ready: true,
          restartCount: 0,
          state: {
            running: { startedAt: '2025-01-01T00:00:10.000Z' }
          }
        }
      ]
    }
  }),
  resourceEntry('v1/Pod', 'default', 'orders-db-0', {
    metadata: {
      name: 'orders-db-0',
      namespace: 'default',
      creationTimestamp: '2025-03-11T01:55:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'orders-db',
        'app.kubernetes.io/component': 'database'
      }
    },
    spec: {
      containers: [
        { name: 'postgres', image: 'mcr.microsoft.com/oss/bitnami/postgresql:16.4.0' }
      ]
    },
    status: {
      phase: 'Running',
      containerStatuses: [
        {
          name: 'postgres',
          ready: true,
          restartCount: 0,
          state: {
            running: { startedAt: '2025-03-11T01:55:30.000Z' }
          }
        }
      ]
    }
  }),
  resourceEntry('v1/Pod', 'default', 'orders-db-1', {
    metadata: {
      name: 'orders-db-1',
      namespace: 'default',
      creationTimestamp: '2025-03-11T01:55:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'orders-db',
        'app.kubernetes.io/component': 'database'
      }
    },
    spec: {
      containers: [
        { name: 'postgres', image: 'mcr.microsoft.com/oss/bitnami/postgresql:16.4.0' }
      ]
    },
    status: {
      phase: 'Running',
      containerStatuses: [
        {
          name: 'postgres',
          ready: true,
          restartCount: 1,
          state: {
            running: { startedAt: '2025-03-11T01:56:00.000Z' }
          }
        }
      ]
    }
  }),
  resourceEntry('v1/Pod', 'default', 'orders-db-2', {
    metadata: {
      name: 'orders-db-2',
      namespace: 'default',
      creationTimestamp: '2025-03-11T01:55:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'orders-db',
        'app.kubernetes.io/component': 'database'
      }
    },
    spec: {
      containers: [
        { name: 'postgres', image: 'mcr.microsoft.com/oss/bitnami/postgresql:16.4.0' }
      ]
    },
    status: {
      phase: 'Running',
      containerStatuses: [
        {
          name: 'postgres',
          ready: true,
          restartCount: 0,
          state: {
            running: { startedAt: '2025-03-11T01:55:45.000Z' }
          }
        }
      ]
    }
  }),
  resourceEntry('v1/Pod', 'default', 'ama-metrics-node-vmss000001', {
    metadata: {
      name: 'ama-metrics-node-vmss000001',
      namespace: 'default',
      creationTimestamp: '2025-03-11T01:40:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'ama-metrics-node'
      }
    },
    spec: {
      containers: [
        { name: 'ama-metrics', image: 'mcr.microsoft.com/azuremonitor/containerinsights/ciprod:3.1.18' }
      ]
    },
    status: {
      phase: 'Running',
      containerStatuses: [
        {
          name: 'ama-metrics',
          ready: true,
          restartCount: 0,
          state: {
            running: { startedAt: '2025-03-11T01:40:20.000Z' }
          }
        }
      ]
    }
  }),
  resourceEntry('v1/Pod', 'default', 'ama-metrics-node-vmss000002', {
    metadata: {
      name: 'ama-metrics-node-vmss000002',
      namespace: 'default',
      creationTimestamp: '2025-03-11T01:40:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'ama-metrics-node'
      }
    },
    spec: {
      containers: [
        { name: 'ama-metrics', image: 'mcr.microsoft.com/azuremonitor/containerinsights/ciprod:3.1.18' }
      ]
    },
    status: {
      phase: 'Running',
      containerStatuses: [
        {
          name: 'ama-metrics',
          ready: true,
          restartCount: 0,
          state: {
            running: { startedAt: '2025-03-11T01:40:35.000Z' }
          }
        }
      ]
    }
  })
];

const podMetrics = [
  {
    name: 'telescope-api-7f6c9d4b7b-abcde',
    namespace: 'default',
    containers: [
      {
        name: 'api',
        cpu_millicores: 35,
        memory_bytes: 73400320
      }
    ],
    cpu_millicores: 35,
    memory_bytes: 73400320
  },
  {
    name: 'orders-db-0',
    namespace: 'default',
    containers: [
      {
        name: 'postgres',
        cpu_millicores: 112,
        memory_bytes: 268435456
      }
    ],
    cpu_millicores: 112,
    memory_bytes: 268435456
  }
];

const resourceFixtures = [
  resourceEntry('apps/v1/StatefulSet', 'default', 'orders-db', {
    metadata: {
      name: 'orders-db',
      namespace: 'default',
      creationTimestamp: '2025-03-11T01:50:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'orders-db',
        'app.kubernetes.io/part-of': 'checkout-platform'
      },
      annotations: {
        'azure.workload.identity/use': 'true'
      }
    },
    spec: {
      replicas: 3,
      serviceName: 'orders-db-headless',
      podManagementPolicy: 'OrderedReady',
      updateStrategy: { type: 'RollingUpdate' },
      selector: {
        matchLabels: {
          'app.kubernetes.io/name': 'orders-db',
          'app.kubernetes.io/component': 'database'
        }
      },
      template: {
        metadata: {
          labels: {
            'app.kubernetes.io/name': 'orders-db',
            'app.kubernetes.io/component': 'database'
          },
          annotations: {
            'azure.workload.identity/client-id': '11111111-2222-3333-4444-555555555555'
          }
        },
        spec: {
          serviceAccountName: 'orders-db',
          containers: [
            {
              name: 'postgres',
              image: 'mcr.microsoft.com/oss/bitnami/postgresql:16.4.0'
            }
          ]
        }
      },
      volumeClaimTemplates: [
        {
          metadata: { name: 'data' },
          spec: {
            accessModes: ['ReadWriteOnce'],
            resources: {
              requests: {
                storage: '128Gi'
              }
            }
          }
        }
      ]
    },
    status: {
      readyReplicas: 3,
      currentReplicas: 3,
      updatedReplicas: 3
    }
  }),
  resourceEntry('apps/v1/StatefulSet', 'default', 'session-store', {
    metadata: {
      name: 'session-store',
      namespace: 'default',
      creationTimestamp: '2025-03-10T18:30:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'session-store',
        'app.kubernetes.io/part-of': 'checkout-platform'
      }
    },
    spec: {
      replicas: 2,
      serviceName: 'session-store-headless',
      podManagementPolicy: 'Parallel',
      updateStrategy: { type: 'RollingUpdate' },
      selector: {
        matchLabels: {
          'app.kubernetes.io/name': 'session-store'
        }
      },
      template: {
        metadata: {
          labels: {
            'app.kubernetes.io/name': 'session-store'
          }
        },
        spec: {
          containers: [
            {
              name: 'redis',
              image: 'mcr.microsoft.com/oss/bitnami/redis:7.2.5'
            }
          ]
        }
      }
    },
    status: {
      readyReplicas: 1,
      currentReplicas: 2,
      updatedReplicas: 2
    }
  }),
  resourceEntry('apps/v1/DaemonSet', 'default', 'ama-metrics-node', {
    metadata: {
      name: 'ama-metrics-node',
      namespace: 'default',
      creationTimestamp: '2025-03-11T01:35:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'ama-metrics-node',
        'app.kubernetes.io/part-of': 'aks-observability'
      }
    },
    spec: {
      updateStrategy: { type: 'RollingUpdate' },
      selector: {
        matchLabels: {
          'app.kubernetes.io/name': 'ama-metrics-node'
        }
      },
      template: {
        metadata: {
          labels: {
            'app.kubernetes.io/name': 'ama-metrics-node'
          }
        },
        spec: {
          containers: [
            {
              name: 'ama-metrics',
              image: 'mcr.microsoft.com/azuremonitor/containerinsights/ciprod:3.1.18'
            }
          ]
        }
      }
    },
    status: {
      desiredNumberScheduled: 5,
      currentNumberScheduled: 5,
      updatedNumberScheduled: 5,
      numberReady: 5,
      numberAvailable: 5
    }
  }),
  resourceEntry('apps/v1/DaemonSet', 'default', 'csi-secrets-store-provider-azure', {
    metadata: {
      name: 'csi-secrets-store-provider-azure',
      namespace: 'default',
      creationTimestamp: '2025-03-10T22:00:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'csi-secrets-store-provider-azure',
        'app.kubernetes.io/part-of': 'aks-platform'
      }
    },
    spec: {
      updateStrategy: { type: 'RollingUpdate' },
      selector: {
        matchLabels: {
          'app.kubernetes.io/name': 'csi-secrets-store-provider-azure'
        }
      },
      template: {
        metadata: {
          labels: {
            'app.kubernetes.io/name': 'csi-secrets-store-provider-azure'
          }
        },
        spec: {
          containers: [
            {
              name: 'provider-azure',
              image: 'mcr.microsoft.com/oss/azure/secrets-store/provider-azure:1.5.2'
            }
          ]
        }
      }
    },
    status: {
      desiredNumberScheduled: 5,
      currentNumberScheduled: 5,
      updatedNumberScheduled: 4,
      numberReady: 4,
      numberAvailable: 4
    }
  }),
  resourceEntry('batch/v1/Job', 'default', 'nightly-ledger-close-28903412', {
    metadata: {
      name: 'nightly-ledger-close-28903412',
      namespace: 'default',
      creationTimestamp: '2025-03-11T02:00:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'nightly-ledger-close',
        'batch.kubernetes.io/job-name': 'nightly-ledger-close-28903412'
      }
    },
    spec: {
      completions: 1,
      parallelism: 1,
      backoffLimit: 2
    },
    status: {
      active: 0,
      succeeded: 1,
      failed: 0,
      startTime: '2025-03-11T02:00:00.000Z',
      completionTime: '2025-03-11T02:03:00.000Z'
    }
  }),
  resourceEntry('batch/v1/Job', 'default', 'sales-export-backfill-20250311', {
    metadata: {
      name: 'sales-export-backfill-20250311',
      namespace: 'default',
      creationTimestamp: '2025-03-11T03:00:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'sales-export-backfill',
        'batch.kubernetes.io/job-name': 'sales-export-backfill-20250311'
      }
    },
    spec: {
      completions: 3,
      parallelism: 1,
      backoffLimit: 4
    },
    status: {
      active: 0,
      succeeded: 1,
      failed: 1,
      startTime: '2025-03-11T03:00:00.000Z'
    }
  }),
  resourceEntry('batch/v1/CronJob', 'default', 'nightly-image-prune', {
    metadata: {
      name: 'nightly-image-prune',
      namespace: 'default',
      creationTimestamp: '2025-03-01T02:00:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'nightly-image-prune',
        'app.kubernetes.io/part-of': 'aks-maintenance'
      }
    },
    spec: {
      schedule: '0 2 * * *',
      suspend: false,
      concurrencyPolicy: 'Forbid'
    },
    status: {
      active: [
        {
          name: 'nightly-image-prune-28903412',
          namespace: 'default'
        }
      ],
      lastScheduleTime: '2025-03-11T02:00:00.000Z'
    }
  }),
  resourceEntry('batch/v1/CronJob', 'default', 'weekly-cost-report', {
    metadata: {
      name: 'weekly-cost-report',
      namespace: 'default',
      creationTimestamp: '2025-03-03T06:30:00.000Z',
      labels: {
        'app.kubernetes.io/name': 'weekly-cost-report',
        'app.kubernetes.io/part-of': 'finops'
      }
    },
    spec: {
      schedule: '30 6 * * 1',
      suspend: true,
      concurrencyPolicy: 'Forbid'
    },
    status: {
      active: [],
      lastScheduleTime: '2025-03-10T06:30:00.000Z'
    }
  })
];

const eventFixtures = [
  eventEntry('default', 'orders-db.1874a7a0f6fd5d1b', {
    metadata: {
      name: 'orders-db.1874a7a0f6fd5d1b',
      namespace: 'default',
      creationTimestamp: '2025-03-11T02:04:00.000Z'
    },
    involvedObject: {
      kind: 'StatefulSet',
      name: 'orders-db',
      namespace: 'default'
    },
    type: 'Normal',
    reason: 'SuccessfulCreate',
    message: 'create Pod orders-db-2 in StatefulSet orders-db successful',
    count: 1,
    firstTimestamp: '2025-03-11T02:04:00.000Z',
    lastTimestamp: '2025-03-11T02:04:00.000Z'
  }),
  eventEntry('default', 'ama-metrics-node.1874a7b51efc5bb5', {
    metadata: {
      name: 'ama-metrics-node.1874a7b51efc5bb5',
      namespace: 'default',
      creationTimestamp: '2025-03-11T02:10:00.000Z'
    },
    involvedObject: {
      kind: 'DaemonSet',
      name: 'ama-metrics-node',
      namespace: 'default'
    },
    type: 'Normal',
    reason: 'RollingUpdate',
    message: 'DaemonSet has successfully progressed the rollout',
    count: 2,
    firstTimestamp: '2025-03-11T02:10:00.000Z',
    lastTimestamp: '2025-03-11T02:12:00.000Z'
  }),
  eventEntry('default', 'nightly-ledger-close-28903412.1874a7c0bb8f8d3e', {
    metadata: {
      name: 'nightly-ledger-close-28903412.1874a7c0bb8f8d3e',
      namespace: 'default',
      creationTimestamp: '2025-03-11T02:03:00.000Z'
    },
    involvedObject: {
      kind: 'Job',
      name: 'nightly-ledger-close-28903412',
      namespace: 'default'
    },
    type: 'Normal',
    reason: 'Completed',
    message: 'Job completed successfully',
    count: 1,
    firstTimestamp: '2025-03-11T02:03:00.000Z',
    lastTimestamp: '2025-03-11T02:03:00.000Z'
  }),
  eventEntry('default', 'nightly-image-prune.1874a7d65d2a18c9', {
    metadata: {
      name: 'nightly-image-prune.1874a7d65d2a18c9',
      namespace: 'default',
      creationTimestamp: '2025-03-11T02:00:30.000Z'
    },
    involvedObject: {
      kind: 'CronJob',
      name: 'nightly-image-prune',
      namespace: 'default'
    },
    type: 'Normal',
    reason: 'SawCompletedJob',
    message: 'Created Job nightly-image-prune-28903412',
    count: 1,
    firstTimestamp: '2025-03-11T02:00:30.000Z',
    lastTimestamp: '2025-03-11T02:00:30.000Z'
  })
];

const state = {
  connectedContext: 'Local Dev Cluster'
};

function currentConnectionState() {
  if (!state.connectedContext) {
    return { state: 'Disconnected' };
  }

  return { state: 'Ready' };
}

const port = Number.parseInt(process.env.STUB_PORT ?? '4174', 10);

const server = http.createServer(async (req, res) => {
  try {
    const url = new URL(req.url ?? '/', `http://${req.headers.host ?? '127.0.0.1'}`);

    if (req.method === 'OPTIONS') {
      res.writeHead(204, corsHeaders);
      res.end();
      return;
    }

    if (req.method === 'GET' && url.pathname === '/healthz') {
      return text(res, 200, 'ok');
    }

    if (req.method === 'GET' && url.pathname === '/api/v1/contexts') {
      return json(res, 200, contexts);
    }

    if (req.method === 'POST' && url.pathname === '/api/v1/connect') {
      const body = await readJson(req);
      const contextName = body?.contextName ?? null;
      const match = contexts.find((context) => context.name === contextName);

      if (!match) {
        return json(res, 404, { message: `Unknown context: ${contextName}` });
      }

      state.connectedContext = match.name;
      return json(res, 200, { ok: true });
    }

    if (req.method === 'POST' && url.pathname === '/api/v1/disconnect') {
      state.connectedContext = null;
      return json(res, 200, { ok: true });
    }

    if (req.method === 'GET' && url.pathname === '/api/v1/connection-state') {
      return json(res, 200, currentConnectionState());
    }

    if (req.method === 'GET' && url.pathname === '/api/v1/namespaces') {
      return json(res, 200, state.connectedContext ? namespaces : []);
    }

    if (req.method === 'GET' && url.pathname === '/api/v1/pods') {
      const namespace = url.searchParams.get('namespace');
      const pods = !state.connectedContext
        ? []
        : namespace
          ? podFixtures.filter((pod) => pod.namespace === namespace)
          : podFixtures;
      return json(res, 200, pods);
    }

    if (req.method === 'GET' && url.pathname === '/api/v1/resources') {
      const gvk = url.searchParams.get('gvk');
      const namespace = url.searchParams.get('namespace');
      let resources = state.connectedContext ? resourceFixtures : [];

      if (gvk) {
        resources = resources.filter((resource) => resource.gvk === gvk);
      }

      if (namespace) {
        resources = resources.filter((resource) => resource.namespace === namespace);
      }

      return json(res, 200, resources);
    }

    if (req.method === 'GET' && url.pathname === '/api/v1/events') {
      const namespace = url.searchParams.get('namespace');
      const involvedObject = url.searchParams.get('involved_object');
      let events = state.connectedContext ? eventFixtures : [];

      if (namespace) {
        events = events.filter((event) => event.namespace === namespace);
      }

      if (involvedObject) {
        events = events.filter((event) => {
          try {
            return JSON.parse(event.content).involvedObject?.name === involvedObject;
          } catch {
            return false;
          }
        });
      }

      return json(res, 200, events);
    }

    if (req.method === 'GET' && url.pathname === '/api/v1/metrics/pods') {
      const namespace = url.searchParams.get('namespace');
      const metrics = !state.connectedContext
        ? []
        : namespace
          ? podMetrics.filter((metric) => metric.namespace === namespace)
          : podMetrics;
      return json(res, 200, metrics);
    }

    if (req.method === 'GET' && url.pathname === '/api/v1/search') {
      const query = url.searchParams.get('q')?.toLowerCase() ?? '';
      const results = !state.connectedContext || !query
        ? []
        : [...podFixtures, ...resourceFixtures]
            .filter((entry) => entry.name.toLowerCase().includes(query) || entry.gvk.toLowerCase().includes(query))
            .slice(0, 20);
      return json(res, 200, results);
    }

    return text(res, 404, 'not found');
  } catch (err) {
    return text(res, 500, `stub server error: ${err instanceof Error ? err.message : String(err)}`);
  }
});

server.listen(port, '127.0.0.1', () => {
  console.log(`[stub] listening on http://127.0.0.1:${port}`);
});
