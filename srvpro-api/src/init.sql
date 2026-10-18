CREATE TABLE IF NOT EXISTS users (
    username TEXT PRIMARY KEY,
    password TEXT NOT NULL,
    permissions TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS secrets (
    name TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS duel_log (
  id             TEXT PRIMARY KEY,
  name           TEXT NOT NULL,
  duel_count     INTEGER NOT NULL,
  players        TEXT NOT NULL,
  start_time     TEXT NOT NULL,
  end_time       TEXT NOT NULL,
  team_a_score   INTEGER NOT NULL,
  team_b_score   INTEGER NOT NULL,
  replays        TEXT NOT NULL
);
