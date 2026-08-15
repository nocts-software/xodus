// Xodus GUI Client Application Logic

const state = {
  activeTab: 'library',
  filter: 'all',
  searchQuery: '',
  hasGamePassSubscription: false,
  gamePassTier: null,
  user: {
    gamertag: 'Xbox Player',
    puid: '',
    presence: 'Active',
    gamerscore: '0',
    avatar: 'https://assets.xboxservices.com/assets/default_avatar.png',
    hasGamePass: false,
    subscriptionTier: null,
  },
  games: [],
  friends: []
};

// Initialize Application
document.addEventListener('DOMContentLoaded', () => {
  setupIPCBridge();
  setupNavigation();
  setupWindowControls();
  setupCustomDropdowns();
  setupSearchAndFilters();
  renderUser();
  updateGamePassVisibility();
  renderGames();
  renderSaves();
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
});


function setupCustomDropdowns() {
  setupSingleDropdown('presenceDropdown', 'presenceTrigger', (value) => {
    updatePresence(value);
    const dot = document.getElementById('presenceDot');
    const text = document.getElementById('presenceCurrentText');
    const avatarBadge = document.getElementById('userPresenceBadge');

    if (text) {
      text.textContent = value === 'Active' ? 'Online' : (value === 'Away' ? 'Away' : 'Invisible');
    }
    if (dot) {
      dot.className = `status-indicator-dot dot-${value === 'Active' ? 'online' : (value === 'Away' ? 'away' : 'invisible')}`;
    }
    if (avatarBadge) {
      avatarBadge.className = `presence-badge ${value === 'Active' ? 'online' : (value === 'Away' ? 'away' : 'offline')}`;
    }
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
  else if (tabId === 'saves') renderSaves();
  else if (tabId === 'library') renderGames();
}

// User Rendering
function renderUser() {
  const nameEl = document.getElementById('userGamertag');
  if (nameEl) nameEl.textContent = state.user.gamertag;
  const avatarEl = document.getElementById('userAvatar');
  if (avatarEl) avatarEl.src = state.user.avatar;
  const badge = document.getElementById('userPresenceBadge');
  if (badge) badge.className = `presence-badge ${state.user.presence.toLowerCase()}`;
  const scoreEl = document.getElementById('userScoreText');
  if (scoreEl) {
    const scoreVal = parseInt(state.user.gamerscore, 10);
    scoreEl.textContent = isNaN(scoreVal) ? state.user.gamerscore : scoreVal.toLocaleString();
  }

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

  if (titleEl) titleEl.textContent = game.title;
  if (descEl) descEl.textContent = `${game.developer} • ${game.size} • ${game.installed ? 'Installed Local Container' : 'Cloud Entitled'}`;
  if (badgeEl) badgeEl.textContent = game.installed ? 'JUST PLAYED • READY TO PLAY' : (game.licenseType === 'gamepass' ? 'INCLUDED WITH GAME PASS' : 'OWNED LICENSE');
  if (bgImg) {
    bgImg.src = game.splash || game.cover;
  }


  if (actionsEl) {
    if (game.installed) {
      actionsEl.innerHTML = `
        <button class="btn btn-primary btn-lg" onclick="launchGame('${game.title}', '${game.path}')">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
            <polygon points="5 3 19 12 5 21 5 3"></polygon>
          </svg>
          <span>Play</span>
        </button>
        <button class="btn btn-secondary btn-lg" onclick="syncGameSaves('${game.path}')">
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/>
          </svg>
          <span>Sync Saves</span>
        </button>
      `;
    } else {
      actionsEl.innerHTML = `
        <button class="btn btn-primary btn-lg" onclick="installGame('${game.title}', '${game.path}')">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          <span>Install Game</span>
        </button>
      `;
    }
  }
}

// Games Grid Rendering
function renderGames() {
  const grid = document.getElementById('gamesGrid');
  if (!grid) return;
  grid.innerHTML = '';

  const filtered = state.games.filter(game => {
    const matchesSearch = !state.searchQuery ||
      game.title.toLowerCase().includes(state.searchQuery) ||
      game.developer.toLowerCase().includes(state.searchQuery) ||
      game.productId.toLowerCase().includes(state.searchQuery);

    if (!matchesSearch) return false;

    if (state.filter === 'installed') return game.installed;
    if (state.filter === 'gamepass') return game.licenseType === 'gamepass';
    if (state.filter === 'owned') return game.licenseType === 'owned';

    if (state.hasGamePassSubscription === false && state.activeTab === 'library') {
      return game.installed || game.licenseType === 'owned';
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

    let actionBtnHtml = `<button class="btn btn-secondary btn-sm" style="flex: 1;" onclick="installGame('${game.title}', '${game.path}')">Install</button>`;
    if (game.installed) {
      actionBtnHtml = `<button class="btn btn-primary btn-sm" style="flex: 1;" onclick="launchGame('${game.title}', '${game.path}')">Play</button>`;
    } else if (game.licenseType === 'gamepass' && state.hasGamePassSubscription === false) {
      actionBtnHtml = `<button class="btn btn-secondary btn-sm" style="flex: 1; opacity: 0.7;" onclick="showToast('Active PC Game Pass subscription required to install this title')">Join Game Pass</button>`;
    }

    card.innerHTML = `
      <div class="game-card-cover">
        <img src="${game.cover}" alt="${game.title}" loading="lazy">
        <span class="game-card-badge ${badgeClass}">${badgeText}</span>
      </div>
      <div class="game-card-info">
        <span class="game-card-title">${game.title}</span>
        <div class="game-card-meta">
          <span>${game.developer}</span>
          <span>${game.size}</span>
        </div>
        <div class="game-card-actions">
          ${actionBtnHtml}
          <button class="btn btn-secondary btn-sm" onclick="syncGameSaves('${game.path}')" title="Sync Saves">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/>
            </svg>
          </button>
        </div>
      </div>
    `;
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

  state.friends.forEach(f => {
    const card = document.createElement('div');
    card.className = 'friend-card';
    const st = (f.state || 'offline').toLowerCase();
    const isIngame = st === 'in-game' || st === 'ingame';
    const isOnline = st === 'online' || st === 'active' || st === 'away';
    const badgeClass = isIngame ? 'online' : (isOnline ? (st === 'away' ? 'away' : 'online') : 'offline');

    card.innerHTML = `
      <div class="friend-main">
        <div class="friend-avatar">
          <img src="${f.avatar}" alt="${f.gamertag}">
          <span class="presence-badge ${badgeClass}"></span>
        </div>
        <div class="friend-details">
          <span class="friend-gamertag">${f.gamertag}</span>
          <span class="friend-presence ${isIngame ? 'in-game' : ''}">${f.richPresence || (isOnline ? 'Online' : 'Offline')}</span>
        </div>
      </div>
      ${f.canJoin ? `<button class="btn btn-primary btn-sm" onclick="joinFriendGame('${f.gamertag}', '${f.gameTitle}')">Join Game</button>` : ''}
    `;

    if (isIngame && inGameList) {
      inGameList.appendChild(card);
      inGameCount++;
    } else if (isOnline && onlineList) {
      onlineList.appendChild(card);
      onlineCount++;
    } else if (offlineList) {
      offlineList.appendChild(card);
    }
  });

  if (inGameList && inGameCount === 0) {
    inGameList.innerHTML = '<div class="friends-empty-hint">No friends currently playing games</div>';
  }
  if (onlineList && onlineCount === 0) {
    onlineList.innerHTML = '<div class="friends-empty-hint">No friends currently online</div>';
  }
  if (offlineList && offlineList.children.length === 0) {
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
  modal.dataset.path = path;
  modal.style.display = 'flex';
};

window.resolveSaveConflict = function(choice) {
  const modal = document.getElementById('cloudSyncModal');
  const path = modal.dataset.path;
  modal.style.display = 'none';
  
  showToast(choice === 'cloud' ? 'Downloading cloud save & launching...' : 'Uploading local save & launching...');
  sendNativeCommand({ cmd: 'resolve_save_conflict', path: path, choice: choice });
};

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

function updatePresence(state) {
  showToast(`Presence status updated to: ${state}`);
  sendNativeCommand({ cmd: 'set_presence', state: state });
}

function refreshUserLicenses() {
  showToast('Synchronizing Microsoft Collections & Game Pass catalog...');
  sendNativeCommand({ cmd: 'sync_licenses' });
}

function refreshFriends() {
  showToast('Updating friends presence...');
  sendNativeCommand({ cmd: 'get_friends' });
}

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
  if (window.ipc && typeof window.ipc.postMessage === 'function') {
    window.ipc.postMessage(JSON.stringify(payload));
  } else {
    console.log('[Native IPC]', payload);
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
      renderSaves();
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
      updateHeroBanner(game);
    }
  };

  window.markAllSavesSynced = () => {
    state.games.forEach(g => {
      if (g.installed) g.cloudSynced = true;
    });
    renderSaves();
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

