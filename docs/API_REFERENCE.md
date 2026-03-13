# API Reference

> **Current status:** This documents the Hub API implemented in `apps/hub/src/main.rs`, `apps/hub/src/routes.rs`, `apps/hub/src/auth.rs`, and `apps/hub/src/ws.rs`. The Hub is functional, but auth remains scaffold-only: when OIDC is enabled it decodes bearer-token claims without signature validation, and `/auth/login` plus `/auth/callback` still return `501 Not Implemented`.

## Base URL and auth model

- **Base URL:** `http://<host>:<port>` (default port: `3001`)
- **Health and WebSocket:** `/healthz` and `/ws`
- **REST API prefix:** `/api/v1`
- **Auth routes:** `/auth/*`

### Authentication behavior

- When `OIDC_ENABLED=false` (the default), API requests run as the anonymous user:
  ```json
  {
    "email": "anonymous@local",
    "name": "Anonymous",
    "groups": []
  }
  ```
- When `OIDC_ENABLED=true`, every `/api/v1/*` route requires `Authorization: Bearer <jwt>`.
- `GET /auth/me` also requires a bearer token when OIDC is enabled.
- `GET /healthz`, `GET /auth/login`, `GET /auth/callback`, `POST /auth/logout`, and `GET /ws` do not use auth middleware.

### Common error format

Most route handlers that return structured errors use this JSON body:

```json
{
  "error": "human-readable message"
}
```

Some handlers intentionally return `200 OK` with an empty array on read failures instead of returning an error payload. Those cases are called out below.

## Common JSON shapes

### ClusterContext

Returned by `GET /api/v1/contexts`.

```json
{
  "name": "dev-aks",
  "cluster_server": "https://example.azmk8s.io:443",
  "namespace": "default",
  "is_active": true,
  "auth_type": "exec"
}
```

### AuthUser

Returned by `GET /auth/me`.

```json
{
  "email": "alice@example.com",
  "name": "Alice Example",
  "groups": ["admins", "devs"]
}
```

### ResourceEntry

Used by cached resource, pod, event, and secret endpoints.

```json
{
  "gvk": "v1/Pod",
  "namespace": "default",
  "name": "nginx-7b6d5f6d8c-abcde",
  "resource_version": "104937",
  "content": "{\"apiVersion\":\"v1\",\"kind\":\"Pod\",\"metadata\":{\"name\":\"nginx-7b6d5f6d8c-abcde\"}}",
  "updated_at": "2025-03-13T12:34:56.000Z"
}
```

Notes:

- `content` is a **JSON string**, not a nested JSON object.
- Secret endpoints also use `ResourceEntry`, but their `content` payload has secret values redacted to `●●●●●●●●`.

### ConnectionState

Returned by `GET /api/v1/connection-state`.

The enum is serialized with a `state` tag and an optional `detail` object.

```json
{"state":"Disconnected"}
```

```json
{
  "state": "Syncing",
  "detail": {
    "resources_synced": 3,
    "resources_total": 12
  }
}
```

Other variants currently include `Connecting`, `Ready`, `Degraded`, `Error`, and `Backoff`.

### ClusterInfo

Returned by `GET /api/v1/cluster-info`.

```json
{
  "server_version": "1.30",
  "platform": "linux/amd64",
  "server_url": "https://example.azmk8s.io:443",
  "auth_type": "exec",
  "exec_plugin": "kubelogin",
  "is_aks": true,
  "auth_hint": "Authenticated via Azure Entra ID (kubelogin)"
}
```

### HelmRelease

Returned by `GET /api/v1/helm/releases`.

```json
{
  "name": "ingress-nginx",
  "namespace": "ingress-nginx",
  "chart": "ingress-nginx-4.10.0",
  "app_version": "1.10.0",
  "revision": 7,
  "status": "deployed",
  "updated": "2025-03-13T11:22:33Z"
}
```

### PodMetrics

Returned by `GET /api/v1/metrics/pods`.

```json
{
  "name": "api-7db9db4d6d-rd9hh",
  "namespace": "default",
  "containers": [
    {
      "name": "api",
      "cpu_millicores": 12,
      "memory_bytes": 50331648
    }
  ],
  "cpu_millicores": 12,
  "memory_bytes": 50331648
}
```

### CrdInfo

Returned by `GET /api/v1/crds`.

```json
{
  "name": "certificates.cert-manager.io",
  "group": "cert-manager.io",
  "kind": "Certificate",
  "version": "v1",
  "scope": "Namespaced",
  "plural": "certificates",
  "short_names": ["cert", "certs"]
}
```

### Audit entry

Returned by `GET /api/v1/audit` as generic JSON values parsed from the audit log.

```json
{
  "timestamp": "2025-03-13T12:00:00Z",
  "actor": "alice@example.com",
  "context": "dev-aks",
  "namespace": "default",
  "action": "connect",
  "resource_type": "context",
  "resource_name": "dev-aks",
  "result": "success",
  "detail": null
}
```

## Endpoints

### GET /healthz

- **Description:** Simple unauthenticated liveness check.
- **Auth required:** No
- **Query parameters:** None
- **Request body:** None
- **Response format:** Plain text `ok`

**Example request**

```http
GET /healthz HTTP/1.1
Host: localhost:3001
```

**Example response**

```http
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8

ok
```

---

### GET /api/v1/contexts

- **Description:** Lists kubeconfig contexts from the Hub runtime environment.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:** None
- **Request body:** None
- **Response format:** `ClusterContext[]`
- **Implementation note:** On kubeconfig read failure, the handler logs the error and returns `200 OK` with `[]`.

**Example request**

```http
GET /api/v1/contexts HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
[
  {
    "name": "dev-aks",
    "cluster_server": "https://example.azmk8s.io:443",
    "namespace": "default",
    "is_active": true,
    "auth_type": "exec"
  },
  {
    "name": "prod-aks",
    "cluster_server": "https://prod.example.azmk8s.io:443",
    "namespace": "production",
    "is_active": false,
    "auth_type": "exec"
  }
]
```

---

### POST /api/v1/connect

- **Description:** Clears cached watched resources, creates a Kubernetes client for the requested context, starts background watch tasks, and marks the Hub as connected.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:** None
- **Request body:**
  - `context_name` (string, required)
  - `contextName` is also accepted as an alias
- **Response format:**
  ```json
  {
    "status": "connected",
    "context": "<context_name>",
    "namespace": "<active namespace>"
  }
  ```
- **Possible error responses:**
  - `403` if `user_can_access_cluster` denies access
  - `400` if the context client cannot be created
  - `500` if the resource store lock fails

**Example request**

```http
POST /api/v1/connect HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
Content-Type: application/json

{"context_name":"dev-aks"}
```

**Example response**

```json
{
  "status": "connected",
  "context": "dev-aks",
  "namespace": "default"
}
```

---

### POST /api/v1/disconnect

- **Description:** Aborts the background watch task, clears cached watched resources, clears the active context, and sets the connection state to `Disconnected`.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:** None
- **Request body:** None
- **Response format:**
  ```json
  {"status":"disconnected"}
  ```
- **Possible error responses:** `500` if the resource store lock fails

**Example request**

```http
POST /api/v1/disconnect HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
{"status":"disconnected"}
```

---

### GET /api/v1/connection-state

- **Description:** Returns the Hub's current connection lifecycle state.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:** None
- **Request body:** None
- **Response format:** `ConnectionState`

**Example request**

```http
GET /api/v1/connection-state HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
{"state":"Ready"}
```

---

### GET /api/v1/resources

- **Description:** Returns cached resources for a specific GVK, optionally filtered by namespace. This reads from the Hub's local SQLite-backed cache populated by background watchers.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:**
  - `gvk` (string, required)
  - `namespace` (string, optional)
- **Request body:** None
- **Response format:** `ResourceEntry[]`
- **Implementation note:** On store lookup failure, the handler logs the error and returns `200 OK` with `[]`.

**Example request**

```http
GET /api/v1/resources?gvk=apps/v1/Deployment&namespace=default HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
[
  {
    "gvk": "apps/v1/Deployment",
    "namespace": "default",
    "name": "api",
    "resource_version": "58122",
    "content": "{\"apiVersion\":\"apps/v1\",\"kind\":\"Deployment\",\"metadata\":{\"name\":\"api\",\"namespace\":\"default\"}}",
    "updated_at": "2025-03-13T12:34:56.000Z"
  }
]
```

---

### GET /api/v1/secrets

- **Description:** Lists secrets live from Kubernetes for the requested namespace. Secret values are redacted before they are returned.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:**
  - `namespace` (string, optional). If omitted or empty, the Hub uses its active namespace (default: `default`).
- **Request body:** None
- **Response format:** `ResourceEntry[]` with redacted secret data in the `content` JSON string
- **Possible error responses:**
  - `400` when the Hub is not connected to a context
  - `500` when the live Kubernetes call fails

**Example request**

```http
GET /api/v1/secrets?namespace=default HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
[
  {
    "gvk": "v1/Secret",
    "namespace": "default",
    "name": "db-creds",
    "resource_version": "7",
    "content": "{\"metadata\":{\"name\":\"db-creds\",\"namespace\":\"default\"},\"data\":{\"username\":\"●●●●●●●●\",\"password\":\"●●●●●●●●\"},\"type\":\"Opaque\"}",
    "updated_at": ""
  }
]
```

---

### GET /api/v1/secrets/:namespace/:name

- **Description:** Returns one secret by namespace and name. Secret values are redacted before the resource is serialized.
- **Auth required:** Yes, when OIDC is enabled
- **Path parameters:**
  - `namespace` (string, required)
  - `name` (string, required)
- **Query parameters:** None
- **Request body:** None
- **Response format:** `ResourceEntry | null`
- **Possible error responses:**
  - `400` when the Hub is not connected to a context
  - `500` when the live Kubernetes call fails

**Example request**

```http
GET /api/v1/secrets/default/db-creds HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
{
  "gvk": "v1/Secret",
  "namespace": "default",
  "name": "db-creds",
  "resource_version": "7",
  "content": "{\"metadata\":{\"name\":\"db-creds\",\"namespace\":\"default\"},\"data\":{\"username\":\"●●●●●●●●\",\"password\":\"●●●●●●●●\"},\"type\":\"Opaque\"}",
  "updated_at": ""
}
```

---

### GET /api/v1/pods

- **Description:** Returns cached pod resources from the Hub store, optionally filtered by namespace.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:**
  - `namespace` (string, optional)
- **Request body:** None
- **Response format:** `ResourceEntry[]`
- **Implementation note:** On store lookup failure, the handler logs the error and returns `200 OK` with `[]`.

**Example request**

```http
GET /api/v1/pods?namespace=default HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
[
  {
    "gvk": "v1/Pod",
    "namespace": "default",
    "name": "api-7db9db4d6d-rd9hh",
    "resource_version": "104937",
    "content": "{\"apiVersion\":\"v1\",\"kind\":\"Pod\",\"metadata\":{\"name\":\"api-7db9db4d6d-rd9hh\",\"namespace\":\"default\"}}",
    "updated_at": "2025-03-13T12:34:56.000Z"
  }
]
```

---

### GET /api/v1/events

- **Description:** Returns cached Kubernetes events. If `involved_object` is set, the handler filters the cached event JSON by looking for the string fragment `"name":"<value>"` inside `ResourceEntry.content`.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:**
  - `namespace` (string, optional)
  - `involved_object` (string, optional)
- **Request body:** None
- **Response format:** `ResourceEntry[]`

**Example request**

```http
GET /api/v1/events?namespace=default&involved_object=api-7db9db4d6d-rd9hh HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
[
  {
    "gvk": "v1/Event",
    "namespace": "default",
    "name": "api-7db9db4d6d-rd9hh.18a9fb0c08b1dfb9",
    "resource_version": "2123",
    "content": "{\"apiVersion\":\"v1\",\"kind\":\"Event\",\"involvedObject\":{\"name\":\"api-7db9db4d6d-rd9hh\"},\"reason\":\"Pulled\"}",
    "updated_at": "2025-03-13T12:35:01.000Z"
  }
]
```

---

### GET /api/v1/namespaces

- **Description:** Lists namespaces.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:** None
- **Request body:** None
- **Response format:** `string[]`
- **Special behavior:** Requires an active cluster connection. Returns an error if no context is active.
- **Possible error responses:** `500` when no active context exists or when the namespace query fails

**Example request**

```http
GET /api/v1/namespaces HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
["default", "kube-system", "ingress-nginx"]
```

---

### GET /api/v1/pods/:namespace/:name/logs

- **Description:** Fetches pod logs and returns them as a single string field.
- **Auth required:** Yes, when OIDC is enabled
- **Path parameters:**
  - `namespace` (string, required)
  - `name` (string, required)
- **Query parameters:**
  - `container` (string, optional)
  - `tail` (integer, optional, defaults to `1000`)
  - `previous` (boolean, optional, defaults to `false`)
- **Request body:** None
- **Response format:**
  ```json
  {"logs":"<pod log text>"}
  ```
- **Possible error responses:** `500` when client creation or log retrieval fails

**Example request**

```http
GET /api/v1/pods/default/api-7db9db4d6d-rd9hh/logs?container=api&tail=200 HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
{
  "logs": "2025-03-13T12:00:00Z starting server\n2025-03-13T12:00:01Z listening on :8080\n"
}
```

---

### GET /api/v1/cluster-info

- **Description:** Returns cluster version, server URL, and auth metadata for the currently connected context.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:** None
- **Request body:** None
- **Response format:** `ClusterInfo`
- **Possible error responses:**
  - `400` when the Hub is not connected
  - `500` when the Kubernetes client or version query fails

**Example request**

```http
GET /api/v1/cluster-info HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
{
  "server_version": "1.30",
  "platform": "linux/amd64",
  "server_url": "https://example.azmk8s.io:443",
  "auth_type": "exec",
  "exec_plugin": "kubelogin",
  "is_aks": true,
  "auth_hint": "Authenticated via Azure Entra ID (kubelogin)"
}
```

---

### GET /api/v1/search

- **Description:** Searches the Hub's cached watched resources by lowercased resource name or GVK.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:**
  - `q` (string, required)
- **Request body:** None
- **Response format:** `ResourceEntry[]`
- **Behavior notes:**
  - Searches only the watched GVK list stored by the Hub
  - Returns at most `20` matches
  - Requires cached data from a prior connection to be useful

**Example request**

```http
GET /api/v1/search?q=deploy HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
[
  {
    "gvk": "apps/v1/Deployment",
    "namespace": "default",
    "name": "api",
    "resource_version": "58122",
    "content": "{\"apiVersion\":\"apps/v1\",\"kind\":\"Deployment\",\"metadata\":{\"name\":\"api\"}}",
    "updated_at": "2025-03-13T12:34:56.000Z"
  }
]
```

---

### GET /api/v1/helm/releases

- **Description:** Lists Helm releases by reading Helm-owned Kubernetes Secrets.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:**
  - `namespace` (string, optional). If omitted, all namespaces are scanned.
- **Request body:** None
- **Response format:** `HelmRelease[]`
- **Possible error responses:** `500` when the Kubernetes client or Helm secret query fails

**Example request**

```http
GET /api/v1/helm/releases?namespace=ingress-nginx HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
[
  {
    "name": "ingress-nginx",
    "namespace": "ingress-nginx",
    "chart": "ingress-nginx-4.10.0",
    "app_version": "1.10.0",
    "revision": 7,
    "status": "deployed",
    "updated": "2025-03-13T11:22:33Z"
  }
]
```

---

### GET /api/v1/metrics/pods

- **Description:** Returns pod CPU and memory usage from the Kubernetes metrics API.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:**
  - `namespace` (string, optional). If omitted, all namespaces are queried.
- **Request body:** None
- **Response format:** `PodMetrics[]`
- **Possible error responses:** `500` when the Kubernetes client or metrics API call fails

**Example request**

```http
GET /api/v1/metrics/pods?namespace=default HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
[
  {
    "name": "api-7db9db4d6d-rd9hh",
    "namespace": "default",
    "containers": [
      {
        "name": "api",
        "cpu_millicores": 12,
        "memory_bytes": 50331648
      }
    ],
    "cpu_millicores": 12,
    "memory_bytes": 50331648
  }
]
```

---

### GET /api/v1/crds

- **Description:** Lists installed CustomResourceDefinitions from the cluster.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:** None
- **Request body:** None
- **Response format:** `CrdInfo[]`
- **Possible error responses:** `500` when the Kubernetes client or CRD query fails

**Example request**

```http
GET /api/v1/crds HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
[
  {
    "name": "certificates.cert-manager.io",
    "group": "cert-manager.io",
    "kind": "Certificate",
    "version": "v1",
    "scope": "Namespaced",
    "plural": "certificates",
    "short_names": ["cert", "certs"]
  }
]
```

---

### GET /api/v1/active-context

- **Description:** Returns the name of the currently active kubeconfig context.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:** None
- **Response format:** `string | null`

---

### POST /api/v1/namespace

- **Description:** Sets the active namespace for the current hub session.
- **Auth required:** Yes, when OIDC is enabled
- **Request body:** `{ "namespace": "string" }`
- **Response format:** `204 No Content` on success
- **Possible error responses:** `400` if namespace is missing

---

### GET /api/v1/resource-counts

- **Description:** Returns counts of cached resources grouped by GVK.
- **Auth required:** Yes, when OIDC is enabled
- **Response format:** `[[string, number]]` (array of `[gvk, count]` pairs)

---

### POST /api/v1/resources/delete

- **Description:** Deletes a namespaced Kubernetes resource.
- **Auth required:** Yes, when OIDC is enabled
- **Request body:** `{ "gvk": "string", "namespace": "string", "name": "string" }`
- **Response format:** `{ "success": bool, "message": "string" }`

---

### POST /api/v1/resources/apply

- **Description:** Applies a JSON/YAML manifest to the cluster.
- **Auth required:** Yes, when OIDC is enabled
- **Request body:** `{ "manifest": "string", "dry_run": bool }`
- **Response format:** `{ "success": bool, "message": "string" }`

---

### POST /api/v1/resources/scale

- **Description:** Scales a Deployment or StatefulSet.
- **Auth required:** Yes, when OIDC is enabled
- **Request body:** `{ "gvk": "string", "namespace": "string", "name": "string", "replicas": number }`
- **Response format:** `{ "message": "string" }`

---

### POST /api/v1/rollout/restart

- **Description:** Triggers a rollout restart on a Deployment, StatefulSet, or DaemonSet.
- **Auth required:** Yes, when OIDC is enabled
- **Request body:** `{ "gvk": "string", "namespace": "string", "name": "string" }`
- **Response format:** `{ "message": "string" }`

---

### GET /api/v1/rollout/status

- **Description:** Returns rollout status for a Deployment or StatefulSet.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:** `gvk`, `namespace`, `name`
- **Response format:** `RolloutStatus` object

---

### POST /api/v1/exec

- **Description:** Executes a non-interactive command in a container.
- **Auth required:** Yes, when OIDC is enabled
- **Request body:** `{ "namespace": "string", "pod": "string", "container": "string?", "command": ["string"] }`
- **Response format:** `{ "stdout": "string", "stderr": "string", "exit_code": number }`

---

### POST /api/v1/port-forward

- **Description:** Starts port-forwarding to a pod.
- **Auth required:** Yes, when OIDC is enabled
- **Request body:** `{ "namespace": "string", "pod": "string", "local_port": number, "remote_port": number }`
- **Response format:** `{ "local_port": number }`

---

### GET /api/v1/containers/{namespace}/{pod}

- **Description:** Lists container names in a pod.
- **Auth required:** Yes, when OIDC is enabled
- **Response format:** `string[]`

---

### GET /api/v1/helm/releases/{namespace}/{name}/history

- **Description:** Returns revision history for a Helm release.
- **Auth required:** Yes, when OIDC is enabled
- **Response format:** `HelmRelease[]`

---

### GET /api/v1/helm/releases/{namespace}/{name}/values

- **Description:** Returns the values for a Helm release. Sensitive values are redacted unless `reveal=true`.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:** `reveal` (bool, optional, default `false`)
- **Response format:** `{ "values": "string" }` (YAML)

---

### POST /api/v1/helm/releases/{namespace}/{name}/rollback

- **Description:** Rolls back a Helm release to a specific revision.
- **Auth required:** Yes, when OIDC is enabled
- **Request body:** `{ "revision": number }`
- **Response format:** `{ "message": "string" }`

---

### GET /api/v1/metrics/nodes

- **Description:** Returns CPU and memory metrics for all nodes.
- **Auth required:** Yes, when OIDC is enabled
- **Response format:** `NodeMetricsData[]`

---

### GET /api/v1/metrics/available

- **Description:** Checks whether the metrics-server API is available on the cluster.
- **Auth required:** Yes, when OIDC is enabled
- **Response format:** `{ "available": bool }`

---

### GET /api/v1/preferences/{key}

- **Description:** Reads a user preference value from the store.
- **Auth required:** Yes, when OIDC is enabled
- **Response format:** `{ "value": "string | null" }`

---

### PUT /api/v1/preferences/{key}

- **Description:** Writes a user preference value to the store.
- **Auth required:** Yes, when OIDC is enabled
- **Request body:** `{ "value": "string" }`
- **Response format:** `204 No Content`

---

### GET /api/v1/audit

- **Description:** Reads the audit log file, parses JSON lines, and returns the most recent entries first.
- **Auth required:** Yes, when OIDC is enabled
- **Query parameters:**
  - `limit` (integer, optional, defaults to `100`)
- **Request body:** None
- **Response format:** `AuditEntry[]` represented as generic JSON values
- **Behavior notes:**
  - Reads from the file path configured by `AUDIT_PATH`
  - Invalid JSON lines are skipped
  - Missing files produce an empty array

**Example request**

```http
GET /api/v1/audit?limit=2 HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
[
  {
    "timestamp": "2025-03-13T12:05:00Z",
    "actor": "alice@example.com",
    "context": "dev-aks",
    "namespace": "default",
    "action": "disconnect",
    "resource_type": "context",
    "resource_name": "dev-aks",
    "result": "success",
    "detail": null
  },
  {
    "timestamp": "2025-03-13T12:00:00Z",
    "actor": "alice@example.com",
    "context": "dev-aks",
    "namespace": "default",
    "action": "connect",
    "resource_type": "context",
    "resource_name": "dev-aks",
    "result": "success",
    "detail": null
  }
]
```

---

### GET /ws

- **Description:** Upgrades the connection to a WebSocket.
- **Auth required:** No
- **Query parameters:** None
- **Request body:** None
- **Response format:** WebSocket text frames
- **Current behavior:**
  - Sends an initial welcome frame
  - Enters a simple echo loop
  - On incoming text, sends `{"echo":<original text>}` as a text frame

**Example request**

```http
GET /ws HTTP/1.1
Host: localhost:3001
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: <key>
Sec-WebSocket-Version: 13
```

**Example frames**

Server welcome frame:

```json
{"type":"welcome","message":"Telescope Hub WebSocket"}
```

If the client sends the text frame:

```json
"ping"
```

The server responds with:

```json
{"echo":"ping"}
```

---

### GET /auth/login

- **Description:** Placeholder login endpoint for future OIDC browser redirects.
- **Auth required:** No
- **Query parameters:** None
- **Request body:** None
- **Response format:** Plain text
- **Current response:** `501 Not Implemented`

**Example request**

```http
GET /auth/login HTTP/1.1
Host: localhost:3001
```

**Example response**

```http
HTTP/1.1 501 Not Implemented
content-type: text/plain; charset=utf-8

OIDC login not yet configured. Set OIDC_ENABLED=true with issuer/client config.
```

---

### GET /auth/callback

- **Description:** Placeholder OIDC callback endpoint.
- **Auth required:** No
- **Query parameters:** None
- **Request body:** None
- **Response format:** Plain text
- **Current response:** `501 Not Implemented`

**Example request**

```http
GET /auth/callback HTTP/1.1
Host: localhost:3001
```

**Example response**

```http
HTTP/1.1 501 Not Implemented
content-type: text/plain; charset=utf-8

OIDC callback placeholder
```

---

### POST /auth/logout

- **Description:** Logout placeholder.
- **Auth required:** No
- **Query parameters:** None
- **Request body:** None
- **Response format:** Plain text `Logged out`

**Example request**

```http
POST /auth/logout HTTP/1.1
Host: localhost:3001
```

**Example response**

```http
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8

Logged out
```

---

### GET /auth/me

- **Description:** Returns the current authenticated identity extracted by auth middleware.
- **Auth required:** Yes, when OIDC is enabled; otherwise returns the anonymous local user
- **Query parameters:** None
- **Request body:** None
- **Response format:** `AuthUser`
- **Possible error responses:** `401` when OIDC is enabled and the bearer token is missing or cannot be decoded

**Example request**

```http
GET /auth/me HTTP/1.1
Host: localhost:3001
Authorization: Bearer <jwt>
```

**Example response**

```json
{
  "email": "alice@example.com",
  "name": "Alice Example",
  "groups": ["admins", "devs"]
}
```
