pub mod analytics;
pub mod discovery;
pub mod lastfm;
pub mod recommendation;
pub mod settings;
pub mod sync;

use std::{collections::HashSet, sync::Arc};

use serde::Serialize;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::auth::AppAuth;

#[derive(Clone)]
pub struct AppState {
    pub auth: AppAuth,
    pub pool: sqlx::SqlitePool,
    pub sync: sync::SyncService,
    pub sync_jobs: Arc<Mutex<HashSet<String>>>,
    pub analytics: analytics::AnalyticsService,
    pub discovery: discovery::DiscoveryService,
    pub recommendations: recommendation::RecommendationService,
    pub settings: settings::SettingsService,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScanAutoSyncResult {
    pub check: sync::ScanCheckResult,
    pub sync_started: bool,
    pub sync_skipped_reason: Option<String>,
}

impl AppState {
    pub async fn check_scan_and_maybe_start_sync(
        self: &Arc<Self>,
        reason: &'static str,
    ) -> anyhow::Result<ScanAutoSyncResult> {
        let check = self.sync.check_subsonic_scan(&self.pool).await?;
        let mut result = ScanAutoSyncResult {
            check,
            sync_started: false,
            sync_skipped_reason: None,
        };

        if !result.check.changed {
            return Ok(result);
        }

        let Some(last_scan) = result.check.scan_status.last_scan.clone() else {
            result.sync_skipped_reason = Some("missing_last_scan".to_string());
            return Ok(result);
        };

        if !reserve_sync_jobs(self, &["subsonic", "lastfm"]).await {
            result.sync_skipped_reason = Some("sync_already_running".to_string());
            return Ok(result);
        }

        if let Err(error) = self.sync.mark_running(&self.pool, 1, "subsonic").await {
            release_sync_jobs(self, &["subsonic", "lastfm"]).await;
            return Err(error);
        }
        if let Err(error) = self.sync.mark_running(&self.pool, 2, "lastfm").await {
            release_sync_jobs(self, &["subsonic", "lastfm"]).await;
            return Err(error);
        }

        let job_state = self.clone();
        tokio::spawn(async move {
            let mut completed = true;
            match job_state.sync.sync_subsonic(&job_state.pool).await {
                Ok(imported_tracks) => {
                    info!(
                        source = "subsonic",
                        reason, imported_tracks, "automatic scan sync step completed"
                    );
                }
                Err(error) => {
                    completed = false;
                    error!(
                        source = "subsonic",
                        reason,
                        error = %error,
                        "automatic scan sync step failed"
                    );
                }
            }

            match job_state.sync.sync_lastfm(&job_state.pool).await {
                Ok(updated_artists) => {
                    info!(
                        source = "lastfm",
                        reason, updated_artists, "automatic scan sync step completed"
                    );
                }
                Err(error) => {
                    completed = false;
                    error!(
                        source = "lastfm",
                        reason,
                        error = %error,
                        "automatic scan sync step failed"
                    );
                }
            }

            if completed {
                if let Err(error) = job_state
                    .sync
                    .mark_scan_processed(&job_state.pool, &last_scan)
                    .await
                {
                    error!(
                        reason,
                        last_scan,
                        error = %error,
                        "failed to mark Navidrome scan as processed"
                    );
                }
            }

            release_sync_jobs(&job_state, &["subsonic", "lastfm"]).await;
        });

        result.sync_started = true;
        Ok(result)
    }
}

async fn reserve_sync_jobs(state: &Arc<AppState>, sources: &[&str]) -> bool {
    let mut running = state.sync_jobs.lock().await;
    if sources.iter().any(|source| running.contains(*source)) {
        return false;
    }
    for source in sources {
        running.insert((*source).to_string());
    }
    true
}

async fn release_sync_jobs(state: &Arc<AppState>, sources: &[&str]) {
    let mut running = state.sync_jobs.lock().await;
    for source in sources {
        running.remove(*source);
    }
}

#[allow(dead_code)]
pub type SharedState = Arc<AppState>;
