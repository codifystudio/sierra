// Sierra v5.0.0 — браузер на Rust (wry + tao, без Tauri)
//
// Архитектура:
//   - ОДНО окно tao
//   - ДВА WebView в нём (через wry):
//     • chrome (сверху, 140px) — HTML интерфейс
//     • content (снизу) — нативный WebView показывает сайт
//   - IPC: chrome → Rust через webview.eval() / ipc_handler
//   - Навигация: content_webview.load_url(url) — прямой вызов wry

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use tao::{
    event::{Event, WindowEvent, StartCause},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, WindowId},
    dpi::LogicalSize,
};
use wry::{WebViewBuilder, WebView};

// ═══════════════════════════════════════════════════════════════════════
//  КОНСТАНТЫ
// ═══════════════════════════════════════════════════════════════════════

const HOMEPAGE: &str = "https://duckduckgo.com";
const SEARCH_URL: &str = "https://duckduckgo.com/?q={query}";
const CHROME_HEIGHT: f64 = 140.0;
const MAX_HISTORY: usize = 500;

// ═══════════════════════════════════════════════════════════════════════
//  МОДЕЛИ ДАННЫХ
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Tab {
    id: u64,
    url: String,
    title: String,
    history: Vec<String>,
    history_idx: i32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Bookmark {
    url: String,
    title: String,
    added: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct HistoryEntry {
    url: String,
    title: String,
    time: String,
}

// ═══════════════════════════════════════════════════════════════════════
//  STATE — общее состояние браузера
// ═══════════════════════════════════════════════════════════════════════

struct State {
    tabs: Vec<Tab>,
    current: usize,
    next_id: u64,
    bookmarks: Vec<Bookmark>,
    history: Vec<HistoryEntry>,
    theme: String,
}

impl State {
    fn new() -> Self {
        Self {
            tabs: vec![Tab {
                id: 0,
                url: HOMEPAGE.to_string(),
                title: "Новая вкладка".to_string(),
                history: vec![],
                history_idx: -1,
            }],
            current: 0,
            next_id: 1,
            bookmarks: load_bookmarks(),
            history: load_history(),
            theme: load_theme(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  ФАЙЛЫ КОНФИГУРАЦИИ
// ═══════════════════════════════════════════════════════════════════════

fn config_dir() -> PathBuf {
    let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("sierra");
    fs::create_dir_all(&dir).ok();
    dir
}

fn bookmarks_path() -> PathBuf { config_dir().join("bookmarks.json") }
fn history_path() -> PathBuf { config_dir().join("history.json") }
fn settings_path() -> PathBuf { config_dir().join("settings.json") }

fn load_json_file<T: serde::de::DeserializeOwned + Default>(path: &PathBuf) -> T {
    fs::read_to_string(path).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_json_file<T: Serialize>(path: &PathBuf, data: &T) {
    if let Ok(s) = serde_json::to_string_pretty(data) {
        let _ = fs::write(path, s);
    }
}

fn load_bookmarks() -> Vec<Bookmark> { load_json_file(&bookmarks_path()) }
fn load_history() -> Vec<HistoryEntry> { load_json_file(&history_path()) }
fn load_theme() -> String {
    let settings: HashMap<String, String> = load_json_file(&settings_path());
    settings.get("theme").cloned().unwrap_or_else(|| "dark".to_string())
}
fn save_theme(theme: &str) {
    let mut settings = HashMap::new();
    settings.insert("theme".to_string(), theme.to_string());
    save_json_file(&settings_path(), &settings);
}

// ═══════════════════════════════════════════════════════════════════════
//  РЕЗОЛВ URL
// ═══════════════════════════════════════════════════════════════════════

fn resolve_input(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return HOMEPAGE.to_string();
    }
    let is_url = text.starts_with("http://") || text.starts_with("https://")
        || text.starts_with("file://") || text.starts_with("about:")
        || text.starts_with("localhost")
        || (text.split('/').next().map(|s| s.contains('.')).unwrap_or(false) && !text.contains(' '));
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
//  HTML CHROME (рендерится в chrome WebView)
// ═══════════════════════════════════════════════════════════════════════

fn chrome_html(state: &State) -> String {
    let tabs_html: String = state.tabs.iter().enumerate().map(|(i, t)| {
        let active = if i == state.current { "active" } else { "" };
        let title = if t.title.is_empty() { "Без названия".to_string() } else { t.title.clone() };
        let truncated = if title.len() > 28 { format!("{}…", &title[..28]) } else { title };
        let safe = html_escape(&truncated);
        format!(
            r#"<div class="tab {active}" data-idx="{i}"><span class="tab-title">{safe}</span><span class="tab-close" data-idx="{i}">×</span></div>"#,
            active = active, i = i, safe = safe
        )
    }).collect();

    let theme = &state.theme;
    let bookmark_active = {
        let url = &state.tabs[state.current].url;
        state.bookmarks.iter().any(|b| b.url == *url) && url != HOMEPAGE
    };

    format!(r##"<!DOCTYPE html>
<html lang="ru">
<head>
<meta charset="UTF-8">
<style>
* {{ margin: 0; padding: 0; box-sizing: border-box; font-family: 'SF Pro Display', 'Segoe UI', sans-serif; }}
:root {{ --red: #fc3f1d; --yellow: #ffcc00; }}
html, body {{ height: 100vh; overflow: hidden; -webkit-user-select: none; user-select: none; }}
body.dark {{ background: #1c1c1e; color: #fff; }}
body.light {{ background: #f5f5f7; color: #1c1c1e; }}

body.dark .tab-strip {{ background: #2a2a2d; }}
body.dark .toolbar {{ background: #2a2a2d; border-bottom: 1px solid #3a3a3d; }}
body.dark .tab {{ background: #3a3a3d; color: #b0b0b5; }}
body.dark .tab.active {{ background: #5b5b5e; color: #fff; }}
body.dark .tab:hover:not(.active) {{ background: #48484b; color: #d4d4d8; }}
body.dark .nav-btn {{ color: #d4d4d8; }}
body.dark .nav-btn:hover {{ background: #3a3a3d; }}
body.dark .nav-btn:active {{ background: var(--red); color: #fff; }}
body.dark .address-bar {{ background: #3a3a3d; color: #fff; border: 1.5px solid transparent; }}
body.dark .address-bar:focus {{ border-color: var(--red); background: #4a4a4d; }}
body.dark .address-bar::placeholder {{ color: #8a8a8e; }}
body.dark .new-tab-btn {{ color: #d4d4d8; }}
body.dark .new-tab-btn:hover {{ background: #3a3a3d; }}
body.dark .menu {{ background: #2a2a2d; border: 1px solid #3a3a3d; }}
body.dark .menu-item:hover {{ background: var(--red); color: #fff; }}
body.dark .menu-item:hover svg {{ stroke: #fff; }}
body.dark .status {{ background: #2a2a2d; color: #8a8a8e; border-top: 2px solid var(--red); }}
body.dark .progress-fill {{ background: var(--red); }}
body.dark .start-search {{ background: #3a3a3d; color: #fff; border: 1.5px solid rgba(255,255,255,0.05); }}
body.dark .start-search:focus {{ border-color: var(--red); }}
body.dark .shortcut {{ background: rgba(255,255,255,0.04); }}
body.dark .shortcut:hover {{ background: rgba(252,63,29,0.15); }}

body.light .tab-strip {{ background: #e8e8ed; }}
body.light .toolbar {{ background: #e8e8ed; border-bottom: 1px solid #d8d8dd; }}
body.light .tab {{ background: #d8d8dd; color: #5a5a5e; }}
body.light .tab.active {{ background: #fff; color: #1c1c1e; }}
body.light .tab:hover:not(.active) {{ background: #c8c8cd; color: #3a3a3e; }}
body.light .nav-btn {{ color: #5a5a5e; }}
body.light .nav-btn:hover {{ background: #d8d8dd; }}
body.light .address-bar {{ background: #fff; color: #1c1c1e; border: 1px solid #d8d8dd; }}
body.light .address-bar:focus {{ border-color: var(--red); }}
body.light .new-tab-btn {{ color: #5a5a5e; }}
body.light .menu {{ background: #fff; border: 1px solid #d8d8dd; }}
body.light .menu-item:hover {{ background: var(--red); color: #fff; }}
body.light .status {{ background: #e8e8ed; color: #5a5a5e; border-top: 2px solid var(--red); }}
body.light .start-search {{ background: #fff; color: #1c1c1e; border: 1px solid #d8d8dd; }}
body.light .shortcut {{ background: rgba(0,0,0,0.04); }}

.tab-strip {{ display: flex; align-items: flex-start; padding: 6px 8px 0 8px; gap: 2px; min-height: 38px; }}
.tabs {{ display: flex; gap: 2px; flex: 1; overflow-x: auto; scrollbar-width: none; }}
.tabs::-webkit-scrollbar {{ display: none; }}
.tab {{ display: flex; align-items: center; padding: 6px 8px 6px 12px; border-radius: 10px; cursor: pointer; min-width: 120px; max-width: 220px; height: 32px; flex-shrink: 0; }}
.tab-title {{ flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 13px; margin-right: 6px; }}
.tab-close {{ width: 18px; height: 18px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 16px; }}
.tab-close:hover {{ background: rgba(252,63,29,0.25); color: #fff; }}
.new-tab-btn {{ width: 28px; height: 28px; border-radius: 50%; border: none; background: transparent; cursor: pointer; font-size: 20px; font-weight: bold; display: flex; align-items: center; justify-content: center; margin: 4px 0 0 6px; flex-shrink: 0; }}

.toolbar {{ display: flex; align-items: center; padding: 6px 10px; gap: 4px; }}
.nav-btn {{ width: 34px; height: 34px; border-radius: 50%; border: none; background: transparent; cursor: pointer; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }}
.nav-btn svg {{ width: 18px; height: 18px; }}
.nav-btn:disabled {{ opacity: 0.3; cursor: default; }}
.address-bar {{ flex: 1; height: 36px; border-radius: 18px; padding: 0 18px; font-size: 14px; border: 1.5px solid transparent; outline: none; min-width: 0; }}
.nav-btn.bookmark-active svg {{ fill: var(--yellow) !important; stroke: var(--yellow) !important; }}

.menu {{ position: absolute; top: 76px; right: 10px; border-radius: 10px; padding: 8px; min-width: 240px; display: none; z-index: 1000; box-shadow: 0 8px 24px rgba(0,0,0,0.3); }}
.menu.show {{ display: block; }}
.menu-item {{ padding: 8px 14px; border-radius: 6px; cursor: pointer; font-size: 13px; display: flex; align-items: center; gap: 12px; }}
.menu-item svg {{ width: 16px; height: 16px; flex-shrink: 0; }}
.menu-sep {{ height: 1px; margin: 4px 8px; opacity: 0.3; background: currentColor; }}
.menu-sc {{ margin-left: auto; opacity: 0.5; font-size: 11px; }}

.status {{ padding: 4px 12px; font-size: 11px; display: flex; align-items: center; gap: 10px; min-height: 22px; }}
.progress {{ flex: 1; height: 3px; background: rgba(128,128,128,0.1); border-radius: 1px; overflow: hidden; max-width: 200px; }}
.progress-fill {{ height: 100%; width: 0; border-radius: 1px; transition: width 0.2s; }}

.start-page {{ flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 20px; overflow-y: auto; }}
.start-logo {{ font-size: 48px; font-weight: 200; letter-spacing: -3px; margin-bottom: 20px; background: linear-gradient(135deg, var(--red), var(--yellow)); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; }}
.start-search {{ width: 100%; max-width: 580px; height: 48px; border-radius: 24px; padding: 0 24px; font-size: 15px; outline: none; }}
.start-shortcuts {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(100px, 1fr)); gap: 12px; margin-top: 24px; width: 100%; max-width: 680px; }}
.shortcut {{ padding: 12px 8px; border-radius: 10px; text-align: center; cursor: pointer; font-size: 11px; }}
.shortcut-icon {{ width: 40px; height: 40px; border-radius: 50%; margin: 0 auto 6px; display: flex; align-items: center; justify-content: center; font-size: 20px; font-weight: 700; }}
.hint {{ margin-top: 20px; font-size: 11px; opacity: 0.5; text-align: center; }}
</style>
</head>
<body class="{theme}">

<div class="tab-strip">
  <div class="tabs" id="tabs">{tabs_html}</div>
  <button class="new-tab-btn" id="btnNew" title="Новая вкладка (Ctrl+T)">+</button>
</div>

<div class="toolbar">
  <button class="nav-btn" id="btnBack" title="Назад">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 12H5M12 19l-7-7 7-7"/></svg>
  </button>
  <button class="nav-btn" id="btnFwd" title="Вперёд">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14M12 5l7 7-7 7"/></svg>
  </button>
  <button class="nav-btn" id="btnRefresh" title="Обновить">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M23 4v6h-6M1 20v-6h6"/><path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/></svg>
  </button>
  <button class="nav-btn" id="btnHome" title="Домой">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
  </button>
  <input type="text" class="address-bar" id="addr" placeholder="URL или поисковый запрос..." autocomplete="off" spellcheck="false">
  <button class="nav-btn" id="btnBm" title="Закладка">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>
  </button>
  <button class="nav-btn" id="btnMenu" title="Меню">
    <svg viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="5" r="2"/><circle cx="12" cy="12" r="2"/><circle cx="12" cy="19" r="2"/></svg>
  </button>
</div>

<div class="menu" id="menu">
  <div class="menu-item" data-act="new"><span>Новая вкладка</span><span class="menu-sc">Ctrl+T</span></div>
  <div class="menu-item" data-act="bookmarks"><span>Закладки</span></div>
  <div class="menu-item" data-act="history"><span>История</span><span class="menu-sc">Ctrl+H</span></div>
  <div class="menu-sep"></div>
  <div class="menu-item" data-act="theme"><span>Сменить тему</span><span class="menu-sc">Ctrl+Shift+T</span></div>
  <div class="menu-item" data-act="clear"><span>Очистить историю</span></div>
</div>

<div class="start-page" id="start">
  <div class="start-logo">Sierra</div>
  <input type="text" class="start-search" id="search" placeholder="Поиск в DuckDuckGo или введите URL" autocomplete="off">
  <div class="start-shortcuts" id="shortcuts"></div>
  <div class="hint">Ctrl+T — новая вкладка · Ctrl+L — адрес · Ctrl+W — закрыть</div>
</div>

<div class="status">
  <span id="status">Готово</span>
  <div class="progress" id="prog" style="display:none"><div class="progress-fill" id="progFill"></div></div>
</div>

<script>
// IPC через wry: window.ipc.postMessage
function send(cmd, args) {{
  const msg = JSON.stringify({{cmd, args: args || {{}}}});
  if (window.ipc && window.ipc.postMessage) {{
    window.ipc.postMessage(msg);
  }} else {{
    console.log('IPC НЕ ДОСТУПЕН');
  }}
}}

// Кнопки навигации
document.getElementById('btnBack').onclick = () => send('back');
document.getElementById('btnFwd').onclick = () => send('forward');
document.getElementById('btnRefresh').onclick = () => send('refresh');
document.getElementById('btnHome').onclick = () => send('home');
document.getElementById('btnNew').onclick = () => send('new_tab');
document.getElementById('btnBm').onclick = () => send('toggle_bookmark');
document.getElementById('btnMenu').onclick = (e) => {{
  e.stopPropagation();
  document.getElementById('menu').classList.toggle('show');
}};

// Адресная строка
const addr = document.getElementById('addr');
addr.addEventListener('keydown', (e) => {{
  if (e.key === 'Enter' && addr.value.trim()) {{
    send('navigate', {{url: addr.value.trim()}});
    addr.blur();
  }} else if (e.key === 'Escape') addr.blur();
}});
addr.addEventListener('focus', () => addr.select());

// Поиск на стартовой
const search = document.getElementById('search');
search.addEventListener('keydown', (e) => {{
  if (e.key === 'Enter' && search.value.trim()) {{
    send('navigate', {{url: search.value.trim()}});
  }}
}});

// Меню
document.querySelectorAll('.menu-item').forEach(item => {{
  item.addEventListener('click', () => {{
    send(item.dataset.act);
    document.getElementById('menu').classList.remove('show');
  }});
}});

// Клик вне меню
document.addEventListener('click', (e) => {{
  if (!e.target.closest('#menu') && !e.target.closest('#btnMenu')) {{
    document.getElementById('menu').classList.remove('show');
  }}
}});

// Вкладки — делегирование событий
document.getElementById('tabs').addEventListener('click', (e) => {{
  const tab = e.target.closest('.tab');
  const close = e.target.closest('.tab-close');
  if (close) {{
    e.stopPropagation();
    send('close_tab', {{idx: parseInt(close.dataset.idx)}});
  }} else if (tab) {{
    send('switch_tab', {{idx: parseInt(tab.dataset.idx)}});
  }}
}});

// Горячие клавиши
document.addEventListener('keydown', (e) => {{
  const ctrl = e.ctrlKey || e.metaKey;
  if (ctrl && e.key === 't') {{ e.preventDefault(); send('new_tab'); }}
  else if (ctrl && e.key === 'w') {{ e.preventDefault(); send('close_current'); }}
  else if (ctrl && e.key === 'l') {{ e.preventDefault(); addr.focus(); }}
  else if (ctrl && e.key === 'd') {{ e.preventDefault(); send('toggle_bookmark'); }}
  else if (ctrl && e.key === 'h') {{ e.preventDefault(); send('history'); }}
  else if (ctrl && e.shiftKey && (e.key === 'T' || e.key === 't')) {{ e.preventDefault(); send('theme'); }}
  else if (e.altKey && e.key === 'ArrowLeft') {{ e.preventDefault(); send('back'); }}
  else if (e.altKey && e.key === 'ArrowRight') {{ e.preventDefault(); send('forward'); }}
  else if (e.altKey && e.key === 'Home') {{ e.preventDefault(); send('home'); }}
  else if (e.key === 'F5') {{ e.preventDefault(); send('refresh'); }}
  else if (e.key === 'Escape') document.getElementById('menu').classList.remove('show');
}});

// Shortcuts на start page
const SHORTCUTS = [
  {{url:'https://github.com', name:'GitHub', icon:'G', color:'#24292e', fg:'#fff'}},
  {{url:'https://duckduckgo.com', name:'DuckDuckGo', icon:'D', color:'#de5833', fg:'#fff'}},
  {{url:'https://youtube.com', name:'YouTube', icon:'Y', color:'#ff0000', fg:'#fff'}},
  {{url:'https://wikipedia.org', name:'Wikipedia', icon:'W', color:'#000', fg:'#fff'}},
  {{url:'https://news.ycombinator.com', name:'HN', icon:'H', color:'#f6f6ef', fg:'#ff6600'}},
  {{url:'https://reddit.com', name:'Reddit', icon:'R', color:'#ff4500', fg:'#fff'}},
  {{url:'https://twitter.com', name:'X', icon:'X', color:'#1da1f2', fg:'#fff'}},
  {{url:'https://stackoverflow.com', name:'SO', icon:'S', color:'#f48024', fg:'#fff'}}
];
const sc = document.getElementById('shortcuts');
SHORTCUTS.forEach(s => {{
  const el = document.createElement('div');
  el.className = 'shortcut';
  el.innerHTML = `<div class="shortcut-icon" style="color:${{s.fg}};background:${{s.color}}">${{s.icon}}</div><div>${{s.name}}</div>`;
  el.onclick = () => send('navigate', {{url: s.url}});
  sc.appendChild(el);
}});

// Глобальный объект для приёма команд от Rust (через webview.eval)
window.sierra_update = function(state) {{
  // state = {{tabs: [...], current, theme, address, bookmarked, status, progress}}
  if (state.tabs) {{
    const tabsEl = document.getElementById('tabs');
    tabsEl.innerHTML = state.tabs.map((t, i) => {{
      const active = i === state.current ? 'active' : '';
      const title = (t.title || 'Без названия').substring(0, 28) + ((t.title || '').length > 28 ? '…' : '');
      const safe = title.replace(/</g, '&lt;').replace(/>/g, '&gt;');
      return `<div class="tab ${{active}}" data-idx="${{i}}"><span class="tab-title">${{safe}}</span><span class="tab-close" data-idx="${{i}}">×</span></div>`;
    }}).join('');
  }}
  if (state.theme) document.body.className = state.theme;
  if (state.address !== undefined && document.activeElement !== addr) addr.value = state.address;
  if (state.bookmarked) document.getElementById('btnBm').classList.add('bookmark-active');
  else document.getElementById('btnBm').classList.remove('bookmark-active');
  if (state.status !== undefined) document.getElementById('status').textContent = state.status;
  if (state.progress !== undefined) {{
    document.getElementById('prog').style.display = state.progress < 100 ? 'block' : 'none';
    document.getElementById('progFill').style.width = state.progress + '%';
  }}
  if (state.showStart !== undefined) {{
    document.getElementById('start').style.display = state.showStart ? 'flex' : 'none';
  }}
}};

console.log('Sierra init OK, IPC: ' + (window.ipc ? 'yes' : 'no'));
send('init');
</script>
</body>
</html>"##, theme = theme, tabs_html = tabs_html)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// ═══════════════════════════════════════════════════════════════════════
//  ОБРАБОТКА IPC СООБЩЕНИЙ
// ═══════════════════════════════════════════════════════════════════════

fn handle_ipc(
    msg: &str,
    state: &Arc<Mutex<State>>,
    content_webview: &Arc<Mutex<Option<WebView>>>,
) -> Option<String> {
    let v: Value = match serde_json::from_str(msg) {
        Ok(v) => v,
        Err(e) => return Some(format!("IPC parse error: {}", e)),
    };
    let cmd = v.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
    let args = v.get("args").cloned().unwrap_or(Value::Null);

    let mut st = state.lock().unwrap();

    let result = match cmd {
        "init" => {
            // Первый запрос — отдаём состояние
            Some(update_state_json(&st))
        }
        "new_tab" => {
            let id = st.next_id;
            st.next_id += 1;
            st.tabs.push(Tab {
                id,
                url: HOMEPAGE.to_string(),
                title: "Новая вкладка".to_string(),
                history: vec![],
                history_idx: -1,
            });
            st.current = st.tabs.len() - 1;
            Some(update_state_json(&st))
        }
        "switch_tab" => {
            let idx = args.get("idx").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
            if idx < st.tabs.len() {
                st.current = idx;
                let url = st.tabs[idx].url.clone();
                drop(st);
                load_url_in_content(content_webview, &url);
                return Some(update_state_json(&state.lock().unwrap()));
            }
            None
        }
        "close_tab" => {
            let idx = args.get("idx").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
            if idx < st.tabs.len() && st.tabs.len() > 1 {
                st.tabs.remove(idx);
                if st.current > idx {
                    st.current -= 1;
                } else if st.current >= st.tabs.len() {
                    st.current = st.tabs.len() - 1;
                }
                let url = st.tabs[st.current].url.clone();
                drop(st);
                load_url_in_content(content_webview, &url);
                return Some(update_state_json(&state.lock().unwrap()));
            }
            Some(update_state_json(&st))
        }
        "close_current" => {
            if st.tabs.len() > 1 {
                let cur = st.current;
                st.tabs.remove(cur);
                if st.current >= st.tabs.len() {
                    st.current = st.tabs.len() - 1;
                }
                let url = st.tabs[st.current].url.clone();
                drop(st);
                load_url_in_content(content_webview, &url);
                return Some(update_state_json(&state.lock().unwrap()));
            }
            Some(update_state_json(&st))
        }
        "navigate" => {
            let url = args.get("url").and_then(|u| u.as_str()).unwrap_or("");
            if !url.is_empty() {
                let resolved = resolve_input(url);
                let cur = st.current;
                let cur_url = st.tabs[cur].url.clone();
                if !cur_url.is_empty() && cur_url != resolved {
                    let hist_idx = st.tabs[cur].history_idx;
                    if hist_idx < (st.tabs[cur].history.len() as i32 - 1) {
                        st.tabs[cur].history.truncate((hist_idx + 1) as usize);
                    }
                    st.tabs[cur].history.push(cur_url.clone());
                    st.tabs[cur].history_idx = (st.tabs[cur].history.len() as i32) - 1;
                    st.history.push(HistoryEntry {
                        url: cur_url,
                        title: st.tabs[cur].title.clone(),
                        time: chrono_now(),
                    });
                    if st.history.len() > MAX_HISTORY {
                        let excess = st.history.len() - MAX_HISTORY;
                        st.history.drain(0..excess);
                    }
                    save_json_file(&history_path(), &st.history);
                }
                st.tabs[cur].url = resolved.clone();
                st.tabs[cur].title = "Загрузка…".to_string();
                drop(st);
                load_url_in_content(content_webview, &resolved);
                return Some(update_state_json(&state.lock().unwrap()));
            }
            None
        }
        "back" => {
            let cur = st.current;
            if st.tabs[cur].history_idx > 0 {
                st.tabs[cur].history_idx -= 1;
                let url = st.tabs[cur].history[st.tabs[cur].history_idx as usize].clone();
                st.tabs[cur].url = url.clone();
                drop(st);
                load_url_in_content(content_webview, &url);
                return Some(update_state_json(&state.lock().unwrap()));
            }
            None
        }
        "forward" => {
            let cur = st.current;
            if st.tabs[cur].history_idx < (st.tabs[cur].history.len() as i32 - 1) {
                st.tabs[cur].history_idx += 1;
                let url = st.tabs[cur].history[st.tabs[cur].history_idx as usize].clone();
                st.tabs[cur].url = url.clone();
                drop(st);
                load_url_in_content(content_webview, &url);
                return Some(update_state_json(&state.lock().unwrap()));
            }
            None
        }
        "refresh" => {
            let url = st.tabs[st.current].url.clone();
            drop(st);
            load_url_in_content(content_webview, &url);
            None
        }
        "home" => {
            let cur = st.current;
            st.tabs[cur].url = HOMEPAGE.to_string();
            st.tabs[cur].title = "Новая вкладка".to_string();
            drop(st);
            load_url_in_content(content_webview, HOMEPAGE);
            Some(update_state_json(&state.lock().unwrap()))
        }
        "toggle_bookmark" => {
            let url = st.tabs[st.current].url.clone();
            let title = st.tabs[st.current].title.clone();
            if url == HOMEPAGE { return None; }
            if let Some(idx) = st.bookmarks.iter().position(|b| b.url == url) {
                st.bookmarks.remove(idx);
            } else {
                st.bookmarks.push(Bookmark {
                    url, title,
                    added: chrono_now(),
                });
            }
            save_json_file(&bookmarks_path(), &st.bookmarks);
            Some(update_state_json(&st))
        }
        "theme" => {
            st.theme = if st.theme == "dark" { "light".to_string() } else { "dark".to_string() };
            save_theme(&st.theme);
            Some(update_state_json(&st))
        }
        "history" => {
            // Показать историю в content
            let html = build_history_html(&st.history);
            drop(st);
            load_html_in_content(content_webview, &html);
            None
        }
        "bookmarks" => {
            let html = build_bookmarks_html(&st.bookmarks);
            drop(st);
            load_html_in_content(content_webview, &html);
            None
        }
        "clear" => {
            st.history.clear();
            save_json_file(&history_path(), &st.history);
            None
        }
        _ => Some(format!("Unknown cmd: {}", cmd)),
    };
    result
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("{}", secs)
}

fn build_history_html(history: &[HistoryEntry]) -> String {
    let items: String = history.iter().rev().take(100).map(|h| {
        format!(
            r#"<li><a href="{}"><strong>{}</strong><br><small>{} · {}</small></a></li>"#,
            html_escape(&h.url), html_escape(&h.title), html_escape(&h.url), html_escape(&h.time)
        )
    }).collect();
    format!(r#"<html><head><title>История</title><style>
    body {{ font-family: system-ui; padding: 40px; background: #fff; color: #000; margin: 0; }}
    h1 {{ color: #fc3f1d; margin-bottom: 20px; }}
    ul {{ list-style: none; padding: 0; }}
    a {{ color: #0066cc; text-decoration: none; display: block; padding: 12px 0; border-bottom: 1px solid #eee; }}
    a:hover {{ color: #fc3f1d; background: #fafafa; }}
    small {{ color: #666; }}
    </style></head><body><h1>История</h1><ul>{}</ul></body></html>"#, items)
}

fn build_bookmarks_html(bookmarks: &[Bookmark]) -> String {
    let items: String = bookmarks.iter().map(|b| {
        format!(
            r#"<li><a href="{}"><strong>{}</strong><br><small>{}</small></a></li>"#,
            html_escape(&b.url), html_escape(&b.title), html_escape(&b.url)
        )
    }).collect();
    format!(r#"<html><head><title>Закладки</title><style>
    body {{ font-family: system-ui; padding: 40px; background: #fff; color: #000; margin: 0; }}
    h1 {{ color: #fc3f1d; margin-bottom: 20px; }}
    ul {{ list-style: none; padding: 0; }}
    a {{ color: #0066cc; text-decoration: none; display: block; padding: 12px 0; border-bottom: 1px solid #eee; }}
    a:hover {{ color: #fc3f1d; background: #fafafa; }}
    small {{ color: #666; }}
    </style></head><body><h1>Закладки</h1><ul>{}</ul></body></html>"#, items)
}

fn update_state_json(state: &State) -> String {
    let cur = state.current;
    let address = state.tabs.get(cur).map(|t| t.url.clone()).unwrap_or_default();
    let bookmarked = state.bookmarks.iter().any(|b| b.url == address) && address != HOMEPAGE;
    let show_start = address == HOMEPAGE || address.is_empty();

    let state_json = serde_json::json!({
        "tabs": state.tabs.iter().map(|t| serde_json::json!({
            "title": t.title,
        })).collect::<Vec<_>>(),
        "current": cur,
        "theme": state.theme,
        "address": address,
        "bookmarked": bookmarked,
        "showStart": show_start,
    });
    format!("window.sierra_update({})", state_json)
}

// ═══════════════════════════════════════════════════════════════════════
//  НАВИГАЦИЯ В CONTENT WEBVIEW
// ═══════════════════════════════════════════════════════════════════════

fn load_url_in_content(content: &Arc<Mutex<Option<WebView>>>, url: &str) {
    println!("[Sierra] Loading URL in content: {}", url);
    if let Some(ref wv) = *content.lock().unwrap() {
        // Используем evaluate_script с window.location.href — это НАДЁЖНЕЕ,
        // чем load_url, потому что работает через JS engine, который
        // гарантированно инициализирован в WebView.
        let encoded_url = serde_json::to_string(url).unwrap_or_else(|_| "\"about:blank\"".to_string());
        let js = format!("window.location.href = {};", encoded_url);
        match wv.evaluate_script(&js) {
            Ok(_) => println!("[Sierra] navigate via JS OK"),
            Err(e) => {
                println!("[Sierra] evaluate_script error: {}, trying load_url", e);
                // Fallback на load_url
                match wv.load_url(url) {
                    Ok(_) => println!("[Sierra] load_url OK (fallback)"),
                    Err(e2) => println!("[Sierra] load_url error: {}", e2),
                }
            }
        }
    } else {
        println!("[Sierra] content webview is None!");
    }
}

fn load_html_in_content(content: &Arc<Mutex<Option<WebView>>>, html: &str) {
    if let Some(ref wv) = *content.lock().unwrap() {
        let _ = wv.load_html(html);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  TOCHKA VХODA
// ═══════════════════════════════════════════════════════════════════════

fn main() -> wry::Result<()> {
    let event_loop = EventLoop::new();

    // === Главное окно (chrome) — узкое, 140px высотой ===
    let chrome_window = WindowBuilder::new()
        .with_title("Sierra")
        .with_inner_size(LogicalSize::new(1200.0, 140.0))
        .with_min_inner_size(LogicalSize::new(600.0, 140.0))
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();
    let chrome_window_id = chrome_window.id();

    let state = Arc::new(Mutex::new(State::new()));
    let content_webview: Arc<Mutex<Option<WebView>>> = Arc::new(Mutex::new(None));
    let chrome_webview_ref: Arc<Mutex<Option<WebView>>> = Arc::new(Mutex::new(None));

    // Создаём chrome WebView с IPC handler
    let state_for_ipc = state.clone();
    let content_for_ipc = content_webview.clone();
    let chrome_for_ipc = chrome_webview_ref.clone();
    let chrome_html_str = {
        let st = state.lock().unwrap();
        chrome_html(&st)
    };
    let chrome_wv = WebViewBuilder::new()
        .with_html(chrome_html_str)
        .with_ipc_handler(move |request: wry::http::Request<String>| {
            let body = request.body();
            println!("[Sierra IPC] {}", body);
            if let Some(response) = handle_ipc(body, &state_for_ipc, &content_for_ipc) {
                if let Some(ref wv) = *chrome_for_ipc.lock().unwrap() {
                    let _ = wv.evaluate_script(&response);
                }
            }
        })
        .build(&chrome_window)?;
    *chrome_webview_ref.lock().unwrap() = Some(chrome_wv);

    // === Окно контента — под chrome, без декораций ===
    let content_window = WindowBuilder::new()
        .with_title("Sierra Content")
        .with_inner_size(LogicalSize::new(1200.0, 600.0))
        .with_decorations(false)
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();
    let content_window_id = content_window.id();

    // Позиционируем content под chrome
    if let Ok(chrome_pos) = chrome_window.outer_position() {
        let chrome_size = chrome_window.outer_size();
        let _ = content_window.set_outer_position(tao::dpi::PhysicalPosition::new(
            chrome_pos.x,
            chrome_pos.y + chrome_size.height as i32,
        ));
    }

    // Создаём content WebView (нативный, с URL)
    let content_wv = WebViewBuilder::new()
        .with_url(HOMEPAGE)
        .with_ipc_handler(move |_req: wry::http::Request<String>| {})
        .build(&content_window)?;
    *content_webview.lock().unwrap() = Some(content_wv);

    // Фокус на chrome_window (чтобы пользователь видел тулбар)
    chrome_window.set_focus();

    // Отправляем начальное состояние в chrome
    {
        let st = state.lock().unwrap();
        let update = update_state_json(&st);
        if let Some(ref wv) = *chrome_webview_ref.lock().unwrap() {
            let _ = wv.evaluate_script(&update);
        }
    }

    // Храним ID окон для обработки событий
    let chrome_id = chrome_window_id;
    let content_id = content_window_id;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                println!("[Sierra] Started");
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, window_id, .. } => {
                if window_id == chrome_id || window_id == content_id {
                    // Закрыли любое окно — выходим
                    chrome_window.set_visible(false);
                    content_window.set_visible(false);
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::WindowEvent { event: WindowEvent::Resized(_), window_id, .. } => {
                // При resize chrome — двигаем content
                if window_id == chrome_id {
                    if let Ok(chrome_pos) = chrome_window.outer_position() {
                        let chrome_size = chrome_window.outer_size();
                        let _ = content_window.set_outer_position(tao::dpi::PhysicalPosition::new(
                            chrome_pos.x,
                            chrome_pos.y + chrome_size.height as i32,
                        ));
                        let _ = content_window.set_inner_size(LogicalSize::new(
                            chrome_size.width as f64,
                            600.0,
                        ));
                    }
                }
            }
            Event::WindowEvent { event: WindowEvent::Moved(_), window_id, .. } => {
                // При движении chrome — двигаем content
                if window_id == chrome_id {
                    if let Ok(chrome_pos) = chrome_window.outer_position() {
                        let chrome_size = chrome_window.outer_size();
                        let _ = content_window.set_outer_position(tao::dpi::PhysicalPosition::new(
                            chrome_pos.x,
                            chrome_pos.y + chrome_size.height as i32,
                        ));
                    }
                }
            }
            _ => {}
        }
    });
}
