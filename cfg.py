import os
from dotenv import load_dotenv

load_dotenv()

DEBUG = os.getenv("DEBUG")
VERBOSE = os.getenv("VERBOSE")
DEVELOPMENT = os.getenv("DEVELOPMENT")
MAIL_API_KEY = os.getenv("MAIL_API_KEY")
