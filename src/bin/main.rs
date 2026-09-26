#![no_std]
#![no_main]

use core::fmt::Write;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    main,
    uart::{Config, Uart},
};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

const WIDTH: usize = 28;
const HEIGHT: usize = 12;
const MAX_BULLETS: usize = 4;
const NUM_ALIENS: usize = 12;

#[derive(Clone, Copy)]
struct Bullet {
    x: usize,
    y: usize,
    active: bool,
}

#[derive(Clone, Copy)]
struct Alien {
    x: usize,
    y: usize,
    alive: bool,
}

#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let delay = Delay::new();

    // UART0: communicate with PC Terminal via Micro-USB port (GPIO1 TX, GPIO3 RX)
    let mut uart0 = Uart::new(peripherals.UART0, Config::default())
        .unwrap()
        .with_rx(peripherals.GPIO3)
        .with_tx(peripherals.GPIO1);

    // Onboard LED (GPIO2)
    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());

    // Onboard BOOT push button (GPIO0)
    let boot_button = Input::new(
        peripherals.GPIO0,
        InputConfig::default().with_pull(Pull::Up),
    );

    // Clear screen once at startup and hide terminal blinking cursor (\x1b[?25l)
    let _ = write!(uart0, "\x1b[2J\x1b[H\x1b[?25l");

    let mut player_x = WIDTH / 2;
    let player_y = HEIGHT - 1;

    let mut bullets = [Bullet {
        x: 0,
        y: 0,
        active: false,
    }; MAX_BULLETS];

    let mut aliens = [Alien {
        x: 0,
        y: 0,
        alive: true,
    }; NUM_ALIENS];

    // Initialize 2 rows of alien invaders
    let init_aliens = |aliens: &mut [Alien; NUM_ALIENS]| {
        let mut idx = 0;
        for row in 0..2 {
            for col in 0..6 {
                aliens[idx] = Alien {
                    x: 3 + col * 4,
                    y: 1 + row * 2,
                    alive: true,
                };
                idx += 1;
            }
        }
    };
    init_aliens(&mut aliens);

    let mut alien_dir: i32 = 1;
    let mut alien_move_counter = 0;
    let mut alien_move_speed = 6; // Move every 6 ticks
    let mut score: u32 = 0;
    let mut wave: u32 = 1;
    let mut game_over = false;
    let mut hit_led_timer = 0;
    let mut last_boot_btn = Level::High;

    let mut rx_buf = [0u8; 16];
    let mut escape_state: u8 = 0;

    loop {
        // --- 1. RECEIVE USER INPUT FROM PC (UART) AND BOOT BUTTON ---
        let mut fire_bullet = false;

        // Non-blocking UART read
        if let Ok(count) = uart0.read_buffered(&mut rx_buf) {
            for &b in &rx_buf[..count] {
                // Decode ANSI Escape Sequences for arrow keys (\x1b[D: Left, \x1b[C: Right, \x1b[A: Up)
                if b == 0x1b {
                    escape_state = 1;
                    continue;
                } else if escape_state == 1 && b == b'[' {
                    escape_state = 2;
                    continue;
                } else if escape_state == 2 {
                    escape_state = 0;
                    match b {
                        b'D' => { // LEFT arrow
                            if player_x > 1 {
                                player_x -= 1;
                            }
                        }
                        b'C' => { // RIGHT arrow
                            if player_x < WIDTH - 2 {
                                player_x += 1;
                            }
                        }
                        b'A' => { // UP arrow (fire laser)
                            fire_bullet = true;
                        }
                        _ => {}
                    }
                    continue;
                }
                escape_state = 0;

                match b {
                    b'a' | b'A' | b'j' | b'4' => {
                        if player_x > 1 {
                            player_x -= 1;
                        }
                    }
                    b'd' | b'l' | b'6' => {
                        if player_x < WIDTH - 2 {
                            player_x += 1;
                        }
                    }
                    b' ' | b'w' | b'W' | b'k' => {
                        fire_bullet = true;
                    }
                    b'r' | b'R' => {
                        if game_over {
                            game_over = false;
                            score = 0;
                            wave = 1;
                            player_x = WIDTH / 2;
                            alien_move_speed = 6;
                            init_aliens(&mut aliens);
                            for b in bullets.iter_mut() {
                                b.active = false;
                            }
                            led.set_level(Level::Low);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Read onboard BOOT push button
        let btn_val = boot_button.level();
        if last_boot_btn == Level::High && btn_val == Level::Low {
            fire_bullet = true;
        }
        last_boot_btn = btn_val;

        // Spawn bullet if fired
        if fire_bullet && !game_over {
            for b in bullets.iter_mut() {
                if !b.active {
                    b.active = true;
                    b.x = player_x;
                    b.y = player_y - 1;
                    break;
                }
            }
        }

        // --- 2. UPDATE GAME STATE & PHYSICS ---
        if !game_over {
            // Update bullet positions
            for b in bullets.iter_mut() {
                if b.active {
                    if b.y > 0 {
                        b.y -= 1;

                        // Check collision between bullet and alien
                        for a in aliens.iter_mut() {
                            if a.alive && a.x == b.x && a.y == b.y {
                                a.alive = false;
                                b.active = false;
                                score += 20;
                                hit_led_timer = 2; // Flash LED on hit!
                                break;
                            }
                        }
                    } else {
                        b.active = false;
                    }
                }
            }

            // Manage hit LED flash
            if hit_led_timer > 0 {
                led.set_level(Level::High);
                hit_led_timer -= 1;
            } else {
                led.set_level(Level::Low);
            }

            // Move alien invaders
            alien_move_counter += 1;
            if alien_move_counter >= alien_move_speed {
                alien_move_counter = 0;

                // Check if any alien reaches the boundary wall
                let mut need_drop = false;
                for a in aliens.iter() {
                    if a.alive {
                        if (alien_dir == 1 && a.x >= WIDTH - 2) || (alien_dir == -1 && a.x <= 1) {
                            need_drop = true;
                            break;
                        }
                    }
                }

                if need_drop {
                    alien_dir = -alien_dir;
                    for a in aliens.iter_mut() {
                        if a.alive {
                            a.y += 1;
                            if a.y >= player_y {
                                game_over = true;
                                led.set_level(Level::High);
                            }
                        }
                    }
                } else {
                    for a in aliens.iter_mut() {
                        if a.alive {
                            if alien_dir == 1 {
                                a.x += 1;
                            } else {
                                a.x -= 1;
                            }
                        }
                    }
                }

                // Check if all aliens are defeated (Stage Clear)
                let remaining = aliens.iter().filter(|a| a.alive).count();
                if remaining == 0 {
                    wave += 1;
                    score += 100;
                    init_aliens(&mut aliens);
                    if alien_move_speed > 2 {
                        alien_move_speed -= 1; // Increase speed in next wave
                    }
                }
            }
        }

        // --- 3. RENDER FRAME (IN-PLACE ANSI REPAINT - ZERO FLICKER) ---
        let mut screen = [[b' '; WIDTH]; HEIGHT];

        if !game_over {
            // Draw player ship
            screen[player_y][player_x] = b'A';

            // Draw active bullets
            for b in bullets.iter() {
                if b.active && b.y < HEIGHT && b.x < WIDTH {
                    screen[b.y][b.x] = b'|';
                }
            }

            // Draw living aliens
            for a in aliens.iter() {
                if a.alive && a.y < HEIGHT && a.x < WIDTH {
                    screen[a.y][a.x] = b'W';
                }
            }
        }

        // Write directly to UART with ANSI codes:
        // \x1b[H : Move cursor to (1,1) instead of clearing screen -> 100% flicker-free!
        let _ = write!(uart0, "\x1b[H");
        let _ = write!(
            uart0,
            "\r\n\x1b[1;36m+--- RETRO SPACE INVADERS ---+\x1b[0m\r\n"
        );
        let _ = write!(
            uart0,
            "\x1b[1;33m| SCORE: {:05}   WAVE: {:02}   |\x1b[0m\r\n",
            score, wave
        );
        let _ = write!(uart0, "\x1b[1;36m+----------------------------+\x1b[0m\r\n");

        if game_over {
            for row in 0..HEIGHT {
                if row == HEIGHT / 2 - 1 {
                    let _ = write!(uart0, "|\x1b[1;31m      *** GAME OVER ***     \x1b[0m|\r\n");
                } else if row == HEIGHT / 2 {
                    let _ = write!(uart0, "|\x1b[1;32m   Press [R] to Play Again  \x1b[0m|\r\n");
                } else {
                    let _ = write!(uart0, "|                            |\r\n");
                }
            }
        } else {
            for row in 0..HEIGHT {
                let _ = write!(uart0, "|");
                for col in 0..WIDTH {
                    let ch = screen[row][col];
                    if ch == b'W' {
                        // Alien: Magenta/Red
                        let _ = write!(uart0, "\x1b[1;35mW\x1b[0m");
                    } else if ch == b'|' {
                        // Bullet: Bright Yellow
                        let _ = write!(uart0, "\x1b[1;33m|\x1b[0m");
                    } else if ch == b'A' {
                        // Player Ship: Neon Green
                        let _ = write!(uart0, "\x1b[1;32mA\x1b[0m");
                    } else {
                        let _ = write!(uart0, " ");
                    }
                }
                let _ = write!(uart0, "|\r\n");
            }
        }

        let _ = write!(uart0, "\x1b[1;36m+----------------------------+\x1b[0m\r\n");
        let _ = write!(
            uart0,
            "\x1b[0;37m| [A]/[D] or [<-]/[->]: Move |\x1b[0m\r\n"
        );
        let _ = write!(
            uart0,
            "\x1b[0;37m| [SPACE] or BOOT Pin: Fire  |\x1b[0m\r\n"
        );

        delay.delay_millis(40); // ~25 FPS smooth loop
    }
}
