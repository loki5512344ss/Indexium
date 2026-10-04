#!/usr/bin/env bash
set -euo pipefail
# check for em dash - use hyphen instead
if grep -r $'\xe2\x80\x94' --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.svelte-kit -n . 2>&1 | grep -q $'\xe2\x80\x94'; then
  echo "ERROR: Found em dash - use hyphen instead"
  grep -r $'\xe2\x80\x94' --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.svelte-kit -n . 2>&1
  exit 1
else
  echo "OK: no em dash found"
fi
