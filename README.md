# 🤖 Spren - AI-Powered Terminal Assistant

[![GitHub release](https://img.shields.io/github/v/release/smadgulkar/spren)](https://github.com/smadgulkar/spren/releases)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0) [![OS](https://img.shields.io/badge/OS-Windows%20%7C%20macOS%20%7C%20Linux-blue)]()

A smart AI shell assistant built with Rust 🦀 that transforms natural language into accurate shell commands (Bash, PowerShell, CMD) for Linux, macOS, and Windows. Boost your command-line productivity, whether you're a beginner or power user.

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#configuration">Configuration</a> •
  <a href="#usage">Usage</a> •
  <a href="#license">License</a>
</p>

## About

Spren is an intelligent command-line (CLI) assistant, written in Rust and powered by AI models (like Claude & OpenAI), designed to translate natural language instructions into precise shell commands. Whether you're new to the terminal or an experienced user, Spren streamlines your workflow by understanding your intent and generating the right commands for Bash (Linux/macOS), PowerShell (Windows), or CMD (Windows). Improve your efficiency and reduce time spent looking up command syntax.

## Features

- 🤖 **Natural Language Processing:** Converts plain English requests into shell commands using AI.
- 🔄 **Cross-Platform:** Native support for Windows (PowerShell/CMD), Linux (Bash), and macOS (Bash).
- 🛡️ **Safe Execution:** Preview commands before running and confirm execution for safety.
- 🧠 **Intelligent Assistance:** Provides error analysis and suggests command corrections.
- ⚡ **Multi-Shell:** Works seamlessly with Bash, PowerShell, and CMD environments.

## Installation

### Linux and macOS

1.  Download the latest release binary for your platform:
    ```bash
    # Linux (amd64)
    curl -LO [https://github.com/smadgulkar/spren/releases/latest/download/spren-linux-amd64.tar.gz](https://github.com/smadgulkar/spren/releases/latest/download/spren-linux-amd64.tar.gz)
    # macOS (amd64)
    curl -LO [https://github.com/smadgulkar/spren/releases/latest/download/spren-macos-amd64.tar.gz](https://github.com/smadgulkar/spren/releases/latest/download/spren-macos-amd64.tar.gz)
    # (Add other architectures like arm64 if available)
    ```
2.  Extract the archive and make the binary executable:
    ```bash
    tar xzf spren-*-amd64.tar.gz
    chmod +x spren
    ```
3.  (Optional) Move the `spren` binary to a directory in your system's PATH for easier access:
    ```bash
    sudo mv spren /usr/local/bin/
    ```

### Windows

1.  Download the `spren-windows-amd64.zip` (or other architecture if available) from the [latest release page](https://github.com/smadgulkar/spren/releases/latest).
2.  Extract the ZIP archive.
3.  You can run `spren.exe` directly from your terminal or move it to a directory included in your system's PATH environment variable.

## Configuration

1.  Run Spren for the first time to generate the default configuration file:
    ```bash
    spren config --show-path # Or simply run 'spren' if it prompts
    ```
2.  Edit the configuration file (`config.toml`) located in the path shown above:
    ```bash
    # Example paths (use the path shown by the command above)
    # Linux/macOS:
    vim ~/.config/spren/config.toml
    # Windows (PowerShell):
    notepad $env:USERPROFILE\.config\spren\config.toml
    ```
    *You'll typically need to add your API keys for the AI providers (like OpenAI or Anthropic) in this file.*

## Usage Examples

Interact with Spren using natural language queries prefixed by `spren` or within its interactive prompt:

Find large files:
```bash
spren show me files larger than 1GB in my home directory
