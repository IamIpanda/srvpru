#![allow(dead_code)]

use std::fs::File;
use std::collections::HashMap;

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use async_trait::async_trait;

use crate::ygopro::message::HostInfo;
use crate::srvpru::Configuration;
use crate::srvpru::plugins;

#[derive(serde::Serialize, serde::Deserialize)]
struct SrvproConfiguration {
    file: String,
    port: u16,
    version: u16,
    hostinfo: HostInfo,
    modules: HashMap<String, Value>,
    ban: HashMap<String, Value>
}
#[async_trait]
trait SrvproModuleConfiguration : DeserializeOwned {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>);
}

pub fn configuration_path() -> String {
    std::env::var("SRVPRO_CONFIG_PATH").unwrap_or("./srvpro_config".to_string())
}

pub async fn generate_srvpru_configuration() {
    let configuration = match load_configuration::<SrvproConfiguration>("config") {
        Ok(config) => config,
        Err(_) => return,
    };
    info!("Generating configuration from srvpro.");
    let mut srvpru_configuration = Configuration::default();
    // At this stage, dependecy hasn't been registered.
    // So just take all of them in.
    srvpru_configuration.plugins.push("position_recorder".to_string());
    srvpru_configuration.plugins.push("deck_recorder".to_string());
    srvpru_configuration.plugins.push("stage_recorder".to_string());
    srvpru_configuration.plugins.push("chat_command".to_string());
    srvpru_configuration.plugins.push("api".to_string());
    let mut srvpru_plugin_configurations: HashMap<String, Value> = HashMap::new();

    srvpru_configuration.ygopro.host_info = configuration.hostinfo;
    write_to_file("version_checker", &plugins::version_checker::Configuration { version: configuration.version });

    for (name, module_config) in configuration.modules {
        let config = &mut srvpru_configuration;
        let plugins = &mut srvpru_plugin_configurations;
        let module_name = name.as_str();
        match module_name {
            "stop"                => if module_config == Value::Bool(true) { panic!("server not open due to srvpro config: stop") },

            "welcome"             => commit_srvpro_plugin::<WelcomeConfig>      (config, plugins, module_name, module_config).await,
            "max_rooms_count"     => commit_srvpro_plugin::<MaxRoomsCountConfig>(config, plugins, module_name, module_config).await,
            "side_timeout"        => commit_srvpro_plugin::<SideTimeoutConfig>  (config, plugins, module_name, module_config).await,
            "replay_deplay"       => commit_srvpro_plugin::<ReplayDelayConfig>  (config, plugins, module_name, module_config).await,
            "hide_name"           => commit_srvpro_plugin::<HideNameConfig>     (config, plugins, module_name, module_config).await,
            "display_watchers"    => {},
            "tips"                => commit_srvpro_plugin::<TipsConfig>         (config, plugins, module_name, module_config).await,
            "dialogues"           => commit_srvpro_plugin::<DialoguesConfig>    (config, plugins, module_name, module_config).await,
            "random_duel"         => commit_srvpro_plugin::<RandomDuelConfig>   (config, plugins, module_name, module_config).await,
            "cloud_replay"        => commit_srvpro_plugin::<CloudReplayConfig>  (config, plugins, module_name, module_config).await,
            "windbot"             => commit_srvpro_plugin::<WindbotConfig>      (config, plugins, module_name, module_config).await,
            "reconnect"           => commit_srvpro_plugin::<ReconnectConfig>    (config, plugins, module_name, module_config).await,
            "heartbeat_detection" => commit_srvpro_plugin::<HeartbeatConfig>    (config, plugins, module_name, module_config).await,
            "mycard"              => commit_srvpro_plugin::<MycardConfig>       (config, plugins, module_name, module_config).await,
            "deck_log"            => commit_srvpro_plugin::<DeckLogConfig>      (config, plugins, module_name, module_config).await,
            "big_brother"         => commit_srvpro_plugin::<BigBrotherConfig>   (config, plugins, module_name, module_config).await,
            "arena_mode"          => commit_srvpro_plugin::<ArenaModeConfig>    (config, plugins, module_name, module_config).await,
            //"tournament_mode"     => {},

            "update"              => warn!("Srvpru don't process config.modules.update, please set this message in i18n."),
            "wait_update"         => warn!("Srvpru don't prcess config.modules.wait_update, please set this message in i18n."),
            "full"                => warn!("Srvpru don't prcess config.modules.full, please set this message in i18n,"),
            "i18n"                => warn!("Srvpru don't support i18n geo configuration for now."),

            // "mysql" => {},
            // "chat_color" => {},
            // "retry_handle" => {},
            // "challonge" => {},
            // "athletic_check" => {},
            // "test_mode" => {},
            // "pre_util" => {},
            // "update_util" => {},
            // "webhook" => {},

            _ => {
                warn!("Srvpro module {} is not supported, configuration on this field will be ignored.", name);
                continue;
            }
        }
    }
    
    write_configs(srvpru_configuration, srvpru_plugin_configurations);
}

fn load_configuration<T: serde::de::DeserializeOwned>(name: &str) -> anyhow::Result<T> {
    let configuration_dir = configuration_path();
    let path = std::path::Path::new(&configuration_dir).join(name.to_string() + ".json");
    let file = std::fs::File::open(&path)?;
    return Ok(serde_json::from_reader::<_, T>(file)?)
}

fn write_configs(config: Configuration, plugins: HashMap<String, Value>) {
    write_to_file("srvpru", &config);
    for (name, value) in plugins.into_iter() {
        write_to_file(&name, &value);
    }
}

fn write_to_file<T: serde::Serialize>(name: &str, value: &T) {
    let path = std::path::Path::new(&crate::srvpru::configuration_path()).join(name.to_string() + ".json");
    serde_json::to_writer_pretty(&File::create(path).expect("Failed to open file"), value).expect(&format!("Write {} failed.", name));
}

async fn commit_srvpro_plugin<T: SrvproModuleConfiguration>(config: &mut Configuration, plugins: &mut HashMap<String, Value>, module_name: &str, module_config: Value) {
    let module_config = serde_json::from_value::<T>(module_config).expect(&format!("Failed to deserialize module {}", module_name));
    module_config.fit_srvpru(config, plugins).await
}

fn commit_srvpru_plugin<T: serde::Serialize>(config: &mut Configuration, plugins: &mut HashMap<String, Value>, plugin_name: &str, plugin_config: T) {
    if ! config.plugins.contains(&plugin_name.to_string()) { config.plugins.push(plugin_name.to_string()); }
    let value = serde_json::to_value(plugin_config).expect(&format!("Failed to serialize config file on plugin {}", plugin_name));
    merge_json_value(plugins.entry(plugin_name.to_string()).or_insert(Value::Null), &value);
}

fn merge_json_value(base: &mut Value, addition: &Value) {
    match (base, addition) {
        (&mut Value::Object(ref mut base), &Value::Object(ref addition)) => {
            for (key, value) in addition {
                merge_json_value(base.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
        (base, addition) => {
            *base = addition.clone();
        }
    }
}

async fn download_from_value<T: DeserializeOwned + Default>(value: Value) -> Option<T> {
    match value {
        Value::String(source) => {
            info!("Downloading {}", source);
            Some(reqwest::get(source).await.ok()?.json::<T>().await.ok()?)
        },
        _ => None
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct WelcomeConfig {
    message: String
}

#[async_trait]
impl SrvproModuleConfiguration for WelcomeConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        commit_srvpru_plugin(config, plugins, "welcome", plugins::welcome::Configuration {
            welcome_message: self.message
        })
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct MaxRoomsCountConfig {
    max_rooms_count: usize 
}

#[async_trait]
impl SrvproModuleConfiguration for MaxRoomsCountConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        if self.max_rooms_count > 0 {
            commit_srvpru_plugin(config, plugins, "max_rooms_count", plugins::max_rooms_count::Configuration {
                max_rooms_count: self.max_rooms_count
            })
        }
    }
}


#[derive(Deserialize)]
#[serde(transparent)]
struct SideTimeoutConfig {
    value: Value
}

#[async_trait]
impl SrvproModuleConfiguration for SideTimeoutConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        let change_side = match self.value {
            Value::Number(minute) => minute,
            _ => return
        };
        commit_srvpru_plugin(config, plugins, "must_start", HashMap::from([
            ("change_side", change_side)
        ]));
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct ReplayDelayConfig {
    enabled: bool
}

#[async_trait]
impl SrvproModuleConfiguration for ReplayDelayConfig {
    async fn fit_srvpru(self, config: &mut Configuration, _: &mut HashMap<String, Value>) {
        if self.enabled {
            config.plugins.push("delayed_replay".to_string());
        }
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct HideNameConfig {
    enabled: bool
}

#[async_trait]
impl SrvproModuleConfiguration for HideNameConfig {
    async fn fit_srvpru(self, config: &mut Configuration, _: &mut HashMap<String, Value>) {
        if self.enabled {
            config.plugins.push("anonymous_opponent".to_string())
        }
    }
}

#[derive(Deserialize)]
struct TipsConfig {
    enabled: bool,
    //#[serde(deserialize_with="deserialize_srvpro_value")]
    get: Value,
    interval: u64,
    interval_ingame: u64
}


#[async_trait]
impl SrvproModuleConfiguration for TipsConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        if ! self.enabled { return; }
        let tips = download_from_value::<Vec<String>>(self.get).await.expect("Download tips failed.");
        commit_srvpru_plugin(config, plugins, "tip", plugins::tip::Configuration {
            tips,
            interval_when_prepare: self.interval,
            interval_when_in_game: self.interval_ingame,
        });
    }
}

#[derive(Deserialize)]
struct DialoguesConfig {
    enabled: bool,
    get: Value 
}

#[async_trait]
impl SrvproModuleConfiguration for DialoguesConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        if ! self.enabled { return; }
        let dialogues = download_from_value::<HashMap<u32, Vec<String>>>(self.get).await.expect("Download dialogues failed.");
        commit_srvpru_plugin(config, plugins, "dialogue", plugins::dialogue::Configuration {
            dialogues
        });
    }
}

#[derive(Deserialize)]
struct RandomDuelConfig {
    enabled: bool,
    default_type: String,
    no_rematch_check: bool,
    record_match_scores: bool,
    post_match_scores: bool,
    post_match_accesskey: String,
    blank_pass_modes: HashMap<String, bool>,
    ready_time: u64,
    hang_timeout: u64
}

#[async_trait]
impl SrvproModuleConfiguration for RandomDuelConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        if !self.enabled { return; } 
        commit_srvpru_plugin(config, plugins, "random_match", plugins::random_match::Configuration {
            default_mode: self.default_type,
            no_rematch: self.no_rematch_check
        });
        if self.ready_time > 0 || self.hang_timeout > 0 {
            commit_srvpru_plugin(config, plugins, "must_start", HashMap::from([
                ("start_game", vec![self.ready_time])
            ]));
        }
        if self.record_match_scores || self.post_match_scores {
            warn!("Srvpru don't support match score.")
        }
    }
}

#[derive(Deserialize)]
struct CloudReplayConfig {
    enabled: bool,
    enabled_halfway_watch: Option<bool>
}

#[async_trait]
impl SrvproModuleConfiguration for CloudReplayConfig {
    async fn fit_srvpru(self, config: &mut Configuration, _: &mut HashMap<String, Value>) {
        if self.enabled { warn!("Srvpru don't support cloud replay.") }
        if self.enabled_halfway_watch == Some(true) {
            config.plugins.push("telescreen".to_string());
        }
    }
}

#[derive(Deserialize)] 
struct WindbotConfig {
    enabled: bool,
    botlist: String,
    spawn: bool,
    port: u32,
    server_ip: String,
    my_ip: String
}

#[async_trait]
impl SrvproModuleConfiguration for WindbotConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        if ! self.enabled { return; }
        commit_srvpru_plugin(config, plugins, "windbot", plugins::windbot::Configuration {
            spawn: if self.spawn { Some(format!("cd windbot && mono ./WindBot.exe ServerMode=true ServerPort={}", self.port)) } else { None },
            server: format!("http://{}:{}", self.server_ip, self.port),
            bots: Vec::new(),
        });
        // my_ip discard. see windbot's document why it's not needed.
    }
}

#[derive(Deserialize)] 
struct ReconnectConfig {
    enabled: bool,
    auto_surrender_after_disconnect: bool,
    allow_kick_reconnect: bool,
    wait_time: u64
}

#[async_trait]
impl SrvproModuleConfiguration for ReconnectConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        if !self.enabled { return; }
        commit_srvpru_plugin(config, plugins, "reconnect", plugins::reconnect::Configuration {
            timeout: self.wait_time,
            can_reconnect_by_kick: self.allow_kick_reconnect 
        });
        if self.auto_surrender_after_disconnect {
            warn!("Srvpru reconnect don't support auto_surrender_after_disconnect.");
        }
    }
}

#[derive(Deserialize)] 
struct HeartbeatConfig {
    enabled: bool,
    interval: u64,
    wait_time: u64
}

#[async_trait]
impl SrvproModuleConfiguration for HeartbeatConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        if !self.enabled { return; }
        commit_srvpru_plugin(config, plugins, "heartbeat", plugins::heartbeat::Configuration {
            beat_per_minute: 60 / self.interval,
            diastole_time: self.wait_time,
        });
    }
}

#[derive(Deserialize)] 
struct MycardConfig {
    enabled: bool,
    auth_base_url: String,
    auth_database: String,
    ban_get: String,
    auth_key: String
}

#[async_trait]
impl SrvproModuleConfiguration for MycardConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        if !self.enabled { return; }
        commit_srvpru_plugin(config, plugins, "mc_login", plugins::mc_login::Configuration {
            auth_base_url: self.auth_base_url,
            auth_access_key: self.auth_key,
            permit_url: None,
            arena: None,
        });
        warn!("srvpru mc_login don't support ban_get, and won't access database.");
    }
}

#[derive(Deserialize)] 
struct DeckLogConfig {
    enabled: bool,
    accesskey: String,
    local: Value,
    post: Value,
    arena: String
}

#[async_trait]
impl SrvproModuleConfiguration for DeckLogConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        if !self.enabled { return; }
        if let Value::String(endpoint) = self.post {
            commit_srvpru_plugin(config, plugins, "deck_report", plugins::deck_report::Configuration {
                endpoint,
                access_key: self.accesskey,
                arena: self.arena,
            });
        }
        if matches!(self.local, Value::String(_)) {
            warn!("Srvpru deck_report don't support record to local.")
        }
    }
}

#[derive(Deserialize)]
struct BigBrotherConfig {
    enabled: bool,
    accesskey: String,
    post: String
}

#[derive(Deserialize)]
struct BadWordsConfig {
    level0: Vec<String>,
    level1: Vec<String>,
    level2: Vec<String>,
    level3: Vec<String>,
}

#[async_trait]
impl SrvproModuleConfiguration for BigBrotherConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        if !self.enabled { return; }
        let bad_words = match load_configuration::<BadWordsConfig>("badwords") {
            Ok(config) => config,
            Err(_) => {
                warn!("Can't find badwords. Big brother won't activate.");
                return;
            },
        };
        commit_srvpru_plugin(config, plugins, "newspeak", plugins::newspeak::Configuration { 
            bad_words: vec![bad_words.level0, bad_words.level1, bad_words.level2, bad_words.level3], 
            behavior: vec![
                plugins::newspeak::BadwordBehavior::None,
                plugins::newspeak::BadwordBehavior::Block,
                plugins::newspeak::BadwordBehavior::Replace,
                plugins::newspeak::BadwordBehavior::Silent
            ], 
            report_to_big_brother: self.post, 
            big_brother_access_key: self.accesskey 
        });
    }
}

#[derive(Deserialize)]
struct ArenaModeConfig {
    enabled: bool,
    mode: String,
    accesskey: String,
    ready_time: u64,
    #[serde(deserialize_with="deserialize_srvpro_value")]
    check_permit: Option<String>,
    #[serde(deserialize_with="deserialize_srvpro_value")]
    post_score: Option<String>,
    #[serde(deserialize_with="deserialize_srvpro_value")]
    get_score: Option<String>,
    punish_quit_before_match: bool,
    init_post: Option<ArenaModeInitPostConfig>
}

#[derive(Deserialize)]
struct ArenaModeInitPostConfig {
    enabled: bool,
    url: String,
    accesskey: String
}

#[async_trait]
impl SrvproModuleConfiguration for ArenaModeConfig {
    async fn fit_srvpru(self, config: &mut Configuration, plugins: &mut HashMap<String, Value>) {
        if !self.enabled { return; }
        if let Some(report_endpoint) = self.post_score{
            commit_srvpru_plugin(config, plugins, "result_report", plugins::result_report::Configuration { 
                report_endpoint, 
                access_key: self.accesskey.clone(),
                arena: self.mode.clone()
            });
        }
        commit_srvpru_plugin(config, plugins, "arena", plugins::arena::Configuration { 
            arena: self.mode.clone(), 
            init: self.init_post.map(|s| s.url),
            permit: self.check_permit, 
            get_score: self.get_score.map(|url| url.replace("?username=", "")), 
            access_key: self.accesskey.clone()
        });
        commit_srvpru_plugin(config, plugins, "must_start", HashMap::from([
            ("start_game", vec![self.ready_time])
        ]));
    }
    
}

#[derive(Deserialize)] 
struct TournamentModeConfig {
    enabled: bool,
    deck_check: bool,
    deck_path: String,
    replay_safe: bool,
    replay_path: String,
    replay_archive_tool: String,
    block_replay_to_player: bool,
    enable_recover: bool,
    show_ip: bool,
    show_info: bool,
    log_save_path: String,
    port: u16
}

#[derive(Deserialize)] 
struct AthleticCheckConfig {
    enabled: bool,
    rank_url: String,
    identifier_url: String,
    athletic_fetch_params: HashMap<String, String>,
    rank_count: u32,
    ttl: u32
}

struct SrvproValueVisitor;
impl<'de> serde::de::Visitor<'de> for SrvproValueVisitor {
    type Value = Option<String>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("null, false or an string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: serde::de::Error, {
        Ok(Some(v.to_string()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> where E: serde::de::Error, {
        Ok(Some(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> where E: serde::de::Error, {
        if v { Ok(Some(String::new())) } else { Ok(None) }
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> where E: serde::de::Error, {
        Ok(None)
    }
}

fn deserialize_srvpro_value<'de, D>(deserializer: D) -> Result<Option<String>, D::Error> where D: serde::de::Deserializer<'de> {
    deserializer.deserialize_any(SrvproValueVisitor)
}