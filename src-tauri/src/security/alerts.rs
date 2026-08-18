//! Security alerts: webhook and email notifications for defense events.

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Alert configuration for webhook/email notifications.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertConfig {
    /// Webhook URL to POST alerts to (e.g. Slack, Discord, custom endpoint).
    pub webhook_url: Option<String>,
    /// Email address to send alerts to.
    pub email_to: Option<String>,
}

/// Send an alert for a defense event.
pub async fn send_alert(event: &super::defender::DefenseEvent, config: &AlertConfig) -> Result<()> {
    // Send webhook if configured
    if let Some(webhook_url) = &config.webhook_url {
        send_webhook(event, webhook_url).await?;
    }

    // Email alerts are logged but not implemented (would need SMTP config)
    if let Some(email) = &config.email_to {
        tracing::info!(
            "alert email would be sent to: {} (SMTP not yet configured)",
            email
        );
    }

    Ok(())
}

/// POST the event payload to the configured webhook URL.
async fn send_webhook(event: &super::defender::DefenseEvent, webhook_url: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "source": "aegis-ai",
        "version": env!("CARGO_PKG_VERSION"),
        "event": event,
        "timestamp_ms": time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000,
    });

    let resp = client
        .post(webhook_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            tracing::debug!("webhook alert sent successfully to {}", webhook_url);
        }
        Ok(r) => {
            tracing::warn!(
                "webhook alert returned HTTP {} from {}",
                r.status(),
                webhook_url
            );
        }
        Err(e) => {
            tracing::warn!("webhook alert failed for {}: {e}", webhook_url);
        }
    }

    Ok(())
}
