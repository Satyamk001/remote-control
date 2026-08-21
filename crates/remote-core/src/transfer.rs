use crc32fast::Hasher;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum TransferError {
    #[error("Invalid filename path traversal attempt")]
    PathTraversal,
    #[error("File already exists")]
    FileExists,
    #[error("Checksum mismatch")]
    ChecksumMismatch,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid state")]
    InvalidState,
}

pub struct ActiveTransfer {
    pub file_id: String,
    pub filename: String,
    pub temp_path: PathBuf,
    pub file: File,
    pub expected_seq: u64,
    pub hasher: Hasher,
    pub total_size: u64,
    pub bytes_written: u64,
}

pub struct FileReceiver {
    downloads_dir: PathBuf,
    active_transfers: HashMap<String, ActiveTransfer>,
}

impl FileReceiver {
    pub fn new(downloads_dir: PathBuf) -> Self {
        Self {
            downloads_dir,
            active_transfers: HashMap::new(),
        }
    }

    fn sanitize_filename(name: &str) -> Result<String, TransferError> {
        if name.contains('/') || name.contains('\\') || name.contains("..") {
            return Err(TransferError::PathTraversal);
        }
        Ok(name.to_string())
    }

    pub fn accept_offer(&mut self, file_id: &str, filename: &str, size: u64) -> Result<(), TransferError> {
        let safe_name = Self::sanitize_filename(filename)?;
        let final_path = self.downloads_dir.join(&safe_name);
        
        if final_path.exists() {
            return Err(TransferError::FileExists);
        }

        let temp_path = self.downloads_dir.join(format!("{}.part", file_id));
        let file = File::create(&temp_path)?;

        let transfer = ActiveTransfer {
            file_id: file_id.to_string(),
            filename: safe_name,
            temp_path,
            file,
            expected_seq: 0,
            hasher: Hasher::new(),
            total_size: size,
            bytes_written: 0,
        };

        self.active_transfers.insert(file_id.to_string(), transfer);
        info!("Accepted file offer for {}", filename);
        Ok(())
    }

    pub fn receive_chunk(&mut self, file_id: &str, seq: u64, data: &[u8], checksum: u32) -> Result<(), TransferError> {
        let transfer = self.active_transfers.get_mut(file_id).ok_or(TransferError::InvalidState)?;

        if seq != transfer.expected_seq {
            error!("Sequence mismatch. Expected {}, got {}", transfer.expected_seq, seq);
            return Err(TransferError::InvalidState);
        }

        let mut chunk_hasher = Hasher::new();
        chunk_hasher.update(data);
        if chunk_hasher.finalize() != checksum {
            return Err(TransferError::ChecksumMismatch);
        }

        transfer.file.write_all(data)?;
        transfer.hasher.update(data);
        transfer.bytes_written += data.len() as u64;
        transfer.expected_seq += 1;

        Ok(())
    }

    pub fn complete_transfer(&mut self, file_id: &str) -> Result<(), TransferError> {
        let transfer = self.active_transfers.remove(file_id).ok_or(TransferError::InvalidState)?;
        
        // Finalize the file
        drop(transfer.file);

        let final_path = self.downloads_dir.join(&transfer.filename);
        fs::rename(&transfer.temp_path, &final_path)?;
        
        info!("File transfer complete: {:?}", final_path);
        Ok(())
    }

    pub fn cancel_transfer(&mut self, file_id: &str) {
        if let Some(transfer) = self.active_transfers.remove(file_id) {
            drop(transfer.file);
            let _ = fs::remove_file(&transfer.temp_path);
            info!("Cancelled file transfer: {}", transfer.filename);
        }
    }
}
