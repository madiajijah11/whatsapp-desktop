# WhatsApp Desktop

Native WhatsApp Web wrapper for Linux, built with Rust + Tauri v2.

Lightweight desktop app that wraps [web.whatsapp.com](https://web.whatsapp.com) in a native Linux window with system tray support and native notifications.

![Rust](https://img.shields.io/badge/Rust-1.95+-dea584?logo=rust&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?logo=tauri&logoColor=white)
![Platform](https://img.shields.io/badge/Platform-Linux-fcc624?logo=linux&logoColor=white)

## Features

- **Native Linux app** — GTK3/WebKitGTK based, no Electron overhead
- **System tray** — Minimize to tray, stays running in background
- **Native notifications** — Desktop notifications for incoming messages
- **External links** — Opens links in your default browser, not inside the app
- **Auto-start on boot** — Can be configured to start with your desktop session

## Requirements

### System Dependencies

**Fedora / AlmaLinux / RHEL:**
```bash
sudo dnf install -y \
  webkit2gtk4.1-devel \
  gtk3-devel \
  librsvg2-devel \
  libayatana-appindicator-gtk3-devel
```

**Ubuntu / Debian:**
```bash
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  librsvg2-dev \
  libayatana-appindicator3-dev
```

### GNOME Desktop

If you use GNOME, install the AppIndicator extension for system tray support:
```bash
# Fedora / AlmaLinux
sudo dnf install -y gnome-shell-extension-appindicator

# Enable it
gnome-extensions enable appindicatorsupport@rgcjonas.gmail.com
```

For other desktop environments (KDE, XFCE, etc.) system tray works out of the box.

## Installation

### From RPM (Fedora / AlmaLinux / RHEL)

Download the `.rpm` package from [Releases](../../releases) and install:

```bash
sudo rpm -i WhatsApp_Desktop-1.0.0-1.x86_64.rpm
```

### From DEB (Ubuntu / Debian)

Download the `.deb` package from [Releases](../../releases) and install:

```bash
sudo dpkg -i WhatsApp_Desktop_1.0.0_amd64.deb
```

### From Source

**Prerequisites:**
- [Rust](https://rustup.rs/) 1.95+
- [Node.js](https://nodejs.org/) 20+

```bash
git clone https://github.com/madiajijah11/whatsapp-desktop.git
cd whatsapp-desktop
npm install
npm run tauri build
```

The built packages will be in `src-tauri/target/release/bundle/`.

To run in development mode:
```bash
npm run tauri dev
```

## Usage

```bash
whatsapp-desktop
```

### System Tray

| Action | Behavior |
|--------|----------|
| Click **X** (close) | Window hides to tray, app keeps running |
| **Right-click** tray icon | Menu: Buka WhatsApp / Keluar |
| Click **Keluar** | Fully exits the application |

### Notifications

Notifications are automatically bridged from WhatsApp Web to native desktop notifications. They work even when the window is hidden to tray.

## How It Works

This app is a lightweight wrapper around WhatsApp Web:

1. **Tauri v2** creates a native WebKitGTK window
2. The window loads `https://web.whatsapp.com`
3. A JavaScript bridge intercepts WhatsApp's Notification API and forwards them to native Linux notifications via the Tauri notification plugin
4. External links are intercepted and opened in the default system browser

The app itself does not store messages or modify WhatsApp's behavior — it simply provides a native desktop experience for the existing web client.

## Tech Stack

| Component | Technology |
|-----------|------------|
| Backend | Rust + Tauri v2 |
| Frontend | web.whatsapp.com (loaded directly) |
| Web Engine | WebKitGTK 4.1 |
| Notifications | tauri-plugin-notification + notify-rust |
| System Tray | libayatana-appindicator |
| Bundler | Tauri bundler (RPM, DEB, AppImage) |

## Project Structure

```
whatsapp-desktop/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Entry point
│   │   └── lib.rs           # App setup, tray, notifications
│   ├── icons/               # App icons
│   ├── capabilities/
│   │   └── default.json     # Tauri permissions
│   ├── tauri.conf.json      # Tauri configuration
│   └── Cargo.toml           # Rust dependencies
├── dist/                    # Minimal frontend (fallback)
└── package.json
```

## License

MIT
