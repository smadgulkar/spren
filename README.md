# Spren - AI-Powered Shell Assistant

Spren is an intelligent command-line assistant that helps you work with shell commands across different platforms.

## Features

- Natural language to shell command conversion
- Support for Bash, PowerShell, and CMD
- Intelligent error analysis and suggestions
- Safe command execution with confirmation
- Cross-platform support

## Installation

### Linux/macOS
```bash
# Extract the archive
tar -xzf spren-linux-x86_64.tar.gz

# Run the installation script
./install.sh
```

### Windows
1. Extract the ZIP file
2. Run install.bat as administrator
3. Add the installation directory to your PATH

## Configuration

The configuration file is located at:
- Linux/macOS: `~/.config/spren/config.toml`
- Windows: `%USERPROFILE%\.config\spren\config.toml`

You'll need to add your API keys to the configuration file:
```toml
[ai]
anthropic_api_key = "your-key-here"
# or
openai_api_key = "your-key-here"
```

## Usage

Simply type your command in natural language:
```bash
spren> show me all pdf files in the current directory
```

Spren will suggest the appropriate command and ask for confirmation before execution.

## License

MIT License - See LICENSE file for details