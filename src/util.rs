//! UI helpers + validators — port từ src/util/UI.js, AddressHelper.js, Validator.js.

use crate::config::MICRO_PER_COIN;

pub const RESET: &str = "\x1b[0m";
pub const BRIGHT: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const CYAN: &str = "\x1b[36m";
pub const MAGENTA: &str = "\x1b[35m";

pub fn success(text: &str) -> String {
    format!("{GREEN}✓ {text}{RESET}")
}
pub fn error(text: &str) -> String {
    format!("{RED}✗ {text}{RESET}")
}
pub fn warning(text: &str) -> String {
    format!("{YELLOW}⚠ {text}{RESET}")
}
pub fn info(text: &str) -> String {
    format!("{CYAN}ℹ {text}{RESET}")
}

/// Box có tiêu đề (port UI.box). `lines` đã chứa màu.
pub fn box_lines(lines: &[String], title: &str, width: usize) -> String {
    // maxLen bỏ qua ANSI escape khi đo
    let visible_len = |s: &str| strip_ansi(s).chars().count();
    let max_len = lines
        .iter()
        .map(|l| visible_len(l))
        .chain([width, title.chars().count() + 4])
        .max()
        .unwrap_or(width);

    let mut out = Vec::new();
    if title.is_empty() {
        out.push(format!("{CYAN}╭{}╮{RESET}", "─".repeat(max_len + 2)));
    } else {
        out.push(format!(
            "{CYAN}╭── {BRIGHT}{title}{RESET}{CYAN} {}╮{RESET}",
            "─".repeat(max_len.saturating_sub(title.chars().count() + 3))
        ));
    }
    for line in lines {
        out.push(format!("{CYAN}│{RESET} {line}"));
    }
    out.push(format!("{CYAN}╰{}╯{RESET}", "─".repeat(max_len + 2)));
    out.join("\n")
}

pub fn key_value(key: &str, value: &str) -> String {
    format!("{DIM}{key:<15}{RESET}: {value}")
}

pub fn divider(width: usize) -> String {
    format!("{DIM}{}{RESET}", "─".repeat(width))
}

pub fn banner() -> &'static str {
    concat!(
        "\n\x1b[36m╔══════════════════════════════════════════════════════╗\n",
        "║                                                        ║\n",
        "║   ⛓  MINI BLOCKCHAIN CLI  ⛓                           ║\n",
        "║                                                        ║\n",
        "╚══════════════════════════════════════════════════════╝\x1b[0m\n",
    )
}

/// Bỏ ANSI escape codes để đo độ dài hiển thị
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // skip đến ký tự kết thúc escape (m)
            for c2 in chars.by_ref() {
                if c2 == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ---- AddressHelper.js ----

pub const COINBASE_SENDER: &str = "COINBASE";
pub const SYSTEM_SENDER: &str = "SYSTEM";

/// Rút gọn address để hiển thị: "12345678...abcdef"
pub fn shorten_address(address: &str) -> String {
    if address.is_empty() || address == COINBASE_SENDER || address == SYSTEM_SENDER {
        return address.to_string();
    }
    let (prefix, suffix) = (8, 6);
    if address.len() <= prefix + suffix + 3 {
        return address.to_string();
    }
    format!("{}...{}", &address[..prefix], &address[address.len() - suffix..])
}

/// Address hợp lệ: 64 ký tự hex
pub fn is_valid_address(address: &str) -> bool {
    address.len() == 64 && address.bytes().all(|b| b.is_ascii_hexdigit())
}

// ---- Validator.js ----

pub fn validate_port(port: &str) -> Result<u16, String> {
    let p: u16 = port.parse().map_err(|_| "Port must be a valid integer".to_string())?;
    if p == 0 {
        return Err("Port must be between 1 and 65535".to_string());
    }
    Ok(p)
}

pub fn is_localhost(host: &str) -> bool {
    matches!(host.to_lowercase().as_str(), "localhost" | "127.0.0.1" | "0.0.0.0" | "::1")
}

pub fn validate_wallet_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Wallet name must be a non-empty string".to_string());
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(
            "Wallet name can only contain letters, numbers, underscore, and dash".to_string(),
        );
    }
    Ok(trimmed.to_string())
}

// ---- micro-coin parse / format ----

/// "12.5" -> 12_500_000. Tối đa 6 chữ số thập phân.
pub fn coins_str_to_micro(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i, f),
        None => (s, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return Err("Amount must be a number".to_string());
    }
    let int_part = if int_part.is_empty() { "0" } else { int_part };
    let int_micro: u64 = int_part
        .parse::<u64>()
        .map_err(|_| "Amount must be a number".to_string())?
        .checked_mul(MICRO_PER_COIN)
        .ok_or("Amount too large")?;
    let frac_micro: u64 = if frac_part.is_empty() {
        0
    } else {
        if frac_part.len() > 6 || !frac_part.bytes().all(|b| b.is_ascii_digit()) {
            return Err("At most 6 decimal places allowed".to_string());
        }
        let padded = format!("{frac_part:0<6}");
        padded[..6].parse::<u64>().map_err(|_| "Amount must be a number".to_string())?
    };
    int_micro
        .checked_add(frac_micro)
        .ok_or_else(|| "Amount too large".to_string())
}

/// 12_500_000 -> "12.5"
pub fn fmt_micro(micro: u64) -> String {
    let coins = micro / MICRO_PER_COIN;
    let frac = micro % MICRO_PER_COIN;
    if frac == 0 {
        coins.to_string()
    } else {
        let frac_str = format!("{frac:06}").trim_end_matches('0').to_string();
        format!("{coins}.{frac_str}")
    }
}

/// Số dư có thể âm tạm thời khi debit (giống JS) — hiển thị dạng đó
pub fn fmt_micro_i(micro: i128) -> String {
    if micro < 0 {
        format!("-{}", fmt_micro(micro.unsigned_abs() as u64))
    } else {
        fmt_micro(micro as u64)
    }
}

/// Timestamp hiện tại (ms unix)
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Format timestamp ms -> "YYYY-MM-DD HH:MM:SS"
pub fn fmt_timestamp(ms: u64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ms.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coin_parsing() {
        assert_eq!(coins_str_to_micro("12.5").unwrap(), 12_500_000);
        assert_eq!(coins_str_to_micro("12").unwrap(), 12_000_000);
        assert_eq!(coins_str_to_micro("0.000001").unwrap(), 1);
        assert_eq!(coins_str_to_micro(".5").unwrap(), 500_000);
        assert!(coins_str_to_micro("abc").is_err());
        assert!(coins_str_to_micro("1.1234567").is_err());
        assert!(coins_str_to_micro("-1").is_err());
    }

    #[test]
    fn coin_formatting() {
        assert_eq!(fmt_micro(12_500_000), "12.5");
        assert_eq!(fmt_micro(12_000_000), "12");
        assert_eq!(fmt_micro(1), "0.000001");
        assert_eq!(fmt_micro_i(-5_000_000), "-5");
    }

    #[test]
    fn address_helpers() {
        assert!(is_valid_address(&"a".repeat(64)));
        assert!(!is_valid_address(&"a".repeat(63)));
        assert!(!is_valid_address("COINBASE"));
        let addr = "0123456789abcdef".repeat(4);
        assert_eq!(shorten_address(&addr), "01234567...abcdef");
        assert_eq!(shorten_address("COINBASE"), "COINBASE");
    }

    #[test]
    fn wallet_name_validation() {
        assert!(validate_wallet_name("Alice_01").is_ok());
        assert!(validate_wallet_name("a/b").is_err());
        assert!(validate_wallet_name("  ").is_err());
    }
}
