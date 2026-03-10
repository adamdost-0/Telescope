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

    return text(res, 404, 'not found');
  } catch (err) {
    return text(res, 500, `stub server error: ${err instanceof Error ? err.message : String(err)}`);
  }
});

server.listen(port, '127.0.0.1', () => {
  // eslint-disable-next-line no-console
  console.log(`[stub] listening on http://127.0.0.1:${port}`);
});
