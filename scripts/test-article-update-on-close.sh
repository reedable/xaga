#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo test -p xaga --lib article_loader::close_tests --offline
npm test -- src-tauri/src/article_loader.test.ts src/components/content/ArticleReader.test.tsx
