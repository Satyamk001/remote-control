use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::fs;
use std::path::PathBuf;
use tracing::{info, error};

pub struct Identity {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl Identity {
    pub fn load_or_generate(app_data_dir: PathBuf) -> Self {
        let key_path = app_data_dir.join("identity.key");

        if key_path.exists() {
            if let Ok(key_bytes) = fs::read(&key_path) {
                if key_bytes.len() == 32 {
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(&key_bytes);
                    let signing_key = SigningKey::from_bytes(&bytes);
                    let verifying_key = signing_key.verifying_key();
                    info!("Loaded existing device identity key.");
                    return Self { signing_key, verifying_key };
                }
            }
        }

        info!("Generating new device identity key...");
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        
        if let Err(e) = fs::write(&key_path, signing_key.to_bytes()) {
            error!("Failed to save identity key: {}", e);
        }

        Self {
            verifying_key: signing_key.verifying_key(),
            signing_key,
        }
    }
}
