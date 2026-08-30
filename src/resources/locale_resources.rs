#[cfg(not(target_arch = "wasm32"))]
use bevy::asset::LoadedFolder;
use bevy::prelude::*;
#[cfg(any(target_arch = "wasm32", test))]
use bevy_fluent::Locale;
#[cfg(any(target_arch = "wasm32", test))]
use unic_langid::LanguageIdentifier;

#[cfg(target_arch = "wasm32")]
use bevy_fluent::BundleAsset;

#[derive(Resource)]
pub enum LocaleAssets {
    #[cfg(not(target_arch = "wasm32"))]
    Native(Handle<LoadedFolder>),
    #[cfg(target_arch = "wasm32")]
    Web([Handle<BundleAsset>; 3]),
}

impl LocaleAssets {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn is_loaded(&self, asset_server: &AssetServer) -> bool {
        match self {
            Self::Native(folder) => matches!(
                asset_server.get_load_state(folder),
                Some(bevy::asset::LoadState::Loaded)
            ),
        }
    }

    pub fn has_failed(&self, asset_server: &AssetServer) -> bool {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Native(folder) => matches!(
                asset_server.get_load_state(folder),
                Some(bevy::asset::LoadState::Failed(_))
            ),
            #[cfg(target_arch = "wasm32")]
            Self::Web(bundles) => bundles.iter().any(|bundle| {
                matches!(
                    asset_server.get_load_state(bundle),
                    Some(bevy::asset::LoadState::Failed(_))
                )
            }),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn native_folder(&self) -> Option<&Handle<LoadedFolder>> {
        match self {
            Self::Native(folder) => Some(folder),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn web_bundles(&self) -> Option<&[Handle<BundleAsset>; 3]> {
        match self {
            Self::Web(bundles) => Some(bundles),
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
pub fn locale_order(locale: &Locale, available: &[LanguageIdentifier]) -> Vec<usize> {
    locale
        .fallback_chain(available.iter())
        .into_iter()
        .filter_map(|requested| {
            available
                .iter()
                .position(|available| available == requested)
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
pub fn build_web_localization(
    locale: &Locale,
    handles: &[Handle<BundleAsset>; 3],
    assets: &Assets<BundleAsset>,
) -> Option<bevy_fluent::Localization> {
    use bevy_fluent::exts::fluent::BundleExt;

    let bundles = handles
        .iter()
        .map(|handle| assets.get(handle).map(|asset| (handle, asset)))
        .collect::<Option<Vec<_>>>()?;
    let available = bundles
        .iter()
        .map(|(_, bundle)| bundle.locale().clone())
        .collect::<Vec<_>>();
    let mut localization = bevy_fluent::Localization::new();
    for index in locale_order(locale, &available) {
        let (handle, bundle) = bundles[index];
        localization.insert(handle, bundle);
    }
    Some(localization)
}

#[cfg(test)]
mod tests {
    use super::*;
    use unic_langid::langid;

    #[test]
    fn locale_order_keeps_requested_locale_before_en_us_fallback() {
        let locale = Locale::new(langid!("ja-JP")).with_default(langid!("en-US"));
        let available = vec![langid!("en-US"), langid!("ja-JP"), langid!("zh-Hans")];

        assert_eq!(locale_order(&locale, &available), vec![1, 0]);
    }
}
