#!/usr/bin/env bash
set -euo pipefail

extension_source="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
extension_root="${VSCODE_EXTENSIONS_DIR:-${HOME}/.vscode/extensions}"
extension_target="${extension_root}/local.commander-blood-languages-0.1.0"

mkdir -p -- "${extension_root}"
if [[ -e "${extension_target}" && ! -L "${extension_target}" ]]; then
    printf 'refusing to replace non-symlink extension path: %s\n' "${extension_target}" >&2
    exit 1
fi

ln -sfn -- "${extension_source}" "${extension_target}"
printf 'registered %s -> %s\n' "${extension_target}" "${extension_source}"
printf 'reload the VS Code window to activate the grammars\n'
