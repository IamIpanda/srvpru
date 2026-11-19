use crate::portal::ConfigField;
use crate::portal::ConfigFieldKind;
use crate::portal::ConfigTab;
use crate::portal::ConfigTabContent;

#[distributed_slice(crate::portal::PORTAL_CONFIG_TABS)]
pub static CONFIG: ConfigTab = ConfigTab {
    plugin: srvpro::plugin::random_match::NAME,
    title: "随机匹配",
    order: 40,
    toggle: true,
    content: ConfigTabContent::Fields(&[
        ConfigField { key: "random_match_default", label: "默认模式", kind: ConfigFieldKind::Text, default: "S", options: &[] },
        ConfigField { key: "random_match_modes", label: "匹配模式列表", kind: ConfigFieldKind::Tags, default: "[]", options: &[] },
        ConfigField { key: "random_match_ready_timeout", label: "准备超时（秒）", kind: ConfigFieldKind::Text, default: "90", options: &[] },
        ConfigField { key: "random_match_start_timeout", label: "开局等待（秒）", kind: ConfigFieldKind::Text, default: "20", options: &[] },
        ConfigField { key: "random_match_min_surrender_turn", label: "允许投降的最早回合", kind: ConfigFieldKind::Text, default: "3", options: &[] },
        ConfigField { key: "random_match_surrender_limited_modes", label: "限制投降的模式", kind: ConfigFieldKind::Tags, default: "[\"S\",\"T\"]", options: &[] },
    ]),
};
