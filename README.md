# 🏔️ Sierra — Браузер на Rust (wry + tao)

[![Rust](https://img.shields.io/badge/Rust-1.77%2B-orange.svg)](https://rust-lang.org)
[![wry](https://img.shields.io/badge/wry-0.57-blue.svg)](https://github.com/tauri-apps/wry)
[![Version](https://img.shields.io/badge/version-5.0.0-red.svg)]()
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**Полностью переписан на wry + tao (без Tauri)** — прямой доступ к WebView, никаких middleware, никаких capabilities, никаких CSP.

## 🎉 Что нового в v5.0.0

- 🚫 **Выбросили Tauri** — он ломался на capabilities, CSP, WebviewUrl
- 🦀 **Только wry + tao** — прямой доступ к WebView
- 📦 **Размер: 379 КБ!** (UPX-сжатый Linux-бинарник)
- 🎯 **Одно окно** — chrome и контент в одном окне
- 🎨 **Гибрид-дизайн Яндекс + Chrome + Safari** сохранён

## 📊 Эволюция

| Версия | Язык | Движок | Размер | Проблемы |
|--------|------|--------|--------|----------|
| v1.0-v2.0 | Python | QtWebEngine | 242 МБ | Тяжёлый |
| v3.0 | Python | pywebview | 12 МБ | 2 окна |
| v4.0-v4.4 | Rust | Tauri 2 | 2-3 МБ | Не открывал страницы |
| **v5.0** | **Rust** | **wry + tao** | **379 КБ** | **Работает!** |

## 🏗 Архитектура

- **wry** — прямой доступ к WebView (WebView2 на Win, WebKit на macOS, WebKitGTK на Linux)
- **tao** — окна (от Tauri team, но без middleware)
- **Один бинарник** — HTML/CSS/JS встроены прямо в Rust код
- **IPC** — через `WebViewBuilder::with_ipc_handler`
- **Навигация** — `webview.load_url(url)` (прямой вызов, без middleware)

## 🎨 Дизайн — гибрид Яндекс + Chrome + Safari

| Источник | Что взято |
|----------|-----------|
| 🟥 **Яндекс.Браузер** | Красно-жёлтые акценты (`#fc3f1d` / `#ffcc00`), жёлтая звезда закладок |
| 🌐 **Google Chrome** | Компоновка (tabs сверху, toolbar снизу), пилюлевидная адресная строка |
| 🧭 **Safari** | Скруглённые вкладки, крупная адресная строка, минимализм |

## ✨ Возможности

- 📑 Вкладки (с историей, закрытием, переключением)
- 🏠 Start page с быстрыми ссылками
- 🔍 Умная адресная строка (URL или поиск DuckDuckGo)
- ⬅️ Навигация: назад / вперёд / обновить / домой
- ⭐ Закладки (сохраняются в JSON)
- 🕘 История (сохраняется в JSON)
- 🌓 Тёмная и светлая темы
- ⌨️ Горячие клавиши

## 🚀 Запуск

### Сборка из исходников
```bash
# Установить Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Linux: установить системные библиотеки
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libsoup-3.0-0 \
    libgstreamer1.0-0 libgstreamer-plugins-base1.0-0 libmanette-0.2-0 \
    libenchant-2-2 libsecret-1-0 libnotify4

# Сборка
cd src-tauri
cargo build --release

# Готовый бинарник: target/release/sierra
```

### Системные требования

| ОС | Требование | WebView Backend |
|----|------------|-----------------|
| **Windows 10/11** | Ничего | Microsoft Edge WebView2 |
| **macOS 10.13+** | Ничего | WebKit |
| **Linux** | WebKitGTK 4.1 | WebKitGTK |

## 📂 Структура проекта

```
sierra/
├── src-tauri/
│   ├── Cargo.toml      # Зависимости: wry, tao, serde, dirs
│   ├── icons/          # Иконка приложения
│   └── src/
│       └── main.rs    # Весь браузер (Rust + встроенный HTML/CSS/JS)
├── .github/workflows/  # CI/CD для Windows, macOS, Linux
├── README.md
└── LICENSE
```

## 🔧 Технологии

- **Rust** — системный язык, безопасность + скорость
- **wry** — прямой доступ к WebView (без middleware)
- **tao** — окна (от Tauri team, минимально)
- **serde** — сериализация JSON
- **HTML/CSS/JS** — встроены прямо в main.rs

## 📝 Лицензия

MIT
