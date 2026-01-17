/**
 * ChatPanel - Chat interface for room communication
 */
import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SidePanel } from "../SidePanel";
import "./ChatPanel.css";

export interface ChatMessage {
  id: string;
  senderId: string;
  senderName: string;
  content: string;
  timestamp: number;
  isSystem: boolean;
}

export interface ChatPanelProps {
  isOpen: boolean;
  onClose: () => void;
  connId: number | null;
  myPeerId: string | null;
}

export function ChatPanel({ isOpen, onClose, connId, myPeerId }: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [isSending, setIsSending] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Scroll to bottom when new messages arrive
  useEffect(() => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages]);

  // Focus input when panel opens
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isOpen]);

  // Poll for new messages
  useEffect(() => {
    if (!isOpen || connId === null) return;

    const pollMessages = async () => {
      try {
        const newMessages = await invoke<ChatMessage[]>("signaling_get_chat_messages", {
          sinceTimestamp: null,
        });
        setMessages(newMessages);
      } catch (err) {
        // Silently ignore errors during polling
      }
    };

    // Initial fetch
    pollMessages();

    // Poll every 500ms
    const interval = setInterval(pollMessages, 500);
    return () => clearInterval(interval);
  }, [isOpen, connId]);

  const handleSend = useCallback(async () => {
    if (!inputValue.trim() || !connId || isSending) return;

    setIsSending(true);
    try {
      await invoke("signaling_send_chat", {
        connId,
        content: inputValue.trim(),
      });
      setInputValue("");
    } catch (err) {
      console.error("Failed to send chat message:", err);
    } finally {
      setIsSending(false);
    }
  }, [inputValue, connId, isSending]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend]
  );

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString("ja-JP", {
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  const isMyMessage = (msg: ChatMessage) => {
    return myPeerId && msg.senderId === myPeerId;
  };

  return (
    <SidePanel isOpen={isOpen} onClose={onClose} title="チャット">
      <div className="chat-panel">
        <div className="chat-panel__messages">
          {messages.length === 0 && (
            <div className="chat-panel__empty">
              メッセージはまだありません
            </div>
          )}
          {messages.map((msg) => (
            <div
              key={msg.id}
              className={`chat-message ${
                msg.isSystem
                  ? "chat-message--system"
                  : isMyMessage(msg)
                  ? "chat-message--mine"
                  : "chat-message--other"
              }`}
            >
              {msg.isSystem ? (
                <div className="chat-message__system-content">
                  {msg.content}
                </div>
              ) : (
                <>
                  {!isMyMessage(msg) && (
                    <div className="chat-message__sender">{msg.senderName}</div>
                  )}
                  <div className="chat-message__bubble">
                    <div className="chat-message__content">{msg.content}</div>
                    <div className="chat-message__time">
                      {formatTime(msg.timestamp)}
                    </div>
                  </div>
                </>
              )}
            </div>
          ))}
          <div ref={messagesEndRef} />
        </div>

        <div className="chat-panel__input-area">
          <input
            ref={inputRef}
            type="text"
            className="chat-panel__input"
            placeholder="メッセージを入力..."
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={isSending || connId === null}
          />
          <button
            className="chat-panel__send-btn"
            onClick={handleSend}
            disabled={!inputValue.trim() || isSending || connId === null}
            aria-label="送信"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
              <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
            </svg>
          </button>
        </div>
      </div>
    </SidePanel>
  );
}

export default ChatPanel;
