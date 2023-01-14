use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Quote::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Quote::Id).big_integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Quote::ServerId).big_unsigned().not_null())
                    .col(ColumnDef::new(Quote::ChannelId).big_unsigned().not_null())
                    .col(ColumnDef::new(Quote::ChannelName).string().not_null())
                    .col(ColumnDef::new(Quote::MessageId).big_unsigned().unique_key().null())
                    .col(ColumnDef::new(Quote::AuthorId).big_unsigned().not_null())
                    .col(ColumnDef::new(Quote::Author).string().not_null())
                    .col(ColumnDef::new(Quote::AuthorImage).blob(BlobSize::Medium).null())
                    .col(ColumnDef::new(Quote::Timestamp).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Quote::Text).string().not_null())
                    .col(ColumnDef::new(Quote::Attachment).blob(BlobSize::Medium).null())
                    .col(ColumnDef::new(Quote::AttachmentName).string().null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create().name("quote-author-id-index").table(Quote::Table).col(Quote::AuthorId).to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Quote::Table).to_owned()).await
    }
}

#[derive(Iden)]
enum Quote {
    Table,
    Id,
    ServerId,
    ChannelId,
    ChannelName,
    MessageId,
    AuthorId,
    Author,
    AuthorImage,
    Timestamp,
    Text,
    Attachment,
    AttachmentName,
}
