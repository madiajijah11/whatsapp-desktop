# Onboarding Guide: WhatsApp Desktop

Welcome to the **WhatsApp Desktop** codebase! This guide will help you understand the architecture, design decisions, and core workflows of the project.

---

## 1. Project Overview

- **Name:** WhatsApp Desktop (`whatsapp-desktop`)
- **Core Languages:** Rust, JavaScript, HTML, TOML, JSON
- **Frameworks & Engines:** Tauri v2, WebKitGTK (Linux)
- **Description:** A native Linux desktop wrapper for [web.whatsapp.com](https://web.whatsapp.com). Built with Rust + Tauri v2, it provides native system tray integration, desktop notifications via `notify-send`, memory footprint controls for WebKitGTK, and native clipboard image paste handling—all with zero Electron overhead.

---

## 2. Architecture Layers

The codebase is organized into four architectural layers:

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Application Core & Runtime Layer                         │
│    - src-tauri/src/main.rs (glibc memory tuning & entry)    │
│    - src-tauri/src/lib.rs (Tauri app, bridge, IPC, tray)    │
│    - src-tauri/build.rs (build script)                      │
├─────────────────────────────────────────────────────────────┤
│ 2. Configuration & Capabilities Layer                       │
│    - src-tauri/tauri.conf.json (Tauri app configuration)    │
│    - src-tauri/capabilities/default.json (ACL permissions)  │
│    - src-tauri/Cargo.toml (Rust crate manifest)             │
│    - package.json (CLI tooling)                             │
├─────────────────────────────────────────────────────────────┤
│ 3. Frontend & Diagnostic Interface Layer                    │
│    - dist/index.html (Diagnostic test harness for IPC)      │
├─────────────────────────────────────────────────────────────┤
│ 4. Documentation & Metadata Layer                           │
│    - README.md (User docs, packaging, tray instructions)    │
│    - LICENSE (MIT License)                                  │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Key Concepts & Design Patterns

### 1. Injected JavaScript Bridge (`WHATSAPP_BRIDGE`)
- **File:** `src-tauri/src/lib.rs`
- Injected via Webview `initialization_script`.
- **Notification API Override:** Overrides `window.Notification` to capture title/body from WhatsApp Web, debounces alerts (10s duplicate window, 3s minimum interval), and calls Tauri IPC `notify`.
- **Unread Badge Poller:** Checks `document.title` every 2000ms using a regex `/\((\d+)\)/` to catch unread message badges even if notifications are suppressed.
- **Image Paste Workaround:** Fixes a known WebKitGTK limitation where async clipboard APIs cannot read clipboard images on Linux. Intercepts `Ctrl+V`, fetches raw pixels via native Rust IPC (`arboard`), encodes to PNG, and synthesizes a `DragEvent('drop')` onto WhatsApp's contenteditable editor.

### 2. Multi-Tier Memory Optimization
- **Files:** `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- WebKitGTK and glibc can accumulate cached pages and fragmented heaps.
- **Step 1 (Pre-init):** `MALLOC_TRIM_THRESHOLD_=131072` and `MALLOC_ARENA_MAX=2` are set in `main.rs` before WebKitGTK initializes.
- **Step 2 (Periodic):** A dedicated background thread calls `malloc_trim(0)` every 300 seconds.
- **Step 3 (Event-based):** `malloc_trim(0)` runs immediately whenever the window minimizes to tray.

### 3. System Tray & Window Interception
- **File:** `src-tauri/src/lib.rs`
- Intercepts `tauri::WindowEvent::CloseRequested` to call `api.prevent_close()` and minimize the window rather than exiting.
- Provides a tray menu with **Open** (restores, unminimizes, and focuses) and **Quit** (clean `app.exit(0)`).

### 4. Tauri v2 Security & ACL Isolation
- **File:** `src-tauri/capabilities/default.json`
- Grants remote URL execution only to `https://web.whatsapp.com/*` and `https://*.whatsapp.com/*`.
- Permits only essential scopes: `core:default`, `core:webview:default`, `core:tray:default`, and `opener:default`.

---

## 4. Guided Tour (Recommended Learning Path)

Follow these steps to familiarize yourself with the codebase:

1. **Step 1: Project Architecture & Overview (`README.md`)**
   - Read the motivation for WebKitGTK over Electron, system requirements (GTK3, AppIndicator), and packaging targets (RPM, AppImage).
2. **Step 2: Process Entry & Memory Environment (`src-tauri/src/main.rs`)**
   - See how Linux glibc environment variables are set before delegating to `whatsapp_desktop_lib::run()`.
3. **Step 3: Core Runtime, IPC & Bridge Script (`src-tauri/src/lib.rs`)**
   - The primary file in the repository. Study:
     - `send_notification()`: spawns detached worker running `notify-send`.
     - `clipboard_*`: IPC commands powered by `arboard` and `image::codecs::png`.
     - `WHATSAPP_BRIDGE`: JavaScript bridge containing clipboard and notification logic.
     - `run()`: Window creation, tray icon setup, close event handler, and memory trimmer thread.
4. **Step 4: Configuration & Capabilities (`tauri.conf.json` & `capabilities/default.json`)**
   - Understand the Tauri v2 configuration model, remote capability scopes, and security boundaries.
5. **Step 5: Diagnostic Test Harness (`dist/index.html`)**
   - Inspect the diagnostic page used to test Tauri notification IPC and bridge availability without loading the full WhatsApp Web interface.

---

## 5. File Map

| Path | Category | Layer | Role & Summary |
|---|---|---|---|
| `src-tauri/src/main.rs` | Code (Rust) | Application Core | Binary entrypoint; sets glibc environment variables. |
| `src-tauri/src/lib.rs` | Code (Rust/JS) | Application Core | Application runtime, tray handler, IPC commands, and bridge script. |
| `src-tauri/build.rs` | Code (Rust) | Application Core | Build script compiling Tauri attributes and icons. |
| `src-tauri/Cargo.toml` | Config | Configuration | Crate dependencies: `tauri`, `arboard`, `image`, `serde`. |
| `src-tauri/tauri.conf.json` | Config | Configuration | Tauri bundle, identifier, window settings, and icons. |
| `src-tauri/capabilities/default.json` | Config | Configuration | ACL permissions for `web.whatsapp.com`. |
| `package.json` | Config | Configuration | npm scripts for running `@tauri-apps/cli`. |
| `dist/index.html` | Code (HTML/JS) | Frontend | Diagnostic page with test notification button. |
| `README.md` | Docs | Documentation | User setup, GNOME AppIndicator notes, and build commands. |
| `LICENSE` | Legal | Documentation | MIT License terms. |

---

## 6. Complexity Hotspots (Handle with Care)

| Component / Area | File | Why It Requires Care |
|---|---|---|
| **Synthetic DragEvent Paste Bridge** | `src-tauri/src/lib.rs` (lines 58–220) | Intercepts native DOM paste events in WebKitGTK. Uses complex async promises, Base64 conversion, synthetic `DragEvent('drop')`, and fallback DataTransfer mechanisms. Changes here can break image uploads in WhatsApp Web. |
| **Notification Debounce & Thread Spawning** | `src-tauri/src/lib.rs` (lines 10–23, 63–76) | Notifications run via detached threads invoking `notify-send`. Modifying debounce timing or arguments may lead to notification flooding or missed alerts. |
| **Memory Tuning & `malloc_trim` FFI** | `src-tauri/src/main.rs`, `src-tauri/src/lib.rs` | Uses `unsafe` C FFI to call `malloc_trim(0)` on glibc. Modifying threshold values or trim frequencies directly affects memory usage and CPU wakeups. |
| **Tauri Remote ACL Boundaries** | `src-tauri/capabilities/default.json` | Any changes to permitted URLs or scopes expose the desktop environment to the loaded webview. Keep permissions minimal. |
