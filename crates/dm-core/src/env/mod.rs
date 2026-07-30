//! Единый `.env` и его распределение по сервисам.
//!
//! Идея: в корне проекта лежит один `.env`, в котором переменные сгруппированы
//! по сервисам. При `dm env sync` переменные раскидываются в `.env` каждого
//! сервиса согласно карте привязок в `dm.yaml`.
//!
//! Формат единого `.env`:
//! ```text
//! # Глобальные переменные (попадают во все сервисы)
//! LOG_LEVEL=info
//!
//! [api]
//! DATABASE_URL=postgres://localhost/api
//! PORT=3001
//!
//! [web]
//! API_URL=http://localhost:3001
//! ```

pub mod sync;

pub use sync::{EnvSection, UnifiedEnv, parse_unified_env, write_service_env};
