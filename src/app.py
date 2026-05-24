from pathlib import Path
import os
import tempfile
from tkinter import filedialog, messagebox

import customtkinter as ctk

import cfg
from libs.excel_analyzer import GradebookProcessor


log = cfg.set_logger()

status_message: ctk.StringVar | None = None
detail_message: ctk.StringVar | None = None
result_message: ctk.StringVar | None = None
primary_button: ctk.CTkButton | None = None
progress_bar: ctk.CTkProgressBar | None = None


def set_status(message: str, detail: str = "", result: str = "") -> None:
    if status_message is not None:
        status_message.set(message)
    if detail_message is not None:
        detail_message.set(detail)
    if result_message is not None:
        result_message.set(result)
    app.update_idletasks()


def set_busy(is_busy: bool) -> None:
    if primary_button is not None:
        primary_button.configure(state="disabled" if is_busy else "normal")
    if progress_bar is not None:
        if is_busy:
            progress_bar.configure(mode="indeterminate")
            progress_bar.start()
        else:
            progress_bar.stop()
            progress_bar.configure(mode="determinate")
            progress_bar.set(0)
    app.update_idletasks()


def ask_sheet_selection(sheet_names: list[str]) -> str | None:
    dialog = ctk.CTkToplevel(app)
    dialog.title("Selección de hoja")
    dialog.geometry("420x230")
    dialog.resizable(False, False)
    dialog.transient(app)
    dialog.attributes("-topmost", True)
    dialog.grab_set()

    selected_sheet = ctk.StringVar(value=sheet_names[0])
    result: list[str | None] = [None]

    container = ctk.CTkFrame(dialog, fg_color="transparent")
    container.pack(expand=True, fill="both", padx=28, pady=26)

    ctk.CTkLabel(
        container,
        text="Hoja del archivo",
        font=("Roboto", 22, "bold"),
        anchor="w",
    ).pack(fill="x")
    ctk.CTkLabel(
        container,
        text="Selecciona la hoja que quieres procesar.",
        font=("Roboto", 13),
        text_color="#9CA3AF",
        anchor="w",
    ).pack(fill="x", pady=(4, 18))
    ctk.CTkOptionMenu(
        container,
        values=sheet_names,
        variable=selected_sheet,
        width=340,
        height=42,
        font=("Roboto", 14),
    ).pack(fill="x")

    actions = ctk.CTkFrame(container, fg_color="transparent")
    actions.pack(fill="x", pady=(24, 0))

    def confirm() -> None:
        result[0] = selected_sheet.get()
        dialog.destroy()

    ctk.CTkButton(actions, text="Cancelar", command=dialog.destroy, width=110, fg_color="#374151").pack(side="left")
    ctk.CTkButton(actions, text="Aceptar", command=confirm, width=140).pack(side="right")

    app.wait_window(dialog)

    if result[0] is None:
        log.info("Sheet selection cancelled by user.")

    return result[0]


def choose_source_file() -> Path | None:
    file_path = filedialog.askopenfilename(
        parent=app,
        title="Selecciona un archivo de notas",
        initialdir=str(cfg.WORKSPACE_DIR),
        filetypes=[
            ("Archivos de notas", "*.xls *.xlsx *.csv"),
            ("Excel", "*.xls *.xlsx"),
            ("CSV", "*.csv"),
            ("Todos los archivos", "*.*"),
        ],
    )
    return Path(file_path).resolve() if file_path else None


def choose_sheet(source_file: Path) -> str | None:
    if source_file.suffix.lower() == ".csv":
        return None

    sheet_names = GradebookProcessor.get_sheet_names(str(source_file))
    if not sheet_names:
        details = ""
        if source_file.suffix.lower() == ".xls":
            details = "\n\nTip: para archivos .xls debes tener instalada la librería 'xlrd'."

        messagebox.showerror(
            "Archivo no válido",
            f"No se pudieron leer hojas del archivo seleccionado.{details}",
        )
        log.error("Could not read sheet names from file: %s", source_file)
        return None

    return ask_sheet_selection(sheet_names)


def choose_output_dir() -> Path | None:
    output_dir = filedialog.askdirectory(
        parent=app,
        title="Elige la carpeta donde guardar los PDFs",
        initialdir=str(cfg.WORKSPACE_DIR),
    )
    return Path(output_dir).resolve() if output_dir else None


def generate_pdfs() -> None:
    log.info("Starting one-click PDF generation flow.")
    set_status("Seleccionando archivo...", "", "")

    source_file = choose_source_file()
    if source_file is None:
        set_status("Listo para generar PDFs.", "No se seleccionó ningún archivo.")
        return

    set_status("Seleccionando hoja...", source_file.name)
    sheet_name = choose_sheet(source_file)
    if source_file.suffix.lower() != ".csv" and sheet_name is None:
        set_status("Listo para generar PDFs.", f"Archivo: {source_file.name}")
        return

    detail = f"Archivo: {source_file.name}"
    if sheet_name:
        detail = f"{detail}\nHoja: {sheet_name}"

    set_status("Seleccionando carpeta de salida...", detail)
    output_dir = choose_output_dir()
    if output_dir is None:
        set_status("Listo para generar PDFs.", detail, "No se seleccionó carpeta de salida.")
        return

    try:
        cfg.ensure_writable_dir(output_dir)
    except Exception as exc:
        log.exception("Could not create output directory: %s", output_dir)
        messagebox.showerror("Carpeta no válida", f"No se pudo crear la carpeta:\n{output_dir}\n\n{exc}")
        set_status("No se pudo crear la carpeta de salida.", detail)
        return

    temp_root = cfg.get_local_temp_dir()
    if temp_root is not None:
        try:
            cfg.ensure_writable_dir(temp_root)
        except Exception as exc:
            log.exception("Could not create local temp directory: %s", temp_root)
            messagebox.showerror(
                "Carpeta temporal no válida",
                f"No se pudo crear la carpeta temporal:\n{temp_root}\n\n{exc}",
            )
            set_status("No se pudo crear la carpeta temporal.", detail)
            return

    processor = GradebookProcessor(str(source_file), sheet_name=sheet_name, logger=log)
    set_busy(True)
    set_status("Generando PDFs...", f"{detail}\nSalida: {output_dir}")

    previous_cwd = Path.cwd()
    try:
        temp_dir_kwargs = {"prefix": "coffee_edu_mailer_"}
        if temp_root is not None:
            temp_dir_kwargs["dir"] = str(temp_root)
        with tempfile.TemporaryDirectory(**temp_dir_kwargs) as temp_dir:
            os.chdir(temp_dir)
            success = processor.run_pipeline(
                output_dir=str(output_dir),
                timestamp_iso=cfg.get_time_now(),
                use_parallel=cfg.is_linux(),
            )
    except Exception as exc:
        log.exception("PDF generation failed.")
        messagebox.showerror("Error generando PDFs", f"No se pudieron generar los PDFs.\n\n{exc}")
        set_status("Error generando PDFs.", detail)
        return
    finally:
        os.chdir(previous_cwd)
        set_busy(False)

    if not success:
        messagebox.showerror(
            "Error generando PDFs",
            "No se pudieron generar los PDFs. Revisa el archivo seleccionado y los logs.",
        )
        set_status("No se pudieron generar los PDFs.", detail)
        return

    pdf_count = len(processor.pdf_files)
    result = f"{pdf_count} PDF{'s' if pdf_count != 1 else ''} en {output_dir}"
    set_status("PDFs generados correctamente.", detail, result)
    log.info("Generated %s PDFs in %s", pdf_count, output_dir)
    messagebox.showinfo(
        "PDFs generados",
        (
            f"Archivo: {source_file.name}\n"
            f"Hoja: {sheet_name or 'CSV'}\n"
            f"PDFs generados: {pdf_count}\n\n"
            f"Carpeta:\n{output_dir}"
        ),
    )


# ==========================================
# CONFIGURACIÓN DE INTERFAZ GRÁFICA (GUI)
# ==========================================

log.info(f"Starting {cfg.APP_NAME} v{cfg.APP_VERSION} in {cfg.COUNTRY} time: {cfg.get_time_now()}")

ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("blue")

app = ctk.CTk()
log.info("Application initialized successfully.")
app.title(cfg.APP_NAME)
app.geometry(cfg.DEFAULT_GEOMETRY_STR)
app.minsize(780, 520)
app.configure(fg_color="#0F172A")

if not cfg.is_linux():
    app.after(0, lambda: app.state("zoomed"))
log.info("Setting up the GUI...")

app.grid_columnconfigure(0, weight=1)
app.grid_rowconfigure(0, weight=1)

main_frame = ctk.CTkFrame(app, fg_color="#111827", corner_radius=0)
main_frame.grid(row=0, column=0, sticky="nsew")
main_frame.grid_columnconfigure(0, weight=1)
main_frame.grid_rowconfigure(0, weight=1)

content = ctk.CTkFrame(main_frame, fg_color="transparent")
content.grid(row=0, column=0, sticky="nsew", padx=48, pady=44)
content.grid_columnconfigure(0, weight=1)
content.grid_rowconfigure(1, weight=1)

header = ctk.CTkFrame(content, fg_color="transparent")
header.grid(row=0, column=0, sticky="ew")
header.grid_columnconfigure(0, weight=1)

ctk.CTkLabel(
    header,
    text=cfg.APP_NAME,
    font=("Roboto", 38, "bold"),
    text_color="#F8FAFC",
    anchor="w",
).grid(row=0, column=0, sticky="w")

ctk.CTkLabel(
    header,
    text="Boletines PDF para archivos de notas",
    font=("Roboto", 16),
    text_color="#94A3B8",
    anchor="w",
).grid(row=1, column=0, sticky="w", pady=(4, 0))

workspace_label = ctk.CTkLabel(
    header,
    text=f"Workspace: {cfg.WORKSPACE_DIR}",
    font=("Roboto", 12),
    text_color="#64748B",
    anchor="e",
)
workspace_label.grid(row=0, column=1, rowspan=2, sticky="e", padx=(24, 0))

action_panel = ctk.CTkFrame(content, fg_color="#172033", border_color="#253247", border_width=1, corner_radius=18)
action_panel.grid(row=1, column=0, sticky="nsew", pady=(36, 0))
action_panel.grid_columnconfigure(0, weight=1)

ctk.CTkLabel(
    action_panel,
    text="Generación de boletines",
    font=("Roboto", 28, "bold"),
    text_color="#E5E7EB",
).grid(row=0, column=0, pady=(46, 8))

ctk.CTkLabel(
    action_panel,
    text="Archivo → hoja → carpeta → PDFs",
    font=("Roboto", 15),
    text_color="#94A3B8",
).grid(row=1, column=0)

primary_button = ctk.CTkButton(
    action_panel,
    text="Generar PDFs",
    command=generate_pdfs,
    width=260,
    height=58,
    corner_radius=14,
    font=("Roboto", 18, "bold"),
    fg_color="#2563EB",
    hover_color="#1D4ED8",
)
primary_button.grid(row=2, column=0, pady=(36, 28))

progress_bar = ctk.CTkProgressBar(action_panel, width=340, height=10, corner_radius=8)
progress_bar.grid(row=3, column=0, pady=(0, 28))
progress_bar.set(0)

status_message = ctk.StringVar(value="Listo para generar PDFs.")
detail_message = ctk.StringVar(value="Selecciona el archivo de notas y la carpeta de salida en el flujo.")
result_message = ctk.StringVar(value="")

ctk.CTkLabel(
    action_panel,
    textvariable=status_message,
    font=("Roboto", 18, "bold"),
    text_color="#F8FAFC",
    wraplength=620,
).grid(row=4, column=0, padx=40, pady=(0, 8))

ctk.CTkLabel(
    action_panel,
    textvariable=detail_message,
    font=("Roboto", 13),
    text_color="#CBD5E1",
    wraplength=680,
    justify="center",
).grid(row=5, column=0, padx=40)

ctk.CTkLabel(
    action_panel,
    textvariable=result_message,
    font=("Roboto", 14, "bold"),
    text_color="#86EFAC",
    wraplength=680,
    justify="center",
).grid(row=6, column=0, padx=40, pady=(14, 46))

log.info("GUI setup complete. Entering main event loop.")
app.mainloop()
log.info("Application closed.")
