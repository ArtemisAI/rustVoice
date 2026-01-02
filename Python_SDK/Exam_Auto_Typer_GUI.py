import tkinter as tk
from tkinter import ttk, messagebox
import pyautogui
import keyboard
import threading
import time
import random
import pyperclip
import sys

# Disable pyautogui's failsafe (corner of screen will not terminate)
pyautogui.FAILSAFE = True

# --- Configuration & Theme ---
COLORS = {
    'bg': '#1e1e1e',
    'fg': '#e0e0e0',
    'accent': '#00ADB5',  # Turquoise
    'accent_hover': '#00ced1',
    'secondary': '#393E46',
    'text_bg': '#2b2b2b',
    'danger': '#FF2E63',
    'success': '#00FF7F'
}

# --- Auto-Typer Logic ---
class AutoTyper:
    def __init__(self):
        self.running = False
        self.paused = False
        self.stop_requested = False
        self.text_to_type = ""
        self.speed_cpm = 1200  # Default chars per minute
        self.jitter = 0.05      # Random deviation percentage
        self.typing_thread = None

    def start_typing(self, text, speed_cpm, progress_callback, finish_callback):
        if self.running:
            return
        
        self.text_to_type = text
        self.speed_cpm = speed_cpm
        self.running = True
        self.paused = False
        self.stop_requested = False
        
        self.typing_thread = threading.Thread(target=self._type_loop, args=(progress_callback, finish_callback), daemon=True)
        self.typing_thread.start()

    def pause(self):
        self.paused = True

    def resume(self):
        self.paused = False

    def stop(self):
        self.stop_requested = True
        self.running = False
        self.paused = False

    def _type_loop(self, progress_callback, finish_callback):
        # Countdown
        for i in range(5, 0, -1):
            if self.stop_requested: break
            progress_callback(0, f"Starting in {i}s... SWITCH WINDOW!")
            time.sleep(1)
        
        if self.stop_requested:
            finish_callback(False)
            return

        progress_callback(0, "Typing...")
        
        total_chars = len(self.text_to_type)
        base_delay = 60.0 / self.speed_cpm
        
        chars_typed = 0
        
        for char in self.text_to_type:
            # Check for Stop/Abort
            if self.stop_requested or keyboard.is_pressed('esc'):
                self.stop_requested = True
                break
            
            # Check for Pause
            while self.paused:
                if self.stop_requested: break
                time.sleep(0.1)
                progress_callback(chars_typed, "Paused")
            
            if self.stop_requested: break

            # Type the character
            try:
                # Handle special characters or newlines if needed
                if char == '\n':
                    pyautogui.press('enter')
                else:
                    # Use clipboard + paste for better compatibility with accented characters
                    pyperclip.copy(char)
                    pyautogui.hotkey('ctrl', 'v')
                
                chars_typed += 1
                
                # Update progress every 5 chars to avoid UI lag
                if chars_typed % 5 == 0:
                    progress_callback(chars_typed, f"Typing... {int((chars_typed/total_chars)*100)}%")

                # Calculate sleep with jitter
                delay = base_delay * random.uniform(1.0 - self.jitter, 1.0 + self.jitter)
                time.sleep(delay)

            except Exception as e:
                print(f"Error typing: {e}")
                break

        self.running = False
        self.stop_requested = False 
        finish_callback(True)


# --- GUI Application ---
class AutoTyperApp:
    def __init__(self, root):
        self.root = root
        self.root.title("SIA2000 Exam Auto-Typer")
        self.root.geometry("600x500")
        self.root.configure(bg=COLORS['bg'])
        self.root.attributes("-topmost", True) # Keep on top initially (optional, maybe distracting)

        self.typer = AutoTyper()
        
        self.setup_ui()
        self.setup_bindings()

    def setup_ui(self):
        # Style Configuration
        style = ttk.Style()
        style.theme_use('clam')
        style.configure("Dark.TFrame", background=COLORS['bg'])
        style.configure("Dark.TLabel", background=COLORS['bg'], foreground=COLORS['fg'], font=('Segoe UI', 10))
        style.configure("Header.TLabel", background=COLORS['bg'], foreground=COLORS['accent'], font=('Segoe UI', 14, 'bold'))
        
        # Main Container
        main_frame = ttk.Frame(self.root, style="Dark.TFrame", padding=20)
        main_frame.pack(fill=tk.BOTH, expand=True)

        # Header
        header = ttk.Label(main_frame, text="Exam Auto-Typer", style="Header.TLabel")
        header.pack(pady=(0, 10))

        # Text Area
        text_frame = ttk.Frame(main_frame, style="Dark.TFrame")
        text_frame.pack(fill=tk.BOTH, expand=True, pady=5)
        
        self.text_area = tk.Text(text_frame, bg=COLORS['text_bg'], fg=COLORS['fg'], 
                                 insertbackground='white', font=('Consolas', 10), height=10, borderwidth=0, padx=10, pady=10)
        self.text_area.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        
        scrollbar = ttk.Scrollbar(text_frame, orient=tk.VERTICAL, command=self.text_area.yview)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        self.text_area.configure(yscrollcommand=scrollbar.set)

        # Controls Container
        controls_frame = ttk.Frame(main_frame, style="Dark.TFrame")
        controls_frame.pack(fill=tk.X, pady=10)

        # Clipboard Button
        self.btn_paste = tk.Button(controls_frame, text="📋 Load Clipboard", command=self.load_clipboard,
                                   bg=COLORS['secondary'], fg=COLORS['fg'], relief=tk.FLAT, activebackground=COLORS['accent'])
        self.btn_paste.pack(side=tk.LEFT, padx=(0, 10))

        # Speed Slider
        speed_frame = ttk.Frame(controls_frame, style="Dark.TFrame")
        speed_frame.pack(side=tk.RIGHT, fill=tk.X)
        
        ttk.Label(speed_frame, text="Speed (CPM):", style="Dark.TLabel").pack(side=tk.LEFT)
        self.speed_var = tk.IntVar(value=1500)
        self.speed_slider = tk.Scale(speed_frame, from_=300, to=5000, orient=tk.HORIZONTAL, 
                                     variable=self.speed_var, bg=COLORS['bg'], fg=COLORS['fg'], 
                                     highlightthickness=0, length=150)
        self.speed_slider.pack(side=tk.LEFT, padx=5)

        # Action Buttons Area
        actions_frame = ttk.Frame(main_frame, style="Dark.TFrame")
        actions_frame.pack(fill=tk.X, pady=10)

        self.btn_play = tk.Button(actions_frame, text="▶ START (5s)", command=self.start_typing,
                                  bg=COLORS['accent'], fg='#1e1e1e', font=('Segoe UI', 11, 'bold'), 
                                  relief=tk.FLAT, activebackground=COLORS['accent_hover'], width=15)
        self.btn_play.pack(side=tk.LEFT, expand=True, fill=tk.X, padx=5)

        self.btn_pause = tk.Button(actions_frame, text="⏸ PAUSE", command=self.toggle_pause, state=tk.DISABLED,
                                   bg=COLORS['secondary'], fg=COLORS['fg'], font=('Segoe UI', 11, 'bold'), 
                                   relief=tk.FLAT, width=10)
        self.btn_pause.pack(side=tk.LEFT, padx=5)

        self.btn_stop = tk.Button(actions_frame, text="⏹ STOP", command=self.stop_typing, state=tk.DISABLED,
                                  bg=COLORS['danger'], fg='white', font=('Segoe UI', 11, 'bold'), 
                                  relief=tk.FLAT, width=10)
        self.btn_stop.pack(side=tk.LEFT, padx=5)

        # Status Bar
        self.status_var = tk.StringVar(value="Ready. Press ESC to Abort at any time.")
        self.status_bar = ttk.Label(main_frame, textvariable=self.status_var, style="Dark.TLabel", anchor=tk.W)
        self.status_bar.pack(fill=tk.X, pady=(10, 0))

        self.progress_var = tk.DoubleVar(value=0)
        self.progress_bar = ttk.Progressbar(main_frame, variable=self.progress_var, maximum=100)
        self.progress_bar.pack(fill=tk.X, pady=5)

    def setup_bindings(self):
        # Global hotkey listener thread is not ideal in Tkinter without care, 
        # but keyboard module hooks are non-blocking usually.
        # We handle ESC inside the typing loop for safety.
        pass

    def load_clipboard(self):
        try:
            content = pyperclip.paste()
            self.text_area.delete("1.0", tk.END)
            self.text_area.insert("1.0", content)
            self.status_var.set(f"Loaded {len(content)} characters from clipboard.")
        except Exception as e:
            self.status_var.set(f"Error loading clipboard: {e}")

    def start_typing(self):
        content = self.text_area.get("1.0", tk.END).strip() # Remove trailing newline from Text widget
        if not content:
            messagebox.showwarning("Empty", "Please enter or paste text to type.")
            return

        self.set_state_typing()
        self.typer.start_typing(content, self.speed_var.get(), self.update_progress, self.on_typing_finished)

    def toggle_pause(self):
        if self.typer.paused:
            self.typer.resume()
            self.btn_pause.config(text="⏸ PAUSE", bg=COLORS['secondary'])
            self.status_var.set("Resuming...")
        else:
            self.typer.pause()
            self.btn_pause.config(text="▶ RESUME", bg=COLORS['accent'])
            self.status_var.set("Paused.")

    def stop_typing(self):
        self.typer.stop()
        self.status_var.set("Stopping...")
        self.set_state_ready()

    def update_progress(self, chars_typed, status_text):
        # Must be thread-safe for Tkinter - use root.after() to run in main thread
        def _update():
            total = len(self.typer.text_to_type)
            if total > 0:
                pct = (chars_typed / total) * 100
                self.progress_var.set(pct)
            else:
                self.progress_var.set(0)
            
            self.status_var.set(status_text)
        
        self.root.after(0, _update)

    def on_typing_finished(self, success):
        def _finish():
            self.set_state_ready()
            if success:
                self.status_var.set("Typing Completed Successfully.")
                self.progress_var.set(100)
            else:
                self.status_var.set("Typing Aborted/Stopped.")
        
        self.root.after(0, _finish)

    def set_state_typing(self):
        self.btn_play.config(state=tk.DISABLED, bg=COLORS['secondary'])
        self.btn_pause.config(state=tk.NORMAL, text="⏸ PAUSE", bg=COLORS['secondary'])
        self.btn_stop.config(state=tk.NORMAL, bg=COLORS['danger'])
        self.text_area.config(state=tk.DISABLED, bg='#333333')

    def set_state_ready(self):
        self.btn_play.config(state=tk.NORMAL, bg=COLORS['accent'])
        self.btn_pause.config(state=tk.DISABLED, text="⏸ PAUSE", bg=COLORS['secondary'])
        self.btn_stop.config(state=tk.DISABLED, bg=COLORS['secondary'])
        self.text_area.config(state=tk.NORMAL, bg=COLORS['text_bg'])


if __name__ == "__main__":
    try:
        root = tk.Tk()
        app = AutoTyperApp(root)
        root.mainloop()
    except KeyboardInterrupt:
        sys.exit()
