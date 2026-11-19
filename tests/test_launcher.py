import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/install-linux-launcher.py"
spec = importlib.util.spec_from_file_location("launcher", SCRIPT)
launcher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(launcher)


class LauncherTests(unittest.TestCase):
    def test_exec_escapes_field_codes_and_shell_characters(self):
        entry = launcher.desktop_entry('/tmp/a b%f/$x`y"/pdf-presenter', '/tmp/icon.png')
        self.assertIn('%%f', entry)
        self.assertIn('\\\\$', entry)
        self.assertIn('\\\\`', entry)
        self.assertIn('StartupWMClass=pdf-presenter', entry)
        self.assertIn('" %f\n', entry)

    def test_install_from_bundle_uses_xdg_location(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            bundle = root / "portable folder"
            bundle.mkdir()
            script = bundle / SCRIPT.name
            script.write_bytes(SCRIPT.read_bytes())
            (bundle / 'pdf-presenter').touch()
            (bundle / 'pdf-presenter.png').touch()
            env = dict(os.environ, XDG_DATA_HOME=str(root / 'data'))
            subprocess.run(['python3', str(script)], env=env, check=True, capture_output=True)
            installed = root / 'data/applications/pdf-presenter.desktop'
            self.assertEqual(installed.read_text(), launcher.desktop_entry(
                bundle / 'pdf-presenter', bundle / 'pdf-presenter.png'))
            (bundle / 'pdf-presenter').unlink()
            result = subprocess.run(['python3', str(script)], env=env, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertTrue(installed.is_file())
