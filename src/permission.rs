use deno_fetch::FetchPermissions;
use deno_io::fs::FsError;
use deno_net::NetPermissions;
use deno_permissions::PermissionDeniedError;
use deno_web::TimersPermission;
use globset::GlobBuilder;
use tracing::info;

pub const DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING: &str = "Domain not allowed";

/// Domain permission policy for network access
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub enum DomainPermission {
    #[default]
    DenyAll,
    AllowAll,
    Allow(Vec<String>),
    Deny(Vec<String>),
}

pub struct CustomPermissions {
    pub domain_permission: DomainPermission,
}

fn glob_match(value: &str, pattern: &str) -> bool {
    if !pattern.contains(&['*', '?', '[', ']', '{', '}'][..]) {
        return value.eq_ignore_ascii_case(pattern);
    }
    // Replace '.' with '/' so literal_separator prevents '*' from crossing domain labels
    let value = value.replace('.', "/");
    let pattern = pattern.replace('.', "/");
    match GlobBuilder::new(&pattern)
        .literal_separator(true)
        .case_insensitive(true)
        .build()
    {
        Ok(glob) => glob.compile_matcher().is_match(value.as_str()),
        Err(_) => value.eq_ignore_ascii_case(&pattern),
    }
}

/// Check if a domain matches a pattern using glob patterns following RFC 6125.
/// Examples:
/// - "localhost" matches "localhost:3000"
/// - "10.0.0.*" matches "10.0.0.1", "10.0.0.255", etc.
/// - "*.example.com" matches "api.example.com", "www.example.com", etc.
fn matches_pattern(domain: &str, pattern: &str) -> bool {
    if glob_match(domain, pattern) {
        return true;
    }
    // Strip :port suffix and retry matching against host only
    if let Some((host, port)) = domain.rsplit_once(':') {
        if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            return glob_match(host, pattern);
        }
    }
    false
}

/// Check if domain matches any pattern in the list
fn matches_pattern_list(domain: &str, patterns: &[String]) -> bool {
    patterns
        .iter()
        .any(|pattern| matches_pattern(domain, pattern))
}

impl TimersPermission for CustomPermissions {
    fn allow_hrtime(&mut self) -> bool {
        // No high resolution time
        false
    }
}

// BLocks all TCP/UDP connections
// Not invoked for http / https connections
impl NetPermissions for CustomPermissions {
    fn check_net<T: AsRef<str>>(
        &mut self,
        _host: &(T, Option<u16>),
        _api_name: &str,
    ) -> Result<(), deno_permissions::PermissionCheckError> {
        Err(deno_permissions::PermissionCheckError::PermissionDenied(
            PermissionDeniedError::Fatal {
                access: "No check_net".to_string(),
            },
        ))
    }

    fn check_read(
        &mut self,
        _p: &str,
        _api_name: &str,
    ) -> Result<std::path::PathBuf, deno_permissions::PermissionCheckError> {
        Err(deno_permissions::PermissionCheckError::PermissionDenied(
            PermissionDeniedError::Fatal {
                access: "No check_read".to_string(),
            },
        ))
    }

    fn check_write(
        &mut self,
        _p: &str,
        _api_name: &str,
    ) -> Result<std::path::PathBuf, deno_permissions::PermissionCheckError> {
        Err(deno_permissions::PermissionCheckError::PermissionDenied(
            PermissionDeniedError::Fatal {
                access: "No check_write".to_string(),
            },
        ))
    }

    fn check_write_path<'a>(
        &mut self,
        _p: &'a std::path::Path,
        _api_name: &str,
    ) -> Result<std::borrow::Cow<'a, std::path::Path>, deno_permissions::PermissionCheckError> {
        Err(deno_permissions::PermissionCheckError::PermissionDenied(
            PermissionDeniedError::Fatal {
                access: "No check_write_path".to_string(),
            },
        ))
    }
}

impl FetchPermissions for CustomPermissions {
    fn check_net_url(
        &mut self,
        url: &deno_core::url::Url,
        _api_name: &str,
    ) -> Result<(), deno_permissions::PermissionCheckError> {
        // Use host_str() instead of domain() to support both domains and IP addresses
        let host = url.host_str().ok_or_else(|| {
            deno_permissions::PermissionCheckError::PermissionDenied(PermissionDeniedError::Fatal {
                access: format!("{DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING}: Invalid URL {url}"),
            })
        })?;

        // Strip brackets from IPv6 addresses (e.g., [2001:db8::1] -> 2001:db8::1)
        let host = host
            .strip_prefix('[')
            .and_then(|h| h.strip_suffix(']'))
            .unwrap_or(host);

        let domain = match url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_string(),
        };

        info!(
            domain = %domain,
            scheme = %url.scheme(),
            "Network request to domain"
        );

        match &self.domain_permission {
            DomainPermission::AllowAll => Ok(()),
            DomainPermission::DenyAll => {
                Err(deno_permissions::PermissionCheckError::PermissionDenied(
                    PermissionDeniedError::Fatal {
                        access: format!(
                            "{DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING}: {url}. All network access is denied"
                        ),
                    },
                ))
            }
            DomainPermission::Allow(allowed_list) => {
                if matches_pattern_list(&domain, allowed_list) {
                    Ok(())
                } else {
                    Err(deno_permissions::PermissionCheckError::PermissionDenied(
                        PermissionDeniedError::Fatal {
                            access: format!(
                                "{DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING}: {url}. Allowed domains: {allowed_list:?}"
                            ),
                        },
                    ))
                }
            }
            DomainPermission::Deny(denied_list) => {
                if matches_pattern_list(&domain, denied_list) {
                    Err(deno_permissions::PermissionCheckError::PermissionDenied(
                        PermissionDeniedError::Fatal {
                            access: format!(
                                "{DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING}: {url}. Domain is in deny list: {denied_list:?}"
                            ),
                        },
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }

    // Used for file:// URLs
    fn check_read<'a>(
        &mut self,
        _resolved: bool,
        _p: &'a std::path::Path,
        _api_name: &str,
    ) -> Result<std::borrow::Cow<'a, std::path::Path>, FsError> {
        Err(FsError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_url(url: &str) -> deno_core::url::Url {
        deno_core::url::Url::parse(url).unwrap()
    }

    #[test]
    fn test_pattern_matching() {
        assert!(matches_pattern("example.com", "example.com"));
        assert!(matches_pattern("224.0.0.1", "22*.*.*.*"));
        assert!(!matches_pattern("example.com", "other.com"));

        assert!(matches_pattern("10.0.0.1", "10.0.0.*"));
        assert!(matches_pattern("192.168.1.1", "192.168.*.*"));
        assert!(!matches_pattern("10.0.1.1", "10.0.0.*"));

        assert!(matches_pattern("2001:db8::1", "2001:db8::*"));
        assert!(matches_pattern("fe80::1", "fe80:*"));
        assert!(!matches_pattern("2001:db8::1", "fe80:*"));

        assert!(matches_pattern("api.example.com", "*.example.com"));
        assert!(!matches_pattern("example.com", "*.example.com"));

        assert!(matches_pattern("10.0.0.1:8080", "10.0.0.*:8080"));
        assert!(!matches_pattern("10.0.0.1:8080", "10.0.0.*:9090"));

        assert!(matches_pattern("172.16.0.1", "172.1[6-9].*.*"));
        assert!(matches_pattern("172.17.5.10", "172.1[6-9].*.*"));
        assert!(matches_pattern("172.18.100.200", "172.1[6-9].*.*"));
        assert!(matches_pattern("172.19.255.255", "172.1[6-9].*.*"));
        assert!(!matches_pattern("172.15.0.1", "172.1[6-9].*.*"));
        assert!(!matches_pattern("172.20.0.1", "172.1[6-9].*.*"));

        assert!(matches_pattern("localhost", "localhost"));
        assert!(matches_pattern("localhost:3000", "localhost"));
        assert!(matches_pattern("example.org", "example.org"));
        assert!(matches_pattern("example.org:3000", "example.org"));

        // RFC 6125 Section 6.4.1: case-insensitive matching
        assert!(matches_pattern("Example.COM", "example.com"));
        assert!(matches_pattern("EXAMPLE.COM", "example.com"));
        assert!(matches_pattern("example.com", "EXAMPLE.COM"));
        assert!(matches_pattern("Api.Example.Com", "*.example.com"));

        // RFC 6125 Section 6.4.3: wildcard must not cross domain labels
        assert!(matches_pattern("foo.example.com", "*.example.com"));
        assert!(!matches_pattern("bar.foo.example.com", "*.example.com"));
    }

    #[test]
    fn test_pattern_list_matching() {
        let patterns = vec![
            "example.com".into(),
            "10.0.0.*".into(),
            "192.168.*.*".into(),
        ];

        assert!(matches_pattern_list("example.com", &patterns));
        assert!(matches_pattern_list("10.0.0.1", &patterns));
        assert!(matches_pattern_list("192.168.1.1", &patterns));
        assert!(!matches_pattern_list("other.com", &patterns));
        assert!(!matches_pattern_list("10.0.1.1", &patterns));
    }

    #[test]
    fn test_permission_deny_list() {
        let mut perms = CustomPermissions {
            domain_permission: DomainPermission::Deny(vec!["10.0.0.*".into(), "fe80:*".into()]),
        };

        assert!(perms
            .check_net_url(&parse_url("http://10.0.0.1/test"), "fetch")
            .is_err());
        assert!(perms
            .check_net_url(&parse_url("http://10.0.1.1/test"), "fetch")
            .is_ok());
        assert!(perms
            .check_net_url(&parse_url("http://[fe80::1]/test"), "fetch")
            .is_err());
        assert!(perms
            .check_net_url(&parse_url("http://[2001:db8::1]/test"), "fetch")
            .is_ok());
    }

    #[test]
    fn test_permission_allow_list() {
        let mut perms = CustomPermissions {
            domain_permission: DomainPermission::Allow(vec![
                "10.0.0.*".into(),
                "2001:db8::*".into(),
            ]),
        };

        assert!(perms
            .check_net_url(&parse_url("http://10.0.0.1/test"), "fetch")
            .is_ok());
        assert!(perms
            .check_net_url(&parse_url("http://10.0.1.1/test"), "fetch")
            .is_err());
        assert!(perms
            .check_net_url(&parse_url("http://[2001:db8::1]/test"), "fetch")
            .is_ok());
        assert!(perms
            .check_net_url(&parse_url("http://[fe80::1]/test"), "fetch")
            .is_err());
    }
}
