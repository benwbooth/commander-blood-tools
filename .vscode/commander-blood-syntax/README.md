# Commander Blood VS Code languages

This project-local extension provides TextMate syntax highlighting and editor
configuration for:

- BloodScript 8 (`*.blood`)
- Commander Blood DESCRIPT (`*.descript`)

Register the checkout's extension source with VS Code:

```sh
.vscode/commander-blood-syntax/install.sh
```

Then run **Developer: Reload Window** in VS Code. The installer creates only a
symlink under `${HOME}/.vscode/extensions`; grammar source remains in this
checkout, so edits take effect after another window reload.
