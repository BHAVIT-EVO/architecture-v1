"""Offline synthetic fixtures for provisioning, integrity, and atomic install."""

import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import minilm


class ModelTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="evo-model-fixture-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.destination = self.root / "installed"
        self.payloads = {
            name: json.dumps({"model_type": "bert", "hidden_size": 384}).encode()
            if name.endswith(".json") else b"synthetic ONNX fixture, not weights"
            for name in minilm.ASSETS
        }
        self.pins = {name: (len(data), hashlib.sha256(data).hexdigest()) for name, data in self.payloads.items()}
        self.addCleanup(patch.stopall)
        patch.object(minilm, "ASSETS", self.pins).start()

    def write(self, destination):
        destination.mkdir(parents=True)
        for name, data in self.payloads.items():
            (destination / name).write_bytes(data)

    def fake_download(self, name, destination):
        # No partial installation is visible while downloading.
        self.assertFalse(self.destination.exists())
        self.assertEqual(destination.parent.parent.parent, self.destination.parent)
        destination.write_bytes(self.payloads[name])

    def test_atomic_install_and_second_fetch_is_offline(self):
        with patch.object(minilm, "download", side_effect=self.fake_download) as downloader, contextlib.redirect_stdout(io.StringIO()):
            directory, checked = minilm.fetch(self.destination)
            self.assertEqual(len(checked), 5)
            self.assertEqual(directory, self.destination)
            self.assertEqual(downloader.call_count, 5)
            downloader.reset_mock()
            minilm.fetch(self.destination)
            downloader.assert_not_called()
        self.assertEqual(list(self.root.iterdir()), [self.destination])

    def test_failed_fetch_leaves_no_install_or_stage(self):
        def corrupt(name, destination):
            self.fake_download(name, destination)
            if name == "tokenizer.json":
                destination.write_bytes(b"x" * len(self.payloads[name]))

        with patch.object(minilm, "download", side_effect=corrupt), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(minilm.ModelError, "SHA-256 mismatch"):
                minilm.fetch(self.destination)
        self.assertFalse(self.destination.exists())
        self.assertEqual(list(self.root.iterdir()), [])

    def test_download_failure_is_not_retried_or_installed(self):
        with patch.object(minilm, "download", side_effect=minilm.ModelError("network failure")) as downloader, contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(minilm.ModelError, "network failure"):
                minilm.fetch(self.destination)
            self.assertEqual(downloader.call_count, 1)
        self.assertEqual(list(self.root.iterdir()), [])

    def test_existing_invalid_directory_is_never_replaced(self):
        self.destination.mkdir()
        marker = self.destination / "personal-file.txt"
        marker.write_bytes(b"user-owned fixture")
        with patch.object(minilm, "download", side_effect=AssertionError("network attempted")):
            with self.assertRaises(minilm.ModelError):
                minilm.fetch(self.destination)
        self.assertEqual(marker.read_bytes(), b"user-owned fixture")

    def test_each_file_is_checked_for_presence_size_and_hash(self):
        self.write(self.destination)
        for name, data in self.payloads.items():
            with self.subTest(name=name):
                path = self.destination / name
                path.unlink()
                with self.assertRaisesRegex(minilm.ModelError, "missing or not a regular file"):
                    minilm.verify(self.destination)
                path.write_bytes(b"")
                with self.assertRaisesRegex(minilm.ModelError, "size mismatch"):
                    minilm.verify(self.destination)
                path.write_bytes(b"x" * len(data))
                with self.assertRaisesRegex(minilm.ModelError, "SHA-256 mismatch"):
                    minilm.verify(self.destination)
                path.write_bytes(data)

    def test_lock_prevents_competing_download(self):
        lock = self.root / ".installed.fetch.lock"
        lock.write_text("existing fetch", encoding="utf-8")
        with patch.object(minilm, "download", side_effect=AssertionError("network attempted")):
            with self.assertRaisesRegex(minilm.ModelError, "fetch lock exists"):
                minilm.fetch(self.destination)
        self.assertEqual(lock.read_text(encoding="utf-8"), "existing fetch")

    def test_destination_appearing_during_fetch_is_preserved(self):
        def race(name, destination):
            destination.write_bytes(self.payloads[name])
            if name == "tokenizer_config.json":
                self.destination.mkdir()
                (self.destination / "user-file").write_bytes(b"preserve")

        with patch.object(minilm, "download", side_effect=race), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(minilm.ModelError, "not overwritten"):
                minilm.fetch(self.destination)
        self.assertEqual((self.destination / "user-file").read_bytes(), b"preserve")
        self.assertEqual(list(self.root.iterdir()), [self.destination])

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_destination_symlink_is_not_replaced(self):
        other = self.root / "other"
        self.write(other)
        self.destination.symlink_to(other, target_is_directory=True)
        with self.assertRaisesRegex(minilm.ModelError, "destination symlink"):
            minilm.fetch(self.destination)
        self.assertTrue(self.destination.is_symlink())

    def test_doctor_does_not_launch_subprocess_or_claim_inference(self):
        self.write(self.destination)
        output = io.StringIO()
        with patch.object(subprocess, "run", side_effect=AssertionError("unexpected subprocess")), contextlib.redirect_stdout(output):
            self.assertEqual(minilm.main(["doctor", "--model-dir", str(self.destination)]), 0)
        self.assertIn("ONNX inference not tested", output.getvalue())

    def test_loader_precedence_cache_ref_and_no_fallback(self):
        env = {"EVO_EMBEDDING_MODEL_DIR": "exact", "HF_HOME": "hf", "FASTEMBED_CACHE_DIR": "fast"}
        self.assertEqual(minilm.configured_directory(env, self.root), (self.root / "exact", "EVO_EMBEDDING_MODEL_DIR"))
        env["EVO_EMBEDDING_MODEL_DIR"] = ""
        with self.assertRaisesRegex(minilm.ModelError, "empty path"):
            minilm.configured_directory(env, self.root)
        del env["EVO_EMBEDDING_MODEL_DIR"]
        ref = self.root / "hf" / minilm.CACHE_REPO / "refs" / "main"
        ref.parent.mkdir(parents=True)
        ref.write_text("aabbcc\n", encoding="utf-8")
        directory, source = minilm.configured_directory(env, self.root)
        self.assertEqual(directory, ref.parent.parent / "snapshots" / "aabbcc")
        self.assertEqual(source, "HF_HOME")
        for invalid in ("", "../model", "/absolute", "ab\ncd"):
            ref.write_text(invalid, encoding="utf-8")
            with self.assertRaisesRegex(minilm.ModelError, "invalid cache ref"):
                minilm.configured_directory(env, self.root)

    def test_download_uses_only_pinned_https_without_credentials(self):
        with patch.object(minilm.shutil, "which", return_value="/usr/bin/curl"), patch.object(subprocess, "run") as run:
            run.return_value.returncode = 0
            minilm.download("model.onnx", self.root / "staged")
            args, kwargs = run.call_args
            command = args[0]
            self.assertEqual(command[1], "--disable")
            self.assertEqual(command[-1], f"https://huggingface.co/{minilm.REPOSITORY}/resolve/{minilm.REVISION}/model.onnx")
            self.assertEqual(set(kwargs["env"]), {"PATH"})
            self.assertNotIn("--retry", command)
            self.assertNotIn("--cookie", command)
            self.assertNotIn("--user", command)


if __name__ == "__main__":
    unittest.main()
