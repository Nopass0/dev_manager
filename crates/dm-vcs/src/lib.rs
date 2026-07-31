#![allow(
    unused_imports,
    dead_code,
    clippy::needless_borrow,
    clippy::redundant_clone
)]

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
pub use commit::{CommitOutcome, commit_all, commit_in_repo};
pub use conventional::{COMMIT_TYPES, ConventionalCommit, group_by_type};
pub use git::{GitOutput, git_binary_version, has_changes, is_repo, run_git};
pub use multi::{RepoOpResult, branch_all, collect_repo_paths, rebase_all, stash_all};
pub use push::{PushOutcome, push_all, push_in_repo};
pub use release::{Bump, Version, suggest_bump};
