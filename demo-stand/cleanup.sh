#!/bin/bash

CLUSTER_NAME="crystal-demo"

echo "🗑️ Cleaning up Crystal Demo..."

if kind get clusters | grep -q "^$CLUSTER_NAME$"; then
    kind delete cluster --name "$CLUSTER_NAME"
    echo "✅ Cluster '$CLUSTER_NAME' deleted."
else
    echo "ℹ️ Cluster '$CLUSTER_NAME' not found. Nothing to do."
fi
