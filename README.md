# 🏔️ Sierra — Лёгкий браузер на Rust

[![Rust](https://img.shields.io/badge/Rust-1.77%2B-orange.svg)](https://rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.x%2B-blue.svg)](https://tauri.app)
[![Version](https://img.shields.io/badge/version-4.0.0-red.svg)]()
[![Size](https://img.shields.io/badge/binary-2%20MB-success.svg)]()
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**Самый лёгкий браузер в истории** — бинарник весит всего **2 МБ** (с UPX-сжатием)!

Написан полностью на **Rust** с использованием **Tauri 2** и системного WebView (Edge WebView2 на Windows, WebKit на macOS, WebKitGTK на Linux).

## 🎉 Что нового в v4.0.0

- 🦀 **Полностью переписан на Rust** (был Python + PySide6/pywebview)
- 🏔️ **Новое имя: Sierra** (был vBrowser)
- 📦 **Размер: 2 МБ** (вместо 12 МБ у v3.0 и 242 МБ у v2.0)
- 🎯 **Одно окно** — больше нет двух окон, контент в iframe прямо в интерфейсе
- 🌐 **Tauri 2 + WebView2/WebKit** — нативный системный движок

## 📊 Сравнение версий

| Версия | Язык | Движок | Размер | Запуск |
|--------|------|--------|--------|--------|
| v1.0-v2.0 | Python | QtWebEngine (Chromium) | 242 МБ | 3-5 сек |
| v3.0 | Python | pywebview (Edge WebView2) | 12 МБ | мгновенно |
| **v4.0** | **Rust** | **Tauri 2 + WebView2** | **2 МБ** | **мгновенно** |

## 🎨 Дизайн — гибрид Яндекс + Chrome + Safari

| Источник | Что взято |
|----------|-----------|
| 🟥 **Яндекс.Браузер** | Красно-жёлтые акценты (`#fc3f1d` / `#ffcc00`), жёлтая звезда закладок, красная полоса в статус-баре |
| 🌐 **Google Chrome** | Компоновка (tabs сверху, toolbar снизу), пилюлевидная адресная строка, кнопка "+", материальная палитра |
| 🧭 **Safari** | Скруглённые отделённые вкладки (10px), крупная адресная строка с тонкой рамкой, минималистичная серая палитра |

## ✨ Возможности

| Возможность | Описание |
|---|---|
| 📑 Вкладки | Скруглённые, отделённые (Safari-style), с возможностью закрытия |
| 🏠 Start page | Скоростная страница с логотипом, поиском и быстрыми ссылками |
| 🔍 Умная адресная строка | Автоопределение URL или поискового запроса (DuckDuckGo) |
| ⬅️ Навигация | Назад / Вперёд / Обновить / Домой |
| ⭐ Закладки | Сохраняются в `~/.config/sierra/bookmarks.json` |
| 🕘 История | Сохраняется в `~/.config/sierra/history.json` (до 500 записей) |
| 🌓 Темы | Тёмная и светлая (переключение: `Ctrl+Shift+T`) |
| ⌨️ Горячие клавиши | Полный набор |
| 🔍 Масштаб | `Ctrl+` / `Ctrl-` / `Ctrl+0` |

## 🚀 Запуск

### Вариант 1: Скачать готовый бинарник
1. Скачайте `sierra-4.0.0-linux-x64.zip` из [Releases](https://github.com/vefyz/sierra/releases)
2. Распакуйте и запустите: `./sierra`

### Вариант 2: Собрать из исходников
```bash
# Установить Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Системные зависимости (Linux)
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libjavascriptcoregtk-4.1-0 \
    libsoup-3.0-0 libgstreamer1.0-0 libgstreamer-plugins-base1.0-0 \
    libharfbuzz-icu0 libhyphen0 libwpe-1.0-1 libwpebackend-fdo-1.0-1 \
    libmanette-0.2-0 libenchant-2-2 libsecret-1-0 libnotify4

# Сборка
cd src-tauri
cargo build --release

# Готовый бинарник: target/release/sierra
```

### Системные требования

| ОС | Требование | Backend |
|----|------------|---------|
| **Windows 10/11** | Ничего (WebView2 встроён) | Microsoft Edge WebView2 |
| **macOS 10.13+** | Ничего (WebKit встроён) | WebKit |
| **Linux** | WebKitGTK 4.1 (через apt) | WebKitGTK |

#### Linux: установка системных библиотек
```bash
# Debian/Ubuntu
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libjavascriptcoregtk-4.1-0 \
    libsoup-3.0-0 libgstreamer1.0-0 libgstreamer-plugins-base1.0-0 \
    libharfbuzz-icu0 libhyphen0 libwpe-1.0-1 libwpebackend-fdo-1.0-1 \
    libmanette-0.2-0 libenchant-2-2 libsecret-1-0 libnotify4

# Fedora
sudo dnf install webkit2gtk4.1 gtk3 javascriptcoregtk4.1 libsoup3 \
    gstreamer1-plugins-base hyphen wpebackend-fdo libmanette enchant2 \
    libsecret libnotify

# Arch
sudo pacman -S webkit2gtk-4.1 gtk3 gstreamer gst-plugins-base-libs \
    hyphen wpebackend-fdo libmanette enchant libsecret libnotify
```

## ⌨️ Горячие клавиши

| Сочетание | Действие |
|---|---|
| `Ctrl+T` | Новая вкладка |
| `Ctrl+W` | Закрыть вкладку |
| `Ctrl+L` | Фокус на адресную строку |
| `Ctrl+D` | Добавить/убрать закладку |
| `Ctrl+H` | История |
| `Ctrl++` | Увеличить масштаб |
| `Ctrl+-` | Уменьшить масштаб |
| `Ctrl+0` | Сбросить масштаб |
| `Ctrl+Shift+T` | Сменить тему |
| `Alt+←` | Назад |
| `Alt+→` | Вперёд |
| `Alt+Home` | Домой |
| `F5` | Обновить |
| `Esc` | Закрыть меню |

## 📂 Структура проекта

```
sierra/
├── src-tauri/          # Rust backend (Tauri 2)
│   ├── Cargo.toml      # Зависимости Rust
│   ├── build.rs        # Build script
│   ├── tauri.conf.json # Конфигурация Tauri
│   ├── icons/          # Иконки
│   └── src/
│       └── main.rs     # Главный файл (Rust)
├── src/                # Frontend (HTML/CSS/JS)
│   ├── index.html      # Структура UI
│   ├── styles.css      # Стили (гибрид Яндекс + Chrome + Safari)
│   ├── app.js          # Логика интерфейса
│   └── icon.png        # Фавикон
└── README.md
```

## 🏗 Архитектура

- **Rust** (`src-tauri/src/main.rs`) — backend: управление вкладками, закладками, историей, настройками. Предоставляет Tauri-команды для JS.
- **HTML/CSS/JS** (`src/`) — frontend: интерфейс браузера в одном окне. Контент сайтов показывается в `<iframe>` прямо в этом же окне — **никаких отдельных окон!**
- **Tauri 2** — связь между Rust и JS через `__TAURI__.invoke()`.

## 🎨 Палитра цветов

| Цвет | HEX | Где используется |
|------|-----|-----------------|
| 🔴 Yandex Red | `#fc3f1d` | Акцент: hover, фокус адресной строки, прогресс-бар |
| 🟡 Yandex Yellow | `#ffcc00` | Иконка закладки (когда в закладках) |
| ⚫ Dark BG | `#1c1c1e` | Фон окна в тёмной теме |
| 🌫️ Tab strip | `#2a2a2d` | Фон вкладок в тёмной теме |
| 🤍 Light BG | `#f5f5f7` | Фон окна в светлой теме (Safari-style) |

## 🔧 Технологии

- **Rust** — системный язык, быстрый и безопасный
- **Tauri 2** — фреймворк для desktop-приложений
- **WebView2** (Windows), **WebKit** (macOS), **WebKitGTK** (Linux) — нативные WebView
- **HTML/CSS/JS** — для интерфейса (без тяжёлых фреймворков)
- **serde** — сериализация JSON
- **chrono** — работа с временем для истории

## 📝 Лицензия

MIT — свободно для использования и модификации.
