use bevy::{prelude::*, time::common_conditions::on_timer};
use std::time::Duration;

use crate::{
    resources::asset_store::{AssetGroupLoaded, AssetStore, LoadAssetGroup, PendingGroups},
    systems::engine::assets_loader::{
        handle_load_requests, pending_not_empty, poll_pending_groups,
    },
};

pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetStore>()
            .init_resource::<PendingGroups>()
            .add_message::<LoadAssetGroup>()
            .add_message::<AssetGroupLoaded>()
            .add_systems(Update, handle_load_requests)
            .add_systems(
                Update,
                poll_pending_groups
                    .run_if(pending_not_empty)
                    .run_if(on_timer(Duration::from_millis(50))),
            );
    }
}
