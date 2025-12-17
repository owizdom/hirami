//! Africa validator tagging logic.

use mev_africa_db::DbPool;
use sqlx::Row;
use std::collections::HashSet;
use tracing::{debug, info};

/// Validator tagger for identifying Africa-related blocks.
pub struct ValidatorTagger {
    africa_fee_recipients: HashSet<String>,
}

impl ValidatorTagger {
    /// Create a new validator tagger from the database.
    ///
    /// # Arguments
    /// * `db` - Database pool
    pub async fn new(db: &DbPool) -> anyhow::Result<Self> {
        let rows = sqlx::query("SELECT DISTINCT fee_recipient FROM validators")
            .fetch_all(db.pool())
            .await?;

        let mut fee_recipients = HashSet::new();
        for row in rows {
            let fee_recipient: String = row.get(0);
            fee_recipients.insert(fee_recipient.to_lowercase());
        }

        info!("Loaded {} Africa validator fee recipients", fee_recipients.len());
        Ok(Self {
            africa_fee_recipients: fee_recipients,
        })
    }

    /// Check if a fee recipient is associated with Africa validators.
    ///
    /// # Arguments
    /// * `fee_recipient` - The fee recipient address to check
    ///
    /// # Returns
    /// True if the fee recipient matches an Africa validator
    pub fn is_africa_tagged(&self, fee_recipient: &str) -> bool {
        let normalized = fee_recipient.to_lowercase();
        let is_tagged = self.africa_fee_recipients.contains(&normalized);
        if is_tagged {
            debug!("Fee recipient {} tagged as Africa validator", fee_recipient);
        }
        is_tagged
    }

    /// Refresh the validator list from the database.
    pub async fn refresh(&mut self, db: &DbPool) -> anyhow::Result<()> {
        let rows = sqlx::query("SELECT DISTINCT fee_recipient FROM validators")
            .fetch_all(db.pool())
            .await?;

        self.africa_fee_recipients.clear();
        for row in rows {
            let fee_recipient: String = row.get(0);
            self.africa_fee_recipients.insert(fee_recipient.to_lowercase());
        }

        info!("Refreshed {} Africa validator fee recipients", self.africa_fee_recipients.len());
        Ok(())
    }
}

