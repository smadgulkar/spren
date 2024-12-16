# 🤖 Spren - AI-Powered Terminal Assistant

[![GitHub release](https://img.shields.io/github/v/release/smadgulkar/spren)](https://github.com/smadgulkar/spren/releases)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![OS](https://img.shields.io/badge/OS-Windows%20%7C%20macOS%20%7C%20Linux-blue)]()

A smart AI shell assistant that transforms natural language into powerful shell commands. Perfect for both beginners and power users.

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#configuration">Configuration</a> •
  <a href="#usage">Usage</a> •
  <a href="#license">License</a>
</p>

## About

Spren is an intelligent command-line assistant powered by AI that translates natural language into precise shell commands. Whether you're a CLI novice or expert, Spren helps you work more efficiently by understanding your intent and suggesting the right commands for PowerShell, Bash, or CMD.

## Features

- 🤖 Natural language to shell command conversion using AI
- 🔄 Cross-platform support for Windows (PowerShell/CMD), Linux (Bash), and macOS
- 🛡️ Safe execution with command previews and safety confirmations
- 🧠 Intelligent error analysis and command suggestions
- ⚡ Multi-shell support (Bash, PowerShell, CMD)

## Installation
### Linux and macOS
1. Download the latest release for your platform:
   ```bash
   # Linux
   curl -LO https://github.com/smadgulkar/spren/releases/latest/download/spren-linux-amd64.tar.gz
   # macOS
   curl -LO https://github.com/smadgulkar/spren/releases/latest/download/spren-macos-amd64.tar.gz
   ```
2. Extract and make executable:
   ```bash
   tar xzf spren-*-amd64.tar.gz
   chmod +x spren
   ```
3. (Optional) Move to a directory in your PATH:
   ```bash
   sudo mv spren /usr/local/bin/
   ```
### Windows
1. Download `spren-windows-amd64.zip` from the [latest release](https://github.com/smadgulkar/spren/releases/latest)
2. Extract the ZIP file
3. Run `spren.exe` from any terminal
## Configuration
1. Run Spren once to create the default config:
   ```bash
   spren
   ```
2. Edit your config file:
   ```bash
   # Linux/macOS
   vim ~/.config/spren/config.toml
   # Windows (PowerShell)
   notepad $env:USERPROFILE\.config\spren\config.toml
   ```

## Examples

Here are some ways to use Spren with natural language:

Find large files:
```bash
spren> show me files larger than 1GB
```

Search through code:
```bash
spren> find all rust files containing the word "config"
```

Complex operations:
```bash
spren> compress all jpg files in current directory and its subdirectories
```

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

Spren is powered by:
- [Anthropic's Claude](https://www.anthropic.com/) - Advanced AI language model
- [OpenAI](https://openai.com/) - AI technology provider

---

<p align="center">
  Made with ❤️ using Rust and AI
  <br>
  <a href="https://smadgulkar.github.io/spren">Website</a> •
  <a href="https://github.com/smadgulkar/spren/issues">Issues</a> •
  <a href="https://github.com/smadgulkar/spren/releases">Releases</a>
</p>
