# Nyx

A desktop GUI for the [Mihomo](https://github.com/MetaCubeX/mihomo) proxy core. A single pure-Rust [gpui](https://github.com/zed-industries/zed) application — no web frontend.

## Architecture

The crate is `nyx_lib` (binary `nyx`). Three top-level modules under `src/`:

| Module        | Responsibility                                                                        |
| ------------- | ------------------------------------------------------------------------------------- |
| `app/`        | Process lifecycle & OS integration: state, bootstrap, tray, hotkeys, deep links, single-instance, window show/hide, autostart, tokio runtime bridge. |
| `backend/`    | Everything that talks to Mihomo: REST `api`, `config`/profile management, `manager` (runtime config), `service`/`service_host` (Windows helper service), `streaming` (live logs/connections/traffic), `updater`, `proxy_convert` (subscription parsing). |
| `ui/`         | gpui views: `root` (the `NyxApp` root view + most state and actions), `rail` (nav), `theme` (color tokens), and `pages/` (one file per screen — home, proxies, profiles, rules, connections, logs, settings). |

- `lib.rs::run()` is the entry point: service dispatch → single-instance guard → gpui app boot (theme, state, window, backend startup, tray, hotkeys, deep links).
- Shared UI state lives in `AppState` (an `Entity<AppState>` global, observed for re-render) in `app/state.rs`. Per-view interaction state lives on `NyxApp` in `ui/root.rs`.
- Async work runs on a tokio runtime via `app::runtime::spawn`; results are folded back into state on the gpui foreground with `cx.spawn`.

## Commands

```bash
cargo run               # dev build
cargo build --release   # optimized binary → target/release/nyx
cargo fmt               # format
cargo clippy -- -D warnings
cargo check
```

On Linux you need the gpui/tray system libraries (see README → Build from source) or just `nix develop`.

Before every commit, the same checks CI runs must pass: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo check`.

## Code Guidelines

### Comments
- NO file-header comment blocks (`//!` module banners) and NO "heading"/divider comments like `// --- helpers ---` or `// ─── VLESS ───`. Group code with functions, not comment art.
- Avoid inline `//` comments. Add one only when the code is genuinely non-obvious (a real footgun) — e.g. a gpui borrow re-entry, a wire-format quirk. Then keep it to a line or two.
- Doc comments (`///`) on public items are fine, but keep them short — a single line describing intent. Code should read for itself.
- Don't narrate the obvious (`// loop over items`). If a comment restates the next line, delete it.
- Keep every comment you do write as short as possible — the fewest words that convey the non-obvious bit. Prefer one line; never write a paragraph where a clause will do.

### Style
- rustfmt is the source of truth — never hand-format against it.
- Match the surrounding code: gpui views are built with the `div()`/`h_flex()`/`v_flex()` builder chains; follow the existing element/spacing idiom in the page you're editing.
- Color tokens come from `ui::theme` (re-exported through `ui::root`); never hardcode hex outside `theme.rs`.

### i18n
- All user-facing strings go through `rust_i18n::t!` with a key.
- Every key must exist in ALL locale files: `locales/en-US.yml`, `ru-RU.yml`, `zh-CN.yml`. Adding a key to only one will fall back silently — add all three.

### gpui gotchas
- Win32 window calls (`ShowWindow`, hide/show/foreground) must run OUTSIDE any live gpui borrow — schedule them via `App::defer`/`App::spawn`, or they re-enter and panic with "RefCell already borrowed" (see `app/window.rs`, `app/actions.rs`).
- The top-level view must render gpui-component's `Root` overlay layers, or toasts/modals never appear (see `ui/root.rs`).

### Linux platform
- Tray: Linux uses a pure D-Bus StatusNotifierItem via `ksni` (no gtk / appindicator). `tray-icon` is cfg-gated to non-Linux, and the two backends diverge inside `app/tray.rs` — keep new tray logic behind the right `cfg`.
- System proxy (`backend/sysproxy.rs`): on Linux we set both GSettings and the proxy env vars (systemd user manager + D-Bus activation env). Only GNOME-like desktops reliably honor the GSettings proxy — `session_honors_proxy()` gates the "partial coverage" note; TUN is the full-device path.
- NixOS: detected via `/etc/NIXOS` (`elevation::is_nixos()`). TUN caps come from `programs.nyx.tunMode` (declarative capability wrapper), not a runtime `setcap`, so the Settings grant button is swapped for instructions there. The flake exposes `packages.nyx` / the `nyx` app in addition to the dev shell.

## Bash Guidelines
- Don't pipe output through `head`/`tail`/`less` to truncate — use tool-native flags (`git log -n 10`, `cargo clippy --message-format=short`). Read the full output.
- Don't create scratch files (scripts, notes) unless asked.
- When given failures, just fix them — don't argue about who introduced them.
