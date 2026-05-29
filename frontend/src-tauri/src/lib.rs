#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            for window_config in app.config().app.windows.clone() {
                let mut window_config = window_config;
                if env_flag_enabled("HIDE_TITLE") {
                    window_config.decorations = false;
                }
                tauri::WebviewWindowBuilder::from_config(app.handle(), &window_config)?.build()?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| flag_value_enabled(&value.to_string_lossy()))
}

fn flag_value_enabled(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

#[cfg(test)]
mod tests {
    use super::flag_value_enabled;

    #[test]
    fn false_like_flag_values_disable_the_flag() {
        for value in ["0", "false", "False", " no ", "OFF"] {
            assert!(!flag_value_enabled(value));
        }
    }

    #[test]
    fn any_other_present_flag_value_enables_the_flag() {
        for value in ["", "1", "true", "yes", "hyprland"] {
            assert!(flag_value_enabled(value));
        }
    }
}
