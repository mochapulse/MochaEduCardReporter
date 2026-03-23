import customtkinter as ctk
import cfg

def load_excel():
    pass

def generate_report_cards():
    pass

def send_emails():
    pass

app = ctk.CTk()
app.title("CoffeeEduMailer")
app.geometry("400x250")

ctk.CTkButton(app, text="Load Excel", command=load_excel).pack(pady=10)
ctk.CTkButton(app, text="Generate Report Cards", command=generate_report_cards).pack(pady=10)
ctk.CTkButton(app, text="Send Emails", command=send_emails).pack(pady=10)

app.mainloop()
