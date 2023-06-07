use anyhow::Result;
use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};

use entity::{kv_store, prelude::KvStore};

pub async fn get<T: for<'a> Deserialize<'a>>(db: &DatabaseConnection, key: &str) -> Result<Option<T>> {
    let db_value = KvStore::find_by_id(key).one(db).await?;
    let Some(json_value) = db_value else { return Ok(None) };
    Ok(Some(serde_json::from_value(json_value.value)?))
}

pub async fn set<T: Serialize>(db: &DatabaseConnection, key: &str, val: &T) -> Result<()> {
    let val = kv_store::ActiveModel { key: Set(key.to_owned()), value: Set(serde_json::to_value(val)?) };
    kv_store::Entity::insert(val)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(kv_store::Column::Key)
                .update_column(kv_store::Column::Value)
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}
