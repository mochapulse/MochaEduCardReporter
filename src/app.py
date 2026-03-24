from pathlib import Path
from tkinter import filedialog, messagebox
import customtkinter as ctk
import cfg
from libs.excel_analyzer import GradebookProcessor

log = cfg.set_logger()

selected_excel_file: str | None = None
selected_sheet_name: str | None = None

def ask_sheet_selection(sheet_names: list[str]) -> str | None:
    dialog = ctk.CTkToplevel(app)
    dialog.title("Selección de hoja")
    left_geometry = str(cfg.DEFAULT_GEOMETRY[0] // 4)
    right_geometry = str(cfg.DEFAULT_GEOMETRY[1] // 4)
    dialog.geometry(f"{left_geometry}x{right_geometry}")
    dialog.attributes("-topmost", True)
    dialog.grab_set()

    selected_sheet = ctk.StringVar(value=sheet_names[0])
    resultado = [None]

    ctk.CTkLabel(dialog, text="Elige la hoja que deseas usar:", font=("Roboto", 16)).pack(pady=(20, 10))
    ctk.CTkOptionMenu(dialog, values=sheet_names, variable=selected_sheet, width=250, height=40, font=("Roboto", 14)).pack(pady=10)

    def confirmar():
        resultado[0] = selected_sheet.get()
        dialog.destroy()

    ctk.CTkButton(dialog, text="Aceptar", command=confirmar, width=150, height=40, font=("Roboto", 14)).pack(pady=20)

    app.wait_window(dialog)

    if resultado[0] is None:
        log.info("Sheet selection cancelled by user (window closed).")
    
    return resultado[0]

def load_excel():
    global selected_excel_file, selected_sheet_name

    log.info("Loading Excel file...")
    file_path = filedialog.askopenfilename(
        parent=app,
        title="Selecciona un archivo de notas",
        initialdir=str(cfg.WORKSPACE_DIR),
        filetypes=[("Excel files", "*.xls *.xlsx")],
    )

    if not file_path:
        log.info("No Excel file was selected.")
        return

    selected_excel_file = str(Path(file_path).resolve())
    log.info("Excel selected: %s", selected_excel_file)

    sheet_names = GradebookProcessor.get_sheet_names(selected_excel_file)
    if not sheet_names:
        file_suffix = Path(selected_excel_file).suffix.lower()
        details = ""
        if file_suffix == ".xls":
            details = "\n\nTip: para archivos .xls debes tener instalada la libreria 'xlrd'."

        messagebox.showerror(
            "Archivo no valido",
            f"No se pudieron leer hojas del archivo seleccionado.{details}",
        )
        log.error("Could not read sheet names from file: %s", selected_excel_file)
        return

    selected_sheet_name = ask_sheet_selection(sheet_names)
    if not selected_sheet_name:
        return

    log.info("Sheet selected: %s", selected_sheet_name)
    messagebox.showinfo(
        "Seleccion completada",
        (
            f"Archivo: {Path(selected_excel_file).name}\n"
            f"Hoja elegida: {selected_sheet_name}\n\n"
            "Listo. Aun no se ejecuta el analisis."
        ),
    )

def generate_report_cards():
    log.info("Generating report cards...")
    messagebox.showinfo("Pendiente", "Esta accion se implementara en el siguiente paso.")

def send_emails():
    log.info("Sending emails...")
    messagebox.showinfo("Pendiente", "Esta accion se implementara en el siguiente paso.")

# ==========================================
# CONFIGURACIÓN DE INTERFAZ GRÁFICA (GUI)
# ==========================================

log.info(f"Starting {cfg.APP_NAME} v{cfg.APP_VERSION} in {cfg.COUNTRY} time: {cfg.get_time_now()}")

# Tema moderno
ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("blue")

app = ctk.CTk()
log.info("Application initialized successfully.")
app.title(cfg.APP_NAME)
app.geometry(cfg.DEFAULT_GEOMETRY_STR)

if not cfg.is_linux():
    app.after(0, lambda: app.state('zoomed')) # Solo en Windows/Mac, en Linux se inicia maximizado por defecto
log.info("Setting up the GUI...")

# Contenedor central para agrupar elementos
main_frame = ctk.CTkFrame(app, corner_radius=20)
main_frame.place(relx=0.5, rely=0.5, anchor="center") # Centrado absoluto en la pantalla

# Títulos
ctk.CTkLabel(main_frame, text=cfg.APP_NAME, font=("Roboto", 48, "bold")).pack(pady=(50, 10), padx=100)
ctk.CTkLabel(main_frame, text="Sistema Automatizado de Boletines", font=("Roboto", 20), text_color="gray").pack(pady=(0, 40))

# Botones grandes y estilizados
btn_font = ("Roboto", 18, "bold")
ctk.CTkButton(main_frame, text="1. Cargar Excel", command=load_excel, width=350, height=60, font=btn_font).pack(pady=15)
ctk.CTkButton(main_frame, text="2. Generar Boletines", command=generate_report_cards, width=350, height=60, font=btn_font).pack(pady=15)
ctk.CTkButton(main_frame, text="3. Enviar Correos", command=send_emails, width=350, height=60, font=btn_font).pack(pady=(15, 50))

log.info("GUI setup complete. Entering main event loop.")
app.mainloop()
log.info("Application closed.")