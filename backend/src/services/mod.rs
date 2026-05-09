pub mod analytics;
pub mod discovery;
pub mod recommendation;
pub mod settings;
pub mod sync;

use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub sync: sync::SyncService,
    pub analytics: analytics::AnalyticsService,
    pub discovery: discovery::DiscoveryService,
    pub recommendations: recommendation::RecommendationService,
    pub settings: settings::SettingsService,
}

pub type SharedState = Arc<AppState>;
