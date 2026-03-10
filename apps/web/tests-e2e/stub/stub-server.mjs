import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

function json(res, status, body) {
  const payload = JSON.stringify(body);
  res.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store',
    'content-length': Buffer.byteLength(payload)
  });
  res.end(payload);
}

function text(res, status, body) {
  res.writeHead(status, {
    'content-type': 'text/plain; charset=utf-8',
    'cache-control': 'no-store'
  });
  res.end(body);
}

async function loadClustersFixture() {
  const fixtureUrl = new URL('../fixtures/clusters.json', import.meta.url);
  const raw = await readFile(fileURLToPath(fixtureUrl), 'utf-8');
  return JSON.parse(raw);
}

const port = Number.parseInt(process.env.STUB_PORT ?? '4174', 10);

const server = http.createServer(async (req, res) => {
  try {
    const url = new URL(req.url ?? '/', `http://${req.headers.host ?? '127.0.0.1'}`);

    if (req.method === 'GET' && url.pathname === '/healthz') {
      return text(res, 200, 'ok');
    }

    if (req.method === 'GET' && url.pathname === '/api/clusters') {
      const fixture = await loadClustersFixture();
      return json(res, 200, fixture);
    }

    // M1 explorer endpoints (deterministic)
    if (req.method === 'GET' && url.pathname === '/api/kinds') {
      return json(res, 200, {
        kinds: [
          { kind: 'Pods', namespaced: true },
          { kind: 'Deployments', namespaced: true },
          { kind: 'Services', namespaced: true },
          { kind: 'ConfigMaps', namespaced: true },
          { kind: 'Nodes', namespaced: false },
          { kind: 'Namespaces', namespaced: false }
        ]
      });
    }

    const nsMatch = url.pathname.match(/^\/api\/clusters\/([^/]+)\/namespaces$/);
    if (req.method === 'GET' && nsMatch) {
      return json(res, 200, { namespaces: [{ name: 'default' }, { name: 'kube-system' }] });
    }

    if (req.method === 'GET' && url.pathname === '/api/resources') {
      const kind = url.searchParams.get('kind') ?? '';
      const namespace = url.searchParams.get('namespace') ?? 'default';

      const items =
        kind === 'Nodes'
          ? [{ name: 'node-1', status: 'Ready', age: '1d' }]
          : kind === 'Deployments'
            ? [{ name: 'api', namespace, status: 'Available', age: '3h' }]
            : [{ name: 'api-7d9', namespace, status: 'Running', age: '12m' }];

      return json(res, 200, { items });
    }

    return text(res, 404, 'not found');
  } catch (err) {
    return text(res, 500, `stub server error: ${err instanceof Error ? err.message : String(err)}`);
  }
});

server.listen(port, '127.0.0.1', () => {
  // eslint-disable-next-line no-console
  console.log(`[stub] listening on http://127.0.0.1:${port}`);
});
