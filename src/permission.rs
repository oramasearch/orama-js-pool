use deno_fetch::FetchPermissions;
use deno_io::fs::FsError;
use deno_net::NetPermissions;
use deno_permissions::PermissionDeniedError;
use deno_web::TimersPermission;
use globset::Glob;
use tracing::trace;

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

/// Check if a domain matches any pattern in the list using glob patterns.
/// Supports wildcard patterns using '*' for IP addresses and domains.
/// Examples:
/// - "10.0.0.*" matches "10.0.0.1", "10.0.0.255", etc.
/// - "192.168.*.*" matches "192.168.1.1", "192.168.0.255", etc.
/// - "*.example.com" matches "api.example.com", "www.example.com", etc.
fn matches_pattern(domain: &str, pattern: &str) -> bool {
    // If no wildcard, do exact match
    if !pattern.contains('*') {
        return domain == pattern;
    }

    // Use globset for pattern matching
    // Note: globset uses Unix-style glob patterns
    match Glob::new(pattern) {
        Ok(glob) => {
            let matcher = glob.compile_matcher();
            matcher.is_match(domain)
        }
        Err(_) => domain == pattern,
    }
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

        trace!(
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
