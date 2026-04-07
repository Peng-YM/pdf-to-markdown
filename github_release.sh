#!/bin/bash

# 从 Cargo.toml 读取版本号
VERSION=$(grep '^version =' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
TAG="v${VERSION}"

echo "Project version: ${VERSION}"
echo "Tag to create: ${TAG}"

# 检查本地是否已有该 tag
if git rev-parse "${TAG}" >/dev/null 2>&1; then
  echo "Error: Tag ${TAG} already exists locally"
  exit 1
fi

# 检查远程是否已有该 tag
if git ls-remote --tags origin | grep -q "refs/tags/${TAG}"; then
  echo "Error: Tag ${TAG} already exists on remote"
  exit 1
fi

# 创建 tag
echo "Creating tag ${TAG}..."
git tag -a "${TAG}" -m "Release ${TAG}"

# 推送 tag
echo "Pushing tag ${TAG} to origin..."
git push origin "${TAG}"

echo "Done! Tag ${TAG} has been created and pushed."
