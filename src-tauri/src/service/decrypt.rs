use super::{
    clipboard::load_clipboards_with_relations, encrypt::looks_like_encrypted_data,
    settings::get_global_settings, sync::get_sync_provider,
};
use crate::{
    prelude::*, service::clipboard::upsert_clipboard_dto, tao::{connection::db, global::get_app}
};
use base64::{engine::general_purpose::STANDARD, Engine};
use common::{
    printlog,
    types::{
        crypto::{EncryptionError, ENCRYPTION_KEY},
        enums::ListenEvent,
        orm_query::FullClipboardDto,
        types::{CommandError, Progress},
    },
};
use entity::clipboard;
use ring::aead;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tauri::{Emitter, EventTarget};

pub async fn decrypt_all_clipboards() -> Result<(), CommandError> {
    let settings = get_global_settings();
    let db = db().await?;

    let clipboards = load_clipboards_with_relations(
        clipboard::Entity::find()
            .filter(clipboard::Column::Encrypted.eq(true))
            .all(&db)
            .await?,
    )
    .await;

    let (remote_clipboards, provider) = if settings.sync {
        let provider = get_sync_provider().await;
        (
            provider
                .fetch_all_clipboards()
                .await
                .expect("Failed to fetch remote clipboards"),
            Some(provider),
        )
    } else {
        (Vec::new(), None)
    };

    let total_clipboards = clipboards.len() as u64;
    for (index, clipboard) in clipboards.into_iter().enumerate() {
        let decrypted_clipboard = decrypt_clipboard(clipboard);

        // Update clipboard in database
        upsert_clipboard_dto(decrypted_clipboard.clone()).await?;

        if let Some(provider) = &provider {
            if let Some(remote_clipboards) = &remote_clipboards
                .iter()
                .find(|c| c.id == decrypted_clipboard.clipboard.id)
            {
                provider
                    .update_clipboard(&decrypted_clipboard, &remote_clipboards)
                    .await
                    .expect("Failed to upsert clipboard");
            }
        }

        let progress = Progress {
            total: total_clipboards as u64,
            current: (index + 1) as u64,
        };

        printlog!("Emitting progress event {:?}", progress);

        get_app()
            .emit_to(
                EventTarget::any(),
                ListenEvent::Progress.to_string().as_str(),
                progress,
            )
            .expect("Failed to emit download progress event");
    }

    Ok(())
}

pub fn decrypt_clipboard(
    mut clipboard: FullClipboardDto,
) -> FullClipboardDto {
    if let Some(text) = &mut clipboard.text {
        if let Ok(data) = STANDARD.decode(text.data.clone()) {
            text.data =
                String::from_utf8(decrypt_data(&data).expect("Failed to decrypt clipboard data"))
                    .expect("Failed to convert decrypted data to string");
        }
    }

    if let Some(html) = &mut clipboard.html {
        if let Ok(data) = STANDARD.decode(html.data.clone()) {
            html.data =
                String::from_utf8(decrypt_data(&data).expect("Failed to decrypt clipboard data"))
                    .expect("Failed to convert decrypted data to string");
        }
    }

    if let Some(rtf) = &mut clipboard.rtf {
        if let Ok(data) = STANDARD.decode(rtf.data.clone()) {
            rtf.data =
                String::from_utf8(decrypt_data(&data).expect("Failed to decrypt clipboard data"))
                    .expect("Failed to convert decrypted data to string");
        }
    }

    if let Some(image) = &mut clipboard.image {
        image.data =
            decrypt_data(&image.data.as_slice()).expect("Failed to decrypt clipboard data");

        if let Ok(thumbnail) = STANDARD.decode(image.thumbnail.clone()) {
            image.thumbnail = STANDARD.encode(
                decrypt_data(&thumbnail).expect("Failed to decrypt clipboard thumbnail data"),
            );
        }
    }

    if !clipboard.files.is_empty() {
        for file in &mut clipboard.files {
            if STANDARD.decode(file.name.clone()).is_ok() {
                file.data =
                    decrypt_data(file.data.as_slice()).expect("File data encryption failed");

                file.name = String::from_utf8(
                    decrypt_data(
                        STANDARD
                            .decode(file.name.clone())
                            .expect("Filename encryption failed")
                            .as_slice(),
                    )
                    .expect("Filename encryption failed"),
                )
                .expect("Failed to convert decrypted data to string");

                if let Some(extension) = &file.extension {
                    file.extension = Some(
                        String::from_utf8(
                            decrypt_data(
                                STANDARD
                                    .decode(extension)
                                    .expect("Filename encryption failed")
                                    .as_slice(),
                            )
                            .expect("File extension encryption failed"),
                        )
                        .expect("Failed to convert decrypted data to string"),
                    );
                }

                if let Some(mime_type) = &file.mime_type {
                    file.mime_type = Some(
                        String::from_utf8(
                            decrypt_data(
                                STANDARD
                                    .decode(mime_type)
                                    .expect("Filename encryption failed")
                                    .as_slice(),
                            )
                            .expect("MIME type encryption failed"),
                        )
                        .expect("Failed to convert decrypted data to string"),
                    );
                }
            }
        }
    }

    clipboard.clipboard.encrypted = false;

    clipboard
}

/// Decrypts data using AES-256-GCM
pub fn decrypt_data(encrypted_data: &[u8]) -> Result<Vec<u8>, EncryptionError> {
    if encrypted_data.len() < 12 {
        return Err(EncryptionError::NotEncrypted);
    }

    let key_bytes = ENCRYPTION_KEY
        .lock()
        .map_err(|_| EncryptionError::KeyLockFailed)?
        .ok_or(EncryptionError::NoKey)?;

    // Create unbound key from key bytes
    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .map_err(|_| EncryptionError::DecryptionFailed)?;
    let key = aead::LessSafeKey::new(unbound_key);

    // Split nonce and encrypted data
    let nonce = aead::Nonce::assume_unique_for_key(
        encrypted_data[..12]
            .try_into()
            .map_err(|_| EncryptionError::DecryptionFailed)?,
    );

    // Decrypt data
    let mut in_out = encrypted_data[12..].to_vec();
    match key.open_in_place(nonce, aead::Aad::empty(), &mut in_out) {
        Ok(decrypted) => Ok(decrypted.to_vec()),
        Err(_) => {
            if looks_like_encrypted_data(encrypted_data) {
                Err(EncryptionError::InvalidKey)
            } else {
                Err(EncryptionError::NotEncrypted)
            }
        }
    }
}

pub fn verify_password(password: String) -> Result<bool, EncryptionError> {
    let mut hasher = ring::digest::Context::new(&ring::digest::SHA256);
    hasher.update(password.as_bytes());
    let key = hasher.finish();
    let mut provided_key = [0u8; 32];
    provided_key.copy_from_slice(key.as_ref());

    let current_key = ENCRYPTION_KEY
        .lock()
        .map_err(|_| EncryptionError::KeyLockFailed)?
        .ok_or(EncryptionError::NoKey)?;

    Ok(provided_key == current_key)
}

pub fn init_password_lock() {
    get_app()
        .emit_to(
            EventTarget::any(),
            ListenEvent::PasswordLock.to_string().as_str(),
            (),
        )
        .expect("Failed to emit download progress event");
}
