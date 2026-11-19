use crate::portal::ConfigField;
use crate::portal::ConfigFieldKind;
use crate::portal::ConfigTab;
use crate::portal::ConfigTabContent;

#[distributed_slice(crate::portal::PORTAL_CONFIG_TABS)]
static CONFIG: ConfigTab = ConfigTab {
    plugin: srvpro::plugin::deck_report::NAME,
    title: "卡组上报",
    order: 51,
    toggle: true,
    content: ConfigTabContent::Fields(&[
        ConfigField { key: "deck_report_url", label: "地址", kind: ConfigFieldKind::Text, default: "", options: &[] },
        ConfigField { key: "deck_report_access_key", label: "访问密钥", kind: ConfigFieldKind::Text, default: "", options: &[] },
        ConfigField { key: "deck_report_arena", label: "赛区名", kind: ConfigFieldKind::Text, default: "", options: &[] },
    ]),
};
