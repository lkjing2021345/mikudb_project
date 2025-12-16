//! 连接管理模块
//!
//! 提供数据库连接的管理，包括连接池、连接字符串解析和网络配置。

use crate::common::{MikuError, MikuResult};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionMode {
    Embedded,
    Standalone,
    ReplicaSet,
    Sharded,
}

impl Default for ConnectionMode {
    fn default() -> Self {
        Self::Embedded
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionString {
    pub scheme: String,
    pub hosts: Vec<Host>,
    pub database: Option<String>,
    pub options: ConnectionOptions,
    pub credentials: Option<Credentials>,
}

#[derive(Debug, Clone)]
pub struct Host {
    pub address: String,
    pub port: u16,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            address: "localhost".to_string(),
            port: crate::DEFAULT_PORT,
        }
    }
}

impl Host {
    pub fn new(address: impl Into<String>, port: u16) -> Self {
        Self {
            address: address.into(),
            port,
        }
    }

    pub fn localhost() -> Self {
        Self::default()
    }

    pub fn to_socket_addr(&self) -> MikuResult<SocketAddr> {
        let addr_str = format!("{}:{}", self.address, self.port);
        addr_str
            .parse()
            .map_err(|e| MikuError::Connection(format!("Invalid address: {}", e)))
    }
}

impl std::fmt::Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.address, self.port)
    }
}

#[derive(Debug, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: Option<String>,
    pub auth_source: Option<String>,
    pub auth_mechanism: AuthMechanism,
}

impl Default for Credentials {
    fn default() -> Self {
        Self {
            username: crate::DEFAULT_USER.to_string(),
            password: Some(crate::DEFAULT_PASSWORD.to_string()),
            auth_source: None,
            auth_mechanism: AuthMechanism::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMechanism {
    None,
    ScramSha256,
    Plain,
}

impl Default for AuthMechanism {
    fn default() -> Self {
        Self::ScramSha256
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionOptions {
    pub connect_timeout: Duration,
    pub socket_timeout: Duration,
    pub server_selection_timeout: Duration,
    pub heartbeat_frequency: Duration,
    pub max_pool_size: u32,
    pub min_pool_size: u32,
    pub max_idle_time: Duration,
    pub wait_queue_timeout: Duration,
    pub retry_writes: bool,
    pub retry_reads: bool,
    pub direct_connection: bool,
    pub tls: Option<TlsOptions>,
    pub compressors: Vec<Compressor>,
    pub app_name: Option<String>,
    pub read_preference: ReadPreference,
    pub write_concern: WriteConcern,
    pub read_concern: ReadConcern,
}

impl Default for ConnectionOptions {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            socket_timeout: Duration::from_secs(0),
            server_selection_timeout: Duration::from_secs(30),
            heartbeat_frequency: Duration::from_secs(10),
            max_pool_size: 100,
            min_pool_size: 0,
            max_idle_time: Duration::from_secs(300),
            wait_queue_timeout: Duration::from_secs(120),
            retry_writes: true,
            retry_reads: true,
            direct_connection: false,
            tls: None,
            compressors: vec![],
            app_name: None,
            read_preference: ReadPreference::default(),
            write_concern: WriteConcern::default(),
            read_concern: ReadConcern::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TlsOptions {
    pub enabled: bool,
    pub ca_file: Option<PathBuf>,
    pub cert_file: Option<PathBuf>,
    pub key_file: Option<PathBuf>,
    pub allow_invalid_certificates: bool,
    pub allow_invalid_hostnames: bool,
}

impl Default for TlsOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            ca_file: None,
            cert_file: None,
            key_file: None,
            allow_invalid_certificates: false,
            allow_invalid_hostnames: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compressor {
    Snappy,
    Zlib,
    Zstd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadPreference {
    Primary,
    PrimaryPreferred,
    Secondary,
    SecondaryPreferred,
    Nearest,
}

impl Default for ReadPreference {
    fn default() -> Self {
        Self::Primary
    }
}

#[derive(Debug, Clone)]
pub struct WriteConcern {
    pub w: WriteConcernLevel,
    pub j: Option<bool>,
    pub wtimeout: Option<Duration>,
}

impl Default for WriteConcern {
    fn default() -> Self {
        Self {
            w: WriteConcernLevel::Majority,
            j: None,
            wtimeout: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteConcernLevel {
    W(u32),
    Majority,
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadConcern {
    Local,
    Available,
    Majority,
    Linearizable,
    Snapshot,
}

impl Default for ReadConcern {
    fn default() -> Self {
        Self::Local
    }
}

impl ConnectionString {
    pub fn parse(uri: &str) -> MikuResult<Self> {
        if uri.starts_with("mikudb://") || uri.starts_with("miku://") {
            Self::parse_uri(uri)
        } else {
            Ok(Self {
                scheme: "file".to_string(),
                hosts: vec![],
                database: Some(uri.to_string()),
                options: ConnectionOptions::default(),
                credentials: None,
            })
        }
    }

    fn parse_uri(uri: &str) -> MikuResult<Self> {
        let scheme_end = uri
            .find("://")
            .ok_or_else(|| MikuError::Connection("Invalid URI scheme".to_string()))?;
        let scheme = uri[..scheme_end].to_string();
        let rest = &uri[scheme_end + 3..];

        let (auth_part, host_part) = if let Some(at_pos) = rest.find('@') {
            (Some(&rest[..at_pos]), &rest[at_pos + 1..])
        } else {
            (None, rest)
        };

        let credentials = if let Some(auth) = auth_part {
            let (username, password) = if let Some(colon_pos) = auth.find(':') {
                (
                    urlencoding_decode(&auth[..colon_pos])?,
                    Some(urlencoding_decode(&auth[colon_pos + 1..])?),
                )
            } else {
                (urlencoding_decode(auth)?, None)
            };
            Some(Credentials {
                username,
                password,
                auth_source: None,
                auth_mechanism: AuthMechanism::default(),
            })
        } else {
            None
        };

        let (hosts_str, db_and_options) = if let Some(slash_pos) = host_part.find('/') {
            (&host_part[..slash_pos], Some(&host_part[slash_pos + 1..]))
        } else {
            (host_part, None)
        };

        let hosts: Vec<Host> = hosts_str
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|h| {
                if let Some(colon_pos) = h.rfind(':') {
                    let addr = &h[..colon_pos];
                    let port = h[colon_pos + 1..]
                        .parse()
                        .unwrap_or(crate::DEFAULT_PORT);
                    Host::new(addr, port)
                } else {
                    Host::new(h, crate::DEFAULT_PORT)
                }
            })
            .collect();

        let hosts = if hosts.is_empty() {
            vec![Host::localhost()]
        } else {
            hosts
        };

        let (database, options) = if let Some(db_opts) = db_and_options {
            if let Some(q_pos) = db_opts.find('?') {
                let db = if q_pos > 0 {
                    Some(db_opts[..q_pos].to_string())
                } else {
                    None
                };
                let opts = Self::parse_options(&db_opts[q_pos + 1..])?;
                (db, opts)
            } else if !db_opts.is_empty() {
                (Some(db_opts.to_string()), ConnectionOptions::default())
            } else {
                (None, ConnectionOptions::default())
            }
        } else {
            (None, ConnectionOptions::default())
        };

        Ok(Self {
            scheme,
            hosts,
            database,
            options,
            credentials,
        })
    }

    fn parse_options(query: &str) -> MikuResult<ConnectionOptions> {
        let mut options = ConnectionOptions::default();

        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                match key {
                    "maxPoolSize" => {
                        if let Ok(v) = value.parse() {
                            options.max_pool_size = v;
                        }
                    }
                    "minPoolSize" => {
                        if let Ok(v) = value.parse() {
                            options.min_pool_size = v;
                        }
                    }
                    "connectTimeoutMS" => {
                        if let Ok(v) = value.parse::<u64>() {
                            options.connect_timeout = Duration::from_millis(v);
                        }
                    }
                    "socketTimeoutMS" => {
                        if let Ok(v) = value.parse::<u64>() {
                            options.socket_timeout = Duration::from_millis(v);
                        }
                    }
                    "serverSelectionTimeoutMS" => {
                        if let Ok(v) = value.parse::<u64>() {
                            options.server_selection_timeout = Duration::from_millis(v);
                        }
                    }
                    "heartbeatFrequencyMS" => {
                        if let Ok(v) = value.parse::<u64>() {
                            options.heartbeat_frequency = Duration::from_millis(v);
                        }
                    }
                    "maxIdleTimeMS" => {
                        if let Ok(v) = value.parse::<u64>() {
                            options.max_idle_time = Duration::from_millis(v);
                        }
                    }
                    "waitQueueTimeoutMS" => {
                        if let Ok(v) = value.parse::<u64>() {
                            options.wait_queue_timeout = Duration::from_millis(v);
                        }
                    }
                    "retryWrites" => {
                        options.retry_writes = value == "true";
                    }
                    "retryReads" => {
                        options.retry_reads = value == "true";
                    }
                    "directConnection" => {
                        options.direct_connection = value == "true";
                    }
                    "tls" | "ssl" => {
                        if value == "true" {
                            options.tls = Some(TlsOptions {
                                enabled: true,
                                ..Default::default()
                            });
                        }
                    }
                    "appName" => {
                        options.app_name = Some(urlencoding_decode(value)?);
                    }
                    "readPreference" => {
                        options.read_preference = match value {
                            "primary" => ReadPreference::Primary,
                            "primaryPreferred" => ReadPreference::PrimaryPreferred,
                            "secondary" => ReadPreference::Secondary,
                            "secondaryPreferred" => ReadPreference::SecondaryPreferred,
                            "nearest" => ReadPreference::Nearest,
                            _ => ReadPreference::Primary,
                        };
                    }
                    "w" => {
                        options.write_concern.w = if value == "majority" {
                            WriteConcernLevel::Majority
                        } else if let Ok(n) = value.parse() {
                            WriteConcernLevel::W(n)
                        } else {
                            WriteConcernLevel::Custom(value.to_string())
                        };
                    }
                    "journal" | "j" => {
                        options.write_concern.j = Some(value == "true");
                    }
                    "wtimeoutMS" => {
                        if let Ok(v) = value.parse::<u64>() {
                            options.write_concern.wtimeout = Some(Duration::from_millis(v));
                        }
                    }
                    "readConcernLevel" => {
                        options.read_concern = match value {
                            "local" => ReadConcern::Local,
                            "available" => ReadConcern::Available,
                            "majority" => ReadConcern::Majority,
                            "linearizable" => ReadConcern::Linearizable,
                            "snapshot" => ReadConcern::Snapshot,
                            _ => ReadConcern::Local,
                        };
                    }
                    "compressors" => {
                        options.compressors = value
                            .split(',')
                            .filter_map(|c| match c.trim() {
                                "snappy" => Some(Compressor::Snappy),
                                "zlib" => Some(Compressor::Zlib),
                                "zstd" => Some(Compressor::Zstd),
                                _ => None,
                            })
                            .collect();
                    }
                    _ => {}
                }
            }
        }

        Ok(options)
    }

    pub fn to_uri(&self) -> String {
        let mut uri = format!("{}://", self.scheme);

        if let Some(ref creds) = self.credentials {
            uri.push_str(&urlencoding_encode(&creds.username));
            if let Some(ref pwd) = creds.password {
                uri.push(':');
                uri.push_str(&urlencoding_encode(pwd));
            }
            uri.push('@');
        }

        let hosts_str: Vec<String> = self.hosts.iter().map(|h| h.to_string()).collect();
        uri.push_str(&hosts_str.join(","));

        if let Some(ref db) = self.database {
            uri.push('/');
            uri.push_str(db);
        }

        uri
    }
}

fn urlencoding_decode(s: &str) -> MikuResult<String> {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            return Err(MikuError::Connection("Invalid URL encoding".to_string()));
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

fn urlencoding_encode(s: &str) -> String {
    let mut result = String::new();

    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                result.push(c);
            }
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }

    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Authenticating,
    Ready,
    Closing,
    Closed,
    Error,
}

pub struct ConnectionInfo {
    pub id: u64,
    pub host: Host,
    pub state: ConnectionState,
    pub created_at: std::time::Instant,
    pub last_used_at: std::time::Instant,
    pub operations_count: u64,
}

impl ConnectionInfo {
    pub fn new(id: u64, host: Host) -> Self {
        let now = std::time::Instant::now();
        Self {
            id,
            host,
            state: ConnectionState::Disconnected,
            created_at: now,
            last_used_at: now,
            operations_count: 0,
        }
    }

    pub fn is_idle(&self, max_idle_time: Duration) -> bool {
        self.last_used_at.elapsed() > max_idle_time
    }

    pub fn touch(&mut self) {
        self.last_used_at = std::time::Instant::now();
        self.operations_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_uri() {
        let conn = ConnectionString::parse("mikudb://localhost:3939/mydb").unwrap();
        assert_eq!(conn.scheme, "mikudb");
        assert_eq!(conn.hosts.len(), 1);
        assert_eq!(conn.hosts[0].address, "localhost");
        assert_eq!(conn.hosts[0].port, 3939);
        assert_eq!(conn.database, Some("mydb".to_string()));
    }

    #[test]
    fn test_parse_uri_with_auth() {
        let conn = ConnectionString::parse("mikudb://miku:pass@localhost:3939/mydb").unwrap();
        assert!(conn.credentials.is_some());
        let creds = conn.credentials.unwrap();
        assert_eq!(creds.username, "miku");
        assert_eq!(creds.password, Some("pass".to_string()));
    }

    #[test]
    fn test_parse_uri_with_options() {
        let conn = ConnectionString::parse(
            "mikudb://localhost/mydb?maxPoolSize=50&retryWrites=true",
        )
        .unwrap();
        assert_eq!(conn.options.max_pool_size, 50);
        assert!(conn.options.retry_writes);
    }

    #[test]
    fn test_parse_multiple_hosts() {
        let conn = ConnectionString::parse(
            "mikudb://host1:3939,host2:3940,host3:3941/mydb",
        )
        .unwrap();
        assert_eq!(conn.hosts.len(), 3);
        assert_eq!(conn.hosts[0].address, "host1");
        assert_eq!(conn.hosts[1].port, 3940);
        assert_eq!(conn.hosts[2].address, "host3");
    }

    #[test]
    fn test_parse_file_path() {
        let conn = ConnectionString::parse("/var/lib/mikudb/data").unwrap();
        assert_eq!(conn.scheme, "file");
        assert_eq!(conn.database, Some("/var/lib/mikudb/data".to_string()));
    }

    #[test]
    fn test_host_to_string() {
        let host = Host::new("localhost", 3939);
        assert_eq!(host.to_string(), "localhost:3939");
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding_decode("hello%20world").unwrap(), "hello world");
        assert_eq!(urlencoding_encode("hello world"), "hello%20world");
    }
}
