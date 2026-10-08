use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::{fd::AsRawFd, raw::c_void},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

static RUNNING: AtomicBool = AtomicBool::new(true);

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
const BLUE: &str = "\x1b[34m";

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

#[derive(Clone, Copy, PartialEq, Eq)]
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
                Point { x: SNAKE_WIDTH / 2, y: SNAKE_HEIGHT / 2 },
                Point { x: SNAKE_WIDTH / 2 - 1, y: SNAKE_HEIGHT / 2 },
                Point { x: SNAKE_WIDTH / 2 - 2, y: SNAKE_HEIGHT / 2 },
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

fn print_banner() {
    clear_screen();
    println!("{CYAN}{BOLD}");
    println!(" █    ██  ██████  ██████▄  ███▄ ▄███▓ ▄▄▄       ██ ▄█▀▓█████  ██▀███");
    println!(" ██  ▓██▒▒██    ▒ ▓██   ▀█ ▓██▒▀█▀ ██▒▒████▄     ██▄█▒ ▓█   ▀ ▓██ ▒ ██▒");
    println!(" ▓██  ▒██░░ ▓██▄   ▓██▀▀▀▄▄ ▓██    ▓██░▒██  ▀█▄  ▓███▄░ ▒███   ▓██ ░▄█ ▒");
    println!(" ▓▓█  ░██░  ▒   ██▒▓██    █ ▒██    ▒██ ░██▄▄▄▄██ ▓██ █▄ ▒▓█  ▄ ▒██▀▀█▄");
    println!(" ▒▒█████▓ ▒██████▒▒▒██████▀ ▒██▒   ░██▒  ▓█   ▓██▒▒██▒ █▄░▒████▒░██▓ ▒██▒");
    println!("  ░▒▓▒ ▒ ▒ ▒ ▒▓▒ ▒ ░▒ ▒ ▒  ░░ ▒░   ░  ░  ▒▒   ▓▒█░▒ ▒▒ ▓▒░░ ▒░ ░░ ▒▓ ░▒▓░");
    println!("  ░░▒░ ░ ░ ░ ░▒  ░ ░░ ░ ░ ░ ░  ░      ░   ▒   ▒▒ ░░ ░▒ ▒░ ░ ░  ░  ░▒ ░ ▒░");
    println!("   ░░░ ░ ░ ░  ░  ░  ░ ░ ░   ░      ░      ░   ▒   ░ ░░ ░    ░     ░░   ░");
    println!("     ░           ░  ░       ░             ░  ░░ ░  ░      ░  ░   ░");
    println!("                            ░{RESET}");
    println!("{MAGENTA}{BOLD}⚡ RAW CORE ENGINE // RUST v2.0.0 ⚡{RESET}\n");
}

fn memory_usage() -> (f64, f64) {
    let text = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0_u64;
    let mut available = 0_u64;

    for line in text.lines() {
        let mut fields = line.split_whitespace();
        match fields.next() {
            Some("MemTotal:") => total = fields.next().and_then(|v| v.parse().ok()).unwrap_or(0),
            Some("MemAvailable:") => {
                available = fields.next().and_then(|v| v.parse().ok()).unwrap_or(0)
            }
            _ => {}
        }
    }

    (
        (total.saturating_sub(available)) as f64 / 1_048_576.0,
        total as f64 / 1_048_576.0,
    )
}

fn print_system_profile() {
    let (used, total) = memory_usage();
    let load = fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|value| value.split_whitespace().next().map(str::to_owned))
        .unwrap_or_else(|| "?".to_owned());

    println!("{BLUE}┌─ SYSTEM ──────────────────────────────────────────────────────────────┐{RESET}");
    println!(
        "{BLUE}│{RESET} RAM {:>5.1} / {:<5.1} GB     Load {:<6}                         {BLUE}│{RESET}",
        used, total, load
    );
    println!("{BLUE}└───────────────────────────────────────────────────────────────────────┘{RESET}\n");
}

fn is_removable(name: &str) -> bool {
    fs::read_to_string(format!("/sys/block/{name}/removable"))
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
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

fn removable_devices() -> io::Result<Vec<(PathBuf, u64)>> {
    let mut devices = Vec::new();

    for entry in fs::read_dir("/sys/block")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();

        if ["loop", "ram", "zram", "dm-"].iter().any(|prefix| name.starts_with(prefix)) {
            continue;
        }

        if !is_removable(&name) {
            continue;
        }

        let path = PathBuf::from(format!("/dev/{name}"));
        if let Ok(size) = device_size(&path) {
            devices.push((path, size));
        }
    }

    devices.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(devices)
}

fn print_usb_devices() {
    println!("{MAGENTA}┌─ USB DEVICES ─────────────────────────────────────────────────────────┐{RESET}");

    match removable_devices() {
        Ok(devices) if devices.is_empty() => {
            println!("{MAGENTA}│{RESET} No removable USB drives detected.");
        }
        Ok(devices) => {
            for (index, (path, size)) in devices.iter().enumerate() {
                println!(
                    "{MAGENTA}│{RESET} {:>2}. {:<22} {:>7.2} GB",
                    index + 1,
                    path.display(),
                    *size as f64 / 1e9
                );
            }
        }
        Err(error) => {
            println!("{MAGENTA}│{RESET} Unable to enumerate USB devices: {error}");
        }
    }

    println!("{MAGENTA}│{RESET} Press {GREEN}[R]{RESET} to refresh.");
    println!("{MAGENTA}└───────────────────────────────────────────────────────────────────────┘{RESET}\n");
}

fn prompt(message: &str) -> io::Result<String> {
    print!("{message}");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_owned())
}

fn is_iso(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("iso"))
}

fn scan_directories() -> io::Result<Vec<PathBuf>> {
    let mut directories = Vec::new();
    let mut pending = vec![PathBuf::from("/")];

    while let Some(directory) = pending.pop() {
        if matches!(
            directory.to_str(),
            Some("/proc") | Some("/sys") | Some("/dev") | Some("/run")
        ) {
            continue;
        }

        directories.push(directory.clone());

        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            }
        }
    }

    directories.sort_by_key(|path| path.to_string_lossy().to_lowercase());
    directories.dedup();
    Ok(directories)
}

fn scan_isos<F>(root: &Path, mut progress: F) -> io::Result<Vec<PathBuf>>
where
    F: FnMut(usize, usize),
{
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    let mut scanned = 0;

    while let Some(directory) = pending.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => {
                scanned += 1;
                progress(scanned, found.len());
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                pending.push(path);
            } else if is_iso(&path) {
                found.push(path);
            }
        }

        scanned += 1;
        progress(scanned, found.len());
    }

    found.sort_by(|a, b| a.to_string_lossy().to_lowercase().cmp(&b.to_string_lossy().to_lowercase()));
    found.dedup();
    Ok(found)
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
        Some(byte[0])
    } else {
        None
    }
}

fn arrow_key() -> Option<u8> {
    if read_key() != Some(0x1b) || read_key() != Some(b'[') {
        return None;
    }
    read_key()
}

fn directory_picker() -> io::Result<PathBuf> {
    let directories = scan_directories()?;
    if directories.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no directories available"));
    }

    let _terminal = TerminalGuard::raw()?;
    let mut selected = 0usize;
    let mut scroll = 0usize;
    const VISIBLE: usize = 14;

    loop {
        clear_screen();
        println!("{CYAN}{BOLD}ISO DIRECTORY SELECTOR{RESET}\n");
        println!("{YELLOW}Choose a directory to scan for ISO images:{RESET}\n");

        let end = (scroll + VISIBLE).min(directories.len());
        for (index, directory) in directories[scroll..end].iter().enumerate() {
            let index = scroll + index;
            if index == selected {
                println!("{GREEN}{BOLD}  > {}{RESET}", directory.display());
            } else {
                println!("    {}", directory.display());
            }
        }

        println!(
            "\n{YELLOW}↑/↓{RESET} Browse   {YELLOW}Enter{RESET} Scan selected   {YELLOW}Esc{RESET} Cancel"
        );
        println!("{CYAN}Directory {} of {}{RESET}", selected + 1, directories.len());
        io::stdout().flush()?;

        loop {
            if let Some(key) = read_key() {
                match key {
                    10 | 13 => return Ok(directories[selected].clone()),
                    0x1b => {
                        if let Some(direction) = arrow_key() {
                            match direction {
                                b'A' if selected > 0 => {
                                    selected -= 1;
                                    if selected < scroll {
                                        scroll = selected;
                                    }
                                    break;
                                }
                                b'B' if selected + 1 < directories.len() => {
                                    selected += 1;
                                    if selected >= scroll + VISIBLE {
                                        scroll = selected - VISIBLE + 1;
                                    }
                                    break;
                                }
                                _ => {}
                            }
                        } else {
                            return Err(io::Error::new(
                                io::ErrorKind::Interrupted,
                                "ISO selection cancelled",
                            ));
                        }
                    }
                    _ => {}
                }
            } else {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn scan_screen(root: &Path) -> io::Result<Vec<PathBuf>> {
    clear_screen();
    println!("{YELLOW}{BOLD}ISO IMAGE SELECTOR{RESET}\n");
    println!("{YELLOW}Scanning {} for ISO images...{RESET}\n", root.display());
    println!("{CYAN}0 directories scanned — {GREEN}0 images found!{RESET}");
    io::stdout().flush()?;

    let mut last_update = Instant::now();
    let images = scan_isos(root, |directories, found| {
        if last_update.elapsed() >= Duration::from_millis(40) {
            print!(
                "\r{CYAN}{directories} directories scanned — {GREEN}{found} image{} found!{RESET}   ",
                if found == 1 { "" } else { "s" }
            );
            let _ = io::stdout().flush();
            last_update = Instant::now();
        }
    })?;

    println!(
        "\r{GREEN}Scan complete!{RESET}  {} image{} found!   ",
        images.len(),
        if images.len() == 1 { "" } else { "s" }
    );
    io::stdout().flush()?;
    std::thread::sleep(Duration::from_millis(500));

    Ok(images)
}

fn iso_picker() -> io::Result<PathBuf> {
    loop {
        let root = directory_picker()?;
        let images = scan_screen(&root)?;

        let _terminal = TerminalGuard::raw()?;
        let mut selected = 0usize;
        let mut scroll = 0usize;
        const VISIBLE: usize = 12;

        loop {
            clear_screen();
            println!("{CYAN}{BOLD}ISO IMAGE SELECTOR{RESET}\n");

            if images.is_empty() {
                println!("{RED}No ISO files found in {}.{RESET}\n", root.display());
                println!(
                    "{YELLOW}Press {BOLD}Tab{RESET} to choose another directory or                      {BOLD}Esc{RESET} to cancel."
                );
            } else {
                let end = (scroll + VISIBLE).min(images.len());
                println!(
                    "Found {} ISO file{} in {}:\n",
                    images.len(),
                    if images.len() == 1 { "" } else { "s" },
                    root.display()
                );

                for (offset, image) in images[scroll..end].iter().enumerate() {
                    let index = scroll + offset;
                    if index == selected {
                        println!("{GREEN}{BOLD}  > {}{RESET}", image.display());
                    } else {
                        println!("    {}", image.display());
                    }
                }

                println!(
                    "\n{YELLOW}↑/↓{RESET} Browse   {YELLOW}Enter{RESET} Select                        {YELLOW}Tab{RESET} Choose directory   {YELLOW}Esc{RESET} Cancel"
                );
            }

            io::stdout().flush()?;

            loop {
                if let Some(key) = read_key() {
                    match key {
                        9 => break,
                        10 | 13 => {
                            if let Some(image) = images.get(selected) {
                                return Ok(image.clone());
                            }
                        }
                        0x1b => {
                            if let Some(direction) = arrow_key() {
                                match direction {
                                    b'A' if selected > 0 => {
                                        selected -= 1;
                                        if selected < scroll {
                                            scroll = selected;
                                        }
                                        break;
                                    }
                                    b'B' if selected + 1 < images.len() => {
                                        selected += 1;
                                        if selected >= scroll + VISIBLE {
                                            scroll = selected - VISIBLE + 1;
                                        }
                                        break;
                                    }
                                    _ => {}
                                }
                            } else {
                                return Err(io::Error::new(
                                    io::ErrorKind::Interrupted,
                                    "ISO selection cancelled",
                                ));
                            }
                        }
                        _ => {}
                    }
                } else {
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }
}

fn interactive() -> io::Result<(PathBuf, PathBuf)> {
    loop {
        print_banner();
        print_system_profile();
        print_usb_devices();

        println!("{CYAN}{BOLD}Select an ISO image:{RESET}");
        let iso = iso_picker()?;
        let device = prompt(&format!("{GREEN}TARGET DEVICE{RESET} > "))?;

        if device.is_empty() || device.eq_ignore_ascii_case("r") {
            continue;
        }

        return Ok((iso, PathBuf::from(device)));
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
                    "iso-flasher 2.0.0\n\n                     Usage: sudo iso-flasher [--iso IMAGE --device /dev/sdX] [--force]"
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
    let metadata = fs::metadata(iso)?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ISO is not a regular file",
        ));
    }

    let device_string = device.to_string_lossy();
    if !device_string.starts_with("/dev/") || device_string[5..].contains('/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "target must be /dev/<device>",
        ));
    }

    let name = &device_string[5..];
    if !force && !is_removable(name) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "target is not removable; use --force only after verifying it",
        ));
    }

    let size = device_size(device)?;
    if size > 0 && metadata.len() > size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "target device is smaller than the ISO",
        ));
    }

    Ok(metadata.len())
}

fn unmount(device: &Path) -> io::Result<()> {
    let status = Command::new("/bin/umount").arg(device).status()?;
    if status.success() || status.code() == Some(32) {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "failed to unmount {}",
            device.display()
        )))
    }
}

fn handle_flash_key(key: u8, snake: &mut Snake, snake_mode: &mut bool) {
    match key {
        b'w' | b'W' => snake.turn(Point { x: 0, y: -1 }),
        b's' | b'S' => snake.turn(Point { x: 0, y: 1 }),
        b'a' | b'A' => snake.turn(Point { x: -1, y: 0 }),
        b'd' | b'D' => snake.turn(Point { x: 1, y: 0 }),
        0x1b => {
            if arrow_key() == Some(b'[') && read_key() == Some(b'Z') {
                *snake_mode = !*snake_mode;
            }
        }
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
            return Err(io::Error::new(io::ErrorKind::WriteZero, "device stopped accepting data"));
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
            println!("ETA     {:02}m {:02}s", (eta as u64) / 60, (eta as u64) % 60);
            println!("\n{YELLOW}Shift+Tab{RESET} Snake    Ctrl+C Cancel");
        }

        io::stdout().flush()?;
    }

    if !RUNNING.load(Ordering::SeqCst) {
        return Err(io::Error::new(io::ErrorKind::Interrupted, "flash cancelled"));
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

    if unsafe { geteuid() } != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "root access is required; run with sudo",
        ));
    }

    let cli = parse_args();
    let (iso, device) = match (cli.iso, cli.device) {
        (Some(iso), Some(device)) => (iso, device),
        _ => interactive()?,
    };

    let total = validate_target(&iso, &device, cli.force)?;

    println!("{RED}{BOLD}This will erase {}.{RESET}", device.display());
    if prompt("Type FLASH to continue: ")? != "FLASH" {
        return Ok(());
    }

    unmount(&device)?;
    unsafe { sync() };

    flash(&iso, &device, total)
}

fn main() {
    clear_screen();

    if let Err(error) = run() {
        eprintln!("{RED}FAIL:{RESET} {error}");
        std::process::exit(1);
    }

    println!("{GREEN}DONE:{RESET} ISO successfully written.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_iso_extension_case_insensitively() {
        assert!(is_iso(Path::new("linux.iso")));
        assert!(is_iso(Path::new("linux.ISO")));
        assert!(!is_iso(Path::new("linux.img")));
    }

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
