"""Regression tests for NicoChat Makefile helpers."""

from __future__ import annotations

import os
import stat
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path


_REPO_ROOT = Path(__file__).resolve().parent.parent


class TestEnsureOllama(unittest.TestCase):
    """Tests for the Makefile ensure-ollama target."""

    def _write_fake_ollama(self, bin_dir: Path, state_dir: Path) -> None:
        script = textwrap.dedent(
            f"""\
            #!/bin/sh
            set -eu
            state_dir="{state_dir}"
            calls_file="$state_dir/calls.log"
            ready_file="$state_dir/ready.flag"
            printf '%s\\n' "$1" >> "$calls_file"
            case "$1" in
              list)
                [ -f "$ready_file" ]
                ;;
              serve)
                touch "$ready_file"
                ;;
              *)
                exit 0
                ;;
            esac
            """
        )
        ollama_path = bin_dir / "ollama"
        ollama_path.write_text(script, encoding="utf-8")
        ollama_path.chmod(ollama_path.stat().st_mode | stat.S_IXUSR)

    def _run_make(self, *, extra_env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        if extra_env:
            env.update(extra_env)
        return subprocess.run(
            ["make", "ensure-ollama"],
            cwd=_REPO_ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

    def test_ensure_ollama_starts_service_when_list_fails(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            bin_dir = temp_path / "bin"
            state_dir = temp_path / "state"
            bin_dir.mkdir()
            state_dir.mkdir()
            self._write_fake_ollama(bin_dir, state_dir)

            result = self._run_make(
                extra_env={"PATH": f"{bin_dir}:{os.environ['PATH']}"}
            )

            self.assertEqual(
                result.returncode,
                0,
                f"stdout:\n{result.stdout}\n\nstderr:\n{result.stderr}",
            )
            self.assertIn("Starting local Ollama service", result.stdout)
            self.assertIn("Ollama is ready.", result.stdout)
            calls = (state_dir / "calls.log").read_text(encoding="utf-8").splitlines()
            self.assertEqual(calls[0], "list")
            self.assertEqual(calls[-1], "list")
            self.assertEqual(calls.count("serve"), 1)

    def test_ensure_ollama_is_skipped_in_mock_mode(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            bin_dir = temp_path / "bin"
            state_dir = temp_path / "state"
            bin_dir.mkdir()
            state_dir.mkdir()
            self._write_fake_ollama(bin_dir, state_dir)

            result = self._run_make(
                extra_env={
                    "PATH": f"{bin_dir}:{os.environ['PATH']}",
                    "NICOCHAT_USE_MOCK": "true",
                }
            )

            self.assertEqual(
                result.returncode,
                0,
                f"stdout:\n{result.stdout}\n\nstderr:\n{result.stderr}",
            )
            self.assertNotIn("Starting local Ollama service", result.stdout)
            self.assertFalse((state_dir / "calls.log").exists())


if __name__ == "__main__":
    unittest.main()
