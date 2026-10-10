// Sierra Browser — JavaScript логика

const { invoke } = window.__TAURI__.tauri;

// ═══════════════════════════════════════════════════════════════════════
//  STATE
// ═══════════════════════════════════════════════════════════════════════

let tabs = [];
let current = 0;
let theme = 'dark';
let bookmarks = [];

// Элементы DOM
const tabsContainer = document.getElementById('tabsContainer');
const addressBar = document.getElementById('addressBar');
const startPage = document.getElementById('startPage');
const startSearch = document.getElementById('startSearch');
const webview = document.getElementById('webview');
const progress = document.getElementById('progress');
const progressFill = document.getElementById('progressFill');
const statusText = document.getElementById('statusText');
const menuDropdown = document.getElementById('menuDropdown');
const btnBookmark = document.getElementById('btnBookmark');
const btnBack = document.getElementById('btnBack');
const btnForward = document.getElementById('btnForward');

// ═══════════════════════════════════════════════════════════════════════
//  INIT
// ═══════════════════════════════════════════════════════════════════════

async function init() {
    // Загружаем тему
    try {
        theme = await invoke('get_theme');
        document.body.className = theme;
    } catch (e) {
        console.error('get_theme failed:', e);
    }
    // Загружаем закладки
    try {
        bookmarks = await invoke('get_bookmarks');
    } catch (e) {
        console.error('get_bookmarks failed:', e);
    }
    // Создаём стартовую вкладку
    await newTab();
}

// ═══════════════════════════════════════════════════════════════════════
//  ВКЛАДКИ
// ═══════════════════════════════════════════════════════════════════════

async function newTab() {
    try {
        const tab = await invoke('new_tab', { url: null });
        await refreshTabs();
        await showStartPage();
        addressBar.value = '';
        startSearch.focus();
    } catch (e) {
        console.error('newTab error:', e);
    }
}

async function switchTab(idx) {
    try {
        const tab = await invoke('switch_tab', { idx });
        if (tab) {
            await refreshTabs();
            await loadTabContent(tab);
        }
    } catch (e) {
        console.error('switchTab error:', e);
    }
}

async function closeTab(event, idx) {
    if (event) event.stopPropagation();
    try {
        const ok = await invoke('close_tab', { idx });
        if (ok) {
            await refreshTabs();
            const [t, c] = await invoke('get_tabs');
            if (t.length === 0) {
                // Последняя вкладка закрыта — создадим новую
                await newTab();
            } else {
                await loadTabContent(t[c]);
            }
        }
    } catch (e) {
        console.error('closeTab error:', e);
    }
}

async function refreshTabs() {
    try {
        const [t, c] = await invoke('get_tabs');
        tabs = t;
        current = c;
        renderTabs();
    } catch (e) {
        console.error('refreshTabs error:', e);
    }
}

function renderTabs() {
    tabsContainer.innerHTML = '';
    for (let i = 0; i < tabs.length; i++) {
        const tab = tabs[i];
        const isActive = i === current;
        const el = document.createElement('div');
        el.className = `tab ${isActive ? 'active' : ''}`;
        el.onclick = () => switchTab(i);
        const title = tab.title || 'Без названия';
        const truncated = title.length > 28 ? title.substring(0, 28) + '…' : title;
        el.innerHTML = `<span class="tab-title">${escapeHtml(truncated)}</span><span class="tab-close" onclick="closeTab(event, ${i})">×</span>`;
        tabsContainer.appendChild(el);
    }
}

function escapeHtml(s) {
    return s.replace(/[&<>"']/g, m => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[m]);
}

async function loadTabContent(tab) {
    if (!tab) return;
    if (tab.url === 'https://duckduckgo.com' || tab.url === '' || tab.url === HOMEPAGE_PAGE()) {
        await showStartPage();
        addressBar.value = '';
    } else {
        await showWebview(tab.url);
        addressBar.value = tab.url;
    }
    updateBookmarkButton();
    updateNavButtons();
}

function HOMEPAGE_PAGE() {
    return 'https://duckduckgo.com';
}

// ═══════════════════════════════════════════════════════════════════════
//  КОНТЕНТ — переключение между start page и webview
// ═══════════════════════════════════════════════════════════════════════

async function showStartPage() {
    startPage.style.display = 'flex';
    webview.style.display = 'none';
    progress.style.display = 'none';
    statusText.textContent = 'Готово';
    startSearch.focus();
}

async function showWebview(url) {
    startPage.style.display = 'none';
    webview.style.display = 'block';
    progress.style.display = 'block';
    progressFill.style.width = '30%';

    // Загружаем URL в iframe
    webview.src = url;

    // Прогресс-симуляция (так как iframe sandbox ограничивает события)
    setTimeout(() => { progressFill.style.width = '60%'; }, 200);
    setTimeout(() => {
        progressFill.style.width = '100%';
        setTimeout(() => { progress.style.display = 'none'; }, 300);
    }, 800);

    statusText.textContent = url;

    // Получаем title (это работает в большинстве случаев)
    try {
        // Из-за политики same-origin мы не можем напрямую прочитать title
        // Но можем использовать title по умолчанию
        const title = url.replace(/^https?:\/\//, '').split('/')[0];
        await invoke('update_tab_title', { title });
        await invoke('add_to_history', { url, title });
        await refreshTabs();
    } catch (e) {
        console.error('update_tab_title error:', e);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  НАВИГАЦИЯ
// ═══════════════════════════════════════════════════════════════════════

async function navigate(url) {
    if (!url || !url.trim()) return;
    try {
        const tab = await invoke('navigate', { url });
        if (tab) {
            await refreshTabs();
            await showWebview(tab.url);
            addressBar.value = tab.url;
            addressBar.blur();
        }
    } catch (e) {
        console.error('navigate error:', e);
    }
}

function onAddressKey(event) {
    if (event.key === 'Enter') {
        const val = addressBar.value.trim();
        if (val) navigate(val);
    } else if (event.key === 'Escape') {
        addressBar.blur();
    }
}

function onStartSearchKey(event) {
    if (event.key === 'Enter') {
        const val = startSearch.value.trim();
        if (val) navigate(val);
    }
}

let prevAddress = '';
function onAddressFocus() {
    prevAddress = addressBar.value;
    addressBar.select();
}
function onAddressBlur() {
    // Если ничего не ввели — возвращаем URL
    if (!addressBar.value && tabs[current]) {
        addressBar.value = tabs[current].url;
    }
}

async function goBack() {
    try {
        const tab = await invoke('go_back');
        if (tab) {
            await refreshTabs();
            await showWebview(tab.url);
            addressBar.value = tab.url;
        }
    } catch (e) { console.error('goBack:', e); }
}

async function goForward() {
    try {
        const tab = await invoke('go_forward');
        if (tab) {
            await refreshTabs();
            await showWebview(tab.url);
            addressBar.value = tab.url;
        }
    } catch (e) { console.error('goForward:', e); }
}

async function refresh() {
    if (webview.style.display !== 'none' && webview.src) {
        // Перезагружаем iframe
        const src = webview.src;
        webview.src = 'about:blank';
        setTimeout(() => { webview.src = src; }, 50);
    }
}

async function goHome() {
    await navigate('https://duckduckgo.com');
}

// ═══════════════════════════════════════════════════════════════════════
//  ЗАКЛАДКИ
// ═══════════════════════════════════════════════════════════════════════

async function toggleBookmark() {
    if (!tabs[current]) return;
    const url = tabs[current].url;
    if (!url || url === 'https://duckduckgo.com') return;
    try {
        const title = tabs[current].title || url;
        const isBookmarked = await invoke('toggle_bookmark', { url, title });
        bookmarks = await invoke('get_bookmarks');
        updateBookmarkButton();
        statusText.textContent = isBookmarked ? 'Добавлено в закладки' : 'Удалено из закладок';
    } catch (e) {
        console.error('toggleBookmark:', e);
    }
}

function updateBookmarkButton() {
    if (!tabs[current]) return;
    const url = tabs[current].url;
    const isBookmarked = bookmarks.some(b => b.url === url);
    if (isBookmarked) {
        btnBookmark.classList.add('active');
    } else {
        btnBookmark.classList.remove('active');
    }
}

async function showBookmarks() {
    try {
        bookmarks = await invoke('get_bookmarks');
        if (bookmarks.length === 0) {
            statusText.textContent = 'Закладок нет';
            return;
        }
        // Открываем новую вкладку со списком закладок
        await newTab();
        const html = `<html><head><title>Закладки</title><style>
            body{font-family:system-ui;padding:40px;background:#fff;color:#000}
            h1{margin-bottom:20px;color:#fc3f1d}
            a{color:#0066cc;text-decoration:none;display:block;padding:8px 0;border-bottom:1px solid #eee}
            a:hover{color:#fc3f1d}
        </style></head><body><h1>Закладки</h1>${bookmarks.map(b =>
            `<a href="${b.url}"><strong>${escapeHtml(b.title)}</strong><br><small>${escapeHtml(b.url)}</small></a>`
        ).join('')}</body></html>`;
        showWebviewWithHtml(html);
    } catch (e) {
        console.error('showBookmarks:', e);
    }
    hideMenu();
}

async function showHistory() {
    try {
        const history = await invoke('get_history');
        if (history.length === 0) {
            statusText.textContent = 'История пуста';
            return;
        }
        await newTab();
        const html = `<html><head><title>История</title><style>
            body{font-family:system-ui;padding:40px;background:#fff;color:#000}
            h1{margin-bottom:20px;color:#fc3f1d}
            a{color:#0066cc;text-decoration:none;display:block;padding:8px 0;border-bottom:1px solid #eee}
            a:hover{color:#fc3f1d}
            small{color:#666}
        </style></head><body><h1>История</h1>${history.slice(-100).reverse().map(h =>
            `<a href="${h.url}"><strong>${escapeHtml(h.title)}</strong><br><small>${escapeHtml(h.url)} · ${h.time.substring(0, 19)}</small></a>`
        ).join('')}</body></html>`;
        showWebviewWithHtml(html);
    } catch (e) {
        console.error('showHistory:', e);
    }
    hideMenu();
}

async function showWebviewWithHtml(html) {
    startPage.style.display = 'none';
    webview.style.display = 'block';
    webview.srcdoc = html;
    progress.style.display = 'none';
}

// ═══════════════════════════════════════════════════════════════════════
//  МАСШТАБ
// ═══════════════════════════════════════════════════════════════════════

let zoom = 1;
async function zoomIn() {
    zoom = Math.min(3, zoom + 0.1);
    webview.style.zoom = zoom;
    hideMenu();
}
async function zoomOut() {
    zoom = Math.max(0.3, zoom - 0.1);
    webview.style.zoom = zoom;
    hideMenu();
}
async function zoomReset() {
    zoom = 1;
    webview.style.zoom = 1;
    hideMenu();
}

// ═══════════════════════════════════════════════════════════════════════
//  ТЕМА
// ═══════════════════════════════════════════════════════════════════════

async function toggleTheme() {
    try {
        theme = await invoke('toggle_theme');
        document.body.className = theme;
    } catch (e) { console.error('toggleTheme:', e); }
    hideMenu();
}

async function clearHistory() {
    try {
        await invoke('clear_history');
        statusText.textContent = 'История очищена';
    } catch (e) { console.error('clearHistory:', e); }
    hideMenu();
}

// ═══════════════════════════════════════════════════════════════════════
//  МЕНЮ
// ═══════════════════════════════════════════════════════════════════════

function toggleMenu(event) {
    if (event) event.stopPropagation();
    menuDropdown.classList.toggle('show');
}

function hideMenu() {
    menuDropdown.classList.remove('show');
}

// ═══════════════════════════════════════════════════════════════════════
//  Кнопки навигации
// ═══════════════════════════════════════════════════════════════════════

function updateNavButtons() {
    if (!tabs[current]) return;
    const tab = tabs[current];
    btnBack.disabled = tab.history_idx <= 0;
    btnForward.disabled = tab.history_idx >= tab.history.length - 1;
}

// ═══════════════════════════════════════════════════════════════════════
//  ГОРЯЧИЕ КЛАВИШИ
// ═══════════════════════════════════════════════════════════════════════

document.addEventListener('keydown', (e) => {
    const ctrl = e.ctrlKey || e.metaKey;
    if (ctrl && e.key === 't') { e.preventDefault(); newTab(); }
    else if (ctrl && e.key === 'w') { e.preventDefault(); closeTab(null, current); }
    else if (ctrl && e.key === 'l') { e.preventDefault(); addressBar.focus(); addressBar.select(); }
    else if (ctrl && e.key === 'd') { e.preventDefault(); toggleBookmark(); }
    else if (ctrl && e.key === 'h') { e.preventDefault(); showHistory(); }
    else if (ctrl && e.shiftKey && (e.key === 'T' || e.key === 't')) { e.preventDefault(); toggleTheme(); }
    else if (ctrl && e.key === '+') { e.preventDefault(); zoomIn(); }
    else if (ctrl && e.key === '-') { e.preventDefault(); zoomOut(); }
    else if (ctrl && e.key === '0') { e.preventDefault(); zoomReset(); }
    else if (e.altKey && e.key === 'ArrowLeft') { e.preventDefault(); goBack(); }
    else if (e.altKey && e.key === 'ArrowRight') { e.preventDefault(); goForward(); }
    else if (e.altKey && e.key === 'Home') { e.preventDefault(); goHome(); }
    else if (e.key === 'F5') { e.preventDefault(); refresh(); }
    else if (e.key === 'Escape') { hideMenu(); }
});

// Закрытие меню при клике вне его
document.addEventListener('click', (e) => {
    if (!e.target.closest('#menuDropdown') && !e.target.closest('#btnMenu')) {
        hideMenu();
    }
});

// Запуск
init();
