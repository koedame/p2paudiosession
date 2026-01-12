// Type definitions for Tauri commands

export interface DeviceInfo {
  id: string;
  name: string;
  is_default: boolean;
}

export interface PeerInfo {
  id: string;
  name: string;
  volume: number;
  muted: boolean;
}

export interface SessionStatus {
  connected: boolean;
  room_id: string | null;
  peer_count: number;
  audio_running: boolean;
  local_monitoring: boolean;
}

export interface AudioConfig {
  sample_rate: number;
  channels: number;
  frame_size: number;
}

export interface ConnectionStats {
  rtt_ms: number;
  packet_loss_percent: number;
  jitter_ms: number;
  bytes_sent: number;
  bytes_received: number;
  packets_sent: number;
  packets_received: number;
}

export interface RoomInfo {
  id: string;
  name: string;
  peer_count: number;
  max_peers: number;
  has_password: boolean;
}

// Application state
export interface AppState {
  audioRunning: boolean;
  sessionActive: boolean;
  localMuted: boolean;
  peers: PeerInfo[];
  currentScreen: Screen;
}

export type Screen = "main" | "mixer" | "settings" | "connection";

// i18n namespace keys
export type TranslationNamespace =
  | "common"
  | "session"
  | "audio"
  | "mixer"
  | "settings"
  | "status"
  | "error";
