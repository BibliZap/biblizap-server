#!/usr/bin/env bash
set -euo pipefail

extension_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
dist_dir="$extension_dir/dist"

case "${1:-all}" in
    all) browsers=(firefox chrome) ;;
    firefox|chrome) browsers=("$1") ;;
    -h|--help)
        printf 'Usage: %s [all|firefox|chrome]\n' "$0"
        exit 0
        ;;
    *)
        printf 'Unknown browser: %s\n' "$1" >&2
        exit 1
        ;;
esac

if (( $# > 1 )); then
    printf 'Expected at most one browser argument.\n' >&2
    exit 1
fi

command -v zip >/dev/null || { printf 'Packaging requires zip.\n' >&2; exit 1; }
mkdir -p -- "$dist_dir"

for browser in "${browsers[@]}"; do
    output_dir="$dist_dir/$browser"
    mkdir -p -- "$output_dir"
    cp -- "$extension_dir/content.js" "$extension_dir/BBZ.png" "$output_dir/"
    cp -- "$extension_dir/manifests/$browser.json" "$output_dir/manifest.json"

    # Build a fresh archive so removed assets cannot remain in an older ZIP.
    archive_dir="$(mktemp -d "$dist_dir/.package-XXXXXX")"
    (
        cd -- "$output_dir"
        zip -q "$archive_dir/biblizap-$browser.zip" manifest.json content.js BBZ.png
    )
    mv -- "$archive_dir/biblizap-$browser.zip" "$dist_dir/biblizap-$browser.zip"
    rmdir -- "$archive_dir"
    printf 'Built %s and %s\n' "$output_dir" "$dist_dir/biblizap-$browser.zip"
done
