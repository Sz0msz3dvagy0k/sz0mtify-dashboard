pub mod analytics;
pub mod discovery;
pub mod lastfm;
pub mod recommendation;
pub mod settings;
pub mod sync;

use std::{collections::HashSet, sync::Arc};

use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub sync: sync::SyncService,
    pub sync_jobs: Arc<Mutex<HashSet<String>>>,
    pub analytics: analytics::AnalyticsService,
    pub discovery: discovery::DiscoveryService,
    pub recommendations: recommendation::RecommendationService,
    pub settings: settings::SettingsService,
}

#[allow(dead_code)]
pub type SharedState = Arc<AppState>;
