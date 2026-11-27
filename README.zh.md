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

srvpru 是一个 [ygopro](https://github.com/mycard/ygopro) 的决斗服务器，提供匹配以及各种其他功能。并额外提供一个 WebUI 来进行管理。

srvpru 与 [srvpro](https://github.com/mycard/srvpro) 绝大部分API均兼容。

## 特性

- **内置网页后台** —— 管理界面（房间列表、历史记录、公告、停服等）由同一个进程在同一台主机上提供，不需要再部署一套前端。
- **实时房间列表** —— 通过 WebSocket 推送，房间的创建、变化与消失会立刻反映到前端。
- **可热插拔的插件** —— 插件在链接期自注册，运行时可以随时从后台或 HTTP API 启用、禁用、重新配置，不需要重启服务器。
- **内嵌决斗后端** —— 房间均在进程内启动，不再依赖外部的`ygopro`程序，也不会产生新进程。
- **账号与权限** —— 基于 SQLite 的账号库，JWT 登录，按权限授权。

## 运行要求

`srvpru`基于[ygopru](https://github.com/iamipanda/ygopru)项目工作。

ygopru引擎在工作目录读取数据：

- `cards.cdb` —— 卡片数据库，可以从 [ygopro-database](https://github.com/mycard/ygopro-database) 获取；
  有 `expansions/*.cdb` 的话也会一起读取
- `script/` —— Lua 卡片脚本，可以从 [ygopro-scripts](https://github.com/Fluorohydride/ygopro-scripts) 获取
- `lflist.conf` —— 禁限表。可选。

以上配置按可以用 `system.conf` 或 `YGOPRO_*` 环境变量覆盖。具体可参考[ygopru文档](https://github.com/IamIpanda/ygopru/blob/master/variables.md)。


## 编译

编译需要 Rust 工具链（edition 2024）、编译 ocgcore 用的 C++14 编译器，以及构建后台用的 Node 22。

内嵌的windbot额外需要.Net工具链。

后台是一个 npm workspace，它会产出 `srvpro-portal/dist`（站点本体）与 `srvpro-api/dist`（各插件页面的包），两者必须由同一个目录提供，默认是 `portal`。

```bash
npm ci
npm run build

mkdir -p portal
cp -r srvpro-portal/dist/. portal/
cp -r srvpro-api/dist/. portal/
cargo build --release -p srvpro-app
```

`srvpro-app` 默认开启 `api`、`zip`、`card`、`forge`、`official` 五个 feature。用 `--no-default-features --features zip,card,forge,official` 编译可以得到一个不带 HTTP API 与后台的纯服务端。

想只发一个二进制的话，先把合并后的目录打包：`embedded-portal` 特性会在编译期把整个前端嵌入可执行文件。

```bash
cargo install web-static-pack-packer --version 0.5.3 --locked
web-static-pack-packer directory-single portal srvpro-portal.pack
cargo build --release -p srvpro-app --features embedded-portal
```

## 运行

预编译镜像已发布在 Docker Hub 上：`iami/srvpru`（除 ygopro 协议外还提供 HTTP API 与后台）与
`iami/srvpru:headless`（只提供 ygopro 协议）。运行示例：

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

用它登录 `http://<host>:7922/`，并在设置中修改为你喜欢的密码。

## 使用方法

服务器使用方法与 [233 服](https://ygo233.com/usage) 几乎完全相同，

在此之上新增的代码：

| 代码 | 含义 |
| ---- | ---- |
| `BO<n>` | 几局几胜 |
| `NM` / `NOMASK` | 不遮挡对手的未知卡片 |
| `G` / `GENESYS` | Genesys 禁卡表 |

例如 `T,BO3#xxxx` 会建立一个三局两胜的双打房，其中 `xxxx` 是任意合法房间名。

`#` 之前的部分按逗号拆开当作代码，无法识别的代码会被直接忽略。

## 配置

配置项由三个来源合并，优先级从低到高：

1. **配置目录里的每个文件** —— `SRVPRO_CONFIG_PATH`，源码树里默认 `srvpro/config`。支持 `json`、`yml`、`yaml`、
   `toml`、`conf`、`ini`，每个配置项的名字是 `<文件名>_<键名>`，所以 `server.toml` 里的 `port = 7911` 会变成 `server_port`。
2. **每个 `SRVPRO_*` 环境变量**。
3. **数据库的 `config` 表**。

完整变量清单见 [`variables.md`](variables.md)。

## HTTP API

与原版 srvpro 尽可能兼容，因此原版的 dashboard 依然可以使用。我们推荐使用本工程的 UI 进行配置。

## 工程结构

| crate | 职责 |
| ----- | ---- |
| `srvpro` | 服务端核心：玩家、房间、插件、游戏模式以及协议管线 |
| `srvpro-api` | HTTP API：JWT 鉴权、账号库、插件与配置接口，以及托管后台 |
| `srvpro-app` | 可执行文件，接上 API 并跑起服务器 |
| `srvpro-windbot` | 内嵌的Windbot引擎 |
| `srvpro-portal` | 站点本体，Preact + Ant Design + Vite |

## 二次开发
`srvpro` 可以作为上游库被下游插件库集成，引用和重新打包，方便地添加自己想要的功能。

这里是一份作者自己的下游参考：[srvpro-iami](https://github.com/IamIpanda/srvpro-iami) (中文)

## 相关工程

- [srvpro](https://github.com/mycard/srvpro) —— 本工程重写的 CoffeeScript 原版
- [ygopru](https://github.com/mycard/ygopru) —— srvpru 所依赖的 Rust YGOPro 服务端栈
- [ygopro](https://github.com/mycard/ygopro) —— C++ 版 YGOPro 客户端
- [ygopro-core](https://github.com/Fluorohydride/ygopro-core) —— ocgcore 决斗引擎
- [windbot](https://github.com/jwyxym/windbot) —— 内嵌windbot使用的bot引擎
