/**
 * Logger utility for structured logging
 */

const LOG_LEVELS = {
  ERROR: 0,
  WARN: 1,
  INFO: 2,
  DEBUG: 3,
  TRACE: 4,
};

class Logger {
  constructor(context = "APP", level = LOG_LEVELS.INFO) {
    this.context = context;
    this.level = level;
    this.colors = {
      reset: "\x1b[0m",
      red: "\x1b[31m",
      yellow: "\x1b[33m",
      blue: "\x1b[34m",
      cyan: "\x1b[36m",
      gray: "\x1b[90m",
      green: "\x1b[32m",
    };
  }

  _formatTime() {
    const now = new Date();
    return now.toISOString().split("T")[1].slice(0, -1); // HH:MM:SS.mmm
  }

  _log(level, levelName, color, message, ...args) {
    if (level > this.level) return;

    const timestamp = this._formatTime();
    const prefix = `${this.colors.gray}[${timestamp}]${this.colors.reset} ${color}[${levelName}]${this.colors.reset} ${this.colors.cyan}[${this.context}]${this.colors.reset}`;

    console.log(prefix, message, ...args);
  }

  error(message, ...args) {
    this._log(LOG_LEVELS.ERROR, "ERROR", this.colors.red, message, ...args);
  }

  warn(message, ...args) {
    this._log(LOG_LEVELS.WARN, "WARN ", this.colors.yellow, message, ...args);
  }

  info(message, ...args) {
    this._log(LOG_LEVELS.INFO, "INFO ", this.colors.blue, message, ...args);
  }

  debug(message, ...args) {
    this._log(LOG_LEVELS.DEBUG, "DEBUG", this.colors.gray, message, ...args);
  }

  trace(message, ...args) {
    this._log(LOG_LEVELS.TRACE, "TRACE", this.colors.gray, message, ...args);
  }

  success(message, ...args) {
    this._log(LOG_LEVELS.INFO, "SUCCESS", this.colors.green, message, ...args);
  }

  /**
   * Create a child logger with a new context
   */
  child(context) {
    return new Logger(`${this.context}:${context}`, this.level);
  }

  /**
   * Set log level
   */
  setLevel(level) {
    if (typeof level === "string") {
      this.level = LOG_LEVELS[level.toUpperCase()] ?? LOG_LEVELS.INFO;
    } else {
      this.level = level;
    }
  }
}

// Global logger instance
const globalLogger = new Logger("BC");

module.exports = {
  Logger,
  LOG_LEVELS,
  logger: globalLogger,
};
