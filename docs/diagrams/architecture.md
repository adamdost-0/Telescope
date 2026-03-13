# Telescope Architecture Diagrams

These standalone diagrams summarize the current Telescope v1.0.0 desktop architecture. They are grounded in the current code layout in `apps/web`, `apps/desktop`, `crates/engine`, `crates/azure`, and `crates/core`.

## 1. System Context Diagram

The Tauri desktop application packages the frontend and connects to shared Rust crates for both Kubernetes API access and Azure ARM management.

```mermaid
flowchart TB
    userDesktop["User"]
    k8s["Kubernetes API"]
    arm["Azure Resource Manager"]

    subgraph desktopMode["Desktop application"]
        desktopApp["Desktop App<br/>Tauri v2 shell"]
        webUi["SvelteKit Frontend<br/>apps/web · 25 components · 39 routes"]
        rustBackend["Rust Backend<br/>apps/desktop/src-tauri · 66 commands"]
    end

    subgraph sharedRust["Shared Rust crates"]
        engine["crates/engine<br/>watchers · logs · exec · port-forward · Helm · metrics · CRDs · secrets · actions · audit · node-ops"]
        azure["crates/azure<br/>ArmClient · AKS operations · identity resolution"]
        core["crates/core<br/>connection state · ResourceStore · ResourceEntry"]
    end

    userDesktop --> desktopApp --> webUi
    desktopApp --> rustBackend
    rustBackend --> engine
    rustBackend --> azure
    rustBackend --> core
    engine --> core
    azure --> core
    engine --> k8s
    azure --> arm
```

## 2. Component Diagram

This diagram breaks the repository into application and crate boundaries, showing the modules and dependency directions.

```mermaid
flowchart LR
    subgraph web["apps/web · desktop frontend"]
        routes["routes/ (39 pages)"]
        components["lib/components/ (25)"]
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
        tauriMain["src-tauri/src/main.rs<br/>66 commands + AppState"]
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
        nodeOps["node_ops"]
        dynamic["dynamic"]

        watcher --> client
        logs --> client
        exec --> client
        portforward --> client
        actions --> client
        helm --> client
        metrics --> client
        crd --> client
        secrets --> client
        nodeOps --> client
        dynamic --> client
    end

    subgraph azureCrate["crates/azure"]
        armClient["client<br/>ArmClient · DefaultAzureCredential"]
        aks["aks<br/>cluster + node pool operations"]
        resolve["resolve<br/>identity resolution"]
        azureTypes["types<br/>AzureCloud · AksResourceId"]

        aks --> armClient
        resolve --> azureTypes
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
    tauriMain --> armClient
    tauriMain --> resourceStore
    tauriMain --> connection
    watcher --> resourceStore
```

## 3. Data Flow Diagram

Shows how a typical user action travels through the Tauri IPC path.

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

## 4. Azure ARM Flow

Shows how AKS management operations flow through the Azure ARM crate, separate from the Kubernetes API path.

```mermaid
sequenceDiagram
    participant User
    participant UI as apps/web UI
    participant Desktop as Tauri IPC
    participant Azure as crates/azure
    participant ARM as Azure Resource Manager

    User->>UI: AKS management action
    UI->>Desktop: invoke(aks_command, args)
    Desktop->>Azure: ArmClient operation
    Azure->>Azure: DefaultAzureCredential → token
    Azure->>ARM: REST API call (GET/PUT/POST/DELETE)
    ARM-->>Azure: Response / provisioning state

    opt Long-running operation
        loop Poll until complete (15s interval)
            Azure->>ARM: GET resource status
            ARM-->>Azure: provisioningState
        end
    end

    Azure-->>Desktop: Result
    Desktop-->>UI: AKS data
```

## 5. Desktop Deployment Architecture

Desktop-only: a local Tauri binary with access to kubeconfig for Kubernetes and Azure credentials for ARM.

```mermaid
flowchart LR
    aks["AKS Cluster /<br/>Kubernetes API"]
    arm["Azure Resource<br/>Manager"]

    subgraph desktopDeploy["Desktop deployment"]
        machine["User machine"]
        tauriBinary["Tauri binary"]
        kubeconfig["Local kubeconfig"]
        azureCreds["Azure credentials<br/>(az CLI / env vars /<br/>managed identity)"]
    end

    machine --> tauriBinary
    tauriBinary --> kubeconfig
    tauriBinary --> azureCreds
    kubeconfig --> aks
    azureCreds --> arm
```
