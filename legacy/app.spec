# -*- mode: python ; coding: utf-8 -*-

import PyInstaller.utils.hooks

customtkinter_datas = PyInstaller.utils.hooks.collect_data_files('customtkinter')

a = Analysis(
    ['src/app.py'],
    pathex=[],
    binaries=[],
    datas=[
        ('.env.example', '.'),
        ('assets/app.png', 'assets'),
    ] + customtkinter_datas,
    hiddenimports=['PIL._tkinter_finder'],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
    optimize=0,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='MochaEduCardReporter',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    icon='assets/app.ico',
    version='resources/windows_version_info.txt',
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)
