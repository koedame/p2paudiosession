/**
 * Tauri IPC wrapper functions
 *
 * Provides typed interfaces for communicating with the Rust backend.
 */

import { invoke } from "@tauri-apps/api/core";

/**
 * Room information from signaling server
 */
export interface RoomInfo {
  id: string;
  name: string;
  peer_count: number;
  max_peers: number;
  has_password: boolean;
}

/**
 * Peer information
 */
export interface PeerInfo {
  id: string;
  name: string;
  public_addr: string | null;
  local_addr: string | null;
}

/**
 * Result of joining or creating a room
 */
export interface JoinResult {
  room_id: string;
  peer_id: string;
  peers: PeerInfo[];
}

/**
 * Connect to a signaling server
 * @param url WebSocket URL of the signaling server (e.g., ws://localhost:8080)
 * @returns Connection ID for subsequent operations
 */
export async function signalingConnect(url: string): Promise<number> {
  return invoke("signaling_connect", { url });
}

/**
 * Disconnect from a signaling server
 * @param connId Connection ID from signalingConnect
 */
export async function signalingDisconnect(connId: number): Promise<void> {
  return invoke("signaling_disconnect", { connId });
}

/**
 * List available rooms on the signaling server
 * @param connId Connection ID from signalingConnect
 * @returns Array of room information
 */
export async function signalingListRooms(connId: number): Promise<RoomInfo[]> {
  return invoke("signaling_list_rooms", { connId });
}

/**
 * Join an existing room
 * @param connId Connection ID from signalingConnect
 * @param roomId Room ID to join
 * @param peerName Display name for this peer
 * @returns Join result with room info and peer list
 */
export async function signalingJoinRoom(
  connId: number,
  roomId: string,
  peerName: string
): Promise<JoinResult> {
  return invoke("signaling_join_room", { connId, roomId, peerName });
}

/**
 * Leave the current room
 * @param connId Connection ID from signalingConnect
 */
export async function signalingLeaveRoom(connId: number): Promise<void> {
  return invoke("signaling_leave_room", { connId });
}

/**
 * Create a new room
 * @param connId Connection ID from signalingConnect
 * @param roomName Name for the new room
 * @param peerName Display name for this peer (room creator)
 * @returns Join result with the new room info
 */
export async function signalingCreateRoom(
  connId: number,
  roomName: string,
  peerName: string
): Promise<JoinResult> {
  return invoke("signaling_create_room", { connId, roomName, peerName });
}
