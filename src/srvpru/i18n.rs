
use std::collections::HashMap;
use once_cell::sync::OnceCell;

use crate::srvpru::plugins;

pub static INTERNATIONAL_LIBRARY: OnceCell<HashMap<String, HashMap<String, String>>> = OnceCell::new();

pub fn load_configuration() -> anyhow::Result<()> {
    info!("Loading i18n directory...");
    let international_library_from_file = plugins::load_configuration::<HashMap<String, HashMap<String, String>>>("i18n")?;
    let mut international_library = HashMap::new();
    for (locale, locale_library_from_file) in international_library_from_file.into_iter() {
        let mut locale_library = HashMap::new();
        for (word, context) in locale_library_from_file.into_iter() {
            locale_library.insert(format!("{{{}}}", word), context);
        } 
        international_library.insert(locale, locale_library);
    }
    INTERNATIONAL_LIBRARY.set(international_library).expect("i18n library already set.");
    Ok(())
}

pub fn render(template: &str, locale: &str) -> String {
    let international_library = INTERNATIONAL_LIBRARY.get().expect("i18n library haven't set yet.");
    let mut answer_string = template.to_string();
    if let Some(words) = international_library.get(locale) {
        for (word, context) in words.into_iter() {
            answer_string = answer_string.replace(word, context);
        }
    }
    return answer_string;
}