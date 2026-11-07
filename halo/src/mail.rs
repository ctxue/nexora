//! 六类邮箱的账号模型、IMAP 收取与正文解析。
//!
//! 轻量化约束：只用 Rust 原生 crate（`imap` + `native-tls` + `mailparse`），
//! 不引 Node/Go/Electron。凭据一律进 Windows 凭据管理器（`keyring`），
//! 账号清单只存「邮箱 + 类型 + 是否监控」，不存密码。
//!
//! 收信优先 `Session::idle()` 长连接；服务器不支持 IDLE 时退回轮询（60/120/300 秒）。
//! 正文只在内存里解析，不落盘、不缓存。

use std::collections::HashMap;
#[cfg(windows)]
#[path = "mail_fetch.rs"]
mod fetch;

pub const SERVICE: &str = "NexoraHalo.Mail";

// ---------------------------------------------------------------- 邮箱类型

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Qq,
    Mail163,
    Mail126,
    Exmail,
    ICloud,
    Gmail,
    Custom,
}

impl Default for Provider {
    fn default() -> Self {
        Provider::Qq
    }
}

impl Provider {
    pub const ALL: [Provider; 7] = [
        Provider::Qq,
        Provider::Mail163,
        Provider::Mail126,
        Provider::Exmail,
        Provider::ICloud,
        Provider::Gmail,
        Provider::Custom,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Provider::Qq => "qq",
            Provider::Mail163 => "163",
            Provider::Mail126 => "126",
            Provider::Exmail => "exmail",
            Provider::ICloud => "icloud",
            Provider::Gmail => "gmail",
            Provider::Custom => "custom",
        }
    }

    pub fn label(self) -> &'static str {
        self.label_for(crate::i18n::Language::Zh)
    }

    pub fn label_for(self, language: crate::i18n::Language) -> &'static str {
        match self {
            Provider::Qq => language.text("QQ 邮箱", "QQ Mail"),
            Provider::Mail163 => language.text("163 邮箱", "163 Mail"),
            Provider::Mail126 => language.text("126 邮箱", "126 Mail"),
            Provider::Exmail => language.text("腾讯企业邮", "Tencent Exmail"),
            Provider::ICloud => language.text("iCloud 邮箱", "iCloud Mail"),
            Provider::Gmail => "Gmail",
            Provider::Custom => language.text("自定义 IMAP", "Custom IMAP"),
        }
    }

    pub fn host(self) -> &'static str {
        match self {
            Provider::Qq => "imap.qq.com",
            Provider::Mail163 => "imap.163.com",
            Provider::Mail126 => "imap.126.com",
            Provider::Exmail => "imap.exmail.qq.com",
            Provider::ICloud => "imap.mail.me.com",
            Provider::Gmail => "imap.gmail.com",
            Provider::Custom => "",
        }
    }

    pub fn port(self) -> u16 {
        993
    }

    /// 163 / 126 在 LOGIN 之前必须发 RFC 2971 的 `ID`，否则会被判 Unsafe Login。
    pub fn needs_id(self) -> bool {
        matches!(self, Provider::Mail163 | Provider::Mail126)
    }

    /// 设置界面的操作指引。
    pub fn hint(self) -> &'static str {
        self.hint_for(crate::i18n::Language::Zh)
    }

    pub fn hint_for(self, language: crate::i18n::Language) -> &'static str {
        match self {
            Provider::Custom => language.text(
                "填写服务器提供的 IMAP over TLS 地址、端口和登录用户名。仅支持应用密码或授权码；更换服务器或用户名后需要重新输入密码。",
                "Enter the IMAP over TLS host, port, and login name provided by the server. App passwords or authorization codes are required; changing the host or username requires entering the password again.",
            ),
            Provider::Qq => language.text(
                "先在 QQ 邮箱网页版的「账号与安全」中开启 IMAP，再生成授权码。下方填写邮箱地址和授权码，不是 QQ 登录密码。",
                "Enable IMAP in QQ Mail → Account & Security, then generate an authorization code. Enter the mailbox address and code here, not your QQ password.",
            ),
            Provider::Mail163 | Provider::Mail126 => language.text(
                "在网易邮箱的「POP3/SMTP/IMAP」中开启 IMAP，再到授权密码管理里生成客户端授权码。下方请填写该授权码。",
                "Enable IMAP under POP3/SMTP/IMAP in NetEase Mail, then create a client authorization password. Enter that authorization code below.",
            ),
            Provider::Exmail => language.text(
                "由企业邮管理员确认 IMAP 已开启。启用安全登录时，请使用客户端专用密码；不同企业版本的入口名称可能不同。",
                "Confirm with the Exmail administrator that IMAP is enabled. When secure login is on, use a client-specific password; menu names vary by tenant.",
            ),
            Provider::ICloud => language.text(
                "在 Apple 账户的「登录与安全」中生成 App 专用密码，账户需先开启双重认证。下方不要填写 Apple 账户密码。",
                "Generate an app-specific password under Apple Account → Sign-In and Security. Two-factor authentication is required; do not use your Apple Account password.",
            ),
            Provider::Gmail => language.text(
                "先开启 Google 两步验证，再在 Google Account 中创建 16 位应用专用密码。若没有创建入口，请检查组织限制或高级保护设置。",
                "Enable Google 2-Step Verification, then create a 16-character app password in your Google Account. If unavailable, check organization restrictions or Advanced Protection.",
            ),
        }
    }

    pub fn setup_link(self) -> (&'static str, &'static str) {
        self.setup_link_for(crate::i18n::Language::Zh)
    }

    pub fn setup_link_for(self, language: crate::i18n::Language) -> (&'static str, &'static str) {
        match self {
            Provider::Qq => (
                language.text("打开 QQ 邮箱网页版", "Open QQ Mail"),
                "https://mail.qq.com/",
            ),
            Provider::Mail163 => (
                language.text("打开 163 邮箱网页版", "Open 163 Mail"),
                "https://mail.163.com/",
            ),
            Provider::Mail126 => (
                language.text("打开 126 邮箱网页版", "Open 126 Mail"),
                "https://mail.126.com/",
            ),
            Provider::Exmail => (
                language.text("打开腾讯企业邮网页版", "Open Tencent Exmail"),
                "https://exmail.qq.com/",
            ),
            Provider::ICloud => (
                language.text("打开 Apple 账户", "Open Apple Account"),
                "https://account.apple.com/",
            ),
            Provider::Gmail => (
                language.text("打开应用专用密码", "Open App passwords"),
                "https://myaccount.google.com/apppasswords",
            ),
            Provider::Custom => ("", ""),
        }
    }

    pub fn parse(key: &str) -> Self {
        match key {
            "163" => Provider::Mail163,
            "126" => Provider::Mail126,
            "exmail" => Provider::Exmail,
            "icloud" => Provider::ICloud,
            "gmail" => Provider::Gmail,
            "custom" => Provider::Custom,
            _ => Provider::Qq,
        }
    }

    /// Common mailbox domains do not need an extra provider-selection step.
    pub fn for_email(email: &str) -> Option<Self> {
        let domain = email.trim().rsplit_once('@')?.1.to_ascii_lowercase();
        match domain.as_str() {
            "qq.com" | "vip.qq.com" | "foxmail.com" => Some(Self::Qq),
            "163.com" => Some(Self::Mail163),
            "126.com" => Some(Self::Mail126),
            "exmail.qq.com" => Some(Self::Exmail),
            "icloud.com" | "me.com" | "mac.com" => Some(Self::ICloud),
            "gmail.com" | "googlemail.com" => Some(Self::Gmail),
            _ => None,
        }
    }

    /// Webmail links are common paste mistakes. Recognize the provider without
    /// pretending that the URL itself is a mailbox account.
    pub fn for_web_url(raw: &str) -> Option<Self> {
        let url = url::Url::parse(raw.trim()).ok()?;
        let host = url.host_str()?.trim_end_matches('.').to_ascii_lowercase();
        match host.as_str() {
            "mail.163.com" | "webmail.163.com" => Some(Self::Mail163),
            "mail.126.com" | "webmail.126.com" => Some(Self::Mail126),
            "mail.qq.com" => Some(Self::Qq),
            "exmail.qq.com" => Some(Self::Exmail),
            "www.icloud.com" | "icloud.com" | "account.apple.com" => Some(Self::ICloud),
            "mail.google.com" | "gmail.com" | "accounts.google.com" => Some(Self::Gmail),
            _ => None,
        }
    }

    pub fn login_example(self) -> &'static str {
        match self {
            Self::Qq => "123456@qq.com",
            Self::Mail163 => "name@163.com",
            Self::Mail126 => "name@126.com",
            Self::Exmail => "name@company.com",
            Self::ICloud => "name@icloud.com",
            Self::Gmail => "name@gmail.com",
            Self::Custom => "name@example.com",
        }
    }
}

// ---------------------------------------------------------------- 账号与状态

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub email: String,
    pub provider: Provider,
    pub custom: Option<CustomImap>,
    /// 是否监控这个账号（设置里可逐个开关）。
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomImap {
    pub host: String,
    pub port: u16,
    pub username: String,
}
impl Default for CustomImap {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 993,
            username: String::new(),
        }
    }
}
impl Account {
    pub fn endpoint(&self) -> Result<(String, u16, String), String> {
        if self.provider != Provider::Custom {
            return Ok((
                self.provider.host().into(),
                self.provider.port(),
                self.email.clone(),
            ));
        }
        let c = self.custom.as_ref().ok_or("请填写自定义 IMAP 连接配置")?;
        let host = c.host.trim().to_ascii_lowercase();
        if host.is_empty()
            || host.len() > 253
            || !host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
        {
            return Err("IMAP 主机必须是域名或 IPv4 地址，不要填写网址、端口、路径或账号".into());
        }
        if c.port == 0 || c.port == 143 {
            return Err("仅支持 IMAP over TLS（通常 993），不支持 143/STARTTLS 或明文连接".into());
        }
        let user = if c.username.trim().is_empty() {
            self.email.trim()
        } else {
            c.username.trim()
        };
        if user.is_empty() || user.len() > 256 || user.chars().any(char::is_control) {
            return Err("IMAP 用户名无效".into());
        }
        Ok((host, c.port, user.into()))
    }
    /// Custom secrets are bound to the TLS endpoint and exact login identity.
    pub fn credential_key(&self) -> String {
        if self.provider != Provider::Custom {
            return key(self);
        }
        let identity = match self.endpoint() {
            Ok((host, port, user)) => format!("{}\0{host}\0{port}\0{user}", key(self)),
            Err(_) => "invalid-custom-endpoint".into(),
        };
        let digest = ring::digest::digest(&ring::digest::SHA256, identity.as_bytes());
        format!(
            "custom-{}",
            digest
                .as_ref()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        )
    }
}

pub fn key(account: &Account) -> String {
    account.email.trim().to_ascii_lowercase()
}

fn accounts_path() -> std::path::PathBuf {
    crate::model::config_dir().join("mail-accounts.json")
}

pub fn load_accounts() -> Vec<Account> {
    match std::fs::read(accounts_path()) {
        Ok(raw) => decode_accounts(&raw),
        Err(_) => Vec::new(),
    }
}

/// 纯解析：账号清单里只有邮箱、类型、开关，没有任何密码字段。
pub fn decode_accounts(raw: &[u8]) -> Vec<Account> {
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    json.as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let email = a.get("email")?.as_str()?.trim().to_string();
            if email.is_empty() {
                return None;
            }
            Some(Account {
                custom: a.get("custom").and_then(|c| {
                    Some(CustomImap {
                        host: c.get("host")?.as_str()?.into(),
                        port: u16::try_from(c.get("port")?.as_u64()?).ok()?,
                        username: c
                            .get("username")
                            .and_then(|u| u.as_str())
                            .unwrap_or("")
                            .into(),
                    })
                }),
                provider: Provider::parse(
                    a.get("provider").and_then(|p| p.as_str()).unwrap_or("qq"),
                ),
                enabled: a.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true),
                email,
            })
        })
        .collect()
}

pub fn encode_accounts(items: &[Account]) -> Vec<u8> {
    let list: Vec<_> = items
        .iter()
        .map(|a| serde_json::json!({"email": a.email, "provider": a.provider.key(), "enabled": a.enabled,"custom":a.custom.as_ref().map(|c|serde_json::json!({"host":c.host,"port":c.port,"username":c.username}))}))
        .collect();
    serde_json::to_vec(&list).unwrap_or_default()
}

pub fn save_accounts(items: &[Account]) -> Result<(), String> {
    let data = encode_accounts(items);
    if data.is_empty() {
        return Err("邮箱账号清单序列化失败".into());
    }
    let path = accounts_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("创建邮箱数据目录失败：{error}"))?;
    }
    crate::model::atomic_write(&path, &data).map_err(|error| format!("保存邮箱账号失败：{error}"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Idle,
    Connecting,
    Ready,
    /// 授权码缺失或失效。
    Auth(String),
    Failed(String),
}

impl Status {
    pub fn label(&self) -> String {
        self.label_for(crate::i18n::Language::Zh)
    }

    pub fn label_for(&self, language: crate::i18n::Language) -> String {
        match self {
            Status::Idle => language.text("未连接", "Not connected").into(),
            Status::Connecting => language.text("连接中…", "Connecting…").into(),
            Status::Ready => language.text("已连接", "Connected").into(),
            Status::Auth(msg) => {
                if language.is_en() {
                    format!("App password required: {msg}")
                } else {
                    format!("需要授权码：{msg}")
                }
            }
            Status::Failed(msg) => {
                if language.is_en() {
                    format!("Connection failed: {msg}")
                } else {
                    format!("连接失败：{msg}")
                }
            }
        }
    }
    pub fn ok(&self) -> bool {
        matches!(self, Status::Ready)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub account: String,
    pub from: String,
    pub subject: String,
    /// 本地时间 `MM-DD HH:MM`
    pub time: String,
    pub code: Option<String>,
    /// IMAP INTERNALDATE, used only to bound code display age.
    pub received_unix: Option<i64>,
    /// 完整纯文字正文，保留分段，仅保存在内存中。
    pub summary: std::sync::Arc<str>,
    pub links: Vec<Link>,
    pub uid: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub label: String,
    pub url: String,
}

/// 仅把有效网页链接交给默认浏览器。
pub fn web_url(raw: &str) -> Option<String> {
    let parsed = url::Url::parse(raw.trim()).ok()?;
    if !matches!(parsed.scheme(), "https" | "http") || parsed.host_str().is_none() {
        return None;
    }
    Some(parsed.into())
}

pub fn links_of(parsed: &Parsed) -> Vec<Link> {
    use std::sync::OnceLock;
    static ANCHOR: OnceLock<regex::Regex> = OnceLock::new();
    static URL: OnceLock<regex::Regex> = OnceLock::new();
    let anchor = ANCHOR.get_or_init(|| {
        regex::Regex::new(
            r#"(?is)<a\b[^>]*?\s+href\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))[^>]*>(.*?)</a\s*>"#,
        )
        .unwrap()
    });
    let url =
        URL.get_or_init(|| regex::Regex::new(r#"(?i)https?://[^\s<>"'，。；！？、]+"#).unwrap());
    let mut links = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut add = |raw: &str, label: String| {
        let decoded = otp::html_to_text(raw);
        if let Some(url) = web_url(decoded.trim()) {
            if seen.insert(url.clone()) {
                links.push(Link {
                    label: if label.trim().is_empty() {
                        url.clone()
                    } else {
                        label.trim().into()
                    },
                    url,
                });
            }
        }
    };
    for cap in anchor.captures_iter(&parsed.html) {
        if let Some(href) = cap.get(1).or_else(|| cap.get(2)).or_else(|| cap.get(3)) {
            add(href.as_str(), otp::html_to_text(&cap[4]));
        }
    }
    for text in [&parsed.text, &otp::html_to_text(&parsed.html)] {
        for hit in url.find_iter(text) {
            let mut target = hit.as_str().trim_end_matches([
                '.', ',', ';', ':', '!', '，', '。', '；', '！', '？', '）', '】', '》',
            ]);
            for (open, close) in [('(', ')'), ('[', ']'), ('{', '}')] {
                while target.ends_with(close)
                    && target.chars().filter(|c| *c == close).count()
                        > target.chars().filter(|c| *c == open).count()
                {
                    target = &target[..target.len() - 1];
                }
            }
            add(target, target.into());
        }
    }
    links
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Status {
        account: String,
        status: Status,
    },
    Mail {
        item: Item,
        new: bool,
    },
    Test {
        account: String,
        ok: bool,
        note: String,
    },
}

// ---------------------------------------------------------------- 正文解析

pub struct Parsed {
    pub from: String,
    pub subject: String,
    /// text/plain 正文（没有则空）。
    pub text: String,
    /// text/html 正文转成的纯文本（没有则空）。
    pub html: String,
    pub emphasized: Vec<String>,
}

fn header(mail: &mailparse::ParsedMail, name: &str) -> String {
    mail.headers
        .iter()
        .find(|h| h.get_key().eq_ignore_ascii_case(name))
        .map(|h| h.get_value().trim().to_string())
        .unwrap_or_default()
}

fn display_addr(raw: &str) -> String {
    mailparse::addrparse(raw)
        .ok()
        .and_then(|list| {
            list.iter().find_map(|a| match a {
                mailparse::MailAddr::Single(s) => {
                    Some(s.display_name.clone().unwrap_or_else(|| s.addr.clone()))
                }
                mailparse::MailAddr::Group(g) => g
                    .addrs
                    .first()
                    .map(|s| s.display_name.clone().unwrap_or_else(|| s.addr.clone())),
            })
        })
        .unwrap_or_else(|| raw.trim().trim_matches('"').to_string())
}

fn collect(mail: &mailparse::ParsedMail, plain: &mut String, html: &mut String) {
    if mail.subparts.is_empty() {
        let ct = mail.ctype.mimetype.to_ascii_lowercase();
        if ct == "text/plain" {
            if let Ok(body) = mail.get_body() {
                plain.push_str(&body);
                plain.push('\n');
            }
        } else if ct == "text/html" {
            if let Ok(body) = mail.get_body() {
                html.push_str(&body);
                html.push('\n');
            }
        }
    } else {
        for part in &mail.subparts {
            collect(part, plain, html);
        }
    }
}

/// 解析一封原始邮件（头部 + 正文），只保留界面上要用的字段。
pub fn parse_message(raw: &[u8]) -> Parsed {
    let Ok(mail) = mailparse::parse_mail(raw) else {
        return Parsed {
            from: String::new(),
            subject: String::new(),
            text: String::from_utf8_lossy(raw).into_owned(),
            html: String::new(),
            emphasized: Vec::new(),
        };
    };
    let subject = header(&mail, "Subject");
    let from = display_addr(&header(&mail, "From"));
    let (mut plain, mut html) = (String::new(), String::new());
    collect(&mail, &mut plain, &mut html);
    let emphasized = if plain.trim().is_empty() {
        otp::emphasized(&html)
    } else {
        Vec::new()
    };
    Parsed {
        from,
        subject,
        text: plain,
        html,
        emphasized,
    }
}

/// 优先纯文本，HTML 邮件转成文字；保留完整正文与段落。
pub fn body_of(parsed: &Parsed) -> String {
    let body = if !parsed.text.trim().is_empty() {
        parsed.text.clone()
    } else if !parsed.html.trim().is_empty() {
        otp::html_to_text(&parsed.html)
    } else {
        String::new()
    };
    body.replace("\r\n", "\n").trim().to_string()
}

/// 紧凑场景使用的短摘要。
pub fn summary_of(parsed: &Parsed) -> String {
    let body = body_of(parsed);
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out: String = flat.chars().take(200).collect();
    if flat.chars().count() > 200 {
        out.push('…');
    }
    out
}

/// 从解析结果里提验证码：先纯文本，纯文本没有再看 HTML 文本。
pub fn code_of(parsed: &Parsed) -> Option<String> {
    if let Some(c) = otp::extract(&parsed.subject, &parsed.text, &parsed.emphasized) {
        return Some(c.text);
    }
    if !parsed.html.trim().is_empty() {
        let text = otp::html_to_text(&parsed.html);
        if let Some(c) = otp::extract(&parsed.subject, &text, &parsed.emphasized) {
            return Some(c.text);
        }
    }
    None
}

use crate::otp;

// ---------------------------------------------------------------- 凭据

#[cfg(windows)]
pub mod secrets {
    /// 凭据只进 Windows 凭据管理器；键是邮箱地址，值是授权码。
    pub fn save(email: &str, password: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(super::SERVICE, &email.to_ascii_lowercase())
            .map_err(|e| format!("凭据管理器不可用：{e}"))?;
        entry
            .set_password(password)
            .map_err(|e| format!("保存授权码失败：{e}"))
    }

    pub fn load(email: &str) -> Result<String, String> {
        let entry = keyring::Entry::new(super::SERVICE, &email.to_ascii_lowercase())
            .map_err(|e| format!("凭据管理器不可用：{e}"))?;
        entry
            .get_password()
            .map_err(|_| "没有保存授权码".to_string())
    }

    pub fn delete(email: &str) {
        if let Ok(entry) = keyring::Entry::new(super::SERVICE, &email.to_ascii_lowercase()) {
            let _ = entry.delete_credential();
        }
    }
}

#[cfg(not(windows))]
pub mod secrets {
    pub fn save(_email: &str, _password: &str) -> Result<(), String> {
        Err("仅 Windows 支持凭据管理器".into())
    }
    pub fn load(_email: &str) -> Result<String, String> {
        Err("仅 Windows 支持凭据管理器".into())
    }
    pub fn delete(_email: &str) {}
}

// ---------------------------------------------------------------- 收信服务

#[cfg(windows)]
mod net {
    //! TLS 连接 + 163/126 的 pre-login `ID` 握手。
    //!
    //! `imap` 2.4 只在已登录的 `Session` 上暴露 `run_command_and_check_ok`，
    //! 而网易要求 greeting → ID → LOGIN 的顺序，所以这里手工读问候、发 ID，
    //! 再把「已经读掉的开头字节」原样交回给 imap。

    use super::Account;
    use imap::extensions::idle::SetReadTimeout;
    use native_tls::{TlsConnector, TlsStream};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    pub type Stream = TlsStream<TcpStream>;

    /// 先吐出预先读到的问候字节，再透传给底层流。
    pub struct Prefixed<S> {
        head: Vec<u8>,
        pos: usize,
        inner: S,
    }

    impl<S> Prefixed<S> {
        fn new(head: Vec<u8>, inner: S) -> Self {
            Self {
                head,
                pos: 0,
                inner,
            }
        }
    }

    impl<S: Read> Read for Prefixed<S> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.pos < self.head.len() {
                let n = (self.head.len() - self.pos).min(buf.len());
                buf[..n].copy_from_slice(&self.head[self.pos..self.pos + n]);
                self.pos += n;
                Ok(n)
            } else {
                self.inner.read(buf)
            }
        }
    }

    impl<S: Write> Write for Prefixed<S> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.inner.write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }

    impl SetReadTimeout for Prefixed<Stream> {
        fn set_read_timeout(&mut self, timeout: Option<Duration>) -> imap::Result<()> {
            self.inner
                .get_ref()
                // imap 结束 IDLE 时传入 None；恢复普通命令的超时，避免无限阻塞。
                .set_read_timeout(timeout.or(Some(Duration::from_secs(20))))
                .map_err(imap::Error::Io)
        }
    }

    fn read_line(stream: &mut Stream, buf: &mut Vec<u8>) -> Result<(), String> {
        let mut byte = [0u8; 1];
        loop {
            match stream.read(&mut byte) {
                Ok(0) => return Err("服务器关闭了连接".into()),
                Ok(_) => {
                    buf.push(byte[0]);
                    if buf.ends_with(b"\r\n") {
                        return Ok(());
                    }
                    if buf.len() > 8192 {
                        return Err("服务器响应过长".into());
                    }
                }
                Err(e) => return Err(format!("读取失败：{e}")),
            }
        }
    }

    pub fn connect(
        account: &Account,
        password: &str,
    ) -> Result<imap::Session<Prefixed<Stream>>, String> {
        let tls = TlsConnector::builder()
            .min_protocol_version(Some(native_tls::Protocol::Tlsv12))
            .build()
            .map_err(|e| format!("TLS 初始化失败：{e}"))?;
        let (host, port, username) = account.endpoint()?;
        let tcp = TcpStream::connect((host.as_str(), port))
            .map_err(|e| format!("无法连接 {host}：{e}"))?;
        let timeout = Some(Duration::from_secs(20));
        let _ = tcp.set_read_timeout(timeout);
        let _ = tcp.set_write_timeout(timeout);
        let mut stream = tls
            .connect(&host, tcp)
            .map_err(|e| format!("TLS 握手失败：{e}"))?;

        // 1) 读问候，直到 "* OK" / "* PREAUTH"
        let mut greeting = Vec::new();
        loop {
            let mut line = Vec::new();
            read_line(&mut stream, &mut line)?;
            let upper = String::from_utf8_lossy(&line).to_ascii_uppercase();
            greeting.extend_from_slice(&line);
            if upper.starts_with("* BYE") {
                return Err("服务器拒绝连接".into());
            }
            if upper.starts_with("* OK") || upper.starts_with("* PREAUTH") {
                break;
            }
            if greeting.len() > 8192 {
                return Err("服务器问候异常".into());
            }
        }

        // 2) 网易系：LOGIN 之前先发 ID（RFC 2971）
        if account.provider.needs_id() {
            stream
                .write_all(b"A0001 ID (\"name\" \"nexora-halo\" \"version\" \"0.5.5\" \"vendor\" \"nexora-halo\")\r\n")
                .map_err(|e| format!("发送 ID 失败：{e}"))?;
            let _ = stream.flush();
            let mut ok = false;
            for _ in 0..20 {
                let mut line = Vec::new();
                read_line(&mut stream, &mut line)?;
                let text = String::from_utf8_lossy(&line).to_string();
                if text.starts_with("A0001 ") {
                    ok = text.to_ascii_uppercase().contains(" OK");
                    break;
                }
            }
            if !ok {
                return Err("服务器不接受 ID 命令".into());
            }
        }

        // 3) 把问候交回 imap，再正常登录
        let mut client = imap::Client::new(Prefixed::new(greeting, stream));
        client
            .read_greeting()
            .map_err(|e| format!("服务器问候异常：{e}"))?;
        let session = client
            .login(username, password)
            .map_err(|(e, _)| login_error(e))?;
        Ok(session)
    }

    pub fn login_error(e: imap::Error) -> String {
        match e {
            imap::Error::No(_) | imap::Error::Bad(_) => {
                "登录被拒绝，请检查用户名、授权码和管理员的 IMAP/认证策略（不支持 OAuth-only）"
                    .into()
            }
            imap::Error::Io(_) => "登录时网络连接中断，请重试".into(),
            _ => "IMAP 登录失败，请核对服务器及认证方式".into(),
        }
    }

    /// 测试连接：连上、登录、选 INBOX，然后退出。
    pub fn probe(account: &Account, password: &str) -> Result<(), String> {
        let mut session = connect(account, password)?;
        session
            .select("INBOX")
            .map_err(|e| format!("无法打开收件箱：{e}"))?;
        let _ = session.logout();
        Ok(())
    }
}

#[cfg(windows)]
mod service {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::Arc;
    use std::time::Duration;

    enum Cmd {
        Sync(Vec<Account>, u32),
        Test(Account, String),
    }

    pub struct Service {
        cmd: Sender<Cmd>,
        ev: Receiver<Event>,
    }

    impl Service {
        pub fn start() -> Self {
            let (ev_tx, ev_rx) = channel();
            let (cmd_tx, cmd_rx) = channel();
            let _ = std::thread::Builder::new()
                .name("nexora-halo-mail".into())
                .spawn(move || supervisor(cmd_rx, ev_tx));
            Self {
                cmd: cmd_tx,
                ev: ev_rx,
            }
        }

        pub fn sync(&self, accounts: &[Account], interval_secs: u32) {
            let _ = self.cmd.send(Cmd::Sync(accounts.to_vec(), interval_secs));
        }

        pub fn test(&self, account: Account, password: String) {
            let _ = self.cmd.send(Cmd::Test(account, password));
        }

        pub fn try_recv(&self) -> Option<Event> {
            self.ev.try_recv().ok()
        }
    }

    fn supervisor(cmd_rx: Receiver<Cmd>, ev: Sender<Event>) {
        let mut workers: HashMap<String, (Arc<AtomicBool>, std::thread::JoinHandle<()>, Account)> =
            HashMap::new();
        let interval = Arc::new(AtomicU64::new(120));
        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                Cmd::Sync(accounts, secs) => {
                    interval.store(secs.clamp(30, 900) as u64, Ordering::Relaxed);
                    let wanted: HashMap<String, Account> = accounts
                        .into_iter()
                        .filter(|a| a.enabled)
                        .map(|a| (key(&a), a))
                        .collect();
                    let drop: Vec<String> = workers
                        .keys()
                        .filter(|k| wanted.get(*k) != workers.get(*k).map(|worker| &worker.2))
                        .cloned()
                        .collect();
                    for k in drop {
                        if let Some((flag, _, _)) = workers.remove(&k) {
                            flag.store(true, Ordering::Relaxed);
                        }
                    }
                    for (k, account) in wanted {
                        if workers.contains_key(&k) {
                            continue;
                        }
                        let stop = Arc::new(AtomicBool::new(false));
                        let flag = stop.clone();
                        let ev = ev.clone();
                        let iv = interval.clone();
                        let name = "nexora-halo-mail".to_string();
                        let running_account = account.clone();
                        if let Ok(handle) = std::thread::Builder::new()
                            .name(name)
                            .spawn(move || worker(account, flag, iv, ev))
                        {
                            workers.insert(k, (stop, handle, running_account));
                        }
                    }
                }
                Cmd::Test(account, password) => {
                    let ev = ev.clone();
                    let _ = std::thread::Builder::new()
                        .name("nexora-halo-mail-test".into())
                        .spawn(move || {
                            let result = net::probe(&account, &password);
                            let _ = ev.send(Event::Test {
                                account: account.email,
                                ok: result.is_ok(),
                                note: result.err().unwrap_or_else(|| "连接成功".into()),
                            });
                        });
                }
            }
        }
        for (_, (flag, _, _)) in workers {
            flag.store(true, Ordering::Relaxed);
        }
    }

    fn sleep_stop(stop: &AtomicBool, secs: u64) -> bool {
        for _ in 0..secs * 2 {
            if stop.load(Ordering::Relaxed) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        false
    }

    fn wait_for_change<
        T: std::io::Read + std::io::Write + imap::extensions::idle::SetReadTimeout,
    >(
        session: &mut imap::Session<T>,
    ) -> imap::Result<()> {
        // 优先响应服务器推送；定期补查，避免服务端漏推送时长时间没有提醒。
        session
            .idle()?
            .wait_with_timeout(Duration::from_secs(10))
            .map(|_| ())
    }

    #[test]
    fn idle_without_push_returns_for_a_fallback_check() {
        use std::io::{BufRead, Write};
        for push in [true, false] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(14)))
                    .unwrap();
                let mut reader = std::io::BufReader::new(socket.try_clone().unwrap());
                write!(socket, "* OK ready\r\n").unwrap();
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                write!(
                    socket,
                    "{} OK logged in\r\n",
                    line.split_whitespace().next().unwrap()
                )
                .unwrap();
                line.clear();
                reader.read_line(&mut line).unwrap();
                let tag = line.split_whitespace().next().unwrap().to_string();
                assert!(line.contains("IDLE"));
                write!(socket, "+ idling\r\n").unwrap();
                if push {
                    write!(socket, "* 1 EXISTS\r\n").unwrap();
                }
                line.clear();
                if reader.read_line(&mut line).is_err() {
                    return false;
                }
                assert_eq!(line, "DONE\r\n");
                write!(socket, "{tag} OK idle ended\r\n").unwrap();
                true
            });
            let mut client = imap::Client::new(std::net::TcpStream::connect(address).unwrap());
            client.read_greeting().unwrap();
            let mut session = client.login("test", "test").unwrap();
            let start = std::time::Instant::now();
            let result = wait_for_change(&mut session);
            assert!(
                server.join().unwrap(),
                "服务端未推送时，必须在 14 秒内主动结束等待并补查"
            );
            result.unwrap();
            if push {
                assert!(
                    start.elapsed() < Duration::from_secs(2),
                    "有推送时必须立即处理，不能等待检查间隔"
                );
            }
        }
    }

    fn worker(
        account: Account,
        stop: Arc<AtomicBool>,
        interval: Arc<AtomicU64>,
        ev: Sender<Event>,
    ) {
        let mut first = true;
        let mut last_uid = 0u32;
        loop {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            let _ = ev.send(Event::Status {
                account: account.email.clone(),
                status: Status::Connecting,
            });
            let password = match secrets::load(&account.credential_key()) {
                Ok(p) => p,
                Err(e) => {
                    let _ = ev.send(Event::Status {
                        account: account.email.clone(),
                        status: Status::Auth(e),
                    });
                    if sleep_stop(&stop, 60) {
                        return;
                    }
                    continue;
                }
            };
            let mut session = match net::connect(&account, &password) {
                Ok(s) => s,
                Err(e) => {
                    let _ = ev.send(Event::Status {
                        account: account.email.clone(),
                        status: Status::Failed(e),
                    });
                    if sleep_stop(&stop, 30) {
                        return;
                    }
                    continue;
                }
            };
            if stop.load(Ordering::Relaxed) {
                let _ = session.logout();
                return;
            }
            let _ = ev.send(Event::Status {
                account: account.email.clone(),
                status: Status::Ready,
            });
            if session.select("INBOX").is_err() {
                let _ = ev.send(Event::Status {
                    account: account.email.clone(),
                    status: Status::Failed("无法打开收件箱".into()),
                });
                if sleep_stop(&stop, 30) {
                    return;
                }
                continue;
            }
            if first {
                last_uid = recent(&mut session, &account, &ev).unwrap_or(0);
                first = false;
            }

            let idle_supported = match session.capabilities() {
                Ok(capabilities) => capabilities.has_str("IDLE"),
                Err(_) => {
                    if sleep_stop(&stop, 10) {
                        return;
                    }
                    continue;
                }
            };
            loop {
                if stop.load(Ordering::Relaxed) {
                    let _ = session.logout();
                    return;
                }
                if idle_supported {
                    // 推送连接出错后重连，不能永久降级为轮询。
                    if wait_for_change(&mut session).is_err() {
                        break;
                    }
                } else {
                    if sleep_stop(&stop, interval.load(Ordering::Relaxed)) {
                        let _ = session.logout();
                        return;
                    }
                    if session.noop().is_err() {
                        break;
                    }
                }
                if stop.load(Ordering::Relaxed) {
                    let _ = session.logout();
                    return;
                }
                if fetch_new(&mut session, &account, &mut last_uid, &ev).is_err() {
                    break;
                }
            }
            if sleep_stop(&stop, 10) {
                return;
            }
        }
    }

    /// 首次连接：把最近 20 封读进面板，但不当作新邮件提示。
    fn recent(
        session: &mut imap::Session<net::Prefixed<net::Stream>>,
        account: &Account,
        ev: &Sender<Event>,
    ) -> Result<u32, ()> {
        let all = session.uid_search("ALL").map_err(|_| ())?;
        let mut uids: Vec<u32> = all.into_iter().collect();
        uids.sort_unstable();
        let max = uids.last().copied().unwrap_or(0);
        let tail: Vec<u32> = uids.iter().rev().take(20).rev().copied().collect();
        for uid in tail {
            match super::fetch::item(session, uid, account) {
                Ok(Some(item)) => {
                    let _ = ev.send(Event::Mail { item, new: false });
                }
                Ok(None) => {}
                // 连接已失步：停止使用这条连接，后面的命令会失败并触发重连。
                Err(()) => break,
            }
        }
        Ok(max)
    }

    #[test]
    #[ignore = "需要真实 QQ 邮箱及新测试邮件，验证后台监听"]
    fn qq_live_notification_worker() {
        let accounts = load_accounts()
            .into_iter()
            .filter(|a| a.provider == Provider::Qq && a.enabled)
            .collect::<Vec<_>>();
        assert!(!accounts.is_empty());
        let service = Service::start();
        service.sync(&accounts, 120);
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(160) {
            if let Ok(event) = service.ev.recv_timeout(Duration::from_secs(1)) {
                match event {
                    Event::Status { status, .. } => println!("worker_status={status:?}"),
                    Event::Mail { new, item } => {
                        println!(
                            "worker_mail new={new} test_message={}",
                            item.subject.starts_with("NEXORA HALO")
                        );
                        if new && item.subject == "NEXORA HALO后台监听实测" {
                            service.sync(&[], 120);
                            return;
                        }
                    }
                    _ => {}
                }
            }
        }
        service.sync(&[], 120);
        panic!("160 秒内未收到新邮件事件");
    }
    /// 显式运行的真实邮箱只读检查；不输出地址、凭据或邮件内容。
    #[test]
    #[ignore = "需要本机已连接的 QQ 邮箱，显式执行只读实测"]
    fn qq_live_read_only_probe() {
        let accounts = load_accounts();
        let account = accounts
            .iter()
            .find(|a| a.provider == Provider::Qq && a.enabled)
            .expect("未找到已启用 QQ 邮箱");
        let password = secrets::load(&account.credential_key()).expect("读取邮箱凭据失败");
        let mut session = net::connect(account, &password).expect("真实登录失败");
        session.examine("INBOX").expect("只读打开收件箱失败");
        println!(
            "idle_supported={}",
            session
                .capabilities()
                .expect("读取能力失败")
                .has_str("IDLE")
        );
        let before = session.uid_search("UNSEEN").expect("读取未读状态失败");
        let (tx, rx) = std::sync::mpsc::channel();
        let mut last = recent(&mut session, account, &tx).expect("最近邮件读取失败");
        let initial = rx.try_iter().collect::<Vec<_>>();
        assert!(initial
            .iter()
            .all(|e| matches!(e, Event::Mail { new: false, .. })));
        println!("login=ok readonly_inbox=ok recent_parsed={}", initial.len());
        for event in &initial {
            if let Event::Mail { item, .. } = event {
                println!(
                    "subject_chars={} sender_chars={} body_bytes={} links={} max_link_chars={}",
                    item.subject.chars().count(),
                    item.from.chars().count(),
                    item.summary.len(),
                    item.links.len(),
                    item.links
                        .iter()
                        .map(|l| l.label.chars().count())
                        .max()
                        .unwrap_or(0)
                );
            }
        }
        let received=initial.iter().any(|e|matches!(e,Event::Mail{item,..} if item.subject=="NEXORA HALO收信测试"&&item.code.as_deref()==Some("482731")&&!item.links.is_empty()));
        println!("test_message_with_code_and_link={received}");
        fetch_new(&mut session, account, &mut last, &tx).expect("增量收取失败");
        let added = rx.try_iter().count();
        let after = session.uid_search("UNSEEN").expect("检查未读状态失败");
        println!(
            "incremental_fetch=ok new_messages={} unseen_unchanged={}",
            added,
            before == after
        );
        session.logout().expect("退出失败");
    }

    fn fetch_new(
        session: &mut imap::Session<net::Prefixed<net::Stream>>,
        account: &Account,
        last: &mut u32,
        ev: &Sender<Event>,
    ) -> Result<(), ()> {
        let set = session
            .uid_search(format!("UID {}:*", last.saturating_add(1)))
            .map_err(|_| ())?;
        let mut uids: Vec<u32> = set.into_iter().filter(|u| *u > *last).collect();
        uids.sort_unstable();
        if uids.len() > 5 {
            uids = uids.split_off(uids.len() - 5);
        }
        for uid in uids {
            if let Some(item) = super::fetch::item(session, uid, account)? {
                let _ = ev.send(Event::Mail { item, new: true });
            }
            *last = (*last).max(uid);
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod service {
    use super::*;

    pub struct Service;

    impl Service {
        pub fn start() -> Self {
            Self
        }
        pub fn sync(&self, _accounts: &[Account], _interval_secs: u32) {}
        pub fn test(&self, _account: Account, _password: String) {}
        pub fn try_recv(&self) -> Option<Event> {
            None
        }
    }
}

pub use service::Service;

#[cfg(test)]
mod tests {
    #[test]
    fn mail_setup_copy_follows_the_selected_language() {
        use crate::i18n::Language;

        assert_eq!(super::Provider::Qq.label_for(Language::En), "QQ Mail");
        assert!(super::Provider::Gmail
            .hint_for(Language::En)
            .contains("Google Account"));
        assert_eq!(
            super::Provider::ICloud.setup_link_for(Language::En).0,
            "Open Apple Account"
        );
        assert_eq!(super::Status::Ready.label_for(Language::En), "Connected");
    }

    #[test]
    fn email_links_preserve_targets_labels_and_query_parameters() {
        let parsed=super::parse_message(b"Content-Type: text/html; charset=utf-8\r\n\r\n<a href=\"https://example.com/review?a=1&amp;b=2\"><b>Review</b></a><a href='https://example.com/review?a=1&amp;b=2'>Again</a><a href=javascript:alert(1)>Unsafe</a>");
        assert_eq!(
            super::links_of(&parsed),
            vec![super::Link {
                label: "Review".into(),
                url: "https://example.com/review?a=1&b=2".into()
            }]
        );
        let plain=super::parse_message(b"Content-Type: text/plain\r\n\r\nRead https://example.com/docs. Then https://example.com/help");
        let links = super::links_of(&plain);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].url, "https://example.com/docs");
        assert!(super::web_url("file:///C:/Windows/notepad.exe").is_none());
        assert!(super::web_url("https://").is_none());
        let parens=super::parse_message(b"Content-Type: text/plain\r\n\r\nhttps://example.com/Function_(mathematics). (https://example.com/help)");
        let targets = super::links_of(&parens);
        assert_eq!(targets[0].url, "https://example.com/Function_(mathematics)");
        assert_eq!(targets[1].url, "https://example.com/help");
    }
    use super::*;

    fn raw(subject: &str, body: &str) -> Vec<u8> {
        format!("From: =?UTF-8?B?5bCP54mb?= <hi@example.com>\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{body}").into_bytes()
    }

    #[test]
    fn parses_subject_sender_and_code() {
        let parsed = parse_message(&raw("验证码", "您的验证码是：834921，10分钟内有效。"));
        assert_eq!(parsed.subject, "验证码");
        assert_eq!(parsed.from, "小牛");
        assert_eq!(code_of(&parsed).as_deref(), Some("834921"));
    }

    #[test]
    fn html_only_mail_still_finds_code() {
        let html = "<html><body><p>您正在登录</p><p>验证码：<b>906213</b></p></body></html>";
        let raw = format!(
            "From: a@b.c\r\nSubject: 登录\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{html}"
        );
        let parsed = parse_message(raw.as_bytes());
        assert_eq!(code_of(&parsed).as_deref(), Some("906213"));
    }

    #[test]
    fn accounts_roundtrip_without_secrets() {
        let items = vec![
            Account {
                custom: None,
                email: "a@qq.com".into(),
                provider: Provider::Qq,
                enabled: true,
            },
            Account {
                custom: None,
                email: "b@163.com".into(),
                provider: Provider::Mail163,
                enabled: false,
            },
        ];
        let bytes = encode_accounts(&items);
        assert_eq!(decode_accounts(&bytes), items);
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.contains("password") && !text.contains("auth"));
        assert_eq!(decode_accounts(b"not json"), Vec::new());
    }

    #[test]
    fn provider_presets_cover_six_kinds() {
        assert_eq!(Provider::ALL.len(), 7);
        assert_eq!(Provider::parse("gmail"), Provider::Gmail);
        assert_eq!(
            Provider::for_email("person@GMAIL.com"),
            Some(Provider::Gmail)
        );
        assert_eq!(
            Provider::for_email("person@foxmail.com"),
            Some(Provider::Qq)
        );
        assert_eq!(Provider::for_email("person@company.example"), None);
        assert_eq!(
            Provider::for_web_url("https://mail.163.com/js6/main.jsp?df=mail163"),
            Some(Provider::Mail163)
        );
        assert_eq!(
            Provider::for_web_url("https://mail.qq.com/"),
            Some(Provider::Qq)
        );
        assert_eq!(Provider::for_web_url("https://example.com/"), None);
        assert_eq!(Provider::Mail163.login_example(), "name@163.com");
        assert_eq!(Provider::Gmail.host(), "imap.gmail.com");
        assert_eq!(Provider::Gmail.port(), 993);
        let gmail = Account {
            custom: None,
            email: "demo@gmail.com".into(),
            provider: Provider::Gmail,
            enabled: true,
        };
        assert_eq!(
            decode_accounts(&encode_accounts(&[gmail.clone()])),
            vec![gmail]
        );
        assert!(Provider::Mail163.needs_id() && Provider::Mail126.needs_id());
        assert!(!Provider::Qq.needs_id());
        assert_eq!(Provider::parse("exmail").host(), "imap.exmail.qq.com");
    }

    #[test]
    fn summary_flattens_body_and_caps_length() {
        let parsed = parse_message(&raw("周会", "第一行\n\n  第二行   有空格\r\n第三行"));
        assert_eq!(summary_of(&parsed), "第一行 第二行 有空格 第三行");
        let long = "字".repeat(400);
        let parsed = parse_message(&raw("长", &long));
        let s = summary_of(&parsed);
        assert_eq!(s.chars().count(), 201, "200 字 + 省略号");
        assert!(s.ends_with('…'));
        // 没有正文时摘要为空，界面只显示主题
        assert_eq!(summary_of(&parse_message(&raw("空", ""))), "");
    }

    #[test]
    fn full_body_keeps_paragraphs_and_content_beyond_preview_and_old_fetch_limit() {
        let text = format!(
            "第一段\n\n{}\n\n最后一段：完整正文结束",
            "正文内容".repeat(10000)
        );
        let parsed = parse_message(&raw("长邮件", &text));
        assert_eq!(body_of(&parsed), text);
        let html=parse_message(b"Content-Type: text/html; charset=utf-8\r\n\r\n<p>First paragraph</p><p>Last paragraph</p>");
        assert_eq!(body_of(&html), "First paragraph\nLast paragraph");
    }
}

#[cfg(test)]
mod custom_endpoint_tests {
    use super::*;
    fn account() -> Account {
        Account {
            email: "student@example.edu".into(),
            provider: Provider::Custom,
            enabled: true,
            custom: Some(CustomImap {
                host: "imap.example.edu".into(),
                port: 993,
                username: "student-id".into(),
            }),
        }
    }
    #[test]
    fn custom_requires_tls_hostname_and_clean_identity() {
        let a = account();
        assert_eq!(
            a.endpoint().unwrap(),
            ("imap.example.edu".into(), 993, "student-id".into())
        );
        for host in [
            "https://imap.example.edu",
            "a/b",
            "a:993",
            "user@host",
            "bad\\host",
            "-bad.example",
            "",
        ] {
            let mut b = a.clone();
            b.custom.as_mut().unwrap().host = host.into();
            assert!(b.endpoint().is_err());
        }
        for port in [0, 143] {
            let mut b = a.clone();
            b.custom.as_mut().unwrap().port = port;
            assert!(b.endpoint().is_err());
        }
        let mut b = a.clone();
        b.custom.as_mut().unwrap().username = "user\r\nLOGIN".into();
        assert!(b.endpoint().is_err());
    }
    #[test]
    fn credentials_are_bound_to_target_and_login() {
        let a = account();
        let k = a.credential_key();
        let mut b = a.clone();
        b.custom.as_mut().unwrap().host = "other.example.edu".into();
        assert_ne!(k, b.credential_key());
        assert_ne!(a, b);
        let mut b = a.clone();
        b.custom.as_mut().unwrap().username = "other-user".into();
        assert_ne!(k, b.credential_key());
        let mut b = a.clone();
        b.custom.as_mut().unwrap().port = 1993;
        assert_ne!(k, b.credential_key());
        let mut b = a.clone();
        b.custom.as_mut().unwrap().host = "IMAP.EXAMPLE.EDU".into();
        assert_eq!(k, b.credential_key());
        assert!(!k.contains("student"));
    }
    #[test]
    fn custom_config_roundtrips_without_a_secret() {
        let a = account();
        let raw = encode_accounts(&[a.clone()]);
        assert_eq!(decode_accounts(&raw), vec![a]);
        let text = String::from_utf8(raw).unwrap();
        assert!(!text.contains("password"));
        assert!(!text.contains("secret"));
    }
}
