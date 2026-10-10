# 🔍 Глобальный аудит кода Sierra v5.2.0

**Репозиторий:** [codifystudio/sierra](https://github.com/codifystudio/sierra)
**Файл:** `src-tauri/src/main.rs` (852 строки)
**Версия:** v5.2.0
**Дата аудита:** 2026-10-10

---

## 📊 Сводка

| Категория | Количество | Критичных |
|-----------|------------|-----------|
| 🔴 Критические баги | 9 | 9 |
| 🟡 Серьёзные баги | 14 | — |
| 🟢 Мелкие баги | 15 | — |
| **Всего** | **38** | **9** |

---

## 🔴 КРИТИЧЕСКИЕ БАГИ (9)

### #1 — XSS-уязвимость в навигации через JS
**Файл:** `main.rs:698`
```rust
let js = format!("window.location.href = '{}';", url.replace('\'', "\\'"));
```
Если URL содержит `'; alert(document.cookie); var x = '`, выполнится произвольный JS в контексте content WebView. Экранирование только одинарных кавычек недостаточно — обратная косая черта и другие символы не обрабатываются.
**Фикс:** использовать `serde_json::to_string(url)` для генерации JS-строки.

### #2 — Panic при обрезке UTF-8 заголовков вкладок
**Файл:** `main.rs:168`
```rust
let truncated = if title.len() > 28 { format!("{}…", &title[..28]) } else { title };
```
`&title[..28]` — это срез по **байтам**, а не символам. Если title содержит русские буквы (2 байта каждая) или эмодзи (4 байта), срез попадёт внутрь многобайтного символа → **panic в debug, мусор в release**.
**Фикс:** `title.chars().take(28).collect::<String>()`

### #3 — Отсутствует CSP в chrome HTML
**Файл:** `main.rs:182` (формат HTML без `<meta http-equiv="Content-Security-Policy">`)
Любая XSS через title вкладки или bookmark title (через #1 или #2) выполнит JS в chrome-контексте и получит доступ к `window.ipc.postMessage()` → полный контроль над браузером.
**Фикс:** добавить CSP header: `default-src 'self'; script-src 'self'`

### #4 — История браузера НИКОГДА не сохраняется
**Файл:** `main.rs:526-547` (команда `navigate`)
Нет вызова `add_to_history()`. Константа `MAX_HISTORY` (строка 36) объявлена, но нигде не используется. Глобальная история (которую видит пользователь через "История") всегда пустая.
**Фикс:** добавить `st.history.push(...)` и `save_json_file(&history_path(), &st.history)` в `navigate`.

### #5 — Заголовок вкладки НИКОГДА не обновляется
**Файл:** `main.rs:541`
```rust
st.tabs[cur].title = "Загрузка…".to_string();
```
Устанавливается "Загрузка…" и больше НИКОГДА не меняется. Нет IPC-команды `update_title`, нет слушателя `document.title` в content WebView. Все вкладки навсегда остаются "Загрузка…".
**Фикс:** добавить в content WebView IPC handler, который слушает `document.title` изменения и шлёт `update_title` в chrome.

### #6 — Кнопки "Назад"/"Вперёд" сломаны после 2 переходов
**Файл:** `main.rs:548-557`
```rust
if st.tabs[cur].history_idx > 0 { ... }
```
Сценарий:
- Старт: `url=HOMEPAGE, history=[], history_idx=-1`
- Navigate `reddit`: `cur_url=HOMEPAGE` → НЕ добавляется в history (условие `cur_url != HOMEPAGE` ложно). `history=[], history_idx=-1, url=reddit`
- Navigate `google`: `cur_url=reddit` → добавляется. `history=[reddit], history_idx=0, url=google`
- Back: `history_idx > 0` → `0 > 0` → **false, не работает!**

**Фикс:** условие должно быть `history_idx >= 0` (но нужно сначала добавить HOMEPAGE в history при старте).

### #7 — Удаление закладок не сохраняется на диск
**Файл:** `main.rs:590-591`
```rust
if let Some(idx) = st.bookmarks.iter().position(|b| b.url == url) {
    st.bookmarks.remove(idx);
}
// save_json_file ОТСУТСТВУЕТ!
```
Удалённые закладки появляются снова после перезапуска. `save_json_file` вызывается только в ветке `else` (при добавлении).
**Фикс:** вызвать `save_json_file(&bookmarks_path(), &st.bookmarks)` после `remove`.

### #8 — `home` ломает историю вкладки
**Файл:** `main.rs:578-585`
```rust
"home" => {
    let cur = st.current;
    st.tabs[cur].url = HOMEPAGE.to_string();
    st.tabs[cur].title = "Новая вкладка".to_string();
    // НЕ сохраняет предыдущий URL в history!
}
```
После `home`, кнопка "Назад" не работает — предыдущий URL потерян.
**Фикс:** сохранить `cur_url` в `history` перед установкой HOMEPAGE.

### #9 — `panic = "abort"` + `Mutex::lock().unwrap()` = мгновенный краш
**Файл:** `Cargo.toml:15` + `main.rs:475`
```toml
[profile.release]
panic = "abort"
```
```rust
let mut st = state.lock().unwrap(); // panic если Mutex отравлен
```
Любой panic (от #2, от deadlock, от отравленного Mutex) мгновенно закрывает приложение без логов. Для браузера это очень плохая стратегия.
**Фикс:** `lock().unwrap_or_else(|e| e.into_inner())` + `panic = "unwind"`.

---

## 🟡 СЕРЬЁЗНЫЕ БАГИ (14)

### #10 — Опечатка в PowerShell workflow (build-windows.yml)
**Файл:** `.github/workflows/build-windows.yml:22`
```powershell
Write-Host "sierra.exe size: $(ath]::Round($size, 2)) MB"
```
`$(ath]` вместо `$([math]`. Это вызовет PowerShell-ошибку и шаг "Build .exe" упадёт.

### #11 — `bookmark_active` вычисляется но не используется
**Файл:** `main.rs:177-180`
```rust
let bookmark_active = { ... };
```
Вычисляется, но не передаётся в HTML. Звёздочка закладки не подсвечивается при рендере HTML (только через `sierra_update()`).

### #12 — Dead code: `CHROME_HEIGHT` и `MAX_HISTORY`
**Файл:** `main.rs:35-36`
```rust
const CHROME_HEIGHT: f64 = 140.0;  // не используется
const MAX_HISTORY: usize = 500;     // не используется
```
`CHROME_HEIGHT` должна использоваться в `WindowBuilder::with_inner_size` вместо магического `140.0`. `MAX_HISTORY` должна ограничивать длину `history`.

### #13 — Десинхронизация двух окон
**Файл:** `main.rs:807-851`
- При закрытии chrome через X — content остаётся на экране (виден)
- При минимизации chrome — content не минимизируется
- При движении chrome мышкой — content следует с задержкой (событие `Moved` приходит после движения)
- Если у chrome фокус, а content кликнуть — chrome может потерять sync

**Фикс:** слушать `Minimized`/`Focused` события и синхронизировать.

### #14 — `with_decorations(false)` + `with_resizable(true)` — противоречие
**Файл:** `main.rs:766-771`
```rust
let content_window = WindowBuilder::new()
    .with_decorations(false)
    .with_resizable(true)
```
Без рамки пользователь не может resize content. Но `set_inner_size` в `Resized` event всё равно перезаписывает размер — ручной resize бесполезен.

### #15 — Высота content фиксирована 600px
**Файл:** `main.rs:832`
```rust
chrome_size.width as f64, 600.0  // всегда 600
```
Не учитывается высота экрана. На 4K-мониторе content займёт треть экрана. На 13" ноуте — норм.

### #16 — `refresh` возвращает `None`, не обновляя состояние
**Файл:** `main.rs:572-577`
```rust
"refresh" => {
    let url = st.tabs[st.current].url.clone();
    drop(st);
    load_url_in_content(content_webview, &url);
    None  // <-- не обновляет UI
}
```
После refresh chrome не получает обновление состояния. Title остаётся "Загрузка…" навсегда.

### #17 — `load_html` ломает последующую навигацию
**Файл:** `main.rs:606-617`
```rust
"history" => {
    let html = build_history_html(&st.history);
    load_html_in_content(content_webview, &html);  // <-- заменяет страницу
}
```
После просмотра истории, кнопка "Назад" не работает — `back` команда читает `st.tabs[cur].url` (это URL сайта, не URL истории), но content уже показывает HTML истории, не сайт.

### #18 — `addr.select()` при focus — плохой UX
**Файл:** `main.rs:349`
```js
addr.addEventListener('focus', () => addr.select());
```
Если пользователь кликает в середине адресной строки для редактирования — выделяется весь текст. Должно быть только при `Ctrl+L` или `Tab`.

### #19 — `addr.value` перезаписывается при `sierra_update`
**Файл:** `main.rs:435`
```js
if (state.address !== undefined && document.activeElement !== addr) addr.value = state.address;
```
Если пользователь напечатал текст, затем кликнул вне адресной строки, и в этот момент приходит `sierra_update` — введённый текст теряется.

### #20 — `init` вызывается синхронно, `window.ipc` может не быть готов
**Файл:** `main.rs:448-449`
```js
console.log('Sierra init OK, IPC: ' + (window.ipc ? 'yes' : 'no'));
send('init');
```
В wry 0.57 `window.ipc` может ещё не быть注入ён в момент выполнения скрипта. Нужно ждать `DOMContentLoaded` или `window.ipc` через `setInterval`.

### #21 — `chrono_now()` возвращает Unix timestamp как строку
**Файл:** `main.rs:629-633`
```rust
fn chrono_now() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH)...;
    format!("{}", secs)  // "1791645600" — нечитаемо
}
```
Должно быть ISO 8601: `"2026-10-10T15:30:00Z"` для читаемой истории.

### #22 — `html_escape` не экранирует одинарные кавычки
**Файл:** `main.rs:455-457`
```rust
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
    // Нет замены ' → &#39;
}
```
Опасно если значение используется в JS-контексте (например, в `build_history_html`).

### #23 — Cargo.lock в `.gitignore` для binary-приложения
**Файл:** `.gitignore:16`
```
Cargo.lock
```
Для binary-приложений Cargo.lock **должен** быть закоммичен для воспроизводимости. Это рекомендация Cargo. Сейчас lock-файл случайно попал в репо (через `git add`), но `.gitignore` говорит игнорировать — противоречие.

---

## 🟢 МЕЛКИЕ БАГИ (15)

### #24 — Комментарий в начале main.rs устарел
**Файл:** `main.rs:1-9`
Описывает "ОДНО окно tao, ДВА WebView в нём", но на самом деле ДВА окна (v5.1+).

### #25 — README описывает v5.0.0, но в репо уже v5.2.0
README не обновлён.

### #26 — `eprintln!` в production коде
**Файл:** `main.rs:693, 699, 701, 703, 706, 707, 712`
```rust
println!("[Sierra] Loading URL in content: {}", url);
```
В release-сборке с `windows_subsystem = "windows"` stdout/stderr не видны. Логи бесполезны. Нужно использовать `log` crate + файловый логгер.

### #27 — `tabs` в `update_state_json` отправляет только `title`
**Файл:** `main.rs:676-678`
```rust
"tabs": state.tabs.iter().map(|t| serde_json::json!({
    "title": t.title,
})).collect::<Vec<_>>(),
```
Не отправляет `url` — невозможно показать favicon или индикатор загрузки.

### #28 — Нет favicon в вкладках
HTML вкладок не содержит `<img class="favicon">`. Все вкладки выглядят одинаково (только title).

### #29 — Нет индикатора загрузки в вкладках
Title = "Загрузка…" — единственный индикатор. Нет спиннера, нет прогресс-бара на вкладке.

### #30 — `update_state_json` отправляет ВСЁ состояние каждый раз
**Файл:** `main.rs:669-686`
Даже если изменился только title вкладки — отправляются все вкладки, тема, адрес, закладки. Неэффективно для большого числа вкладок.

### #31 — `State` хранит всю историю всех вкладок в памяти
Нет лимита на `tabs.len()` или `tab.history.len()`. После 100 вкладок с историей по 100 URL = 10000 строк. Не критично, но нет cleanup.

### #32 — Нет unit-тестов
В коде нет ни одного `#[test]`. `cargo test` ничего не проверяет.

### #33 — `tests.yml` проверяет только `cargo check` на Ubuntu
Нет матрицы тестов для Windows/macOS. Баги платформенных API не ловятся.

### #34 — UPX не применяется к Windows .exe
**Файл:** `.github/workflows/build-windows.yml`
Нет шага UPX-сжатия. Windows .exe = ~3 МБ вместо ~300 КБ.

### #35 — Нет обработки ошибок `load_html_in_content`
**Файл:** `main.rs:716-720`
```rust
fn load_html_in_content(content: &Arc<Mutex<Option<WebView>>>, html: &str) {
    if let Some(ref wv) = *content.lock().unwrap() {
        let _ = wv.load_html(html);  // <-- игнорирует ошибку
    }
}
```

### #36 — `WindowId` импортирован но не используется
**Файл:** `main.rs:24`
```rust
window::{WindowBuilder, WindowId},
```
`WindowId` не нужен — `window_id` получается через `window.id()`.

### #37 — `StartCause` импортирован но используется только для лога
**Файл:** `main.rs:22, 810-812`
```rust
Event::NewEvents(StartCause::Init) => {
    println!("[Sierra] Started");  // бесполезный лог
}
```

### #38 — `RequestAsyncResponder` и `WebViewExtWindows` не импортируются но упоминались в ранних версиях
В v5.0 был импорт `RequestAsyncResponder` — сейчас убран, но комментарий в строке 8 ещё упоминает `webview.eval()` (нужно `evaluate_script`).

---

## 🎯 ПРИОРИТЕТЫ ИСПРАВЛЕНИЯ

### Срочно (блокируют работу):
1. **#6** — back/forward сломаны (пользователь не может вернуться)
2. **#4** — история не сохраняется (функция не работает)
3. **#5** — title не обновляется (вкладки всегда "Загрузка…")
4. **#10** — PowerShell опечатка ломает Windows CI
5. **#1** — XSS в навигации (безопасность)

### Важно (портят UX):
6. **#13** — десинхронизация окон
7. **#16** — refresh не обновляет UI
8. **#17** — load_html ломает back
9. **#8** — home ломает историю
10. **#7** — удаление закладок не сохраняется

### Безопасность:
11. **#3** — CSP
12. **#2** — UTF-8 panic
13. **#22** — html_escape неполный
14. **#9** — panic=abort

### Качество кода:
14-38 — рефакторинг, тесты, документация

---

## 📋 РЕКОМЕНДАЦИИ

### Архитектурные:
- Разбить `main.rs` на модули: `state.rs`, `ipc.rs`, `html.rs`, `webview.rs`, `window.rs`
- Добавить `Result<T, E>` вместо `unwrap()` везде
- Логирование через `log` + `env_logger` или файловый логгер
- Тесты: unit для `resolve_input`, `html_escape`, `update_state_json`

### Безопасность:
- CSP header в chrome HTML
- Полный HTML escape (включая `'`)
- JS-навигация через `JSON.stringify` вместо format!
- Sanitize URL (запретить `javascript:` scheme)

### UX:
- Слушать `document.title` в content WebView → `update_title` IPC
- Favicon во вкладках
- Спиннер загрузки во вкладке
- Синхронизация минимизации/фокуса окон
- Высота content = высота экрана - высота chrome

### CI/CD:
- Исправить PowerShell опечатку
- UPX для Windows .exe
- Матрица тестов (Ubuntu, Windows, macOS)
- Закоммитить Cargo.lock

---

**Аудит завершён.** Найдено 38 багов: 9 критичных, 14 серьёзных, 15 мелких. Рекомендую начать с исправления критичных багов (#1-#9), особенно #6 (back/forward сломаны) и #4 (история не работает) — это блокирует базовый UX браузера.
