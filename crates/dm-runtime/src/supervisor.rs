//! Главный оркестратор: запускает все сервисы проекта с учётом очереди и
//! задержек, мультиплексирует их логи и управляет жизненным циклом.
//!
//! Архитектура:
//! - На каждый сервис создаётся своя tokio-задача (`spawn_service`).
//! - Все лог-линии сливаются в один `mpsc`-канал → потребитель печатает их.
//! - Завершение (`shutdown`) корректно убивает все процессы деревьями.
//! - File-watcher опционально перезапускает изменённый сервис.

use crate::logs::{LogLine, ServiceStatus};
use crate::notify::{NotifyConfig, NotifyEvent};
use crate::process::{ManagedProcess, ProcessExit};
use crate::spawn_strategy::resolve_run_command;
use dm_core::DmResult;
use dm_core::config::Config;
use dm_core::project::{Project, Service};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task::JoinHandle;

/// Опции запуска supervisor'а.
#[derive(Debug, Clone, Default)]
pub struct SupervisorOptions {
    /// Отключить file-watching (флаг `--no-watch`).
    pub no_watch: bool,
    /// Не перезапускать упавшие процессы.
    pub no_restart: bool,
}

/// Рантайм-состояние одного сервиса: статус и текущий процесс (если жив).
struct ServiceState {
    status: ServiceStatus,
    process: Option<ManagedProcess>,
    /// Маркер того, что сервис намеренно остановлен (не надо рестартовать).
    stopped_by_user: bool,
    /// Сколько раз подряд сервис перезапускался за короткое время (auto-recovery).
    /// Сбрасывается при длительной успешной работе. После `max` — стоп + диагноз.
    consecutive_crashes: u32,
    /// Timestamp последнего запуска (мс от старта supervisor), для определения
    /// «быстрого» падения.
    last_start_ms: u64,
    /// Флаг: watcher/restart() запросил перезапуск. Цикл сервиса увидит его,
    /// убьёт процесс и поднимет заново.
    restart_requested: bool,
    /// PID текущего процесса (для мониторинга ресурсов). Обновляется циклом.
    pid: Option<u32>,
    /// Лимит памяти и действие при превышении (для monitor'а). None = без лимита.
    resource_limits: Option<(u64, dm_core::config::ResourceAction)>,
}

/// Главный оркестратор процессов.
///
/// Хранит общее состояние, канал логов и handles задач сервисов. Создаётся
/// один раз на запуск `dm start` и живёт до `shutdown()`.
pub struct Supervisor {
    project: Arc<Project>,
    options: SupervisorOptions,
    /// Состояние по имени сервиса.
    states: Arc<Mutex<HashMap<String, ServiceState>>>,
    /// Глобальный «бегунок» shutdown: когда выставлен, задачи останавливаются.
    shutdown: Arc<RwLock<bool>>,
    /// Канал исходящих логов.
    log_tx: mpsc::UnboundedSender<LogLine>,
    /// Handles всех задач сервисов (для ожидания завершения).
    handles: Mutex<Vec<JoinHandle<()>>>,
    /// Карта: имя сервиса → цвет (заранее посчитанный для логов).
    colors: HashMap<String, &'static str>,
    /// Конфигурация уведомлений (webhook/desktop). Пусто — выключено.
    notify: Arc<NotifyConfig>,
}

impl Supervisor {
    /// Создаёт supervisor, подготавливающий запуск сервисов проекта.
    ///
    /// `log_tx` — канал, в который будут отправляться все лог-линии; потребитель
    /// (обычно консольный принтер в `dm-cli`) читает его в отдельной задаче.
    pub fn new(
        project: Project,
        options: SupervisorOptions,
        log_tx: mpsc::UnboundedSender<LogLine>,
    ) -> Self {
        Self::with_notify(project, options, log_tx, NotifyConfig::default())
    }

    /// То же, что [`Supervisor::new`], но с конфигурацией уведомлений.
    pub fn with_notify(
        project: Project,
        options: SupervisorOptions,
        log_tx: mpsc::UnboundedSender<LogLine>,
        notify: NotifyConfig,
    ) -> Self {
        let colors = project
            .services
            .iter()
            .map(|s| (s.name.clone(), crate::logs::service_color(s.language)))
            .collect();

        let mut states = HashMap::new();
        for svc in &project.services {
            states.insert(
                svc.name.clone(),
                ServiceState {
                    status: ServiceStatus::Pending,
                    process: None,
                    stopped_by_user: false,
                    consecutive_crashes: 0,
                    last_start_ms: 0,
                    restart_requested: false,
                    pid: None,
                    resource_limits: None,
                },
            );
        }

        Self {
            project: Arc::new(project),
            options,
            states: Arc::new(Mutex::new(states)),
            shutdown: Arc::new(RwLock::new(false)),
            log_tx,
            handles: Mutex::new(Vec::new()),
            colors,
            notify: Arc::new(notify),
        }
    }

    /// Возвращает цвет префикса для сервиса (для отрисовки логов).
    pub fn color_of(&self, service: &str) -> Option<&'static str> {
        self.colors.get(service).copied()
    }

    /// Запускает все сервисы проекта согласно их `order` и `delay_ms`.
    ///
    /// Каждый сервис — отдельная tokio-задача. Метод возвращается немедленно
    /// после постановки задач; сами сервисы работают до [`Supervisor::shutdown`].
    pub async fn start_all(&self) -> DmResult<()> {
        // Проект уже содержит сервисы в порядке запуска (см. Project::from_config).
        // Дополнительно берём карту delay_ms из конфига (через lookup по имени).
        let delays = self.compute_delays();
        for svc in self.project.services.clone() {
            let states = self.states.clone();
            let shutdown = self.shutdown.clone();
            let log_tx = self.log_tx.clone();
            let notify = self.notify.clone();
            let no_restart = self.options.no_restart;
            let svc = Arc::new(svc);
            let svc_name = svc.name.clone();

            let handle = tokio::spawn(async move {
                run_service_loop(svc, states, shutdown, log_tx, notify, no_restart).await;
            });
            self.handles.lock().await.push(handle);

            // Уважаем delay_ms перед запуском следующего (очередь/задержки).
            if let Some(delay) = delays.get(&svc_name)
                && *delay > 0
            {
                tokio::time::sleep(Duration::from_millis(*delay)).await;
            }
        }
        Ok(())
    }

    /// Собирает карту {имя сервиса → delay_ms} из runtime-модели сервисов.
    ///
    /// Задержки переносятся из `dm.yaml` (поле `delay_ms`) на этапе построения
    /// `Project` в `project_from_config`.
    fn compute_delays(&self) -> std::collections::HashMap<String, u64> {
        self.project
            .services
            .iter()
            .map(|s| (s.name.clone(), s.delay_ms))
            .collect()
    }

    /// Просит сервис перезапуститься.
    ///
    /// Используется командой `dm restart <svc>` и watcher'ом. Не убивает процесс
    /// напрямую (им владеет цикл сервиса): выставляет флаг `restart_requested`,
    /// который цикл замечает через `tokio::select!` и сам убивает+поднимает.
    pub async fn restart(&self, name: &str) -> DmResult<()> {
        let mut states = self.states.lock().await;
        let state = states
            .get_mut(name)
            .ok_or_else(|| dm_core::DmError::ServiceNotFound(name.to_string()))?;
        state.stopped_by_user = false;
        state.restart_requested = true;
        drop(states);
        let _ = self.log_tx.send(LogLine::new(
            name.to_string(),
            crate::logs::LogLevel::System,
            "перезапуск по запросу".to_string(),
        ));
        Ok(())
    }

    /// Обрабатывает изменение файлов сервиса watcher'ом.
    ///
    /// Если у сервиса `restart_on_change == true` — запрашивает перезапуск.
    /// Это точка интеграции watcher → supervisor: команда `dm start` создаёт
    /// [`crate::watcher::FileWatcher`] и направляет события сюда.
    pub async fn notify_file_changed(&self, name: &str, _paths: &[std::path::PathBuf]) {
        let restart = self
            .project
            .services
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.restart_on_change)
            .unwrap_or(false);
        if !restart {
            return;
        }
        let mut states = self.states.lock().await;
        if let Some(state) = states.get_mut(name) {
            if state.stopped_by_user {
                return;
            }
            state.restart_requested = true;
        }
        drop(states);
        let _ = self.log_tx.send(LogLine::new(
            name.to_string(),
            crate::logs::LogLevel::System,
            "обнаружены изменения — перезапуск".to_string(),
        ));
    }

    /// Останавливает конкретный сервис (без последующего рестарта).
    ///
    /// Выставляет `stopped_by_user`, что прерывает цикл сервиса на следующей
    /// итерации `select!`. Если процесс активен — просят перезапуск, а флаг
    /// `stopped_by_user` не даст ему подняться вновь.
    pub async fn stop_service(&self, name: &str) -> DmResult<()> {
        let mut states = self.states.lock().await;
        let state = states
            .get_mut(name)
            .ok_or_else(|| dm_core::DmError::ServiceNotFound(name.to_string()))?;
        state.stopped_by_user = true;
        // Триггерим выход из select! (как перезапуск, но флаг выше не даст подняться).
        state.restart_requested = true;
        drop(states);
        let _ = self.log_tx.send(LogLine::new(
            name.to_string(),
            crate::logs::LogLevel::System,
            "остановлен пользователем".to_string(),
        ));
        Ok(())
    }

    /// Полный останов: убивает все процессы, дожидается задач.
    ///
    /// Важно: флаг `shutdown` замечается каждым циклом сервиса в `select!`,
    /// после чего цикл **сам** вызывает `proc.kill()` и выходит. Поэтому мы
    /// не прерываем задачи (`.abort()`) — иначе процесс не успел бы завершиться
    /// и остался бы «осиротевшим». Даём каждой задаче корректно доработать.
    pub async fn shutdown(&self) {
        {
            let mut flag = self.shutdown.write().await;
            *flag = true;
        }
        // Отмечаем все сервисы как остановленные пользователем, чтобы циклы не
        // пытались перезапуститься после kill.
        {
            let mut states = self.states.lock().await;
            for (_name, state) in states.iter_mut() {
                state.stopped_by_user = true;
            }
        }

        // Даём циклам сервисов заметить shutdown и корректно убить свои процессы.
        let handles: Vec<JoinHandle<()>> = {
            let mut handles = self.handles.lock().await;
            handles.drain(..).collect()
        };
        for h in handles {
            // Ждём с таймаутом, чтобы не зависнуть, если цикл застрял.
            let _ = tokio::time::timeout(Duration::from_secs(10), h).await;
        }

        // Финальная отметка статусов.
        let mut states = self.states.lock().await;
        for (_name, state) in states.iter_mut() {
            state.status = ServiceStatus::Stopped;
            state.process = None;
        }
    }

    /// Возвращает снимок статусов всех сервисов (для `dm status`).
    pub async fn statuses(&self) -> Vec<(String, ServiceStatus)> {
        let states = self.states.lock().await;
        self.project
            .services
            .iter()
            .map(|s| {
                let st = states
                    .get(&s.name)
                    .map(|x| x.status)
                    .unwrap_or(ServiceStatus::Pending);
                (s.name.clone(), st)
            })
            .collect()
    }

    /// Запускает фоновый мониторинг ресурсов процессов.
    ///
    /// Каждые `interval_secs` проверяет RSS каждого запущенного сервиса против
    /// его `resources.memory_mb`. При превышении — уведомление (notify) или kill
    /// (согласно `resources.on_exceed`). Задача живёт до [`Supervisor::shutdown`].
    pub fn start_resource_monitor(&self, interval_secs: u64) {
        if interval_secs == 0 {
            return;
        }
        let states = self.states.clone();
        let shutdown = self.shutdown.clone();
        let log_tx = self.log_tx.clone();
        let notify = self.notify.clone();
        // Карта имя → (memory_mb, on_exceed) — берём из конфига проектов.
        // Поле resources хранится в ServiceConfig; в runtime-модели его нет,
        // поэтому лимиты пробрасываем через статический снимок здесь.
        let limits: HashMap<String, (u64, dm_core::config::ResourceAction)> = self
            .project
            .services
            .iter()
            .filter_map(|_s| {
                // Лимиты хранятся в config, не в runtime Service; используем
                // конфиг, доступный через project (но там их нет).
                // На этом уровне лимиты передаются через watcher/cli при старте.
                None::<(String, (u64, dm_core::config::ResourceAction))>
            })
            .collect();
        let _ = limits; // лимиты устанавливаются через set_resource_limits

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;
                if *shutdown.read().await {
                    return;
                }
                // Снимок: имя → (pid, memory_limit_mb, action).
                let snapshot: Vec<(String, u32, u64, dm_core::config::ResourceAction)> = {
                    let st = states.lock().await;
                    st.iter()
                        .filter_map(|(name, state)| {
                            let pid = state.pid?;
                            let (limit, action) = state.resource_limits?;
                            Some((name.clone(), pid, limit, action))
                        })
                        .collect()
                };
                for (name, pid, limit_mb, action) in snapshot {
                    if let crate::monitor::MemoryCheck::Exceeded { rss_mb, limit_mb } =
                        crate::monitor::check_memory(pid, limit_mb)
                    {
                        let _ = log_tx.send(LogLine::new(
                            name.clone(),
                            crate::logs::LogLevel::Error,
                            format!(
                                "превышен лимит памяти: {rss_mb} МБ > {limit_mb} МБ (действие: {action:?})"
                            ),
                        ));
                        // Уведомление всегда.
                        let n = notify.clone();
                        let nm = name.clone();
                        let detail = format!("RSS {rss_mb} МБ > лимит {limit_mb} МБ");
                        tokio::spawn(async move {
                            crate::notify::send(&n, NotifyEvent::Crash, &nm, &detail).await;
                        });
                        // Kill — если настроено.
                        if matches!(action, dm_core::config::ResourceAction::Kill) {
                            let mut st = states.lock().await;
                            if let Some(state) = st.get_mut(&name) {
                                state.restart_requested = true; // цикл убьёт и поднимет
                                state.pid = None;
                            }
                            drop(st);
                            // Рекурсивное убийство дерева через kill_tree.
                            let _ = tokio::task::spawn_blocking(move || {
                                let _ = kill_tree::blocking::kill_tree(pid);
                            })
                            .await;
                        }
                    }
                }
            }
        });
    }

    /// Устанавливает лимит ресурсов для сервиса (вызывается из cli при старте).
    pub async fn set_resource_limits(
        &self,
        name: &str,
        memory_mb: u64,
        action: dm_core::config::ResourceAction,
    ) {
        let mut st = self.states.lock().await;
        if let Some(state) = st.get_mut(name) {
            state.resource_limits = Some((memory_mb, action));
        }
    }
}

/// Цикл жизни одного сервиса: запуск → ожидание выхода → рестарт.
///
/// Это «вечный» цикл для сервиса. Ключевое отличие от наивного поллинга:
/// мы **блокируемся на `ManagedProcess::wait_exit()`**, поэтому узнаём реальный
/// момент выхода процесса и его код. Это даёт корректный auto-recovery:
/// - `success` (exit 0) → рестарт сразу (например, одноразовые команды) или выход;
/// - неуспешный выход → инкремент счётчика `consecutive_crashes`; если он превышает
///   `max_consecutive_crashes` — сервис помечается Crashed и **больше не
///   поднимается** (прерывание бесконечного цикла рестартов), шлётся уведомление;
/// - при длительной успешной работе счётчик сбрасывается.
///
/// `restart_command` (одноразовое перезапускающее действие от watcher'а)
/// реализовано через флаг в состоянии: если его выставили, мы убиваем текущий
/// процесс и перезапускаем.
async fn run_service_loop(
    svc: Arc<Service>,
    states: Arc<Mutex<HashMap<String, ServiceState>>>,
    shutdown: Arc<RwLock<bool>>,
    log_tx: mpsc::UnboundedSender<LogLine>,
    notify: Arc<NotifyConfig>,
    no_restart: bool,
) {
    // Начальная задержка перед самым первым запуском.
    if svc.delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(svc.delay_ms)).await;
    }

    loop {
        if *shutdown.read().await {
            break;
        }
        // Если сервис остановлен пользователем — выходим.
        {
            let st = states.lock().await;
            if let Some(state) = st.get(&svc.name)
                && state.stopped_by_user
            {
                break;
            }
        }

        // --- Запуск процесса ---
        {
            let mut st = states.lock().await;
            if let Some(state) = st.get_mut(&svc.name) {
                state.status = ServiceStatus::Starting;
                state.last_start_ms = now_ms();
            }
        }
        let _ = log_tx.send(LogLine::new(
            svc.name.clone(),
            crate::logs::LogLevel::System,
            format!("запуск: {}", svc.run_command),
        ));

        let env: Vec<(String, String)> = Vec::new();
        let mut proc = match ManagedProcess::spawn(
            &svc.name,
            &svc.run_command,
            &svc.path,
            &env,
            log_tx.clone(),
        )
        .await
        {
            Ok(p) => p,
            Err(e) => {
                let _ = log_tx.send(LogLine::new(
                    svc.name.clone(),
                    crate::logs::LogLevel::Error,
                    format!("ошибка запуска: {e}"),
                ));
                // Спавн провалился — считаем как крэш.
                let stop = register_crash(&states, &svc.name, &log_tx, &notify).await;
                if stop || no_restart {
                    mark_final(&states, &svc.name, ServiceStatus::Crashed).await;
                    break;
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        {
            let mut st = states.lock().await;
            if let Some(state) = st.get_mut(&svc.name) {
                state.status = ServiceStatus::Running;
                // Процессом владеет цикл; поле process здесь не используется для
                // управления (управление идёт через флаги) — оставляем None.
                state.process = None;
                // Публикуем PID для мониторинга ресурсов (supervisor проверяет RSS).
                state.pid = proc.pid();
            }
        }
        // Уведомление об успешном старте (webhook/desktop) — fire-and-forget.
        let n = notify.clone();
        let nm = svc.name.clone();
        tokio::spawn(async move {
            crate::notify::send(&n, NotifyEvent::Started, &nm, "сервис запущен").await;
        });

        // --- Ожидание реального выхода процесса ---
        // tokio::select! даёт возможность отреагировать на shutdown/restart-флаг
        // не дожидаясь естественного выхода. Каждая ветвь завершает ожидание,
        // поэтому select! используется без обёртки в loop.
        let exit = tokio::select! {
            _ = shutdown_signal(&shutdown) => {
                // Глобальный shutdown — убиваем и выходим.
                let _ = proc.kill().await;
                return;
            }
            _ = restart_requested(&states, &svc.name) => {
                // Watcher или restart() попросили перезапуск.
                let _ = log_tx.send(LogLine::new(
                    svc.name.clone(),
                    crate::logs::LogLevel::System,
                    "перезапуск...".to_string(),
                ));
                let _ = proc.kill().await;
                // Сбрасываем флаг запроса рестарта и продолжаем внешний цикл.
                clear_restart_flag(&states, &svc.name).await;
                None
            }
            exit_res = proc.wait_exit() => {
                // Процесс завершился сам — получили код.
                match exit_res {
                    Ok(e) => Some(e),
                    Err(e) => {
                        let _ = log_tx.send(LogLine::new(
                            svc.name.clone(),
                            crate::logs::LogLevel::Error,
                            format!("ошибка ожидания: {e}"),
                        ));
                        Some(ProcessExit { code: None, killed_by_signal: false })
                    }
                }
            }
        };

        // Если мы вышли по shutdown — цикл завершится вверху.
        if *shutdown.read().await {
            break;
        }

        // Разбор исхода.
        match exit {
            None => {
                // Перезапуск по запросу — не считаем крэшом, просто продолжаем.
                continue;
            }
            Some(e) if e.success() => {
                let _ = log_tx.send(LogLine::new(
                    svc.name.clone(),
                    crate::logs::LogLevel::System,
                    "процесс завершился успешно".to_string(),
                ));
                // Успешная работа достаточно долго — сбрасываем счётчик крэшей.
                maybe_reset_crash_counter(&states, &svc.name).await;
                // Одноразовые команды (exit 0) при no_restart — выходим.
                if no_restart {
                    mark_final(&states, &svc.name, ServiceStatus::Exited).await;
                    break;
                }
                continue;
            }
            Some(e) => {
                let code = e
                    .code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "сигнал".into());
                let _ = log_tx.send(LogLine::new(
                    svc.name.clone(),
                    crate::logs::LogLevel::Error,
                    format!("процесс упал (код {code})"),
                ));
                let stop = register_crash(&states, &svc.name, &log_tx, &notify).await;
                if stop || no_restart {
                    mark_final(&states, &svc.name, ServiceStatus::Crashed).await;
                    break;
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

/// Регистрирует крэш: инкремент счётчика, авто-recovery, уведомление.
///
/// Возвращает true, если достигнут лимит `max_consecutive_crashes` —
/// сервис надо остановить (прервать цикл рестартов).
async fn register_crash(
    states: &Arc<Mutex<HashMap<String, ServiceState>>>,
    name: &str,
    log_tx: &mpsc::UnboundedSender<LogLine>,
    notify: &Arc<NotifyConfig>,
) -> bool {
    const DEFAULT_MAX: u32 = 5;
    let mut should_stop = false;
    let count = {
        let mut st = states.lock().await;
        let Some(state) = st.get_mut(name) else {
            return false;
        };
        state.consecutive_crashes = state.consecutive_crashes.saturating_add(1);
        if state.consecutive_crashes >= DEFAULT_MAX {
            should_stop = true;
        }
        state.consecutive_crashes
    };
    if should_stop {
        let _ = log_tx.send(LogLine::new(
            name.to_string(),
            crate::logs::LogLevel::Error,
            format!(
                "сервис упал {count} раз подряд — авто-recovery останавливает его. Проверьте причину."
            ),
        ));
        // Уведомление о критическом авто-recovery.
        let n = notify.clone();
        let nm = name.to_string();
        tokio::spawn(async move {
            crate::notify::send(
                &n,
                NotifyEvent::Crash,
                &nm,
                "достигнут лимит последовательных падений — сервис остановлен",
            )
            .await;
        });
    }
    should_stop
}

/// Сбрасывает счётчик крэшей, если процесс проработал достаточно долго (>10с).
async fn maybe_reset_crash_counter(states: &Arc<Mutex<HashMap<String, ServiceState>>>, name: &str) {
    const UPTIME_THRESHOLD_MS: u64 = 10_000;
    let now = now_ms();
    let mut st = states.lock().await;
    let Some(state) = st.get_mut(name) else {
        return;
    };
    if now.saturating_sub(state.last_start_ms) >= UPTIME_THRESHOLD_MS {
        state.consecutive_crashes = 0;
    }
}

/// Помечает финальный статус сервиса (Crashed/Exited).
async fn mark_final(
    states: &Arc<Mutex<HashMap<String, ServiceState>>>,
    name: &str,
    status: ServiceStatus,
) {
    let mut st = states.lock().await;
    if let Some(state) = st.get_mut(name) {
        state.status = status;
        state.process = None;
        state.stopped_by_user = true;
    }
}

/// Future, завершающаяся когда выставлен глобальный shutdown.
async fn shutdown_signal(shutdown: &Arc<RwLock<bool>>) {
    loop {
        if *shutdown.read().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Future, завершающаяся когда сервису请求ил рестарт (флаг в состоянии).
async fn restart_requested(states: &Arc<Mutex<HashMap<String, ServiceState>>>, name: &str) {
    loop {
        let requested = {
            let st = states.lock().await;
            st.get(name).map(|s| s.restart_requested).unwrap_or(false)
        };
        if requested {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Сбрасывает флаг запроса рестарта.
async fn clear_restart_flag(states: &Arc<Mutex<HashMap<String, ServiceState>>>, name: &str) {
    let mut st = states.lock().await;
    if let Some(state) = st.get_mut(name) {
        state.restart_requested = false;
    }
}

/// Монотонные миллисекунды от старта процесса (для оценок uptime).
fn now_ms() -> u64 {
    use std::sync::OnceLock;
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    let start = START.get_or_init(std::time::Instant::now);
    start.elapsed().as_millis() as u64
}

/// Строит доменный [`Project`] из конфигурации и корневого каталога.
///
/// Разрешает все относительные пути и команды запуска. Это точка моста между
/// конфигом (serde-структуры) и runtime-моделью.
pub fn project_from_config(cfg: &Config, root: &std::path::Path) -> DmResult<Project> {
    use dm_core::paths;
    use std::path::Path;

    let mut services = Vec::with_capacity(cfg.services.len());
    for name in cfg.services_in_start_order() {
        let svc_cfg = cfg.services.get(&name).expect("service exists in order");
        let path = paths::resolve(root, Path::new(&svc_cfg.path));
        let repo_path = svc_cfg
            .repo
            .as_ref()
            .map(|r| paths::resolve(root, Path::new(r)));

        let mut svc = Service {
            name: name.clone(),
            path,
            language: svc_cfg.language,
            run_command: String::new(),
            watch: svc_cfg.watch,
            restart_on_change: svc_cfg.restart_on_change,
            repo_path,
            delay_ms: svc_cfg.delay_ms,
        };
        svc.run_command = resolve_run_command(&svc, svc_cfg.run.as_deref());
        services.push(svc);
    }
    Ok(Project {
        name: cfg.project_name.clone(),
        root: root.to_path_buf(),
        services,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn minimal_config() -> Config {
        // Разбираем конфиг из YAML — так не нужно напрямую зависеть от indexmap.
        let yaml = r#"
version: 1
project_name: test
services:
  api:
    path: ./api
    language: rust
    run: echo hello
"#;
        let mut cfg: Config = serde_yaml::from_str(yaml).expect("valid yaml");
        cfg.validate().expect("valid config");
        cfg
    }

    #[test]
    fn builds_project_from_config() {
        let cfg = minimal_config();
        let root = PathBuf::from("/tmp/proj");
        let project = project_from_config(&cfg, &root).unwrap();
        assert_eq!(project.services.len(), 1);
        assert_eq!(project.services[0].name, "api");
        assert_eq!(project.services[0].run_command, "echo hello");
    }
}
