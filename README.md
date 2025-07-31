# Geektools | Geek Toolbox

[English](./README.md) | [中文](./README_CN.md) 

## Special Thanks

- [rust](https://www.rust-lang.org/)
- [homebrew](https://brew.sh/)
- [crates.io](https://crates.io/)

## What Is This?

- A CLI tool that executes **built-in** shell scripts  
- A CLI tool that executes shell scripts from **remote URLs**
- Built-in OTA to switch between published versions
- …and more to come

## How to Install?

### First: Windows Users (the most common case)

Geektools currently **does not support Windows**.  
Please stay tuned for “The Emperor’s Future Project” (coming soon).

### Next: Pick One—Download a Pre-built Package or Build from Source

### Download the Package and fast start


1. In your terminal:

   ```bash
   # install wget
   # sudo apt install curl | sudo yum install curl
   curl "https://raw.githubusercontent.com/PeterFujiyu/geektools/refs/heads/master/install.sh" | bash
   ```
2. Enjoy 🎉

### Build Manually
*(only tested with a Rust toolchain on macOS; Linux is unverified)*

#### Preparation
```
bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```
#### Build (choose one)
```
bash
# Cross-platform artifacts (macOS universal + Linux x86_64/aarch64)
sh ./allrelease.sh
# → artifacts in ./target/dist

# Current host only
cargo build --release
# → binary at ./target/release/geektools
```
## Contributing Guide

1. Fork this repository and clone it locally.
2. Create a feature branch:

   ```bash
   git checkout -b feature/your-feature
   ```

3. Commit your changes:

   ```bash
   git commit -m "feat: your feature"
   ```

4. Push the branch:

   ```bash
   git push origin feature/your-feature
   ```

5. Open a Pull Request on GitHub.

Issues and PRs are warmly welcome—let’s improve Geektools together!

## License
Copyright ©️ PeterFujiyu
[GPLv3 LICENSE](./LICENSE)

