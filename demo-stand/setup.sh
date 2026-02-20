#!/bin/bash

set -e

CLUSTER_NAME="kubetile-demo"

echo "🚀 Starting KubeTile Demo Setup..."

# Check if kind is installed
if ! command -v kind &> /dev/null; then
    echo "❌ Error: 'kind' is not installed."
    echo "Please install it from https://kind.sigs.k8s.io/"
    exit 1
fi

# Check if kubectl is installed
if ! command -v kubectl &> /dev/null; then
    echo "❌ Error: 'kubectl' is not installed."
    echo "Please install it from https://kubernetes.io/docs/tasks/tools/"
    exit 1
fi

# Create cluster if it doesn't exist
if kind get clusters | grep -q "^$CLUSTER_NAME$"; then
    echo "ℹ️ Cluster '$CLUSTER_NAME' already exists. Skipping creation."
else
    echo "🏗️ Creating kind cluster '$CLUSTER_NAME'..."
    kind create cluster --name "$CLUSTER_NAME"
fi

# Set context
echo "🎯 Switching to context 'kind-$CLUSTER_NAME'..."
kubectl config use-context "kind-$CLUSTER_NAME"

# Apply manifests
echo "📦 Applying manifests..."
kubectl apply -f manifests/

echo ""
echo "✅ Setup complete!"
echo "You can now run 'kubetile' to explore the cluster."
echo "Use './cleanup.sh' to remove the cluster when finished."
