use crate::portal::ConfigField;
use crate::portal::ConfigFieldKind;
use crate::portal::ConfigTab;
use crate::portal::ConfigTabContent;

#[distributed_slice(crate::portal::PORTAL_CONFIG_TABS)]
pub static CONFIG: ConfigTab = ConfigTab {
    plugin: srvpro::plugin::reconnect::NAME,
    title: "断线重连",
    order: 42,
    toggle: true,
    content: ConfigTabContent::Fields(&[
        ConfigField { key: "reconnect_wait_time", label: "等待时间（秒）", kind: ConfigFieldKind::Text, default: "60", options: &[] },
        ConfigField { key: "reconnect_auto_surrender_after_disconnect", label: "掉线后自动投降", kind: ConfigFieldKind::Boolean, default: "false", options: &[] },
        ConfigField { key: "reconnect_allow_kick_reconnect", label: "允许顶号重连", kind: ConfigFieldKind::Boolean, default: "false", options: &[] },
    ]),
};
