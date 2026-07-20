# Finch

Multi-surface Nest product scaffolded from [`templates/product`](../../templates/product)
by `scripts/scaffold-product-app.sh`.

## Surfaces

| Surface | Path | Command |
|---------|------|---------|
| Desktop (Tauri + React) | `desktop/` | `./build desktop dev` |
| TUI (Ratatui) | `tui/` | `./build tui run` |
| CLI | `cli/` | `./build cli build` |

## Shared core

Business logic and the shared `CliCommand` enum live in `crates/core`. Each surface
is a thin adapter that delegates to the core.

## Quick start

```bash
# Desktop
./build desktop dev

# TUI
./build tui run

# CLI
./build cli run greet World

# Build everything
./build all build
```

See [Nest build standard](../../docs/build.md),
[nest-tauri docs](../../docs/nest-tauri/README.md),
[nest-tui docs](../../docs/nest-tui/README.md), and
[nest-cli docs](../../docs/nest-cli/README.md).
