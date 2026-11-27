# srvpro-windbot

静态编译、内嵌的 [WindBot](https://github.com/IceYGO/windbot)。

本插件的对外接口与使用外置进程的 [`srvpro::windbot`](https://docs.rs/srvpro/latest/srvpro/windbot/index.html) 模块一致，将本 crate 链接进使用 `srvpro` 的crate 即可启用。

## 构建

本工程以 WindBot 的 [fork](https://github.com/jwyxym/windbot/tree/windbot-rs) 作为 git submodule 参与编译。编译已在以下环境中测试通过：

| 平台 | 目标 | 所需工具 |
|---|---|---|
| Linux | `x86_64-unknown-linux-gnu` | .NET 10 SDK、clang、zlib 头文件 |
| Linux (musl) | `x86_64-unknown-linux-musl` | .NET 10 SDK、musl 交叉工具链 |
| Windows | `x86_64-pc-windows-msvc` | .NET 10 SDK、MSVC C++ 工具链、Windows SDK；整个应用需 `-C target-feature=+crt-static` |
| macOS | `aarch64-apple-darwin` | .NET 10 SDK、Xcode 命令行工具 |

工程也允许使用外置静态库进行链接，这样可以直接使用 WindBot [发布的库](https://github.com/jwyxym/windbot/releases)。使用下列环境变量：

| 变量 | 含义 |
|---|---|
| `WINDBOT_NATIVE_DIR` | 预编译静态库目录，需包含 `WindBot.a`（Windows 为 `WindBot.lib`）、`runtime/` 和 `native-dependencies.txt` |
| `WINDBOT_SOURCE_DIR` | C# 源码目录，默认为本 crate 旁的 `windbot/` |
| `WINDBOT_DOTNET` | `dotnet` 可执行文件，默认为 `dotnet` |

## Features

GNU ld 2.39+ 与 lld 默认启用 `-z start-stop-gc`，会把 NativeAOT 的 `__modules` 段当作未使用的代码回收，导致链接时报 `undefined symbol: __start___modules`。常见的会出现该问题的环境包括：

+ 绝大多数 Linux 发行版
+ Android

`patch-archive` 会在链接前改写复制出的静态库，确保相关段落不会被回收。若你的 `build.rs` 中已自行传入该链接参数，可考虑关闭此 feature。

## 注

在 musl 目标上始终会从 GC 静态库中移除 `libunwind.cpp.o`，避免与 Rust 自带的 unwind 实现重复定义。
