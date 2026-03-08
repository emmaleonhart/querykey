"""
Build script to package the Python backend into a standalone executable using PyInstaller.

Usage:
    python build_backend.py

Output:
    dist/tojo-backend/tojo-backend.exe  (one-dir mode for faster startup)
"""

import subprocess
import sys
import os

ROOT_DIR = os.path.dirname(os.path.abspath(__file__))
BACKEND_DIR = os.path.join(ROOT_DIR, "backend")
SERVER_SCRIPT = os.path.join(BACKEND_DIR, "server.py")


def main():
    # Ensure PyInstaller is installed
    try:
        import PyInstaller  # noqa: F401
    except ImportError:
        print("[build] Installing PyInstaller...")
        subprocess.check_call([sys.executable, "-m", "pip", "install", "pyinstaller"])

    print("[build] Packaging backend with PyInstaller...")

    cmd = [
        sys.executable, "-m", "PyInstaller",
        "--name", "tojo-backend",
        "--distpath", os.path.join(ROOT_DIR, "dist", "backend"),
        "--workpath", os.path.join(ROOT_DIR, "build", "pyinstaller"),
        "--specpath", os.path.join(ROOT_DIR, "build"),
        # One-dir mode (faster startup than one-file)
        "--noconfirm",
        "--clean",
        # Hidden imports that PyInstaller may not auto-detect
        "--hidden-import", "uvicorn.logging",
        "--hidden-import", "uvicorn.loops",
        "--hidden-import", "uvicorn.loops.auto",
        "--hidden-import", "uvicorn.protocols",
        "--hidden-import", "uvicorn.protocols.http",
        "--hidden-import", "uvicorn.protocols.http.auto",
        "--hidden-import", "uvicorn.protocols.websockets",
        "--hidden-import", "uvicorn.protocols.websockets.auto",
        "--hidden-import", "uvicorn.lifespan",
        "--hidden-import", "uvicorn.lifespan.on",
        "--hidden-import", "uvicorn.lifespan.off",
        "--hidden-import", "sqlalchemy.dialects.sqlite",
        "--hidden-import", "sqlalchemy.dialects.postgresql",
        "--hidden-import", "sqlalchemy.dialects.mysql",
        # Collect all backend subpackages
        "--collect-submodules", "backend",
        # Add the project root to the path so 'backend' package resolves
        "--paths", ROOT_DIR,
        # Entry point
        SERVER_SCRIPT,
    ]

    result = subprocess.run(cmd, cwd=ROOT_DIR)

    if result.returncode == 0:
        print("[build] Backend packaged successfully!")
        print(f"[build] Output: dist/backend/tojo-backend/tojo-backend.exe")
    else:
        print("[build] PyInstaller failed.", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
