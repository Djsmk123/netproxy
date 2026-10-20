//! Root CA used to mint per-host leaf certificates.
//!
//! M1 stores the key as a 0600 file in the app data directory. Moving it into the macOS Keychain
//! (and the device-setup/rotation flow) is an M2 task; the key is never part of any export.

use crate::engine::EngineError;
use hudsucker::rcgen::{
    BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose,
};
use std::{fs, io::Write, path::Path};

#[derive(Clone)]
pub struct CaMaterial {
    pub cert_pem: String,
    pub key_pem: String,
}

impl CaMaterial {
    pub fn generate(common_name: &str) -> Result<Self, EngineError> {
        let e = |x: hudsucker::rcgen::Error| EngineError::Ca(x.to_string());
        let key = KeyPair::generate().map_err(e)?;
        let mut p = CertificateParams::default();
        p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        p.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        p.distinguished_name.push(DnType::CommonName, common_name);
        p.distinguished_name
            .push(DnType::OrganizationName, "NetProxy");
        let cert = p.self_signed(&key).map_err(e)?;
        Ok(Self {
            cert_pem: cert.pem(),
            key_pem: key.serialize_pem(),
        })
    }

    /// Loads the CA from `dir`, creating it on first use. `ca.pem` (public) always lives in `dir`;
    /// the private key lives where `storage` says.
    pub fn load_or_create_with(
        dir: &Path,
        common_name: &str,
        storage: &KeyStorage,
    ) -> Result<Self, EngineError> {
        let (cp, kp) = (dir.join("ca.pem"), dir.join("ca.key"));
        let ca_err = |e: &dyn std::fmt::Display| EngineError::Ca(e.to_string());
        let read_cert = || fs::read_to_string(&cp).map_err(|e| ca_err(&e));
        let validate = |ca: Self| -> Result<Self, EngineError> {
            KeyPair::from_pem(&ca.key_pem)
                .map_err(|e| EngineError::Ca(format!("corrupt CA key: {e}")))?;
            Ok(ca)
        };
        match storage {
            KeyStorage::File => {
                if cp.exists() && kp.exists() {
                    return validate(Self {
                        cert_pem: read_cert()?,
                        key_pem: fs::read_to_string(&kp).map_err(|e| ca_err(&e))?,
                    });
                }
            }
            #[cfg(target_os = "macos")]
            KeyStorage::Keychain { service } => {
                let account = dir.to_string_lossy().into_owned();
                if cp.exists() {
                    if let Some(key) = keychain::get(service, &account)? {
                        return validate(Self {
                            cert_pem: read_cert()?,
                            key_pem: key,
                        });
                    }
                    // Legacy: a plaintext key file from before Keychain storage. Move it, then delete it.
                    if kp.exists() {
                        let key_pem = fs::read_to_string(&kp).map_err(|e| ca_err(&e))?;
                        let ca = validate(Self {
                            cert_pem: read_cert()?,
                            key_pem,
                        })?;
                        keychain::set(service, &account, &ca.key_pem)?;
                        fs::remove_file(&kp).map_err(|e| ca_err(&e))?;
                        return Ok(ca);
                    }
                }
            }
            #[cfg(not(target_os = "macos"))]
            KeyStorage::Keychain { .. } => {
                return Err(EngineError::Ca(
                    "Keychain storage is only available on macOS".into(),
                ))
            }
        }
        fs::create_dir_all(dir).map_err(|e| ca_err(&e))?;
        let ca = Self::generate(common_name)?;
        match storage {
            KeyStorage::File => {
                let _ = fs::remove_file(&kp);
                write_private(&kp, ca.key_pem.as_bytes()).map_err(|e| ca_err(&e))?;
            }
            #[cfg(target_os = "macos")]
            KeyStorage::Keychain { service } => {
                keychain::set(service, &dir.to_string_lossy(), &ca.key_pem)?
            }
            #[cfg(not(target_os = "macos"))]
            KeyStorage::Keychain { .. } => unreachable!(),
        }
        fs::write(&cp, &ca.cert_pem).map_err(|e| ca_err(&e))?;
        Ok(ca)
    }

    pub fn load_or_create(dir: &Path, common_name: &str) -> Result<Self, EngineError> {
        Self::load_or_create_with(dir, common_name, &KeyStorage::File)
    }

    /// Certificate as DER (what Android's "CA certificate" installer and `.crt` downloads expect).
    pub fn cert_der(&self) -> Result<Vec<u8>, EngineError> {
        use hudsucker::rustls::pki_types::{pem::PemObject, CertificateDer};
        CertificateDer::from_pem_slice(self.cert_pem.as_bytes())
            .map(|c| c.to_vec())
            .map_err(|e| EngineError::Ca(e.to_string()))
    }

    /// iOS/macOS configuration profile that installs the CA as a trusted root candidate.
    /// (After installing, iOS still requires enabling it under Settings > General > About >
    /// Certificate Trust Settings.) The profile is unsigned, so the OS shows it as "Not Verified".
    pub fn mobileconfig(&self, display_name: &str) -> Result<String, EngineError> {
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD.encode(self.cert_der()?);
        let (u1, u2) = (random_uuid(), random_uuid());
        let name = display_name.replace('&', "&amp;").replace('<', "&lt;");
        Ok(format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>PayloadContent</key><array><dict>
<key>PayloadCertificateFileName</key><string>netproxy-ca.cer</string>
<key>PayloadContent</key><data>{b64}</data>
<key>PayloadDescription</key><string>Adds the NetProxy root certificate</string>
<key>PayloadDisplayName</key><string>{name}</string>
<key>PayloadIdentifier</key><string>dev.netproxy.ca.cert.{u2}</string>
<key>PayloadType</key><string>com.apple.security.root</string>
<key>PayloadUUID</key><string>{u2}</string>
<key>PayloadVersion</key><integer>1</integer>
</dict></array>
<key>PayloadDisplayName</key><string>{name}</string>
<key>PayloadIdentifier</key><string>dev.netproxy.ca.{u1}</string>
<key>PayloadRemovalDisallowed</key><false/>
<key>PayloadType</key><string>Configuration</string>
<key>PayloadUUID</key><string>{u1}</string>
<key>PayloadVersion</key><integer>1</integer>
</dict></plist>
"#
        ))
    }
}

fn random_uuid() -> String {
    // v4-shaped identifier from process-local entropy (profiles only need uniqueness, not secrecy).
    use std::hash::{BuildHasher, Hasher};
    let mut b = [0u8; 16];
    for chunk in b.chunks_mut(8) {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        chunk.copy_from_slice(&h.finish().to_le_bytes());
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02X}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// Where the CA private key is kept.
#[derive(Clone, Debug)]
pub enum KeyStorage {
    /// `ca.key` next to the certificate, mode 0600. For tests and non-macOS hosts.
    File,
    /// macOS login Keychain, as a generic password item `(service, data-dir)`.
    Keychain { service: String },
}

#[cfg(target_os = "macos")]
mod keychain {
    use crate::engine::EngineError;
    use security_framework::passwords::{get_generic_password, set_generic_password};

    const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

    pub fn get(service: &str, account: &str) -> Result<Option<String>, EngineError> {
        match get_generic_password(service, account) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|e| EngineError::Ca(format!("keychain item is not UTF-8: {e}"))),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
            Err(e) => Err(EngineError::Ca(format!("keychain read failed: {e}"))),
        }
    }

    pub fn set(service: &str, account: &str, key_pem: &str) -> Result<(), EngineError> {
        set_generic_password(service, account, key_pem.as_bytes())
            .map_err(|e| EngineError::Ca(format!("keychain write failed: {e}")))
    }
}

#[cfg(unix)]
fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(data)
}

#[cfg(not(unix))]
fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    fs::write(path, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn der_and_profile_are_well_formed() {
        let ca = CaMaterial::generate("Profile CA").unwrap();
        let der = ca.cert_der().unwrap();
        assert_eq!(der[0], 0x30, "DER certificate starts with a SEQUENCE");
        let p = ca.mobileconfig("Net & Proxy").unwrap();
        assert!(p.contains("com.apple.security.root") && p.contains("Net &amp; Proxy"));
        use base64::Engine as _;
        assert!(p.contains(&base64::engine::general_purpose::STANDARD.encode(&der)));
        assert_ne!(super::random_uuid(), super::random_uuid());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn keychain_storage_keeps_key_out_of_the_filesystem_and_migrates_legacy_files() {
        let d = std::env::temp_dir().join(format!("np-kc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let service = format!("dev.netproxy.test.{}", std::process::id());
        let st = KeyStorage::Keychain {
            service: service.clone(),
        };
        // a) legacy plaintext key file gets moved into the keychain and deleted
        let legacy = CaMaterial::load_or_create(&d, "KC CA").unwrap();
        assert!(d.join("ca.key").exists());
        let migrated = CaMaterial::load_or_create_with(&d, "KC CA", &st).unwrap();
        assert_eq!(migrated.key_pem, legacy.key_pem);
        assert!(
            !d.join("ca.key").exists(),
            "plaintext key must be gone after migration"
        );
        // b) reload comes from the keychain
        let again = CaMaterial::load_or_create_with(&d, "KC CA", &st).unwrap();
        assert_eq!(
            (again.cert_pem, again.key_pem),
            (legacy.cert_pem, legacy.key_pem)
        );
        // c) fresh dir => generated straight into the keychain, never on disk
        let d2 = d.join("fresh");
        let fresh = CaMaterial::load_or_create_with(&d2, "KC CA 2", &st).unwrap();
        assert!(!d2.join("ca.key").exists() && d2.join("ca.pem").exists());
        assert_eq!(
            CaMaterial::load_or_create_with(&d2, "KC CA 2", &st)
                .unwrap()
                .key_pem,
            fresh.key_pem
        );
        // cleanup keychain items
        for acct in [
            d.to_string_lossy().into_owned(),
            d2.to_string_lossy().into_owned(),
        ] {
            let _ = security_framework::passwords::delete_generic_password(&service, &acct);
        }
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn create_then_reload_same_ca_with_private_key_mode() {
        let d = std::env::temp_dir().join(format!("np-ca-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        let a = CaMaterial::load_or_create(&d, "Test CA").unwrap();
        let b = CaMaterial::load_or_create(&d, "Test CA").unwrap();
        assert_eq!(a.cert_pem, b.cert_pem);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(d.join("ca.key")).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::remove_dir_all(&d).unwrap();
    }
}
