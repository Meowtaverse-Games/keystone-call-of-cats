// Hide console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Desktop entrypoint. Android calls `keystone_cc::main` from GameActivity instead.
fn main() {
    keystone_cc::main();
}
