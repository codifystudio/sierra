# Sierra

Веб-браузер, реализованный на Rust с использованием библиотек wry и tao. В качестве движка рендеринга страниц применяются системные WebView: WebView2 (Windows), WKWebView (macOS), WebKitGTK (Linux). Бинарный файл занимает менее 300 КБ благодаря прямому вызову системных API без промежуточных слоёв.

[![Rust](https://img.shields.io/badge/Rust-1.77%2B-orange.svg)](https://rust-lang.org)
[![wry](https://img.shields.io/badge/wry-0.57-blue.svg)](https://github.com/tauri-apps/wry)
[![Version](https://img.shields.io/badge/version-5.3.0-red.svg)](https://github.com/codifystudio/sierra/releases)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Содержание

- [Описание](#описание)
- [Архитектура](#архитектура)
- [Возможности](#возможности)
- [Системные требования](#системные-требования)
- [Установка из бинарных сборок](#установка-из-бинарных-сборок)
- [Сборка из исходников](#сборка-из-исходников)
- [Структура проекта](#структура-проекта)
- [IPC-протокол](#ipc-протокол)
- [Ограничения](#ограничения)
- [Лицензия](#лицензия)

## Описание

Sierra — это настольный веб-браузер, написанный полностью на Rust. В отличие от решений на базе Electron или Tauri, Sierra использует библиотеку wry, которая предоставляет прямой доступ к нативным WebView операционной системы без промежуточной среды выполнения и без дополнительных конфигурационных файлов.

### Сравнение с предыдущими реализациями

| Версия | Язык | Движок | Размер Windows .exe |
|--------|------|--------|---------------------|
| 1.0—2.0 | Python | QtWebEngine (Chromium) | 242 МБ |
| 3.0 | Python | pywebview (WebView2) | 12 МБ |
| 4.0—4.4 | Rust | Tauri 2 (WebView2) | 2—3 МБ |
| 5.0+ | Rust | wry + tao | 280 КБ |

## Архитектура

Sierra использует одно окно tao с одним WebView внутри. Веб-контент загружается во встроенный iframe, а интерфейс браузера (вкладки, адресная строка, кнопки навигации) реализован на HTML/CSS/JavaScript.

```
Окно tao (1200x800)
└─ WebView (wry)
   ├─ Панель вкладок + кнопка "+"
   ├─ Тулбар: back | forward | refresh | home | URL | bookmark | menu
   ├─ iframe (контент сайта, src = https://example.com)
   └─ Статус-бар
```

### Компоненты

- **wry** — обёртка над нативными WebView. Создаёт WebView с заданным HTML-контентом, обрабатывает IPC-сообщения.
- **tao** — управление окном, событийный цикл, обработка событий клавиатуры и мыши.
- **serde / serde_json** — сериализация состояния и IPC-сообщений.
- **dirs** — определение конфигурационных директорий ОС.

### Поток данных

1. Пользователь вводит URL в адресную строку и нажимает Enter.
2. JavaScript в chrome-WebView формирует IPC-сообщение `{cmd: "navigate", args: {url: "..."}}` и отправляет его через `window.ipc.postMessage()`.
3. Rust-обработчик `handle_ipc` принимает сообщение, парсит JSON, обновляет состояние (история вкладки, глобальная история, заголовок).
4. Rust возвращает JSON с обновлённым состоянием через `WebView::evaluate_script()`, вызывая `window.sierra_update(state)`.
5. JavaScript в chrome-WebView устанавливает `iframe.src = state.contentUrl`, что приводит к загрузке страницы.

## Возможности

- Вкладки: создание, переключение, закрытие, отдельная история для каждой
- Навигация: назад, вперёд, обновить, домашняя страница
- Адресная строка с автоматическим определением URL или поискового запроса (DuckDuckGo)
- Закладки: добавление, удаление, просмотр списка. Хранение в `~/.config/sierra/bookmarks.json`
- История посещений: хранение в `~/.config/sierra/history.json`, лимит 500 записей
- Стартовая страница с предустановленными быстрыми ссылками
- Тёмная и светлая темы оформления
- Горячие клавиши (см. раздел ниже)

### Горячие клавиши

| Сочетание | Действие |
|-----------|----------|
| Ctrl+T | Новая вкладка |
| Ctrl+W | Закрыть текущую вкладку |
| Ctrl+L | Фокус на адресную строку |
| Ctrl+D | Добавить или удалить закладку |
| Ctrl+H | Показать историю |
| Ctrl+Shift+T | Переключить тему оформления |
| Alt+← | Назад |
| Alt+→ | Вперёд |
| Alt+Home | Домашняя страница |
| F5 | Обновить |

## Системные требования

| ОС | Минимальная версия | Требуемые системные библиотеки |
|----|-------------------|-------------------------------|
| Windows | 10 (Build 17763) или 11 | WebView2 Runtime (предустановлен в Windows 10/11) |
| macOS | 10.13 High Sierra | Нет (WKWebView встроен) |
| Linux | Любой современный дистрибутив | WebKitGTK 4.1, GTK 3, libsoup 3.0, GStreamer 1.0 |

## Установка из бинарных сборок

Готовые сборки доступны на [странице релизов](https://github.com/codifystudio/sierra/releases).

### Windows

1. Скачайте `sierra-<version>-windows-x64.zip`.
2. Распакуйте архив в произвольную директорию.
3. Запустите `sierra.exe`.

Дополнительная установка не требуется, так как WebView2 входит в состав Windows 10/11.

### macOS

```bash
unzip sierra-<version>-macos-x64.zip
chmod +x sierra
./sierra
```

### Linux

Перед запуском установите системные зависимости:

```bash
# Debian, Ubuntu
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libsoup-3.0-0 \
    libgstreamer1.0-0 libgstreamer-plugins-base1.0-0 libmanette-0.2-0 \
    libenchant-2-2 libsecret-1-0 libnotify4

# Fedora
sudo dnf install webkit2gtk4.1 gtk3 libsoup3 gstreamer1-plugins-base \
    hyphen wpebackend-fdo libmanette enchant2 libsecret libnotify

# Arch Linux
sudo pacman -S webkit2gtk-4.1 gtk3 gstreamer gst-plugins-base-libs \
    hyphen wpebackend-fdo libmanette enchant libsecret libnotify
```

Затем:

```bash
unzip sierra-<version>-linux-x64.zip
chmod +x sierra-linux-x64
./sierra-linux-x64
```

## Сборка из исходников

### Предварительные требования

- Rust 1.77 или новее
- На Linux: системные библиотеки, перечисленные выше, плюс dev-пакеты (`-dev`)

### Шаги сборки

```bash
# Установка Rust (если не установлен)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Клонирование репозитория
git clone https://github.com/codifystudio/sierra.git
cd sierra

# Сборка
cd src-tauri
cargo build --release

# Готовый бинарник
./target/release/sierra  # Linux/macOS
./target/release/sierra.exe  # Windows
```

### Параметры оптимизации

В `Cargo.toml` включена агрессивная оптимизация для release-сборки:

```toml
[profile.release]
opt-level = "z"      # максимальная оптимизация по размеру
lto = true            # link-time optimization
codegen-units = 1     # одна единица компиляции
strip = true           # удаление отладочной информации
panic = "abort"       # прерывание при панике
```

Для дополнительного сжатия используется UPX:

```bash
upx --best --lzma target/release/sierra
```

## Структура проекта

```
sierra/
├── src-tauri/
│   ├── Cargo.toml                  # Зависимости: wry, tao, serde, dirs
│   ├── Cargo.lock                  # Зафиксированные версии зависимостей
│   ├── src/
│   │   └── main.rs                 # Весь код браузера (Rust + встроенный HTML/CSS/JS)
│   └── icons/
│       └── icon.png                # Иконка приложения
├── .github/
│   └── workflows/
│       ├── build-windows.yml       # CI-сборка для Windows
│       ├── build-macos.yml        # CI-сборка для macOS
│       ├── build-linux.yml        # CI-сборка для Linux
│       └── tests.yml              # Smoke-тесты
├── sierra-audit-report.md          # Отчёт аудита кода (38 багов)
├── README.md                       # Этот файл
└── LICENSE                         # MIT License
```

## IPC-протокол

Взаимодействие между JavaScript (chrome) и Rust реализовано через wry IPC. JavaScript отправляет сообщение через `window.ipc.postMessage()`, Rust принимает его в обработчике `with_ipc_handler`.

### Формат сообщения

```json
{
  "cmd": "navigate",
  "args": {
    "url": "https://example.com"
  }
}
```

### Команды

| Команда | Аргументы | Описание |
|---------|----------|----------|
| `init` | нет | Запрос начального состояния |
| `new_tab` | нет | Создать новую вкладку |
| `switch_tab` | `{idx: number}` | Переключиться на вкладку по индексу |
| `close_tab` | `{idx: number}` | Закрыть вкладку по индексу |
| `close_current` | нет | Закрыть текущую вкладку |
| `navigate` | `{url: string}` | Навигация по URL или поисковому запросу |
| `back` | нет | Переход назад по истории вкладки |
| `forward` | нет | Переход вперёд по истории вкладки |
| `refresh` | нет | Обновить текущую страницу |
| `home` | нет | Переход на домашнюю страницу |
| `toggle_bookmark` | нет | Добавить или удалить закладку |
| `theme` | нет | Переключить тему оформления |
| `history` | нет | Показать историю в iframe |
| `bookmarks` | нет | Показать закладки в iframe |
| `clear` | нет | Очистить историю посещений |

### Ответ Rust -> JS

Rust возвращает обновлённое состояние вызовом `window.sierra_update(state)`:

```json
{
  "tabs": [{"title": "Example Domain"}],
  "current": 0,
  "theme": "dark",
  "address": "https://example.com",
  "contentUrl": "https://example.com",
  "bookmarked": false,
  "showStart": false
}
```

## Хранение данных

Конфигурационные файлы хранятся в директории, определяемой библиотекой `dirs`:

| ОС | Путь |
|----|------|
| Windows | `%APPDATA%\sierra\` |
| macOS | `~/Library/Application Support/sierra/` |
| Linux | `~/.config/sierra/` |

Содержимое:

- `bookmarks.json` — список закладок
- `history.json` — история посещений (до 500 записей)
- `settings.json` — настройки (тема оформления)

## Ограничения

1. **iframe и X-Frame-Options.** Контент загружается во встроенный iframe. Сайты, отправляющие заголовок `X-Frame-Options: DENY` (например, Google, YouTube), не будут отображаться. Используйте сайты, допускающие встраивание: DuckDuckGo, Wikipedia, Reddit, Stack Overflow и др.

2. **Отсутствие обновления заголовка вкладки.** Заголовок вкладки устанавливается в "Загрузка…" и не обновляется после загрузки страницы. Это связано с ограничением same-origin политики для iframe.

3. **Отсутствие favicon.** Вкладки не отображают иконки сайтов.

4. **Одиночный процесс.** Все вкладки загружаются в одном iframe. Одновременное отображение нескольких сайтов не поддерживается.

5. **Отсутствие cookies и кэша.** Сессии не сохраняются между запусками.

Полный список известных проблем доступен в [отчёте аудита](sierra-audit-report.md) и в [issue #1](https://github.com/codifystudio/sierra/issues/1).

## Лицензия

MIT License. Полный текст — в файле [LICENSE](LICENSE).
