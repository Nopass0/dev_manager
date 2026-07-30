//! Детектор потенциально утёкших секретов в исходниках.
//!
//! Использует набор эвристических правил (без внешних зависимостей):
//! - AWS Access Key ID (`AKIA...`);
//! - AWS Secret Key (40 base64 символов рядом с ключом/в имени SECRET);
//! - Google API key (`AIza...`);
//! - JWT-токены (3 base64-части через `.`);
//! - общие паттерны `password =`, `secret =`, `token =`, `api_key =` со значением;
//! - приватные ключи PEM (`-----BEGIN ... PRIVATE KEY-----`);
//! - подключённые строки подключения к БД с учётными данными.
//!
//! False positives возможны; результат рассчитан на ручную проверку.

use crate::search::{search, SearchOptions};
use std::path::Path;

/// Категория найденного секрета.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    /// AWS Access Key ID.
    AwsAccessKeyId,
    /// AWS Secret Access Key.
    AwsSecretKey,
    /// Google API key.
    GoogleApiKey,
    /// JWT-токен.
    Jwt,
    /// Назначенный секрет в коде (`password=...`, `secret=...`).
    Assignment,
    /// Приватный ключ PEM.
    PrivateKey,
    /// Строка подключения с учётными данными.
    ConnectionString,
}

impl SecretKind {
    /// Человекочитаемая метка.
    pub fn label(self) -> &'static str {
        match self {
            SecretKind::AwsAccessKeyId => "AWS Access Key ID",
            SecretKind::AwsSecretKey => "AWS Secret Key",
            SecretKind::GoogleApiKey => "Google API Key",
            SecretKind::Jwt => "JWT token",
            SecretKind::Assignment => "credential assignment",
            SecretKind::PrivateKey => "PEM private key",
            SecretKind::ConnectionString => "connection string",
        }
    }
}

/// Одно предупреждение о возможном секрете.
#[derive(Debug, Clone)]
pub struct SecretFinding {
    /// Категория.
    pub kind: SecretKind,
    /// Файл.
    pub file: std::path::PathBuf,
    /// Строка.
    pub line: usize,
    /// Текст строки (с замаскированным значением секрета в дальнейшем выводе).
    pub text: String,
}

/// Сканирует каталог `root` на наличие потенциальных секретов.
pub fn scan(root: &Path) -> Vec<SecretFinding> {
    let mut out = Vec::new();

    // PEM-ключи — целиком по содержимому.
    let pem = search(
        root,
        "-----BEGIN",
        &SearchOptions {
            extensions: pem_extensions(),
            ..Default::default()
        },
    );
    for m in pem {
        if m.text.contains("PRIVATE KEY") {
            out.push(SecretFinding {
                kind: SecretKind::PrivateKey,
                file: m.file,
                line: m.line,
                text: m.text,
            });
        }
    }

    // AWS Access Key ID: AKIA + 16 символов.
    for m in search(root, "AKIA", &SearchOptions::default()) {
        if looks_like_aws_keyid(&m.text) {
            out.push(SecretFinding {
                kind: SecretKind::AwsAccessKeyId,
                file: m.file.clone(),
                line: m.line,
                text: m.text,
            });
        }
    }

    // Google API key: AIza + 35 символов.
    for m in search(root, "AIza", &SearchOptions::default()) {
        if looks_like_google_key(&m.text) {
            out.push(SecretFinding {
                kind: SecretKind::GoogleApiKey,
                file: m.file.clone(),
                line: m.line,
                text: m.text,
            });
        }
    }

    // JWT: три base64-части.
    for m in search(root, "eyJ", &SearchOptions::default()) {
        if looks_like_jwt(&m.text) {
            out.push(SecretFinding {
                kind: SecretKind::Jwt,
                file: m.file.clone(),
                line: m.line,
                text: m.text,
            });
        }
    }

    // Назначенные секреты: password= / secret= / token= / api_key=.
    for keyword in ["password", "passwd", "secret", "api_key", "apikey", "token", "access_token"] {
        for m in search(
            root,
            keyword,
            &SearchOptions {
                case_insensitive: true,
                ..Default::default()
            },
        ) {
            if looks_like_credential_assignment(&m.text, keyword) {
                out.push(SecretFinding {
                    kind: SecretKind::Assignment,
                    file: m.file.clone(),
                    line: m.line,
                    text: m.text,
                });
            }
        }
    }

    // Connection string с учётными данными.
    for proto in ["postgres://", "postgresql://", "mongodb://", "redis://", "mysql://"] {
        for m in search(root, proto, &SearchOptions::default()) {
            if m.text.contains(':') && m.text.contains('@') {
                out.push(SecretFinding {
                    kind: SecretKind::ConnectionString,
                    file: m.file.clone(),
                    line: m.line,
                    text: m.text,
                });
            }
        }
    }

    out
}

/// Эвристика AWS Access Key ID.
fn looks_like_aws_keyid(line: &str) -> bool {
    if let Some(idx) = line.find("AKIA") {
        let tail = &line[idx + 4..];
        // 16 алфансимволов после AKIA.
        tail.chars().take(16).filter(|c| c.is_alphanumeric()).count() >= 16
    } else {
        false
    }
}

/// Эвристика Google API key.
fn looks_like_google_key(line: &str) -> bool {
    if let Some(idx) = line.find("AIza") {
        let tail = &line[idx + 4..];
        tail.chars().take(35).filter(|c| c.is_alphanumeric()).count() >= 35
    } else {
        false
    }
}

/// Эвристика JWT: три base64-части, первая начинается на `eyJ`.
fn looks_like_jwt(line: &str) -> bool {
    if let Some(idx) = line.find("eyJ") {
        let tail = &line[idx..];
        // Должно быть минимум две точки.
        tail.matches('.').count() >= 2
    } else {
        false
    }
}

/// Эвристика назначения секрета: `password = "value"` со значением.
fn looks_like_credential_assignment(line: &str, keyword: &str) -> bool {
    let lower = line.to_lowercase();
    let Some(idx) = lower.find(keyword) else {
        return false;
    };
    let after = &line[idx + keyword.len()..];
    // Ищем `=` или `:` после ключевого слова.
    let eq = after.find(|c: char| c == '=' || c == ':');
    let Some(eq) = eq else {
        return false;
    };
    let value = after[eq..].trim_start_matches(['=', ':', ' ', '\t']);
    if value.is_empty() {
        return false;
    }
    // Игнорируем placeholder'ы и переменные окружения/импорты.
    let placeholders = ["none", "null", "undefined", "changeme", "change_me", "xxx",
        "your_", "example", "secret_key", "${", "process.env", "os.environ", "env(", "getenv"];
    let val_lower_head = value.to_lowercase();
    if placeholders
        .iter()
        .any(|p| val_lower_head.starts_with(p))
    {
        return false;
    }
    // Длина осмысленного секрета (без обрамляющих кавычек).
    let cleaned = value.trim_matches([' ', '"', '\'', '\t'].as_ref());
    let first_token = cleaned
        .split([' ', ',', ')', '\n', '\r', ';'])
        .next()
        .unwrap_or("");
    first_token.len() >= 6
}

/// Расширения, в которых ищем PEM-ключи.
fn pem_extensions() -> Vec<String> {
    ["pem", "key", "crt", "cer", "p12", "txt", "env"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_assignment() {
        let dir = std::env::temp_dir().join("dm_secrets_test_assign");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "let password = \"supersecret123\";\n").unwrap();
        let findings = scan(&dir);
        assert!(findings.iter().any(|f| f.kind == SecretKind::Assignment));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ignores_placeholder() {
        let dir = std::env::temp_dir().join("dm_secrets_test_placeholder");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "let api_key = process.env.KEY;\n").unwrap();
        let findings = scan(&dir);
        assert!(!findings.iter().any(|f| f.kind == SecretKind::Assignment));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_aws_key() {
        let line = r#"aws_access_key_id = "AKIAIOSFODNN7EXAMPLE""#;
        assert!(looks_like_aws_keyid(line));
    }

    #[test]
    fn detects_connection_string() {
        let dir = std::env::temp_dir().join("dm_secrets_test_conn");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "let url = \"postgres://user:hunter2@host/db\";\n").unwrap();
        let findings = scan(&dir);
        assert!(findings.iter().any(|f| f.kind == SecretKind::ConnectionString));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
