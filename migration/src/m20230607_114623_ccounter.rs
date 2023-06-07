use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(KVStore::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(KVStore::Key).primary_key().string().not_null())
                    .col(ColumnDef::new(KVStore::Value).json().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(KVStore::Table).to_owned()).await
    }
}

#[derive(Iden)]
enum KVStore {
    Table,
    Key,
    Value,
}
