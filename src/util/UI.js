// CLI UI Helper - Simple console formatting

const COLORS = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  dim: "\x1b[2m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  cyan: "\x1b[36m",
  magenta: "\x1b[35m",
};

const ICONS = {
  success: "✓",
  error: "✗",
  warning: "⚠",
  info: "ℹ",
  block: "⬛",
  chain: "⛓",
  wallet: "💰",
  mining: "⛏",
  network: "🌐",
  send: "📤",
  receive: "📥",
  coin: "💎",
  arrow: "→",
  pending: "⏳",
};

class UI {
  static success(text) {
    return `${COLORS.green}${ICONS.success} ${text}${COLORS.reset}`;
  }

  static error(text) {
    return `${COLORS.red}${ICONS.error} ${text}${COLORS.reset}`;
  }

  static warning(text) {
    return `${COLORS.yellow}${ICONS.warning} ${text}${COLORS.reset}`;
  }

  static info(text) {
    return `${COLORS.cyan}${ICONS.info} ${text}${COLORS.reset}`;
  }

  static box(content, title = "", width = 50) {
    const lines = content.split("\n");
    const maxLen = Math.max(
      width,
      ...lines.map((l) => l.length),
      title.length + 4,
    );
    let result = [];

    // Top border
    if (title) {
      result.push(
        `${COLORS.cyan}╭── ${COLORS.bright}${title}${COLORS.reset}${COLORS.cyan} ${"─".repeat(maxLen - title.length - 3)}╮${COLORS.reset}`,
      );
    } else {
      result.push(`${COLORS.cyan}╭${"─".repeat(maxLen + 2)}╮${COLORS.reset}`);
    }

    // Content
    lines.forEach((line) => {
      result.push(`${COLORS.cyan}│${COLORS.reset} ${line}`);
    });

    // Bottom border
    result.push(`${COLORS.cyan}╰${"─".repeat(maxLen + 2)}╯${COLORS.reset}`);

    return result.join("\n");
  }

  static keyValue(key, value, keyWidth = 15) {
    return `  ${COLORS.dim}${key.padEnd(keyWidth)}${COLORS.reset}: ${value}`;
  }

  static divider(char = "─", width = 40) {
    return `${COLORS.dim}${char.repeat(width)}${COLORS.reset}`;
  }

  static banner() {
    return `
${COLORS.cyan}╔══════════════════════════════════════════════════════╗
║                                                        ║
║   ${ICONS.chain}  MINI BLOCKCHAIN CLI  ${ICONS.chain}                          ║
║                                                        ║
╚══════════════════════════════════════════════════════╝${COLORS.reset}
`;
  }
}

module.exports = { UI, COLORS, ICONS };
