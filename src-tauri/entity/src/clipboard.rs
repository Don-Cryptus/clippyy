//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.1

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "clipboard"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub r#type: String,
    pub star: Option<bool>,
    pub created_date: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    Type,
    Star,
    CreatedDate,
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
    ClipboardFile,
    ClipboardHtml,
    ClipboardImage,
    ClipboardRtf,
    ClipboardText,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::Integer.def(),
            Self::Type => ColumnType::String(StringLen::None).def(),
            Self::Star => ColumnType::Boolean.def().null(),
            Self::CreatedDate => ColumnType::DateTime.def().null(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::ClipboardFile => Entity::has_many(super::clipboard_file::Entity).into(),
            Self::ClipboardHtml => Entity::has_many(super::clipboard_html::Entity).into(),
            Self::ClipboardImage => Entity::has_many(super::clipboard_image::Entity).into(),
            Self::ClipboardRtf => Entity::has_many(super::clipboard_rtf::Entity).into(),
            Self::ClipboardText => Entity::has_many(super::clipboard_text::Entity).into(),
        }
    }
}

impl Related<super::clipboard_file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClipboardFile.def()
    }
}

impl Related<super::clipboard_html::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClipboardHtml.def()
    }
}

impl Related<super::clipboard_image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClipboardImage.def()
    }
}

impl Related<super::clipboard_rtf::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClipboardRtf.def()
    }
}

impl Related<super::clipboard_text::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClipboardText.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
