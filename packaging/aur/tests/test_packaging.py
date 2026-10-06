import hashlib
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest


RECIPE_DIR = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("prepare_ci", RECIPE_DIR / "prepare_ci.py")
prepare_ci = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(prepare_ci)


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.git("init", "-q")
        self.git("config", "user.name", "Packaging Test")
        self.git("config", "user.email", "packaging@example.invalid")
        crate = self.repo / "logmancer-desktop"
        crate.mkdir()
        (crate / "Cargo.toml").write_text('[package]\nversion = "0.5.0-beta.2"\n')
        recipes = self.repo / "packaging/aur"
        recipes.mkdir(parents=True)
        for name in ("PKGBUILD", "logmancer.desktop"):
            shutil.copy2(RECIPE_DIR / name, recipes)
        self.git("add", ".")
        self.git("commit", "-qm", "fixture")
        self.git("tag", "v0.5.0-beta.2")

    def git(self, *args):
        return prepare_ci.git(self.repo, *args)

    def test_release_archive_and_recipe(self):
        original = (self.repo / "packaging/aur/PKGBUILD").read_bytes()
        destination = self.root / "release"
        self.assertEqual(prepare_ci.stage(self.repo, destination, "v0.5.0-beta.2", True), "0.5.0beta2")
        self.assertEqual(original, (self.repo / "packaging/aur/PKGBUILD").read_bytes())
        recipe = (destination / "PKGBUILD").read_text()
        self.assertIn("_srcname=logmancer-source", recipe)
        with (destination / "logmancer-source.tar.gz").open("rb") as source:
            checksum = hashlib.file_digest(source, "sha256").hexdigest()
        self.assertIn(checksum, recipe)
        with tarfile.open(destination / "logmancer-source.tar.gz") as source:
            self.assertIn("logmancer-source/logmancer-desktop/Cargo.toml", source.getnames())

    def test_snapshots_exclude_uncommitted_files(self):
        (self.repo / "untracked.txt").write_text("not part of the workflow source")
        (self.repo / "logmancer-desktop/Cargo.toml").write_text("local changes")
        destination = self.root / "snapshot"
        version = prepare_ci.stage(self.repo, destination, "HEAD", False)
        self.assertRegex(version, r"^0\.5\.0beta2\.r\d+\.g[0-9a-f]{12}$")
        self.assertEqual((destination / "SOURCE_COMMIT").read_text().strip(), self.git("rev-parse", "HEAD"))
        with tarfile.open(destination / "logmancer-source.tar.gz") as source:
            self.assertNotIn("logmancer-source/untracked.txt", source.getnames())
            self.assertIn(b"0.5.0-beta.2", source.extractfile("logmancer-source/logmancer-desktop/Cargo.toml").read())

    def test_reject_wrong_release_tag(self):
        self.git("tag", "v0.5.0-beta.3")
        with self.assertRaises(ValueError):
            prepare_ci.stage(self.repo, self.root / "bad", "v0.5.0-beta.3", True)
        self.assertFalse((self.root / "bad").exists())

    def test_reject_existing_destination(self):
        with self.assertRaises(FileExistsError):
            prepare_ci.stage(self.repo, self.repo, "HEAD", False)

    def test_versions(self):
        self.assertEqual(prepare_ci.arch_version("0.5.0-beta.2"), "0.5.0beta2")
        self.assertEqual(prepare_ci.arch_version("0.5.0"), "0.5.0")
        for version in ("main", "0.5.0-rc.1", "0.5.0;echo bad"):
            with self.assertRaises(ValueError):
                prepare_ci.arch_version(version)

    @unittest.skipUnless(shutil.which("vercmp"), "Requires pacman")
    def test_pacman_version_ordering(self):
        ordered = ["0.4.1", "0.5.0beta1", "0.5.0beta2", "0.5.0beta2.r100.gabc",
                   "0.5.0beta2.r200.gdef", "0.5.0beta3", "0.5.0", "0.5.0.r200.gabc", "0.5.1"]
        for previous, following in zip(ordered, ordered[1:]):
            self.assertLess(int(subprocess.check_output(["vercmp", previous, following])), 0)

    @unittest.skipUnless(shutil.which("makepkg"), "Requires makepkg")
    def test_ci_sources_pass_makepkg_checksums(self):
        destination = self.root / "checksums"
        prepare_ci.stage(self.repo, destination, "HEAD", False)
        subprocess.run(["makepkg", "--verifysource", "--nodeps"], cwd=destination, check=True,
                       capture_output=True)

    def test_package_layout_without_compiling(self):
        src = self.root / "src"
        suite = src / "fixture"
        target = suite / "target/release"
        target.mkdir(parents=True)
        for binary in ("logmancer", "logmancer-desktop", "logmancer-web", "logmancer-tui"):
            (target / binary).write_text("binary fixture")
        assets = suite / "target/site/pkg"
        assets.mkdir(parents=True)
        for extension in ("css", "js", "wasm"):
            (assets / f"logmancer-web.{extension}").write_text("asset fixture")
        icons = suite / "logmancer-desktop/icons"
        icons.mkdir(parents=True)
        for icon in ("32x32.png", "128x128.png", "128x128@2x.png"):
            (icons / icon).write_text("icon fixture")
        (suite / "LICENSE").write_text("MIT fixture")
        shutil.copy2(RECIPE_DIR / "logmancer.desktop", src)
        pkg = self.root / "pkg"
        subprocess.run(["bash", "-eu", "-c", 'source "$1"; srcdir=$2; pkgdir=$3; _srcname=fixture; package',
                        "test", str(RECIPE_DIR / "PKGBUILD"), str(src), str(pkg)], check=True)
        for binary in ("logmancer", "logmancer-desktop", "logmancer-web", "logmancer-tui"):
            self.assertEqual((pkg / "usr/bin" / binary).resolve(), pkg / "usr/lib/logmancer" / binary)
            self.assertEqual((pkg / "usr/lib/logmancer" / binary).stat().st_mode & 0o777, 0o755)
        self.assertEqual((pkg / "usr/lib/Logmancer/site").resolve(), pkg / "usr/lib/logmancer/site")
        self.assertTrue((pkg / "usr/share/applications/logmancer.desktop").is_file())
        self.assertTrue((pkg / "usr/share/licenses/logmancer/LICENSE").is_file())

    def test_namcap_errors_fail_even_with_zero_exit_status(self):
        tool = self.root / "namcap"
        tool.write_text('#!/bin/sh\nprintf "logmancer E: missing dependency\\n"\n')
        tool.chmod(0o755)
        env = dict(os.environ, PATH=f"{self.root}:{os.environ['PATH']}")
        command = ["python", str(RECIPE_DIR / "check_namcap.py"), "PKGBUILD"]
        result = subprocess.run(command, env=env, capture_output=True)
        self.assertEqual(result.returncode, 1)
        tool.write_text('#!/bin/sh\nprintf "logmancer W: review this warning\\n"\n')
        result = subprocess.run(command, env=env, capture_output=True)
        self.assertEqual(result.returncode, 0)

    def test_package_rejects_missing_generated_assets(self):
        src = self.root / "src"
        (src / "fixture").mkdir(parents=True)
        result = subprocess.run(
            ["bash", "-eu", "-c", 'source "$1"; srcdir=$2; pkgdir=$3; _srcname=fixture; package',
             "test", str(RECIPE_DIR / "PKGBUILD"), str(src), str(self.root / "pkg")],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("Missing generated Leptos asset", result.stderr)

    @unittest.skipUnless(shutil.which("makepkg"), "Requires makepkg")
    def test_committed_srcinfo_matches_recipe(self):
        generated = subprocess.check_output(["makepkg", "--printsrcinfo"], cwd=RECIPE_DIR, text=True)
        self.assertEqual(generated, (RECIPE_DIR / ".SRCINFO").read_text())


if __name__ == "__main__":
    unittest.main()
