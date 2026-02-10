use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, LazyLock};

use i18n_embed::{
    LanguageLoader,
    Localizer,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use rust_embed::RustEmbed;
use unic_langid::LanguageIdentifier;

#[derive(RustEmbed)]
#[folder = "./i18n/"]
pub struct ClapI18nLocalizations;

pub static CLAP_I18N_LANGUAGE_LOADER: LazyLock<Arc<FluentLanguageLoader>> = LazyLock::new(|| {
    Arc::new(create_language_loader())
});

pub static CLAP_I18N_LANGUAGE_LOADER_MAP: LazyLock<HashMap<LanguageIdentifier, Arc<FluentLanguageLoader>>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for lang in available_languages() {
        let loader = create_language_loader();
        if let Err(error) = loader.load_languages(&ClapI18nLocalizations, std::slice::from_ref(&lang)) {
            eprintln!("Error while preloading languages for library_fluent {error}");
        }
        let localizer = i18n_embed::DefaultLocalizer::new(&loader, &ClapI18nLocalizations);
        if let Err(error) = localizer.select(std::slice::from_ref(&lang)) {
            eprintln!("Error while selecting language for library_fluent {error}");
        }
        loader.set_use_isolating(false);
        map.insert(lang, Arc::new(loader));
    }
    map
});

thread_local! {
    static CLAP_I18N_THREAD_LOCAL_LOADER: RefCell<Option<Arc<FluentLanguageLoader>>> = const { RefCell::new(None) };
    static CLAP_I18N_PRERENDERED_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub(crate) fn create_language_loader() -> FluentLanguageLoader {
    let loader: FluentLanguageLoader = fluent_language_loader!();

    loader
        .load_fallback_language(&ClapI18nLocalizations)
        .expect("Error while loading fallback language");

    loader
}

pub(crate) fn available_languages() -> Vec<LanguageIdentifier> {
    let mut langs = BTreeSet::new();

    for path in ClapI18nLocalizations::iter() {
        let path = path.as_ref();
        if let Some((lang, _)) = path.split_once('/') {
            langs.insert(lang.to_string());
        }
    }

    langs
        .into_iter()
        .filter_map(|lang| lang.parse::<LanguageIdentifier>().ok())
        .collect()
}

pub(crate) fn with_scoped_loader<R>(loader: Arc<FluentLanguageLoader>, f: impl FnOnce() -> R) -> R {
    CLAP_I18N_THREAD_LOCAL_LOADER.with(|cell| {
        let previous = cell.replace(Some(loader));
        let result = f();
        cell.replace(previous);
        result
    })
}

pub(crate) fn get_loader_for_languages(languages: &[LanguageIdentifier]) -> Arc<FluentLanguageLoader> {
    for lang in languages {
        if let Some(loader) = CLAP_I18N_LANGUAGE_LOADER_MAP.get(lang) {
            return Arc::clone(loader);
        }
    }

    Arc::clone(&CLAP_I18N_LANGUAGE_LOADER)
}

pub(crate) fn with_language_loader<R>(f: impl FnOnce(&FluentLanguageLoader) -> R) -> R {
    CLAP_I18N_THREAD_LOCAL_LOADER.with(|cell| {
        if let Some(loader) = cell.borrow().as_ref() {
            return f(loader);
        }

        f(&CLAP_I18N_LANGUAGE_LOADER)
    })
}

#[doc(hidden)]
pub fn set_prerendered_error(message: String) {
    CLAP_I18N_PRERENDERED_ERROR.with(|cell| {
        *cell.borrow_mut() = Some(message);
    });
}

pub(crate) fn take_prerendered_error() -> Option<String> {
    CLAP_I18N_PRERENDERED_ERROR.with(|cell| {
        cell.borrow_mut().take()
    })
}

/// Get a translated string by message ID (runtime lookup, no compile-time validation)
#[doc(hidden)]
pub fn get_translation(message_id: &str) -> String {
    with_language_loader(|loader| loader.get(message_id))
}

macro_rules! fl {
    ($message_id:literal) => {{
        crate::lang::with_language_loader(|loader| {
            i18n_embed_fl::fl!(loader, $message_id)
        })
    }};

    ($message_id:literal, $($args:tt)+) => {{
        crate::lang::with_language_loader(|loader| {
            i18n_embed_fl::fl!(loader, $message_id, $($args)+)
        })
    }};
}

pub(crate) use fl;
