use deno_fetch::FetchPermissions;
use deno_io::fs::FsError;
use deno_net::NetPermissions;
use deno_permissions::PermissionDeniedError;
use deno_web::TimersPermission;

pub const DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING: &str = "Domain not allowed";

pub struct CustomPermissions {
    pub allowed_hosts: Option<Vec<String>>,
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
        let domain_with_port: Option<(String, &Vec<String>)> =
            match (url.domain(), url.port(), self.allowed_hosts.as_ref()) {
                (Some(domain), Some(port), Some(l)) => Some((format!("{domain}:{port}"), l)),
                (Some(domain), None, Some(l)) => Some((domain.to_string(), l)),
                (_, _, None) => return Ok(()),
                _ => None,
            };

        if let Some((domain, list)) = domain_with_port {
            let is_allowed = list.contains(&domain);
            if is_allowed {
                return Ok(());
            }
        }

        Err(deno_permissions::PermissionCheckError::PermissionDenied(
            PermissionDeniedError::Fatal {
                access: format!(
                    "{}: {}. Allowed {:?}",
                    DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING, url, self.allowed_hosts
                ),
            },
        ))
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
