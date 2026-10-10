# 🏔️ Sierra — Браузер на Rust (wry + tao)

[![Rust](https://img.shields.io/badge/Rust-1.77%2B-orange.svg)](https://rust-lang.org)
[![wry](https://img.shields.io/badge/wry-0.57-blue.svg)](https://github.com/tauri-apps/wry)
[![Version](https://img.shields.io/badge/version-5.2.0-red.svg)](https://github.com/codifystudio/sierra/releases)
[![Size](https://img.shields.io/badge/size-282%20KB-success.svg)]()
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**Браузер на Rust с прямым доступом к WebView (wry + tao, без Tauri)** — бинарник всего **282 КБ** для Windows!

## 🎉 Что нового в v5.2.0

- ✅ **Убрана красная debug-полоска** — чистый интерфейс
- ✅ **Исправлена бесконечная загрузка** — навигация через JS (`evaluate_script`) вместо ненадёжного `load_url`
- ✅ **Два окна** — chrome (сверху) + content (под ним, без декораций)
- ✅ **Синхронизация окон** — при движении/resize chrome → content следует

## 📊 Эволюция размеров

| Версия | Язык | Движок | Размер Windows | Проблемы |
|--------|------|--------|----------------|----------|
| v1.0-v2.0 | Python | QtWebEngine | 242 МБ | Тяжёлый |
| v3.0 | Python | pywebview | 12 МБ | 2 окна |
| v4.0-v4.4 | Rust | Tauri 2 | 2-3 МБ | Не открывал страницы |
| **v5.0-v5.2** | **Rust** | **wry + tao** | **282 КБ** | **Работает!** |

Уменьшение в **883 раза** по сравнению с первой версией!

## 📦 Downloads (v5.2.0)

| Платформа | Файл | Размер |
|-----------|------|--------|
| 🪟 **Windows x64** | `sierra-5.2.0-windows-x64.zip` | **282 КБ** |
| 🍎 **macOS x64** | `sierra-5.2.0-macos-x64.zip` | **384 КБ** |
| 🐧 **Linux x64** | `sierra-5.2.0-linux-x64.zip` | **380 КБ** |

👉 [Все релизы](https://github.com/codifystudio/sierra/releases)

## 🏗 Архитектура

- **wry** — прямой доступ к WebView (WebView2 на Win, WebKit на macOS, WebKitGTK на Linux)
- **tao** — окна (от Tauri team, но без middleware)
- **Два окна**:
  - `chrome_window` (1200x140) — вкладки, тулбар, адресная строка
  - `content_window` (1200x600, без декораций) — нативный WebView показывает сайт
- **IPC** — через `WebViewBuilder::with_ipc_handler`
- **Навигация** — `content_webview.evaluate_script("window.location.href = '...'")`

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

## ⚠️ Известные баги

Полный аудит выявил 38 багов (9 критичных). Главные:
- #6: Back/Forward сломаны после 2 переходов
- #4: История браузера не сохраняется
- #5: Title вкладки навсегда «Загрузка…»

См. [issue #1](https://github.com/codifystudio/sierra/issues/1) для деталей и плана исправления.

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
