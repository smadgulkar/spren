# 🤖 Spren

A smart shell assistant that converts natural language into shell commands.

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#configuration">Configuration</a> •
  <a href="#usage">Usage</a> •
  <a href="#license">License</a>
</p>

## About

Spren is an intelligent command-line assistant that translates natural language into shell commands. Whether you're a CLI novice or expert, Spren helps you work more efficiently by understanding your intent and suggesting the right commands.

## Features

- 🤖 Natural language to shell command conversion
- 🔄 Cross-platform support (Windows, Linux, macOS)
- 🛡️ Safe execution with command previews and confirmations
- 🧠 Intelligent error analysis and suggestions
- ⚡ Support for multiple shells (Bash, PowerShell, CMD)

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

3. Add your LLM API key:
   ```toml
   [ai]
   provider = "anthropic"  # or "openai"
   anthropic_api_key = "your-api-key-here"
   # or
   openai_api_key = "your-api-key-here"
   ```

## Usage

Simply describe what you want to do:

```bash
$ spren find all pdf files modified in the last week

📝 Command: find . -name "*.pdf" -mtime -7

Would you like to execute this command? [y/N]:
```

Spren will:
1. Understand your intent
2. Generate the appropriate command
3. Show you a preview
4. Ask for confirmation before execution

## Examples

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
- [Anthropic's Claude](https://www.anthropic.com/)
- [OpenAI](https://openai.com/)