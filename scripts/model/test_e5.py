"""Network-free synthetic E5 fixtures; these are NOT audited upstream JSON blobs."""

import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import e5


class E5ModelTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="evo-e5-synthetic-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.destination = self.root / "installed"
        self.payloads = {
            "onnx/model.onnx": b"synthetic ONNX fixture, not weights",
            "tokenizer.json": b'{"synthetic_fixture": true}',
            "config.json": b'{"model_type": "bert", "hidden_size": 384}',
            "special_tokens_map.json": b'{"synthetic_fixture": true}',
            "tokenizer_config.json": b'{"synthetic_fixture": true}',
        }
        self.pins = {name: (len(data), hashlib.sha256(data).hexdigest()) for name, data in self.payloads.items()}
        self.addCleanup(patch.stopall)
        patch.object(e5, "ASSETS", self.pins).start()
        patch.object(subprocess, "run", side_effect=AssertionError("unexpected subprocess/network access")).start()
        patch.object(socket, "create_connection", side_effect=AssertionError("unexpected network access")).start()

    def write(self, destination):
        destination.mkdir(parents=True)
        for name, data in self.payloads.items():
            path = destination / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)

    def fake_download(self, name, destination):
        self.assertFalse(self.destination.exists(), "partial installation became visible")
        staged = destination.parents[1] if name.startswith("onnx/") else destination.parent
        self.assertEqual(staged.name, "model")
        self.assertEqual(staged.parent.parent, self.destination.parent)
        self.assertEqual(destination.relative_to(staged).as_posix(), name)
        self.assertTrue(destination.parent.is_dir(), "nested staging parent was not created")
        destination.write_bytes(self.payloads[name])

    def fetch_quietly(self):
        with contextlib.redirect_stdout(io.StringIO()):
            return e5.fetch(self.destination)

    def repin_fixture(self, name, data):
        self.payloads[name] = data
        self.pins[name] = (len(data), hashlib.sha256(data).hexdigest())

    def test_atomic_install_retains_nested_layout_and_second_fetch_is_offline(self):
        with patch.object(e5, "download", side_effect=self.fake_download) as downloader:
            directory, checked = self.fetch_quietly()
            self.assertEqual(directory, self.destination)
            self.assertEqual(len(checked), 5)
            self.assertEqual(downloader.call_count, 5)
            self.assertTrue((directory / "onnx/model.onnx").is_file())
            self.assertFalse((directory / "model.onnx").exists())
            downloader.reset_mock()
            self.fetch_quietly()
            downloader.assert_not_called()
        self.assertEqual(list(self.root.iterdir()), [self.destination])

    def test_corrupt_fetch_leaves_no_install_stage_or_lock(self):
        def corrupt(name, destination):
            self.fake_download(name, destination)
            if name == "tokenizer.json":
                destination.write_bytes(b"x" * len(self.payloads[name]))

        with patch.object(e5, "download", side_effect=corrupt):
            with self.assertRaisesRegex(e5.ModelError, "SHA-256 mismatch"):
                self.fetch_quietly()
        self.assertEqual(list(self.root.iterdir()), [])

    def test_download_failure_is_not_retried_or_installed(self):
        with patch.object(e5, "download", side_effect=e5.ModelError("network failure")) as downloader:
            with self.assertRaisesRegex(e5.ModelError, "network failure"):
                self.fetch_quietly()
            self.assertEqual(downloader.call_count, 1)
        self.assertEqual(list(self.root.iterdir()), [])

    def test_existing_invalid_directory_is_never_repaired(self):
        self.destination.mkdir()
        marker = self.destination / "user-owned.txt"
        marker.write_bytes(b"preserve this synthetic marker")
        with patch.object(e5, "download") as downloader:
            with self.assertRaisesRegex(e5.ModelError, "missing or not a regular file"):
                self.fetch_quietly()
            downloader.assert_not_called()
        self.assertEqual(marker.read_bytes(), b"preserve this synthetic marker")

    def test_each_file_is_checked_for_presence_size_and_digest(self):
        self.write(self.destination)
        for name, data in self.payloads.items():
            with self.subTest(name=name):
                path = self.destination / name
                path.unlink()
                with self.assertRaisesRegex(e5.ModelError, "missing or not a regular file"):
                    e5.verify(self.destination)
                path.write_bytes(b"")
                with self.assertRaisesRegex(e5.ModelError, "size mismatch"):
                    e5.verify(self.destination)
                path.write_bytes(b"x" * len(data))
                with self.assertRaisesRegex(e5.ModelError, "SHA-256 mismatch"):
                    e5.verify(self.destination)
                path.write_bytes(data)

    def test_flattened_onnx_layout_is_refused(self):
        self.write(self.destination)
        (self.destination / "onnx/model.onnx").rename(self.destination / "model.onnx")
        with self.assertRaisesRegex(e5.ModelError, "onnx/model.onnx"):
            e5.verify(self.destination)

    def test_all_json_files_must_be_objects_even_when_hash_matches(self):
        for name in self.payloads:
            if not name.endswith(".json"):
                continue
            with self.subTest(name=name), tempfile.TemporaryDirectory(dir=self.root) as fixture:
                with patch.dict(self.pins, {name: (2, hashlib.sha256(b"[]").hexdigest())}):
                    self.write(Path(fixture) / "model")
                    (Path(fixture) / "model" / name).write_bytes(b"[]")
                    with self.assertRaisesRegex(e5.ModelError, "expected a JSON object"):
                        e5.verify(Path(fixture) / "model")

    def test_config_must_be_384_dimensional_bert(self):
        for content in ({"model_type": "xlm-roberta", "hidden_size": 384}, {"model_type": "bert", "hidden_size": 768}, {}):
            with self.subTest(content=content), tempfile.TemporaryDirectory(dir=self.root) as fixture:
                self.repin_fixture("config.json", json.dumps(content).encode())
                self.write(Path(fixture) / "model")
                with self.assertRaisesRegex(e5.ModelError, "384-dimensional BERT"):
                    e5.verify(Path(fixture) / "model")

    def test_invalid_json_with_matching_hash_blocks_main(self):
        self.repin_fixture("special_tokens_map.json", b"not json")
        self.write(self.destination)
        with contextlib.redirect_stderr(io.StringIO()) as error:
            self.assertEqual(e5.main(["doctor", "--model-dir", str(self.destination)]), 1)
        self.assertIn("[BLOCKED]", error.getvalue())

    def test_lock_prevents_competing_fetch_without_retry(self):
        lock = self.root / ".installed.fetch.lock"
        lock.write_text("existing fetch", encoding="utf-8")
        with patch.object(e5, "download") as downloader:
            with self.assertRaisesRegex(e5.ModelError, "fetch lock exists"):
                self.fetch_quietly()
            downloader.assert_not_called()
        self.assertEqual(lock.read_text(encoding="utf-8"), "existing fetch")

    def test_destination_appearing_at_rename_is_never_replaced(self):
        real_install = e5.atomic_installer()

        def racing_install(source, target):
            target.mkdir()
            # Even an empty destination must not be replaced by os.rename.
            return real_install(source, target)

        with patch.object(e5, "atomic_installer", return_value=racing_install), patch.object(e5, "download", side_effect=self.fake_download):
            with self.assertRaises((e5.ModelError, FileExistsError)):
                self.fetch_quietly()
        self.assertTrue(self.destination.is_dir())
        self.assertEqual(list(self.destination.iterdir()), [])
        self.assertEqual(list(self.root.iterdir()), [self.destination])

    def test_unsupported_atomic_install_blocks_before_network(self):
        with patch.object(e5, "atomic_installer", side_effect=e5.ModelError("unsupported platform")), patch.object(e5, "download") as downloader:
            with self.assertRaisesRegex(e5.ModelError, "unsupported platform"):
                self.fetch_quietly()
            downloader.assert_not_called()
        self.assertEqual(list(self.root.iterdir()), [])

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_destination_symlink_is_refused_even_when_target_is_valid(self):
        other = self.root / "other"
        self.write(other)
        self.destination.symlink_to(other, target_is_directory=True)
        with self.assertRaisesRegex(e5.ModelError, "destination symlink"):
            self.fetch_quietly()
        self.assertTrue(self.destination.is_symlink())

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_dangling_destination_symlink_is_refused(self):
        self.destination.symlink_to(self.root / "missing", target_is_directory=True)
        with self.assertRaisesRegex(e5.ModelError, "destination symlink"):
            self.fetch_quietly()
        self.assertTrue(self.destination.is_symlink())

    def test_doctor_is_offline_and_does_not_claim_inference(self):
        self.write(self.destination)
        with patch.object(e5, "download") as downloader, contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(e5.main(["doctor", "--model-dir", str(self.destination)]), 0)
            downloader.assert_not_called()
        self.assertIn(e5.REVISION, output.getvalue())
        self.assertIn("onnx/model.onnx", output.getvalue())
        self.assertIn("ONNX inference not tested", output.getvalue())

    def test_fetch_prints_both_profile_and_shell_escaped_directory(self):
        self.destination = self.root / "directory with 'quotes'"
        with patch.object(e5, "download", side_effect=self.fake_download), contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(e5.main(["fetch", "--model-dir", str(self.destination)]), 0)
        self.assertIn("export EVO_EMBEDDING_PROFILE=multilingual-e5-small", output.getvalue())
        import shlex
        self.assertIn("export EVO_EMBEDDING_MODEL_DIR=" + shlex.quote(str(self.destination)), output.getvalue())

    def test_loader_precedence_and_profile_specific_cache(self):
        env = {"EVO_EMBEDDING_MODEL_DIR": "exact", "HF_HOME": "hf", "FASTEMBED_CACHE_DIR": "fast"}
        self.assertEqual(e5.configured_directory(env, self.root), (self.root / "exact", "EVO_EMBEDDING_MODEL_DIR"))
        for name in list(env):
            with self.subTest(empty=name), patch.dict(env, {name: ""}):
                if name != "EVO_EMBEDDING_MODEL_DIR":
                    reduced = {key: value for key, value in env.items() if key != "EVO_EMBEDDING_MODEL_DIR"}
                    if name == "FASTEMBED_CACHE_DIR":
                        reduced.pop("HF_HOME")
                else:
                    reduced = env
                with self.assertRaisesRegex(e5.ModelError, "empty path"):
                    e5.configured_directory(reduced, self.root)
        for selected, source in ((self.root / "hf", "HF_HOME"), (self.root / "fast", "FASTEMBED_CACHE_DIR"), (self.root / ".fastembed_cache", "current-directory .fastembed_cache")):
            ref = selected / e5.CACHE_REPO / "refs" / "main"
            ref.parent.mkdir(parents=True)
            ref.write_text("aabbcc\n", encoding="utf-8")
            env.pop("EVO_EMBEDDING_MODEL_DIR", None)
            if source != "HF_HOME":
                env.pop("HF_HOME", None)
            if source.startswith("current-directory"):
                env.pop("FASTEMBED_CACHE_DIR", None)
            self.assertEqual(e5.configured_directory(env, self.root), (ref.parent.parent / "snapshots/aabbcc", source))
            for invalid in ("", "../model", "/absolute", "ab\ncd"):
                ref.write_text(invalid, encoding="utf-8")
                with self.assertRaisesRegex(e5.ModelError, "invalid cache ref"):
                    e5.configured_directory(env, self.root)

    def test_missing_configured_cache_never_falls_back(self):
        with self.assertRaises(FileNotFoundError):
            e5.configured_directory({"HF_HOME": "missing", "FASTEMBED_CACHE_DIR": "also-missing"}, self.root)

    def test_curl_has_pinned_https_url_no_credentials_and_no_retries(self):
        with patch.object(e5.shutil, "which", return_value="/usr/bin/curl"), patch.object(subprocess, "run") as run:
            run.return_value.returncode = 0
            for name in self.pins:
                with self.subTest(name=name):
                    e5.download(name, self.root / "staged")
                    args, kwargs = run.call_args
                    command = args[0]
                    self.assertEqual(command[1], "--disable")
                    self.assertEqual(command[-1], f"https://huggingface.co/{e5.REPOSITORY}/resolve/{e5.REVISION}/{name}")
                    self.assertEqual(command[command.index("--proto") + 1], "=https")
                    self.assertEqual(command[command.index("--proto-redir") + 1], "=https")
                    self.assertEqual(command[command.index("--max-filesize") + 1], str(self.pins[name][0] + 4096))
                    self.assertEqual(set(kwargs["env"]), {"PATH"})
                    self.assertEqual(kwargs["stdout"], subprocess.DEVNULL)
                    for forbidden in ("--retry", "--cookie", "--user", "--netrc", "--header", "--config"):
                        self.assertNotIn(forbidden, command)

    def test_curl_failure_does_not_log_network_details_or_retry(self):
        with patch.object(e5.shutil, "which", return_value="/usr/bin/curl"), patch.object(subprocess, "run") as run:
            run.return_value.returncode = 22
            run.return_value.stderr = b"sensitive signed CDN URL must not appear"
            with self.assertRaisesRegex(e5.ModelError, "curl exit 22") as error:
                e5.download("onnx/model.onnx", self.root / "staged")
            self.assertNotIn("sensitive", str(error.exception))
            self.assertEqual(run.call_count, 1)

    def test_unknown_asset_is_refused_before_curl(self):
        with self.assertRaisesRegex(e5.ModelError, "unrecognized public asset"):
            e5.download("../unreviewed", self.root / "staged")

    def test_missing_curl_blocks_fetch_download(self):
        with patch.object(e5.shutil, "which", return_value=None):
            with self.assertRaisesRegex(e5.ModelError, "curl is required"):
                e5.download("onnx/model.onnx", self.root / "staged")


class E5PinTests(unittest.TestCase):
    def test_production_identity_and_lfs_pins_are_immutable(self):
        self.assertEqual(e5.REPOSITORY, "intfloat/multilingual-e5-small")
        self.assertEqual(e5.REVISION, "614241f622f53c4eeff9890bdc4f31cfecc418b3")
        self.assertEqual(e5.LICENSE, "MIT")
        self.assertEqual(e5.ASSETS["onnx/model.onnx"], (470268510, "ca456c06b3a9505ddfd9131408916dd79290368331e7d76bb621f1cba6bc8665"))
        self.assertEqual(e5.ASSETS["tokenizer.json"], (17082730, "0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39"))
        self.assertEqual(e5.ASSETS["config.json"], (655, "69137736cab8b8903a07fe8afaafdda25aac55415a12a55d1bffa9f581abf959"))
        self.assertEqual(e5.ASSETS["special_tokens_map.json"], (167, "d05497f1da52c5e09554c0cd874037a083e1dc1b9cfd48034d1c717f1afc07a7"))
        self.assertEqual(e5.ASSETS["tokenizer_config.json"], (443, "a1d6bc8734a6f635dc158508bef000f8e2e5a759c7d92f984b2c86e5ff53425b"))
        self.assertEqual({name: size for name, (size, _) in e5.ASSETS.items()}, {
            "onnx/model.onnx": 470268510, "tokenizer.json": 17082730,
            "config.json": 655, "special_tokens_map.json": 167, "tokenizer_config.json": 443,
        })

    def test_missing_pins_block_all_entrypoints_before_io(self):
        incomplete = dict(e5.ASSETS)
        missing = ("config.json", "special_tokens_map.json", "tokenizer_config.json")
        for name in missing:
            incomplete[name] = (incomplete[name][0], None)
        with tempfile.TemporaryDirectory(prefix="evo-e5-pin-gate-") as temp, patch.object(e5, "ASSETS", incomplete), patch.object(subprocess, "run", side_effect=AssertionError("network attempted")), patch.object(e5, "atomic_installer", side_effect=AssertionError("install attempted")):
            root = Path(temp)
            directory = root / "not-created" / "model"
            for action in (lambda: e5.fetch(directory), lambda: e5.verify(directory), lambda: e5.download("onnx/model.onnx", root / "download")):
                with self.assertRaises(e5.ModelError) as error:
                    action()
                for name in missing:
                    self.assertIn(name, str(error.exception))
            for mode in ("fetch", "doctor"):
                with contextlib.redirect_stderr(io.StringIO()) as error:
                    self.assertEqual(e5.main([mode, "--model-dir", str(directory)]), 1)
                self.assertIn("missing independently audited SHA-256 pins", error.getvalue())
            self.assertEqual(list(root.iterdir()), [])

    def test_malformed_builtin_digest_cannot_enable_download(self):
        for bad in ("", "a" * 63, "z" * 64, "A" * 64, 42):
            with self.subTest(bad=bad), patch.object(e5, "ASSETS", {"config.json": (655, bad)}):
                with self.assertRaisesRegex(e5.ModelError, "invalid built-in"):
                    e5.require_pins()


if __name__ == "__main__":
    unittest.main()
