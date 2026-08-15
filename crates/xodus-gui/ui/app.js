// Xodus GUI Client Application Logic

const state = {
  activeTab: 'library',
  filter: 'all',
  searchQuery: '',
  user: {
    gamertag: 'nocatix',
    puid: '0003BFFDB416EF4E',
    presence: 'Active',
    gamerscore: '20227',
    avatar: 'https://images-eds-ssl.xboxlive.com/image?url=8Oaj9Ryq1G1_p3lLnXlsaZgGzAie6Mnu24_PawYuDYIoH77pJ.X5Z.MqQPibUVTcS9jr0n8i7LY1tL3U7AiafQlcpGDAiHI1vgxmFGi1m3EKZRqEIJxcDZa.OAt89g5A&format=png',
  },
  games: [
    {
      id: '77BB5AFB',
      productId: '9N44Q5Q49DBC',
      title: 'Brotato',
      developer: 'Blobfish / Seaven Studio',
      licenseType: 'owned',
      installed: true,
      size: '423.2 MB',
      path: '/mnt/w11/XboxGames/Brotato',
      cover: window.BROTATO_COVER || 'https://shared.steamstatic.com/store_item_assets/steam/apps/2042420/library_600x900.jpg',
      splash: window.BROTATO_SPLASH || window.BROTATO_COVER,
      cloudSynced: true,
      lastPlayed: 'Today'
    },
    {
      id: '4F56E789',
      productId: '9PKX8Z3K73NP',
      title: 'Hi-Fi RUSH',
      developer: 'Tango Gameworks / Bethesda',
      licenseType: 'gamepass',
      installed: false,
      size: '14.2 GB',
      path: '/mnt/w11/XboxGames/HiFiRush',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1817230/library_600x900.jpg',
      cloudSynced: false,
      lastPlayed: '3 days ago'
    },
    {
      id: '89ABCDEF',
      productId: '9N6Z4S0B3RST',
      title: 'Vampire Survivors',
      developer: 'poncle',
      licenseType: 'owned',
      installed: false,
      size: '512 MB',
      path: '/mnt/w11/XboxGames/VampireSurvivors',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1794680/library_600x900.jpg',
      cloudSynced: false,
      lastPlayed: '1 week ago'
    },
    {
      id: '9PW1QZCRRP82',
      productId: '9PW1QZCRRP82',
      title: 'Forza Horizon 5',
      developer: 'Playground Games / Xbox Game Studios',
      licenseType: 'gamepass',
      installed: false,
      size: '110.4 GB',
      path: '/mnt/w11/XboxGames/ForzaHorizon5',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1551360/library_600x900.jpg',
      cloudSynced: true,
      lastPlayed: '2 weeks ago'
    },
    {
      id: '9NCK5NRMN521',
      productId: '9NCK5NRMN521',
      title: 'Starfield',
      developer: 'Bethesda Game Studios',
      licenseType: 'gamepass',
      installed: false,
      size: '125.8 GB',
      path: '/mnt/w11/XboxGames/Starfield',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1716740/library_600x900.jpg',
      cloudSynced: true,
      lastPlayed: '1 month ago'
    },
    {
      id: '9PP5G1F0C2B6',
      productId: '9PP5G1F0C2B6',
      title: 'Halo Infinite',
      developer: '343 Industries / Xbox Game Studios',
      licenseType: 'owned',
      installed: false,
      size: '48.5 GB',
      path: '/mnt/w11/XboxGames/HaloInfinite',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1240440/library_600x900.jpg',
      cloudSynced: false,
      lastPlayed: 'Last month'
    },
    {
      id: '9P2N57MC619K',
      productId: '9P2N57MC619K',
      title: 'Sea of Thieves',
      developer: 'Rare Ltd / Xbox Game Studios',
      licenseType: 'gamepass',
      installed: true,
      size: '82.1 GB',
      path: '/mnt/w11/XboxGames/Sea of Thieves',
      cover: window.SOT_COVER || 'https://shared.steamstatic.com/store_item_assets/steam/apps/1172620/library_600x900.jpg',
      splash: window.SOT_SPLASH || window.SOT_COVER,
      cloudSynced: true,
      lastPlayed: '3 weeks ago'
    },
    {
      id: '9NBLGGH2JHXJ',
      productId: '9NBLGGH2JHXJ',
      title: 'Minecraft for Windows',
      developer: 'Mojang Studios / Xbox Game Studios',
      licenseType: 'owned',
      installed: false,
      size: '1.2 GB',
      path: '/mnt/w11/XboxGames/Minecraft',
      cover: 'https://store-images.s-microsoft.com/image/apps.415.13510798885735219.53a3b855-fde7-4304-925c-9db1cd1c34a8.b07e27c9-cdb1-4433-982b-7df0888f871c',
      cloudSynced: true,
      lastPlayed: 'Yesterday'
    },
    {
      id: '9NZ5W0R3W4F5',
      productId: '9NZ5W0R3W4F5',
      title: 'Lies of P',
      developer: 'NEOWIZ / Round8 Studio',
      licenseType: 'gamepass',
      installed: false,
      size: '35.6 GB',
      path: '/mnt/w11/XboxGames/LiesOfP',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1627720/library_600x900.jpg',
      cloudSynced: false,
      lastPlayed: '2 months ago'
    },
    {
      id: '9MZ16G7K0519',
      productId: '9MZ16G7K0519',
      title: 'Persona 3 Reload',
      developer: 'ATLUS / SEGA',
      licenseType: 'gamepass',
      installed: false,
      size: '24.1 GB',
      path: '/mnt/w11/XboxGames/Persona3Reload',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/2161700/library_600x900.jpg',
      cloudSynced: true,
      lastPlayed: '5 days ago'
    },
    {
      id: '9N49NZ9PZ59T',
      productId: '9N49NZ9PZ59T',
      title: 'Palworld',
      developer: 'Pocketpair',
      licenseType: 'gamepass',
      installed: false,
      size: '18.3 GB',
      path: '/mnt/w11/XboxGames/Palworld',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1623730/library_600x900.jpg',
      cloudSynced: false,
      lastPlayed: '3 weeks ago'
    },
    {
      id: '9P5S26314HWQ',
      productId: '9P5S26314HWQ',
      title: 'DOOM Eternal: Standard Edition',
      developer: 'id Software / Bethesda',
      licenseType: 'gamepass',
      installed: false,
      size: '78.4 GB',
      path: '/mnt/w11/XboxGames/DoomEternal',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/782330/library_600x900.jpg',
      cloudSynced: true,
      lastPlayed: '4 months ago'
    },
    {
      id: '9NZ7K1Q5018W',
      productId: '9NZ7K1Q5018W',
      title: 'Microsoft Flight Simulator 2024',
      developer: 'Asobo Studio / Xbox Game Studios',
      licenseType: 'gamepass',
      installed: false,
      size: '50.2 GB',
      path: '/mnt/w11/XboxGames/MSFS2024',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1250410/library_600x900.jpg',
      cloudSynced: false,
      lastPlayed: 'New'
    },
    {
      id: '9P1Z9N5L6F7M',
      productId: '9P1Z9N5L6F7M',
      title: 'Indiana Jones and the Great Circle',
      developer: 'MachineGames / Bethesda',
      licenseType: 'gamepass',
      installed: false,
      size: '88.0 GB',
      path: '/mnt/w11/XboxGames/IndianaJones',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/2677660/library_600x900.jpg',
      cloudSynced: false,
      lastPlayed: 'New'
    },
    {
      id: '9N0B90L0151R',
      productId: '9N0B90L0151R',
      title: 'S.T.A.L.K.E.R. 2: Heart of Chornobyl',
      developer: 'GSC Game World',
      licenseType: 'gamepass',
      installed: false,
      size: '142.5 GB',
      path: '/mnt/w11/XboxGames/Stalker2',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1643320/library_600x900.jpg',
      cloudSynced: false,
      lastPlayed: 'New'
    },
    {
      id: '9P8K317P7V2Z',
      productId: '9P8K317P7V2Z',
      title: 'Gears 5: Game of the Year Edition',
      developer: 'The Coalition / Xbox Game Studios',
      licenseType: 'owned',
      installed: false,
      size: '64.2 GB',
      path: '/mnt/w11/XboxGames/Gears5',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1097840/library_600x900.jpg',
      cloudSynced: true,
      lastPlayed: '6 months ago'
    },
    {
      id: '9N19R5N8N5X3',
      productId: '9N19R5N8N5X3',
      title: 'DEATHLOOP',
      developer: 'Arkane Studios / Bethesda',
      licenseType: 'gamepass',
      installed: false,
      size: '32.0 GB',
      path: '/mnt/w11/XboxGames/Deathloop',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1252330/library_600x900.jpg',
      cloudSynced: false,
      lastPlayed: '2 months ago'
    },
    {
      id: '9P5Z2P8L8L9L',
      productId: '9P5Z2P8L8L9L',
      title: 'Psychonauts 2',
      developer: 'Double Fine Productions',
      licenseType: 'owned',
      installed: false,
      size: '28.5 GB',
      path: '/mnt/w11/XboxGames/Psychonauts2',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/607080/library_600x900.jpg',
      cloudSynced: true,
      lastPlayed: '5 months ago'
    },
    {
      id: '9NX78L88Q51K',
      productId: '9NX78L88Q51K',
      title: 'Age of Empires IV: Anniversary Edition',
      developer: 'Relic Entertainment / World\'s Edge',
      licenseType: 'gamepass',
      installed: false,
      size: '42.8 GB',
      path: '/mnt/w11/XboxGames/AoE4',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/1466860/library_600x900.jpg',
      cloudSynced: true,
      lastPlayed: '1 month ago'
    },
    {
      id: '9P4K39185NWL',
      productId: '9P4K39185NWL',
      title: 'Dead Cells',
      developer: 'Motion Twin',
      licenseType: 'owned',
      installed: false,
      size: '2.1 GB',
      path: '/mnt/w11/XboxGames/DeadCells',
      cover: 'https://shared.steamstatic.com/store_item_assets/steam/apps/588650/library_600x900.jpg',
      cloudSynced: true,
      lastPlayed: '3 weeks ago'
    }
  ],
  friends: [
    {
      xuid: '2533274991823401',
      gamertag: 'ShadowRunner',
      avatar: 'https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?w=128&auto=format&fit=crop&q=80',
      state: 'In-Game',
      richPresence: 'Playing Brotato (Wave 17)',
      gameTitle: 'Brotato',
      gameId: '77BB5AFB',
      canJoin: true
    },
    {
      xuid: '2533274889102345',
      gamertag: 'Valkyrie99',
      avatar: 'https://images.unsplash.com/photo-1580489944761-15a19d654956?w=128&auto=format&fit=crop&q=80',
      state: 'Online',
      richPresence: 'Online • Home Screen',
      canJoin: false
    },
    {
      xuid: '2533274776109923',
      gamertag: 'PixelKnight',
      avatar: 'https://images.unsplash.com/photo-1570295999919-56ceb5ecca61?w=128&auto=format&fit=crop&q=80',
      state: 'Away',
      richPresence: 'Away (15m)',
      canJoin: false
    },
    {
      xuid: '2533274665401129',
      gamertag: 'CyberNova',
      avatar: 'https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?w=128&auto=format&fit=crop&q=80',
      state: 'Offline',
      richPresence: 'Last seen 2h ago',
      canJoin: false
    }
  ]
};

// Initialize Application
document.addEventListener('DOMContentLoaded', () => {
  setupIPCBridge();
  setupNavigation();
  setupWindowControls();
  setupCustomDropdowns();
  setupSearchAndFilters();
  renderUser();
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
          <span>Play In-Place</span>
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
        <button class="btn btn-secondary btn-lg" onclick="showToast('Verifying ${game.title} digital license...')">
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
          </svg>
          <span>License Details</span>
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
  showToast(`Launching ${title} with Proton CachyOS...`);
  sendNativeCommand({ cmd: 'launch_game', path: path });
}

function installGame(title, path) {
  const game = state.games.find(g => g.title === title || g.path === path);
  if (game && game.licenseType === 'gamepass' && state.hasGamePassSubscription === false && !game.installed) {
    showToast('PC Game Pass subscription required to install this title');
    return;
  }

  showToast(`Initiating MSIXVC package download for ${title}...`);
  let progress = 0;
  showProgress(`Connecting to Microsoft Delivery Optimization...`, 5);
  sendNativeCommand({ cmd: 'download_game', title: title, path: path });

  const interval = setInterval(() => {
    progress += Math.floor(Math.random() * 12) + 8;
    if (progress >= 100) {
      clearInterval(interval);
      showProgress(`Finished downloading ${title}`, 100, 'Complete');
      setTimeout(() => {
        hideProgress();
        showToast(`${title} installed and verified ready to play!`);
        if (game) {
          game.installed = true;
          renderGames();
          updateHeroBanner(game);
        }
      }, 1000);
    } else {
      showProgress(`Downloading & Decrypting ${title} via MSIXVC...`, progress, `${(28.4 + Math.random() * 8).toFixed(1)} MB/s`);
    }
  }, 250);
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
      if (profile.display_pic) state.user.avatar = profile.display_pic;
      if (profile.gamerscore) state.user.gamerscore = profile.gamerscore;
      renderUser();
      showToast(`Welcome back, ${state.user.gamertag}!`);
    }
  };

  window.setGamePassStatus = (hasSubscription) => {
    state.hasGamePassSubscription = !!hasSubscription;
    console.log('[XODUS] PC Game Pass Active:', state.hasGamePassSubscription);
    renderGames();
  };

  window.setLibraryData = (gamesList) => {
    if (Array.isArray(gamesList) && gamesList.length > 0) {
      const processed = gamesList.map(g => {
        if (g.title === 'Brotato' && window.BROTATO_COVER) {
          g.cover = window.BROTATO_COVER;
          g.splash = window.BROTATO_SPLASH || window.BROTATO_COVER;
        } else if (g.title === 'Sea of Thieves' && window.SOT_COVER) {
          g.cover = window.SOT_COVER;
          g.splash = window.SOT_SPLASH || window.SOT_COVER;
        }
        return g;
      });

      state.games = processed;
      renderGames();
      renderSaves();
      if (state.games.length > 0) {
        updateHeroBanner(state.games.find(g => g.installed) || state.games[0]);
      }
      showToast(`Synchronized ${state.games.length} titles from Microsoft Collections & Game Pass`);
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
    if (Array.isArray(friendsList) && friendsList.length > 0) {
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
      renderFriends();
    }
  };
}

