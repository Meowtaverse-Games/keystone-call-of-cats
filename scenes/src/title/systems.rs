use bevy::prelude::*;
use super::components::TitleUI;

/// タイトル画面セットアップ：タイトル画像を中央に配置
pub fn setup_title(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
//    commands.spawn(Sprite::from_image(asset_server.load("images/logo.png")));
    let logo_handle: Handle<Image> = asset_server.load("images/logo_with_black.png");

    let fixed_width = 128.0;
    let aspect = {
        // 仮に 512×256 の画像なら 256.0 / 512.0 = 0.5
        let w = 250.0;
        let h = 250.0;
        h / w
    };
    let custom_size = Vec2::new(fixed_width, fixed_width * aspect);

    // ④ Sprite を直接スポーン。Transform は自動で (0,0,0)、
    //    GlobalTransform や Visibility も必要に応じて挿入されます
    commands.spawn(Sprite {
        image: logo_handle.clone(),
        custom_size: Some(custom_size),
        ..Default::default()
    });

    // commands.spawn((
    //     Node {
    //         width: Val::Percent(100.0),
    //         height: Val::Percent(100.0),
    //         justify_content: JustifyContent::Center,
    //         align_items: AlignItems::Center,
    //         ..default()
    //     },
    //     TitleUI,
    // ))
    // .with_children(|parent| {
    //     parent.spawn((
    //         Sprite::from_image(asset_server.load("images/logo.png")),

    //         // Image::from(writeln!())
    //         // {
    //         //     width: Val::Px(400.0),
    //         //     Val::Px(200.0)),
    //         //         ..default()
    //         //     },
    //         //     image: UiImage(asset_server.load("title/title.png")),
    //         //     ..default()o
    //         // },
    //         TitleUI, // 画像スプライトにも同じタグを付与
    //     ));
    // });
}

/// タイトル画面退出時に TitleUI タグ付きエンティティを全削除
pub fn cleanup_title(
    mut commands: Commands,
    query: Query<Entity, With<TitleUI>>,
) {
    for ent in query.iter() {
        // 子孫も含めて再帰的に削除
        commands.entity(ent).despawn_recursive();
    }
}
