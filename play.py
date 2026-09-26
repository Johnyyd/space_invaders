#!/usr/bin/env python3
"""
Zero-Latency Full-Duplex Retro Arcade Terminal
Directly bridges PC keyboard inputs to ESP32 with <0.1ms latency and zero local echo.
"""

import os
import sys
import glob
import tty
import termios
import select

BAUD = termios.B115200

def find_serial_port():
    if len(sys.argv) > 1:
        return sys.argv[1]
    
    ports = glob.glob("/dev/ttyUSB*") + glob.glob("/dev/ttyACM*")
    if ports:
        return sorted(ports)[-1] # Use the most recent port (e.g. /dev/ttyUSB1 if ttyUSB0 is busy)
    return "/dev/ttyUSB0"

def main():
    port = find_serial_port()

    if not os.path.exists(port):
        print(f"Error: Serial port '{port}' not found.")
        print("Please check your ESP32 USB cable connection!")
        sys.exit(1)

    # 1. Open serial port directly using low-level O_RDWR (bypassing libc buffering)
    try:
        fd_port = os.open(port, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
    except Exception as e:
        print(f"Cannot open '{port}': {e}")
        print("Please ensure other processes (like espflash) have released the port.")
        sys.exit(1)

    # Configure serial port: 115200 baud, 8N1, Raw mode, No echo
    attr = termios.tcgetattr(fd_port)
    attr[0] = 0  # iflag: raw input
    attr[1] = 0  # oflag: raw output
    attr[2] = termios.CS8 | termios.CREAD | termios.CLOCAL  # cflag
    attr[3] = 0  # lflag: no echo, non-canonical
    attr[4] = BAUD  # ispeed
    attr[5] = BAUD  # ospeed
    attr[6][termios.VMIN] = 1
    attr[6][termios.VTIME] = 0
    termios.tcsetattr(fd_port, termios.TCSANOW, attr)

    # 2. Set terminal stdin to RAW mode (instant key registration, echo disabled)
    fd_stdin = sys.stdin.fileno()
    old_stdin_attr = termios.tcgetattr(fd_stdin)

    print("\x1b[2J\x1b[H")  # Clear screen
    print(f"=====================================================")
    print(f"   ESP32 RETRO ARCADE TERMINAL READY on [{port}]    ")
    print(f"   - [A] / [D] or [<-] / [->] : Move Ship            ")
    print(f"   - [Space] or [^]           : Fire Laser           ")
    print(f"   - [R]                      : Restart on Game Over ")
    print(f"   - [Ctrl + C]               : Exit Game            ")
    print(f"=====================================================\r\n")

    try:
        tty.setraw(fd_stdin)

        while True:
            # Multiplex stdin and serial port
            rlist, _, _ = select.select([fd_stdin, fd_port], [], [])

            # Keyboard input from user
            if fd_stdin in rlist:
                key_bytes = os.read(fd_stdin, 64)
                if not key_bytes:
                    break
                # Handle Ctrl + C for graceful exit
                if b'\x03' in key_bytes:
                    break
                # Direct kernel write to serial port - Zero Latency!
                os.write(fd_port, key_bytes)

            # Frame buffer stream from ESP32
            if fd_port in rlist:
                screen_bytes = os.read(fd_port, 4096)
                if screen_bytes:
                    os.write(sys.stdout.fileno(), screen_bytes)

    except KeyboardInterrupt:
        pass
    finally:
        # Restore original terminal attributes
        termios.tcsetattr(fd_stdin, termios.TCSADRAIN, old_stdin_attr)
        os.close(fd_port)
        print("\x1b[?25h\r\n\nGame exited. Terminal state restored.")

if __name__ == "__main__":
    main()
