/**
 * Error message conversion utility
 *
 * Converts technical error messages from the backend to user-friendly
 * messages using i18n keys. This provides a consistent error display
 * experience across the application.
 */

/**
 * Error category types matching the i18n error structure
 */
export type ErrorCategory =
  | "connection.timeout"
  | "connection.refused"
  | "connection.lost"
  | "audio.device"
  | "audio.permission"
  | "room.full"
  | "room.password"
  | "room.notFound"
  | "generic";

/**
 * Parsed error result
 */
export interface ParsedError {
  /** Error category for styling/handling */
  category: ErrorCategory;
  /** Original error message from the backend */
  originalMessage: string;
  /** i18n key for the error title */
  titleKey: string;
  /** i18n key for the error message */
  messageKey: string;
}

/**
 * Error pattern matchers
 *
 * Each pattern is a tuple of [regex, category]
 */
const ERROR_PATTERNS: Array<[RegExp, ErrorCategory]> = [
  // Connection errors
  [/connection\s*refused/i, "connection.refused"],
  [/timed?\s*out/i, "connection.timeout"],
  [/timeout/i, "connection.timeout"],
  [/connection\s*(reset|lost)/i, "connection.lost"],
  [/network\s*(is\s*)?(unreachable|down)/i, "connection.lost"],
  [/disconnected/i, "connection.lost"],

  // Room errors
  [/room\s*(is\s*)?full/i, "room.full"],
  [/room\s*not\s*found/i, "room.notFound"],
  [/invalid\s*(invite\s*)?code/i, "room.notFound"],
  [/(invalid|incorrect|wrong)\s*password/i, "room.password"],

  // Audio errors
  [/(audio|microphone|speaker)\s*device\s*not\s*found/i, "audio.device"],
  [/device\s*(is\s*)?busy/i, "audio.device"],
  [/(permission\s*denied|no\s*permission)/i, "audio.permission"],
  [/access\s*denied/i, "audio.permission"],
];

/**
 * Get the error category for a technical error message
 *
 * @param errorMessage Technical error message from the backend
 * @returns Error category for i18n lookup
 */
export function getErrorCategory(errorMessage: string): ErrorCategory {
  if (!errorMessage) {
    return "generic";
  }

  for (const [pattern, category] of ERROR_PATTERNS) {
    if (pattern.test(errorMessage)) {
      return category;
    }
  }

  return "generic";
}

/**
 * Parse a technical error message into a user-friendly structure
 *
 * @param errorMessage Technical error message from the backend
 * @returns Parsed error with i18n keys
 */
export function parseErrorMessage(errorMessage: string): ParsedError {
  const category = getErrorCategory(errorMessage);

  return {
    category,
    originalMessage: errorMessage,
    titleKey: `error.${category}.title`,
    messageKey: `error.${category}.message`,
  };
}

/**
 * Format error message for display
 *
 * This is a convenience function that returns the i18n keys
 * and can be used with the useTranslation hook.
 *
 * @param errorMessage Technical error message
 * @param t i18n translation function
 * @returns Object with title and message for display
 */
export function formatErrorForDisplay(
  errorMessage: string,
  t: (key: string) => string
): { title: string; message: string } {
  const parsed = parseErrorMessage(errorMessage);

  return {
    title: t(parsed.titleKey),
    message: t(parsed.messageKey),
  };
}
