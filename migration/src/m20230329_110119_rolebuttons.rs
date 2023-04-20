use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RoleButtonServer::Table)
                    .col(ColumnDef::new(RoleButtonServer::Id).big_integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(RoleButtonServer::ServerId).big_unsigned().not_null().unique_key())
                    .col(ColumnDef::new(RoleButtonServer::PostChannelId).big_unsigned().null())
                    .col(ColumnDef::new(RoleButtonServer::PostMessageId).big_unsigned().null())
                    .col(ColumnDef::new(RoleButtonServer::Roles).array(ColumnType::BigInteger).not_null())
                    .col(ColumnDef::new(RoleButtonServer::RoleEmojis).array(ColumnType::String(None)).not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("rolebuttonserver-server-id-index")
                    .table(RoleButtonServer::Table)
                    .col(RoleButtonServer::ServerId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(RoleButtonServer::Table).to_owned()).await
    }
}

#[derive(Iden)]
enum RoleButtonServer {
    Table,
    Id,
    ServerId,
    PostChannelId,
    PostMessageId,
    Roles,
    RoleEmojis,
}
