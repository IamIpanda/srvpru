# Variables

Every setting is an `SRVPRO_*` environment variable. The same name without the prefix, lowercased, is the key
of the entry in a configuration file or in the `config` table of the database: `SRVPRO_TIP_INTERVAL` is
`tip_interval`. A plugin owns the entries whose key is prefixed with its name, so `srvpro::plugin::tip` owns
`tip_*` and `srvpro_api::roomlist` owns `http_*`.

Variables suit scalar values. Structured or multi-line ones are easier to write in a file, where the file name
is the key prefix and the indentation is free: `groups` in `bad_words.yaml`, `tips` in `tip.yaml`, `message` in
`welcome.yaml` — the entries the portal edits are the same ones.

The entries are merged, in increasing priority, from:

1. **Files** — every `json`, `yml`, `yaml`, `toml`, `conf` and `ini` file in the configuration directory
   (`SRVPRO_CONFIG_PATH`, default `srvpro/config`). An entry is named `<file name>_<key>`, so a `server.toml`
   holding `port = 7911` becomes `server_port`. Values that are not plain strings are flattened to JSON, which
   is how the YAML files holding nested plugin data work.
2. **The `SRVPRO_*` variables below.**
3. **The `config` table of the database** — what the portal writes to.

> [!WARNING]
> Anything changed in the UI is stored in the database, and the database always wins over the files and the
> variables. An entry edited there keeps overriding `SRVPRO_*` forever; to hand that entry back to the variable,
> delete it from the database and restart the server.

## Core

| Variable | Default | Description |
| -------- | ------- | ----------- |
| `SRVPRO_SERVER_PORT` | `7911` | The port the ygopro protocol listens on |
| `SRVPRO_API_PORT` | `7922` | The port the HTTP API and the portal listen on |
| `SRVPRO_API_DATABASE` | `srvpro.db` | The SQLite account database |
| `SRVPRO_API_STATIC_DIR` | `portal` | The directory the portal is served from |
| `SRVPRO_API_NAME` | `Srvpru Server` | The server name shown in the portal |
| `SRVPRO_AUTH_TOKEN_TTL_DAYS` | `180` | How long a login token stays valid |
| `SRVPRO_BOOTSTRAP_USERNAME` | `admin` | The account created on the first start |
| `SRVPRO_BOOTSTRAP_PASSWORD` | empty | The password of that account; a random one is generated and logged when empty |
| `SRVPRO_ENABLE_PLUGINS` | empty | Comma-separated module names added to the enabled plugins at start |
| `SRVPRO_DISABLE_PLUGINS` | empty | Comma-separated module names removed from the enabled plugins at start |
| `SRVPRO_HTTP_SHOW_IP` | `true` | Show player addresses in the room list |
| `SRVPRO_HTTP_SHOW_INFO` | `true` | Show per-player score, LP and hand size in the room list |
| `SRVPRO_HTTP_PUBLIC_ROOMLIST` | `false` | Let unauthenticated callers read the room list |

The two plugin lists are read before the `config` table is merged, so that table cannot override them. What the
portal switches instead are `api_enabled_plugins` and `api_disabled_plugins`, the entries of that table it writes
itself.

## Plugins

| Variable | Default | Description |
| -------- | ------- | ----------- |
| `SRVPRO_BAD_WORDS_CHECK_ROOM_NAME` | `true` | Check room names |
| `SRVPRO_BAD_WORDS_CHECK_PLAYER_NAME` | `true` | Check player names |
| `SRVPRO_BAD_WORDS_GROUPS` | `{}` | `{"<group>": {"strategy": "block"\|"warn"\|"echo"\|"kick", "words": ["..."]}}`; prefer `groups` in `bad_words.yaml` |
| `SRVPRO_BAD_WORDS_LENGTH_STRATEGY` | `warn` | Applied to a room name, a player name or a message that is too long |
| `SRVPRO_BAD_WORDS_MAX_LENGTH` | `0` | Longest accepted text in characters; `0` accepts any length |
| `SRVPRO_CLOUD_REPLAY_PROVIDER` | empty | `file` or `api`; the provider that plays `R` rooms back |
| `SRVPRO_DECK_REPORT_URL` | empty | Where the deck report of a duel is posted |
| `SRVPRO_DECK_REPORT_ACCESS_KEY` | empty | Bearer token sent to that report endpoint |
| `SRVPRO_DECK_REPORT_ARENA` | empty | Arena name sent with the report |
| `SRVPRO_DIALOGUES_DIALOGUES` | `{}` | `{"<card code>": ["<line>", ...]}`; prefer `dialogues` in `dialogues.yaml` |
| `SRVPRO_DUEL_LOG_ALWAYS_SAVE_REPLAY_TO_DATABASE` | `false` | Store every replay inline in the duel log instead of referencing a saved replay file |
| `SRVPRO_HAISUI_LP_LOW_MESSAGE_WHEN_DAMAGED` | 你的生命已经如风中残烛了！ | Sent when a damage leaves a player at 100 LP or less |
| `SRVPRO_HAISUI_LP_LOW_MESSAGE_WHEN_PAID_COST` | 背水一战！ | Sent when paying a cost does the same |
| `SRVPRO_HIDE_NAME_MODE` | `disabled` | Mask opponent names in random rooms; `disabled`, `start` or `always` |
| `SRVPRO_MINILUV_JOIN_TIMEOUT` | `5` | Seconds a connection may stay before it joins a room; `0` disables the check |
| `SRVPRO_MINILUV_MAX_CONNECTIONS_PER_IP` | `5` | Connections one address may hold at once; `0` disables the check |
| `SRVPRO_RANDOM_MATCH_DEFAULT` | `S` | The mode used when the room field is empty |
| `SRVPRO_RANDOM_MATCH_MODES` | `[]` | The modes a random room may be created with, e.g. `["S", "M", "T"]`; prefer `modes` in `random_match.yaml` |
| `SRVPRO_RANDOM_MATCH_MIN_SURRENDER_TURN` | `3` | Turns within which a random room refuses surrender; `0` disables |
| `SRVPRO_RANDOM_MATCH_READY_TIMEOUT` | `90` | Seconds a player in a random room has to ready up |
| `SRVPRO_RANDOM_MATCH_START_TIMEOUT` | `20` | Seconds a filled random room waits before it starts |
| `SRVPRO_RANDOM_MATCH_SURRENDER_LIMITED_MODES` | `["S","T"]` | The random room types that refusal applies to |
| `SRVPRO_RECONNECT_WAIT_TIME` | `60` | Seconds a disconnected player has to come back |
| `SRVPRO_RECONNECT_AUTO_SURRENDER_AFTER_DISCONNECT` | `false` | Surrender on the disconnected player's behalf |
| `SRVPRO_RECONNECT_ALLOW_KICK_RECONNECT` | `false` | Let a reconnect take the seat of the player still in it |
| `SRVPRO_RECOVER_PROVIDER` | empty | `file` or `api`; the provider that reads the replay of an `RC` room |
| `SRVPRO_REPORT_URL` | empty | Where the result of a duel is posted |
| `SRVPRO_REPORT_ACCESS_KEY` | empty | Bearer token sent to that report endpoint |
| `SRVPRO_REPORT_ARENA` | empty | Arena name sent with the report |
| `SRVPRO_REPORT_FORMAT` | `forge` | Replay format the report embeds; `ygopro`, `forge`, `srvpro` or `raw` |
| `SRVPRO_RETRY_HANDLE_MAX_RETRY_COUNT` | `5` | Retries allowed for a failing duel start |
| `SRVPRO_SIDE_TIMEOUT_SIDE_TIMEOUT` | `3` | Minutes allowed for the side deck change |
| `SRVPRO_STOP_MESSAGE` | empty | Shown to a player whose join is refused while the server is stopped; set by `/api/stop` |
| `SRVPRO_TIP_INTERVAL` | `120000` | Milliseconds between two tips |
| `SRVPRO_TIP_PREFIX` | `Tip: ` | Prefix of every tip line |
| `SRVPRO_TIP_TIPS` | `[]` | The tip lines, e.g. `["...", "..."]`; prefer `tips` in `tip.yaml` |
| `SRVPRO_WELCOME_MESSAGE` | a Chinese greeting | Sent to a player when they join; prefer `message` in `welcome.yaml` |

`recover` and `cloud_replay` read a replay through a provider, and the providers keep entries of their own. The
`file` providers take `SRVPRO_FILE_PATH` (default `replays`, the directory replays are read from and saved to),
`SRVPRO_FILE_MACHINE` (default `0`, the machine number a saved replay id is built from) and
`SRVPRO_FILE_LIST_LIMIT` (default `10`, how many replays a search returns). The `api` providers take
`SRVPRO_API_LIST_URL` and `SRVPRO_API_REPLAY_URL` (`{id}` in the former is replaced by the search key and in the
latter by the replay id) and `SRVPRO_API_AUTH`, the `Authorization` header sent with the requests. The two
plugins share these names, so one `file_path` sets the directory of both.

Both plugins are off by default: nothing is recorded, and the `R` and `RC` rooms do not exist. Turning one on
means enabling the plugin itself (`srvpro::plugin::cloud_replay`, `srvpro::plugin::recover`), enabling the
provider it should use (`srvpro::plugin::cloud_replay::file`, `srvpro::plugin::recover::api`, ...), and naming
that provider in the plugin's own `provider` entry.

The `death` plugin reads no variable: its rule is fixed, and a death match is started through `/api/death`.

## Engine

The duelling engine reads its own configuration from `system.conf` (or `YGOPRO_CONFIG_PATH`) and from the
`YGOPRO_*` variables, independently of the above:

| Variable | Default | Description |
| -------- | ------- | ----------- |
| `YGOPRO_PATH` | `./` | Base directory card scripts are resolved against |
| `YGOPRO_DB_PATH` | `cards.cdb, expansions/*.cdb` | Card databases, comma-separated, globs allowed |
| `YGOPRO_LFLIST_PATH` | `expansions/lflist.conf, lflist.conf` | Ban lists, comma-separated, globs allowed |

## Environment

| Variable | Description |
| -------- | ----------- |
| `SRVPRO_CONFIG_PATH` | The configuration directory, default `srvpro/config` |
| `YGOPRO_CONFIG_PATH` | The engine configuration file, default `system.conf` |
