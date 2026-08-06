#![allow(
    unused_imports,
    dead_code,
    clippy::needless_borrow,
    clippy::redundant_clone,
    clippy::needless_return,
    clippy::collapsible_if,
    clippy::manual_find,
    clippy::trim_split_whitespace,
    clippy::derivable_impls,
    clippy::let_unit_value,
    clippy::redundant_closure,
    clippy::unnecessary_first_then_check,
    clippy::useless_conversion
)]
//! # dm-deploy
//!
//! SSH-деплой по заданным целям. В этой итерации заложен каркас и трейт
//! [`Deployer`], а конкретная реализация на `russh` будет подключена в
//! следующей итерации (чтобы не тянуть тяжёлую C-зависимость сейчас и не
//! блокировать сборку ядра).
//!
//! Триггеры деплоя (`manual` / `after_push` / `after_commit`) проверяются
//! функцией [`should_trigger`].

use dm_core::DmResult;
use dm_core::config::{DeployTarget, DeployTrigger};
use std::path::Path;

/// Точка расширения для конкретного движка деплоя (russh / local / docker…).
///
/// Реализация выполняет последовательность шагов `steps` на цели `target`.
#[allow(async_fn_in_trait)] // simple trait, no Send bounds needed for current stub
pub trait Deployer {
    /// Выполняет деплой на `target`, возвращая сводный отчёт.
    async fn deploy(&self, target: &DeployTarget) -> DmResult<DeployReport>;
}

/// Отчёт о результате деплоя.
#[derive(Debug, Clone, Default)]
pub struct DeployReport {
    /// Имя цели.
    pub target_name: String,
    /// Успешно ли завершились все шаги.
    pub success: bool,
    /// Лог каждой команды (команда → вывод).
    pub step_logs: Vec<(String, String)>,
}

/// Возвращает true, если триггер `configured` соответствует событию `event`.
///
/// - `Manual` всегда требует явного вызова `dm deploy` (события не триггерят).
/// - `AfterPush` срабатывает после `dm push`.
/// - `AfterCommit` срабатывает после `dm commit`.
pub fn should_trigger(configured: DeployTrigger, event: DeployTrigger) -> bool {
    match configured {
        DeployTrigger::Manual => false,
        other => other == event,
    }
}

/// Заглушка-движок деплоя: не выполняет реальной работы, пригоден для тестов
/// и для режима, когда russh отключён feature-флагом.
pub struct StubDeployer;

impl Deployer for StubDeployer {
    async fn deploy(&self, target: &DeployTarget) -> DmResult<DeployReport> {
        let step_logs = target
            .steps
            .iter()
            .map(|step| (step.clone(), "[stub] шаг пропущен".to_string()))
            .collect();
        Ok(DeployReport {
            target_name: target.name.clone(),
            success: true,
            step_logs,
        })
    }
}

/// Главная точка входа: подбирает цель по имени и деплоит.
///
/// Возвращает отчёт. В текущей версии использует [`StubDeployer`]; в следующей
/// итерации сюда подключается russh-реализация.
pub async fn run_deploy(
    config: &dm_core::Config,
    target_name: &str,
    _project_root: &Path,
) -> DmResult<DeployReport> {
    let target = config
        .deploy
        .iter()
        .find(|t| t.name == target_name)
        .ok_or_else(|| {
            dm_core::DmError::invalid_config(format!(
                "цель деплоя '{target_name}' не найдена в секции `deploy`"
            ))
        })?;
    let deployer = StubDeployer;
    deployer.deploy(target).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_logic() {
        assert!(!should_trigger(
            DeployTrigger::Manual,
            DeployTrigger::AfterPush
        ));
        assert!(should_trigger(
            DeployTrigger::AfterPush,
            DeployTrigger::AfterPush
        ));
        assert!(!should_trigger(
            DeployTrigger::AfterCommit,
            DeployTrigger::AfterPush
        ));
    }
}
