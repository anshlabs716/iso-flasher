use std::{
    env,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::{fd::AsRawFd, raw::c_void},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

mod devices;
mod helpers;
mod portal;
mod select;
mod session;

static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn stop(_signal: i32) {
    RUNNING.store(false, Ordering::SeqCst);
}

const BUFFER_SIZE: usize = 4 * 1024 * 1024;
const SNAKE_WIDTH: i32 = 42;
const SNAKE_HEIGHT: i32 = 14;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const MAGENTA: &str = "\x1b[35m";

const SIGINT: i32 = 2;
const POLLIN: i16 = 1;
const BLKGETSIZE64: u64 = 0x8008_1272;
const BLKFLSBUF: u64 = 0x1261;

unsafe extern "C" {
    fn geteuid() -> u32;
    fn sync();
    fn ioctl(fd: i32, request: u64, arg: *mut u64) -> i32;
    fn signal(signal: i32, handler: usize) -> usize;
    fn poll(fds: *mut PollFd, count: usize, timeout: i32) -> i32;
    fn read(fd: i32, buffer: *mut c_void, count: usize) -> isize;
}

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Point {
    x: i32,
    y: i32,
}

struct Snake {
    body: Vec<Point>,
    direction: Point,
    food: Point,
    score: u32,
    seed: u32,
}

impl Snake {
    fn new() -> Self {
        let mut snake = Self {
            body: vec![
                Point {
                    x: SNAKE_WIDTH / 2,
                    y: SNAKE_HEIGHT / 2,
                },
                Point {
                    x: SNAKE_WIDTH / 2 - 1,
                    y: SNAKE_HEIGHT / 2,
                },
                Point {
                    x: SNAKE_WIDTH / 2 - 2,
                    y: SNAKE_HEIGHT / 2,
                },
            ],
            direction: Point { x: 1, y: 0 },
            food: Point { x: 0, y: 0 },
            score: 0,
            seed: 0x51A9_C0DE,
        };
        snake.place_food();
        snake
    }

    fn random(&mut self) -> u32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed
    }

    fn occupies(&self, point: Point) -> bool {
        self.body.contains(&point)
    }

    fn place_food(&mut self) {
        for _ in 0..500 {
            let point = Point {
                x: (self.random() % SNAKE_WIDTH as u32) as i32,
                y: (self.random() % SNAKE_HEIGHT as u32) as i32,
            };
            if !self.occupies(point) {
                self.food = point;
                return;
            }
        }
    }

    fn turn(&mut self, direction: Point) {
        if direction.x != -self.direction.x || direction.y != -self.direction.y {
            self.direction = direction;
        }
    }

    fn step(&mut self) {
        let head = self.body[0];
        let next = Point {
            x: head.x + self.direction.x,
            y: head.y + self.direction.y,
        };

        if next.x < 0
            || next.x >= SNAKE_WIDTH
            || next.y < 0
            || next.y >= SNAKE_HEIGHT
            || self.occupies(next)
        {
            *self = Self::new();
            return;
        }

        let ate_food = next == self.food;
        self.body.insert(0, next);

        if ate_food {
            self.score += 1;
            self.place_food();
        } else {
            self.body.pop();
        }
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn raw() -> io::Result<Self> {
        let status = Command::new("stty")
            .args(["-icanon", "-echo", "min", "0", "time", "0"])
            .status()?;

        if !status.success() {
            return Err(io::Error::other("failed to configure terminal"));
        }

        print!("\x1b[?25l");
        io::stdout().flush()?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = Command::new("stty").arg("sane").status();
        print!("\x1b[?25h{RESET}");
        let _ = io::stdout().flush();
    }
}

fn clear_screen() {
    print!("\x1b[2J\x1b[3J\x1b[H");
}

fn device_size(path: &Path) -> io::Result<u64> {
    let file = OpenOptions::new().read(true).open(path)?;
    let mut size = 0_u64;

    let result = unsafe { ioctl(file.as_raw_fd(), BLKGETSIZE64, &mut size) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(size)
    }
}

fn prompt(message: &str) -> io::Result<String> {
    print!("{message}");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_owned())
}

fn read_key() -> Option<u8> {
    let mut poll_fd = PollFd {
        fd: io::stdin().as_raw_fd(),
        events: POLLIN,
        revents: 0,
    };

    if unsafe { poll(&mut poll_fd, 1, 0) } <= 0 || poll_fd.revents & POLLIN == 0 {
        return None;
    }

    let mut byte = [0_u8; 1];
    if unsafe { read(poll_fd.fd, byte.as_mut_ptr() as *mut c_void, 1) } == 1 {
        if byte[0] == 3 {
            RUNNING.store(false, Ordering::SeqCst);
        }
        Some(byte[0])
    } else {
        None
    }
}

fn title_screen() {
    clear_screen();
    println!("{MAGENTA}{BOLD}╔══════════════════════════════════════════════╗{RESET}");
    println!("{MAGENTA}{BOLD}║              ISO-FLASHER 2.0                ║{RESET}");
    println!("{MAGENTA}{BOLD}╚══════════════════════════════════════════════╝{RESET}\n");
    println!("{CYAN}Fast, lightweight ISO-to-USB flashing for Linux.{RESET}\n");
    println!(
        "File chooser: {CYAN}{}{RESET}\n",
        select::describe_backend()
    );
    println!("The next two steps choose what to flash and where:");
    println!("  1. Pick the USB drive from the numbered list");
    println!("  2. Pick the ISO image in your desktop file dialog\n");
    println!("{YELLOW}Do not select a partition such as /dev/sdb1.{RESET}");
    println!("{YELLOW}Select the whole device, such as /dev/sdb.{RESET}\n");
    println!("{GREEN}No filesystem scanning. No custom browser. Just pick and flash.{RESET}\n");
    println!("{CYAN}Ctrl+C or 'q' cancels at any prompt.{RESET}");
    println!("Press Enter to list removable devices.");
    let _ = io::stdout().flush();
    let _ = prompt("");
}

/// Resolve both the ISO and the target device.
fn interactive() -> io::Result<(PathBuf, PathBuf)> {
    title_screen();

    let device = select::choose_device(None).map_err(selection_to_io)?;
    println!(
        "\n{CYAN}Target{RESET}  {BOLD}/dev/{}{RESET}  {}",
        device.name,
        device.describe()
    );

    let iso = select::choose_iso().map_err(selection_to_io)?;
    Ok((iso, device.path))
}

fn selection_to_io(error: select::SelectionError) -> io::Error {
    match error {
        select::SelectionError::Cancelled => {
            io::Error::new(io::ErrorKind::Interrupted, "cancelled by user")
        }
        other => io::Error::other(other.to_string()),
    }
}

#[derive(Default)]
struct Cli {
    iso: Option<PathBuf>,
    device: Option<PathBuf>,
    force: bool,
}

fn parse_args() -> Cli {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let mut cli = Cli::default();
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "-i" | "--iso" => {
                index += 1;
                if let Some(value) = arguments.get(index) {
                    cli.iso = Some(PathBuf::from(value));
                }
            }
            "-d" | "--device" => {
                index += 1;
                if let Some(value) = arguments.get(index) {
                    cli.device = Some(PathBuf::from(value));
                }
            }
            "-f" | "--force" => cli.force = true,
            "-h" | "--help" => {
                println!(
                    "iso-flasher 2.0.0\n\n\
                     Usage: sudo iso-flasher [--iso IMAGE --device /dev/sdX] [--force]\n\n\
                     ISO files are chosen with the desktop's own file chooser via the\n\
                     XDG Desktop Portal, so KDE, GNOME, Xfce, Cinnamon, MATE, Budgie,\n\
                     LXQt and COSMIC all work on Wayland and X11 without extra packages.\n\
                     kdialog/zenity/yad are used only if no portal is available.\n\n\
                     USB devices are not files, so they are listed from sysfs with their\n\
                     model and capacity rather than shown in a file picker. Partitions\n\
                     such as /dev/sdb1 are always rejected.\n\n\
                     Current file chooser: {}",
                    select::describe_backend()
                );
                std::process::exit(0);
            }
            argument => eprintln!("{YELLOW}Ignoring unknown argument: {argument}{RESET}"),
        }
        index += 1;
    }

    cli
}

fn validate_target(iso: &Path, device: &Path, force: bool) -> io::Result<u64> {
    let total = select::validate_iso(iso)?.metadata()?.len();

    // Reject partitions and non-device nodes, then require the device to look
    // like removable media unless the user explicitly overrides with --force.
    let info = devices::validate_target(device)?;

    if !force && !(info.removable || info.transport.as_deref() == Some("usb")) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "/dev/{} is not removable; use --force only after verifying it",
                info.name
            ),
        ));
    }

    let size = device_size(device)?;
    if size > 0 && total > size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "target device is smaller than the ISO",
        ));
    }

    Ok(total)
}

fn handle_flash_key(key: u8, snake: &mut Snake, snake_mode: &mut bool) {
    match key {
        b'w' | b'W' => snake.turn(Point { x: 0, y: -1 }),
        b's' | b'S' => snake.turn(Point { x: 0, y: 1 }),
        b'a' | b'A' => snake.turn(Point { x: -1, y: 0 }),
        b'd' | b'D' => snake.turn(Point { x: 1, y: 0 }),
        // Escape sequence introducer: read the final byte of the sequence.
        0x1b if read_key() == Some(b'[') => match read_key() {
            Some(b'A') => snake.turn(Point { x: 0, y: -1 }),
            Some(b'B') => snake.turn(Point { x: 0, y: 1 }),
            Some(b'C') => snake.turn(Point { x: 1, y: 0 }),
            Some(b'D') => snake.turn(Point { x: -1, y: 0 }),
            Some(b'Z') => *snake_mode = !*snake_mode,
            _ => {}
        },
        _ => {}
    }
}

fn draw_snake(snake: &Snake, percentage: f64) {
    clear_screen();
    println!(
        "{GREEN}{BOLD}🐍 SNAKE{RESET}  Flash {:>5.1}%  Score {}\n",
        percentage, snake.score
    );

    for y in 0..SNAKE_HEIGHT {
        print!("|");
        for x in 0..SNAKE_WIDTH {
            let point = Point { x, y };
            let character = if snake.food == point {
                '@'
            } else if snake.body[0] == point {
                'O'
            } else if snake.body.iter().skip(1).any(|segment| *segment == point) {
                'o'
            } else {
                ' '
            };
            print!("{character}");
        }
        println!("|");
    }

    println!("\nWASD/arrows move • Shift+Tab flash view");
}

fn write_all_buffer(output: &mut File, buffer: &[u8]) -> io::Result<()> {
    let mut written = 0;

    while written < buffer.len() {
        let count = output.write(&buffer[written..])?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "device stopped accepting data",
            ));
        }
        written += count;
    }

    Ok(())
}

fn flush_device(output: &File) -> io::Result<()> {
    output.sync_all()?;

    let result = unsafe { ioctl(output.as_raw_fd(), BLKFLSBUF, std::ptr::null_mut()) };
    if result < 0 {
        let error = io::Error::last_os_error();
        if !matches!(error.raw_os_error(), Some(22 | 25)) {
            return Err(error);
        }
    }

    Ok(())
}

fn flash(iso: &Path, device: &Path, total: u64) -> io::Result<()> {
    let mut input = File::open(iso)?;
    let mut output = OpenOptions::new().write(true).open(device)?;
    let _terminal = TerminalGuard::raw()?;

    let mut buffer = vec![0_u8; BUFFER_SIZE];
    let mut snake = Snake::new();
    let mut snake_mode = false;
    let mut last_snake_update = Instant::now();
    let start = Instant::now();
    let mut written = 0_u64;

    while RUNNING.load(Ordering::SeqCst) {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }

        write_all_buffer(&mut output, &buffer[..count])?;
        written += count as u64;

        while let Some(key) = read_key() {
            handle_flash_key(key, &mut snake, &mut snake_mode);
        }

        let elapsed = start.elapsed().as_secs_f64().max(0.001);
        let speed = written as f64 / 1_048_576.0 / elapsed;
        let percentage = (written as f64 * 100.0 / total as f64).min(100.0);
        let remaining = total.saturating_sub(written);
        let eta = if speed > 0.0 {
            remaining as f64 / 1_048_576.0 / speed
        } else {
            0.0
        };

        if snake_mode {
            if last_snake_update.elapsed() >= Duration::from_millis(120) {
                snake.step();
                last_snake_update = Instant::now();
            }
            draw_snake(&snake, percentage);
        } else {
            clear_screen();
            println!("{CYAN}{BOLD}ISO FLASHER{RESET}\n");
            println!("Device  {BOLD}{}{RESET}", device.display());
            println!("Image   {}", iso.display());
            println!(
                "\n[{:<42}] {:>5.1}%\n",
                "=".repeat((percentage * 42.0 / 100.0) as usize),
                percentage
            );
            println!("Speed   {:>7.1} MiB/s", speed);
            println!(
                "ETA     {:02}m {:02}s",
                (eta as u64) / 60,
                (eta as u64) % 60
            );
            println!("\n{YELLOW}Shift+Tab{RESET} Snake    Ctrl+C Cancel");
            println!("{YELLOW}Press Ctrl+C to cancel the flash at any time.{RESET}");
        }

        io::stdout().flush()?;
    }

    if !RUNNING.load(Ordering::SeqCst) {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "flash cancelled",
        ));
    }

    if written != total {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!("ISO was not completely written ({written} of {total} bytes)"),
        ));
    }

    flush_device(&output)
}

fn run() -> io::Result<()> {
    unsafe {
        signal(SIGINT, stop as *const () as usize);
    }

    // Parse arguments first so `--help` works without root.
    let cli = parse_args();

    if unsafe { geteuid() } != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "root access is required to write to a block device; run with sudo",
        ));
    }

    RUNNING.store(true, Ordering::SeqCst);
    let (iso, device) = match (cli.iso, cli.device) {
        (Some(iso), Some(device)) => (iso, device),
        _ => interactive()?,
    };

    let total = validate_target(&iso, &device, cli.force)?;

    println!("{RED}{BOLD}This will erase {}.{RESET}", device.display());
    if prompt("Type FLASH to continue: ")? != "FLASH" {
        println!("{YELLOW}Aborted; nothing was written.{RESET}");
        return Ok(());
    }

    select::unmount(&device)?;
    unsafe { sync() };

    flash(&iso, &device, total)
}

fn main() {
    clear_screen();

    if let Err(error) = run() {
        // A user-cancelled selection is a normal outcome, not a failure.
        if error.kind() == io::ErrorKind::Interrupted {
            eprintln!("{YELLOW}Cancelled.{RESET} Nothing was written.");
            std::process::exit(0);
        }

        eprintln!("{RED}FAIL:{RESET} {error}");
        std::process::exit(1);
    }

    println!("{GREEN}DONE:{RESET} ISO successfully written.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_does_not_reverse_direction() {
        let mut snake = Snake::new();
        snake.turn(Point { x: -1, y: 0 });
        assert_eq!(snake.direction, Point { x: 1, y: 0 });
    }

    #[test]
    fn snake_turns_when_direction_is_valid() {
        let mut snake = Snake::new();
        snake.turn(Point { x: 0, y: -1 });
        assert_eq!(snake.direction, Point { x: 0, y: -1 });
    }

    #[test]
    fn cli_defaults_are_empty() {
        let cli = Cli::default();
        assert!(cli.iso.is_none());
        assert!(cli.device.is_none());
        assert!(!cli.force);
    }
}
