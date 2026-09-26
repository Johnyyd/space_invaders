# Retro Space Invaders for ESP32 (8-Bit Arcade)

A classic 1980s retro space shooter game engine running bare-metal (`no_std`) in Rust on the **ESP32 SoC (Xtensa Dual-Core LX6 @ 240MHz)**, rendered directly on your Linux terminal via ANSI VT100 escape codes with **zero screen flicker** and **sub-millisecond input response**.

---

## 🎮 Game Preview

```text
+--- RETRO SPACE INVADERS ---+
| SCORE: 00120   WAVE: 01    |
+----------------------------+
|                            |
|    W   W   W   W   W   W   |
|                            |
|    W   W   W   W   W   W   |
|                            |
|              |             |
|                            |
|              A             |
+----------------------------+
| [A]/[D] or [<-]/[->]: Move |
| [SPACE] or BOOT Pin: Fire  |
```

---

## ✨ Key Features

- **Zero External Peripherals Required**:
  - The game logic, enemy AI, wave progression, and physics run entirely on the ESP32.
  - Uses the **Onboard LED (`GPIO2`)** for visual feedback (flashes upon hitting an alien, solid on Game Over).
  - Uses the **Onboard BOOT Button (`GPIO0`)** as a physical arcade trigger.
- **Dual Control Schemes**:
  - Control your ship using your computer keyboard (`A`/`D`, Arrow keys, `Space`).
  - Or tap the physical **BOOT button** on the ESP32 board to fire lasers.
- **Flicker-Free 25 FPS ANSI Rendering**:
  - Instead of clearing the entire screen (`\x1b[2J`), it uses in-place repainting with cursor homing (`\x1b[H`), completely eliminating terminal flicker.
- **Zero-Latency Terminal Driver (`play.py`)**:
  - A lightweight, pure-Python bridge leveraging low-level Unix system calls (`termios`, `tty.setraw()`, `select.select()`, and unbuffered `os.write(2)`).
  - Bypasses standard libc stream buffers to provide instant key registration and suppress unwanted local echo.
  - Features **automatic serial port detection** (detects `/dev/ttyUSB0`, `/dev/ttyUSB1`, etc.).

---

## 🕹️ Controls

| Action | Computer Keyboard | ESP32 Hardware |
| :--- | :--- | :--- |
| **Move Left** | `A` or `←` (Left Arrow) or `J` | — |
| **Move Right** | `D` or `→` (Right Arrow) or `L` | — |
| **Fire Laser** | `Space` or `↑` (Up Arrow) or `W` | **BOOT button** (`GPIO0`) |
| **Restart Game** | `R` (after Game Over) | — |
| **Exit** | `Ctrl + C` | — |

---

## 🚀 How to Build & Play

### 1. Build and Flash Firmware

Connect your ESP32 to your computer via a Micro-USB cable, then run:

```bash
# Activate the Xtensa Rust environment
source $HOME/export-esp.sh

# Navigate to the game folder
cd /home/tringuyen/Documents/GitHub/rust/space_invaders

# Build in release mode and flash to ESP32
cargo run --release
```

Once `espflash` completes flashing (`Flashing has completed!`), press **`Ctrl + C`** to release the serial port.

---

### 2. Launch the Game Terminal

Run the zero-latency game runner:

```bash
python3 play.py
```

> **Tip**: If you plugged your ESP32 into a different USB port, `play.py` automatically detects active ports, or you can specify it manually:
> ```bash
> python3 play.py /dev/ttyUSB1
> ```

---

## 📂 Project Structure

```
space_invaders/
├── Cargo.toml            # Rust dependencies (esp-hal ~1.2.2 with esp32 feature)
├── play.py               # Zero-latency interactive terminal bridge (Python standard library)
├── play_game.sh          # Launcher wrapper script
├── README.md             # Project documentation
└── src/
    └── bin/
        └── main.rs       # Complete bare-metal game engine, collision physics, and ANSI renderer
```

---

## 🛠️ Technical Details

- **Microcontroller**: ESP32 (Revision v3.1, Dual Core @ 240MHz, 4MB Flash).
- **HAL**: [`esp-hal v1.2.2`](https://github.com/esp-rs/esp-hal).
- **UART Configuration**: `UART0` routed to hardware pins `GPIO1` (TX) and `GPIO3` (RX) at 115,200 baud.
- **Input Handling**: Non-blocking hardware FIFO reads via `uart0.read_buffered()`.
- **Target Architecture**: `xtensa-esp32-none-elf` (bare-metal `no_std`).
