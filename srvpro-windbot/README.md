# srvpro-windbot

A statically compiled, embedded [WindBot](https://github.com/IceYGO/windbot).

The plugin exposes the same interface as the [`srvpro::windbot`](https://docs.rs/srvpro/latest/srvpro/windbot/index.html) module, which drives WindBot as a separate process. Link this crate into a crate that already uses `srvpro` to enable it.

## Building

The WindBot [fork](https://github.com/jwyxym/windbot/tree/windbot-rs) is compiled from a git submodule. The following environments have been tested:

| Platform | Target | Required tools |
|---|---|---|
| Linux | `x86_64-unknown-linux-gnu` | .NET 10 SDK, clang, zlib headers |
| Linux (musl) | `x86_64-unknown-linux-musl` | .NET 10 SDK, musl cross toolchain |
| Windows | `x86_64-pc-windows-msvc` | .NET 10 SDK, MSVC C++ tools, Windows SDK; the whole application requires `-C target-feature=+crt-static` |
| macOS | `aarch64-apple-darwin` | .NET 10 SDK, Xcode command line tools |

A prebuilt static library can be linked instead, so the libraries published by WindBot can be used directly. The following environment variables are recognized:

| Variable | Meaning |
|---|---|
| `WINDBOT_NATIVE_DIR` | Prebuilt library directory containing `WindBot.a` (`WindBot.lib` on Windows), `runtime/` and `native-dependencies.txt` |
| `WINDBOT_SOURCE_DIR` | C# source directory, defaults to `windbot/` next to this crate |
| `WINDBOT_DOTNET` | `dotnet` executable, defaults to `dotnet` |

## Features

GNU ld 2.39+ and lld enable `-z start-stop-gc` by default, which treats the NativeAOT `__modules` section as unused code and drops it, so linking fails with `undefined symbol: __start___modules`. Affected environments include:

+ most Linux distributions
+ Android

`patch-archive` rewrites the copied static library before linking so the section is retained. If your `build.rs` already passes the link argument itself, consider disabling this feature.

## Note

On musl targets `libunwind.cpp.o` is always removed from the GC static library, to avoid duplicate definitions with the unwind implementation shipped by Rust.
