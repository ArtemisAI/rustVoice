import tkinter as tk
from tkinter import ttk, messagebox
import keyboard
import pyperclip
import threading
import time
import random
import sys

# Disable fail-safe if using pyautogui (we are using keyboard mostly, but careful with loops)
# keyboard module doesn't have a fail-safe like pyautogui, so we rely on explicit stop checks

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
        self.mode = "Natural" # Natural, Turbo, Block
        self.speed_cpm = 1200
        self.jitter = 0.1
        self.typing_thread = None

    def start_typing(self, text, mode, speed_cpm, progress_callback, finish_callback):
        if self.running: return
        
        self.text_to_type = text
        self.mode = mode
        self.speed_cpm = speed_cpm
        self.running = True
        self.paused = False
        self.stop_requested = False
        
        self.typing_thread = threading.Thread(
            target=self._type_loop, 
            args=(progress_callback, finish_callback), 
            daemon=True
        )
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
        
        try:
            if self.mode == "Turbo (Instant Paste)":
                self._mode_turbo()
                progress_callback(100, "Paste Complete.")
            
            elif self.mode == "Block (Line-by-Line)":
                self._mode_block(progress_callback)
            
            else: # Natural (Keystrokes)
                self._mode_natural(progress_callback)

            success = not self.stop_requested
        except Exception as e:
            print(f"Error: {e}")
            success = False

        self.running = False
        self.stop_requested = False 
        finish_callback(success)

    def _wait_paused(self):
        while self.paused:
            if self.stop_requested: return True
            time.sleep(0.1)
        return False

    def _check_stop(self):
        if self.stop_requested or keyboard.is_pressed('esc'):
            self.stop_requested = True
            return True
        return False

    def _mode_turbo(self):
        """Copies all text to clipboard and pastes once."""
        pyperclip.copy(self.text_to_type)
        time.sleep(0.1)
        keyboard.send('ctrl+v')
        time.sleep(0.1)

    def _mode_block(self, progress_callback):
        """Pastes line by line."""
        lines = self.text_to_type.split('\n')
        total_lines = len(lines)
        
        for idx, line in enumerate(lines):
            if self._check_stop(): break
            if self._wait_paused(): break # Handle pause logic if loop continues

            # Copy line to clipboard
            pyperclip.copy(line)
            time.sleep(0.05)
            keyboard.send('ctrl+v')
            time.sleep(0.05)
            keyboard.send('enter')
            
            # Progress
            pct = int(((idx + 1) / total_lines) * 100)
            progress_callback(pct, f"Pasting Line {idx+1}/{total_lines}")
            
            # Small delay between lines
            time.sleep(0.2)

    def _mode_natural(self, progress_callback):
        """Types character by character using keystrokes (No Clipboard)."""
        base_delay = 60.0 / self.speed_cpm
        total_chars = len(self.text_to_type)
        
        for idx, char in enumerate(self.text_to_type):
            if self._check_stop(): break
            if self._wait_paused(): 
                if self._check_stop(): break # Re-check after pause
            
            # Type the char directly
            keyboard.write(char)
            
            # Progress update
            if idx % 10 == 0:
                pct = int(((idx + 1) / total_chars) * 100)
                progress_callback(pct, f"Typing... {pct}%")

            # Random delay
            delay = base_delay * random.uniform(1.0 - self.jitter, 1.0 + self.jitter)
            time.sleep(delay)


# --- GUI Application ---
class AutoTyperApp:
    def __init__(self, root):
        self.root = root
        self.root.title("SIA2000 Auto-Typer v2 (Safe Mode)")
        self.root.geometry("650x550")
        self.root.configure(bg=COLORS['bg'])
        
        self.typer = AutoTyper()
        
        self.setup_ui()

    def setup_ui(self):
        style = ttk.Style()
        style.theme_use('clam')
        style.configure("Dark.TFrame", background=COLORS['bg'])
        style.configure("Dark.TLabel", background=COLORS['bg'], foreground=COLORS['fg'], font=('Segoe UI', 10))
        style.configure("Header.TLabel", background=COLORS['bg'], foreground=COLORS['accent'], font=('Segoe UI', 14, 'bold'))
        
        main_frame = ttk.Frame(self.root, style="Dark.TFrame", padding=20)
        main_frame.pack(fill=tk.BOTH, expand=True)

        # Header
        ttk.Label(main_frame, text="Auto-Typer v2", style="Header.TLabel").pack(pady=(0, 10))

        # Text Area
        text_frame = ttk.Frame(main_frame, style="Dark.TFrame")
        text_frame.pack(fill=tk.BOTH, expand=True, pady=5)
        
        self.text_area = tk.Text(text_frame, bg=COLORS['text_bg'], fg=COLORS['fg'], 
                                 insertbackground='white', font=('Consolas', 10), height=10, 
                                 borderwidth=0, padx=10, pady=10)
        self.text_area.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        
        scrollbar = ttk.Scrollbar(text_frame, orient=tk.VERTICAL, command=self.text_area.yview)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        self.text_area.configure(yscrollcommand=scrollbar.set)

        # Controls
        controls_frame = ttk.Frame(main_frame, style="Dark.TFrame")
        controls_frame.pack(fill=tk.X, pady=10)

        # Left: Clipboard & Mode
        left_controls = ttk.Frame(controls_frame, style="Dark.TFrame")
        left_controls.pack(side=tk.LEFT)

        tk.Button(left_controls, text="📋 Paste Clipboard", command=self.load_clipboard,
                  bg=COLORS['secondary'], fg=COLORS['fg'], relief=tk.FLAT).pack(side=tk.LEFT, padx=(0, 10))

        ttk.Label(left_controls, text="Mode:", style="Dark.TLabel").pack(side=tk.LEFT)
        self.mode_var = tk.StringVar(value="Natural (Keystrokes)")
        self.mode_combo = ttk.Combobox(left_controls, textvariable=self.mode_var, 
                                       values=["Natural (Keystrokes)", "Turbo (Instant Paste)", "Block (Line-by-Line)"],
                                       state="readonly", width=20)
        self.mode_combo.pack(side=tk.LEFT, padx=5)

        # Right: Speed
        right_controls = ttk.Frame(controls_frame, style="Dark.TFrame")
        right_controls.pack(side=tk.RIGHT)
        
        ttk.Label(right_controls, text="Speed:", style="Dark.TLabel").pack(side=tk.LEFT)
        self.speed_var = tk.IntVar(value=1500)
        self.speed_slider = tk.Scale(right_controls, from_=300, to=5000, orient=tk.HORIZONTAL, 
                                     variable=self.speed_var, bg=COLORS['bg'], fg=COLORS['fg'], 
                                     highlightthickness=0, length=120)
        self.speed_slider.pack(side=tk.LEFT, padx=5)

        # Buttons
        actions_frame = ttk.Frame(main_frame, style="Dark.TFrame")
        actions_frame.pack(fill=tk.X, pady=10)

        self.btn_play = tk.Button(actions_frame, text="▶ START (5s delay)", command=self.start_typing,
                                  bg=COLORS['accent'], fg='#1e1e1e', font=('Segoe UI', 11, 'bold'), 
                                  relief=tk.FLAT, width=20)
        self.btn_play.pack(side=tk.LEFT, expand=True, fill=tk.X, padx=5)

        self.btn_stop = tk.Button(actions_frame, text="⏹ STOP (ESC)", command=self.stop_typing, state=tk.DISABLED,
                                  bg=COLORS['danger'], fg='white', font=('Segoe UI', 11, 'bold'), 
                                  relief=tk.FLAT, width=15)
        self.btn_stop.pack(side=tk.LEFT, padx=5)

        # Footer
        self.status_var = tk.StringVar(value="Ready. Select a mode above.")
        ttk.Label(main_frame, textvariable=self.status_var, style="Dark.TLabel").pack(fill=tk.X, pady=(10, 0))
        
        self.progress_var = tk.DoubleVar(value=0)
        ttk.Progressbar(main_frame, variable=self.progress_var, maximum=100).pack(fill=tk.X, pady=5)

    def load_clipboard(self):
        try:
            content = pyperclip.paste()
            self.text_area.delete("1.0", tk.END)
            self.text_area.insert("1.0", content)
            self.status_var.set(f"Loaded {len(content)} chars.")
        except Exception as e:
            self.status_var.set(f"Clipboard Error: {e}")

    def start_typing(self):
        content = self.text_area.get("1.0", tk.END).strip()
        if not content:
            messagebox.showwarning("Empty", "Are you typing invisible ink? Enter some text!")
            return

        self.set_state_typing()
        self.typer.start_typing(content, self.mode_var.get(), self.speed_var.get(), 
                                self.update_progress, self.on_finish)

    def stop_typing(self):
        self.typer.stop()
        self.status_var.set("Stopping...")

    def update_progress(self, pct, text):
        self.root.after(0, lambda: [self.progress_var.set(pct), self.status_var.set(text)])

    def on_finish(self, success):
        self.root.after(0, lambda: [
            self.set_state_ready(),
            self.status_var.set("Done!" if success else "Stopped.")
        ])

    def set_state_typing(self):
        self.btn_play.config(state=tk.DISABLED, bg=COLORS['secondary'])
        self.btn_stop.config(state=tk.NORMAL)
        self.text_area.config(state=tk.DISABLED, bg='#333333')
        self.mode_combo.config(state=tk.DISABLED)

    def set_state_ready(self):
        self.btn_play.config(state=tk.NORMAL, bg=COLORS['accent'])
        self.btn_stop.config(state=tk.DISABLED)
        self.text_area.config(state=tk.NORMAL, bg=COLORS['text_bg'])
        self.mode_combo.config(state="readonly")

if __name__ == "__main__":
    try:
        root = tk.Tk()
        app = AutoTyperApp(root)
        root.mainloop()
    except KeyboardInterrupt:
        sys.exit()
