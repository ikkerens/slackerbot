use crate::m20220101_000001_create_table::Quote;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create().name("quote-channel-id-index").table(Quote::Table).col(Quote::ChannelId).to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_index(Index::drop().name("quote-channel-id-index").table(Quote::Table).to_owned()).await
    }
}
