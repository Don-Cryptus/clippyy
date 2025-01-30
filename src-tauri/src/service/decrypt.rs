use std::thread::sleep;

use super::{
    clipboard::load_clipboards_with_relations,
    encrypt::{is_key_set, looks_like_encrypted_data},
    settings::{get_global_settings, update_settings_db},
    sync::{get_sync_manager, get_sync_provider},
};
use crate::{
    prelude::*,
    service::clipboard::upsert_clipboard_dto,
    tao::{connection::db, global::get_app},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use common::types::{
    cipher::{EncryptionError, ENCRYPTION_KEY},
    enums::ListenEvent,
    orm_query::FullClipboardDto,
    types::{CommandError, Progress},
};
use entity::clipboard;
use ring::aead;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tauri::{Emitter, EventTarget};

pub async fn decrypt_all_clipboards() -> Result<(), CommandError> {
    let settings = get_global_settings();
    let db = db().await?;

    // Get all local encrypted clipboards
    let mut clipboards = load_clipboards_with_relations(
        clipboard::Entity::find()
            .filter(clipboard::Column::Encrypted.eq(true))
            .all(&db)
            .await?,
    )
    .await;

    // Get remote clipboards if sync enabled
    let (provider, remote_clipboards) = if settings.sync {
        // Stop the sync manager before making changes
        get_sync_manager().lock().await.stop().await;

        let provider = get_sync_provider().await;
        let remote_clipboards = provider
            .fetch_all_clipboards()
            .await
            .expect("Failed to fetch remote clipboards");

        // Download all remote clipboards with progress logging
        let download_total = remote_clipboards.len();
        for (index, remote) in remote_clipboards.iter().enumerate() {
            get_app().emit_to(
                EventTarget::any(),
                ListenEvent::Progress.to_string().as_str(),
                Progress {
                    label: "SETTINGS.ENCRYPT.DOWNLOADING_REMOTE_CLIPBOARDS".to_string(),
                    total: download_total,
                    current: index + 1,
                },
            )?;

            if !remote.encrypted {
                continue;
            }

            if let Ok(clipboard) = provider.download_by_id(&remote.provider_id).await {
                clipboards.push(clipboard);
            }
        }
        (Some(provider), remote_clipboards)
    } else {
        (None, Vec::new())
    };

    let total = clipboards.len();
    for (index, clipboard) in clipboards.into_iter().enumerate() {
        get_app().emit_to(
            EventTarget::any(),
            ListenEvent::Progress.to_string().as_str(),
            Progress {
                label: "SETTINGS.ENCRYPT.DECRYPTION_PROGRESS".to_string(),
                total,
                current: index + 1,
            },
        )?;
        match decrypt_clipboard(clipboard.clone()) {
            Ok(decrypted) => {
                upsert_clipboard_dto(decrypted.clone()).await?;

                if let Some(provider) = &provider {
                    if let Some(remote) = remote_clipboards
                        .iter()
                        .find(|r| r.id == decrypted.clipboard.id)
                    {
                        provider
                            .update_clipboard(&decrypted, remote)
                            .await
                            .expect("Failed to update remote clipboard");
                    }
                }
            }
            Err(e) => {
                printlog!(
                    "Failed to decrypt clipboard {}: {:?}",
                    clipboard.clipboard.id,
                    e
                );

                // Delete locally
                clipboard::Entity::delete_by_id(clipboard.clipboard.id)
                    .exec(&db)
                    .await?;

                // Mark for deletion remotely if sync is enabled
                if let Some(provider) = &provider {
                    if let Some(remote) = remote_clipboards
                        .iter()
                        .find(|r| r.id == clipboard.clipboard.id)
                    {
                        provider.mark_for_deletion(remote).await;
                    }
                }
            }
        }
    }

    if settings.sync && provider.is_some() {
        // race condition with settings sync
        tauri::async_runtime::spawn(async {
            sleep(std::time::Duration::from_secs(5));
            get_sync_manager().lock().await.start().await;
        });
    }

    Ok(())
}

pub fn decrypt_clipboard(
    mut clipboard: FullClipboardDto,
) -> Result<FullClipboardDto, EncryptionError> {
    if !clipboard.clipboard.encrypted {
        return Err(EncryptionError::NotEncrypted);
    }

    if let Some(text) = &mut clipboard.text {
        match STANDARD.decode(&text.data) {
            Ok(decoded) => {
                match decrypt_data(&decoded) {
                    Ok(decrypted) => {
                        match String::from_utf8(decrypted) {
                            Ok(str_data) => text.data = str_data,
                            Err(e) => {
                                printlog!("Failed to convert decrypted text to UTF-8 for clipboard {}: {}", clipboard.clipboard.id, e);
                                return Err(EncryptionError::DecryptionFailed);
                            }
                        }
                    }
                    Err(e) => {
                        printlog!(
                            "Failed to decrypt text data for clipboard {}: {:?}",
                            clipboard.clipboard.id,
                            e
                        );
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                printlog!(
                    "Failed to base64 decode text for clipboard {}: {}",
                    clipboard.clipboard.id,
                    e
                );
                return Err(EncryptionError::DecryptionFailed);
            }
        }
    }

    if let Some(html) = &mut clipboard.html {
        match STANDARD.decode(&html.data) {
            Ok(decoded) => {
                match decrypt_data(&decoded) {
                    Ok(decrypted) => {
                        match String::from_utf8(decrypted) {
                            Ok(str_data) => html.data = str_data,
                            Err(e) => {
                                printlog!("Failed to convert decrypted HTML to UTF-8 for clipboard {}: {}", clipboard.clipboard.id, e);
                                return Err(EncryptionError::DecryptionFailed);
                            }
                        }
                    }
                    Err(e) => {
                        printlog!(
                            "Failed to decrypt HTML data for clipboard {}: {:?}",
                            clipboard.clipboard.id,
                            e
                        );
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                printlog!(
                    "Failed to base64 decode HTML for clipboard {}: {}",
                    clipboard.clipboard.id,
                    e
                );
                return Err(EncryptionError::DecryptionFailed);
            }
        }
    }

    if let Some(rtf) = &mut clipboard.rtf {
        match STANDARD.decode(&rtf.data) {
            Ok(decoded) => match decrypt_data(&decoded) {
                Ok(decrypted) => match String::from_utf8(decrypted) {
                    Ok(str_data) => rtf.data = str_data,
                    Err(e) => {
                        printlog!(
                            "Failed to convert decrypted RTF to UTF-8 for clipboard {}: {}",
                            clipboard.clipboard.id,
                            e
                        );
                        return Err(EncryptionError::DecryptionFailed);
                    }
                },
                Err(e) => {
                    printlog!(
                        "Failed to decrypt RTF data for clipboard {}: {:?}",
                        clipboard.clipboard.id,
                        e
                    );
                    return Err(e);
                }
            },
            Err(e) => {
                printlog!(
                    "Failed to base64 decode RTF for clipboard {}: {}",
                    clipboard.clipboard.id,
                    e
                );
                return Err(EncryptionError::DecryptionFailed);
            }
        }
    }

    if let Some(image) = &mut clipboard.image {
        match decrypt_data(&image.data) {
            Ok(decrypted) => image.data = decrypted,
            Err(e) => {
                printlog!(
                    "Failed to decrypt image data for clipboard {}: {:?}",
                    clipboard.clipboard.id,
                    e
                );
                return Err(e);
            }
        }

        match STANDARD.decode(&image.thumbnail) {
            Ok(thumbnail_decoded) => match decrypt_data(&thumbnail_decoded) {
                Ok(thumbnail_decrypted) => image.thumbnail = STANDARD.encode(thumbnail_decrypted),
                Err(e) => {
                    printlog!(
                        "Failed to decrypt image thumbnail for clipboard {}: {:?}",
                        clipboard.clipboard.id,
                        e
                    );
                    return Err(e);
                }
            },
            Err(e) => {
                printlog!(
                    "Failed to base64 decode image thumbnail for clipboard {}: {}",
                    clipboard.clipboard.id,
                    e
                );
                return Err(EncryptionError::DecryptionFailed);
            }
        }
    }

    if !clipboard.files.is_empty() {
        for (index, file) in clipboard.files.iter_mut().enumerate() {
            match STANDARD.decode(&file.name) {
                Ok(name_decoded) => match decrypt_data(&name_decoded) {
                    Ok(name_decrypted) => match String::from_utf8(name_decrypted) {
                        Ok(str_data) => file.name = str_data,
                        Err(e) => {
                            printlog!("Failed to convert decrypted filename to UTF-8 for clipboard {} file {}: {}", 
                                        clipboard.clipboard.id, index, e);
                            return Err(EncryptionError::DecryptionFailed);
                        }
                    },
                    Err(e) => {
                        printlog!(
                            "Failed to decrypt filename for clipboard {} file {}: {:?}",
                            clipboard.clipboard.id,
                            index,
                            e
                        );
                        return Err(e);
                    }
                },
                Err(e) => {
                    printlog!(
                        "Failed to base64 decode filename for clipboard {} file {}: {}",
                        clipboard.clipboard.id,
                        index,
                        e
                    );
                    return Err(EncryptionError::DecryptionFailed);
                }
            }

            match decrypt_data(&file.data) {
                Ok(decrypted) => file.data = decrypted,
                Err(e) => {
                    printlog!(
                        "Failed to decrypt file data for clipboard {} file {}: {:?}",
                        clipboard.clipboard.id,
                        index,
                        e
                    );
                    return Err(e);
                }
            }

            if let Some(extension) = &file.extension {
                match STANDARD.decode(extension) {
                    Ok(ext_decoded) => {
                        match decrypt_data(&ext_decoded) {
                            Ok(ext_decrypted) => match String::from_utf8(ext_decrypted) {
                                Ok(str_data) => file.extension = Some(str_data),
                                Err(e) => {
                                    printlog!("Failed to convert decrypted file extension to UTF-8 for clipboard {} file {}: {}", 
                                            clipboard.clipboard.id, index, e);
                                    return Err(EncryptionError::DecryptionFailed);
                                }
                            },
                            Err(e) => {
                                printlog!("Failed to decrypt file extension for clipboard {} file {}: {:?}", 
                                    clipboard.clipboard.id, index, e);
                                return Err(e);
                            }
                        }
                    }
                    Err(e) => {
                        printlog!(
                            "Failed to base64 decode file extension for clipboard {} file {}: {}",
                            clipboard.clipboard.id,
                            index,
                            e
                        );
                        return Err(EncryptionError::DecryptionFailed);
                    }
                }
            }

            if let Some(mime_type) = &file.mime_type {
                match STANDARD.decode(mime_type) {
                    Ok(mime_decoded) => match decrypt_data(&mime_decoded) {
                        Ok(mime_decrypted) => match String::from_utf8(mime_decrypted) {
                            Ok(str_data) => file.mime_type = Some(str_data),
                            Err(e) => {
                                printlog!("Failed to convert decrypted mime type to UTF-8 for clipboard {} file {}: {}", 
                                            clipboard.clipboard.id, index, e);
                                return Err(EncryptionError::DecryptionFailed);
                            }
                        },
                        Err(e) => {
                            printlog!(
                                "Failed to decrypt mime type for clipboard {} file {}: {:?}",
                                clipboard.clipboard.id,
                                index,
                                e
                            );
                            return Err(e);
                        }
                    },
                    Err(e) => {
                        printlog!(
                            "Failed to base64 decode mime type for clipboard {} file {}: {}",
                            clipboard.clipboard.id,
                            index,
                            e
                        );
                        return Err(EncryptionError::DecryptionFailed);
                    }
                }
            }
        }
    }

    clipboard.clipboard.encrypted = false;
    Ok(clipboard)
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

pub async fn remove_encryption(password: String) -> Result<(), CommandError> {
    if !is_key_set() {
        return Err(CommandError::new("MAIN.ERROR.NO_ENCRYPTION_KEY_SET"));
    }

    let is_password_valid =
        verify_password(password).map_err(|e| CommandError::new(&e.to_string()))?;

    if !is_password_valid {
        return Err(CommandError::new("MAIN.ERROR.INCORRECT_PASSWORD"));
    }

    decrypt_all_clipboards().await?;

    let mut settings = get_global_settings();
    settings.encryption = false;
    update_settings_db(settings).await?;

    clear_encryption_key();

    Ok(())
}

pub fn clear_encryption_key() {
    *ENCRYPTION_KEY
        .lock()
        .map_err(|e| CommandError::new(&e.to_string()))
        .unwrap() = None;
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
