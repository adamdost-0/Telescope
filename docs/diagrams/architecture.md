# Telescope Architecture Diagrams

These standalone diagrams summarize the current Telescope desktop architecture. They are grounded in the current code layout in `apps/web`, `apps/desktop`, `crates/engine`, and `crates/core`.

## 1. System Context Diagram

This diagram shows the primary user entry point: the Tauri desktop application, which packages the frontend and connects to the shared Rust engine to reach the Kubernetes API.

```mermaid
flowchart TB
    userDesktop["User"]
    k8s["Kubernetes API"]

    subgraph desktopMode["Desktop application"]
        desktopApp["Desktop App<br/>Tauri v2 shell"]
        webUi["SvelteKit Frontend<br/>apps/web"]
        rustBackend["Rust Backend<br/>apps/desktop/src-tauri"]
    end

    subgraph sharedRust["Shared Rust crates"]
        engine["crates/engine<br/>watchers · logs · exec · port-forward · Helm · metrics · CRDs · secrets · actions · audit"]
        core["crates/core<br/>connection state · ResourceStore · ResourceEntry"]
    end

    userDesktop --> desktopApp --> webUi
    desktopApp --> rustBackend
    rustBackend --> engine
    rustBackend --> core
    engine --> core
    engine --> k8s
```

## 2. Component Diagram

This diagram breaks the repository into the main application and crate boundaries, then highlights the important modules inside each subsystem and the main dependency directions between them.

```mermaid
flowchart LR
    subgraph web["apps/web · desktop frontend"]
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

    tauriMain --> client
    tauriMain --> kubeconfig
    tauriMain --> resourceStore
    tauriMain --> connection
    watcher --> resourceStore
```

## 3. Data Flow Diagram

This diagram shows how a typical user action travels from the packaged frontend through the Tauri IPC path, and how watcher-driven synchronization keeps cached resource data current.

```mermaid
sequenceDiagram
    participant User
    participant UI as apps/web UI
    participant API as lib/api.ts
    participant Desktop as Tauri IPC
    participant Engine as engine function
    participant Kube as kube-rs
    participant Store as ResourceStore
    participant K8s as Kubernetes API

    User->>UI: Trigger action in the interface
    UI->>API: Call frontend helper
    API->>Desktop: invoke(command, args)
    Desktop->>Engine: Run Tauri command handler

    Engine->>Kube: Execute list / get / action / watch
    Kube->>K8s: Kubernetes client request
    K8s-->>Kube: Objects, status, or stream events
    Kube-->>Engine: Typed response
    Engine->>Store: upsert / delete ResourceEntry
    Engine-->>Desktop: Result or event
    Desktop-->>API: IPC payload
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

## 4. Desktop Deployment Architecture

This diagram shows the supported deployment shape today: a local desktop binary running on a user workstation with access to local kubeconfig.

```mermaid
flowchart LR
    aks["AKS cluster / Kubernetes API"]

    subgraph desktopDeploy["Desktop deployment"]
        machine["User machine"] --> tauriBinary["Tauri binary"] --> kubeconfig["Local kubeconfig"]
    end

    kubeconfig --> aks
```
