use bevy::prelude::*;
use bevy_egui::egui;

use crate::{resources::asset_store::AssetStore, scenes::assets::FontKey};

pub const UI_FONT_ID: &str = "ui_font";

pub fn apply_font_for_locale(
    ctx: &egui::Context,
    locale_code: &str,
    asset_store: &AssetStore,
    fonts: &Assets<Font>,
) {
    let mut defs = egui::FontDefinitions::default();
    let mut font_data = None;

    if locale_code == "zh-Hans" {
        if let Some(handle) = asset_store.font(FontKey::Chinese) {
            if let Some(font) = fonts.get(&handle) {
                font_data = Some(egui::FontData::from_owned(font.data.as_ref().clone()).into());
            } else {
                warn!("Chinese font handle found but data not loaded");
            }
        } else {
            warn!("Chinese font key not found in asset store");
        }
    }

    // Fallback to pixel_mplus if not Chinese or system font load failed
    if font_data.is_none() {
        if let Some(handle) = asset_store.font(FontKey::Default) {
            if let Some(font) = fonts.get(&handle) {
                font_data = Some(egui::FontData::from_owned(font.data.as_ref().clone()).into());
            } else {
                warn!("Default font handle found but data not loaded");
            }
        } else {
            warn!("Default font key not found in asset store");
        }
    }

    if let Some(data) = font_data {
        defs.font_data.insert(UI_FONT_ID.to_owned(), data);

        defs.families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, UI_FONT_ID.to_owned());
        defs.families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .insert(0, UI_FONT_ID.to_owned());

        ctx.set_fonts(defs);
    }
}
