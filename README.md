<div align="center">
    <h1>srvpru</h1>
    <p>
        <img alt="Rust" src="https://img.shields.io/badge/rustc-1.96.0-blue?logo=rust&logoColor=white">
        <img alt="Preact" src="https://img.shields.io/badge/preact-10.29.7-673AB8?logo=preact&logoColor=white">
        <img alt="Ant Design" src="https://img.shields.io/badge/ant%20design-6.6.4-0170FE?logo=antdesign&logoColor=white">
    </p>
    <p><a href="README.md">English</a> · <a href="README.zh.md">中文</a></p>
</div>

---

> [!IMPORTANT]
> This project is still under development. Anything you see may change later.

srvpru is a duel server for [ygopro](https://github.com/mycard/ygopro). It provides matchmaking and various other features, plus a WebUI for administration.

srvpru is compatible with the vast majority of the [srvpro](https://github.com/mycard/srvpro) APIs.

## Features

- **Built-in web portal** — the admin UI (room list, history, announcements, shutdown and so on) is served by the same process on the same host; no separate frontend deployment is needed.
- **Live room list** — pushed over WebSocket, so room creation, changes and disappearance show up in the frontend immediately.
- **Hot-swappable plugins** — plugins register themselves at link time and can be enabled, disabled and reconfigured at runtime from the portal or the HTTP API, without restarting the server.
- **Embedded duel backend** — every room starts in-process; no external `ygopro` program is needed and no new process is spawned.
- **Accounts and permissions** — a SQLite-backed account database, JWT login and per-permission authorization.

## Requirements

`srvpru` works on top of the [ygopru](https://github.com/iamipanda/ygopru) project.

The ygopru engine reads its data from the working directory:

- `cards.cdb` — the card database, take one from [ygopro-database](https://github.com/mycard/ygopro-database); `expansions/*.cdb` are read as well if present
- `script/` — the Lua card scripts, take them from [ygopro-scripts](https://github.com/Fluorohydride/ygopro-scripts)
- `lflist.conf` — the ban list. Optional.

All of the above can be overridden with `system.conf` or `YGOPRO_*` environment variables. See the [ygopru documentation](https://github.com/IamIpanda/ygopru/blob/master/variables.md) for details.

## Building

Building needs a Rust toolchain (edition 2024), a C++14 compiler for ocgcore, and Node 22 for building the portal.

The embedded windbot additionally needs a .NET toolchain.

The portal is an npm workspace: it emits `srvpro-portal/dist` (the shell) and `srvpro-api/dist` (the per-plugin page bundles), and both have to be served from the same directory, `portal` by default.

```bash
npm ci
npm run build

mkdir -p portal
cp -r srvpro-portal/dist/. portal/
cp -r srvpro-api/dist/. portal/
cargo build --release -p srvpro-app
```

`srvpro-app` enables `api`, `zip`, `card`, `forge` and `official` by default. Building it with `--no-default-features --features zip,card,forge,official` produces a headless server without the HTTP API and the portal.

To ship a single binary instead, pack the merged directory first: the `embedded-portal` feature embeds the whole frontend into the executable at compile time.

```bash
cargo install web-static-pack-packer --version 0.5.3 --locked
web-static-pack-packer directory-single portal srvpro-portal.pack
cargo build --release -p srvpro-app --features embedded-portal
```

## Running

Prebuilt images are published on Docker Hub: `iami/srvpru`, which serves the HTTP API and the portal in addition to the ygopro protocol, and `iami/srvpru:headless`, which serves the ygopro protocol alone. Example:

```bash
git clone --depth 1 https://github.com/mycard/ygopro-database
git clone --depth 1 https://github.com/Fluorohydride/ygopro-scripts script
touch srvpro.db # docker would otherwise turn the mount point into a directory

# pull the image and run it
docker run -d --name srvpru -p 7911:7911 -p 7922:7922 \
    -v "$PWD/ygopro-database/locales/zh-CN/cards.cdb:/srvpro/cards.cdb" \
    -v "$PWD/script:/srvpro/script" \
    -v "$PWD/srvpro.db:/srvpro/srvpro.db" \
    iami/srvpru
```

`locales/zh-CN` can be replaced with any other locale directory of ygopro-database.

## Setup

On the first start srvpru creates the account database (`api_database`, default `srvpro.db`), its schema, a
token signing secret, and an `admin` account. If `bootstrap_password` is not set, a random password is
generated and written to the log:

```text
no initial password is configured, created the account admin in srvpro.db with the generated password <...>
```

Log in with it at `http://<host>:7922/` and change it in the settings to a password you like.

## Usage

The server is used almost exactly the same way as the [233 server](https://ygo233.com/usage).

The codes it adds on top of that:

| Code | Meaning |
| ---- | ------- |
| `BO<n>` | best of n |
| `NM` / `NOMASK` | do not mask the opponent's unknown cards |
| `G` / `GENESYS` | the Genesys ban list |

For example, `T,BO3#xxxx` creates a tag room, best of three, where `xxxx` is any valid room name.

Everything before the `#` is split on commas and read as codes; codes that are not recognised are simply ignored.

## Configuration

Configuration entries are merged from three sources, in increasing priority:

1. **Every file in the configuration directory** — `SRVPRO_CONFIG_PATH`, default `srvpro/config` in the source tree. `json`, `yml`, `yaml`, `toml`, `conf` and `ini` are supported, and each entry is named `<file name>_<key name>`, so `port = 7911` in `server.toml` becomes `server_port`.
2. **Every `SRVPRO_*` environment variable**.
3. **The `config` table of the database**.

See [`variables.md`](variables.md) for the full list of variables.

## HTTP API

Compatible with the original srvpro wherever possible, so the original dashboard still works. We recommend using this project's UI for configuration.

## Project structure

| Crate | Role |
| ----- | ---- |
| `srvpro` | the core server: players, rooms, plugins, game modes and the protocol plumbing |
| `srvpro-api` | the HTTP API: JWT authentication, the account database, the plugin and configuration endpoints, and hosting the portal |
| `srvpro-app` | the binary; wires the API in and runs the server |
| `srvpro-windbot` | the embedded windbot engine |
| `srvpro-portal` | the portal itself — Preact, Ant Design and Vite |

## Secondary development

`srvpro` can be used as an upstream library and integrated, referenced and repackaged by downstream plugin crates, making it easy to add the features you want.

Here is a downstream reference written by the author: [srvpro-iami](https://github.com/IamIpanda/srvpro-iami) (Chinese)

## Related projects

- [srvpro](https://github.com/mycard/srvpro) — the CoffeeScript server this project rewrites
- [ygopru](https://github.com/mycard/ygopru) — the Rust YGOPro server stack srvpru builds on
- [ygopro](https://github.com/mycard/ygopro) — the C++ YGOPro client
- [ygopro-core](https://github.com/Fluorohydride/ygopro-core) — the ocgcore duelling engine
- [windbot](https://github.com/jwyxym/windbot) — the bot engine used by the embedded windbot
