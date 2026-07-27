# cloudflare-cli

[![CI](https://github.com/Osso/cloudflare-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/Osso/cloudflare-cli/actions/workflows/ci.yml)
[![GitHub release](https://img.shields.io/github/v/release/Osso/cloudflare-cli)](https://github.com/Osso/cloudflare-cli/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

CLI for managing Cloudflare Zero Trust (tunnels, access apps, gateway rules).

## Installation

```bash
cargo install --path .
```

## Setup

```bash
cloudflare config
```

## Usage

```bash
cloudflare tunnels      # Manage Cloudflare Tunnels
cloudflare tunnels set-origin-server-name <tunnel> <hostname> <origin-server-name>
cloudflare apps         # Manage Access Applications
cloudflare gateway      # Manage Gateway firewall rules
```

## License

MIT
