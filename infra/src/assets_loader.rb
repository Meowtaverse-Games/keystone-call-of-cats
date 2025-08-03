

fn check_and_fire_event(
    mut events: EventWriter<AssetsLoadedEvent>,
    loading: Res<LoadingGroups>,
    asset_server: Res<AssetServer>,
) {
    use bevy::asset::LoadState;

    // Splash
    if loading.splash.is_loaded(&asset_server) {
        events.send(AssetsLoadedEvent(AssetGroup::Splash));
    }
    // Game
    if loading.game.is_loaded(&asset_server) {
        events.send(AssetsLoadedEvent(AssetGroup::Game));
    }
}
