// Sierra Browser — JavaScript логика
// Главное: ждём пока Tauri будет готов, потом инициализируем

let invoke = null;
let isReady = false;

// State
let tabs = [];
let current = 0;
let theme = 'dark';
let bookmarks = [];

// DOM
let tabsContainer, addressBar, startPage, startSearch;
let progress, progressFill, statusText, menuDropdown;
let btnBookmark, btnBack, btnForward;

// ═══════════════════════════════════════════════════════════════════════
//  ОЖИДАНИЕ TAURI
// ═══════════════════════════════════════════════════════════════════════

function waitForTauri() {
    return new Promise((resolve) => {
        const check = () => {
            if (window.__TAURI__ && (window.__TAURI__.invoke || (window.__TAURI__.core && window.__TAURI__.core.invoke))) {
                invoke = window.__TAURI__.invoke || window.__TAURI__.core.invoke;
                resolve();
            } else {
                setTimeout(check, 30);
            }
        };
        check();
    });
}

// ═══════════════════════════════════════════════════════════════════════
//  INIT
// ═══════════════════════════════════════════════════════════════════════

async function init() {
    // Получаем DOM элементы
    tabsContainer = document.getElementById('tabsContainer');
    addressBar = document.getElementById('addressBar');
    startPage = document.getElementById('startPage');
    startSearch = document.getElementById('startSearch');
    progress = document.getElementById('progress');
    progressFill = document.getElementById('progressFill');
    statusText = document.getElementById('statusText');
    menuDropdown = document.getElementById('menuDropdown');
    btnBookmark = document.getElementById('btnBookmark');
    btnBack = document.getElementById('btnBack');
    btnForward = document.getElementById('btnForward');

    // Ждём Tauri
    await waitForTauri();
    isReady = true;
    console.log('Tauri bridge ready');
    statusText.textContent = 'Готово';

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
    if (!isReady) { console.warn('Tauri not ready yet'); return; }
    try {
        await invoke('new_tab', { url: null });
        await refreshTabs();
        await showStartPage();
        addressBar.value = '';
        startSearch.focus();
    } catch (e) {
        console.error('newTab error:', e);
        statusText.textContent = 'Ошибка: ' + e;
    }
}

async function switchTab(idx) {
    if (!isReady) return;
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
    if (!isReady) return;
    try {
        const ok = await invoke('close_tab', { idx });
        if (ok) {
            await refreshTabs();
            const [t, c] = await invoke('get_tabs');
            if (t.length === 0) {
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
        updateNavButtons();
    } catch (e) {
        console.error('refreshTabs error:', e);
    }
}

function renderTabs() {
    if (!tabsContainer) return;
    tabsContainer.innerHTML = '';
    for (let i = 0; i < tabs.length; i++) {
        const tab = tabs[i];
        const isActive = i === current;
        const el = document.createElement('div');
        el.className = 'tab' + (isActive ? ' active' : '');
        el.onclick = () => switchTab(i);
        const title = tab.title || 'Без названия';
        const truncated = title.length > 28 ? title.substring(0, 28) + '…' : title;
        const safeTitle = escapeHtml(truncated);
        el.innerHTML = `<span class="tab-title">${safeTitle}</span><span class="tab-close" onclick="event.stopPropagation(); closeTab(null, ${i})">×</span>`;
        tabsContainer.appendChild(el);
    }
}

function escapeHtml(s) {
    return String(s).replace(/[&<>"']/g, m => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[m]);
}

async function loadTabContent(tab) {
    if (!tab) return;
    if (tab.url === 'https://duckduckgo.com' || tab.url === '' || tab.url === 'https://duckduckgo.com/') {
        await showStartPage();
        addressBar.value = '';
    } else {
        await showWebview(tab.url);
        addressBar.value = tab.url;
    }
    updateBookmarkButton();
}

// ═══════════════════════════════════════════════════════════════════════
//  КОНТЕНТ — переключение между start page и нативным WebView окном
// ═══════════════════════════════════════════════════════════════════════

async function showStartPage() {
    if (startPage) startPage.style.display = 'flex';
    if (progress) progress.style.display = 'none';
    statusText.textContent = 'Готово';
    // Скрываем окно контента
    try {
        await invoke('hide_content');
    } catch (e) {
        console.error('hide_content:', e);
    }
    if (startSearch) startSearch.focus();
}

async function showWebview(url) {
    if (startPage) startPage.style.display = 'none';
    if (progress) progress.style.display = 'block';
    if (progressFill) progressFill.style.width = '30%';

    statusText.textContent = 'Загрузка: ' + url;

    // Открываем URL в нативном Tauri WebviewWindow
    try {
        await invoke('open_content', { url });
        // Прогресс-симуляция
        setTimeout(() => { if (progressFill) progressFill.style.width = '70%'; }, 200);
        setTimeout(() => {
            if (progressFill) progressFill.style.width = '100%';
            setTimeout(() => { if (progress) progress.style.display = 'none'; }, 300);
        }, 800);

        // Обновляем title (используем URL как заголовок, так как Tauri webview
        // не даёт напрямую получить title из другого окна)
        const title = url.replace(/^https?:\/\//, '').split('/')[0];
        await invoke('update_tab_title', { title });
        await invoke('add_to_history', { url, title });
        await refreshTabs();
    } catch (e) {
        console.error('open_content error:', e);
        statusText.textContent = 'Ошибка загрузки: ' + e;
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  НАВИГАЦИЯ
// ═══════════════════════════════════════════════════════════════════════

async function navigate(url) {
    if (!url || !url.trim() || !isReady) return;
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
        statusText.textContent = 'Ошибка: ' + e;
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
    if (!addressBar.value && tabs[current]) {
        addressBar.value = tabs[current].url;
    }
}

async function goBack() {
    if (!isReady) return;
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
    if (!isReady) return;
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
    if (!isReady) return;
    if (tabs[current] && tabs[current].url && tabs[current].url !== 'https://duckduckgo.com') {
        // Перезагружаем, открывая URL заново
        await showWebview(tabs[current].url);
    } else {
        await showStartPage();
    }
}

async function goHome() {
    await navigate('https://duckduckgo.com');
}

// ═══════════════════════════════════════════════════════════════════════
//  ЗАКЛАДКИ
// ═══════════════════════════════════════════════════════════════════════

async function toggleBookmark() {
    if (!isReady || !tabs[current]) return;
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
    if (!btnBookmark || !tabs[current]) return;
    const url = tabs[current].url;
    const isBookmarked = bookmarks.some(b => b.url === url);
    if (isBookmarked) {
        btnBookmark.classList.add('active');
    } else {
        btnBookmark.classList.remove('active');
    }
}

async function showBookmarks() {
    if (!isReady) return;
    try {
        bookmarks = await invoke('get_bookmarks');
        if (bookmarks.length === 0) {
            statusText.textContent = 'Закладок нет';
            return;
        }
        // Открываем HTML страницу с закладками в окне контента
        const html = buildBookmarksHtml();
        await invoke('open_content', { url: 'data:text/html;charset=utf-8,' + encodeURIComponent(html) });
        hideMenu();
    } catch (e) {
        console.error('showBookmarks:', e);
    }
}

function buildBookmarksHtml() {
    const items = bookmarks.map(b =>
        `<li><a href="${escapeHtml(b.url)}"><strong>${escapeHtml(b.title)}</strong><br><small>${escapeHtml(b.url)}</small></a></li>`
    ).join('');
    return `<html><head><title>Закладки</title><style>
        body{font-family:system-ui;padding:40px;background:#fff;color:#000;margin:0}
        h1{margin-bottom:20px;color:#fc3f1d}
        ul{list-style:none;padding:0}
        li{margin:0}
        a{color:#0066cc;text-decoration:none;display:block;padding:12px 0;border-bottom:1px solid #eee}
        a:hover{color:#fc3f1d;background:#fafafa}
        small{color:#666}
    </style></head><body><h1>Закладки</h1><ul>${items}</ul></body></html>`;
}

async function showHistory() {
    if (!isReady) return;
    try {
        const history = await invoke('get_history');
        if (history.length === 0) {
            statusText.textContent = 'История пуста';
            return;
        }
        const items = history.slice(-100).reverse().map(h =>
            `<li><a href="${escapeHtml(h.url)}"><strong>${escapeHtml(h.title)}</strong><br><small>${escapeHtml(h.url)} · ${h.time.substring(0, 19)}</small></a></li>`
        ).join('');
        const html = `<html><head><title>История</title><style>
            body{font-family:system-ui;padding:40px;background:#fff;color:#000;margin:0}
            h1{margin-bottom:20px;color:#fc3f1d}
            ul{list-style:none;padding:0}
            li{margin:0}
            a{color:#0066cc;text-decoration:none;display:block;padding:12px 0;border-bottom:1px solid #eee}
            a:hover{color:#fc3f1d;background:#fafafa}
            small{color:#666}
        </style></head><body><h1>История</h1><ul>${items}</ul></body></html>`;
        await invoke('open_content', { url: 'data:text/html;charset=utf-8,' + encodeURIComponent(html) });
        hideMenu();
    } catch (e) {
        console.error('showHistory:', e);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  МАСШТАБ (для стартовой страницы)
// ═══════════════════════════════════════════════════════════════════════

async function zoomIn() { hideMenu(); }
async function zoomOut() { hideMenu(); }
async function zoomReset() { hideMenu(); }

// ═══════════════════════════════════════════════════════════════════════
//  ТЕМА
// ═══════════════════════════════════════════════════════════════════════

async function toggleTheme() {
    if (!isReady) return;
    try {
        theme = await invoke('toggle_theme');
        document.body.className = theme;
    } catch (e) { console.error('toggleTheme:', e); }
    hideMenu();
}

async function clearHistory() {
    if (!isReady) return;
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
    if (menuDropdown) menuDropdown.classList.toggle('show');
}

function hideMenu() {
    if (menuDropdown) menuDropdown.classList.remove('show');
}

// ═══════════════════════════════════════════════════════════════════════
//  Кнопки навигации
// ═══════════════════════════════════════════════════════════════════════

function updateNavButtons() {
    if (!btnBack || !btnForward || !tabs[current]) return;
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
    else if (ctrl && e.key === 'l') { e.preventDefault(); if (addressBar) { addressBar.focus(); addressBar.select(); } }
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
    if (menuDropdown && !e.target.closest('#menuDropdown') && !e.target.closest('#btnMenu')) {
        hideMenu();
    }
});

// ═══════════════════════════════════════════════════════════════════════
//  ГЛОБАЛЬНЫЕ ФУНКЦИИ (для onclick в HTML)
// ═══════════════════════════════════════════════════════════════════════

window.newTab = newTab;
window.switchTab = switchTab;
window.closeTab = closeTab;
window.navigate = navigate;
window.goBack = goBack;
window.goForward = goForward;
window.refresh = refresh;
window.goHome = goHome;
window.toggleBookmark = toggleBookmark;
window.toggleMenu = toggleMenu;
window.showBookmarks = showBookmarks;
window.showHistory = showHistory;
window.zoomIn = zoomIn;
window.zoomOut = zoomOut;
window.zoomReset = zoomReset;
window.toggleTheme = toggleTheme;
window.clearHistory = clearHistory;
window.onAddressKey = onAddressKey;
window.onStartSearchKey = onStartSearchKey;
window.onAddressFocus = onAddressFocus;
window.onAddressBlur = onAddressBlur;

// ═══════════════════════════════════════════════════════════════════════
//  ЗАПУСК
// ═══════════════════════════════════════════════════════════════════════

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
