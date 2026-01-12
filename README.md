# jamjam

P2P Audio Communication for Musicians

Low-latency peer-to-peer audio communication application for macOS, Windows, and Linux.

## Documentation

- [Documentation Site](https://koedame.github.io/p2paudiosession/) - Getting started, installation guides, and development documentation

## Features

- Cross-platform support (macOS, Windows, Linux)
- Low-latency audio streaming
- Peer-to-peer connection (no central server required for audio)
- Multiple audio codec support (Opus, PCM)

## Development

See the [documentation site](https://koedame.github.io/p2paudiosession/) for detailed development guides.

### Quick Start

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/koedame/p2paudiosession.git
cd p2paudiosession

# Build core library
cargo build

# Run tests
cargo test
```

### Running the Desktop App (Tauri)

```bash
# Run from project root (not src-tauri directory)

# Development mode (frontend server starts automatically)
cargo tauri dev

# Production build
cargo tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

## License

MIT License
