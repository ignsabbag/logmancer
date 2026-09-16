use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::runtime_parameter::RuntimeParameterSource;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteRootSource {
    Environment,
    LauncherInstalledResource,
    DesktopResource,
    Development,
    Installed,
    Fallback,
}

impl fmt::Display for SiteRootSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source = match self {
            Self::Environment => "environment",
            Self::LauncherInstalledResource => "installed_resource",
            Self::DesktopResource => "desktop_resource",
            Self::Development => "development",
            Self::Installed => "installed",
            Self::Fallback => "fallback",
        };

        formatter.write_str(source)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedSiteRoot {
    pub path: PathBuf,
    pub source: SiteRootSource,
}

pub fn is_valid_site_root(site_root: &Path) -> bool {
    let package_directory = site_root.join("pkg");
    package_directory.join("logmancer-web.css").is_file()
        && package_directory.join("logmancer-web.js").is_file()
        && package_directory.join("logmancer-web.wasm").is_file()
}

pub fn resolve_site_root(
    configured_site_root: Option<OsString>,
    desktop_site_root: Option<PathBuf>,
    executable: Option<&Path>,
    fallback_site_root: PathBuf,
) -> ResolvedSiteRoot {
    resolve_site_root_with_parameter_source(
        configured_site_root,
        RuntimeParameterSource::Environment,
        desktop_site_root,
        executable,
        fallback_site_root,
    )
}

pub fn resolve_site_root_with_parameter_source(
    configured_site_root: Option<OsString>,
    configured_site_root_source: RuntimeParameterSource,
    desktop_site_root: Option<PathBuf>,
    executable: Option<&Path>,
    fallback_site_root: PathBuf,
) -> ResolvedSiteRoot {
    if let Some(site_root) = configured_site_root.filter(|path| !path.is_empty()) {
        return ResolvedSiteRoot {
            path: PathBuf::from(site_root),
            source: match configured_site_root_source {
                RuntimeParameterSource::InstalledResource => {
                    SiteRootSource::LauncherInstalledResource
                }
                RuntimeParameterSource::Environment | RuntimeParameterSource::Default => {
                    SiteRootSource::Environment
                }
            },
        };
    }

    if let Some(site_root) = desktop_site_root {
        return ResolvedSiteRoot {
            path: site_root,
            source: SiteRootSource::DesktopResource,
        };
    }

    if fallback_site_root.is_dir() {
        return ResolvedSiteRoot {
            path: fallback_site_root,
            source: SiteRootSource::Development,
        };
    }

    if let Some(site_root) = executable
        .and_then(Path::parent)
        .map(|directory| directory.join("site"))
        .filter(|site_root| is_valid_site_root(site_root))
    {
        return ResolvedSiteRoot {
            path: site_root,
            source: SiteRootSource::Installed,
        };
    }

    ResolvedSiteRoot {
        path: fallback_site_root,
        source: SiteRootSource::Fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_site_root, SiteRootSource};
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    #[test]
    fn explicit_site_root_overrides_desktop_resource() {
        let site_root = resolve_site_root(
            Some(OsString::from("/configured/site")),
            Some(PathBuf::from("/desktop/resource/site")),
            Some(Path::new("/installed/bin/logmancer-web")),
            PathBuf::from("target/site"),
        );

        assert_eq!(site_root.path, PathBuf::from("/configured/site"));
        assert_eq!(site_root.source, SiteRootSource::Environment);
    }

    #[test]
    fn desktop_resource_overrides_standalone_layout() {
        let site_root = resolve_site_root(
            None,
            Some(PathBuf::from("/desktop/resource/site")),
            Some(Path::new("/installed/bin/logmancer-web")),
            PathBuf::from("target/site"),
        );

        assert_eq!(site_root.path, PathBuf::from("/desktop/resource/site"));
        assert_eq!(site_root.source, SiteRootSource::DesktopResource);
    }

    #[test]
    fn existing_development_site_root_overrides_executable_adjacent_site() {
        let distribution = tempfile::tempdir().unwrap();
        let executable = distribution.path().join("logmancer-web");
        let installed_site_root = distribution.path().join("site");
        std::fs::create_dir(&installed_site_root).unwrap();
        std::fs::write(installed_site_root.join("index.html"), "").unwrap();
        std::fs::create_dir(installed_site_root.join("pkg")).unwrap();
        let development_site_root = distribution.path().join("target/site");
        std::fs::create_dir_all(&development_site_root).unwrap();

        let resolved =
            resolve_site_root(None, None, Some(&executable), development_site_root.clone());

        assert_eq!(resolved.path, development_site_root);
        assert_eq!(resolved.source, SiteRootSource::Development);
    }

    #[test]
    fn installed_standalone_layout_accepts_ssr_artifacts_without_index_html() {
        let distribution = tempfile::tempdir().unwrap();
        let executable = distribution.path().join("logmancer-web");
        let installed_site_root = distribution.path().join("site");
        std::fs::create_dir(&installed_site_root).unwrap();
        let fallback_site_root = distribution.path().join("target/site");

        let resolved = resolve_site_root(None, None, Some(&executable), fallback_site_root.clone());

        assert_eq!(resolved.path, fallback_site_root);
        assert_eq!(resolved.source, SiteRootSource::Fallback);

        let package_directory = installed_site_root.join("pkg");
        std::fs::create_dir(&package_directory).unwrap();
        std::fs::write(package_directory.join("logmancer-web.css"), "").unwrap();
        std::fs::write(package_directory.join("logmancer-web.js"), "").unwrap();
        std::fs::write(package_directory.join("logmancer-web.wasm"), "").unwrap();

        let resolved = resolve_site_root(None, None, Some(&executable), fallback_site_root);

        assert_eq!(resolved.path, installed_site_root);
        assert_eq!(resolved.source, SiteRootSource::Installed);
    }

    #[test]
    fn installed_standalone_layout_rejects_incomplete_ssr_artifacts() {
        let distribution = tempfile::tempdir().unwrap();
        let executable = distribution.path().join("logmancer-web");
        let installed_site_root = distribution.path().join("site");
        let package_directory = installed_site_root.join("pkg");
        std::fs::create_dir_all(&package_directory).unwrap();
        std::fs::write(package_directory.join("logmancer-web.css"), "").unwrap();
        std::fs::write(package_directory.join("logmancer-web.js"), "").unwrap();
        let fallback_site_root = distribution.path().join("target/site");

        let resolved = resolve_site_root(None, None, Some(&executable), fallback_site_root.clone());

        assert_eq!(resolved.path, fallback_site_root);
        assert_eq!(resolved.source, SiteRootSource::Fallback);
    }

    #[test]
    fn development_fallback_is_used_without_an_installed_site() {
        let resolved = resolve_site_root(
            None,
            None,
            Some(Path::new("/build/target/debug/logmancer-web")),
            PathBuf::from("target/site"),
        );

        assert_eq!(resolved.path, PathBuf::from("target/site"));
        assert_eq!(resolved.source, SiteRootSource::Fallback);
    }
}
