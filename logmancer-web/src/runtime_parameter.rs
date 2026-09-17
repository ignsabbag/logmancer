use std::ffi::OsStr;
use std::fmt;

pub const LEPTOS_OUTPUT_NAME_SOURCE_ENV: &str = "LOGMANCER_LEPTOS_OUTPUT_NAME_SOURCE";
pub const LEPTOS_SITE_ROOT_SOURCE_ENV: &str = "LOGMANCER_LEPTOS_SITE_ROOT_SOURCE";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeParameterSource {
    Environment,
    Default,
    InstalledResource,
}

impl fmt::Display for RuntimeParameterSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Environment => "environment",
            Self::Default => "default",
            Self::InstalledResource => "installed_resource",
        })
    }
}

pub fn resolve_runtime_parameter_source(
    configured_value: Option<&OsStr>,
    launcher_source: Option<&OsStr>,
) -> RuntimeParameterSource {
    if configured_value.is_none_or(OsStr::is_empty) {
        return RuntimeParameterSource::Default;
    }

    match launcher_source.and_then(OsStr::to_str) {
        Some("default") => RuntimeParameterSource::Default,
        Some("installed_resource") => RuntimeParameterSource::InstalledResource,
        _ => RuntimeParameterSource::Environment,
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_runtime_parameter_source, RuntimeParameterSource};
    use std::ffi::OsStr;

    #[test]
    fn preserves_launcher_source_for_an_injected_value() {
        assert_eq!(
            resolve_runtime_parameter_source(
                Some(OsStr::new("logmancer-web")),
                Some(OsStr::new("default")),
            ),
            RuntimeParameterSource::Default
        );
        assert_eq!(
            resolve_runtime_parameter_source(
                Some(OsStr::new("/installed/site")),
                Some(OsStr::new("installed_resource")),
            ),
            RuntimeParameterSource::InstalledResource
        );
    }

    #[test]
    fn classifies_unmarked_configured_values_as_environment() {
        assert_eq!(
            resolve_runtime_parameter_source(Some(OsStr::new("custom-output")), None),
            RuntimeParameterSource::Environment
        );
    }
}
