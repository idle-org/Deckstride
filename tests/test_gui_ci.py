import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("gui_ci", ROOT / "scripts/gui_ci.py")
gui = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gui)


@unittest.skipUnless(os.name == "posix", "Xvfb wrapper is for POSIX CI workers")
class GuiCiTests(unittest.TestCase):
    def test_missing_server_or_command_fails_explicitly(self):
        with self.assertRaises(ValueError):
            gui.run([])
        with patch.object(gui.shutil, "which", return_value=None), self.assertRaisesRegex(RuntimeError, "uv cannot install the X server"):
            gui.run([sys.executable, "-c", "pass"])

    def test_child_receives_isolated_display_and_exit_status_survives(self):
        with tempfile.TemporaryDirectory() as directory:
            server = Path(directory) / "Xvfb"
            server.write_text(f"#!{sys.executable}\nimport os,sys,signal\nos.write(int(sys.argv[2]),b'47\\n')\nsignal.pause()\n")
            server.chmod(0o700)
            command = [sys.executable, "-c", "import os,sys; assert os.environ['DISPLAY']==':47'; assert os.environ['PDF_PRESENTER_GUI_TESTS']=='1'; assert 'WAYLAND_DISPLAY' not in os.environ; sys.exit(7)"]
            with patch.object(gui.shutil, "which", return_value=str(server)):
                self.assertEqual(gui.run(command), 7)
            # Invalid readiness cannot run the requested command.
            server.write_text(f"#!{sys.executable}\nimport os,sys\nos.write(int(sys.argv[2]),b'invalid\\n')\n")
            with patch.object(gui.shutil, "which", return_value=str(server)), self.assertRaises(RuntimeError):
                gui.run(command)
