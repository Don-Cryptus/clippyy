use sea_orm::EnumIter;
pub use sea_orm_migration::prelude::*;

mod m000001_create_clipboard;
mod m000002_create_settings;
mod m000003_create_hotkey;
mod m000004_seed;
pub struct Migrator;

#[derive(Iden, EnumIter)]
pub enum ClipboardType {
    #[iden = "text"]
    Text,
    #[iden = "image"]
    Image,
    #[iden = "link"]
    Link,
    #[iden = "hex"]
    Hex,
    #[iden = "rgb"]
    Rgb,
}

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m000001_create_clipboard::Migration),
            Box::new(m000002_create_settings::Migration),
            Box::new(m000003_create_hotkey::Migration),
            Box::new(m000004_seed::Migration),
        ]
    }
}
