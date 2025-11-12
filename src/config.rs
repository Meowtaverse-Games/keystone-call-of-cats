const DEFAULT_STEAM_APP_ID: u32 = 4169380;

pub fn steam_app_id() -> u32 {
    // if let Ok(s) = env::var("STEAM_APP_ID") {
    //     if let Ok(v) = s.parse::<u32>() {
    //         return v;
    //     }
    // }
    DEFAULT_STEAM_APP_ID
}
