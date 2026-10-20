#!/bin/sh
set -eu

cargo test -p xaga favicon_pdf_response_stays_with_image_consumer --lib --locked --offline
XAGA_DOWNLOAD_SMOKE_CASE=favicon-pdf \
  cargo test -p xaga --test resource_download_webview \
    --features wry-pdf-downloads --locked --offline
