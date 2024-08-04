//! Snapshot tests of `cargo semver-checks` runs to ensure
//! that we define how we handle edge cases.
//!
//! # Updating test output
//!
//! If you introduce changes into `cargo-semver-checks` that modify its behavior
//! so that these tests fail, the snapshot may need to be updated.  After you've
//! determined that the new behavior is not a regression and the test should
//! be updated, run the following:
//!
//! `$ cargo insta review` (you may need to `cargo install cargo-insta`)
//!
//! If the changes are intended, to update the test, accept the new output
//! in the `cargo insta review` CLI.  Make sure to commit the
//! `src/snapshots/{name}.snap` file in your PR.
//!
//! Alternatively, if you can't use `cargo-insta`, review the changed files
//! in the `tests/snapshots/ directory by moving `{name}.snap.new` to
//! `{name}.snap` to update the snapshot.  To update all changed tests,
//! run `INSTA_UPDATE=always cargo test --test snapshot_tests`
//!
//! # Adding a new test
//!
//! To add a new test, typically you will want to use `create_command`
//! and add arguments (especially `--manifest-path` and `--baseline-root`).
//! Then, call [`assert_cmd_snapshot!`] with the command.
//!
//! Then run `cargo test --lib snapshot_tests`.  The new test should fail, as
//! there is no snapshot to compare to.  Review the output with `cargo insta review`,
//! and accept it when the captured behavior is correct. (see above if you can't use
//! `cargo-insta`)

use std::{cell::RefCell, fmt, io::Cursor, rc::Rc};

use clap::Parser;
use insta::{assert_ron_snapshot, assert_snapshot};

use crate::cli::{Cargo, SemverChecks};
use cargo_semver_checks::{Check, GlobalConfig};

#[derive(Debug)]
struct CheckOutput {
    /// Whether the check succeed; that is, whether `cargo semver-checks` would exit 0.
    success: bool,
    /// The captured stdout of the check.
    stdout: String,
    /// The captured stderr of the check.
    stderr: String,
}

impl fmt::Display for CheckOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "success: {}", self.success)?;
        writeln!(f, "--- stdout ---\n{}", self.stdout)?;
        writeln!(f, "--- stderr ---\n{}", self.stderr)?;
        Ok(())
    }
}

#[derive(Debug)]
struct CheckResult(anyhow::Result<CheckOutput>);

impl fmt::Display for CheckResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "exit code: {}", self.exit_code())?;
        match &self.0 {
            Ok(out) => write!(f, "{out}"),
            Err(err) => writeln!(f, "--- error ---\n{err}"),
        }
    }
}

impl CheckResult {
    #[must_use]
    fn new(mut config: GlobalConfig, check: Check) -> Self {
        /// Hack struct to implement `Write + 'static`
        /// on a Vec<u8> to read from later.
        #[derive(Clone, Default)]
        struct W(Rc<RefCell<Cursor<Vec<u8>>>>);

        impl std::io::Write for W {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.borrow_mut().write(buf)
            }

            fn flush(&mut self) -> std::io::Result<()> {
                self.0.borrow_mut().flush()
            }
        }

        let stdout = W::default();
        let stderr = W::default();

        config.set_stdout(Box::new(stdout.clone()));
        config.set_stderr(Box::new(stderr.clone()));

        let result = check.check_release(&mut config);

        Self(result.map(|x| CheckOutput {
            success: x.success(),
            stdout: String::from_utf8_lossy(stdout.0.borrow().get_ref()).into_owned(),
            stderr: String::from_utf8_lossy(stderr.0.borrow().get_ref()).into_owned(),
        }))
    }

    #[inline]
    fn exit_code(&self) -> i32 {
        match &self.0 {
            Ok(out) => out.success as i32,
            Err(_) => 1,
        }
    }
}

/// [#163](https://github.com/obi1kenobi/cargo-semver-checks/issues/163)
///
/// Running `cargo semver-checks --workspace` on a workspace that has library
/// targets should be an error.
#[test]
fn workspace_no_lib_targets_error() {
    let cli_config = Cargo::try_parse_from([
        "cargo",
        "semver-checks",
        "--manifest-path",
        "test_crates/manifest_tests/no_lib_targets/new",
        "--baseline-root",
        "test_crates/manifest_tests/no_lib_targets/old",
        "--workspace",
    ])
    .expect("args should be valid");

    let Cargo::SemverChecks(SemverChecks { check_release, .. }) = cli_config;

    let check = check_release.into();

    assert_ron_snapshot!("check-stage", check);

    let out = CheckResult::new(GlobalConfig::new(), check);
    assert_snapshot!(out);
}
