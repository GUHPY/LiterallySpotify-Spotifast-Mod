import hashlib
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest


def load_module(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


engine = load_module("engine_sync", "update-engine.py")
signer = load_module("release_signer", "sign-release.py")


class EngineUpdates(unittest.TestCase):
    def test_only_engine_dependencies_change(self):
        base = '[package]\nname="spotifast"\nversion="0.13.3"\n[dependencies]\neframe="0.36"\nrodio="0.21"\n'
        local = base.replace('version="0.13.3"', 'version="0.1.0"')
        upstream = base.replace('version="0.13.3"', 'version="0.14.1"').replace('eframe="0.36"', 'eframe="0.37"').replace('rodio="0.21"', 'rodio="0.22"')
        updated, names = engine.update_dependencies(local, base, upstream, ["rodio"])
        self.assertIn('version="0.1.0"', updated)
        self.assertIn('eframe="0.36"', updated)
        self.assertIn('rodio="0.22"', updated)
        self.assertEqual(names, ["dependencies.rodio"])

    def test_engine_dependency_conflicts_require_review(self):
        base = '[dependencies]\nrodio="0.21"\n'
        with self.assertRaisesRegex(ValueError, "manual reconciliation"):
            engine.update_dependencies(base.replace('0.21', '0.21.1'), base, base.replace('0.21', '0.22'), ["rodio"])

    def test_new_engine_table_preserves_existing_ui_dependencies(self):
        base = '[dependencies]\neframe="0.36"\n'
        target = base + '[target.\'cfg(windows)\'.dependencies]\ncpal="0.16"\n'
        updated, _ = engine.update_dependencies(base, base, target, ["cpal"])
        self.assertEqual(engine.manifest_identity(base, ["cpal"]), engine.manifest_identity(updated, ["cpal"]))
        self.assertIn('cpal="0.16"', updated)

    def test_unknown_nested_dependency_format_fails_closed(self):
        with self.assertRaisesRegex(ValueError, "Unsupported engine dependency table"):
            engine.dependency_entries('[dependencies.rodio]\nversion="0.22"\n', ["rodio"])

    def test_core_patch_never_imports_the_upstream_ui(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            def git(*args, data=None):
                return subprocess.check_output(["git", *args], cwd=root, input=data)
            git("init", "--quiet", "--initial-branch=main")
            git("config", "user.name", "Maintenance fixture")
            git("config", "user.email", "fixture@example.invalid")
            (root / "src/ui").mkdir(parents=True)
            (root / "src/backend.rs").write_text("engine version one\n")
            (root / "src/ui/home.rs").write_text("upstream original UI\n")
            git("add", "."); git("commit", "--quiet", "-m", "base")
            base = git("rev-parse", "HEAD").decode().strip()
            git("switch", "--quiet", "-c", "upstream-fixture")
            (root / "src/backend.rs").write_text("engine version two\n")
            (root / "src/ui/home.rs").write_text("upstream replaced UI\n")
            git("add", "."); git("commit", "--quiet", "-m", "upstream")
            target = git("rev-parse", "HEAD").decode().strip()
            git("switch", "--quiet", "main")
            (root / "src/ui/home.rs").write_text("LiterallySpotify UI stays here\n")
            git("add", "."); git("commit", "--quiet", "-m", "fork UI")
            before = hashlib.sha256((root / "src/ui/home.rs").read_bytes()).hexdigest()
            patch = git("diff", "--binary", "--no-renames", base, target, "--", "src/backend.rs")
            git("apply", "--3way", "--index", data=patch)
            self.assertEqual((root / "src/backend.rs").read_text(), "engine version two\n")
            self.assertEqual(hashlib.sha256((root / "src/ui/home.rs").read_bytes()).hexdigest(), before)
            self.assertEqual(git("diff", "--cached", "--name-only").decode().strip(), "src/backend.rs")

    def test_ref_input_cannot_supply_git_options(self):
        for reference in ["--upload-pack=other", "../main", "branch;command"]:
            with self.assertRaises(ValueError):
                engine.prepare(reference, False, {})


class SignedReleases(unittest.TestCase):
    def test_manifest_is_signed_with_the_embedded_publisher_key(self):
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
        from cryptography.hazmat.primitives import serialization
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "Spotify.exe").write_bytes(b"release fixture")
            key = Ed25519PrivateKey.generate()
            public = key.public_key().public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)
            seed = key.private_bytes(serialization.Encoding.Raw, serialization.PrivateFormat.Raw, serialization.NoEncryption())
            public_file = root / "publisher.hex"
            public_file.write_text(public.hex())
            signer.sign(root, public_file, seed.hex())
            manifest = (root / "checksums.txt").read_bytes()
            key.public_key().verify((root / "checksums.txt.sig").read_bytes(), manifest)
            self.assertIn(hashlib.sha256(b"release fixture").hexdigest().encode(), manifest)
            other = Ed25519PrivateKey.generate().private_bytes(serialization.Encoding.Raw, serialization.PrivateFormat.Raw, serialization.NoEncryption())
            with self.assertRaisesRegex(ValueError, "does not match"):
                signer.sign(root, public_file, other.hex())


if __name__ == "__main__":
    unittest.main()
