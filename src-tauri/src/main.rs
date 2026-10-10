// Sierra Browser — Rust backend
// Лёгкий браузер с гибрид-дизайном Яндекс + Chrome + Safari
//
// Структура:
//   - TabsState — управление вкладками (с навигацией, историей)
//   - BookmarksState — закладки
//   - HistoryState — история (до 500 записей)
//   - SettingsState — настройки (тема)
//
// Все Tauri-команды (#[tauri::command]) доступны из JS как
//   window.__TAURI__.invoke('command_name', { args })

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;
use chrono::Utc;
// ═══════════════════════════════════════════════════════════════════════
//  КОНСТАНТЫ
// ═══════════════════════════════════════════════════════════════════════

const HOMEPAGE: &str = "https://duckduckgo.com";
const SEARCH_URL: &str = "https://duckduckgo.com/?q={query}";
const MAX_HISTORY: usize = 500;

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
//  КОНФИГУРАЦИЯ — пути к файлам настроек
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

/// Определяет, URL это или поисковый запрос
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

// ═══════════════════════════════════════════════════════════════════════
//  TAURI-КОМАНДЫ — управление вкладками
// ═══════════════════════════════════════════════════════════════════════

/// Создать новую вкладку
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

/// Закрыть вкладку по индексу
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

/// Переключиться на вкладку по индексу
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

/// Получить список всех вкладок и индекс текущей
#[tauri::command]
fn get_tabs(state: State<TabsState>) -> Result<(Vec<Tab>, usize), String> {
    let tabs = state.tabs.lock().unwrap().clone();
    let current = *state.current.lock().unwrap();
    Ok((tabs, current))
}

/// Навигация по URL
#[tauri::command]
fn navigate(state: State<TabsState>, url: String) -> Result<Option<Tab>, String> {
    let resolved = resolve_input(&url);
    let mut tabs = state.tabs.lock().unwrap();
    let current = *state.current.lock().unwrap();
    if current >= tabs.len() {
        return Ok(None);
    }
    let tab = &mut tabs[current];

    // Сохраняем текущий URL в историю вкладки
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

/// Назад
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

/// Вперёд
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

/// Домой
#[tauri::command]
fn go_home(state: State<TabsState>) -> Result<Option<Tab>, String> {
    navigate(state, HOMEPAGE.to_string())
}
/// Обновить заголовок вкладки (вызывается из JS при загрузке страницы)
#[tauri::command]
fn update_tab_title(state: State<TabsState>, title: String) -> Result<(), String> {
    let mut tabs = state.tabs.lock().unwrap();
    let current = *state.current.lock().unwrap();
    if current < tabs.len() {
        tabs[current].title = title;
    }
    Ok(())
}

/// Обновить URL вкладки (вызывается из JS при навигации внутри страницы)
#[tauri::command]
fn update_tab_url(state: State<TabsState>, url: String) -> Result<(), String> {
    let mut tabs = state.tabs.lock().unwrap();
    let current = *state.current.lock().unwrap();
    if current < tabs.len() {
        tabs[current].url = url.clone();
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
fn toggle_bookmark(
    state: State<BookmarksState>,
    url: String,
    title: String,
) -> Result<bool, String> {
    let mut bookmarks = state.bookmarks.lock().unwrap();
    if let Some(idx) = bookmarks.iter().position(|b| b.url == url) {
        bookmarks.remove(idx);
        save_json(&bookmarks_path(), &*bookmarks);
        Ok(false)
    } else {
        bookmarks.push(Bookmark {
            url,
            title,
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
        url,
        title,
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
        .manage(BookmarksState {
            bookmarks: Mutex::new(load_json(&bookmarks_path())),
        })
        .manage(HistoryState {
            history: Mutex::new(load_json(&history_path())),
        })
        .manage(SettingsState {
            settings: Mutex::new(load_json(&settings_path())),
        })
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
            get_bookmarks,
            is_bookmarked,
            toggle_bookmark,
            get_history,
            add_to_history,
            clear_history,
            get_theme,
            toggle_theme,
        ])
        .run(tauri::generate_context!())
        .expect("Ошибка запуска Sierra");
}
