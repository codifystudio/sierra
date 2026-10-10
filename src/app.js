// Sierra Browser — JavaScript логика v4.2.0
// Главное: ВСЕ события через addEventListener, явный debug overlay

// ═══════════════════════════════════════════════════════════════════════
//  DEBUG — показывает ошибки прямо в UI
// ═══════════════════════════════════════════════════════════════════════

let debugOverlayEl = null;
let debugLines = [];
const MAX_DEBUG = 10;

function showDebug(msg) {
    console.log('[Sierra]', msg);
    debugLines.push(msg);
    if (debugLines.length > MAX_DEBUG) debugLines.shift();
    if (debugOverlayEl) {
        debugOverlayEl.style.display = 'block';
        debugOverlayEl.innerHTML = debugLines.map(l => '› ' + l).join('<br>');
    }
}

function showDebugError(msg, err) {
    const text = msg + (err ? ': ' + (err.message || err) : '');
    console.error('[Sierra ERROR]', text);
    debugLines.push('⚠ ' + text);
    if (debugLines.length > MAX_DEBUG) debugLines.shift();
    if (debugOverlayEl) {
        debugOverlayEl.style.display = 'block';
        debugOverlayEl.innerHTML = debugLines.map(l => '› ' + l).join('<br>');
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  STATE
// ═══════════════════════════════════════════════════════════════════════

let invoke = null;
let isReady = false;
let tabs = [];
let current = 0;
let theme = 'dark';
let bookmarks = [];

// DOM
let tabsContainer, addressBar, startPage, startSearch;
let progress, progressFill, statusText, menuDropdown;
let btnBookmark, btnBack, btnForward, btnRefresh, btnHome, btnMenu, btnNewTab;

// ═══════════════════════════════════════════════════════════════════════
//  ОЖИДАНИЕ TAURI — поддерживает все возможные структуры
// ═══════════════════════════════════════════════════════════════════════

function getInvoke() {
    const t = window.__TAURI__;
    if (!t) return null;
    // Tauri 2.x: window.__TAURI__.core.invoke
    if (t.core && typeof t.core.invoke === 'function') return t.core.invoke.bind(t.core);
    // Tauri 1.x: window.__TAURI__.tauri.invoke
    if (t.tauri && typeof t.tauri.invoke === 'function') return t.tauri.invoke.bind(t.tauri);
    // Прямой invoke
    if (typeof t.invoke === 'function') return t.invoke.bind(t);
    return null;
}

function waitForTauri(maxMs = 10000) {
    return new Promise((resolve) => {
        const start = Date.now();
        const check = () => {
            const inv = getInvoke();
            if (inv) {
                resolve(inv);
            } else if (Date.now() - start > maxMs) {
                showDebugError('Tauri bridge не появился за ' + (maxMs/1000) + ' сек');
                resolve(null);
            } else {
                setTimeout(check, 50);
            }
        };
        check();
    });
}

// ═══════════════════════════════════════════════════════════════════════
//  INIT
// ═══════════════════════════════════════════════════════════════════════

function bindDomElements() {
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
    btnRefresh = document.getElementById('btnRefresh');
    btnHome = document.getElementById('btnHome');
    btnMenu = document.getElementById('btnMenu');
    btnNewTab = document.getElementById('btnNewTab');

    debugOverlayEl = document.getElementById('debugOverlay');
}

function attachEventListeners() {
    showDebug('Attaching event listeners...');

    // Кнопки навигации
    if (btnBack) btnBack.addEventListener('click', () => { showDebug('Back clicked'); goBack(); });
    if (btnForward) btnForward.addEventListener('click', () => { showDebug('Forward clicked'); goForward(); });
    if (btnRefresh) btnRefresh.addEventListener('click', () => { showDebug('Refresh clicked'); refresh(); });
    if (btnHome) btnHome.addEventListener('click', () => { showDebug('Home clicked'); goHome(); });
    if (btnNewTab) btnNewTab.addEventListener('click', () => { showDebug('New tab clicked'); newTab(); });
    if (btnBookmark) btnBookmark.addEventListener('click', () => { showDebug('Bookmark clicked'); toggleBookmark(); });
    if (btnMenu) btnMenu.addEventListener('click', (e) => {
        e.stopPropagation();
        if (menuDropdown) menuDropdown.classList.toggle('show');
    });

    // Адресная строка
    if (addressBar) {
        addressBar.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                const val = addressBar.value.trim();
                if (val) {
                    showDebug('Navigate: ' + val);
                    navigate(val);
                }
            } else if (e.key === 'Escape') {
                addressBar.blur();
            }
        });
        addressBar.addEventListener('focus', () => addressBar.select());
    }

    // Поиск на start page
    if (startSearch) {
        startSearch.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                const val = startSearch.value.trim();
                if (val) {
                    showDebug('Search: ' + val);
                    navigate(val);
                }
            }
        });
    }

    // Меню items
    document.querySelectorAll('.menu-item').forEach(item => {
        item.addEventListener('click', () => {
            const action = item.dataset.action;
            showDebug('Menu action: ' + action);
            if (action === 'new-tab') newTab();
            else if (action === 'show-bookmarks') showBookmarks();
            else if (action === 'show-history') showHistory();
            else if (action === 'toggle-theme') toggleTheme();
            else if (action === 'clear-history') clearHistory();
            if (menuDropdown) menuDropdown.classList.remove('show');
        });
    });

    // Shortcuts на start page
    document.querySelectorAll('.shortcut').forEach(s => {
        s.addEventListener('click', () => {
            const url = s.dataset.url;
            if (url) {
                showDebug('Shortcut: ' + url);
                navigate(url);
            }
        });
    });

    // Закрытие меню при клике вне
    document.addEventListener('click', (e) => {
        if (menuDropdown && !e.target.closest('#menuDropdown') && !e.target.closest('#btnMenu')) {
            menuDropdown.classList.remove('show');
        }
    });

    showDebug('Event listeners attached OK');
}

async function init() {
    bindDomElements();
    showDebug('Sierra init started');
    showDebug('UA: ' + (navigator.userAgent || 'unknown').substring(0, 60));
    showDebug('window.__TAURI__: ' + (window.__TAURI__ ? 'yes' : 'no'));

    attachEventListeners();

    if (statusText) statusText.textContent = 'Загрузка Tauri...';

    // Ждём Tauri
    invoke = await waitForTauri();
    if (!invoke) {
        showDebugError('Tauri bridge НЕ ЗАГРУЖЕН — кнопки работать не будут!');
        if (statusText) statusText.textContent = 'ОШИБКА: Tauri не загружен';
        return;
    }

    showDebug('Tauri invoke ready');
    isReady = true;
    if (statusText) statusText.textContent = 'Готово';

    // Загружаем тему
    try {
        theme = await invoke('get_theme');
        document.body.className = theme;
        showDebug('Theme: ' + theme);
    } catch (e) {
        showDebugError('get_theme failed', e);
    }

    // Загружаем закладки
    try {
        bookmarks = await invoke('get_bookmarks');
        showDebug('Bookmarks: ' + bookmarks.length);
    } catch (e) {
        showDebugError('get_bookmarks failed', e);
    }

    // Создаём стартовую вкладку
    try {
        await newTab();
        showDebug('Init complete');
    } catch (e) {
        showDebugError('newTab failed', e);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  ВКЛАДКИ
// ═══════════════════════════════════════════════════════════════════════

async function newTab() {
    if (!isReady) { showDebugError('newTab: not ready'); return; }
    try {
        await invoke('new_tab', { url: null });
        await refreshTabs();
        await showStartPage();
        if (addressBar) addressBar.value = '';
        if (startSearch) startSearch.focus();
    } catch (e) {
        showDebugError('newTab error', e);
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
        showDebugError('switchTab error', e);
    }
}

async function closeTab(idx) {
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
        showDebugError('closeTab error', e);
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
        showDebugError('refreshTabs error', e);
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
        el.dataset.idx = i;
        const title = tab.title || 'Без названия';
        const truncated = title.length > 28 ? title.substring(0, 28) + '…' : title;
        const safeTitle = escapeHtml(truncated);
        el.innerHTML = `<span class="tab-title">${safeTitle}</span><span class="tab-close">×</span>`;
        // Клик по вкладке — переключаем
        el.addEventListener('click', (e) => {
            if (!e.target.classList.contains('tab-close')) {
                switchTab(i);
            }
        });
        // Клик по крестику — закрываем
        const closeBtn = el.querySelector('.tab-close');
        if (closeBtn) {
            closeBtn.addEventListener('click', (e) => {
                e.stopPropagation();
                closeTab(i);
            });
        }
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
        if (addressBar) addressBar.value = '';
    } else {
        await showWebview(tab.url);
        if (addressBar) addressBar.value = tab.url;
    }
    updateBookmarkButton();
}

// ═══════════════════════════════════════════════════════════════════════
//  КОНТЕНТ
// ═══════════════════════════════════════════════════════════════════════

async function showStartPage() {
    if (startPage) startPage.style.display = 'flex';
    if (progress) progress.style.display = 'none';
    if (statusText) statusText.textContent = 'Готово';
    try {
        await invoke('hide_content');
    } catch (e) {
        showDebugError('hide_content', e);
    }
    if (startSearch) startSearch.focus();
}

async function showWebview(url) {
    if (startPage) startPage.style.display = 'none';
    if (progress) progress.style.display = 'block';
    if (progressFill) progressFill.style.width = '30%';
    if (statusText) statusText.textContent = 'Загрузка: ' + url;

    try {
        showDebug('open_content called: ' + url);
        await invoke('open_content', { url });
        showDebug('open_content returned OK');
        // Прогресс-симуляция
        setTimeout(() => { if (progressFill) progressFill.style.width = '70%'; }, 300);
        setTimeout(() => {
            if (progressFill) progressFill.style.width = '100%';
            setTimeout(() => { if (progress) progress.style.display = 'none'; }, 400);
        }, 1500);

        // Получаем URL от content окна (для обновления title)
        try {
            const actualUrl = await invoke('get_content_url');
            showDebug('content_url: ' + (actualUrl || 'null'));
            const finalUrl = actualUrl || url;
            const title = finalUrl.replace(/^https?:\/\//, '').split('/')[0];
            await invoke('update_tab_title', { title });
            await invoke('add_to_history', { url: finalUrl, title });
            await refreshTabs();
        } catch (e) {
            showDebugError('get_content_url failed', e);
            // Fallback: используем исходный URL
            const title = url.replace(/^https?:\/\//, '').split('/')[0];
            await invoke('update_tab_title', { title });
            await invoke('add_to_history', { url, title });
            await refreshTabs();
        }
    } catch (e) {
        showDebugError('open_content error', e);
        if (statusText) statusText.textContent = 'Ошибка: ' + e;
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
            if (addressBar) addressBar.value = tab.url;
            addressBar.blur();
        }
    } catch (e) {
        showDebugError('navigate error', e);
        if (statusText) statusText.textContent = 'Ошибка: ' + e;
    }
}

async function goBack() {
    if (!isReady) return;
    try {
        const tab = await invoke('go_back');
        if (tab) {
            await refreshTabs();
            await showWebview(tab.url);
            if (addressBar) addressBar.value = tab.url;
        }
    } catch (e) { showDebugError('goBack', e); }
}

async function goForward() {
    if (!isReady) return;
    try {
        const tab = await invoke('go_forward');
        if (tab) {
            await refreshTabs();
            await showWebview(tab.url);
            if (addressBar) addressBar.value = tab.url;
        }
    } catch (e) { showDebugError('goForward', e); }
}

async function refresh() {
    if (!isReady) return;
    if (tabs[current] && tabs[current].url && tabs[current].url !== 'https://duckduckgo.com') {
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
        if (statusText) statusText.textContent = isBookmarked ? 'Добавлено в закладки' : 'Удалено из закладок';
    } catch (e) {
        showDebugError('toggleBookmark', e);
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
            if (statusText) statusText.textContent = 'Закладок нет';
            return;
        }
        const html = buildBookmarksHtml();
        await invoke('open_content', { url: 'data:text/html;charset=utf-8,' + encodeURIComponent(html) });
    } catch (e) {
        showDebugError('showBookmarks', e);
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
            if (statusText) statusText.textContent = 'История пуста';
            return;
        }
        const items = history.slice(-100).reverse().map(h =>
            `<li><a href="${escapeHtml(h.url)}"><strong>${escapeHtml(h.title)}</strong><br><small>${escapeHtml(h.url)} · ${h.time.substring(0, 19)}</small></a></li>`
        ).join('');
        const html = `<html><head><title>История</title><style>
            body{font-family:system-ui;padding:40px;background:#fff;color:#000;margin:0}
            h1{margin-bottom:20px;color:#fc3f1d}
            ul{list-style:none;padding:0}
            a{color:#0066cc;text-decoration:none;display:block;padding:12px 0;border-bottom:1px solid #eee}
            a:hover{color:#fc3f1d;background:#fafafa}
            small{color:#666}
        </style></head><body><h1>История</h1><ul>${items}</ul></body></html>`;
        await invoke('open_content', { url: 'data:text/html;charset=utf-8,' + encodeURIComponent(html) });
    } catch (e) {
        showDebugError('showHistory', e);
    }
}

async function toggleTheme() {
    if (!isReady) return;
    try {
        theme = await invoke('toggle_theme');
        document.body.className = theme;
        showDebug('Theme: ' + theme);
    } catch (e) {
        showDebugError('toggleTheme', e);
    }
}

async function clearHistory() {
    if (!isReady) return;
    try {
        await invoke('clear_history');
        if (statusText) statusText.textContent = 'История очищена';
    } catch (e) {
        showDebugError('clearHistory', e);
    }
}

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
    else if (ctrl && e.key === 'w') { e.preventDefault(); closeTab(current); }
    else if (ctrl && e.key === 'l') { e.preventDefault(); if (addressBar) { addressBar.focus(); addressBar.select(); } }
    else if (ctrl && e.key === 'd') { e.preventDefault(); toggleBookmark(); }
    else if (ctrl && e.key === 'h') { e.preventDefault(); showHistory(); }
    else if (ctrl && e.shiftKey && (e.key === 'T' || e.key === 't')) { e.preventDefault(); toggleTheme(); }
    else if (e.altKey && e.key === 'ArrowLeft') { e.preventDefault(); goBack(); }
    else if (e.altKey && e.key === 'ArrowRight') { e.preventDefault(); goForward(); }
    else if (e.altKey && e.key === 'Home') { e.preventDefault(); goHome(); }
    else if (e.key === 'F5') { e.preventDefault(); refresh(); }
    else if (e.key === 'Escape') {
        if (menuDropdown) menuDropdown.classList.remove('show');
    }
});

// ═══════════════════════════════════════════════════════════════════════
//  ЗАПУСК
// ═══════════════════════════════════════════════════════════════════════

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
