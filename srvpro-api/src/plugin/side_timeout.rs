use crate::portal::ConfigField;
use crate::portal::ConfigFieldKind;
use crate::portal::ConfigTab;
use crate::portal::ConfigTabContent;

#[distributed_slice(crate::portal::PORTAL_CONFIG_TABS)]
pub static CONFIG: ConfigTab = ConfigTab {
    plugin: srvpro::plugin::side_timeout::NAME,
    title: "换备超时",
    order: 41,
    toggle: true,
    content: ConfigTabContent::Fields(&[ConfigField { key: "side_timeout_side_timeout", label: "超时（分钟）", kind: ConfigFieldKind::Text, default: "3", options: &[] }]),
};
