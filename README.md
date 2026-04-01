# zed-markdown-fold

A Zed editor extension that adds proper folding support for Markdown files: heading sections, fenced code blocks, and blockquotes.

## Why this exists

Zed's built-in folding is indentation-based, which works for code but completely fails for Markdown — headings define structure via `#` prefix levels, not indentation. This has been an [open issue](https://github.com/zed-industries/zed/issues/4924) for ~3 years.

Zed _does_ support LSP-based folding (merged in [PR #48611](https://github.com/zed-industries/zed/pull/48611)), but no Markdown LSP worked well enough out of the box. The only candidate, [IWE](https://github.com/iwe-org/iwe), requires per-project initialization (`iwe init`) and crashes on older Linux servers due to GLIBC incompatibility — making it unusable for SSH remote development.

This extension is a minimal, standalone solution: a ~400-line LSP server that provides `textDocument/foldingRange` for Markdown files.

## What it does

| Feature | Status |
|---------|--------|
| Heading section folding (`# H1` through `###### H6`) | Working |
| Fenced code block folding (`` ``` `` and `~~~`) | Working |
| Blockquote folding (`> ...`) | Working |
| SSH remote support | Working |

### Folding behavior

- **Headings**: A heading folds everything up to the next heading of the same or higher level (or end of file). Sub-headings fold independently within their parent section.
- **Code blocks**: Fenced code blocks (backtick or tilde) fold from the opening fence to the closing fence.
- **Blockquotes**: Consecutive `>` lines fold as a single block.

## Installation

You need Rust with the `wasm32-wasip2` target. If you don't have it:

```bash
rustup target add wasm32-wasip2
```

> **Nix users**: Nix's Rust doesn't ship `wasm32-wasip2`, so you need `rustup` separately:
> ```bash
> curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
> source "$HOME/.cargo/env"
> rustup target add wasm32-wasip2
> ```
> Run `source "$HOME/.cargo/env"` before the build step below.

**Build and install:**

```bash
# Build the WASM extension
cargo build --target wasm32-wasip2 --release

# Copy into Zed's extension directory
DEST="$HOME/Library/Application Support/Zed/extensions/installed/markdown-fold"
mkdir -p "$DEST"
cp extension.toml "$DEST/"
cp target/wasm32-wasip2/release/zed_markdown_fold.wasm "$DEST/extension.wasm"
```

**Enable folding in Zed** — add to `~/.config/zed/settings.json`:

```json
{
  "languages": {
    "Markdown": {
      "document_folding_ranges": "on"
    }
  }
}
```

**Restart Zed.** Fold indicators (triangles) should appear in the gutter next to headings, code fences, and multi-line blockquotes. The LSP server binary is downloaded automatically from GitHub releases on first launch — you don't need to build it yourself.

SSH remotes work automatically: the extension detects the remote platform and downloads the correct Linux binary.

## Project structure

```
zed-markdown-fold/
├── extension.toml              # Zed extension manifest (v0.3.0)
├── Cargo.toml                  # WASM extension crate (compiled to wasm32-wasip2)
├── src/
│   └── lib.rs                  # Extension entry point — locates/downloads the LSP binary
├── md-fold-server/             # The LSP server (standalone native binary)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs             # LSP server: handles initialize, didOpen/didChange, foldingRange
└── .github/
    └── workflows/
        └── release.yml         # Cross-compile and publish binaries on tag push
```

There are two separate Rust projects:

1. **The extension** (`Cargo.toml` at root) — a WASM module (compiled to `wasm32-wasip2`) that Zed loads. It locates the LSP binary, downloading it from GitHub releases if needed. Uses `zed_extension_api` v0.7.0.
2. **The LSP server** (`md-fold-server/`) — a native binary that communicates with Zed over stdio using the LSP protocol. This does the actual Markdown parsing and fold range computation.

## How binary discovery works

The extension tries these in order:

1. **Cached path** from a previous invocation in this session
2. **System PATH** via `worktree.which("md-fold-server")` — works if you've installed the binary globally
3. **GitHub releases** — downloads the platform-specific binary from the latest release at [rsomani95/zed-markdown-fold](https://github.com/rsomani95/zed-markdown-fold/releases), caches it in the extension work directory
4. **Extension work directory** — fallback for manual installs (copy binary to `~/Library/Application Support/Zed/extensions/work/markdown-fold/`)

For SSH remotes, `current_platform()` reports the remote platform, so the extension downloads the Linux binary automatically.

### Supported platforms

| Platform | Release binary | Notes |
|----------|---------------|-------|
| macOS aarch64 (Apple Silicon) | Yes | Native build |
| Linux x86_64 | Yes | Statically linked (musl) |
| Linux aarch64 | Yes | Cross-compiled (musl) |
| macOS x86_64 (Intel) | No | Handled in code, but no release binary — install via PATH |

## Releasing

Tag and push to trigger the GitHub Actions release workflow:

```bash
git tag v0.3.0
git push origin v0.3.0
```

This cross-compiles `md-fold-server` for macOS (aarch64) and Linux (x86_64, aarch64) with musl for glibc-independent static linking, then creates a GitHub release with gzipped binaries.

## Workarounds and known issues

### Manual extension installation

The extension is installed by copying files directly into Zed's `extensions/installed/` directory rather than using `zed: install dev extension` or the extensions marketplace. This works because Zed scans that directory and auto-indexes `extension.toml` files on startup.

**Proper fix**: Publish to the [Zed extensions marketplace](https://github.com/zed-industries/extensions).

### WASM build toolchain

Nix's Rust package doesn't ship the `wasm32-wasip2` standard library, so `rustup` is needed separately (with `--no-modify-path`) for the WASM build. Run `source "$HOME/.cargo/env"` before `cargo build --target wasm32-wasip2` to use the rustup toolchain.

### `lsp-server` crate initialization gotcha

`lsp_server::Connection::initialize(value)` wraps the provided value in `{"capabilities": value}`. If you pass an `InitializeResult` (which already has a `capabilities` field), you get double-nesting and Zed never sees the server's capabilities. The fix is to use `initialize_start()` + `initialize_finish()` and pass the full `InitializeResult` directly.

### Dynamic LSP registration for SSH remotes

Zed has a bug where, on SSH remotes, the client checks server capabilities before the remote server's static capabilities have been propagated. This means `folding_range_provider: true` in the initialize result gets ignored, and Zed never sends `textDocument/foldingRange` requests.

The workaround: the LSP server sends a `client/registerCapability` request to dynamically register `textDocument/foldingRange` support after receiving `initialized` or `textDocument/didOpen`. This ensures the client recognizes fold support regardless of the static capability race.

## Future work

### Publish to Zed extensions marketplace

Submit a PR to [zed-industries/extensions](https://github.com/zed-industries/extensions).

### Additional folding targets

The LSP server could be extended to fold:
- HTML comment blocks (`<!-- ... -->`)
- YAML frontmatter (`---` blocks at the top of a file)
- Nested list items (if indentation-based folding doesn't cover them well enough)

### Blockquote syntax highlighting

Blockquotes (`> text`) currently have no special syntax highlighting in Zed's Markdown mode. This is a tree-sitter `highlights.scm` concern, not an LSP issue. Options:
- A Zed extension _might_ be able to ship supplementary `highlights.scm` queries, but the extension API doesn't clearly support this for languages the extension doesn't own
- May require a PR to Zed's built-in Markdown grammar queries ([`crates/grammars/src/markdown/highlights.scm`](https://github.com/zed-industries/zed/blob/main/crates/grammars/src/markdown/highlights.scm))
- Alternatively, a custom theme could style blockquote tokens if the tree-sitter grammar already parses them (it does — the grammar has `block_quote` nodes)

### macOS Intel release binary

The extension code handles `x86_64-apple-darwin` but the release workflow doesn't build for it. Could be added to the GitHub Actions matrix if needed.

## Key references

- [Zed issue #4924](https://github.com/zed-industries/zed/issues/4924) — Original markdown folding feature request (open since 2023)
- [Zed issue #22703](https://github.com/zed-industries/zed/issues/22703) — `folds.scm` support request (the "proper" long-term solution)
- [Zed PR #48611](https://github.com/zed-industries/zed/pull/48611) — LSP `textDocument/foldingRange` support (merged, what makes this extension possible)
- [IWE PR #235](https://github.com/iwe-org/iwe/pull/235) — IWE's folding range implementation (reference for the LSP protocol)
- [Zed extension docs](https://zed.dev/docs/extensions/languages) — How extensions register language servers
