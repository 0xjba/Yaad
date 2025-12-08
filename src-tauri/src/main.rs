// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod embeddings;
mod whisper;
mod models;
mod errors;

#[cfg(test)]
mod tests;

use std::sync::Mutex;
use std::{thread, time::Duration};
use tauri::{Manager, Emitter, WindowEvent, Size, LogicalSize};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::image::Image;
use tauri_plugin_positioner::{Position, WindowExt};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(Mutex::new(commands::RecordingState {
            is_recording: false,
            start_time: None,
        }))
        .invoke_handler(tauri::generate_handler![
            commands::save_memory,
            commands::search_memories,
            commands::get_memory,
            commands::delete_memory,
            commands::start_recording,
            commands::stop_recording,
            commands::cancel_recording,
            commands::download_models,
            commands::initialize_app
        ])
        .setup(|app| {
            let app_data_dir = app.path().app_local_data_dir().unwrap();
            db::set_app_data_dir(app_data_dir).unwrap();
            db::init_db().unwrap();
            
            // --- WINDOW SETUP ---
            if let Some(window) = app.get_webview_window("panel") {
                // Allow shrinking to 1px
                let _ = window.set_min_size(Some(Size::Logical(LogicalSize { width: 300.0, height: 1.0 })));
            }

            std::thread::spawn(|| { let _ = embeddings::init_embedder(); });
            if let Err(e) = whisper::init_whisper(app.handle().clone()) { eprintln!("Whisper init error: {}", e); }
            
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            
            let default_icon = app.default_window_icon().unwrap().clone();
            
            let _tray = TrayIconBuilder::new()
                .icon(default_icon)
                .on_tray_icon_event(move |tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                    
                    match event {
                        // LEFT CLICK: Capture
                        TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Down, .. } => {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("panel") {
                                // 1. Resize (Exact 44px)
                                let _ = window.set_size(Size::Logical(LogicalSize { width: 320.0, height: 44.0 }));
                                // 2. Position
                                let _ = window.move_window(Position::TrayCenter);
                                // 3. Show & Focus
                                let _ = window.show();
                                let _ = window.set_always_on_top(true);
                                let _ = window.set_focus();
                                // 4. KILL SHADOW LAST (Fixes artifact)
                                let _ = window.set_shadow(false);
                                
                                thread::sleep(Duration::from_millis(50));
                                let _ = window.emit("set-view", "capture");
                            }
                        }
                        // RIGHT CLICK: Recall
                        TrayIconEvent::Click { button: MouseButton::Right, button_state: MouseButtonState::Down, .. } => {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("panel") {
                                let _ = window.set_size(Size::Logical(LogicalSize { width: 320.0, height: 280.0 }));
                                let _ = window.move_window(Position::TrayCenter);
                                let _ = window.show();
                                let _ = window.set_always_on_top(true);
                                let _ = window.set_focus();
                                let _ = window.set_shadow(false); // Kill shadow last
                                
                                thread::sleep(Duration::from_millis(50));
                                let _ = window.emit("set-view", "recall");
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;
            
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(focused) = event {
                if !focused { 
                    // Stop recording (discard) when focus is lost
                    let _ = window.emit("window-blur", ());
                    let _ = window.hide(); 
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
