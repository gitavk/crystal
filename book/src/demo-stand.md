# Demo Stand

The KubeTile repository includes a **Demo Stand**—a set of scripts and manifests to quickly spin up a local [kind](https://kind.sigs.k8s.io/) (Kubernetes in Docker) cluster. This is the fastest way to explore KubeTile's features in a controlled environment.

## Prerequisites

To use the demo stand, you need the following installed:
- **Docker**
- **kind**
- **kubectl**

## Getting Started

1. **Set up the cluster**:
   Run the setup script from the project root. This creates a `kind` cluster named `kubetile-demo` and deploys sample resources.
   ```bash
   ./demo-stand/setup.sh
   ```

2. **Run KubeTile**:
   Once the cluster is ready, start KubeTile:
   ```bash
   cargo run
   ```

3. **Explore the Demo**:
   The setup script creates several resources designed to showcase specific features:
   - **Namespaces**: Explore `kubetile-prod`, `kubetile-staging`, and `kubetile-dev`.
   - **Logs & Exec**: 
     - Find the `noisy-pod` in `kubetile-dev` to test log streaming.
     - Exec into one of the `frontend` pods in `kubetile-prod`.
   - **Debugging**:
     - Check the `crashing-pod` in `kubetile-dev` to see `CrashLoopBackOff` states.
     - View the `pending-pod` to see how KubeTile displays resources that cannot be scheduled.
   - **Port Forwarding**: Try port-forwarding to the `frontend` service in `kubetile-prod`.

## Cleanup

When you're finished, you can delete the local cluster and all its resources:
```bash
./demo-stand/cleanup.sh
```

## Included Resources

The demo environment includes a variety of standard Kubernetes objects:
- **Deployments & Services**: A typical frontend/backend/redis stack.
- **ConfigMaps & Secrets**: For testing resource inspection and YAML views.
- **Jobs & CronJobs**: To observe batch resource lifecycles.
- **Error Cases**: Intentionally misconfigured pods to test troubleshooting workflows.
