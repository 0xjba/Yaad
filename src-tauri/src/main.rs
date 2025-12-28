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
            commands::get_contextual_suggestions
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
            
            // --- PROACTIVE SUI LOOP ---
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut intent_state = sui::IntentState::default();
                
                // 1. Resolve Icon Path SAFELY (Do not unwrap inside loop)
                // This looks for "icons/search.png" in the "resources" folder we defined in tauri.conf.json
                let search_icon_path = app_handle.path()
                    .resolve("icons/search.png", BaseDirectory::Resource)
                    .unwrap_or_else(|_| std::path::PathBuf::from("icons/search.png"));

                loop {
                    std::thread::sleep(Duration::from_secs(2));
                    
                    // Capture active window
                    let result = unsafe { commands::capture_active_window() };
                    let result_str = result.to_string();
                    if result_str.starts_with("ERROR:") { continue; }
                    
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result_str) {
                        let app_name = parsed["app_name"].as_str().unwrap_or("");
                        let ocr_text = parsed["ocr"].as_str().unwrap_or("");
                        let title = parsed["title"].as_str().unwrap_or("");
                        let url = parsed["url"].as_str().unwrap_or("");
                        
                        if sui::check_utility_trigger(title, app_name, url, ocr_text, &mut intent_state) {
                            // Check for relevant memories
                            if let Ok(conn) = db::get_connection() {
                                let text_query = format!("{} {} {} {}", title, app_name, url, ocr_text);
                                if let Ok(text_embedding) = embeddings::generate_embedding(&text_query) {
                                    if let Ok(matches) = embeddings::search_similar(&conn, &text_embedding, 1) {
                                        if !matches.is_empty() && matches[0].1 > 0.85 {
                                            // TRIGGER GLOW
                                            let _ = app_handle.emit("show-glow", ());
                                            
                                            // Update tray icon SAFELY
                                            if let Some(tray) = app_handle.tray_by_id("main") {
                                                if let Ok(icon) = Image::from_path(&search_icon_path) {
                                                    let _ = tray.set_icon(Some(icon));
                                                } else {
                                                    eprintln!("Warning: Failed to load icon from {:?}", search_icon_path);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });

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
