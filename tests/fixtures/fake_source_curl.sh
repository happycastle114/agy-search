#!/bin/sh
set -eu

log_path="$0.argv"
: > "$log_path"
for argument in "$@"; do
  printf '%s\n' "$argument" >> "$log_path"
done

source_url=''
for argument in "$@"; do
  source_url=$argument
done

case "$source_url" in
  */good)
    printf '%s' '<div data-tab-buttons></div><button data-tab="cli">Example CLI</button><div data-list-panel="cli"><div data-section-row><h2>1.2.3</h2><time>August 3, 2026</time></div></div>'
    printf '\nAGY_SOURCE_META:200:0\n'
    ;;
  */redirect)
    printf '%s' 'redirect body'
    printf '\nAGY_SOURCE_META:302:0\n'
    ;;
  */oversize)
    dd if=/dev/zero bs=524289 count=1 2>/dev/null | tr '\000' x
    printf '\nAGY_SOURCE_META:200:0\n'
    ;;
  */nonutf8)
    printf '\377'
    printf '\nAGY_SOURCE_META:200:0\n'
    ;;
  */empty)
    printf '\nAGY_SOURCE_META:200:0\n'
    ;;
  *)
    printf '\nAGY_SOURCE_META:404:0\n'
    exit 22
    ;;
esac
