<p align="center"><img width="128" src="assets/Icon/Icon.ico" /></p>
<h1 align="center">Xodus</h1>
<p align="center"><strong>The Native Microsoft Store & Xbox Game Pass Client for Linux</strong></p>
<p align="center">
    <a href="https://github.com/nocts-software/xodus/releases">
        <img src="https://img.shields.io/github/v/release/nocts-software/xodus?style=for-the-badge&color=blue" alt="Latest Release" />
    </a>
    <a href="https://discord.gg/ZG774FK4tq">
        <img src="https://img.shields.io/discord/1123890623586504714?logo=discord&style=for-the-badge&color=green&label=Discord" alt="Discord" />
    </a>
    <img src="https://img.shields.io/badge/Platform-Linux%20%7C%20SteamDeck-orange?style=for-the-badge" alt="Platform" />
    <img src="https://img.shields.io/badge/License-GPL--3.0-purple?style=for-the-badge" alt="License" />
</p>

> [!CAUTION]
> **Unofficial Project Notice**: Xodus is an independent, open-source project and is not affiliated with, endorsed by, or sponsored by Microsoft Corporation or Xbox. All trademarks, product names, and logos are property of their respective owners.

---

## 🌟 Overview

**Xodus** is a high-performance, native Linux client and ecosystem designed to download, decrypt, manage, and launch **Xbox Game Pass PC** and **Microsoft Store** titles seamlessly on Linux and Steam Deck using **Wine** and **Proton**.

Traditionally, Microsoft Store games are packaged in proprietary, encrypted **MSIXVC** (Xbox Virtual Disk) containers with deep dependencies on Windows Game Development Kit (GDK) services, making them unplayable on Linux. Xodus bridges this gap completely by combining:

1. **Direct Package Streaming & Real-Time Decryption**: Downloads and streams chunks directly from Microsoft CDN, decrypting encrypted game binaries on-the-fly.
2. **Built-in Microsoft Gaming Runtime (`xgameruntime`)**: An open-source GDK runtime implementation providing asynchronous task queues, user authentication, and persistent local storage.
3. **Automated Cloud Save Syncing**: Bidirectional Xbox Live cloud save synchronization before launch and on game exit.
4. **Seamless Proton/Wine Integration**: Automated drive letter mappings, Proton compatibility prefix setup, Easy Anti-Cheat runtime discovery, and DLL override management.
5. **Modern Desktop GUI**: A sleek, hardware-accelerated desktop interface with library browsing, cover art hydration, search filtering, and one-click launch.

---

## ✨ Key Features

- **🎮 Comprehensive Library Management**:
  - Automatically fetches and displays your owned Microsoft Store library and active Xbox Game Pass titles.
  - Cached offline SQLite database for instant startup and offline browsing.
  - Rich metadata, high-resolution box art, developer information, and license indicators.

- **⚡ MSIXVC Streaming & Decryption**:
  - Direct chunk streaming from Microsoft Store CDNs with zero-copy decryption.
  - On-demand in-place binary extraction without duplicating entire game folders.
  - Full support for `.msixvc` containers, Content directories, and `MicrosoftGame.config` parsing.

- **🚀 Native Execution via Proton / Wine**:
  - Automatic discovery of Proton installations (Proton CachyOS, GE-Proton, Proton Experimental, Proton 9/8).
  - Custom `dosdevices` virtual drive mapping (`X:`) preventing anti-cheat drive rejection (fixes Easy Anti-Cheat `Z:\` root drive errors).
  - Full compatibility with Steam's Linux Easy Anti-Cheat Runtime (`Proton EasyAntiCheat Runtime/v2`).

- **☁️ Xbox Live Cloud Saves**:
  - Cloud save synchronization querying Xbox Live Title Storage and Connected Storage (SCID/XUID).
  - Automatic `pull` on game startup and automated `push` upon game session exit.
  - Local save translation into Wine prefix application data directories.

- **🛠️ Power-User CLI & Background Service**:
  - Rich command-line interface (`xodus-cli`) for headless servers, scripts, and Steam Deck Game Mode shortcuts.
  - Inter-process communication daemon (`xodus-service`) handling token management and GDK runtime integration.

---

## 📦 Project Architecture

The Xodus workspace is organized into modular Rust crates:

```
xodus/
├── crates/
│   ├── msixvc/        # High-performance parser and decryptor for MSIXVC / XVD containers
│   ├── xodus/         # Core library: Xbox Live OAuth, MSA tokens, catalog, and cloud saves
│   ├── xodus-cli/     # Command-line interface for downloads, running games, and save management
│   ├── xodus-gui/     # Modern Wry/Tao desktop client with dark aesthetic and MangoHud controls
│   └── xodus-service/ # IPC daemon and system service for xgameruntime.dll integration
├── assets/            # Desktop icons, SVG graphics, and visual assets
├── docs/              # In-depth architectural and protocol documentation
└── build-appimage.sh  # Automated build script producing standalone AppImage releases
```

---

## 🚀 Quick Start & Installation

### Option 1: Standalone AppImage (Recommended)

Download the latest `Xodus-x86_64.AppImage` from the [Releases](https://github.com/nocts-software/xodus/releases) page:

```bash
chmod +x Xodus-x86_64.AppImage

# Launch the Graphical Client
./Xodus-x86_64.AppImage

# Or use as a CLI tool directly
./Xodus-x86_64.AppImage streaming 9PK087LNGJC5 /mnt/w11/XboxGames/Balatro
./Xodus-x86_64.AppImage run /mnt/w11/XboxGames/Balatro
```

### Option 2: Building from Source

#### Prerequisites

- **Rust toolchain** (supporting `edition = "2024"`, 1.85+)
- **System dependencies** (Debian/Ubuntu/Arch/Fedora):
  - `pkg-config`, `openssl-devel` / `libssl-dev`
  - `webkit2gtk4.1-devel` / `libwebkit2gtk-4.1-dev` (for GUI)
  - `protoc` (protobuf compiler)
  - `wine`, `winegcc`, `winebuild`, `widl` (for compiling runtime libraries)
  - `appimagetool` (optional, for packaging AppImages)

#### Compilation

1. Clone the repository and submodules:
   ```bash
   git clone https://github.com/nocts-software/xodus.git --recursive
   cd xodus
   ```

2. Build all release binaries:
   ```bash
   cargo build --release --workspace
   ```

3. Build the self-contained AppImage:
   ```bash
   ./build-appimage.sh
   ```

---

## 🖥️ Usage Guide

### 1. Graphical Interface (`xodus-gui`)

Launch `xodus-gui` to authenticate with your Microsoft / Xbox account:

- **Sign In**: Click "Sign In with Microsoft" to authenticate via secure OAuth2 device login.
- **Library Sync**: Your game entitlements and Xbox Game Pass catalog are fetched and cached automatically.
- **Install & Stream**: Select any game and click **Install** to stream and decrypt package files.
- **Launch**: Click **Play** on any installed title to launch with configured Proton/Wine options.

### 2. Command-Line Interface (`xodus-cli`)

The `xodus` CLI provides complete control over authentication, package extraction, execution, and cloud saves.

#### Authentication & Status

```bash
# Authenticate with Microsoft account via browser device code flow
xodus login

# Inspect current authentication status and user Gamertag
xodus status

# View license entitlements and ownership
xodus entitlements
```

#### Downloading & Streaming Games

```bash
# Stream and decrypt a game directly by Store BigID (e.g. Balatro: 9PK087LNGJC5)
xodus streaming 9PK087LNGJC5 /mnt/w11/XboxGames/Balatro

# Download raw MSIXVC package from CDN
xodus download 9PK087LNGJC5 /mnt/w11/XboxGames/Downloads
```

#### Running Games

```bash
# Launch an installed game folder with Proton and cloud save sync
xodus run /mnt/w11/XboxGames/Balatro

# Launch with custom Proton binary
PROTON_PATH=/usr/share/steam/compatibilitytools.d/proton-cachyos-native/proton \
xodus run /mnt/w11/XboxGames/Balatro
```

#### Cloud Saves

```bash
# Manually pull Xbox Live cloud saves for a game directory
xodus save pull /mnt/w11/XboxGames/Balatro

# Manually push local saves to Xbox Live cloud
xodus save push /mnt/w11/XboxGames/Balatro
```

#### Extraction & Encryption Tools

```bash
# Dump Content Encryption Keys (CIK)
xodus license <path-to-msixvc>

# Decrypt and extract local MSIXVC container
xodus extract <path-to-msixvc> <output-dir>
```

---

## ⚙️ Environment Variables & Tuning

| Variable | Description | Default |
|---|---|---|
| `PROTON_PATH` | Path to custom Proton executable script | Auto-detected from Steam & system paths |
| `XODUS_RUNTIME_PATH` | Directory containing GDK runtime DLLs | Bundled with AppImage or `/usr/lib/xodus` |
| `WINEDLLPATH` | Wine shared library search path | Configured automatically |
| `WINEDLLOVERRIDES` | DLL override specifications for Wine | Configured automatically for GDK shims |
| `XODUS_LOG` | Rust logging level (`trace`, `debug`, `info`, `warn`, `error`) | `info` |

---

## 🤝 Contributing & Community

Contributions are welcome! Please feel free to submit pull requests, report bugs, or request features on GitHub.

- **Discord**: [Join our Game Launchers Reverse Engineering Discord](https://discord.gg/ZG774FK4tq)
- **Sister Projects**:
  - [xgameruntime](https://github.com/nocts-software/xgameruntime) ([upstream](https://github.com/xodus-gaming/xgameruntime)): Open-source Wine implementation of Microsoft Gaming Runtime.
  - [xgameruntime-docs](https://github.com/nocts-software/xgameruntime-docs) ([upstream](https://github.com/xodus-gaming/xgameruntime-docs)): Detailed documentation of GDK COM interfaces and reverse engineering notes.
- **Original Upstream Repositories**:
  - [xodus-gaming/xodus](https://github.com/xodus-gaming/xodus)
  - [xodus-gaming/xgameruntime](https://github.com/xodus-gaming/xgameruntime)
  - [xodus-gaming/xgameruntime-docs](https://github.com/xodus-gaming/xgameruntime-docs)

---

## 📜 License

This project is licensed under the **GNU General Public License v3.0 (GPL-3.0)**. See the [LICENSE](LICENSE) file for complete details.

### Acknowledgments & Special Thanks
- The original [xodus-gaming](https://github.com/xodus-gaming) organization and contributors for creating [xodus](https://github.com/xodus-gaming/xodus), [xgameruntime](https://github.com/xodus-gaming/xgameruntime), and [xgameruntime-docs](https://github.com/xodus-gaming/xgameruntime-docs).
- [LukeFZ](https://github.com/LukeFZ) for pioneering research in `XvdTool.Streaming` and `CikExtractor`.
- The Wine and Proton communities for runtime compatibility foundations.
