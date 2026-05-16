#!/usr/bin/env python3

import os
import sys
import shutil
import logging
import pathlib
import datetime
import tkinter as tk
from tkinter import filedialog
from zoneinfo import ZoneInfo
from dotenv import load_dotenv

# ==========================================
# 1. DIRECTORY CONFIGURATION & ENV LOADING
# ==========================================

APP_DATA_DIR = pathlib.Path.home() / ".coffee_edu_mailer"
APP_DATA_DIR.mkdir(parents=True, exist_ok=True)

if getattr(sys, 'frozen', False):
    BUNDLE_DIR = pathlib.Path(sys._MEIPASS)
else:
    SRC_DIR = pathlib.Path(__file__).resolve().parent
    BUNDLE_DIR = SRC_DIR.parent

POINTER_FILE = APP_DATA_DIR / "workspace_path.txt"

def get_or_create_workspace() -> pathlib.Path:
    if POINTER_FILE.exists():
        with open(POINTER_FILE, "r") as f:
            return pathlib.Path(f.read().strip())
            
    root = tk.Tk()
    root.withdraw() 
    
    # Se agrega initialdir para forzar la apertura en la carpeta del usuario
    chosen_dir = filedialog.askdirectory(
        initialdir=pathlib.Path.home(),
        title="Elige dónde guardar los boletines y excels"
    )
    
    root.destroy()
    
    if not chosen_dir:
        sys.exit("Debes seleccionar una carpeta para continuar.")
        
    chosen_path = pathlib.Path(chosen_dir) / "CoffeeEduMailer_Workspace"
    chosen_path.mkdir(parents=True, exist_ok=True)
    
    with open(POINTER_FILE, "w") as f:
        f.write(str(chosen_path))
        
    return chosen_path

WORKSPACE_DIR = get_or_create_workspace()

def ensure_env_file() -> None:
    env_path = APP_DATA_DIR / ".env"
    env_example_path = BUNDLE_DIR / ".env.example"
    
    if not env_path.exists():
        if env_example_path.exists():
            shutil.copyfile(env_example_path, env_path)

ensure_env_file()
load_dotenv(dotenv_path=APP_DATA_DIR / ".env")

VERBOSE = os.getenv("VERBOSE", "false").lower() == "true"
DEBUG = os.getenv("DEBUG", "false").lower() == "true"
DEVELOPMENT = os.getenv("DEVELOPMENT", "false").lower() == "true"
COUNTRY = os.getenv("COUNTRY", "America/Bogota")
APP_NAME = os.getenv("APP_NAME", "CoffeeEduMailer")
APP_VERSION = os.getenv("APP_VERSION", "0.1.0")
DEFAULT_GEOMETRY_STR = os.getenv("DEFAULT_GEOMETRY", "1920x1080")
DEFAULT_GEOMETRY = tuple(map(int, DEFAULT_GEOMETRY_STR.split("x")))

LOGS_DIR = APP_DATA_DIR / "logs"
LOGS_DIR.mkdir(parents=True, exist_ok=True)

def get_time_now(naming: bool = False) -> str:
    tz = ZoneInfo(COUNTRY)
    now = datetime.datetime.now(tz)
    
    if naming:
        return now.strftime("%d_%m_%y_%H_%M_%S")
        
    return now.isoformat(sep=" ", timespec="seconds")

def is_linux() -> bool:
    """Devuelve True si el sistema operativo es Linux."""
    return sys.platform.startswith("linux")

# ==========================================
# 2. LOGGER CONFIGURATION
# ==========================================

class SimpleFormatter(logging.Formatter):
    def format(self, record: logging.LogRecord) -> str:
        if record.exc_info: 
            record.levelname = "EXCEPTION"
        record.levelname = f"{record.levelname:<9}"
        return super().format(record)

def cleanup_old_logs(max_files=50):
    """Borra los logs más viejos si hay 50 o más."""
    logs = sorted(LOGS_DIR.glob("*.log")) # Se ordenan alfabéticamente (por fecha en el nombre)
    while len(logs) >= max_files:
        logs.pop(0).unlink()

def set_logger() -> logging.Logger:
    try: 
        name = pathlib.Path(sys.argv[0]).stem.upper()
    except Exception: 
        name = "APP"

    logger = logging.getLogger(name)
    logger.setLevel(logging.DEBUG) 
    
    fmt = SimpleFormatter("%(asctime)s [%(name)s]%(levelname)s %(message)s", datefmt="%Y-%m-%d %H:%M:%S")
    tz = ZoneInfo(COUNTRY)
    fmt.converter = lambda *args: datetime.datetime.now(tz).timetuple()

    # Si el logger no tiene handlers, se los agregamos
    if not logger.handlers:
        # 1. Handler para la consola
        console_level = logging.DEBUG if DEBUG else (logging.INFO if VERBOSE else logging.ERROR)
        c_handler = logging.StreamHandler(sys.stdout)
        c_handler.setLevel(console_level)
        c_handler.setFormatter(fmt)
        logger.addHandler(c_handler)

        # 2. Handler para el archivo (Siempre guarda en DEBUG para tener todo el rastro)
        cleanup_old_logs(50) # Limpiamos antes de crear el nuevo
        log_filename = datetime.datetime.now(tz).strftime("%Y-%m-%d_%H-%M-%S.log")
        f_handler = logging.FileHandler(LOGS_DIR / log_filename, mode='w', encoding='utf-8')
        f_handler.setLevel(logging.DEBUG)
        f_handler.setFormatter(fmt)
        logger.addHandler(f_handler)

    return logger

log = set_logger()
log.info(f"Starting {APP_NAME} v{APP_VERSION}...")

if __name__ == "__main__":
    log.info(f"  - RUTAS DEL SISTEMA (Ocultas): {APP_DATA_DIR}")
    log.info(f"  - ESPACIO DE USUARIO (Limpio): {WORKSPACE_DIR}")
