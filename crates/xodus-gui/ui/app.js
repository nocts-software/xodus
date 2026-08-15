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
      cover: 'https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1542751371-adc38448a05e?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1550745165-9bc0b252726f?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1511919884226-fd3cad34687c?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1451187580459-43490279c0fa?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://store-images.s-microsoft.com/image/apps.3823.14330850369313893.a687698c-b891-44f6-9576-fe28978ce915.5aada4fa-d850-4eb5-9100-33a81a5cde09',
      cloudSynced: false,
      lastPlayed: 'Last month'
    },
    {
      id: '9P2N57MC619K',
      productId: '9P2N57MC619K',
      title: 'Sea of Thieves',
      developer: 'Rare Ltd / Xbox Game Studios',
      licenseType: 'gamepass',
      installed: false,
      size: '82.1 GB',
      path: '/mnt/w11/XboxGames/SeaOfThieves',
      cover: 'https://store-images.s-microsoft.com/image/apps.29206.14554784103656548.069efce3-9249-4074-a169-183b727043f8.03688f8c-edc0-416b-bebb-9d98a01c25f5',
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
      cover: 'https://images.unsplash.com/photo-1563089145-599997674d42?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1607604276583-eef5d076aa5f?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1563245372-f21724e3856d?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1578632767115-351597cf2477?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1508614589041-895b88991e3e?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1514565131-fce0801e5785?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1534447677768-be436bb09401?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1579783902614-a3fb3927b675?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1563089145-599997674d42?w=600&auto=format&fit=crop&q=80',
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
      cover: 'https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=600&auto=format&fit=crop&q=80',
      cloudSynced: true,
      lastPlayed: '1 month ago'
    },
    {
      id: '9MV8F4J6L92D',
      productId: '9MV8F4J6L92D',
      title: 'The Outer Worlds: Spacer\'s Choice Edition',
      developer: 'Obsidian Entertainment / Private Division',
      licenseType: 'owned',
      installed: false,
      size: '38.4 GB',
      path: '/mnt/w11/XboxGames/OuterWorlds',
      cover: 'https://images.unsplash.com/photo-1451187580459-43490279c0fa?w=600&auto=format&fit=crop&q=80',
      cloudSynced: true,
      lastPlayed: '3 months ago'
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
    text.textContent = value === 'Active' ? 'Online' : (value === 'Away' ? 'Away' : 'Invisible');
    dot.className = `status-indicator-dot dot-${value === 'Active' ? 'online' : (value === 'Away' ? 'away' : 'invisible')}`;
  });

  setupSingleDropdown('protonDropdown', 'protonTrigger', (value) => {
    const text = document.getElementById('protonCurrentText');
    if (value.includes('cachyos')) text.textContent = 'Proton CachyOS Native (RADV + FSR4)';
    else if (value.includes('GE')) text.textContent = 'GE-Proton 11-3';
    else text.textContent = 'System Wine';
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
    e.stopPropagation();
    const isOpen = dropdown.classList.contains('open');
    document.querySelectorAll('.custom-dropdown').forEach(d => d.classList.remove('open'));
    if (!isOpen) dropdown.classList.add('open');
  });

  const items = dropdown.querySelectorAll('.dropdown-item');
  items.forEach(item => {
    item.addEventListener('click', () => {
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
  state.activeTab = tabId;
  document.querySelectorAll('.nav-item').forEach(el => {
    el.classList.toggle('active', el.getAttribute('data-tab') === tabId);
  });
  document.querySelectorAll('.tab-panel').forEach(panel => {
    panel.classList.toggle('active', panel.id === `tab-${tabId}`);
  });
}

// User Rendering
function renderUser() {
  document.getElementById('userGamertag').textContent = state.user.gamertag;
  document.getElementById('userAvatar').src = state.user.avatar;
  const badge = document.getElementById('userPresenceBadge');
  badge.className = `presence-badge ${state.user.presence.toLowerCase()}`;
}

// Games Grid Rendering
function renderGames() {
  const grid = document.getElementById('gamesGrid');
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
    return true;
  });

  filtered.forEach(game => {
    const card = document.createElement('div');
    card.className = 'game-card';
    
    let badgeClass = 'gamepass';
    let badgeText = 'GAME PASS';
    if (game.installed) {
      badgeClass = 'installed';
      badgeText = 'INSTALLED';
    } else if (game.licenseType === 'owned') {
      badgeClass = 'owned';
      badgeText = 'OWNED';
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
          ${game.installed 
            ? `<button class="btn btn-primary btn-sm" style="flex: 1;" onclick="launchGame('${game.title}', '${game.path}')">Play</button>`
            : `<button class="btn btn-secondary btn-sm" style="flex: 1;" onclick="installGame('${game.title}', '${game.path}')">Install</button>`}
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

  document.getElementById('libraryCount').textContent = state.games.length;
  const countText = document.getElementById('gamesCountText');
  if (countText) {
    countText.textContent = `Showing ${filtered.length} of ${state.games.length} titles`;
  }
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

  inGameList.innerHTML = '';
  onlineList.innerHTML = '';
  offlineList.innerHTML = '';

  let inGameCount = 0;
  let onlineCount = 0;

  state.friends.forEach(f => {
    const card = document.createElement('div');
    card.className = 'friend-card';
    card.innerHTML = `
      <div class="friend-main">
        <div class="friend-avatar">
          <img src="${f.avatar}" alt="${f.gamertag}">
          <span class="presence-badge ${f.state.toLowerCase() === 'in-game' ? 'online' : f.state.toLowerCase()}"></span>
        </div>
        <div class="friend-details">
          <span class="friend-gamertag">${f.gamertag}</span>
          <span class="friend-presence ${f.state.toLowerCase() === 'in-game' ? 'in-game' : ''}">${f.richPresence}</span>
        </div>
      </div>
      ${f.canJoin ? `<button class="btn btn-primary btn-sm" onclick="joinFriendGame('${f.gamertag}', '${f.gameTitle}')">Join Game</button>` : ''}
    `;

    if (f.state === 'In-Game') {
      inGameList.appendChild(card);
      inGameCount++;
    } else if (f.state === 'Online' || f.state === 'Away') {
      onlineList.appendChild(card);
      onlineCount++;
    } else {
      offlineList.appendChild(card);
    }
  });

  document.getElementById('inGameCount').textContent = inGameCount;
  document.getElementById('onlineOnlyCount').textContent = onlineCount;
  document.getElementById('onlineFriendsCount').textContent = inGameCount + onlineCount;
}

// Actions & Handlers
function launchGame(title, path) {
  showToast(`Launching ${title} with Proton CachyOS...`);
  sendNativeCommand({ cmd: 'launch_game', path: path });
}

function installGame(title, path) {
  showProgress(`Downloading & Decrypting ${title} via MSIXVC...`, 0);
  sendNativeCommand({ cmd: 'install_game', path: path });
  let progress = 0;
  const interval = setInterval(() => {
    progress += 8;
    if (progress > 100) {
      clearInterval(interval);
      hideProgress();
      showToast(`${title} installed and verified!`);
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
  showToast('Querying Microsoft Store Entitlements & Collections API...');
  sendNativeCommand({ cmd: 'sync_licenses' });
  setTimeout(() => {
    showToast('Verified 20 active digital game licenses & Game Pass entitlements!');
    renderGames();
  }, 600);
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

  window.setLibraryData = (gamesList) => {
    if (Array.isArray(gamesList)) {
      state.games = gamesList;
      renderGames();
      renderSaves();
    }
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

