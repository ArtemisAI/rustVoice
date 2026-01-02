import tkinter as tk
from tkinter import ttk, messagebox
import keyboard
import pyperclip
import threading
import time
import random
import sys
import logging
import os

# --- Logging Setup ---
if not os.path.exists('logs'): os.makedirs('logs')
logging.basicConfig(
    filename='logs/auto_typer_v4.log',
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)

COLORS = {
    'bg': '#1e1e1e',
    'fg': '#e0e0e0',
    'accent': '#00ADB5',
    'accent_hover': '#00ced1',
    'secondary': '#393E46',
    'text_bg': '#2b2b2b',
    'danger': '#FF2E63',
    'warning': '#FFB74D',
    'success': '#00FF7F'
}

NEIGHBORS = {
    'a': 'qwsz', 'b': 'vghn', 'c': 'xdfv', 'd': 'serfcx', 'e': 'wsdr', 'f': 'drtgv',
    'g': 'ftyhb', 'h': 'gyunj', 'i': 'ujko', 'j': 'hunik', 'k': 'jiolm', 'l': 'kop',
    'm': 'njk', 'n': 'bhjm', 'o': 'iklp', 'p': 'ol', 'q': 'wa', 'r': 'edft',
    's': 'awedxz', 't': 'rfgy', 'u': 'yhji', 'v': 'cfgb', 'w': 'qase', 'x': 'zsdc',
    'y': 'tghu', 'z': 'asx', ' ': ' '
}

# --- Funny Speed Labels ---
def get_speed_label(cpm):
    if cpm < 500: return "🐢 Grandma (Comfortably Slow)"
    if cpm < 1200: return "👨‍💼 Average Joe (Human)"
    if cpm < 2000: return "⚡ Pro Gamer (Fast)"
    if cpm < 3000: return "🐒 ADHD Monkey (Hyper)"
    return "🤖 Matrix Mode (Godlike)"

class AutoTyper:
    def __init__(self):
        self.running = False
        self.paused = False
        self.pause_pending = False  # For Smart Pause
        self.stop_requested = False
        self.text_to_type = ""
        self.mode = "Natural"
        self.speed_cpm = 1200
        self.jitter = 0.1
        self.typo_chance = 0.03
        
        self.typing_thread = None
        self.last_esc_time = 0
        
        # Hook ESC key globally
        keyboard.on_press_key('esc', self._on_esc_press)
        self.status_callback = None 

    def set_status_callback(self, cb):
        self.status_callback = cb

    def _on_esc_press(self, event):
        if not self.running: return
        
        curr_time = time.time()
        # Double press detection (< 500ms)
        if (curr_time - self.last_esc_time) < 0.5:
            self.stop()
            if self.status_callback: self.status_callback("STOPPED (Double ESC)", paused=False)
        else:
            self.toggle_smart_pause()
            
        self.last_esc_time = curr_time

    def toggle_smart_pause(self):
        if self.paused:
            self.paused = False
            self.pause_pending = False
            logging.info("Resumed")
            if self.status_callback: self.status_callback("RESUMED", paused=False)
        else:
            self.pause_pending = True
            logging.info("Pause Pending (Smart Pause)...")
            if self.status_callback: self.status_callback("Pausing at next space...", paused=False)

    def start_typing(self, text, mode, speed_cpm, progress_callback, finish_callback):
        if self.running: return
        
        self.text_to_type = text.replace('\r\n', '\n')
        self.mode = mode
        self.speed_cpm = speed_cpm
        self.running = True
        self.paused = False
        self.pause_pending = False
        self.stop_requested = False
        
        self.typing_thread = threading.Thread(
            target=self._type_loop, 
            args=(progress_callback, finish_callback), 
            daemon=True
        )
        self.typing_thread.start()

    def stop(self):
        self.stop_requested = True
        self.running = False
        self.paused = False
        self.pause_pending = False

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
            elif "Super-Human" in self.mode:
                self._mode_super_human(progress_callback)
            else: # Natural
                self._mode_natural(progress_callback)

            success = not self.stop_requested
        except Exception as e:
            logging.error(f"Error: {e}")
            print(e)
            success = False

        self.running = False
        self.stop_requested = False 
        finish_callback(success)

    def _handle_smart_pause(self, current_char):
        if self.stop_requested: return True
        
        # Check if we should enter paused state
        if self.pause_pending:
            # Only pause on space or newline, OR if no more text (handled elsewhere)
            if current_char in [' ', '\n', '\t']:
                self.paused = True
                self.pause_pending = False
                if self.status_callback: self.status_callback("PAUSED (Smart)", paused=True)
                
        # Wait loop while paused
        while self.paused:
            if self.stop_requested: return True
            time.sleep(0.1)
            
        return False

    def _random_sleep(self, base_delay):
        delay = base_delay * random.uniform(1.0 - self.jitter, 1.0 + self.jitter)
        time.sleep(delay)

    def _mode_turbo(self):
        pyperclip.copy(self.text_to_type)
        time.sleep(0.1)
        keyboard.send('ctrl+v')

    def _mode_block(self, progress_callback):
        lines = self.text_to_type.split('\n')
        total = len(lines)
        for idx, line in enumerate(lines):
            if self.stop_requested: break
            
            # Simple pause check between lines for block mode
            while self.paused or self.pause_pending:
                 self.paused = True; self.pause_pending = False; time.sleep(0.1); 
                 if self.stop_requested: break
            
            pyperclip.copy(line)
            time.sleep(0.05)
            keyboard.send('ctrl+v')
            time.sleep(0.05)
            if idx < total - 1:
                keyboard.send('enter')
            
            pct = int(((idx + 1) / total) * 100)
            progress_callback(pct, f"Line {idx+1}/{total}")
            time.sleep(0.3)

    def _mode_natural(self, progress_callback):
        base_delay = 60.0 / self.speed_cpm
        total = len(self.text_to_type)
        
        for idx, char in enumerate(self.text_to_type):
            if self._handle_smart_pause(char): break
            
            keyboard.write(char)
            if idx % 10 == 0:
                pct = int(((idx + 1) / total) * 100)
                progress_callback(pct, f"Typing... {pct}%")
            self._random_sleep(base_delay)

    def _mode_super_human(self, progress_callback):
        base_delay = 60.0 / self.speed_cpm
        total = len(self.text_to_type)
        
        i = 0
        while i < total:
            char = self.text_to_type[i]
            if self._handle_smart_pause(char): break
            
            if char == '\n':
                keyboard.write(char)
                think_time = random.uniform(1.0, 3.0)
                progress_callback(int(((i+1)/total)*100), "Thinking...")
                time.sleep(think_time)
                i += 1
                continue

            lower_char = char.lower()
            if (random.random() < self.typo_chance) and (lower_char in NEIGHBORS):
                typo_char = random.choice(NEIGHBORS[lower_char])
                if char.isupper(): typo_char = typo_char.upper()
                
                keyboard.write(typo_char)
                self._random_sleep(base_delay)
                time.sleep(random.uniform(0.1, 0.3)) # Reaction
                
                keyboard.send('backspace')
                time.sleep(random.uniform(0.05, 0.1))
                
                keyboard.write(char)
                logging.info(f"Typo corrected: {typo_char}->{char}")
            else:
                keyboard.write(char)
            
            if i % 10 == 0:
                pct = int(((i + 1) / total) * 100)
                progress_callback(pct, f"Human Mode... {pct}%")
            
            self._random_sleep(base_delay)
            i += 1


class AutoTyperApp:
    def __init__(self, root):
        self.root = root
        self.root.title("rustVoice (Legacy v4)")
        self.root.geometry("700x700")
        self.root.configure(bg=COLORS['bg'])
        
        self.typer = AutoTyper()
        self.typer.set_status_callback(self.on_external_status)
        
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
        header_frame = ttk.Frame(main_frame, style="Dark.TFrame")
        header_frame.pack(fill=tk.X, pady=(0, 10))
        ttk.Label(header_frame, text="rustVoice v4", style="Header.TLabel").pack(side=tk.LEFT)
        
        # Opacity
        op_frame = ttk.Frame(header_frame, style="Dark.TFrame")
        op_frame.pack(side=tk.RIGHT)
        ttk.Label(op_frame, text="Opacity:", style="Dark.TLabel").pack(side=tk.LEFT)
        self.op_slider = tk.Scale(op_frame, from_=0.1, to=1.0, resolution=0.1, orient=tk.HORIZONTAL,
                                  bg=COLORS['bg'], fg=COLORS['fg'], highlightthickness=0, length=100, 
                                  command=lambda v: self.root.attributes("-alpha", float(v)))
        self.op_slider.set(1.0)
        self.op_slider.pack(side=tk.LEFT, padx=5)

        # Text Area
        text_frame = ttk.Frame(main_frame, style="Dark.TFrame")
        text_frame.pack(fill=tk.BOTH, expand=True, pady=5)
        self.text_area = tk.Text(text_frame, bg=COLORS['text_bg'], fg=COLORS['fg'], 
                                 insertbackground='white', font=('Consolas', 10), height=10, 
                                 borderwidth=0, padx=10, pady=10)
        self.text_area.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        ttk.Scrollbar(text_frame, orient=tk.VERTICAL, command=self.text_area.yview).pack(side=tk.RIGHT, fill=tk.Y)

        # Controls Row 1
        ctrl_frame = ttk.Frame(main_frame, style="Dark.TFrame")
        ctrl_frame.pack(fill=tk.X, pady=10)
        tk.Button(ctrl_frame, text="📋 Paste", command=self.load_clipboard, bg=COLORS['secondary'], fg=COLORS['fg'], relief=tk.FLAT).pack(side=tk.LEFT, padx=5)
        tk.Button(ctrl_frame, text="🗑 Clear", command=lambda: self.text_area.delete("1.0", tk.END), bg=COLORS['secondary'], fg=COLORS['fg'], relief=tk.FLAT).pack(side=tk.LEFT, padx=5)
        
        ttk.Label(ctrl_frame, text="Mode:", style="Dark.TLabel").pack(side=tk.LEFT, padx=5)
        self.mode_var = tk.StringVar(value="Super-Human (Typo+Correct)")
        self.mode_combo = ttk.Combobox(ctrl_frame, textvariable=self.mode_var, values=["Natural (Keystrokes)", "Super-Human (Typo+Correct)", "Turbo (Instant Paste)", "Block (Line-by-Line)"], state="readonly", width=25)
        self.mode_combo.pack(side=tk.LEFT, padx=5)

        # Speed Row
        speed_frame = ttk.Frame(main_frame, style="Dark.TFrame")
        speed_frame.pack(fill=tk.X, pady=(0, 5))
        ttk.Label(speed_frame, text="Speed:", style="Dark.TLabel").pack(side=tk.LEFT)
        self.speed_var = tk.IntVar(value=1200)
        self.speed_slider = tk.Scale(speed_frame, from_=300, to=4000, orient=tk.HORIZONTAL, 
                                     variable=self.speed_var, bg=COLORS['bg'], fg=COLORS['fg'], 
                                     highlightthickness=0, length=300, command=self.update_speed_label)
        self.speed_slider.pack(side=tk.LEFT, padx=5, fill=tk.X, expand=True)
        
        # Funny Label
        self.speed_label_var = tk.StringVar(value="👨‍💼 Average Joe (Human)")
        ttk.Label(main_frame, textvariable=self.speed_label_var, style="Dark.TLabel", font=('Segoe UI', 9, 'italic')).pack(fill=tk.X, padx=50)

        # Actions
        act_frame = ttk.Frame(main_frame, style="Dark.TFrame")
        act_frame.pack(fill=tk.X, pady=15)
        
        self.btn_play = tk.Button(act_frame, text="▶ START (5s)", command=self.start_typing, bg=COLORS['accent'], fg='#1e1e1e', font=('Segoe UI', 11, 'bold'), relief=tk.FLAT, width=15)
        self.btn_play.pack(side=tk.LEFT, expand=True, fill=tk.X, padx=5)

        self.btn_pause = tk.Button(act_frame, text="⏸ PAUSE (ESC)", command=self.toggle_pause, state=tk.DISABLED, bg=COLORS['warning'], fg='#1e1e1e', font=('Segoe UI', 11, 'bold'), relief=tk.FLAT, width=15)
        self.btn_pause.pack(side=tk.LEFT, padx=5)

        self.btn_stop = tk.Button(act_frame, text="⏹ STOP (2xESC)", command=self.stop_typing, state=tk.DISABLED, bg=COLORS['danger'], fg='white', font=('Segoe UI', 11, 'bold'), relief=tk.FLAT, width=15)
        self.btn_stop.pack(side=tk.LEFT, padx=5)

        # Status
        self.status_var = tk.StringVar(value="Ready. Double-Tap ESC to Stop.")
        ttk.Label(main_frame, textvariable=self.status_var, style="Dark.TLabel").pack(fill=tk.X)
        self.progress_var = tk.DoubleVar(value=0)
        ttk.Progressbar(main_frame, variable=self.progress_var, maximum=100).pack(fill=tk.X, pady=5)

    def load_clipboard(self):
        try:
            self.text_area.delete("1.0", tk.END)
            self.text_area.insert("1.0", pyperclip.paste())
        except: pass

    def update_speed_label(self, val):
        cpm = int(val)
        self.speed_label_var.set(get_speed_label(cpm))

    def start_typing(self):
        content = self.text_area.get("1.0", tk.END).strip()
        if not content: return
        self.set_state_typing()
        self.typer.start_typing(content, self.mode_var.get(), self.speed_var.get(), self.update_progress, self.on_finish)

    def stop_typing(self):
        self.typer.stop()

    def toggle_pause(self):
        self.typer.toggle_smart_pause()
        self.update_pause_btn_state()

    def update_pause_btn_state(self):
        if self.typer.pause_pending:
            self.status_var.set("Pausing after word...")
            self.btn_pause.config(text="⏱ PAUSING...", bg=COLORS['warning'])
        elif self.typer.paused:
            self.btn_pause.config(text="▶ RESUME (ESC)", bg=COLORS['accent'])
            self.status_var.set("PAUSED")
        else:
            self.btn_pause.config(text="⏸ PAUSE (ESC)", bg=COLORS['warning'])
            self.status_var.set("Typing...")

    def update_progress(self, pct, text):
        self.root.after(0, lambda: [self.progress_var.set(pct), self.status_var.set(text)])

    def on_external_status(self, status_text, paused):
        self.root.after(0, lambda: [
            self.status_var.set(status_text),
            self.update_pause_btn_state()
        ])

    def on_finish(self, success):
        self.root.after(0, lambda: [self.set_state_ready(), self.status_var.set("Done!" if success else "Stopped.")])

    def set_state_typing(self):
        self.btn_play.config(state=tk.DISABLED)
        self.btn_pause.config(state=tk.NORMAL)
        # self.btn_pause.config(text="⏸ PAUSE (ESC)", bg=COLORS['warning']) # Reset button text
        self.btn_stop.config(state=tk.NORMAL)
        self.text_area.config(state=tk.DISABLED, bg='#333333')

    def set_state_ready(self):
        self.btn_play.config(state=tk.NORMAL)
        self.btn_pause.config(state=tk.DISABLED, text="⏸ PAUSE (ESC)", bg=COLORS['warning'])
        self.btn_stop.config(state=tk.DISABLED)
        self.text_area.config(state=tk.NORMAL, bg=COLORS['text_bg'])

if __name__ == "__main__":
    try:
        root = tk.Tk()
        app = AutoTyperApp(root)
        root.mainloop()
    except KeyboardInterrupt:
        sys.exit()
