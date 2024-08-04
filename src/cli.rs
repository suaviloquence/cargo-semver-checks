use std::path::PathBuf;

// #[cfg(test)]
// pub(self) use crate::{Check, PackageSelection, ReleaseType, Rustdoc, ScopeSelection};

// #[cfg(not(test))]
pub(self) use cargo_semver_checks::{
    Check, PackageSelection, ReleaseType, Rustdoc, ScopeSelection,
};

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
#[command(version, propagate_version = true)]
pub(crate) enum Cargo {
    SemverChecks(SemverChecks),
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct SemverChecks {
    #[arg(long, global = true, exclusive = true)]
    pub bugreport: bool,

    #[arg(long, global = true, exclusive = true)]
    pub explain: Option<String>,

    #[arg(long, global = true, exclusive = true)]
    pub list: bool,

    #[clap(flatten)]
    pub check_release: CheckRelease,

    #[command(subcommand)]
    pub command: Option<SemverChecksCommands>,

    // we need to use clap::ColorChoice instead of anstream::ColorChoice
    // because ValueEnum is implemented for it.
    /// Choose whether to output colors
    #[arg(long = "color", global = true, value_name = "WHEN", value_enum)]
    pub color_choice: Option<clap::ColorChoice>,
}

/// Check your crate for semver violations.
#[derive(Debug, Subcommand)]
pub(crate) enum SemverChecksCommands {
    #[command(alias = "diff-files")]
    CheckRelease(CheckRelease),
}

#[derive(Debug, Args, Clone)]
#[non_exhaustive]
pub(crate) struct CheckRelease {
    #[command(flatten, next_help_heading = "Current")]
    pub(crate) manifest: clap_cargo::Manifest,

    #[command(flatten, next_help_heading = "Current")]
    pub(crate) workspace: clap_cargo::Workspace,

    /// The current rustdoc json output to test for semver violations.
    #[arg(
        long,
        short_alias = 'c',
        alias = "current",
        value_name = "JSON_PATH",
        help_heading = "Current",
        requires = "baseline_rustdoc",
        conflicts_with_all = [
            "default_features",
            "only_explicit_features",
            "features",
            "baseline_features",
            "current_features",
            "all_features",
        ]
    )]
    pub current_rustdoc: Option<PathBuf>,

    /// Version from registry to lookup for a baseline
    #[arg(
        long,
        value_name = "X.Y.Z",
        help_heading = "Baseline",
        group = "baseline"
    )]
    pub(crate) baseline_version: Option<String>,

    /// Git revision to lookup for a baseline
    #[arg(
        long,
        value_name = "REV",
        help_heading = "Baseline",
        group = "baseline"
    )]
    pub(crate) baseline_rev: Option<String>,

    /// Directory containing baseline crate source
    #[arg(
        long,
        value_name = "MANIFEST_ROOT",
        help_heading = "Baseline",
        group = "baseline"
    )]
    pub(crate) baseline_root: Option<PathBuf>,

    /// The rustdoc json file to use as a semver baseline.
    #[arg(
        long,
        short_alias = 'b',
        alias = "baseline",
        value_name = "JSON_PATH",
        help_heading = "Baseline",
        group = "baseline",
        conflicts_with_all = [
            "default_features",
            "only_explicit_features",
            "features",
            "baseline_features",
            "current_features",
            "all_features",
        ]
    )]
    pub(crate) baseline_rustdoc: Option<PathBuf>,

    /// Sets the release type instead of deriving it from the version number.
    #[arg(
        value_enum,
        long,
        value_name = "TYPE",
        help_heading = "Overrides",
        group = "overrides"
    )]
    pub(crate) release_type: Option<ReleaseType>,

    /// Use only the crate-defined default features, as well as any features
    /// added explicitly via other flags.
    ///
    /// Using this flag disables the heuristic that enables all features
    /// except `unstable`, `nightly`, `bench`, `no_std`, and ones starting with prefixes
    /// `_`, `unstable_`, `unstable-`.
    #[arg(
        long,
        help_heading = "Features",
        conflicts_with = "only_explicit_features"
    )]
    pub(crate) default_features: bool,

    /// Use no features except ones explicitly added by other flags.
    ///
    /// Using this flag disables the heuristic that enables all features
    /// except `unstable`, `nightly`, `bench`, `no_std`, and ones starting with prefixes
    /// `_`, `unstable_`, `unstable-`.
    #[arg(long, help_heading = "Features")]
    pub(crate) only_explicit_features: bool,

    /// Add a feature to the set of features being checked.
    /// The feature will be used in both the baseline and the current version
    /// of the crate.
    #[arg(long, value_name = "NAME", help_heading = "Features")]
    pub(crate) features: Vec<String>,

    /// Add a feature to the set of features being checked.
    /// The feature will be used in the baseline version of the crate only.
    #[arg(long, value_name = "NAME", help_heading = "Features")]
    pub(crate) baseline_features: Vec<String>,

    /// Add a feature to the set of features being checked.
    /// The feature will be used in the current version of the crate only.
    #[arg(long, value_name = "NAME", help_heading = "Features")]
    pub(crate) current_features: Vec<String>,

    /// Use all the features, including features named
    /// `unstable`, `nightly`, `bench`, `no_std` or starting with prefixes
    /// `_`, `unstable_`, `unstable-` that are otherwise disabled by default.
    #[arg(
        long,
        help_heading = "Features",
        conflicts_with_all = [
            "default_features",
            "only_explicit_features",
            "features",
            "baseline_features",
            "current_features",
        ]
    )]
    pub(crate) all_features: bool,

    /// Which target to build the crate for, to check platform-specific APIs, e.g.
    /// `x86_64-unknown-linux-gnu`.
    #[arg(long = "target")]
    pub(crate) build_target: Option<String>,

    // docstring for help is on the `clap_verbosity_flag::Verbosity` struct itself
    #[command(flatten)]
    pub(crate) verbosity: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,
}

impl From<CheckRelease> for Check {
    fn from(value: CheckRelease) -> Self {
        let (current, current_project_root) = if let Some(current_rustdoc) = value.current_rustdoc {
            (Rustdoc::from_path(current_rustdoc), None)
        } else if let Some(manifest) = value.manifest.manifest_path {
            let project_root = if manifest.is_dir() {
                manifest
            } else {
                manifest
                    .parent()
                    .expect("manifest path doesn't have a parent")
                    .to_path_buf()
            };
            (Rustdoc::from_root(&project_root), Some(project_root))
        } else {
            let project_root = std::env::current_dir().expect("can't determine current directory");
            (Rustdoc::from_root(&project_root), Some(project_root))
        };
        let mut check = Self::new(current);
        if value.workspace.all || value.workspace.workspace {
            // Specified explicit `--workspace` or `--all`.
            let mut selection = PackageSelection::new(ScopeSelection::Workspace);
            selection.set_excluded_packages(value.workspace.exclude);
            check.set_package_selection(selection);
        } else if !value.workspace.package.is_empty() {
            // Specified explicit `--package`.
            check.set_packages(value.workspace.package);
        } else if !value.workspace.exclude.is_empty() {
            // Specified `--exclude` without `--workspace/--all`.
            // Leave the scope selection to the default ("workspace if the manifest is a workspace")
            // while excluding any specified packages.
            let mut selection = PackageSelection::new(ScopeSelection::DefaultMembers);
            selection.set_excluded_packages(value.workspace.exclude);
            check.set_package_selection(selection);
        }
        let custom_baseline = {
            if let Some(baseline_version) = value.baseline_version {
                Some(Rustdoc::from_registry(baseline_version))
            } else if let Some(baseline_rev) = value.baseline_rev {
                let root = if let Some(baseline_root) = value.baseline_root {
                    baseline_root
                } else if let Some(current_root) = current_project_root {
                    current_root
                } else {
                    std::env::current_dir().expect("can't determine current directory")
                };
                Some(Rustdoc::from_git_revision(root, baseline_rev))
            } else if let Some(baseline_rustdoc) = value.baseline_rustdoc {
                Some(Rustdoc::from_path(baseline_rustdoc))
            } else {
                // Either there's a manually-set baseline root path, or fall through
                // to the default behavior.
                value.baseline_root.map(Rustdoc::from_root)
            }
        };
        if let Some(baseline) = custom_baseline {
            check.set_baseline(baseline);
        }

        if let Some(release_type) = value.release_type {
            check.set_release_type(release_type);
        }

        if value.all_features {
            check.with_all_features();
        } else if value.default_features {
            check.with_default_features();
        } else if value.only_explicit_features {
            check.with_only_explicit_features();
        } else {
            check.with_heuristically_included_features();
        }
        let mut mutual_features = value.features;
        let mut current_features = value.current_features;
        let mut baseline_features = value.baseline_features;
        current_features.append(&mut mutual_features.clone());
        baseline_features.append(&mut mutual_features);

        // Treat --features="" as a no-op like cargo does
        let trim_features = |features: &mut Vec<String>| {
            features.retain(|feature| !(feature.is_empty() || feature == "\"\""));
        };
        trim_features(&mut current_features);
        trim_features(&mut baseline_features);

        check.set_extra_features(current_features, baseline_features);

        if let Some(build_target) = value.build_target {
            check.set_build_target(build_target);
        }

        check
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cargo::command().debug_assert()
}

#[test]
fn features_empty_string_is_no_op() {
    use cargo_semver_checks::Check;

    let Cargo::SemverChecks(SemverChecks {
        check_release: no_features,
        ..
    }) = Cargo::parse_from(["cargo", "semver-checks"]);

    let empty_features = CheckRelease {
        features: vec![String::new()],
        current_features: vec![String::new(), "\"\"".to_string()],
        baseline_features: vec!["\"\"".to_string()],
        ..no_features.clone()
    };

    assert_eq!(Check::from(no_features), Check::from(empty_features));
}
