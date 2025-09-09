use crate::entities::bundle;
use crate::traits::BundleDatastore;
use alloy_rpc_types_mev::EthSendBundle;
use anyhow::Result;
use sea_orm::prelude::*;
use sea_orm::{ActiveModelTrait, EntityTrait};

/// PostgreSQL implementation of the BundleDatastore trait
pub struct PostgresDatastore {
    db: DatabaseConnection,
}

impl PostgresDatastore {
    /// Create a new PostgreSQL datastore instance
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Get a reference to the database connection
    pub fn connection(&self) -> &DatabaseConnection {
        &self.db
    }
}

#[async_trait::async_trait]
impl BundleDatastore for PostgresDatastore {
    async fn insert_bundle(&self, bundle: EthSendBundle) -> Result<Uuid> {
        let active_model: bundle::ActiveModel = bundle.into();
        let model = active_model.insert(&self.db).await?;
        Ok(model.id)
    }

    async fn get_bundle(&self, id: Uuid) -> Result<Option<EthSendBundle>> {
        let model = bundle::Entity::find_by_id(id).one(&self.db).await?;
        match model {
            Some(m) => Ok(Some(m.try_into()?)),
            None => Ok(None),
        }
    }
}
