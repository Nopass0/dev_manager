//! # dm-vcs
//!
//! Git-автоматизация для Dev Manager. Все операции выполняются через системный
//! `git` CLI (а не через `git2`/libgit2) — это сознательное решение, чтобы:
//! - не тянуть C-зависимость (сложность сборки на Windows);
//! - всегда иметь поведение, идентичное `git` в терминале пользователя.
//!
//! Требование: `git` должен быть установлен и доступен в PATH.
//!
//! ## Возможности
//! - [`git::run`] — универсальный вызов `git -C <repo> <args>`.
//! - [`git::is_repo`], [`git::has_changes`] — проверки состояния.
//! - [`commit::commit_all`] — коммит во все/конкретный репозиторий.
//! - [`push::push`] — пуш каждого репозитория в свой origin.

pub mod changelog;
pub mod commit;
pub mod conventional;
pub mod diff;
pub mod git;
pub mod multi;
pub mod push;
pub mod release;

pub use changelog::render_release_section;
pub use commit::{commit_all, commit_in_repo, CommitOutcome};
pub use conventional::{group_by_type, ConventionalCommit, COMMIT_TYPES};
pub use git::{git_binary_version, has_changes, is_repo, run_git, GitOutput};
pub use multi::{branch_all, collect_repo_paths, rebase_all, stash_all, RepoOpResult};
pub use push::{push_all, push_in_repo, PushOutcome};
pub use release::{suggest_bump, Bump, Version};
