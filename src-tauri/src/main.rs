// Sierra Browser — Rust backend
// Лёгкий браузер с гибрид-дизайном Яндекс + Chrome + Safari
//
// Архитектура:
//   - Главное окно (main) — chrome: вкладки, тулбар, адресная строка
//   - Окно контента (content) — нативный WebView, показывает сайт
//   - Синхронизация позиций окон через события

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WindowEvent, LogicalPosition, LogicalSize};
use chrono::Utc;

// ═══════════════════════════════════════════════════════════════════════
//  КОНСТАНТЫ
// ═══════════════════════════════════════════════════════════════════════

const HOMEPAGE: &str = "https://duckduckgo.com";
const SEARCH_URL: &str = "https://duckduckgo.com/?q={query}";
const MAX_HISTORY: usize = 500;
const CHROME_HEIGHT: f64 = 140.0;
const DEFAULT_CONTENT_HEIGHT: f64 = 600.0;

// Флаг для предотвращения бесконечной рекурсии при синхронизации окон
static SYNCING: AtomicBool = AtomicBool::new(false);

// ═══════════════════════════════════════════════════════════════════════
//  МОДЕЛИ ДАННЫХ
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Tab {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub history: Vec<String>,
    pub history_idx: i32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Bookmark {
    pub url: String,
    pub title: String,
    pub added: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub time: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Settings {
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self { theme: "dark".to_string() }
    }
}

#[derive(Default)]
pub struct TabsState {
    pub tabs: Mutex<Vec<Tab>>,
    pub current: Mutex<usize>,
    pub next_id: Mutex<u64>,
}

#[derive(Default)]
pub struct BookmarksState {
    pub bookmarks: Mutex<Vec<Bookmark>>,
}

#[derive(Default)]
pub struct HistoryState {
    pub history: Mutex<Vec<HistoryEntry>>,
}

#[derive(Default)]
pub struct SettingsState {
    pub settings: Mutex<Settings>,
}

// ═══════════════════════════════════════════════════════════════════════
//  КОНФИГУРАЦИЯ
// ═══════════════════════════════════════════════════════════════════════

fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sierra");
    fs::create_dir_all(&dir).ok();
    dir
}

fn bookmarks_path() -> PathBuf { config_dir().join("bookmarks.json") }
fn history_path() -> PathBuf { config_dir().join("history.json") }
fn settings_path() -> PathBuf { config_dir().join("settings.json") }

fn load_json<T: serde::de::DeserializeOwned + Default>(path: &PathBuf) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_json<T: Serialize>(path: &PathBuf, data: &T) {
    if let Ok(s) = serde_json::to_string_pretty(data) {
        let _ = fs::write(path, s);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ
// ═══════════════════════════════════════════════════════════════════════

fn resolve_input(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return HOMEPAGE.to_string();
    }

    let is_url = text.starts_with("http://")
        || text.starts_with("https://")
        || text.starts_with("file://")
        || text.starts_with("about:")
        || text.starts_with("localhost")
        || (text.split('/').next().map(|s| s.contains('.')).unwrap_or(false)
            && !text.contains(' '));

    if is_url {
        if text.starts_with("http://") || text.starts_with("https://")
            || text.starts_with("file://") || text.starts_with("about:") {
            text.to_string()
        } else {
            format!("https://{}", text)
        }
    } else {
        SEARCH_URL.replace("{query}", &text.replace(' ', "+"))
    }
}

/// Синхронизирует позицию content окна с main окном
fn sync_content_position(app: &AppHandle) {
    if SYNCING.load(Ordering::SeqCst) {
        return;
    }
    SYNCING.store(true, Ordering::SeqCst);

    if let Some(main) = app.get_webview_window("main") {
        if let Some(content) = app.get_webview_window("content") {
            if let (Ok(pos), Ok(m_size), Ok(c_size)) =
                (main.outer_position(), main.outer_size(), content.outer_size())
            {
                let _ = content.set_position(LogicalPosition::new(
                    pos.x as f64,
                    (pos.y + m_size.height as i32) as f64,
                ));
                // Ширина content = ширина main, высота не меняется
                let _ = content.set_size(LogicalSize::new(
                    m_size.width as f64,
                    c_size.height as f64,
                ));
            }
        }
    }

    SYNCING.store(false, Ordering::SeqCst);
}

/// Синхронизирует позицию main окна с content окном (при resize content)
fn sync_main_position(app: &AppHandle) {
    if SYNCING.load(Ordering::SeqCst) {
        return;
    }
    SYNCING.store(true, Ordering::SeqCst);

    if let Some(main) = app.get_webview_window("main") {
        if let Some(content) = app.get_webview_window("content") {
            if let (Ok(c_pos), Ok(c_size), Ok(m_size)) =
                (content.outer_position(), content.outer_size(), main.outer_size())
            {
                // Main остаётся сверху, на той же X позиции
                // Content сдвигается вниз на высоту main
                let _ = main.set_position(LogicalPosition::new(
                    c_pos.x as f64,
                    c_pos.y as f64,
                ));
                let _ = content.set_position(LogicalPosition::new(
                    c_pos.x as f64,
                    (c_pos.y + m_size.height as i32) as f64,
                ));
                // Main ширина = content ширина
                let _ = main.set_size(LogicalSize::new(
                    c_size.width as f64,
                    m_size.height as f64,
                ));
            }
        }
    }

    SYNCING.store(false, Ordering::SeqCst);
}

// ═══════════════════════════════════════════════════════════════════════
//  TAURI-КОМАНДЫ — управление вкладками
// ═══════════════════════════════════════════════════════════════════════

#[tauri::command]
fn new_tab(state: State<TabsState>, url: Option<String>) -> Result<Tab, String> {
    let url = url.unwrap_or_else(|| HOMEPAGE.to_string());
    let mut next_id = state.next_id.lock().unwrap();
    let id = *next_id;
    *next_id += 1;
    drop(next_id);

    let tab = Tab {
        id,
        url: url.clone(),
        title: if url == HOMEPAGE { "Новая вкладка".to_string() } else { "Загрузка…".to_string() },
        history: Vec::new(),
        history_idx: -1,
    };

    let mut tabs = state.tabs.lock().unwrap();
    tabs.push(tab.clone());
    let idx = tabs.len() - 1;
    drop(tabs);

    let mut current = state.current.lock().unwrap();
    *current = idx;

    Ok(tab)
}

#[tauri::command]
fn close_tab(state: State<TabsState>, idx: usize) -> Result<bool, String> {
    let mut tabs = state.tabs.lock().unwrap();
    if tabs.is_empty() || idx >= tabs.len() {
        return Ok(false);
    }
    tabs.remove(idx);
    let mut current = state.current.lock().unwrap();
    if *current >= tabs.len() && !tabs.is_empty() {
        *current = tabs.len() - 1;
    } else if tabs.is_empty() {
        *current = 0;
    }
    Ok(true)
}

#[tauri::command]
fn switch_tab(state: State<TabsState>, idx: usize) -> Result<Option<Tab>, String> {
    let tabs = state.tabs.lock().unwrap();
    if idx >= tabs.len() {
        return Ok(None);
    }
    let mut current = state.current.lock().unwrap();
    *current = idx;
    Ok(Some(tabs[idx].clone()))
}

#[tauri::command]
fn get_tabs(state: State<TabsState>) -> Result<(Vec<Tab>, usize), String> {
    let tabs = state.tabs.lock().unwrap().clone();
    let current = *state.current.lock().unwrap();
    Ok((tabs, current))
}

#[tauri::command]
fn navigate(state: State<TabsState>, url: String) -> Result<Option<Tab>, String> {
    let resolved = resolve_input(&url);
    let mut tabs = state.tabs.lock().unwrap();
    let current = *state.current.lock().unwrap();
    if current >= tabs.len() {
        return Ok(None);
    }
    let tab = &mut tabs[current];

    if !tab.url.is_empty() && tab.url != HOMEPAGE {
        if tab.history_idx < (tab.history.len() as i32 - 1) {
            tab.history.truncate((tab.history_idx + 1) as usize);
        }
        tab.history.push(tab.url.clone());
        tab.history_idx = (tab.history.len() as i32) - 1;
    }
    tab.url = resolved.clone();
    tab.title = "Загрузка…".to_string();

    Ok(Some(tab.clone()))
}

#[tauri::command]
fn go_back(state: State<TabsState>) -> Result<Option<Tab>, String> {
    let mut tabs = state.tabs.lock().unwrap();
    let current = *state.current.lock().unwrap();
    if current >= tabs.len() {
        return Ok(None);
    }
    let tab = &mut tabs[current];
    if tab.history_idx > 0 {
        tab.history_idx -= 1;
        let url = tab.history[tab.history_idx as usize].clone();
        tab.url = url.clone();
        return Ok(Some(tab.clone()));
    }
    Ok(None)
}

#[tauri::command]
fn go_forward(state: State<TabsState>) -> Result<Option<Tab>, String> {
    let mut tabs = state.tabs.lock().unwrap();
    let current = *state.current.lock().unwrap();
    if current >= tabs.len() {
        return Ok(None);
    }
    let tab = &mut tabs[current];
    if tab.history_idx < (tab.history.len() as i32 - 1) {
        tab.history_idx += 1;
        let url = tab.history[tab.history_idx as usize].clone();
        tab.url = url.clone();
        return Ok(Some(tab.clone()));
    }
    Ok(None)
}

#[tauri::command]
fn go_home(state: State<TabsState>) -> Result<Option<Tab>, String> {
    navigate(state, HOMEPAGE.to_string())
}

#[tauri::command]
fn update_tab_title(state: State<TabsState>, title: String) -> Result<(), String> {
    let mut tabs = state.tabs.lock().unwrap();
    let current = *state.current.lock().unwrap();
    if current < tabs.len() {
        tabs[current].title = title;
    }
    Ok(())
}

#[tauri::command]
fn update_tab_url(state: State<TabsState>, url: String) -> Result<(), String> {
    let mut tabs = state.tabs.lock().unwrap();
    let current = *state.current.lock().unwrap();
    if current < tabs.len() {
        tabs[current].url = url;
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  TAURI-КОМАНДЫ — контент окно (нативный WebView)
// ═══════════════════════════════════════════════════════════════════════

#[tauri::command]
fn open_content(app: AppHandle, url: String) -> Result<(), String> {
    let main = app.get_webview_window("main").ok_or("no main window")?;
    let pos = main.outer_position().map_err(|e| e.to_string())?;
    let size = main.outer_size().map_err(|e| e.to_string())?;

    let x = pos.x as f64;
    let y = (pos.y + size.height as i32) as f64;
    let width = size.width as f64;
    let height = DEFAULT_CONTENT_HEIGHT;

    let parsed_url: tauri::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;

    if let Some(content) = app.get_webview_window("content") {
        // Окно уже существует — навигируем
        content.navigate(parsed_url).map_err(|e| e.to_string())?;
        content.set_position(LogicalPosition::new(x, y)).map_err(|e| e.to_string())?;
        content.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())?;
        content.show().map_err(|e| e.to_string())?;
        content.set_focus().map_err(|e| e.to_string())?;
    } else {
        // Создаём новое окно контента
        let content = tauri::WebviewWindow::builder(
            &app,
            "content",
            tauri::WebviewUrl::External(parsed_url),
        )
        .title("Sierra")
        .position(x, y)
        .inner_size(width, height)
        .decorations(false)
        .visible(true)
        .build()
        .map_err(|e| e.to_string())?;

        // При закрытии content — закрываем main
        let app_handle = app.clone();
        content.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { .. } = event {
                if let Some(main) = app_handle.get_webview_window("main") {
                    let _ = main.close();
                }
            }
            if let WindowEvent::Moved(_) = event {
                sync_main_position(&app_handle);
            }
            if let WindowEvent::Resized(_) = event {
                sync_main_position(&app_handle);
            }
        });
    }
    Ok(())
}

#[tauri::command]
fn hide_content(app: AppHandle) -> Result<(), String> {
    if let Some(content) = app.get_webview_window("content") {
        content.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn focus_main(app: AppHandle) -> Result<(), String> {
    if let Some(main) = app.get_webview_window("main") {
        main.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  TAURI-КОМАНДЫ — закладки
// ═══════════════════════════════════════════════════════════════════════

#[tauri::command]
fn get_bookmarks(state: State<BookmarksState>) -> Result<Vec<Bookmark>, String> {
    Ok(state.bookmarks.lock().unwrap().clone())
}

#[tauri::command]
fn is_bookmarked(state: State<BookmarksState>, url: String) -> Result<bool, String> {
    Ok(state.bookmarks.lock().unwrap().iter().any(|b| b.url == url))
}

#[tauri::command]
fn toggle_bookmark(state: State<BookmarksState>, url: String, title: String) -> Result<bool, String> {
    let mut bookmarks = state.bookmarks.lock().unwrap();
    if let Some(idx) = bookmarks.iter().position(|b| b.url == url) {
        bookmarks.remove(idx);
        save_json(&bookmarks_path(), &*bookmarks);
        Ok(false)
    } else {
        bookmarks.push(Bookmark {
            url, title,
            added: Utc::now().to_rfc3339(),
        });
        save_json(&bookmarks_path(), &*bookmarks);
        Ok(true)
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  TAURI-КОМАНДЫ — история
// ═══════════════════════════════════════════════════════════════════════

#[tauri::command]
fn get_history(state: State<HistoryState>) -> Result<Vec<HistoryEntry>, String> {
    Ok(state.history.lock().unwrap().clone())
}

#[tauri::command]
fn add_to_history(state: State<HistoryState>, url: String, title: String) -> Result<(), String> {
    if url.is_empty() || url.starts_with("data:") || url.starts_with("about:") {
        return Ok(());
    }
    let mut history = state.history.lock().unwrap();
    if let Some(last) = history.last() {
        if last.url == url {
            return Ok(());
        }
    }
    history.push(HistoryEntry {
        url, title,
        time: Utc::now().to_rfc3339(),
    });
    if history.len() > MAX_HISTORY {
        let start = history.len() - MAX_HISTORY;
        *history = history.split_off(start);
    }
    save_json(&history_path(), &*history);
    Ok(())
}

#[tauri::command]
fn clear_history(state: State<HistoryState>) -> Result<(), String> {
    let mut history = state.history.lock().unwrap();
    history.clear();
    save_json(&history_path(), &*history);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  TAURI-КОМАНДЫ — настройки
// ═══════════════════════════════════════════════════════════════════════

#[tauri::command]
fn get_theme(state: State<SettingsState>) -> Result<String, String> {
    Ok(state.settings.lock().unwrap().theme.clone())
}

#[tauri::command]
fn toggle_theme(state: State<SettingsState>) -> Result<String, String> {
    let mut settings = state.settings.lock().unwrap();
    settings.theme = if settings.theme == "dark" { "light".to_string() } else { "dark".to_string() };
    save_json(&settings_path(), &*settings);
    Ok(settings.theme.clone())
}

// ═══════════════════════════════════════════════════════════════════════
//  ТОЧКА ВХОДА
// ═══════════════════════════════════════════════════════════════════════

fn main() {
    tauri::Builder::default()
        .manage(TabsState::default())
        .manage(BookmarksState { bookmarks: Mutex::new(load_json(&bookmarks_path())) })
        .manage(HistoryState { history: Mutex::new(load_json(&history_path())) })
        .manage(SettingsState { settings: Mutex::new(load_json(&settings_path())) })
        .invoke_handler(tauri::generate_handler![
            new_tab,
            close_tab,
            switch_tab,
            get_tabs,
            navigate,
            go_back,
            go_forward,
            go_home,
            update_tab_title,
            update_tab_url,
            open_content,
            hide_content,
            focus_main,
            get_bookmarks,
            is_bookmarked,
            toggle_bookmark,
            get_history,
            add_to_history,
            clear_history,
            get_theme,
            toggle_theme,
        ])
        .setup(|app| {
            let main = app.get_webview_window("main").unwrap();
            let app_handle = app.handle().clone();

            // Синхронизация: при движении/resize main → content следует
            main.on_window_event(move |event| {
                let app = app_handle.clone();
                match event {
                    WindowEvent::Moved(_) => {
                        sync_content_position(&app);
                    }
                    WindowEvent::Resized(_) => {
                        sync_content_position(&app);
                    }
                    WindowEvent::CloseRequested { .. } => {
                        if let Some(content) = app.get_webview_window("content") {
                            let _ = content.close();
                        }
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Ошибка запуска Sierra");
}
