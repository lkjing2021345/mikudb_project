//! TLS/SSL 加密支持模块
//!
//! 本模块实现 MikuDB 服务器的 TLS/SSL 加密功能:
//! - 证书和私钥加载
//! - TLS 配置构建
//! - 客户端证书验证(可选)
//! - 支持 TLS 1.2 和 TLS 1.3

#[cfg(feature = "tls")]
use crate::{ServerError, ServerResult};
#[cfg(feature = "tls")]
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
#[cfg(feature = "tls")]
use rustls::server::WebPkiClientVerifier;
#[cfg(feature = "tls")]
use rustls::{RootCertStore, ServerConfig};
#[cfg(feature = "tls")]
use rustls_pemfile::{certs, private_key};
#[cfg(feature = "tls")]
use std::fs::File;
#[cfg(feature = "tls")]
use std::io::BufReader;
#[cfg(feature = "tls")]
use std::path::Path;
#[cfg(feature = "tls")]
use std::sync::Arc;
#[cfg(feature = "tls")]
use tracing::{debug, info};

/// TLS 配置构建器
#[cfg(feature = "tls")]
pub struct TlsConfigBuilder;

#[cfg(feature = "tls")]
impl TlsConfigBuilder {
    /// 从文件加载服务器配置
    ///
    /// # Arguments
    /// * `cert_path` - 证书文件路径 (PEM 格式)
    /// * `key_path` - 私钥文件路径 (PEM 格式)
    /// * `ca_path` - CA 证书文件路径 (可选,用于客户端证书验证)
    /// * `require_client_cert` - 是否要求客户端证书
    pub fn build_server_config(
        cert_path: &Path,
        key_path: &Path,
        ca_path: Option<&Path>,
        require_client_cert: bool,
    ) -> ServerResult<Arc<ServerConfig>> {
        info!("Loading TLS configuration...");

        // 加载服务器证书
        let certs = Self::load_certs(cert_path)?;
        debug!("Loaded {} certificate(s) from {}", certs.len(), cert_path.display());

        // 加载私钥
        let key = Self::load_private_key(key_path)?;
        debug!("Loaded private key from {}", key_path.display());

        // 创建 TLS 配置
        let mut config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| ServerError::Tls(format!("Failed to create TLS config: {}", e)))?;

        // 如果需要客户端证书验证
        if require_client_cert {
            if let Some(ca_path) = ca_path {
                info!("Enabling client certificate verification");
                let client_auth = Self::build_client_verifier(ca_path)?;

                config = ServerConfig::builder()
                    .with_client_cert_verifier(client_auth)
                    .with_single_cert(Self::load_certs(cert_path)?, Self::load_private_key(key_path)?)
                    .map_err(|e| ServerError::Tls(format!("Failed to create TLS config with client auth: {}", e)))?;
            } else {
                return Err(ServerError::Config(
                    "Client certificate verification requested but no CA file provided".to_string(),
                ));
            }
        }

        // 配置协议版本 (支持 TLS 1.2 和 1.3)
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        info!("TLS configuration loaded successfully");
        Ok(Arc::new(config))
    }

    /// 从 PEM 文件加载证书链
    fn load_certs(path: &Path) -> ServerResult<Vec<CertificateDer<'static>>> {
        let file = File::open(path)
            .map_err(|e| ServerError::Tls(format!("Failed to open certificate file: {}", e)))?;
        let mut reader = BufReader::new(file);

        certs(&mut reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ServerError::Tls(format!("Failed to parse certificates: {}", e)))
    }

    /// 从 PEM 文件加载私钥
    fn load_private_key(path: &Path) -> ServerResult<PrivateKeyDer<'static>> {
        let file = File::open(path)
            .map_err(|e| ServerError::Tls(format!("Failed to open private key file: {}", e)))?;
        let mut reader = BufReader::new(file);

        private_key(&mut reader)
            .map_err(|e| ServerError::Tls(format!("Failed to parse private key: {}", e)))?
            .ok_or_else(|| ServerError::Tls("No private key found in file".to_string()))
    }

    /// 构建客户端证书验证器
    fn build_client_verifier(ca_path: &Path) -> ServerResult<Arc<dyn rustls::server::ClientCertVerifier>> {
        let mut root_store = RootCertStore::empty();

        // 加载 CA 证书
        let ca_file = File::open(ca_path)
            .map_err(|e| ServerError::Tls(format!("Failed to open CA file: {}", e)))?;
        let mut ca_reader = BufReader::new(ca_file);

        let ca_certs = certs(&mut ca_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ServerError::Tls(format!("Failed to parse CA certificates: {}", e)))?;

        for cert in ca_certs {
            root_store.add(cert)
                .map_err(|e| ServerError::Tls(format!("Failed to add CA certificate: {}", e)))?;
        }

        debug!("Loaded {} CA certificate(s)", root_store.len());

        WebPkiClientVerifier::builder(Arc::new(root_store))
            .build()
            .map_err(|e| ServerError::Tls(format!("Failed to build client verifier: {}", e)))
    }
}

/// 生成自签名证书(用于测试)
#[cfg(all(feature = "tls", test))]
pub mod test_utils {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// 生成测试用的自签名证书和私钥
    pub fn generate_test_cert() -> ServerResult<(NamedTempFile, NamedTempFile)> {
        // 这里使用简单的测试证书
        // 实际生产环境应使用 Let's Encrypt 或其他 CA 签发的证书

        let cert_pem = r#"-----BEGIN CERTIFICATE-----
MIIDazCCAlOgAwIBAgIUX9QrxFqxMX7BhDqDk5dYnXXhF0AwDQYJKoZIhvcNAQEL
BQAwRTELMAkGA1UEBhMCQ04xEzARBgNVBAgMClNvbWUtU3RhdGUxITAfBgNVBAoM
GE1pa3VEQiBUZXN0IENlcnRpZmljYXRlMB4XDTI0MDEwMTAwMDAwMFoXDTI1MDEw
MTAwMDAwMFowRTELMAkGA1UEBhMCQ04xEzARBgNVBAgMClNvbWUtU3RhdGUxITAf
BgNVBAoMGE1pa3VEQiBUZXN0IENlcnRpZmljYXRlMIIBIjANBgkqhkiG9w0BAQEF
AAOCAQ8AMIIBCgKCAQEAq8Zq8xZJxWxl5tK3UqHgF2cW8dQ3k/7YQJQvP3mK5rLQ
...
-----END CERTIFICATE-----"#;

        let key_pem = r#"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCrxmrzFknFbGXm
...
-----END PRIVATE KEY-----"#;

        let mut cert_file = NamedTempFile::new()
            .map_err(|e| ServerError::Tls(format!("Failed to create temp cert file: {}", e)))?;
        cert_file.write_all(cert_pem.as_bytes())
            .map_err(|e| ServerError::Tls(format!("Failed to write cert file: {}", e)))?;

        let mut key_file = NamedTempFile::new()
            .map_err(|e| ServerError::Tls(format!("Failed to create temp key file: {}", e)))?;
        key_file.write_all(key_pem.as_bytes())
            .map_err(|e| ServerError::Tls(format!("Failed to write key file: {}", e)))?;

        Ok((cert_file, key_file))
    }
}

#[cfg(all(feature = "tls", test))]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_builder() {
        // 测试 TLS 配置构建
        // TODO: 实现完整测试
    }
}
