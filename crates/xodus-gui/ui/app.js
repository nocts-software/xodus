// Xodus GUI Client Application Logic

function safeGetStorage(key, fallback = null) {
  try {
    return (typeof localStorage !== 'undefined' && localStorage.getItem(key)) || fallback;
  } catch (e) {
    return fallback;
  }
}

function safeSetStorage(key, val) {
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(key, val);
    }
  } catch (e) {}
}

// Native IPC Bridge (Defined first so error handlers and early events can use it)
function sendNativeCommand(payload) {
  const str = typeof payload === 'string' ? payload : JSON.stringify(payload);
  if (window.ipc && typeof window.ipc.postMessage === 'function') {
    try {
      window.ipc.postMessage(str);
    } catch (e) {
      console.error('[Native IPC Error]', e);
    }
  } else if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.ipc && typeof window.webkit.messageHandlers.ipc.postMessage === 'function') {
    try {
      window.webkit.messageHandlers.ipc.postMessage(str);
    } catch (e) {
      console.error('[Native WebKit IPC Error]', e);
    }
  } else {
    console.log('[Native IPC Pending]', payload);
    setTimeout(() => {
      if (window.ipc && typeof window.ipc.postMessage === 'function') {
        window.ipc.postMessage(str);
      } else if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.ipc) {
        window.webkit.messageHandlers.ipc.postMessage(str);
      }
    }, 150);
  }
}
window.sendNativeCommand = sendNativeCommand;

window.onerror = function(msg, url, line, col, error) {
  if (msg === 'Script error.' && (!url || url === '')) return true;
  console.error('[JS ERROR]', msg, 'at', url, line + ':' + col, error);
  sendNativeCommand({ cmd: 'js_error', msg: String(msg), url: String(url), line: line, col: col, stack: error ? error.stack : '' });
  return false;
};
window.addEventListener('unhandledrejection', function(event) {
  console.error('[JS UNHANDLED PROMISE]', event.reason);
  sendNativeCommand({ cmd: 'js_error', msg: 'Unhandled Promise: ' + String(event.reason) });
});

const state = {
  activeTab: 'library',
  filter: 'all',
  searchQuery: '',
  hasGamePassSubscription: false,
  gamePassTier: null,
  user: {
    gamertag: 'Xbox Player',
    puid: '',
    presence: safeGetStorage('xodus_user_presence', 'Active'),
    gamerscore: '0',
    avatar: 'https://assets.xboxservices.com/assets/default_avatar.png',
    hasGamePass: false,
    subscriptionTier: null,
  },
  games: [],
  friends: []
};
window.state = state;

// Initialize Application
function initApp() {
  setupNavigation();
  setupWindowControls();
  setupCustomDropdowns();
  setupSearchAndFilters();
  renderUser();
  updateGamePassVisibility();
  renderGames();
  renderFriends();
  updateHeroBanner(state.games[0]);

  const authBtn = document.getElementById('authButton');
  if (authBtn) {
    authBtn.addEventListener('click', () => {
      showToast('Opening Microsoft Sign-In...');
      sendNativeCommand({ cmd: 'login' });
    });
  }

  // Request live Xbox Live profile, friends, and entitlements from backend
  sendNativeCommand({ cmd: 'init' });
}

// Initialize IPC bridge immediately
setupIPCBridge();

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initApp);
} else {
  initApp();
}


function setupCustomDropdowns() {
  setupSingleDropdown('presenceDropdown', 'presenceTrigger', (value) => {
    updatePresence(value);
  });

  setupSingleDropdown('protonDropdown', 'protonTrigger', (value) => {
    const text = document.getElementById('protonCurrentText');
    if (text) {
      if (value.includes('cachyos')) text.textContent = 'Proton CachyOS Native (RADV + FSR4)';
      else if (value.includes('GE')) text.textContent = 'GE-Proton 11-3';
      else text.textContent = 'System Wine';
    }
  });

  document.addEventListener('click', (e) => {
    if (!e.target.closest('.custom-dropdown')) {
      document.querySelectorAll('.custom-dropdown').forEach(d => d.classList.remove('open'));
    }
  });
}

function setupSingleDropdown(dropdownId, triggerId, onSelect) {
  const dropdown = document.getElementById(dropdownId);
  const trigger = document.getElementById(triggerId);
  if (!dropdown || !trigger) return;

  trigger.addEventListener('click', (e) => {
    e.preventDefault();
    e.stopPropagation();
    const isOpen = dropdown.classList.contains('open');
    document.querySelectorAll('.custom-dropdown').forEach(d => d.classList.remove('open'));
    if (!isOpen) {
      dropdown.classList.add('open');
    }
  });

  const items = dropdown.querySelectorAll('.dropdown-item');
  items.forEach(item => {
    item.addEventListener('click', (e) => {
      e.stopPropagation();
      items.forEach(i => i.classList.remove('selected'));
      item.classList.add('selected');
      const val = item.getAttribute('data-value');
      onSelect(val);
      dropdown.classList.remove('open');
    });
  });
}

function setupWindowControls() {
  const minBtn = document.getElementById('winMinimize');
  const maxBtn = document.getElementById('winMaximize');
  const closeBtn = document.getElementById('winClose');
  const titlebar = document.getElementById('appTitlebar');

  if (minBtn) minBtn.addEventListener('click', () => sendNativeCommand({ cmd: 'minimize' }));
  if (maxBtn) maxBtn.addEventListener('click', () => sendNativeCommand({ cmd: 'maximize' }));
  if (closeBtn) closeBtn.addEventListener('click', () => sendNativeCommand({ cmd: 'close' }));

  if (titlebar) {
    titlebar.addEventListener('mousedown', (e) => {
      if (e.target.closest('button') || e.target.closest('input') || e.target.closest('select')) return;
      if (e.buttons === 1) {
        sendNativeCommand({ cmd: 'drag_window' });
      }
    });
  }
}

// Navigation Handling
function setupNavigation() {
  const navItems = document.querySelectorAll('.nav-item');
  navItems.forEach(item => {
    item.addEventListener('click', () => {
      const tab = item.getAttribute('data-tab');
      switchTab(tab);
    });
  });
}

function setupSearchAndFilters() {
  const searchInput = document.getElementById('searchInput');
  if (searchInput) {
    searchInput.addEventListener('input', (e) => {
      state.searchQuery = e.target.value.toLowerCase().trim();
      renderGames();
    });
  }

  const pills = document.querySelectorAll('#filterPills .pill');
  pills.forEach(pill => {
    pill.addEventListener('click', () => {
      pills.forEach(p => p.classList.remove('active'));
      pill.classList.add('active');
      state.filter = pill.getAttribute('data-filter') || 'all';
      renderGames();
    });
  });
}

function setFilter(filterId) {
  state.filter = filterId || 'all';
  const pills = document.querySelectorAll('#filterPills .pill');
  pills.forEach(p => p.classList.toggle('active', p.getAttribute('data-filter') === state.filter));
  renderGames();
}
window.setFilter = setFilter;

function switchTab(tabId) {
  if (tabId === 'gamepass') {
    state.activeTab = 'library';
    state.filter = 'gamepass';
    const pills = document.querySelectorAll('#filterPills .pill');
    pills.forEach(p => p.classList.toggle('active', p.getAttribute('data-filter') === 'gamepass'));
    document.querySelectorAll('.nav-item').forEach(el => {
      el.classList.toggle('active', el.getAttribute('data-tab') === 'gamepass');
    });
    document.querySelectorAll('.tab-panel').forEach(panel => {
      panel.classList.toggle('active', panel.id === 'tab-library');
    });
    renderGames();
    return;
  }

  state.activeTab = tabId;
  document.querySelectorAll('.nav-item').forEach(el => {
    el.classList.toggle('active', el.getAttribute('data-tab') === tabId);
  });
  document.querySelectorAll('.tab-panel').forEach(panel => {
    panel.classList.toggle('active', panel.id === `tab-${tabId}`);
  });

  if (tabId === 'friends') renderFriends();
  else if (tabId === 'library') renderGames();
}
window.switchTab = switchTab;

// User Rendering
function renderUser() {
  const nameEl = document.getElementById('userGamertag');
  if (nameEl) nameEl.textContent = state.user.gamertag;
  const avatarEl = document.getElementById('userAvatar');
  if (avatarEl) avatarEl.src = state.user.avatar;
  const scoreEl = document.getElementById('userScoreText');
  if (scoreEl) {
    const scoreVal = parseInt(state.user.gamerscore, 10);
    scoreEl.textContent = isNaN(scoreVal) ? state.user.gamerscore : scoreVal.toLocaleString();
  }

  // Update Presence Status Indicators
  const presenceVal = state.user.presence || 'Active';
  const textEl = document.getElementById('presenceCurrentText');
  const dotEl = document.getElementById('presenceDot');
  const badgeEl = document.getElementById('userPresenceBadge');

  if (textEl) {
    textEl.textContent = presenceVal === 'Active' ? 'Online' : (presenceVal === 'Away' ? 'Away' : 'Invisible');
  }
  if (dotEl) {
    dotEl.className = `status-indicator-dot dot-${presenceVal === 'Active' ? 'online' : (presenceVal === 'Away' ? 'away' : 'invisible')}`;
  }
  if (badgeEl) {
    badgeEl.className = `presence-badge ${presenceVal === 'Active' ? 'online' : (presenceVal === 'Away' ? 'away' : 'offline')}`;
  }

  const dropdownItems = document.querySelectorAll('#presenceMenu .dropdown-item');
  dropdownItems.forEach(item => {
    item.classList.toggle('selected', item.getAttribute('data-value') === presenceVal);
  });

  // Render Game Pass Tier in User Profile Box
  const gpBadge = document.getElementById('userGamePassBadge');
  const gpTierText = document.getElementById('userGamePassTierText');
  const hasGP = !!state.hasGamePassSubscription || !!(state.user && state.user.hasGamePass);
  const rawTier = state.gamePassTier || (state.user && state.user.subscriptionTier);
  const tier = rawTier || (hasGP ? 'PC Game Pass' : null);

  if (gpBadge && gpTierText) {
    if (hasGP && tier) {
      gpBadge.style.display = 'inline-flex';
      gpTierText.textContent = tier;
      if (tier.toLowerCase().includes('ultimate')) {
        gpBadge.className = 'user-gp-badge tier-ultimate';
      } else {
        gpBadge.className = 'user-gp-badge';
      }
    } else {
      gpBadge.style.display = 'none';
    }
  }
}

function updateGamePassVisibility() {
  const hasGP = !!state.hasGamePassSubscription || !!(state.user && state.user.hasGamePass);

  // Sidebar Game Pass Navigation Item
  const gpNavItem = document.getElementById('navItemGamePass');
  if (gpNavItem) {
    gpNavItem.style.display = hasGP ? 'flex' : 'none';
  }

  // Filter Pill in Library View
  const gpFilterPill = document.getElementById('filterPillGamePass');
  if (gpFilterPill) {
    gpFilterPill.style.display = hasGP ? 'inline-flex' : 'none';
  }

  // If user does not have Game Pass and is currently on gamepass filter or tab, reset to all
  if (!hasGP) {
    if (state.filter === 'gamepass') {
      state.filter = 'all';
      const pills = document.querySelectorAll('#filterPills .pill');
      pills.forEach(p => p.classList.toggle('active', p.getAttribute('data-filter') === 'all'));
    }
    if (state.activeTab === 'gamepass') {
      switchTab('library');
    }
  }
}

function renderSidebarInstalled() {
  const container = document.getElementById('sidebarInstalledList');
  const countEl = document.getElementById('sidebarInstalledCount');
  if (!container) return;
  container.innerHTML = '';

  const installedGames = state.games.filter(g => g.installed);
  if (countEl) {
    countEl.textContent = installedGames.length;
  }

  installedGames.forEach(game => {
    const item = document.createElement('div');
    item.className = 'sidebar-installed-item';
    item.title = game.title;
    item.innerHTML = `
      <img class="sidebar-installed-icon" src="${game.cover}" alt="${game.title}">
      <span class="sidebar-installed-name">${game.title}</span>
      <button class="sidebar-installed-play" title="Play ${game.title}">
        <svg viewBox="0 0 24 24" width="10" height="10" fill="currentColor">
          <polygon points="5 3 19 12 5 21 5 3"></polygon>
        </svg>
      </button>
    `;
    item.addEventListener('click', (e) => {
      if (e.target.closest('.sidebar-installed-play')) {
        launchGame(game.title, game.path);
      } else {
        switchTab('library');
        updateHeroBanner(game);
      }
    });
    container.appendChild(item);
  });
}

function updateHeroBanner(game) {
  if (!game) return;
  const bgImg = document.getElementById('heroBgImage');
  const titleEl = document.getElementById('heroTitle');
  const descEl = document.getElementById('heroDesc');
  const badgeEl = document.getElementById('heroBadge');
  const actionsEl = document.getElementById('heroActions');

  const gameTitle = game.title || 'Unknown Title';
  const gameDev = game.developer || 'Xbox Game Studios';
  const gameSize = game.size || 'Standard';
  const gamePath = game.path || `/mnt/w11/XboxGames/${game.productId || game.id || ''}`;
  const isInstalled = !!game.installed;

  if (titleEl) titleEl.textContent = gameTitle;
  if (descEl) descEl.textContent = `${gameDev} • ${gameSize} • ${isInstalled ? 'Installed Local Container' : 'Cloud Entitled'}`;
  if (badgeEl) badgeEl.textContent = isInstalled ? 'JUST PLAYED • READY TO PLAY' : (game.licenseType === 'gamepass' ? 'INCLUDED WITH GAME PASS' : 'OWNED LICENSE');
  if (bgImg) {
    bgImg.src = game.splash || game.cover || 'https://assets.xboxservices.com/assets/default_cover.png';
  }

  if (actionsEl) {
    actionsEl.innerHTML = '';
    if (isInstalled) {
      const playBtn = document.createElement('button');
      playBtn.className = 'btn btn-primary btn-lg';
      playBtn.id = 'heroPlayBtn';
      playBtn.innerHTML = `
        <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
          <polygon points="5 3 19 12 5 21 5 3"></polygon>
        </svg>
        <span>Play</span>
      `;
      playBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        launchGame(gameTitle, gamePath);
      });
      actionsEl.appendChild(playBtn);

      const syncBtn = document.createElement('button');
      syncBtn.className = 'btn btn-secondary btn-lg';
      syncBtn.id = 'heroSyncBtn';
      syncBtn.innerHTML = `
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/>
        </svg>
        <span>Sync Saves</span>
      `;
      syncBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        syncGameSaves(gamePath);
      });
      actionsEl.appendChild(syncBtn);

      const uninstallBtn = document.createElement('button');
      uninstallBtn.className = 'btn btn-secondary btn-lg btn-danger-hover';
      uninstallBtn.title = `Uninstall ${gameTitle}`;
      uninstallBtn.innerHTML = `
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="3 6 5 6 21 6"></polyline>
          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
        </svg>
        <span>Uninstall</span>
      `;
      uninstallBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        promptUninstallGame(gameTitle, gamePath);
      });
      actionsEl.appendChild(uninstallBtn);
    } else {
      const installBtn = document.createElement('button');
      installBtn.className = 'btn btn-primary btn-lg';
      installBtn.innerHTML = `
        <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
          <polyline points="7 10 12 15 17 10"/>
          <line x1="12" y1="15" x2="12" y2="3"/>
        </svg>
        <span>Install Game</span>
      `;
      installBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        installGame(gameTitle, gamePath);
      });
      actionsEl.appendChild(installBtn);
    }
  }
}

// Games Grid Rendering
function renderGames() {
  const grid = document.getElementById('gamesGrid');
  if (!grid) return;
  grid.innerHTML = '';

  const filtered = state.games.filter(game => {
    if (!game) return false;
    const q = state.searchQuery || '';
    const title = (game.title || '').toLowerCase();
    const dev = (game.developer || '').toLowerCase();
    const pid = (game.productId || game.id || '').toLowerCase();
    const matchesSearch = !q || title.includes(q) || dev.includes(q) || pid.includes(q);

    if (!matchesSearch) return false;

    if (state.filter === 'installed') return !!game.installed;
    if (state.filter === 'gamepass') return game.licenseType === 'gamepass';
    if (state.filter === 'owned') return game.licenseType === 'owned';

    if (state.hasGamePassSubscription === false && state.activeTab === 'library') {
      return !!game.installed || game.licenseType === 'owned';
    }

    return true;
  });

  filtered.forEach(game => {
    const card = document.createElement('div');
    card.className = 'game-card';
    card.addEventListener('click', (e) => {
      if (!e.target.closest('button')) {
        updateHeroBanner(game);
      }
    });

    let badgeClass = 'gamepass';
    let badgeText = 'GAME PASS';
    if (game.installed) {
      badgeClass = 'installed';
      badgeText = 'INSTALLED';
    } else if (game.licenseType === 'owned') {
      badgeClass = 'owned';
      badgeText = 'OWNED';
    }

    const coverUrl = game.cover || 'https://assets.xboxservices.com/assets/default_cover.png';
    const gameTitle = game.title || 'Untitled';
    const gameDev = game.developer || 'Xbox';
    const gameSize = game.size || 'Standard';
    const gamePath = game.path || `/mnt/w11/XboxGames/${game.productId || game.id || ''}`;

    const coverDiv = document.createElement('div');
    coverDiv.className = 'game-card-cover';
    coverDiv.innerHTML = `
      <img src="${coverUrl}" alt="" loading="lazy">
      <span class="game-card-badge ${badgeClass}">${badgeText}</span>
    `;
    card.appendChild(coverDiv);

    const infoDiv = document.createElement('div');
    infoDiv.className = 'game-card-info';
    infoDiv.innerHTML = `
      <span class="game-card-title">${gameTitle}</span>
      <div class="game-card-meta">
        <span>${gameDev}</span>
        <span>${gameSize}</span>
      </div>
    `;

    const actionsDiv = document.createElement('div');
    actionsDiv.className = 'game-card-actions';

    if (game.installed) {
      const playBtn = document.createElement('button');
      playBtn.className = 'btn btn-primary btn-sm';
      playBtn.style.flex = '1';
      playBtn.textContent = 'Play';
      playBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        launchGame(gameTitle, gamePath);
      });
      actionsDiv.appendChild(playBtn);
    } else if (game.licenseType === 'gamepass' && state.hasGamePassSubscription === false) {
      const joinBtn = document.createElement('button');
      joinBtn.className = 'btn btn-secondary btn-sm';
      joinBtn.style.flex = '1';
      joinBtn.style.opacity = '0.7';
      joinBtn.textContent = 'Join Game Pass';
      joinBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        showToast('Active PC Game Pass subscription required to install this title');
      });
      actionsDiv.appendChild(joinBtn);
    } else {
      const installBtn = document.createElement('button');
      installBtn.className = 'btn btn-secondary btn-sm';
      installBtn.style.flex = '1';
      installBtn.textContent = 'Install';
      installBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        installGame(gameTitle, gamePath);
      });
      actionsDiv.appendChild(installBtn);
    }

    const syncBtn = document.createElement('button');
    syncBtn.className = 'btn btn-secondary btn-sm';
    syncBtn.title = 'Sync Saves';
    syncBtn.innerHTML = `
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/>
      </svg>
    `;
    syncBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      syncGameSaves(gamePath);
    });
    actionsDiv.appendChild(syncBtn);

    if (game.installed) {
      const uninstallBtn = document.createElement('button');
      uninstallBtn.className = 'btn btn-secondary btn-sm btn-danger-hover';
      uninstallBtn.title = `Uninstall ${gameTitle}`;
      uninstallBtn.innerHTML = `
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="3 6 5 6 21 6"></polyline>
          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
        </svg>
      `;
      uninstallBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        promptUninstallGame(gameTitle, gamePath);
      });
      actionsDiv.appendChild(uninstallBtn);
    }

    infoDiv.appendChild(actionsDiv);
    card.appendChild(infoDiv);
    grid.appendChild(card);
  });

  const userOwnedInstalledCount = state.games.filter(g => g.installed || g.licenseType === 'owned').length;
  const libraryCountEl = document.getElementById('libraryCount');
  if (libraryCountEl) {
    libraryCountEl.textContent = state.hasGamePassSubscription !== false ? state.games.length : userOwnedInstalledCount;
  }
  const countText = document.getElementById('gamesCountText');
  if (countText) {
    countText.textContent = `Showing ${filtered.length} of ${state.hasGamePassSubscription !== false ? state.games.length : userOwnedInstalledCount} titles`;
  }

  renderSidebarInstalled();
}


// Cloud Saves Rendering
function renderSaves() {
  const container = document.getElementById('savesList');
  if (!container) return;
  container.innerHTML = '';

  state.games.forEach(game => {
    const item = document.createElement('div');
    item.className = 'save-item';
    item.innerHTML = `
      <div class="save-item-info">
        <span class="save-item-title">${game.title}</span>
        <span class="save-item-meta">Product ID: ${game.productId} • Dev: ${game.developer}</span>
      </div>
      <div class="save-item-actions">
        <span class="status-indicator-dot ${game.cloudSynced ? 'synced' : 'dot-away'}" title="${game.cloudSynced ? 'In Sync' : 'Needs Sync'}"></span>
        <button class="btn btn-secondary btn-sm" onclick="pullSave('${game.path}')">Pull Cloud</button>
        <button class="btn btn-secondary btn-sm" onclick="pushSave('${game.path}')">Push Local</button>
      </div>
    `;
    container.appendChild(item);
  });
}

// Friends List Rendering
function renderFriends() {
  const inGameList = document.getElementById('inGameList');
  const onlineList = document.getElementById('onlineList');
  const offlineList = document.getElementById('offlineList');

  if (inGameList) inGameList.innerHTML = '';
  if (onlineList) onlineList.innerHTML = '';
  if (offlineList) offlineList.innerHTML = '';

  let inGameCount = 0;
  let onlineCount = 0;
  let offlineCount = 0;

  state.friends.forEach(f => {
    if (!f) return;
    const card = document.createElement('div');
    card.className = 'friend-card';
    const st = (f.state || f.presenceState || 'offline').toLowerCase();
    const isIngame = st === 'in-game' || st === 'ingame';
    const isOnline = st === 'online' || st === 'active' || st === 'away';
    const badgeClass = isIngame ? 'online' : (isOnline ? (st === 'away' ? 'away' : 'online') : 'offline');
    const avatarUrl = f.avatar || f.displayPicRaw || 'https://assets.xboxservices.com/assets/default_avatar.png';
    const gamertag = f.gamertag || 'Xbox Friend';
    const richText = f.richPresence || f.presenceText || (isOnline ? 'Online' : 'Offline');

    card.innerHTML = `
      <div class="friend-main">
        <div class="friend-avatar">
          <img src="${avatarUrl}" alt="">
          <span class="presence-badge ${badgeClass}"></span>
        </div>
        <div class="friend-details">
          <span class="friend-gamertag">${gamertag}</span>
          <span class="friend-presence ${isIngame ? 'in-game' : ''}">${richText}</span>
        </div>
      </div>
    `;

    if (f.canJoin) {
      const joinBtn = document.createElement('button');
      joinBtn.className = 'btn btn-primary btn-sm';
      joinBtn.textContent = 'Join Game';
      joinBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        joinFriendGame(gamertag, f.gameTitle || '');
      });
      card.appendChild(joinBtn);
    }

    if (isIngame && inGameList) {
      inGameList.appendChild(card);
      inGameCount++;
    } else if (isOnline && onlineList) {
      onlineList.appendChild(card);
      onlineCount++;
    } else if (offlineList) {
      offlineList.appendChild(card);
      offlineCount++;
    }
  });

  if (inGameList && inGameCount === 0) {
    inGameList.innerHTML = '<div class="friends-empty-hint">No friends currently playing games</div>';
  }
  if (onlineList && onlineCount === 0) {
    onlineList.innerHTML = '<div class="friends-empty-hint">No friends currently online</div>';
  }
  if (offlineList && offlineCount === 0) {
    offlineList.innerHTML = '<div class="friends-empty-hint">No offline friends found</div>';
  }

  const inGameCountEl = document.getElementById('inGameCount');
  if (inGameCountEl) inGameCountEl.textContent = inGameCount;
  const onlineOnlyEl = document.getElementById('onlineOnlyCount');
  if (onlineOnlyEl) onlineOnlyEl.textContent = onlineCount;
  const onlineFriendsEl = document.getElementById('onlineFriendsCount');
  if (onlineFriendsEl) onlineFriendsEl.textContent = inGameCount + onlineCount;
  
  const friendsNavBadge = document.querySelector('.nav-item[data-tab="friends"] .badge');
  if (friendsNavBadge) friendsNavBadge.textContent = inGameCount + onlineCount;
}

// Actions & Handlers
function launchGame(title, path) {
  showToast(`Checking cloud saves for ${title}...`);
  sendNativeCommand({ cmd: 'launch_game', path: path });
}

window.showCloudSyncDialog = function(path, localInfo, cloudInfo) {
  const modal = document.getElementById('cloudSyncModal');
  document.getElementById('localSaveInfo').innerText = localInfo;
  document.getElementById('cloudSaveInfo').innerText = cloudInfo;
  if (modal) {
    modal.dataset.path = path;
    modal.classList.add('visible');
    modal.style.display = 'flex';
  }
};

window.resolveSaveConflict = function(choice) {
  const modal = document.getElementById('cloudSyncModal');
  const path = modal ? modal.dataset.path : '';
  if (modal) {
    modal.classList.remove('visible');
    modal.style.display = 'none';
  }
  
  showToast(choice === 'cloud' ? 'Downloading cloud save & launching...' : 'Uploading local save & launching...');
  sendNativeCommand({ cmd: 'resolve_save_conflict', path: path, choice: choice });
};

window.promptUninstallGame = function(title, path) {
  state.pendingUninstall = { title, path };
  const modal = document.getElementById('uninstallModal');
  const titleEl = document.getElementById('uninstallGameTitle');
  if (titleEl) titleEl.textContent = title;
  if (modal) {
    modal.classList.add('visible');
    modal.style.display = 'flex';
  }
};

window.closeUninstallModal = function() {
  const modal = document.getElementById('uninstallModal');
  if (modal) {
    modal.classList.remove('visible');
    modal.style.display = 'none';
  }
  state.pendingUninstall = null;
};

window.confirmUninstallGame = function() {
  const modal = document.getElementById('uninstallModal');
  if (modal) {
    modal.classList.remove('visible');
    modal.style.display = 'none';
  }
  if (!state.pendingUninstall) return;
  const { title, path } = state.pendingUninstall;
  state.pendingUninstall = null;
  uninstallGame(title, path);
};

function uninstallGame(title, path) {
  showToast(`Uninstalling ${title}...`);
  showProgress(`Uninstalling ${title}...`, 50, 'Removing');
  sendNativeCommand({
    cmd: 'uninstall_game',
    title: title,
    path: path
  });
}

function installGame(title, path) {
  const game = state.games.find(g => g.title === title || g.path === path);
  if (game && game.licenseType === 'gamepass' && state.hasGamePassSubscription === false && !game.installed) {
    showToast('PC Game Pass subscription required to install this title');
    return;
  }

  showToast(`Connecting to Microsoft Delivery Optimization for ${title}...`);
  showProgress(`Connecting to Microsoft Delivery Optimization...`, 5, 'Connecting');
  sendNativeCommand({
    cmd: 'install_game',
    title: title,
    productId: game ? (game.productId || game.id) : '',
    path: path
  });
}


function syncGameSaves(path) {
  showToast('Synchronizing Xbox Connected Storage saves...');
  sendNativeCommand({ cmd: 'sync_saves', path: path });
}

function syncAllSaves() {
  showToast('Synchronizing all cloud saves with titlestorage.xboxlive.com...');
  sendNativeCommand({ cmd: 'sync_all_saves' });
}

function pullSave(path) {
  showToast('Pulling cloud save...');
  sendNativeCommand({ cmd: 'pull_save', path: path });
}

function pushSave(path) {
  showToast('Pushing local save to cloud...');
  sendNativeCommand({ cmd: 'push_save', path: path });
}

function joinFriendGame(gamertag, gameTitle) {
  showToast(`Joining ${gamertag}'s ${gameTitle} multiplayer session...`);
  sendNativeCommand({ cmd: 'join_game', gamertag: gamertag, title: gameTitle });
}

function updatePresence(stateVal) {
  state.user.presence = stateVal;
  safeSetStorage('xodus_user_presence', stateVal);
  renderUser();
  const label = stateVal === 'Active' ? 'Online' : (stateVal === 'Away' ? 'Away' : 'Invisible');
  showToast(`Presence status updated to: ${label}`);
  sendNativeCommand({ cmd: 'set_presence', state: stateVal });
}
window.updatePresence = updatePresence;

function refreshUserLicenses() {
  showToast('Querying Microsoft Entitlements & Licenses...');
  sendNativeCommand({ cmd: 'sync_licenses' });
}
window.refreshUserLicenses = refreshUserLicenses;

function refreshFriends() {
  showToast('Updating Xbox Live social graph...');
  sendNativeCommand({ cmd: 'get_friends' });
}
window.refreshFriends = refreshFriends;

window.launchGame = launchGame;
window.installGame = installGame;
window.uninstallGame = uninstallGame;
window.promptUninstallGame = promptUninstallGame;
window.confirmUninstallGame = confirmUninstallGame;
window.closeUninstallModal = closeUninstallModal;
window.syncGameSaves = syncGameSaves;
window.syncAllSaves = syncAllSaves;
window.pullSave = pullSave;
window.pushSave = pushSave;
window.joinFriendGame = joinFriendGame;
window.switchTab = switchTab;

// Progress Bar
function showProgress(title, percent, speed = '32.4 MB/s') {
  const bar = document.getElementById('statusBar');
  bar.style.display = 'flex';
  document.getElementById('statusTitle').textContent = title;
  document.getElementById('statusSpeed').textContent = speed;
  document.getElementById('progressBarFill').style.width = `${percent}%`;
}

function hideProgress() {
  const bar = document.getElementById('statusBar');
  bar.style.display = 'none';
}

function cancelActiveTask() {
  hideProgress();
  showToast('Task canceled by user.');
}

// Toast Notifications
function showToast(message) {
  const toast = document.getElementById('toast');
  document.getElementById('toastMessage').textContent = message;
  toast.classList.add('visible');
  clearTimeout(window.toastTimer);
  window.toastTimer = setTimeout(() => {
    toast.classList.remove('visible');
  }, 3500);
}

// Native IPC Bridge
function sendNativeCommand(payload) {
  const str = typeof payload === 'string' ? payload : JSON.stringify(payload);
  if (window.ipc && typeof window.ipc.postMessage === 'function') {
    window.ipc.postMessage(str);
  } else if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.ipc && typeof window.webkit.messageHandlers.ipc.postMessage === 'function') {
    window.webkit.messageHandlers.ipc.postMessage(str);
  } else {
    console.log('[Native IPC Pending]', payload);
    setTimeout(() => {
      if (window.ipc && typeof window.ipc.postMessage === 'function') {
        window.ipc.postMessage(str);
      } else if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.ipc) {
        window.webkit.messageHandlers.ipc.postMessage(str);
      }
    }, 150);
  }
}

function setupIPCBridge() {
  window.setUserData = (profile) => {
    if (profile) {
      if (profile.gamertag) state.user.gamertag = profile.gamertag;
      if (profile.displayPic) state.user.avatar = profile.displayPic;
      if (profile.displayPicRaw) state.user.avatar = profile.displayPicRaw;
      if (profile.display_pic) state.user.avatar = profile.display_pic;
      if (profile.gamerscore) state.user.gamerscore = profile.gamerscore;
      if (profile.gamerScore) state.user.gamerscore = profile.gamerScore;
      if (profile.presence) {
        state.user.presence = profile.presence;
        safeSetStorage('xodus_user_presence', profile.presence);
      }
      if (profile.hasGamePass !== undefined) {
        state.hasGamePassSubscription = !!profile.hasGamePass;
        state.user.hasGamePass = !!profile.hasGamePass;
      }
      if (profile.subscriptionTier !== undefined) {
        state.gamePassTier = profile.subscriptionTier;
        state.user.subscriptionTier = profile.subscriptionTier;
      }
      renderUser();
      updateGamePassVisibility();
      showToast(`Welcome back, ${state.user.gamertag}!`);
    }
  };

  window.setGamePassStatus = (hasSubscription, tier) => {
    state.hasGamePassSubscription = !!hasSubscription;
    state.user.hasGamePass = !!hasSubscription;
    if (tier !== undefined) {
      state.gamePassTier = tier;
      state.user.subscriptionTier = tier;
    }
    console.log('[XODUS] Game Pass Status:', state.hasGamePassSubscription, 'Tier:', state.gamePassTier);
    renderUser();
    updateGamePassVisibility();
    renderGames();
  };

function getEditionTier(title) {
  const lower = (title || '').toLowerCase();
  if (lower.includes('ultimate') || lower.includes('complete') || lower.includes('anniversary') || lower.includes('collector')) {
    return 4;
  } else if (lower.includes('premium') || lower.includes('gold')) {
    return 3;
  } else if (lower.includes('deluxe')) {
    return 2;
  } else if (lower.includes('enhanced') || lower.includes('special') || lower.includes('day one')) {
    return 1;
  }
  return 0; // Standard / Base
}

  window.setLibraryData = (gamesList) => {
    if (Array.isArray(gamesList) && gamesList.length > 0) {
      const uniqueMap = new Map();

      gamesList.forEach(g => {
        if (!g.title) return;
        g.licenseType = g.licenseType || g.license_type || 'owned';
        g.license_type = g.licenseType;
        g.productId = g.productId || g.product_id || g.id || '';
        g.product_id = g.productId;
        g.cloudSynced = g.cloudSynced !== undefined ? g.cloudSynced : (g.cloud_synced !== undefined ? g.cloud_synced : true);
        g.cloud_synced = g.cloudSynced;
        g.lastPlayed = g.lastPlayed || g.last_played || 'Recent';
        g.last_played = g.lastPlayed;

        const titleLower = g.title.toLowerCase().trim();
        if (titleLower === 'gamesave' || titleLower === 'wgs' || titleLower === 'msixvc' || titleLower.startsWith('.')) return;

        let norm = titleLower
          .replace(/ - windows/g, '')
          .replace(/ \(windows\)/g, '')
          .replace(/ - pc/g, '')
          .replace(/ \(pc\)/g, '')
          .replace(/ - xbox series x\|s/g, '')
          .replace(/ - xbox one/g, '')
          .replace(/ windows 10 edition/g, '')
          .replace(/ windows edition/g, '')
          .replace(/: 2026 edition/g, '')
          .replace(/: 2025 edition/g, '')
          .replace(/: 2024 edition/g, '')
          .replace(/ standard edition/g, '')
          .replace(/ digital edition/g, '')
          .trim();

        if (g.title === 'Brotato' && window.BROTATO_COVER) {
          g.cover = window.BROTATO_COVER;
          g.splash = window.BROTATO_SPLASH || window.BROTATO_COVER;
        } else if (g.title === 'Sea of Thieves' && window.SOT_COVER) {
          g.cover = window.SOT_COVER;
          g.splash = window.SOT_SPLASH || window.SOT_COVER;
        }

        if (uniqueMap.has(norm)) {
          const existing = uniqueMap.get(norm);
          const existingTier = getEditionTier(existing.title);
          const gTier = getEditionTier(g.title);

          const isGInstalled = g.installed;
          const isExistingInstalled = existing.installed;

          if (existing.licenseType === 'owned' && g.licenseType === 'gamepass') {
            if (state.hasGamePassSubscription && gTier > existingTier) {
              // Game Pass has higher edition tier (e.g. Deluxe vs Standard) and user has active Game Pass
              const wasInstalled = isExistingInstalled || isGInstalled;
              const path = isGInstalled ? g.path : existing.path;
              Object.assign(existing, g);
              existing.installed = wasInstalled;
              existing.path = path;
            } else {
              // Otherwise, always prefer owned license!
              if (isGInstalled) { existing.installed = true; existing.path = g.path; }
            }
          } else if (existing.licenseType === 'gamepass' && g.licenseType === 'owned') {
            if (state.hasGamePassSubscription && existingTier > gTier) {
              // Game Pass in map has higher tier edition than owned
              if (isGInstalled) { existing.installed = true; existing.path = g.path; }
            } else {
              // Otherwise, prefer owned license!
              const wasInstalled = isExistingInstalled || isGInstalled;
              const path = isGInstalled ? g.path : existing.path;
              Object.assign(existing, g);
              existing.licenseType = 'owned';
              existing.installed = wasInstalled;
              existing.path = path;
            }
          } else {
            // Both owned or both gamepass
            if (gTier > existingTier || (!existing.installed && g.installed)) {
              const wasOwned = existing.licenseType === 'owned' || g.licenseType === 'owned';
              Object.assign(existing, g);
              if (wasOwned) existing.licenseType = 'owned';
            }
          }

          if (g.productId && g.productId.length === 12 && (!existing.productId || existing.productId.length !== 12)) {
            existing.productId = g.productId;
            existing.id = g.productId;
          }
          if ((!existing.developer || existing.developer === 'Xbox Game Studios' || existing.developer === 'Local Game Container' || existing.developer === 'Local Game') && g.developer) {
            existing.developer = g.developer;
          }
          if (existing.cover && existing.cover.includes('library_600x900.jpg') && g.cover && !g.cover.includes('library_600x900.jpg')) {
            existing.cover = g.cover;
          }
        } else {
          uniqueMap.set(norm, g);
        }
      });

      state.games = Array.from(uniqueMap.values());
      renderGames();
      if (state.games.length > 0) {
        updateHeroBanner(state.games.find(g => g.installed) || state.games[0]);
      }
      showToast(`Synchronized ${state.games.length} titles from Microsoft Collections & Game Pass`);
    }
  };

  window.updateDownloadProgress = (title, percent, speed) => {
    showProgress(`Downloading ${title} via MSIXVC...`, percent, speed || '32.4 MB/s');
  };

  window.onInstallError = (title, msg) => {
    hideProgress();
    showToast(msg || `Failed to install ${title}`);
    const game = state.games.find(g => g.title === title || g.title.toLowerCase().includes(title.toLowerCase()));
    if (game) {
      game.installed = false;
      renderGames();
      updateHeroBanner(game);
    }
  };

  window.onInstallComplete = (title, path) => {
    hideProgress();
    showToast(`${title} installed and verified ready to play!`);
    const game = state.games.find(g => g.title === title || g.title.toLowerCase().includes(title.toLowerCase()));
    if (game) {
      game.installed = true;
      if (path) game.path = path;
      renderGames();
      renderSidebarInstalled();
      updateHeroBanner(game);
    } else {
      renderGames();
      renderSidebarInstalled();
    }
  };

  window.onUninstallComplete = (title, path) => {
    hideProgress();
    showToast(`Successfully uninstalled ${title}`);
    const game = state.games.find(g => g.title === title || g.path === path || (path && g.path && g.path.includes(path)) || g.title.toLowerCase().includes(title.toLowerCase()));
    if (game) {
      game.installed = false;
      game.size = 'Uninstalled';
      renderGames();
      renderSidebarInstalled();
      updateHeroBanner(game);
    } else {
      renderGames();
      renderSidebarInstalled();
    }
  };

  window.markAllSavesSynced = () => {
    state.games.forEach(g => {
      if (g.installed) g.cloudSynced = true;
    });
    showToast('Auto-synced Xbox Live cloud saves for all installed games');
  };

  window.setFriendsData = (friendsList) => {
    if (Array.isArray(friendsList)) {
      if (friendsList.length === 0) {
        state.friends = [];
      } else {
        state.friends = friendsList.map(f => {
          let stateStr = 'Offline';
          let richPresence = f.presenceText || (f.presence_text || 'Offline');
          let canJoin = false;
          let gameTitle = '';
          let gameId = '';

          if (f.presenceState === 'Online' || f.presence_state === 'Online') {
            stateStr = 'Online';
          }

          const details = f.presenceDetails || f.presence_details || [];
          if (details.length > 0) {
            const d = details[0];
            if (d.titleName || d.title_name) {
              stateStr = 'In-Game';
              gameTitle = d.titleName || d.title_name;
              gameId = d.titleId || d.title_id || '';
              richPresence = `Playing ${gameTitle}`;
              canJoin = true;
            }
          }

          return {
            xuid: f.xuid,
            gamertag: f.gamertag || f.Gamertag || 'Xbox Friend',
            avatar: f.displayPicRaw || f.display_pic_raw || 'https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?w=128&auto=format&fit=crop&q=80',
            state: stateStr,
            richPresence: richPresence,
            gameTitle: gameTitle,
            gameId: gameId,
            canJoin: canJoin
          };
        });
      }
      renderFriends();
    }
  };
}

