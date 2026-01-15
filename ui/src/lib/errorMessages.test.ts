/**
 * Error message conversion tests
 *
 * Tests for converting technical error messages to user-friendly messages.
 */

import { describe, it, expect } from "vitest";
import {
  parseErrorMessage,
  getErrorCategory,
} from "./errorMessages";

describe("getErrorCategory", () => {
  describe("connection errors", () => {
    it("categorizes 'connection refused' as refused", () => {
      expect(getErrorCategory("Connection refused")).toBe("connection.refused");
    });

    it("categorizes 'connection refused' case-insensitively", () => {
      expect(getErrorCategory("CONNECTION REFUSED")).toBe("connection.refused");
    });

    it("categorizes 'timed out' as timeout", () => {
      expect(getErrorCategory("Connection timed out")).toBe("connection.timeout");
    });

    it("categorizes 'timeout' as timeout", () => {
      expect(getErrorCategory("Request timeout")).toBe("connection.timeout");
    });

    it("categorizes 'connection reset' as lost", () => {
      expect(getErrorCategory("Connection reset by peer")).toBe("connection.lost");
    });

    it("categorizes 'connection lost' as lost", () => {
      expect(getErrorCategory("Connection lost")).toBe("connection.lost");
    });

    it("categorizes 'network unreachable' as lost", () => {
      expect(getErrorCategory("Network is unreachable")).toBe("connection.lost");
    });
  });

  describe("room errors", () => {
    it("categorizes 'room not found' as notFound", () => {
      expect(getErrorCategory("Room not found")).toBe("room.notFound");
    });

    it("categorizes 'room is full' as full", () => {
      expect(getErrorCategory("Room is full")).toBe("room.full");
    });

    it("categorizes 'invalid code' as notFound", () => {
      expect(getErrorCategory("Invalid invite code")).toBe("room.notFound");
    });

    it("categorizes 'invalid password' as password", () => {
      expect(getErrorCategory("Invalid password")).toBe("room.password");
    });

    it("categorizes 'incorrect password' as password", () => {
      expect(getErrorCategory("Incorrect password")).toBe("room.password");
    });
  });

  describe("audio errors", () => {
    it("categorizes 'device not found' as device", () => {
      expect(getErrorCategory("Audio device not found")).toBe("audio.device");
    });

    it("categorizes 'permission denied' as permission", () => {
      expect(getErrorCategory("Microphone permission denied")).toBe("audio.permission");
    });

    it("categorizes 'no permission' as permission", () => {
      expect(getErrorCategory("No permission to access microphone")).toBe("audio.permission");
    });
  });

  describe("generic errors", () => {
    it("categorizes unknown errors as generic", () => {
      expect(getErrorCategory("Some unknown error happened")).toBe("generic");
    });

    it("categorizes empty string as generic", () => {
      expect(getErrorCategory("")).toBe("generic");
    });
  });
});

describe("parseErrorMessage", () => {
  it("returns parsed error with category and original message", () => {
    const result = parseErrorMessage("Connection refused by server");
    expect(result.category).toBe("connection.refused");
    expect(result.originalMessage).toBe("Connection refused by server");
  });

  it("returns i18n keys for title and message", () => {
    const result = parseErrorMessage("Connection timed out");
    expect(result.titleKey).toBe("error.connection.timeout.title");
    expect(result.messageKey).toBe("error.connection.timeout.message");
  });

  it("returns generic keys for unknown errors", () => {
    const result = parseErrorMessage("Unknown error");
    expect(result.titleKey).toBe("error.generic.title");
    expect(result.messageKey).toBe("error.generic.message");
  });
});
