// jamjam UI Application

const { invoke } = window.__TAURI__.core;

// State
let audioRunning = false;
let sessionActive = false;
let peers = [];

// DOM Elements
const elements = {
    status: document.getElementById('status'),
    port: document.getElementById('port'),
    btnCreate: document.getElementById('btn-create'),
    btnLeave: document.getElementById('btn-leave'),
    sessionInfo: document.getElementById('session-info'),
    localAddr: document.getElementById('local-addr'),
    peerCount: document.getElementById('peer-count'),
    inputDevice: document.getElementById('input-device'),
    outputDevice: document.getElementById('output-device'),
    sampleRate: document.getElementById('sample-rate'),
    frameSize: document.getElementById('frame-size'),
    localMonitor: document.getElementById('local-monitor'),
    btnStartAudio: document.getElementById('btn-start-audio'),
    btnStopAudio: document.getElementById('btn-stop-audio'),
    mixerChannels: document.getElementById('mixer-channels'),
    meterLocal: document.getElementById('meter-local'),
    volumeLocal: document.getElementById('volume-local'),
    muteLocal: document.getElementById('mute-local'),
    latency: document.getElementById('latency'),
};

// Initialize
async function init() {
    console.log('Initializing jamjam UI...');

    // Load devices
    await loadDevices();

    // Load audio config
    await loadAudioConfig();

    // Setup event listeners
    setupEventListeners();

    // Start status polling
    setInterval(updateStatus, 1000);

    console.log('jamjam UI initialized');
}

// Load audio devices
async function loadDevices() {
    try {
        const inputDevices = await invoke('cmd_get_input_devices');
        const outputDevices = await invoke('cmd_get_output_devices');

        // Populate input devices
        elements.inputDevice.innerHTML = '<option value="">Default</option>';
        inputDevices.forEach(device => {
            const option = document.createElement('option');
            option.value = device.id;
            option.textContent = device.name + (device.is_default ? ' (default)' : '');
            elements.inputDevice.appendChild(option);
        });

        // Populate output devices
        elements.outputDevice.innerHTML = '<option value="">Default</option>';
        outputDevices.forEach(device => {
            const option = document.createElement('option');
            option.value = device.id;
            option.textContent = device.name + (device.is_default ? ' (default)' : '');
            elements.outputDevice.appendChild(option);
        });
    } catch (error) {
        console.error('Failed to load devices:', error);
    }
}

// Load audio configuration
async function loadAudioConfig() {
    try {
        const config = await invoke('cmd_get_audio_config');
        elements.sampleRate.value = config.sample_rate.toString();
        elements.frameSize.value = config.frame_size.toString();
    } catch (error) {
        console.error('Failed to load audio config:', error);
    }
}

// Setup event listeners
function setupEventListeners() {
    // Session controls
    elements.btnCreate.addEventListener('click', createSession);
    elements.btnLeave.addEventListener('click', leaveSession);

    // Audio controls
    elements.btnStartAudio.addEventListener('click', startAudio);
    elements.btnStopAudio.addEventListener('click', stopAudio);
    elements.localMonitor.addEventListener('change', toggleLocalMonitor);

    // Audio settings
    elements.sampleRate.addEventListener('change', updateAudioConfig);
    elements.frameSize.addEventListener('change', updateAudioConfig);

    // Local mute
    elements.muteLocal.addEventListener('click', toggleLocalMute);
}

// Create session
async function createSession() {
    try {
        const port = parseInt(elements.port.value);
        const localAddr = await invoke('cmd_create_session', { port });

        sessionActive = true;
        updateSessionUI(localAddr);
        showNotification('Session created', 'success');
    } catch (error) {
        console.error('Failed to create session:', error);
        showNotification('Failed to create session: ' + error, 'error');
    }
}

// Leave session
async function leaveSession() {
    try {
        await invoke('cmd_leave_session');

        sessionActive = false;
        updateSessionUI(null);
        showNotification('Left session', 'info');
    } catch (error) {
        console.error('Failed to leave session:', error);
    }
}

// Start audio
async function startAudio() {
    try {
        await updateAudioConfig();

        const inputDevice = elements.inputDevice.value || null;
        const outputDevice = elements.outputDevice.value || null;

        await invoke('cmd_start_audio', { inputDevice, outputDevice });

        audioRunning = true;
        elements.btnStartAudio.disabled = true;
        elements.btnStopAudio.disabled = false;
        showNotification('Audio started', 'success');
    } catch (error) {
        console.error('Failed to start audio:', error);
        showNotification('Failed to start audio: ' + error, 'error');
    }
}

// Stop audio
async function stopAudio() {
    try {
        await invoke('cmd_stop_audio');

        audioRunning = false;
        elements.btnStartAudio.disabled = false;
        elements.btnStopAudio.disabled = true;
        showNotification('Audio stopped', 'info');
    } catch (error) {
        console.error('Failed to stop audio:', error);
    }
}

// Toggle local monitoring
async function toggleLocalMonitor() {
    try {
        const enabled = elements.localMonitor.checked;
        await invoke('cmd_set_local_monitoring', { enabled });
    } catch (error) {
        console.error('Failed to toggle local monitoring:', error);
        elements.localMonitor.checked = !elements.localMonitor.checked;
    }
}

// Update audio configuration
async function updateAudioConfig() {
    try {
        const sampleRate = parseInt(elements.sampleRate.value);
        const frameSize = parseInt(elements.frameSize.value);

        await invoke('cmd_set_audio_config', {
            sampleRate,
            channels: 1,
            frameSize,
        });
    } catch (error) {
        console.error('Failed to update audio config:', error);
    }
}

// Toggle local mute
let localMuted = false;
function toggleLocalMute() {
    localMuted = !localMuted;
    elements.muteLocal.querySelector('.icon').textContent = localMuted ? '🔇' : '🔊';
    elements.muteLocal.classList.toggle('muted', localMuted);
}

// Update session UI
function updateSessionUI(localAddr) {
    if (localAddr) {
        elements.sessionInfo.style.display = 'block';
        elements.localAddr.textContent = localAddr;
        elements.btnCreate.disabled = true;
        elements.btnLeave.disabled = false;

        const statusDot = elements.status.querySelector('.status-dot');
        const statusText = elements.status.querySelector('.status-text');
        statusDot.classList.add('connected');
        statusText.textContent = 'Connected';
    } else {
        elements.sessionInfo.style.display = 'none';
        elements.btnCreate.disabled = false;
        elements.btnLeave.disabled = true;

        const statusDot = elements.status.querySelector('.status-dot');
        const statusText = elements.status.querySelector('.status-text');
        statusDot.classList.remove('connected');
        statusText.textContent = 'Disconnected';
    }
}

// Update status periodically
async function updateStatus() {
    try {
        const status = await invoke('cmd_get_session_status');
        elements.peerCount.textContent = status.peer_count;
        elements.localMonitor.checked = status.local_monitoring;

        // Update peers in mixer
        if (status.connected) {
            const peerList = await invoke('cmd_get_peers');
            updateMixerPeers(peerList);
        }
    } catch (error) {
        // Ignore errors during polling
    }
}

// Update mixer with peers
function updateMixerPeers(peerList) {
    // Remove old peer channels (keep local)
    const existingChannels = elements.mixerChannels.querySelectorAll('.channel:not(.local)');
    existingChannels.forEach(ch => ch.remove());

    // Add peer channels
    peerList.forEach(peer => {
        const channel = createPeerChannel(peer);
        elements.mixerChannels.appendChild(channel);
    });
}

// Create a peer channel element
function createPeerChannel(peer) {
    const channel = document.createElement('div');
    channel.className = 'channel peer';
    channel.dataset.peerId = peer.id;

    channel.innerHTML = `
        <div class="channel-label">${escapeHtml(peer.name)}</div>
        <div class="meter">
            <div class="meter-fill" id="meter-${peer.id}"></div>
        </div>
        <input type="range" class="volume-slider" id="volume-${peer.id}" min="0" max="100" value="100">
        <div class="channel-controls">
            <button class="btn-icon mute-btn" title="Mute">
                <span class="icon">🔊</span>
            </button>
        </div>
    `;

    // Add mute handler
    const muteBtn = channel.querySelector('.mute-btn');
    let muted = false;
    muteBtn.addEventListener('click', () => {
        muted = !muted;
        muteBtn.querySelector('.icon').textContent = muted ? '🔇' : '🔊';
        muteBtn.classList.toggle('muted', muted);
    });

    return channel;
}

// Show notification
function showNotification(message, type = 'info') {
    console.log(`[${type.toUpperCase()}] ${message}`);
    // Could implement toast notifications here
}

// Escape HTML
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', init);
