// Xodus GUI Client Application Logic

const state = {
  activeTab: 'library',
  user: {
    gamertag: 'noct',
    puid: '2533274839201029',
    presence: 'Active',
    avatar: 'https://images.unsplash.com/photo-1566492031773-4f4e44671857?w=128&auto=format&fit=crop&q=80',
  },
  games: [
    {
      id: '77BB5AFB',
      title: 'Brotato',
      developer: 'Blobfish / Seaven Studio',
      installed: true,
      size: '423.2 MB',
      path: '/mnt/w11/XboxGames/Brotato',
      cover: 'https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=600&auto=format&fit=crop&q=80',
      cloudSynced: true,
      lastPlayed: 'Today'
    },
    {
      id: '4F56E789',
      title: 'Hi-Fi RUSH',
      developer: 'Tango Gameworks',
      installed: false,
      size: '14.2 GB',
      path: '/mnt/w11/XboxGames/HiFiRush',
      cover: 'https://images.unsplash.com/photo-1542751371-adc38448a05e?w=600&auto=format&fit=crop&q=80',
      cloudSynced: false,
      lastPlayed: '3 days ago'
    },
    {
      id: '89ABCDEF',
      title: 'Vampire Survivors',
      developer: 'poncle',
      installed: false,
      size: '512 MB',
      path: '/mnt/w11/XboxGames/VampireSurvivors',
      cover: 'https://images.unsplash.com/photo-1550745165-9bc0b252726f?w=600&auto=format&fit=crop&q=80',
      cloudSynced: false,
      lastPlayed: '1 week ago'
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
  setupNavigation();
  renderUser();
  renderGames();
  renderSaves();
  renderFriends();
  setupIPCBridge();
});

// Navigation Handling
function setupNavigation() {
  const navItems = document.querySelectorAll('.nav-item');
  navItems.forEach(item => {
    item.addEventListener('click', () => {
      const tab = item.getAttribute('data-tab');
      switchTab(tab);
    });
  });

  const presenceSelect = document.getElementById('presenceSelect');
  presenceSelect.addEventListener('change', (e) => {
    updatePresence(e.target.value);
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

  state.games.forEach(game => {
    const card = document.createElement('div');
    card.className = 'game-card';
    card.innerHTML = `
      <div class="game-card-cover">
        <img src="${game.cover}" alt="${game.title}">
        <span class="game-card-badge">${game.installed ? 'INSTALLED' : 'READY'}</span>
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
        <span class="save-item-meta">Title ID: ${game.id} • SCID: 00000000-0000-0000-0000-0000${game.id.toLowerCase()}</span>
      </div>
      <div class="save-item-actions">
        <span class="status-indicator-dot synced" title="In Sync"></span>
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
  showProgress(`Installing ${title}...`, 0);
  sendNativeCommand({ cmd: 'install_game', path: path });
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
  sendNativeCommand({ cmd: 'set_presence', state: state });
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
  document.getElementById('statusPercent').textContent = `${percent}%`;
}

function hideProgress() {
  document.getElementById('statusBar').style.display = 'none';
}

// Notification Toast
function showToast(msg) {
  console.log('[XODUS]', msg);
}

// Native IPC Bridge
function sendNativeCommand(msg) {
  if (window.ipc) {
    window.ipc.postMessage(JSON.stringify(msg));
  } else {
    console.log('[Native IPC Simulation]:', msg);
  }
}

function setupIPCBridge() {
  window.onNativeMessage = function(data) {
    try {
      const msg = typeof data === 'string' ? JSON.parse(data) : data;
      if (msg.type === 'profile') {
        state.user.gamertag = msg.gamertag;
        renderUser();
      } else if (msg.type === 'friends') {
        state.friends = msg.friends;
        renderFriends();
      } else if (msg.type === 'progress') {
        showProgress(msg.title, msg.percent, msg.speed);
        if (msg.percent >= 100) setTimeout(hideProgress, 2000);
      }
    } catch (e) {
      console.error('Failed to parse native message:', e);
    }
  };
}
