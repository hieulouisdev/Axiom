//! Internationalization: English (default) and Vietnamese.
//!
//! v0.1 uses a static translation table. Phase 2 will switch to
//! `fluent-bundle` for proper pluralization and gender rules.

use std::collections::HashMap;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Locale {
    En,
    Vi,
}

impl Locale {
    pub fn from_code(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vi" | "vn" | "vi-vn" | "vi_vn" => Locale::Vi,
            _ => Locale::En,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Vi => "vi",
        }
    }
}

/// Translation key → (English, Vietnamese).
static TABLE: Lazy<HashMap<&'static str, (&'static str, &'static str)>> = Lazy::new(|| {
    [
        // App
        ("app.name", ("Aegis AI", "Aegis AI")),
        ("app.tagline", ("Secure cross-platform AI assistant", "Trợ lý AI bảo mật đa nền tảng")),
        // Sidebar
        ("nav.chat", ("Chat", "Trò chuyện")),
        ("nav.providers", ("AI Providers", "Nhà cung cấp AI")),
        ("nav.memory", ("Memory", "Bộ nhớ")),
        ("nav.security", ("Security", "Bảo mật")),
        ("nav.settings", ("Settings", "Cài đặt")),
        ("nav.modes", ("Modes", "Chế độ")),
        // Chat
        ("chat.placeholder", ("Type a message…", "Nhập tin nhắn…")),
        ("chat.send", ("Send", "Gửi")),
        ("chat.new_conversation", ("New conversation", "Cuộc trò chuyện mới")),
        ("chat.clear", ("Clear", "Xóa")),
        ("chat.empty.title", ("No messages yet", "Chưa có tin nhắn")),
        ("chat.empty.subtitle", ("Start a conversation to begin.", "Bắt đầu trò chuyện để khởi tạo.")),
        ("chat.thinking", ("Thinking…", "Đang suy nghĩ…")),
        ("chat.error_no_provider", ("No AI provider configured. Go to Settings → Providers to add one.", "Chưa có nhà cung cấp AI. Vào Cài đặt → Providers để thêm.")),
        // Settings
        ("settings.title", ("Settings", "Cài đặt")),
        ("settings.language", ("Language", "Ngôn ngữ")),
        ("settings.mode", ("Operational mode", "Chế độ hoạt động")),
        ("settings.mode.continuous", ("Continuous (always-on)", "Liên tục (luôn bật)")),
        ("settings.mode.ondemand", ("On-demand (saves cost)", "Khi được gọi (tiết kiệm chi phí)")),
        ("settings.allow_autonomous", ("Allow AI to act without asking", "Cho phép AI tự hành động không cần hỏi")),
        ("settings.allow_autonomous.hint", ("Dangerous: skips safety confirmation. Use only with trusted providers.", "Nguy hiểm: bỏ qua xác nhận an toàn. Chỉ dùng với provider tin cậy.")),
        // Providers
        ("providers.title", ("AI Providers", "Nhà cung cấp AI")),
        ("providers.add", ("Add provider", "Thêm provider")),
        ("providers.configure", ("Configure", "Cấu hình")),
        ("providers.test", ("Test connection", "Kiểm tra kết nối")),
        ("providers.activate", ("Set as active", "Đặt làm mặc định")),
        ("providers.api_key", ("API key", "Khóa API")),
        ("providers.base_url", ("Base URL", "URL cơ sở")),
        ("providers.model", ("Model", "Mô hình")),
        ("providers.enabled", ("Enabled", "Bật")),
        ("providers.category.cloud_major", ("Cloud — major", "Cloud — chính")),
        ("providers.category.cloud_other", ("Cloud — other", "Cloud — khác")),
        ("providers.category.local", ("Local", "Cục bộ")),
        ("providers.category.custom", ("Custom", "Tùy chỉnh")),
        ("providers.test.success", ("Connection OK", "Kết nối thành công")),
        ("providers.test.failure", ("Connection failed", "Kết nối thất bại")),
        // Memory
        ("memory.title", ("Memory", "Bộ nhớ")),
        ("memory.conversations", ("Conversations", "Cuộc trò chuyện")),
        ("memory.activities", ("Activity log", "Nhật ký hoạt động")),
        ("memory.knowledge", ("Knowledge base", "Cơ sở kiến thức")),
        ("memory.search", ("Search…", "Tìm kiếm…")),
        ("memory.clear_all", ("Clear everything", "Xóa toàn bộ")),
        ("memory.stats.conversations", ("Conversations", "Cuộc trò chuyện")),
        ("memory.stats.messages", ("Messages", "Tin nhắn")),
        ("memory.stats.activities", ("Activities", "Hoạt động")),
        ("memory.stats.knowledge", ("Facts", "Sự kiện")),
        // Security
        ("security.title", ("Security", "Bảo mật")),
        ("security.status", ("Status", "Trạng thái")),
        ("security.monitor", ("Process monitor", "Theo dõi tiến trình")),
        ("security.auto_defense", ("Auto-defense", "Tự động phòng thủ")),
        ("security.scanner", ("Virus scanner", "Quét virus")),
        ("security.scan_now", ("Scan now", "Quét ngay")),
        ("security.quarantine", ("Quarantine", "Cách ly")),
        ("security.restore", ("Restore", "Khôi phục")),
        ("security.delete", ("Delete permanently", "Xóa vĩnh viễn")),
        ("security.threats.recent", ("Recent threats", "Mối đe dọa gần đây")),
        ("security.events.recent", ("Recent defense events", "Sự kiện phòng thủ gần đây")),
        ("severity.info", ("Info", "Thông tin")),
        ("severity.low", ("Low", "Thấp")),
        ("severity.medium", ("Medium", "Trung bình")),
        ("severity.high", ("High", "Cao")),
        ("severity.critical", ("Critical", "Nghiêm trọng")),
        // Modes
        ("modes.title", ("Modes", "Chế độ")),
        ("modes.continuous.title", ("Continuous", "Liên tục")),
        ("modes.continuous.desc", ("AI is always on and listens to events. Higher cost, lower latency.", "AI luôn bật và lắng nghe sự kiện. Chi phí cao, độ trễ thấp.")),
        ("modes.ondemand.title", ("On-demand", "Khi được gọi")),
        ("modes.ondemand.desc", ("AI stays dormant until called. Lowest cost. Security monitor still runs.", "AI ngủ cho đến khi được gọi. Chi phí thấp nhất. Bộ bảo mật vẫn chạy.")),
        // Common
        ("common.save", ("Save", "Lưu")),
        ("common.cancel", ("Cancel", "Hủy")),
        ("common.confirm", ("Confirm", "Xác nhận")),
        ("common.deny", ("Deny", "Từ chối")),
        ("common.close", ("Close", "Đóng")),
        ("common.delete", ("Delete", "Xóa")),
        ("common.retry", ("Retry", "Thử lại")),
        ("common.yes", ("Yes", "Có")),
        ("common.no", ("No", "Không")),
        ("common.loading", ("Loading…", "Đang tải…")),
        ("common.error", ("Error", "Lỗi")),
        ("common.success", ("Success", "Thành công")),
        // Safety
        ("safety.confirm.title", ("Confirm action", "Xác nhận hành động")),
        ("safety.confirm.body", ("The AI wants to perform an action that may affect your system. Review and confirm.", "AI muốn thực hiện hành động có thể ảnh hưởng đến hệ thống. Vui lòng xem xét và xác nhận.")),
        ("safety.deny.title", ("Action blocked", "Hành động bị chặn")),
        ("safety.deny.body", ("This action was blocked by the safety policy.", "Hành động này bị chặn bởi chính sách an toàn.")),
    ]
    .into_iter()
    .collect()
});

/// Process-wide current locale. Defaults to English.
static CURRENT: Lazy<RwLock<Locale>> = Lazy::new(|| RwLock::new(Locale::En));

pub fn set_locale(locale: Locale) {
    *CURRENT.write() = locale;
}

pub fn current() -> Locale {
    *CURRENT.read()
}

/// Translate a key. Falls back to English if the Vietnamese translation is
/// missing, falls back to the key itself if neither exists.
pub fn t(key: &str) -> String {
    let locale = current();
    if let Some((en, vi)) = TABLE.get(key) {
        match locale {
            Locale::En => en.to_string(),
            Locale::Vi => {
                if vi.is_empty() {
                    en.to_string()
                } else {
                    vi.to_string()
                }
            }
        }
    } else {
        key.to_string()
    }
}

/// Returns a JSON map of all translations for the given locale. Used by
/// the frontend on startup to hydrate its i18n table.
pub fn all_for_locale(locale: Locale) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (k, (en, vi)) in TABLE.iter() {
        let v = match locale {
            Locale::En => en,
            Locale::Vi => vi,
        };
        map.insert(k.to_string(), serde_json::Value::String(v.to_string()));
    }
    serde_json::Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_default() {
        set_locale(Locale::En);
        assert_eq!(t("nav.chat"), "Chat");
    }

    #[test]
    fn vietnamese_lookup() {
        set_locale(Locale::Vi);
        assert_eq!(t("nav.chat"), "Trò chuyện");
    }

    #[test]
    fn missing_key_returns_key() {
        set_locale(Locale::En);
        assert_eq!(t("does.not.exist"), "does.not.exist");
    }
}
