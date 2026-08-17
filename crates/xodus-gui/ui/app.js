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

document.addEventListener('click', function(e) {
  const btn = e.target.closest('button') || e.target.closest('.pill') || e.target.closest('.nav-item') || e.target.closest('.game-card') || e.target.closest('.dropdown-trigger') || e.target.closest('.dropdown-item');
  if (btn) {
    console.log('[CLICK]', btn.tagName, btn.id || btn.className);
  }
}, true);

const state = {
  activeTab: 'library',
  filter: 'all',
  searchQuery: '',
  storagePath: safeGetStorage('xodus_storage_path', '~/Games'),
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
window.runningGames = window.runningGames || {};

// Initialize Application
function initApp() {
  setupNavigation();
  setupWindowControls();
  setupCustomDropdowns();
  setupSearchAndFilters();
  setupSettingsControls();
  setupDownloadControls();
  renderUser();
  updateGamePassVisibility();
  renderGames();
  renderFriends();


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

function setupSettingsControls() {
  const storageInput = document.getElementById('storagePathInput');
  if (storageInput) {
    storageInput.value = state.storagePath || '~/Games';
    const handleUpdate = () => {
      const newPath = storageInput.value.trim() || '~/Games';
      if (newPath !== state.storagePath) {
        state.storagePath = newPath;
        safeSetStorage('xodus_storage_path', newPath);
        const statusDest = document.getElementById('statusDestPath');
        if (statusDest) statusDest.textContent = newPath;
        const installModalPath = document.getElementById('installModalPath');
        if (installModalPath && state.pendingInstall) {
          const sanitizedTitle = (state.pendingInstall.title || '').replace(/[\\/:*?"<>|]/g, '').trim();
          installModalPath.textContent = `${newPath}/${sanitizedTitle}`;
        }
        sendNativeCommand({ cmd: 'set_storage_path', path: newPath });
        showToast(`Default storage path updated to ${newPath}`);
      }
    };
    storageInput.addEventListener('change', handleUpdate);
    storageInput.addEventListener('blur', handleUpdate);
  }
}

window.setStoragePath = function(path) {
  if (!path) return;
  state.storagePath = path;
  safeSetStorage('xodus_storage_path', path);
  const storageInput = document.getElementById('storagePathInput');
  if (storageInput && document.activeElement !== storageInput) {
    storageInput.value = path;
  }
  const statusDest = document.getElementById('statusDestPath');
  if (statusDest) statusDest.textContent = path;
};

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
    const isRunning = window.runningGames[game.path] === true;
    const item = document.createElement('div');
    item.className = 'sidebar-installed-item';
    item.title = game.title;
    item.innerHTML = `
      <img class="sidebar-installed-icon" src="${game.cover}" alt="${game.title}">
      <span class="sidebar-installed-name">${game.title}</span>
      <button class="sidebar-installed-play ${isRunning ? 'text-danger' : ''}" title="${isRunning ? 'Stop' : 'Play'} ${game.title}">
        ${isRunning ? `
        <svg viewBox="0 0 24 24" width="10" height="10" fill="currentColor">
          <rect x="6" y="6" width="12" height="12"></rect>
        </svg>` : `
        <svg viewBox="0 0 24 24" width="10" height="10" fill="currentColor">
          <polygon points="5 3 19 12 5 21 5 3"></polygon>
        </svg>`}
      </button>
    `;
    item.addEventListener('click', (e) => {
      if (e.target.closest('.sidebar-installed-play')) {
        launchGame(game.title, game.path);
      } else {
        switchTab('library');
        window.showGameDetailsModal(game);
      }
    });
    container.appendChild(item);
  });
}

// Games Grid Rendering
function renderGames() {
  const grid = document.getElementById('gamesGrid');
  if (!grid) return;
  grid.innerHTML = '';

  const filtered = state.games.filter(game => {
    if (!game) return false;
    const title = (game.title || '').toLowerCase();

    // Filter out Xbox console-only games unless locally installed on disk
    if (
      title.includes(' - xbox one') ||
      title.includes(' (xbox one)') ||
      title.includes(' - xbox series') ||
      title.includes(' (xbox series') ||
      title.includes(' - xbox 360') ||
      title.includes(' (xbox 360)') ||
      title.includes(' (xbox)') ||
      (title.includes(' - xbox') && !title.includes('windows') && !title.includes('pc'))
    ) {
      if (!game.installed) return false;
    }

    // Filter out Game Pass games if user has no Game Pass subscription
    const hasGP = !!state.hasGamePassSubscription || !!(state.user && state.user.hasGamePass);
    if (!hasGP && game.licenseType === 'gamepass' && !game.installed) {
      return false;
    }

    const q = state.searchQuery || '';
    const dev = (game.developer || '').toLowerCase();
    const pid = (game.productId || game.id || '').toLowerCase();
    const matchesSearch = !q || title.includes(q) || dev.includes(q) || pid.includes(q);

    if (!matchesSearch) return false;

    if (state.filter === 'installed') return !!game.installed;
    if (state.filter === 'gamepass') return game.licenseType === 'gamepass';
    if (state.filter === 'owned') return game.licenseType === 'owned';

    return true;
  });

  // Always show installed games first, followed by alphabetical order
  filtered.sort((a, b) => {
    if (a.installed && !b.installed) return -1;
    if (!a.installed && b.installed) return 1;
    return (a.title || '').localeCompare(b.title || '');
  });

  filtered.forEach(game => {
    const card = document.createElement('div');
    card.className = 'game-card';
    card.addEventListener('click', (e) => {
      if (!e.target.closest('button')) {

        window.showGameDetailsModal(game);
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
    const basePath = state.storagePath || '~/Games';
    const gamePath = game.path || `${basePath}/${game.productId || game.id || ''}`;

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
      playBtn.className = window.runningGames[gamePath] === true ? 'btn btn-danger btn-sm' : 'btn btn-primary btn-sm';
      playBtn.style.flex = '1';
      playBtn.textContent = window.runningGames[gamePath] === true ? 'Stop' : 'Play';
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
        showInstallModal(game);
      });
      actionsDiv.appendChild(installBtn);
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

window.showGameDetailsModal = function(game) {
  const gameTitle = game.title || 'Untitled';
  const gameDev = game.developer || 'Xbox';
  const gameSize = game.size || 'Standard';
  const basePath = state.storagePath || '~/Games';
  const gamePath = game.path || `${basePath}/${game.productId || game.id || ''}`;
  const isInstalled = !!game.installed;
  const isRunning = window.runningGames[gamePath] === true;
  
  window.currentPopupGame = game;

  const modal = document.getElementById('gameDetailsModal');
  if (!modal) return;

  const titleEl = document.getElementById('detailsModalTitle');
  if (titleEl) titleEl.textContent = gameTitle;

  const coverEl = document.getElementById('detailsModalCover');
  if (coverEl) coverEl.src = game.cover || 'https://assets.xboxservices.com/assets/default_cover.png';

  const devEl = document.getElementById('detailsModalDev');
  if (devEl) devEl.textContent = gameDev;

  const sizeEl = document.getElementById('detailsModalSize');
  if (sizeEl) sizeEl.textContent = gameSize;

  const statusEl = document.getElementById('detailsModalStatus');
  if (statusEl) {
    if (isInstalled) {
      statusEl.textContent = 'Installed';
      statusEl.style.color = '#107c10'; // Xbox Green
    } else {
      statusEl.textContent = 'Not Installed';
      statusEl.style.color = '#f39c12';
    }
  }

  const actionsEl = document.getElementById('detailsModalActions');
  if (actionsEl) {
    if (isInstalled) {
      actionsEl.innerHTML = `
        <button class="btn ${isRunning ? 'btn-danger' : 'btn-primary'} w-100" onclick="closeGameDetailsModal(); window.launchGame('${gameTitle.replace(/'/g, "\\'")}', '${gamePath.replace(/'/g, "\\'")}')">
          ${isRunning ? 'Stop Game' : 'Play Game'}
        </button>
        <button class="btn btn-secondary w-100" onclick="closeGameDetailsModal(); window.showInstallModal(window.currentPopupGame)">Modify Packages & Add-ons</button>
        <button class="btn btn-secondary w-100" onclick="closeGameDetailsModal(); window.syncGameSaves('${gamePath.replace(/'/g, "\\'")}')">Sync Cloud Saves</button>
        <button class="btn btn-danger w-100" onclick="closeGameDetailsModal(); window.promptUninstallGame('${gameTitle.replace(/'/g, "\\'")}', '${gamePath.replace(/'/g, "\\'")}')">Uninstall</button>
      `;
    } else {
      actionsEl.innerHTML = `
        <button class="btn btn-primary w-100" onclick="closeGameDetailsModal(); window.showInstallModal(window.currentPopupGame)">Install Game</button>
      `;
    }
  }

  modal.style.display = 'flex';
  setTimeout(() => modal.classList.add('visible'), 10);
};

window.closeGameDetailsModal = function() {
  const modal = document.getElementById('gameDetailsModal');
  if (modal) {
    modal.classList.remove('visible');
    setTimeout(() => modal.style.display = 'none', 200);
  }
};

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
  if (window.runningGames[path]) {
    window.showStopModal(title, path);
    return;
  }
  
  const game = state.games.find(g => g.path === path || g.title === title) || {};
  
  const overlay = document.getElementById('loadingOverlay');
  if (overlay) {
    const splashEl = document.getElementById('loadingOverlaySplash');
    if (splashEl) {
      splashEl.style.backgroundImage = `url('${game.splash || game.cover || ''}')`;
    }
    const coverEl = document.getElementById('loadingOverlayCover');
    if (coverEl) {
      coverEl.src = game.cover || 'https://assets.xboxservices.com/assets/default_cover.png';
    }
    const titleEl = document.getElementById('loadingOverlayTitle');
    if (titleEl) {
       titleEl.textContent = title;
    }
    overlay.style.display = 'flex';
    if (window.loadingOverlayTimeout) clearTimeout(window.loadingOverlayTimeout);
    window.loadingOverlayTimeout = setTimeout(() => {
      window.onGameWindowReady();
    }, 15000); // 15 second safety fallback timeout
  }

  showToast(`Checking cloud saves for ${title}...`);
  sendNativeCommand({ cmd: 'launch_game', path: path });
  
  window.runningGames[path] = true;
  updateUIForRunningGame(path, true);
}

window.updateUIForRunningGame = function(path, isRunning) {
  renderSidebarInstalled();
  if (window.currentHeroGame && window.currentHeroGame.path === path) {

  }
};

window.onGameStopped = function(path) {
  delete window.runningGames[path];
  updateUIForRunningGame(path, false);
  window.onGameWindowReady(); // Hide loading overlay just in case it's still up
};

window.onGameWindowReady = function() {
  const overlay = document.getElementById('loadingOverlay');
  if (overlay) {
    overlay.style.display = 'none';
  }
  if (window.loadingOverlayTimeout) {
    clearTimeout(window.loadingOverlayTimeout);
    window.loadingOverlayTimeout = null;
  }
};

window.showStopModal = function(title, path) {
  const modalHtml = `
  <div class="modal fade" id="stopGameModal" tabindex="-1" aria-hidden="true">
    <div class="modal-dialog modal-dialog-centered">
      <div class="modal-content" style="background-color: #2c2c2c; color: white;">
        <div class="modal-header" style="border-bottom: 1px solid #444;">
          <h5 class="modal-title">Stop ${title}?</h5>
          <button type="button" class="btn-close btn-close-white" data-bs-dismiss="modal" aria-label="Close"></button>
        </div>
        <div class="modal-body">
          Are you sure you want to force stop this game? Any unsaved progress may be lost.
        </div>
        <div class="modal-footer" style="border-top: 1px solid #444;">
          <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">Cancel</button>
          <button type="button" class="btn btn-danger" onclick="window.confirmStopGame('${path.replace(/'/g, "\\'")}')">Stop Game</button>
        </div>
      </div>
    </div>
  </div>`;
  
  const oldModal = document.getElementById('stopGameModal');
  if (oldModal) oldModal.remove();
  
  document.body.insertAdjacentHTML('beforeend', modalHtml);
  const stopModal = new bootstrap.Modal(document.getElementById('stopGameModal'));
  stopModal.show();
};

window.confirmStopGame = function(path) {
  sendNativeCommand({ cmd: 'stop_game', path: path });
  const stopModalEl = document.getElementById('stopGameModal');
  if (stopModalEl) {
    const stopModal = bootstrap.Modal.getInstance(stopModalEl);
    if (stopModal) stopModal.hide();
  }
};

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

  showInstallModal(game || { title, path, productId: '' });
}

window.showInstallModal = function(game) {
  const safeTitle = game.title || 'Game';
  const prodId = game.productId || game.id || '';
  const sanitizedTitle = safeTitle.replace(/[\\/:*?"<>|]/g, '').trim();
  const basePath = state.storagePath || '~/Games';
  const defPath = `${basePath}/${sanitizedTitle}`;

  state.pendingInstall = {
    title: safeTitle,
    path: defPath,
    productId: prodId,
    cover: game.cover || 'https://assets.xboxservices.com/assets/default_cover.png',
    developer: game.developer || 'Xbox Game Studios',
    isGameInstalled: !!game.installed,
    packages: []
  };

  const modal = document.getElementById('installModal');
  const titleEl = document.getElementById('installModalTitle');
  const devEl = document.getElementById('installModalDev');
  const coverEl = document.getElementById('installModalCover');
  const sizeEl = document.getElementById('installModalSize');
  const pathEl = document.getElementById('installModalPath');
  const descEl = document.getElementById('installModalDesc');
  const pkgListEl = document.getElementById('installPackageList');
  const btnTextEl = document.getElementById('startInstallBtnText');
  const tagEl = document.getElementById('installModalTag');

  if (titleEl) titleEl.textContent = safeTitle;
  if (devEl) devEl.textContent = `${game.developer || 'Xbox Game Studios'} • Windows PC`;
  if (coverEl) coverEl.src = game.cover || 'https://assets.xboxservices.com/assets/default_cover.png';
  if (tagEl) tagEl.textContent = game.installed ? 'INSTALLED • MANAGE PACKAGES & ADD-ONS' : 'WINDOWS PC • MSIXVC';
  if (sizeEl) {
    sizeEl.textContent = 'Calculating package size...';
    sizeEl.classList.add('size-highlight');
  }
  if (pathEl) pathEl.textContent = defPath;
  if (descEl) descEl.textContent = 'Retrieving game overview and component manifest...';
  if (pkgListEl) pkgListEl.innerHTML = '<div style="color: #a0a0a0; font-size: 13px; padding: 10px 0;">Scanning available MSIXVC packages & DLCs...</div>';
  if (btnTextEl) btnTextEl.textContent = game.installed ? 'Modify Packages' : 'Start Installation';

  if (modal) {
    modal.classList.add('visible');
    modal.style.display = 'flex';
  }

  // Request full Display Catalog info and package breakdown from backend
  sendNativeCommand({
    cmd: 'get_install_details',
    title: safeTitle,
    productId: prodId,
    path: defPath
  });
};

window.onInstallDetailsLoaded = function(details) {
  if (!state.pendingInstall) return;
  state.pendingInstall.details = details;
  state.pendingInstall.packages = (details.packages || []).map(p => ({ ...p }));

  const titleEl = document.getElementById('installModalTitle');
  const devEl = document.getElementById('installModalDev');
  const coverEl = document.getElementById('installModalCover');
  const descEl = document.getElementById('installModalDesc');
  const pathEl = document.getElementById('installModalPath');
  const tagEl = document.getElementById('installModalTag');

  if (titleEl && details.title) titleEl.textContent = details.title;
  if (devEl && details.developer) devEl.textContent = `${details.developer} • Windows PC`;
  if (coverEl && (details.heroImage || details.coverImage)) coverEl.src = details.heroImage || details.coverImage;
  if (descEl && details.description) descEl.textContent = details.description;
  if (pathEl && details.installPath) pathEl.textContent = details.installPath;
  if (tagEl) tagEl.textContent = (details.isInstalled || state.pendingInstall.isGameInstalled) ? 'INSTALLED • MANAGE PACKAGES & ADD-ONS' : 'WINDOWS PC • MSIXVC';

  renderInstallPackages();
};

function renderInstallPackages() {
  const pkgListEl = document.getElementById('installPackageList');
  const sizeEl = document.getElementById('installModalSize');
  const btnTextEl = document.getElementById('startInstallBtnText');
  if (!pkgListEl || !state.pendingInstall || !state.pendingInstall.packages) return;

  const isGameInstalled = !!state.pendingInstall.details?.isInstalled || !!state.pendingInstall.isGameInstalled;

  pkgListEl.innerHTML = '';
  let totalBytes = 0;

  state.pendingInstall.packages.forEach((pkg, index) => {
    if (pkg.selected) {
      totalBytes += (pkg.sizeBytes || 0);
    }

    const isPkgInstalled = !!pkg.installed;
    let tagHtml = '';
    if (pkg.required) {
      tagHtml = '<span class="package-tag-required">Required</span>';
    } else if (isPkgInstalled) {
      tagHtml = '<span class="package-tag-installed">Installed</span>';
    } else {
      tagHtml = '<span class="package-tag-addon">Add-on</span>';
    }

    const item = document.createElement('div');
    item.className = `package-item ${pkg.required ? 'disabled' : ''} ${isPkgInstalled ? 'is-installed-item' : ''}`;
    item.innerHTML = `
      <label class="package-item-left" style="cursor: ${pkg.required ? 'default' : 'pointer'};">
        <input type="checkbox" class="pkg-checkbox" ${pkg.selected ? 'checked' : ''} ${pkg.required ? 'disabled' : ''} data-index="${index}">
        <span>${pkg.name || 'Component'}</span>
        ${tagHtml}
      </label>
      <span class="package-item-size">${pkg.sizeFormatted || 'Standard'}</span>
    `;

    const checkbox = item.querySelector('.pkg-checkbox');
    if (checkbox && !pkg.required) {
      checkbox.addEventListener('change', (e) => {
        state.pendingInstall.packages[index].selected = e.target.checked;
        renderInstallPackages();
      });
    }

    pkgListEl.appendChild(item);
  });

  const formattedTotal = formatBytesJS(totalBytes);
  const finalDisplaySize = totalBytes > 0 ? formattedTotal : (state.pendingInstall.details?.totalSizeFormatted || 'Standard (~15-45 GB)');
  const isResume = state.pendingInstall.details?.isResume;
  const existingFormatted = state.pendingInstall.details?.existingFormatted || '0 B';
  const existingBytes = state.pendingInstall.details?.existingBytes || 0;

  if (sizeEl) {
    if (isGameInstalled) {
      const newlyChecked = state.pendingInstall.packages.filter(p => p.selected && !p.installed);
      const addedBytes = newlyChecked.reduce((acc, p) => acc + (p.sizeBytes || 0), 0);
      if (newlyChecked.length > 0) {
        sizeEl.innerHTML = `${finalDisplaySize} <span style="font-size: 11px; color: #52b788; font-weight: 600; margin-left: 6px;">(+${formatBytesJS(addedBytes)} to add)</span>`;
      } else {
        sizeEl.innerHTML = `${finalDisplaySize} <span style="font-size: 11px; color: #52b788; font-weight: 600; margin-left: 6px;">(Installed)</span>`;
      }
    } else if (isResume && totalBytes > 0) {
      const remainingBytes = Math.max(0, totalBytes - existingBytes);
      sizeEl.innerHTML = `${finalDisplaySize} <span style="font-size: 11px; color: #52b788; font-weight: 600; margin-left: 6px;">(${existingFormatted} on disk, ${formatBytesJS(remainingBytes)} remaining)</span>`;
    } else {
      sizeEl.textContent = finalDisplaySize;
    }
    sizeEl.classList.add('size-highlight');
  }

  if (btnTextEl) {
    if (isGameInstalled) {
      const newlyChecked = state.pendingInstall.packages.filter(p => p.selected && !p.installed);
      const addedBytes = newlyChecked.reduce((acc, p) => acc + (p.sizeBytes || 0), 0);
      if (newlyChecked.length > 0) {
        btnTextEl.textContent = `Install Selected Add-ons (+${formatBytesJS(addedBytes)})`;
      } else {
        btnTextEl.textContent = 'All Selected Packages Installed (Close)';
      }
    } else if (isResume) {
      const remainingBytes = Math.max(0, totalBytes - existingBytes);
      btnTextEl.textContent = remainingBytes > 0 ? `Resume Download (${formatBytesJS(remainingBytes)} remaining)` : 'Verify & Resume Download';
    } else {
      btnTextEl.textContent = totalBytes > 0 ? `Start Installation (${formattedTotal})` : 'Start Installation';
    }
  }
}

function formatBytesJS(bytes) {
  if (!bytes || bytes <= 0) return 'Standard (~15-45 GB)';
  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(2)} GB`;
  const mb = bytes / (1024 * 1024);
  if (mb >= 1) return `${mb.toFixed(1)} MB`;
  return `${(bytes / 1024).toFixed(0)} KB`;
}

window.closeInstallModal = function() {
  const modal = document.getElementById('installModal');
  if (modal) {
    modal.classList.remove('visible');
    modal.style.display = 'none';
  }
  state.pendingInstall = null;
};

window.confirmStartInstall = function() {
  if (!state.pendingInstall) return;
  const { title, path, productId, packages, isGameInstalled } = state.pendingInstall;
  const selectedPackages = (packages || []).filter(p => p.selected).map(p => p.id);
  const newlyChecked = (packages || []).filter(p => p.selected && !p.installed).map(p => p.id);

  if (isGameInstalled && newlyChecked.length === 0) {
    closeInstallModal();
    return;
  }

  closeInstallModal();

  if (isGameInstalled && newlyChecked.length > 0) {
    state.activeDownload = {
      title: title,
      productId: productId,
      path: path,
      selectedPackages: newlyChecked,
      isPaused: false,
      percent: 5,
    };
    showToast(`Downloading and installing ${newlyChecked.length} new component(s) for ${title}...`);
    showProgress(`Adding components for ${title}...`, 5, 'Connecting');
    sendNativeCommand({
      cmd: 'install_game',
      title: title,
      path: path,
      productId: productId,
      selectedPackages: newlyChecked
    });
    return;
  }

  state.activeDownload = {
    title: title,
    productId: productId,
    path: path,
    selectedPackages: selectedPackages,
    isPaused: false,
    percent: 5,
  };
  showToast(`Connecting to Microsoft Delivery Optimization for ${title}...`);
  showProgress(`Connecting to Microsoft Delivery Optimization...`, 5, 'Connecting');
  sendNativeCommand({
    cmd: 'install_game',
    title: title,
    productId: productId,
    path: path,
    selectedPackages: selectedPackages
  });
};


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

function formatEta(seconds) {
  if (!seconds || seconds <= 0 || !isFinite(seconds)) return 'Estimating time...';
  if (seconds < 60) return `${Math.round(seconds)}s remaining`;
  const mins = Math.floor(seconds / 60);
  const secs = Math.round(seconds % 60);
  if (mins < 60) return `${mins}m ${secs}s remaining`;
  const hours = Math.floor(mins / 60);
  const remMins = mins % 60;
  return `${hours}h ${remMins}m remaining`;
}

function formatSpeed(bytesPerSec) {
  if (!bytesPerSec || bytesPerSec <= 0 || !isFinite(bytesPerSec)) return '-- MB/s';
  const mb = bytesPerSec / (1024 * 1024);
  if (mb >= 1) return `${mb.toFixed(1)} MB/s`;
  const kb = bytesPerSec / 1024;
  return `${kb.toFixed(0)} KB/s`;
}

function formatBytesJS(bytes) {
  if (!bytes || bytes <= 0 || !isFinite(bytes)) return '0 B';
  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(2)} GB`;
  const mb = bytes / (1024 * 1024);
  if (mb >= 1) return `${mb.toFixed(1)} MB`;
  const kb = bytes / 1024;
  return `${kb.toFixed(0)} KB`;
}

// Download Pause / Resume / Cancel Controls
function setupDownloadControls() {
  const pauseBtn = document.getElementById('statusPauseBtn');
  if (pauseBtn) {
    pauseBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      if (state.activeDownload && state.activeDownload.isPaused) {
        resumeDownload();
      } else {
        pauseDownload();
      }
    });
  }

  const cancelBtn = document.getElementById('statusCancelBtn');
  if (cancelBtn) {
    cancelBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      cancelDownload();
    });
  }
}

function updateStatusControls() {
  const pauseBtn = document.getElementById('statusPauseBtn');
  const iconPause = pauseBtn ? pauseBtn.querySelector('.icon-pause') : null;
  const iconResume = pauseBtn ? pauseBtn.querySelector('.icon-resume') : null;
  const stageEl = document.getElementById('statusStage');
  const fillEl = document.getElementById('progressBarFill');
  const iconBox = document.getElementById('statusIconBox');

  const isPaused = !!(state.activeDownload && state.activeDownload.isPaused);

  if (pauseBtn) {
    if (isPaused) {
      pauseBtn.classList.add('is-paused');
      pauseBtn.title = 'Resume Download';
      pauseBtn.setAttribute('aria-label', 'Resume Download');
      if (iconPause) iconPause.style.display = 'none';
      if (iconResume) iconResume.style.display = 'block';
    } else {
      pauseBtn.classList.remove('is-paused');
      pauseBtn.title = 'Pause Download';
      pauseBtn.setAttribute('aria-label', 'Pause Download');
      if (iconPause) iconPause.style.display = 'block';
      if (iconResume) iconResume.style.display = 'none';
    }
  }

  if (stageEl) {
    if (isPaused) {
      stageEl.classList.add('is-paused');
      stageEl.textContent = 'PAUSED';
    } else {
      stageEl.classList.remove('is-paused');
    }
  }

  if (fillEl) {
    if (isPaused) {
      fillEl.classList.add('is-paused');
    } else {
      fillEl.classList.remove('is-paused');
    }
  }

  if (iconBox) {
    if (isPaused) {
      iconBox.classList.add('is-paused');
    } else {
      iconBox.classList.remove('is-paused');
    }
  }
}

function pauseDownload() {
  if (!state.activeDownload || !state.activeDownload.title) return;
  state.activeDownload.isPaused = true;
  updateStatusControls();

  const speedEl = document.getElementById('statusSpeed');
  if (speedEl) speedEl.textContent = '-- MB/s';
  const etaEl = document.getElementById('statusEta');
  if (etaEl) etaEl.textContent = 'Paused';

  sendNativeCommand({
    cmd: 'pause_download',
    title: state.activeDownload.title
  });

  const game = state.games.find(g => g.title === state.activeDownload.title || g.title.toLowerCase().includes(state.activeDownload.title.toLowerCase()));
  if (game) {
    game.downloadPaused = true;
    renderGames();

  }

  showToast(`Download paused for ${state.activeDownload.title}`);
}
window.pauseDownload = pauseDownload;

function resumeDownload() {
  if (!state.activeDownload || !state.activeDownload.title) return;
  state.activeDownload.isPaused = false;
  updateStatusControls();

  const stageEl = document.getElementById('statusStage');
  if (stageEl) stageEl.textContent = 'Resuming';
  const etaEl = document.getElementById('statusEta');
  if (etaEl) etaEl.textContent = 'Connecting...';

  sendNativeCommand({
    cmd: 'resume_download',
    title: state.activeDownload.title,
    productId: state.activeDownload.productId,
    path: state.activeDownload.path,
    selectedPackages: state.activeDownload.selectedPackages
  });

  const game = state.games.find(g => g.title === state.activeDownload.title || g.title.toLowerCase().includes(state.activeDownload.title.toLowerCase()));
  if (game) {
    game.downloadPaused = false;
    renderGames();

  }

  showToast(`Resuming download for ${state.activeDownload.title}...`);
}
window.resumeDownload = resumeDownload;

function cancelDownload() {
  if (!state.activeDownload || !state.activeDownload.title) {
    hideProgress();
    return;
  }
  const title = state.activeDownload.title;

  sendNativeCommand({
    cmd: 'cancel_download',
    title: title
  });

  state.activeDownload = null;
  hideProgress();

  const game = state.games.find(g => g.title === title || g.title.toLowerCase().includes(title.toLowerCase()));
  if (game) {
    game.downloading = false;
    game.downloadPaused = false;
    game.downloadProgress = 0;
    renderGames();
    renderSidebarInstalled();

  }

  showToast(`Download for ${title} canceled`);
}
window.cancelDownload = cancelDownload;

// Progress Bar
function showProgress(title, percent, speedText, stageText, bytesText, etaText, pkgText) {
  const bar = document.getElementById('statusBar');
  if (!bar) return;
  bar.style.display = 'flex';

  const isPaused = !!(state.activeDownload && state.activeDownload.isPaused);

  const titleEl = document.getElementById('statusTitle');
  if (titleEl) titleEl.textContent = title || 'Downloading Game';

  const speedEl = document.getElementById('statusSpeed');
  if (speedEl) speedEl.textContent = isPaused ? '-- MB/s' : (speedText || '-- MB/s');

  const stageEl = document.getElementById('statusStage');
  if (stageEl) stageEl.textContent = isPaused ? 'PAUSED' : (stageText || 'Downloading');

  const pkgBadge = document.getElementById('statusPkgBadge');
  if (pkgBadge) {
    if (pkgText) {
      pkgBadge.textContent = pkgText;
      pkgBadge.style.display = 'inline-block';
    } else {
      pkgBadge.style.display = 'none';
    }
  }

  const bytesEl = document.getElementById('statusBytes');
  if (bytesEl) bytesEl.textContent = bytesText || `${Math.round(percent)}%`;

  const etaEl = document.getElementById('statusEta');
  if (etaEl) etaEl.textContent = isPaused ? 'Paused' : (etaText || 'Estimating time...');

  const percentEl = document.getElementById('statusPercent');
  if (percentEl) percentEl.textContent = `${Math.round(percent)}%`;

  const fillEl = document.getElementById('progressBarFill');
  if (fillEl) fillEl.style.width = `${Math.max(0, Math.min(100, percent))}%`;

  updateStatusControls();
}

function hideProgress() {
  const bar = document.getElementById('statusBar');
  if (bar) bar.style.display = 'none';
  if (state.activeDownload) {
    state.activeDownload = null;
  }
}

function cancelActiveTask() {
  cancelDownload();
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
      state.games.sort((a, b) => {
        if (a.installed && !b.installed) return -1;
        if (!a.installed && b.installed) return 1;
        return (a.title || '').localeCompare(b.title || '');
      });
      renderGames();
      renderSidebarInstalled();
      if (state.games.length > 0) {

      }
      showToast(`Synchronized ${state.games.length} titles from Microsoft Collections`);
    }
  };

  window.updateDownloadProgress = (title, percent, speed) => {
    showProgress(title, percent, speed || '-- MB/s', 'Downloading', `${Math.round(percent)}%`, 'Estimating time...', '');
  };

  window.onDetailedDownloadProgress = (title, progress) => {
    if (!progress) return;
    if (state.activeDownload && state.activeDownload.isPaused) return;

    if (!state.activeDownload) {
      state.activeDownload = {
        title: title,
        isPaused: false
      };
    }
    state.activeDownload.title = title;

    const percent = typeof progress.percent === 'number' ? progress.percent : 0;
    const speed = formatSpeed(progress.speed);
    const eta = formatEta(progress.eta);
    const stage = progress.stage || 'Streaming';
    const bytesFormatted = (progress.total > 0)
      ? `${formatBytesJS(progress.bytes)} / ${formatBytesJS(progress.total)}`
      : `${formatBytesJS(progress.bytes)}`;
    const pkgText = (progress.totalPackages && progress.totalPackages > 1)
      ? `Component ${progress.packageIndex || 1}/${progress.totalPackages}`
      : '';

    showProgress(title, percent, speed, stage, bytesFormatted, eta, pkgText);

    // Update state active download info
    const game = state.games.find(g => g.title === title || g.title.toLowerCase().includes(title.toLowerCase()));
    if (game) {
      game.downloading = true;
      game.downloadPaused = false;
      game.downloadProgress = percent;
      game.downloadSpeed = speed;
      game.downloadEta = eta;
    }
  };

  window.onDownloadPaused = (title) => {
    if (state.activeDownload) {
      state.activeDownload.isPaused = true;
      updateStatusControls();
    }
    const game = state.games.find(g => g.title === title || g.title.toLowerCase().includes(title.toLowerCase()));
    if (game) {
      game.downloadPaused = true;
      renderGames();

    }
  };

  window.onDownloadResumed = (title) => {
    if (state.activeDownload) {
      state.activeDownload.isPaused = false;
      updateStatusControls();
    }
    const game = state.games.find(g => g.title === title || g.title.toLowerCase().includes(title.toLowerCase()));
    if (game) {
      game.downloadPaused = false;
      renderGames();

    }
  };

  window.onDownloadCanceled = (title) => {
    state.activeDownload = null;
    hideProgress();
    const game = state.games.find(g => g.title === title || g.title.toLowerCase().includes(title.toLowerCase()));
    if (game) {
      game.downloading = false;
      game.downloadPaused = false;
      game.downloadProgress = 0;
      renderGames();
      renderSidebarInstalled();

    }
  };

  window.onInstallError = (title, msg) => {
    state.activeDownload = null;
    hideProgress();
    showToast(msg || `Failed to install ${title}`);
    const game = state.games.find(g => g.title === title || g.title.toLowerCase().includes(title.toLowerCase()));
    if (game) {
      game.downloading = false;
      game.downloadPaused = false;
      game.installed = false;
      renderGames();

    }
  };

  window.onInstallComplete = (title, path) => {
    state.activeDownload = null;
    hideProgress();
    showToast(`${title} installed and verified ready to play!`);
    const game = state.games.find(g => g.title === title || g.title.toLowerCase().includes(title.toLowerCase()));
    if (game) {
      game.downloading = false;
      game.downloadPaused = false;
      game.installed = true;
      if (path) game.path = path;
      renderGames();
      renderSidebarInstalled();

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


function openUrl(url) {
  sendNativeCommand({ cmd: 'open_url', url: url });
}
window.openUrl = openUrl;

