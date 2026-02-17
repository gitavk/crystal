# Crystal Demo Stand

This directory contains scripts and manifests to set up a local Kubernetes cluster for demonstrating **Crystal**.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [kind](https://kind.sigs.k8s.io/) (Kubernetes in Docker)
- [kubectl](https://kubernetes.io/docs/tasks/tools/)

## Getting Started

1. **Set up the cluster:**
   Run the setup script to create a `kind` cluster and deploy sample resources.
   ```bash
   ./setup.sh
   ```

2. **Run Crystal:**
   Navigate back to the project root and run Crystal (ensure you've built it first).
   ```bash
   cargo run
   ```

3. **Explore the Demo:**
   The setup script creates several resources to showcase Crystal's features:
   - **Namespaces:** `crystal-prod`, `crystal-staging`, `crystal-dev`.
   - **Multi-pane navigation:** Try splitting panes and viewing different namespaces.
   - **Logs & Exec:**
     - Find the `noisy-pod` in `crystal-dev` to view streaming logs.
     - Exec into one of the `frontend` pods in `crystal-prod`.
   - **Debugging:**
     - Check the `crashing-pod` in `crystal-dev` to see CrashLoopBackOff states.
     - View the `pending-pod` (it has impossible resource requests) to see Pending state.
   - **Jobs & CronJobs:** View the `nightly-cleanup` CronJob and `database-migration` Job in `crystal-prod`.
   - **Port Forwarding:** Try port-forwarding to the `frontend` service on port 80.

## Cleanup

When you're finished with the demo, you can delete the cluster:
```bash
./cleanup.sh
```

## Sample Resources Included

- **Deployments:** `frontend` (3 replicas), `backend` (2 replicas), `redis` (1 replica).
- **Services:** `frontend`, `backend`.
- **Config & Secrets:** `app-config`, `app-secret`.
- **Batch:** `nightly-cleanup` (CronJob), `database-migration` (Job).
- **Troubleshooting:** `crashing-pod`, `pending-pod`, `noisy-pod`.
