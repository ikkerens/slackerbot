pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20230329_110119_rolebuttons;
mod m20230607_114623_ccounter;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20230329_110119_rolebuttons::Migration),
            Box::new(m20230607_114623_ccounter::Migration),
        ]
    }
}
