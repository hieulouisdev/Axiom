//! Internationalization: 7 supported locales.
//!
//! v0.8: expanded to 7 languages (English, Vietnamese, Spanish, French,
//! German, Japanese, Simplified Chinese). The backend table holds only
//! English + Vietnamese translations; the frontend ships its own complete
//! 7-locale table in `src/i18n/index.ts`. For locales without a backend
//! translation, the backend falls back to English so server-side strings
//! (alerts, notifications, audit log entries) always render correctly.
//!
//! Future versions may migrate to `fluent-bundle` for proper pluralization
//! and gender rules.

use std::collections::HashMap;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Supported UI locales.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Locale {
    En,
    Vi,
    Es,
    Fr,
    De,
    Ja,
    ZhCn,
}

impl Locale {
    /// Parse a BCP-47-style code (`en`, `vi`, `zh-CN`, …) into a `Locale`.
    /// Defaults to `En` for unknown codes.
    pub fn from_code(s: &str) -> Self {
        let norm = s.trim().to_lowercase();
        // Normalize zh-cn / zh_cn / zh-cn → zh-cn
        let norm = norm.replace('_', "-");
        match norm.as_str() {
            "vi" | "vn" | "vi-vn" | "vi-vi" => Locale::Vi,
            "es" | "es-es" | "es-419" => Locale::Es,
            "fr" | "fr-fr" | "fr-ca" => Locale::Fr,
            "de" | "de-de" | "de-at" | "de-ch" => Locale::De,
            "ja" | "ja-jp" | "jp" => Locale::Ja,
            "zh-cn" | "zh" | "zh-hans" | "zh-sg" => Locale::ZhCn,
            _ => Locale::En,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Vi => "vi",
            Locale::Es => "es",
            Locale::Fr => "fr",
            Locale::De => "de",
            Locale::Ja => "ja",
            Locale::ZhCn => "zh-CN",
        }
    }

    /// Whether the backend ships a full translation table for this locale.
    /// Locales that return `false` will fall back to English in `t()` /
    /// `all_for_locale()`.
    pub fn has_backend_table(&self) -> bool {
        matches!(self, Locale::En | Locale::Vi)
    }
}

/// Translation key → (English, Vietnamese).
/// For locales without a backend translation, the frontend ships its own
/// complete table; the backend `t()` here always returns the English or
/// Vietnamese string.
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
        ("nav.web", ("Web Search", "Tìm kiếm web")),
        ("nav.guide", ("User Guide", "Hướng dẫn sử dụng")),
        ("nav.theme.toggle", ("Toggle theme", "Đổi giao diện")),
        ("nav.sidebar.toggle", ("Toggle sidebar", "Ẩn/hiện thanh bên")),
        // Chat
        ("chat.placeholder", ("Type a message…", "Nhập tin nhắn…")),
        ("chat.send", ("Send", "Gửi")),
        ("chat.new_conversation", ("New conversation", "Cuộc trò chuyện mới")),
        ("chat.clear", ("Clear", "Xóa")),
        ("chat.empty.title", ("No messages yet", "Chưa có tin nhắn")),
        ("chat.empty.subtitle", ("Start a conversation to begin.", "Bắt đầu trò chuyện để khởi tạo.")),
        ("chat.empty.feature1", ("Search the web in real time", "Tìm kiếm web theo thời gian thực")),
        ("chat.empty.feature2", ("Remember facts about you automatically", "Tự động ghi nhớ thông tin về bạn")),
        ("chat.empty.feature3", ("Run shell commands safely", "Chạy lệnh shell an toàn")),
        ("chat.thinking", ("Thinking…", "Đang suy nghĩ…")),
        ("chat.error_no_provider", ("No AI provider configured. Go to Settings → Providers to add one.", "Chưa có nhà cung cấp AI. Vào Cài đặt → Providers để thêm.")),
        ("chat.copy", ("Copy", "Sao chép")),
        ("chat.copied", ("Copied!", "Đã sao chép!")),
        ("chat.regenerate", ("Regenerate", "Tạo lại")),
        // Settings
        ("settings.title", ("Settings", "Cài đặt")),
        ("settings.language", ("Language", "Ngôn ngữ")),
        ("settings.mode", ("Operational mode", "Chế độ hoạt động")),
        ("settings.mode.continuous", ("Continuous (always-on)", "Liên tục (luôn bật)")),
        ("settings.mode.ondemand", ("On-demand (saves cost)", "Khi được gọi (tiết kiệm chi phí)")),
        ("settings.allow_autonomous", ("Allow AI to act without asking", "Cho phép AI tự hành động không cần hỏi")),
        ("settings.allow_autonomous.hint", ("Dangerous: skips safety confirmation. Use only with trusted providers.", "Nguy hiểm: bỏ qua xác nhận an toàn. Chỉ dùng với provider tin cậy.")),
        ("settings.bypass_mode", ("Bypass Mode (advanced)", "Chế độ Bypass (nâng cao)")),
        ("settings.bypass_mode.hint", ("Skip confirmation for medium/high-risk actions except an irrevocable hard-deny list.", "Bỏ qua xác nhận cho hành động rủi ro trung bình/cao trừ danh sách chặn cứng.")),
        ("settings.theme", ("Theme", "Giao diện")),
        ("settings.theme.light", ("Light", "Sáng")),
        ("settings.theme.dark", ("Dark", "Tối")),
        ("settings.data_privacy", ("Data & Privacy", "Dữ liệu & Quyền riêng tư")),
        ("settings.data.export", ("Export all data (JSON)", "Xuất toàn bộ dữ liệu (JSON)")),
        ("settings.data.forget", ("Forget all data", "Xóa toàn bộ dữ liệu")),
        ("settings.data.forget.confirm", ("This permanently deletes all conversations, knowledge, and audit logs. Continue?", "Thao tác này xóa vĩnh viễn mọi cuộc trò chuyện, kiến thức và nhật ký kiểm toán. Tiếp tục?")),
        ("settings.encryption", ("Database encryption", "Mã hóa cơ sở dữ liệu")),
        ("settings.encryption.enabled", ("Encrypted at rest (SQLCipher).", "Đã mã hóa khi lưu (SQLCipher).")),
        ("settings.encryption.disabled", ("Not encrypted. Database is stored in plaintext on disk.", "Chưa mã hóa. Cơ sở dữ liệu lưu dạng văn bản rõ.")),
        ("settings.encryption.not_supported", ("Encryption is not supported on this platform.", "Mã hóa không được hỗ trợ trên nền tảng này.")),
        ("settings.sandbox", ("AI Sandbox", "Hộp cát AI")),
        ("settings.sandbox.hint", ("Restrict where the AI can write files on your machine.", "Giới hạn vị trí AI có thể ghi tệp.")),
        ("settings.sandbox.enabled", ("Sandbox enabled", "Bật hộp cát")),
        ("settings.sandbox.allowed_dirs", ("Allowed directories", "Thư mục được phép")),
        ("settings.sandbox.add_dir", ("Add directory", "Thêm thư mục")),
        ("settings.sandbox.remove", ("Remove", "Xóa")),
        ("settings.sandbox.empty", ("No extra directories allowed.", "Không có thư mục bổ sung nào được phép.")),
        ("settings.telemetry", ("Anonymous Telemetry", "Telemetry ẩn danh")),
        ("settings.telemetry.hint", ("Send anonymous usage metrics to help improve Aegis AI. Never on by default.", "Gửi chỉ số sử dụng ẩn danh để cải thiện Aegis AI. Mặc định tắt.")),
        ("settings.telemetry.status.enabled", ("Opted in. Anonymous events will be sent.", "Đã tham gia. Sự kiện ẩn danh sẽ được gửi.")),
        ("settings.telemetry.status.disabled", ("Not opted in. No data leaves your device.", "Chưa tham gia. Không có dữ liệu nào rời thiết bị.")),
        ("settings.telemetry.opt_in", ("Opt in", "Tham gia")),
        ("settings.telemetry.opt_out", ("Opt out", "Rút lui")),
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
        ("memory.search", ("Search memory…", "Tìm trong bộ nhớ…")),
        ("memory.stats.conversations", ("Conversations", "Cuộc trò chuyện")),
        ("memory.stats.messages", ("Messages", "Tin nhắn")),
        ("memory.stats.activities", ("Activities", "Hoạt động")),
        ("memory.stats.knowledge", ("Knowledge entries", "Mục kiến thức")),
        ("memory.knowledge.add", ("Remember this", "Ghi nhớ điều này")),
        ("memory.knowledge.empty", ("No knowledge entries yet. Tell the AI to remember something.", "Chưa có mục kiến thức. Yêu cầu AI ghi nhớ điều gì đó.")),
        // Security
        ("security.title", ("Security", "Bảo mật")),
        ("security.monitor", ("Process monitor", "Theo dõi tiến trình")),
        ("security.auto_defense", ("Auto-defense", "Tự động phòng thủ")),
        ("security.scanner", ("On-demand scanner", "Quét theo yêu cầu")),
        ("security.quarantine", ("Quarantine", "Cách ly")),
        ("security.quarantine.empty", ("No quarantined files.", "Không có tệp bị cách ly.")),
        ("security.quarantine.restore", ("Restore", "Khôi phục")),
        ("security.integrity", ("File integrity", "Toàn vẹn tệp")),
        ("security.network_scan", ("Network scan", "Quét mạng")),
        ("security.threats.recent", ("Recent threats", "Mối đe dọa gần đây")),
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

/// Translate a key using the current locale. For locales without a backend
/// translation table (es, fr, de, ja, zh-CN), the frontend ships its own
/// table; the backend falls back to English so server-side strings always
/// render correctly.
pub fn t(key: &str) -> String {
    let locale = current();
    if let Some((en, vi)) = TABLE.get(key) {
        match locale {
            Locale::Vi => {
                if vi.is_empty() {
                    en.to_string()
                } else {
                    vi.to_string()
                }
            }
            // All other locales fall back to English on the backend; the
            // frontend uses its own per-locale table.
            _ => en.to_string(),
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
            Locale::Vi => vi,
            _ => en,
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

    #[test]
    fn from_code_handles_all_seven_locales() {
        assert_eq!(Locale::from_code("en"), Locale::En);
        assert_eq!(Locale::from_code("vi"), Locale::Vi);
        assert_eq!(Locale::from_code("es"), Locale::Es);
        assert_eq!(Locale::from_code("fr"), Locale::Fr);
        assert_eq!(Locale::from_code("de"), Locale::De);
        assert_eq!(Locale::from_code("ja"), Locale::Ja);
        assert_eq!(Locale::from_code("zh-CN"), Locale::ZhCn);
        assert_eq!(Locale::from_code("zh_cn"), Locale::ZhCn);
        assert_eq!(Locale::from_code("ZH"), Locale::ZhCn);
    }

    #[test]
    fn unsupported_locale_falls_back_to_english() {
        set_locale(Locale::Ja);
        // Japanese has no backend table — `t()` should return English.
        assert_eq!(t("nav.chat"), "Chat");
    }

    #[test]
    fn codes_round_trip() {
        for l in [
            Locale::En,
            Locale::Vi,
            Locale::Es,
            Locale::Fr,
            Locale::De,
            Locale::Ja,
            Locale::ZhCn,
        ] {
            assert_eq!(Locale::from_code(l.code()), l);
        }
    }
}
