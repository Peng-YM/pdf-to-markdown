# Installation

## One-Click Install (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/Peng-YM/pdf-to-markdown/master/install.sh | bash
```

The script detects your OS and architecture, downloads the correct prebuilt binary from GitHub Releases, and installs to `~/.local/bin/pdf-to-markdown`. After installation, the script warns if `~/.local/bin` is not in your PATH.

To verify installation:

```bash
pdf-to-markdown --version
```

If the command is not found, add `~/.local/bin` to your PATH:

```bash
# Add to ~/.zshrc or ~/.bashrc
export PATH="$HOME/.local/bin:$PATH"
# Then reload
source ~/.zshrc
```

## Specifying a Version

```bash
curl -fsSL https://raw.githubusercontent.com/Peng-YM/pdf-to-markdown/master/install.sh | bash -s v0.5.0
```

## Alternative: Download from GitHub Releases

Visit https://github.com/Peng-YM/pdf-to-markdown/releases and download the binary for your platform:

- `pdf-to-markdown-macos-x86_64` or `pdf-to-markdown-macos-aarch64`
- `pdf-to-markdown-linux-x86_64` or `pdf-to-markdown-linux-aarch64`
- `pdf-to-markdown-windows-x86_64.exe`

## Building from Source

Requires Rust toolchain (install via https://rustup.rs):

```bash
git clone https://github.com/Peng-YM/pdf-to-markdown.git
cd pdf-to-markdown
cargo build --release
# Binary at target/release/pdf-to-markdown
```

## Platform-Specific Notes

### Credential Storage Backend

| Platform | Backend | Notes |
|---|---|---|
| macOS | System Keychain (security-framework) | No extra dependencies needed |
| Linux | Secret Service (keyring crate) | See prerequisites below; encrypted file fallback available |
| Windows | Credential Manager (keyring crate) | No extra dependencies needed |

### Linux Prerequisites

The secure credential storage (`pdf-to-markdown login`) depends on D-Bus and libsecret:

```bash
# Debian/Ubuntu
sudo apt install libdbus-1-dev libsecret-1-dev

# Fedora
sudo dnf install dbus-devel libsecret-devel

# Arch
sudo pacman -S libsecret
```

If these libraries are unavailable, the tool automatically falls back to an AES-256-GCM encrypted file at `~/.config/pdf-to-markdown/credentials.enc`. This fallback is transparent — no configuration needed.

### Path Configuration

| Shell | Config File | Example |
|---|---|---|
| zsh | `~/.zshrc` | `export PATH="$HOME/.local/bin:$PATH"` |
| bash | `~/.bashrc` | `export PATH="$HOME/.local/bin:$PATH"` |
| fish | `~/.config/fish/config.fish` | `fish_add_path ~/.local/bin` |
