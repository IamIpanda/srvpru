use crate::portal::ConfigField;
use crate::portal::ConfigFieldKind;
use crate::portal::ConfigTab;
use crate::portal::ConfigTabContent;

#[distributed_slice(crate::portal::PORTAL_CONFIG_TABS)]
pub static CONFIG: ConfigTab = ConfigTab {
    plugin: srvpro::plugin::report::NAME,
    title: "对局上报",
    order: 50,
    toggle: true,
    content: ConfigTabContent::Fields(&[
        ConfigField { key: "report_url", label: "地址", kind: ConfigFieldKind::Text, default: "", options: &[] },
        ConfigField { key: "report_access_key", label: "访问密钥", kind: ConfigFieldKind::Text, default: "", options: &[] },
        ConfigField { key: "report_arena", label: "赛区名", kind: ConfigFieldKind::Text, default: "", options: &[] },
        ConfigField { key: "report_format", label: "录像格式", kind: ConfigFieldKind::Select, default: "forge", options: &[("ygopro", "ygopro"), ("forge", "forge"), ("srvpro", "srvpro")] },
    ]),
};
