//! SemVer: версионирование и bump.
//!
//! Минимальная реализация SemVer 2.0 (без pre-release/build-метаданных в этой
//! итерации) для `dm release patch|minor|major`.

use std::fmt;

/// SemVer-версия `MAJOR.MINOR.PATCH`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// Мажорная версия.
    pub major: u64,
    /// Минорная версия.
    pub minor: u64,
    /// Патч-версия.
    pub patch: u64,
}

impl Version {
    /// Разбирает строку вида `1.2.3`. Возвращает `None` при ошибке.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('v');
        let mut parts = s.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next().unwrap_or("0").parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    /// Возвращает новую версию с применением bump'а.
    pub fn bumped(self, kind: Bump) -> Self {
        match kind {
            Bump::Major => Self {
                major: self.major + 1,
                minor: 0,
                patch: 0,
            },
            Bump::Minor => Self {
                major: self.major,
                minor: self.minor + 1,
                patch: 0,
            },
            Bump::Patch => Self {
                major: self.major,
                minor: self.minor,
                patch: self.patch + 1,
            },
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Тип SemVer-bump'а.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bump {
    /// Мажорная версия (breaking changes).
    Major,
    /// Минорная версия (новые возможности).
    Minor,
    /// Патч (исправления).
    Patch,
}

impl Bump {
    /// Разбирает строку в [`Bump`].
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "major" => Some(Self::Major),
            "minor" => Some(Self::Minor),
            "patch" => Some(Self::Patch),
            _ => None,
        }
    }
}

/// Определяет тип bump'а на основе списка conventional-commits.
///
/// - Любой `BREAKING CHANGE`/`!` → Major.
/// - Есть `feat` → Minor.
/// - Иначе → Patch.
pub fn suggest_bump(commits: &[crate::conventional::ConventionalCommit]) -> Bump {
    if commits.iter().any(|c| c.breaking) {
        return Bump::Major;
    }
    if commits.iter().any(|c| c.kind == "feat") {
        return Bump::Minor;
    }
    Bump::Patch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_display() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.to_string(), "1.2.3");
        assert_eq!(Version::parse("v2.0.0").unwrap().major, 2);
        assert!(Version::parse("abc").is_none());
    }

    #[test]
    fn bump_logic() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.bumped(Bump::Patch).to_string(), "1.2.4");
        assert_eq!(v.bumped(Bump::Minor).to_string(), "1.3.0");
        assert_eq!(v.bumped(Bump::Major).to_string(), "2.0.0");
    }

    #[test]
    fn suggest_uses_breaking_and_feat() {
        use crate::conventional::ConventionalCommit;
        let breaking = vec![ConventionalCommit::parse("fix!: критично").unwrap()];
        assert_eq!(suggest_bump(&breaking), Bump::Major);
        let feat = vec![ConventionalCommit::parse("feat: новая фича").unwrap()];
        assert_eq!(suggest_bump(&feat), Bump::Minor);
        let fix = vec![ConventionalCommit::parse("fix: баг").unwrap()];
        assert_eq!(suggest_bump(&fix), Bump::Patch);
    }
}
