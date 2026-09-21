# Security Policy

## Scope

TidyUp moves files in folders you chose. A real vulnerability here is anything that would make
it move, overwrite or lose a file it should not touch:

- **The rule engine and mover** (`src-tauri/src/engine.rs`, `src-tauri/src/fsutil.rs`) — a way
  to make it move a file outside a watched folder, follow a path outside the chosen
  destination, overwrite an existing file (it must always rename on a clash), delete anything,
  or act on a file still being written.
- **Undo** (`Put back`) — a way to make it put a file somewhere other than where it came from.
- **The source-site rule** — TidyUp reads the download's origin from the Zone.Identifier
  stream. A crafted stream must never do more than influence which category folder is chosen.

General bugs, crashes and UI issues are not security reports; please use the normal
[issue tracker](https://github.com/nur-arpon/TidyUp/issues) for those.

## Reporting a vulnerability

**Please do not open a public issue for a security problem.** Report it privately:

1. Go to the [Security tab](https://github.com/nur-arpon/TidyUp/security) of this repository.
2. Click **"Report a vulnerability"** to open a private security advisory.

Include the version affected, exact steps to reproduce, and what you observed. You will get an
acknowledgement, and a fix ships as a normal release with credit to you unless you prefer not.

## What TidyUp does not do

No network code, no updater, no elevation, no hooks, no services. See PRIVACY.md.
