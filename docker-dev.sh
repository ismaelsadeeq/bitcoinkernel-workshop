#!/usr/bin/env bash

# Docker Auto-cleanup and Refresh Script
# Usage: ./docker-dev.sh [image-name]

IMAGE_NAME="${1:-workshop}"
CONTAINER_NAME="${IMAGE_NAME}-dev"

echo "=== Docker Development Environment ==="
echo "Image: $IMAGE_NAME"
echo "======================================"

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "Cleaning up..."
    
    # Stop and remove any containers using this image
    echo "Stopping containers..."
    docker ps -a --filter "ancestor=$IMAGE_NAME" --format "{{.ID}}" | xargs -r docker stop 2>/dev/null
    docker ps -a --filter "ancestor=$IMAGE_NAME" --format "{{.ID}}" | xargs -r docker rm 2>/dev/null
    
    # Also try to remove by container name if it exists
    docker stop "$CONTAINER_NAME" 2>/dev/null
    docker rm "$CONTAINER_NAME" 2>/dev/null
    
    # Remove the image
    echo "Removing image..."
    docker rmi -f "$IMAGE_NAME" 2>/dev/null
    
    echo "Cleanup complete!"
}

# Set trap to cleanup on script exit
trap cleanup EXIT

# Build the image
echo "Building image..."
if ! docker build -t "$IMAGE_NAME" .; then
    echo "Build failed!"
    exit 1
fi

echo ""
echo "Starting container..."
echo "Press Ctrl+D or type 'exit' to quit and cleanup"
echo ""

# Run the container with auto-remove flag
docker run -it --rm \
    --name "$CONTAINER_NAME" \
    -v "$(pwd):/workspace" \
    "$IMAGE_NAME"

