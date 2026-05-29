// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    configure_linux_webview_acceleration();

    app_lib::run();
}

#[cfg(target_os = "linux")]
fn configure_linux_webview_acceleration() {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && is_nvidia_session() {
        if std::env::var_os("__NV_DISABLE_EXPLICIT_SYNC").is_none() {
            // Required on this WebKitGTK/NVIDIA path to avoid Wayland protocol failures at startup.
            std::env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1");
        }

        if std::env::var_os("WEBKIT_SKIA_GPU_PAINTING_THREADS").is_none() {
            // NVIDIA 595 can crash WebKit's threaded Skia GPU worker during heavy scrolling.
            // A zero worker count keeps GPU rendering enabled, but performs Skia GPU painting
            // on WebKit's main rendering thread instead of the SkiaGPUWorker pool.
            std::env::set_var("WEBKIT_SKIA_GPU_PAINTING_THREADS", "0");
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_linux_webview_acceleration() {}

#[cfg(target_os = "linux")]
fn is_nvidia_session() -> bool {
    std::env::var_os("GBM_BACKEND").is_some_and(|backend| backend == "nvidia-drm")
        || std::env::var_os("__GLX_VENDOR_LIBRARY_NAME").is_some_and(|vendor| vendor == "nvidia")
        || std::path::Path::new("/proc/driver/nvidia/version").exists()
}
