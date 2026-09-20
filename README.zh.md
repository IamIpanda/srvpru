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
> 本工程仍在开发中，你所见的一切都可能发生变化。
> 本文档由 AI 生成，可能包含错误与未完成的功能。

srvpru 是一个 [ygopro](https://github.com/mycard/ygopro) 的决斗服务器，提供匹配以及各种其他功能。并额外提供一个 WebUI 来进行管理。

srvpru 与 [srvpro](https://github.com/mycard/srvpro) 绝大部分的兼容。

## 特性

- **内置网页后台** —— 管理界面（房间列表、玩家、实时比分、插件开关、账号与配置）由同一个进程在同一台主机上提供。
  不需要 PHP，不需要再部署一套前端，也没有一个和后端对不上的配置文件。
- **实时房间列表** —— 通过 WebSocket 推送，房间的创建、变化与消失会立刻反映到浏览器，不轮询。
- **插件可热插拔** —— 插件在链接期自注册，运行时可以随时从后台或 HTTP API 启用、禁用、重新配置，不用重启服务器；
  插件依赖会自动带上。
- **两种决斗后端** —— 每个房间既可以跑进程内的 ygopru `DuelHost`，也可以拉起外部的 `ygopro` 二进制，按房间选择。
- **账号与权限** —— 基于 SQLite 的账号库，JWT 登录，按权限授权。
- **不用改文件就能配置** —— 配置文件、`SRVPRO_*` 环境变量与数据库合并成一份配置；后台里的改动立即生效，并且能挺过重启。
- **srvpro 协议的超集** —— 房间名、模式选项、加入/离开/重连流程，以及整套 STOC/CTOS 消息集，行为与 srvpro 玩家期待的一致。

## 运行要求

引擎按工作目录读取数据：

- `cards.cdb` —— 卡片数据库，可以从 [ygopro-database](https://github.com/mycard/ygopro-database) 获取；
  有 `expansions/*.cdb` 的话也会一起读取
- `script/` —— Lua 卡片脚本，可以从 [ygopro-scripts](https://github.com/Fluorohydride/ygopro-scripts) 获取
- `lflist.conf`（可选；会优先尝试 `expansions/lflist.conf`）

这些路径来自引擎自己的配置，可以用 `system.conf` 或 `YGOPRO_*` 环境变量覆盖。

编译需要 Rust 工具链（edition 2024）、编译 ocgcore 用的 C++14 编译器，以及构建后台用的 Node 22。

## 编译

```bash
cargo build --release -p srvpro-app

cd srvpro-portal
npm ci
npm run build # → srvpro-portal/dist
```

`srvpro-app` 默认开启 `api`、`zip`、`card` 三个 feature。用 `--no-default-features --features zip,card`
编译可以得到一个不带 HTTP API 与后台的纯服务端。

## 运行

预编译镜像已发布在 Docker Hub 上：`iami/srvpru`。先把数据放到旁边：

```bash
git clone --depth 1 https://github.com/mycard/ygopro-database
git clone --depth 1 https://github.com/Fluorohydride/ygopro-scripts script
touch srvpro.db # 不先创建的话 docker 会把挂载点变成一个目录

# 拉取镜像并运行
docker run -d --name srvpru -p 7911:7911 -p 7922:7922 \
    -v "$PWD/ygopro-database/locales/zh-CN/cards.cdb:/srvpro/cards.cdb" \
    -v "$PWD/script:/srvpro/script" \
    -v "$PWD/srvpro.db:/srvpro/srvpro.db" \
    iami/srvpru
```

`locales/zh-CN` 可以换成 ygopro-database 里的任意其他语言目录。

## 初次设置

首次启动时 srvpru 会创建账号库（`api_database`，默认 `srvpro.db`）、建表、生成签名密钥，并创建一个 `admin`
账号。如果没有配置 `bootstrap_password`，会随机生成一个密码写进日志：

```text
no initial password is configured, created the account admin in srvpro.db with the generated password <...>
```

用它登录 `http://<host>:7922/`，并在那里改掉。

## 使用方法

用法与 [233 服](https://ygo233.com/usage) 几乎完全相同：房名代码照旧生效，`M#xxxx` 是比赛房，`T#xxxx` 是双打房。

在此之上新增的代码：

| 代码 | 含义 |
| ---- | ---- |
| `BO<n>` | 几局几胜 |
| `NM` / `NOMASK` | 不遮挡对手的未知卡片 |
| `E<n>` / `ENGINE<n>` | 决斗后端：`1` 内嵌，`2` 外部 `./ygopro` 二进制 |
| `G` / `GENESYS` | Genesys 禁卡表 |

例如 `T,BO3#xxxx` 会建立一个三局两胜的双打房，其中 `xxxx` 是任意合法房间名。

`#` 之前的部分按逗号拆开当作代码，无法识别的代码会被直接忽略。玩家输入的那串字符串本身就是房间标识：房间在
第一次加入时创建，之后加入的人都必须输入一模一样的字符串。`#` 之后的房间名里如果出现 `$`，`$` 之后的部分就是
密码：后台会把该房间标记为需要密码，而提供给匿名访问者的房间列表会隐藏 `$` 之后的内容。

## 配置

配置项由三个来源合并，优先级从低到高：

1. **配置目录里的每个文件** —— `SRVPRO_CONFIG_PATH`，源码树里默认 `srvpro/config`。支持 `json`、`yml`、`yaml`、
   `toml`、`conf`、`ini`，每个配置项的名字是 `<文件名>_<键名>`，所以 `server.toml` 里的 `port = 7911` 会变成 `server_port`。
2. **每个 `SRVPRO_*` 环境变量**，转小写。
3. **数据库的 `config` 表**，也就是后台写入的地方。

完整变量清单见 [`variables.md`](variables.md)。

## HTTP API

与原版 srvpro 兼容，因此原版的 dashboard 依然可以使用。我们推荐使用 UI 进行配置。

## 工程结构

| crate | 职责 |
| ----- | ---- |
| `srvpro` | 服务端核心：玩家、房间、插件、游戏模式以及协议管线 |
| `srvpro-api` | HTTP API：JWT 鉴权、账号库、插件与配置接口，以及托管后台 |
| `srvpro-app` | 可执行文件，接上 API 并跑起服务器 |
| `srvpro-portal` | 后台本体，Preact + Ant Design + Vite |

## 相关工程

- [srvpro](https://github.com/mycard/srvpro) —— 本工程重写的 CoffeeScript 原版
- [ygopru](https://github.com/mycard/ygopru) —— srvpru 所依赖的 Rust YGOPro 服务端栈
- [ygopro](https://github.com/mycard/ygopro) —— C++ 版 YGOPro 客户端
- [ygopro-core](https://github.com/Fluorohydride/ygopro-core) —— ocgcore 决斗引擎
