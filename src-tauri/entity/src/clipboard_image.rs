//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.1

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "clipboard_image"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub clipboard_id: i32,
    pub data: Vec<u8>,
    pub extension: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub size: Option<String>,
    pub thumbnail: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    ClipboardId,
    Data,
    Extension,
    Width,
    Height,
    Size,
    Thumbnail,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Id,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = i32;
    fn auto_increment() -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Clipboard,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::Integer.def(),
            Self::ClipboardId => ColumnType::Integer.def(),
            Self::Data => ColumnType::Blob.def(),
            Self::Extension => ColumnType::String(StringLen::None).def().null(),
            Self::Width => ColumnType::Integer.def().null(),
            Self::Height => ColumnType::Integer.def().null(),
            Self::Size => ColumnType::String(StringLen::None).def().null(),
            Self::Thumbnail => ColumnType::Text.def().null(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Clipboard => Entity::belongs_to(super::clipboard::Entity)
                .from(Column::ClipboardId)
                .to(super::clipboard::Column::Id)
                .into(),
        }
    }
}

impl Related<super::clipboard::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Clipboard.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
