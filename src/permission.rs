use deno_fetch::FetchPermissions;
use deno_io::fs::FsError;
use deno_net::NetPermissions;
use deno_permissions::PermissionDeniedError;
use deno_web::TimersPermission;

pub const DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING: &str = "Domain not allowed";

/// Domain permission policy for network access
#[derive(Debug, Clone)]
pub enum DomainPermission {
    /// Deny all network access
    DenyAll,
    /// Allow all network access
    AllowAll,
    /// Allow only specific domains
    Allow(Vec<String>),
    /// Deny only specific domains (allow all others)
    Deny(Vec<String>),
}

pub struct CustomPermissions {
    pub domain_permission: DomainPermission,
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
        // Extract domain (with port if present)
        let domain = match (url.domain(), url.port()) {
            (Some(domain), Some(port)) => format!("{domain}:{port}"),
            (Some(domain), None) => domain.to_string(),
            _ => {
                return Err(deno_permissions::PermissionCheckError::PermissionDenied(
                    PermissionDeniedError::Fatal {
                        access: format!(
                            "{DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING}: Invalid URL {url}"
                        ),
                    },
                ))
            }
        };

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
                if allowed_list.contains(&domain) {
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
                if denied_list.contains(&domain) {
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
