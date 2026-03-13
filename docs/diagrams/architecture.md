# Telescope Architecture Diagrams

These standalone diagrams summarize the current Telescope architecture across desktop and web modes. They are grounded in the current code layout in `apps/web`, `apps/desktop`, `apps/hub`, `crates/engine`, and `crates/core`.

## 1. System Context Diagram

This diagram shows the two primary user entry points: the Tauri desktop application and the browser-based web experience. Both runtime paths ultimately rely on the shared Rust engine and core crates to reach the Kubernetes API.

```mermaid
flowchart TB
    userDesktop["User"]
    userWeb["User"]
    k8s["Kubernetes API"]

    subgraph desktopMode["Desktop mode"]
        desktopApp["Desktop App<br/>Tauri v2 shell"]
        rustBackend["Rust Backend<br/>apps/desktop/src-tauri"]
    end

    subgraph webMode["Web mode"]
        browser["Browser"]
        webUi["SvelteKit Frontend<br/>apps/web"]
        hub["Axum Hub<br/>apps/hub"]
    end

    subgraph sharedRust["Shared Rust crates"]
        engine["crates/engine<br/>watchers · logs · exec · port-forward · Helm · metrics · CRDs · secrets · actions · audit"]
        core["crates/core<br/>connection state · ResourceStore · ResourceEntry"]
    end

    userDesktop --> desktopApp --> rustBackend
    rustBackend --> engine
    rustBackend --> core

    userWeb --> browser --> webUi --> hub
    hub --> engine
    hub --> core

    engine --> core
    engine --> k8s
```

## 2. Component Diagram

This diagram breaks the repository into the main application and crate boundaries, then highlights the important modules inside each subsystem and the main dependency directions between them.

```mermaid
flowchart LR
    subgraph web["apps/web · shared SvelteKit frontend"]
        routes["routes/"]
        components["lib/components/"]
        api["lib/api.ts"]
        stores["lib/stores.ts + lib/stores/*"]
        webBuild["build output"]

        routes --> components
        routes --> api
        routes --> stores
        components --> api
        components --> stores
        routes --> webBuild
        components --> webBuild
        api --> webBuild
        stores --> webBuild
    end

    subgraph desktop["apps/desktop · Tauri shell"]
        prepare["scripts/prepare-frontend.mjs<br/>build + copy apps/web"]
        tauriMain["src-tauri/src/main.rs<br/>commands + AppState"]
    end

    subgraph hubApp["apps/hub · Axum service"]
        hubMain["main.rs<br/>router + bootstrap"]
        hubRoutes["routes.rs<br/>HTTP handlers"]
        hubAuth["auth.rs<br/>auth scaffolding"]
        hubWs["ws.rs<br/>WebSocket entrypoint"]
        hubState["state.rs<br/>HubState + store handle"]

        hubMain --> hubRoutes
        hubMain --> hubAuth
        hubMain --> hubWs
        hubRoutes --> hubState
        hubWs --> hubState
    end

    subgraph engineCrate["crates/engine"]
        client["client"]
        watcher["watcher"]
        logs["logs"]
        exec["exec"]
        portforward["portforward"]
        actions["actions"]
        helm["helm"]
        metrics["metrics"]
        crd["crd"]
        secrets["secrets"]
        audit["audit"]
        kubeconfig["kubeconfig"]

        watcher --> client
        logs --> client
        exec --> client
        portforward --> client
        actions --> client
        helm --> client
        metrics --> client
        crd --> client
        secrets --> client
    end

    subgraph coreCrate["crates/core"]
        connection["connection<br/>state machine"]
        resourceStore["store<br/>ResourceStore / SQLite"]
        resourceEntry["store<br/>ResourceEntry"]

        resourceStore --> resourceEntry
    end

    prepare -. packages .-> webBuild
    api -. desktop IPC .-> tauriMain
    api -. web HTTP / WS .-> hubRoutes

    tauriMain --> client
    tauriMain --> kubeconfig
    tauriMain --> resourceStore
    tauriMain --> connection

    hubRoutes --> client
    hubRoutes --> resourceStore
    hubRoutes --> connection
    hubAuth --> hubRoutes
    hubState --> resourceStore
    connection --> hubState
    watcher --> resourceStore
```

## 3. Data Flow Diagram

This diagram shows how a typical user action travels from the shared UI through either the desktop IPC path or the hub HTTP path, and how watcher-driven synchronization keeps cached resource data current.

```mermaid
sequenceDiagram
    participant User
    participant UI as apps/web UI
    participant API as lib/api.ts
    participant Desktop as Tauri IPC
    participant Hub as Axum Hub
    participant Engine as engine function
    participant Kube as kube-rs
    participant Store as ResourceStore
    participant K8s as Kubernetes API

    User->>UI: Trigger action in the interface
    UI->>API: Call frontend helper

    alt Desktop mode
        API->>Desktop: invoke(command, args)
        Desktop->>Engine: Run Tauri command handler
    else Web mode
        API->>Hub: HTTP or WebSocket request
        Hub->>Engine: Run route handler
    end

    Engine->>Kube: Execute list / get / action / watch
    Kube->>K8s: Kubernetes client request
    K8s-->>Kube: Objects, status, or stream events
    Kube-->>Engine: Typed response
    Engine->>Store: upsert / delete ResourceEntry

    alt Desktop response
        Engine-->>Desktop: Result or event
        Desktop-->>API: IPC payload
    else Web response
        Engine-->>Hub: JSON or WS payload
        Hub-->>API: HTTP / WS payload
    end

    API-->>UI: Update stores and components

    rect rgb(235, 245, 255)
        Note over Engine,Store: Watcher-driven cache refresh
        Engine->>Kube: Start ResourceWatcher
        Kube->>K8s: LIST + WATCH
        loop For each resource event
            K8s-->>Kube: ADDED / MODIFIED / DELETED
            Kube-->>Engine: Watch event
            Engine->>Store: upsert / delete ResourceEntry
            opt UI refresh signal
                Store-->>UI: UI polls cache or receives event-triggered refresh
            end
        end
    end
```

## 4. Desktop vs Web Mode Comparison

This side-by-side comparison highlights where the two runtime modes differ in transport, storage location, engine placement, and currently available feature surface.

```mermaid
flowchart LR
    subgraph desktopCompare["Desktop mode"]
        d1["Transport<br/>Tauri IPC"] --> d2["Storage<br/>local SQLite ResourceStore"] --> d3["Execution<br/>direct local engine access"] --> d4["Feature surface<br/>full command set and desktop-first features"]
    end

    subgraph webCompare["Web mode"]
        w1["Transport<br/>HTTP + WebSocket to Hub"] --> w2["Storage<br/>hub-side SQLite ResourceStore"] --> w3["Execution<br/>server-side engine in apps/hub"] --> w4["Feature surface<br/>full CRUD, Helm, exec, port-forward, metrics; log streaming still TODO"]
    end
```

## 5. Deployment Architecture

This diagram shows the two deployment shapes supported today: a local desktop binary running on a user workstation and a browser-based deployment backed by the hub service running in Docker or Kubernetes.

```mermaid
flowchart LR
    aks["AKS cluster / Kubernetes API"]

    subgraph desktopDeploy["Desktop deployment"]
        machine["User machine"] --> tauriBinary["Tauri binary"] --> kubeconfig["Local kubeconfig"]
    end

    subgraph webDeploy["Web deployment"]
        browserClient["Browser"] --> hubRuntime["Hub service<br/>Docker or Kubernetes"] --> hubCreds["Hub kubeconfig or ServiceAccount"]
    end

    kubeconfig --> aks
    hubCreds --> aks
```
