// Copyright (c) 2025 tommyZZM
// tommys-comfy-screen-capturer is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.
//

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod windows_utils;

use base64::encode;
use image::ImageFormat;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Listener;
use tauri::{generate_handler, AppHandle, Emitter, EventLoopMessage, Manager, State, Window, Wry};
use tauri::{
    menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem},
    tray::TrayIconBuilder,
};
use tokio::sync::oneshot;
use warp::http::Response;
use warp::hyper::Body;
use warp::{reject::Reject, Filter};
use windows_utils::capture::capture_screen;
// use tauri_plugin_clipboard_manager;

#[derive(Debug)]
struct CustomError {
    message: String,
}

impl Reject for CustomError {}

impl CustomError {
    fn ImageWriteError(message: String) -> Self {
        CustomError { message }
    }
}

struct RecorderState {
    http_server: Mutex<Option<tokio::task::JoinHandle<()>>>,
    stop_signal: Mutex<Option<oneshot::Sender<()>>>,
    tokio_runtime: tokio::runtime::Runtime,
    last_port: Mutex<Option<u16>>,
    is_pin: AtomicBool,
    tray_menu: Mutex<Option<Menu<tauri::Wry>>>, // 添加这个字段
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(message: &str) -> String {
    println!("greet message: {}", message);
    format!("Hello, {}! You've been greeted from Rust!", message)
}

#[tauri::command]
fn resize_window(window: Window, width: f64, height: f64) -> Result<(), String> {
    // println!(
    //     "resize_window invoked with width: {}, height: {}",
    //     width, height
    // );
    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_window_title(window: Window, title: String) {
    window.set_title(&title).unwrap();
}

#[tauri::command]
fn capture_window_screenshot(window: Window, scale_factor: f32) -> Result<String, String> {
    println!("capture_window_screenshot");
    let tauri_window_hwnd = window.hwnd().unwrap_or_default();
    // let tauri_window_pid = get_window_pid(tauri_window_hwnd);

    match capture_screen(tauri_window_hwnd, scale_factor) {
        Ok(image) => {
            let mut cursor = Cursor::new(Vec::new());
            image
                .write_to(&mut cursor, ImageFormat::Png)
                .map_err(|e| e.to_string())?;
            let encoded_image = encode(cursor.into_inner());
            Ok(encoded_image)
        }
        Err(_) => Err("Screenshot capture failed".into()),
    }
}

#[tauri::command]
fn restart_http_server(
    state: State<'_, RecorderState>,
    window: Window,
    port: u16,
    scale_factor: f32,
) {
    // 检查传入的端口号是否与上一次相同
    {
        let last_port = state.last_port.lock().unwrap();
        if let Some(last_port) = *last_port {
            if last_port == port {
                println!(
                    "Port {} is already in use, no need to restart the server.",
                    port
                );
                return;
            }
        }
    }

    // Stop the existing server if running
    stop_http_server(state.clone());

    println!("restart_http_server... {}", port);

    {
        let mut last_port = state.last_port.lock().unwrap();
        *last_port = Some(port);
    }

    let (tx, rx) = oneshot::channel();

    let tray_menu = state.tray_menu.lock().unwrap();
    if let Some(tray_menu) = &*tray_menu {
        if let Some(menu_item_kind) = tray_menu.get("copy_screenshot_url") {
            if let MenuItemKind::MenuItem(menu_item) = menu_item_kind {
                menu_item.set_enabled(true);
            }
        }
    }

    let handle = state.tokio_runtime.spawn(async move {
        let is_capturing = Arc::new(Mutex::new(false));
        let capture_route = warp::path("capture_screen")
            .and(with_capturing(is_capturing.clone()))
            .and_then(move |_| {
                let window = window.clone();
                async move {
                    let tauri_window_hwnd = window.hwnd().unwrap_or_default();
                    let result = match capture_screen(tauri_window_hwnd, scale_factor) {
                        Ok(image) => {
                            let mut cursor = Cursor::new(Vec::new());
                            image.write_to(&mut cursor, ImageFormat::Png).map_err(|e| {
                                warp::reject::custom(CustomError::ImageWriteError(e.to_string()))
                            })?;
                            let binary_image = cursor.into_inner();
                            Ok::<_, warp::Rejection>(
                                Response::builder()
                                    .header("Content-Type", "image/png")
                                    .body(Body::from(binary_image))
                                    .unwrap(),
                            )
                        }
                        Err(_) => Err(warp::reject::not_found()),
                    };

                    result
                }
            });

        let default_route = warp::any().map(|| {
            warp::reply::with_status("Not Found".to_string(), warp::http::StatusCode::NOT_FOUND)
        });

        let routes = capture_route
            .or(default_route)
            .with(warp::log("capture_screen"));

        let (_addr, server) =
            warp::serve(routes).bind_with_graceful_shutdown(([127, 0, 0, 1], port), async {
                rx.await.ok();
            });

        server.await;

        println!("server stopped...");
    });

    *state.http_server.lock().unwrap() = Some(handle);
    *state.stop_signal.lock().unwrap() = Some(tx);
}

fn with_capturing(
    is_capturing: Arc<Mutex<bool>>,
) -> impl Filter<Extract = (Arc<Mutex<bool>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || is_capturing.clone())
}

#[tauri::command]
fn stop_http_server(state: State<'_, RecorderState>) {
    let mut stop_signal = state.stop_signal.lock().unwrap();
    let mut http_server = state.http_server.lock().unwrap();

    println!("stop_http_server...");

    if stop_signal.is_none() && http_server.is_none() {
        // Server is not running, silently return
        return;
    }

    if let Some(sender) = stop_signal.take() {
        let _ = sender.send(());
    }

    if let Some(handle) = http_server.take() {
        handle.abort();
        // 等待任务结束
        if handle.is_finished() {
            // println!("Waiting for the server to stop...");
            *state.http_server.lock().unwrap() = None;
            *state.stop_signal.lock().unwrap() = None;
        }

        let tray_menu = state.tray_menu.lock().unwrap();
        if let Some(tray_menu) = &*tray_menu {
            if let Some(menu_item_kind) = tray_menu.get("copy_screenshot_url") {
                if let MenuItemKind::MenuItem(menu_item) = menu_item_kind {
                    menu_item.set_enabled(false);
                }
            }
        }
    }
}

#[tauri::command]
fn get_is_server_running(state: State<'_, RecorderState>) -> bool {
    let http_server = state.http_server.lock().unwrap();
    let is_running = http_server.is_some();

    // let tray_menu = state.tray_menu.lock().unwrap();
    // if let Some(tray_menu) = &*tray_menu {
    //     if let Some(menu_item_kind) = tray_menu.get("copy_screenshot_url") {
    //         if let MenuItemKind::MenuItem(menu_item) = menu_item_kind {
    //             menu_item.set_enabled(is_running);
    //         }
    //     }
    // }

    is_running
}

#[tauri::command]
fn get_is_pin(state: State<'_, RecorderState>) -> bool {
    state.is_pin.load(Ordering::SeqCst)
}

#[tauri::command]
fn set_is_pin(state: State<'_, RecorderState>, app_handle: AppHandle, is_pin: bool) {
    let tray_menu = state.tray_menu.lock().unwrap();
    if let Some(tray_menu) = &*tray_menu {
        handle_is_pin_changed_enable_menu(tray_menu, is_pin);
    }
    state.is_pin.store(is_pin, Ordering::SeqCst);
    app_handle.emit("is_pin_changed", is_pin).unwrap();
}

fn handle_is_pin_changed_enable_menu(tray_menu: &Menu<Wry>, is_pin: bool) {
    if let Some(menu_item_kind) = tray_menu.get("pin") {
        if let MenuItemKind::MenuItem(menu_item) = menu_item_kind {
            menu_item.set_enabled(!is_pin);
        }
    }

    if let Some(menu_item_kind) = tray_menu.get("unpin") {
        if let MenuItemKind::MenuItem(menu_item) = menu_item_kind {
            menu_item.set_enabled(is_pin);
        }
    }
}

#[tauri::command]
fn start_dragging(window: Window) {
    window.start_dragging().unwrap();
}

#[tauri::command]
fn quit_app(app_handle: AppHandle) {
    app_handle.exit(0);
}

pub fn run() {
    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard::init()) 
        .setup(|app| {
            let pin_i = MenuItem::with_id(app, "pin", "Pin", true, None::<&str>)?;
            let unpin_i = MenuItem::with_id(app, "unpin", "UnPin", false, None::<&str>)?;
            let copy_url_i =
                MenuItem::with_id(app, "copy_screenshot_url", "Copy ScreenShoot Url", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &copy_url_i,
                    &PredefinedMenuItem::separator(app)?,
                    &pin_i,
                    &unpin_i,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, Some("Quit"))?,
                ],
            )?;

            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app_handle, event| {
                    let state = app_handle.state::<RecorderState>();
                    let tray_menu = state.tray_menu.lock().unwrap();
                    if let Some(tray_menu) = &*tray_menu {
                        match event.id.as_ref() {
                            "pin" => {
                                app_handle.emit("is_pin_changed", true).unwrap();
                                handle_is_pin_changed_enable_menu(tray_menu, true);
                            }
                            "unpin" => {
                                app_handle.emit("is_pin_changed", false).unwrap();
                                handle_is_pin_changed_enable_menu(tray_menu, false);
                            }
                            "copy_screenshot_url" => {
                                app_handle.emit("copy_screenshot_url", ()).unwrap();
                            }
                            _ => {
                                println!("menu item {:?} not handled", event.id);
                            }
                        }
                    }
                })
                .build(app)?;

            // 监听 is_pin_changed 事件
            let app_handle = app.handle().clone();
            app.listen("is_pin_changed", move |event| {
                if let Some(is_pin) = event.payload().parse::<bool>().ok() {
                    let window = app_handle.get_webview_window("main").unwrap();
                    if is_pin {
                        // window.set_decorations(false).unwrap(); // 隐藏顶部栏
                        window.set_shadow(false).unwrap();
                        window.set_ignore_cursor_events(true).unwrap(); // 不允许窗口阻挡鼠标点击
                        window.set_skip_taskbar(true).unwrap(); // 从任务栏中隐藏
                    } else {
                        // window.set_decorations(true).unwrap(); // 显示顶部栏
                        window.set_shadow(true).unwrap();
                        window.set_ignore_cursor_events(false).unwrap(); // 允许窗口阻挡鼠标点击
                        window.set_skip_taskbar(false).unwrap(); // 在任务栏中显示
                    }
                }
            });

            {
                let app_handle = app.handle().clone();
                let state = app_handle.state::<RecorderState>();
                *state.tray_menu.lock().unwrap() = Some(menu);
            }

            Ok(())
        })
        .manage(RecorderState {
            http_server: Mutex::new(None),
            stop_signal: Mutex::new(None),
            tokio_runtime,
            last_port: Mutex::new(None),
            is_pin: AtomicBool::new(false),
            tray_menu: Mutex::new(None), // 初始化tray_menu
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            resize_window,
            set_window_title,
            capture_window_screenshot,
            restart_http_server,
            stop_http_server,
            get_is_server_running,
            get_is_pin,
            set_is_pin,
            start_dragging,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
