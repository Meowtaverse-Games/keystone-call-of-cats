#![windows_subsystem = "windows"]

use std::env;
use std::path::PathBuf;
use std::process::Command;
use windows::Win32::System::SystemInformation::{
    GetNativeSystemInfo, PROCESSOR_ARCHITECTURE_ARM64, SYSTEM_INFO,
};

fn is_native_arm64() -> bool {
    let mut system_info = SYSTEM_INFO::default();
    unsafe {
        GetNativeSystemInfo(&mut system_info);
    }
    // PROCESSOR_ARCHITECTURE_ARM64 is 12 (0xC)
    unsafe {
        system_info.Anonymous.Anonymous.wProcessorArchitecture.0 == PROCESSOR_ARCHITECTURE_ARM64.0
    }
}

fn main() {
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let is_arm = is_native_arm64();

    let exe_path = if is_arm {
        "bin\\arm64\\keystone-cc.exe"
    } else {
        "bin\\x64\\keystone-cc.exe"
    };

    // Check if the target executable exists, fallback to x64 if arm64 missing (e.g. on intel machine running logic)
    // or just trust the existence.
    // Better to let it fail or try the other if missing?
    // For now, strict logic: if ARM machine, use ARM binary. If Intel machine, use Intel binary.

    let target_path = current_dir.join(exe_path);

    // Spawn the game process
    if let Ok(_child) = Command::new(&target_path).current_dir(&current_dir).spawn() {
        // Launcher exits immediately, letting the game run
    } else {
        // If spawn fails, maybe show a message box?
        // For now, simpler is better. Bevy app usually handles logging.
        // If this is a silent crash, it might be hard to debug, but this is a simple dispatcher.
        // Try to fallback to x64 if arm64 failed? No, mixed arch is dangerous.
    }
}
