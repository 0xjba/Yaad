// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod embeddings;
mod whisper;
mod models;
mod visuals;
mod sui;
mod errors;

#[cfg(test)]
mod tests;

use std::sync::Mutex;
use std::{thread, time::Duration};
use tauri::{Manager, Emitter, WindowEvent, Size, LogicalSize};
use tauri::path::BaseDirectory;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::image::Image;
use tauri_plugin_positioner::{Position, WindowExt};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Mutex::new(commands::RecordingState {
            is_recording: false,
            start_time: None,
        }))
        .manage(commands::VisualState {
            embedder: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::save_memory,
            commands::search_memories,
            commands::get_memory,
            commands::delete_memory,
            commands::start_recording,
            commands::stop_recording,
            commands::cancel_recording,
            commands::download_models,
            commands::initialize_app,
            commands::capture_active_window_cmd,
            commands::get_contextual_suggestions,
            commands::check_permissions
        ])
        .setup(|app| {
            // ✅ Set to Prohibited early (applicationWillFinishLaunching phase)
            // Prevents dock icon flash and focus stealing during startup
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Prohibited);
            
            let app_data_dir = app.path().app_local_data_dir().unwrap();
            db::set_app_data_dir(app_data_dir).unwrap();
            db::init_db().unwrap();
            
            // --- WINDOW SETUP ---
            if let Some(window) = app.get_webview_window("panel") {
                // Allow shrinking to 1px
                let _ = window.set_min_size(Some(Size::Logical(LogicalSize { width: 300.0, height: 1.0 })));
                
                // ✅ CONVENTIONAL WAY: Disable shadow once at startup
                let _ = window.set_shadow(false);
            }

            std::thread::spawn(|| { let _ = embeddings::init_embedder(); });
            if let Err(e) = whisper::init_whisper(app.handle().clone()) { eprintln!("Whisper init error: {}", e); }
            
            // --- NEW: LIGHTWEIGHT SUI LOOP ---
            let app_handle = app.handle().clone();
            let search_icon_path = app_handle.path()
                .resolve("icons/search.png", BaseDirectory::Resource)
                .unwrap_or_else(|_| std::path::PathBuf::from("icons/search.png"));

            tauri::async_runtime::spawn(async move {
                let mut intent_state = sui::IntentState::default();
                
                loop {
                    // 1. Fetch Metadata (Fast, Low CPU)
                    {
                        let raw_json = unsafe { commands::fetch_metadata_public() };
                        let json_str = raw_json.as_str();

                        if let Ok(metadata) = serde_json::from_str::<sui::WindowMetadata>(json_str) {
                            
                            // 2. Decide
                            let decision = sui::process_metadata_trigger(&metadata, &mut intent_state);
                            
                            if decision == sui::TriggerDecision::Activate {
                                // 3. Trigger Action
                                // Optional: Check DB for embeddings here if you want to be extra sure
                                // For now, we trust the metadata + temporal confirmation
                                
                                let _ = app_handle.emit("show-glow", ());
                                
                                // Update Tray
                                if let Some(tray) = app_handle.tray_by_id("main") {
                                    if let Ok(icon) = Image::from_path(&search_icon_path) {
                                        let _ = tray.set_icon(Some(icon));
                                    }
                                }
                            }
                        }
                    } // raw_json dropped here
                    
                    // Sleep 1s
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });
            // ---------------------------------

            let default_icon = app.default_window_icon().unwrap().clone();
            
            let _tray = TrayIconBuilder::with_id("main")
                .icon(default_icon.clone())
                .on_tray_icon_event(move |tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                    
                    match event {
                        // LEFT CLICK: Capture
                        TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Down, .. } => {
                            let app = tray.app_handle();
                            // Reset icon on click
                            let _ = tray.set_icon(Some(app.default_window_icon().unwrap().clone()));
                            
                            if let Some(window) = app.get_webview_window("panel") {
                                // 1. Resize (Exact 44px)
                                let _ = window.set_size(Size::Logical(LogicalSize { width: 320.0, height: 44.0 }));
                                // 2. Position
                                let _ = window.move_window(Position::TrayCenter);
                                // 3. Show & Focus
                                let _ = window.show();
                                let _ = window.set_always_on_top(true);
                                let _ = window.set_focus();
                                
                                thread::sleep(Duration::from_millis(50));
                                let _ = window.emit("set-view", "capture");
                            }
                        }
                        // RIGHT CLICK: Recall
                        TrayIconEvent::Click { button: MouseButton::Right, button_state: MouseButtonState::Down, .. } => {
                            let app = tray.app_handle();
                            // Reset icon on click
                            let _ = tray.set_icon(Some(app.default_window_icon().unwrap().clone()));

                            if let Some(window) = app.get_webview_window("panel") {
                                let _ = window.set_size(Size::Logical(LogicalSize { width: 320.0, height: 280.0 }));
                                let _ = window.move_window(Position::TrayCenter);
                                let _ = window.show();
                                let _ = window.set_always_on_top(true);
                                let _ = window.set_focus();
                                
                                thread::sleep(Duration::from_millis(50));
                                let _ = window.emit("set-view", "recall");
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;
            
            // ✅ Switch to Accessory after launch completes (applicationDidFinishLaunching phase)
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            
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
