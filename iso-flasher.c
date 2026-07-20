/**
 * ============================================================================
 * █    ██  ██████  ██████▄  ███▄ ▄███▓ ▄▄▄       ██ ▄█▀▓█████  ██▀███
 * ██  ▓██▒▒██    ▒ ▓██   ▀█ ▓██▒▀█▀ ██▒▒████▄     ██▄█▒ ▓█   ▀ ▓██ ▒ ██▒
 * ▓██  ▒██░░ ▓██▄   ▓██▀▀▀▄▄ ▓██    ▓██░▒██  ▀█▄  ▓███▄░ ▒███   ▓██ ░▄█ ▒
 * ▓▓█  ░██░  ▒   ██▒▓██    █ ▒██    ▒██ ░██▄▄▄▄██ ▓██ █▄ ▒▓█  ▄ ▒██▀▀█▄
 * ▒▒█████▓ ▒██████▒▒▒██████▀ ▒██▒   ░██▒  ▓█   ▓██▒▒██▒ █▄░▒████▒░██▓ ▒██▒
 *  ░▒▓▒ ▒ ▒ ▒ ▒▓▒ ▒ ░▒ ▒ ▒  ░░ ▒░   ░  ░  ▒▒   ▓▒█░▒ ▒▒ ▓▒░░ ▒░ ░░ ▒▓ ░▒▓░
 *  ░░▒░ ░ ░ ░ ░▒  ░ ░░ ░ ░ ░ ░  ░      ░   ▒   ▒▒ ░░ ░▒ ▒░ ░ ░  ░  ░▒ ░ ▒░
 *   ░░░ ░ ░ ░  ░  ░  ░ ░ ░   ░      ░      ░   ▒   ░ ░░ ░    ░     ░░   ░
 *     ░           ░  ░       ░             ░  ░░ ░  ░      ░  ░   ░
 *                            ░
 * ============================================================================
 * PURE C RAW BLOCK ENGINE // CODENAMED: MONOLITH
 * DEVELOPED BY: AnshLabs716 & shozanthebozan
 * ============================================================================
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <getopt.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/sysinfo.h>
#include <sys/ioctl.h>
#include <linux/fs.h>
#include <time.h>
#include <dirent.h>
#include <signal.h>

// ========================================
// DESIGN SYSTEM (ANSI RGB DRIP)
// ========================================
#define RED     "\033[38;2;255;95;95m"
#define GREEN   "\033[38;2;95;255;175m"
#define YELLOW  "\033[38;2;255;215;0m"
#define BLUE    "\033[38;2;95;175;255m"
#define MAGENTA "\033[38;2;215;95;255m"
#define CYAN    "\033[38;2;95;255;255m"
#define GRAY    "\033[38;2;118;118;118m"
#define BOLD    "\033[1m"
#define RESET   "\033[0m"

#define MAX_PATH 512
#define CHUNK_SIZE (4 * 1024 * 1024) // 4MB Buffer

typedef struct {
    char iso_path[MAX_PATH];
    char target_dev[MAX_PATH];
    int force_mode;
} Config;

Config app_config = { "", "", 0 };
volatile sig_atomic_t keep_running = 1;

void handle_sigint(int sig) {
    (void)sig;
    keep_running = 0;
    printf("\n%s[!] Interrupted via Ctrl+C. Powering down safely...%s\n", RED, RESET);
}

void log_msg(const char *level, const char *color, const char *msg) {
    time_t now;
    time(&now);
    struct tm *local = localtime(&now);
    char time_str[64];
    strftime(time_str, sizeof(time_str), "%H:%M:%S", local);
    printf("%s[%s] %s[%s]%s %s\n", GRAY, time_str, color, level, RESET, msg);
}

// ========================================
// BANNER & VISUAL LAYOUT
// ========================================
void print_banner() {
    system("clear");
    printf("%s" BOLD, CYAN);
    printf(" █    ██  ██████  ██████▄  ███▄ ▄███▓ ▄▄▄       ██ ▄█▀▓█████  ██▀███  \n");
    printf(" ██  ▓██▒▒██    ▒ ▓██   ▀█ ▓██▒▀█▀ ██▒▒████▄     ██▄█▒ ▓█   ▀ ▓██ ▒ ██▒\n");
    printf(" ▓██  ▒██░░ ▓██▄   ▓██▀▀▀▄▄ ▓██    ▓██░▒██  ▀█▄  ▓███▄░ ▒███   ▓██ ░▄█ ▒\n");
    printf(" ▓▓█  ░██░  ▒   ██▒▓██    █ ▒██    ▒██ ░██▄▄▄▄██ ▓██ █▄ ▒▓█  ▄ ▒██▀▀█▄  \n");
    printf(" ▒▒█████▓ ▒██████▒▒▒██████▀ ▒██▒   ░██▒  ▓█   ▓██▒▒██▒ █▄░▒████▒░██▓ ▒██▒\n");
    printf("  ░▒▓▒ ▒ ▒ ▒ ▒▓▒ ▒ ░▒ ▒ ▒  ░░ ▒░   ░  ░  ▒▒   ▓▒█░▒ ▒▒ ▓▒░░ ▒░ ░░ ▒▓ ░▒▓░\n");
    printf("  ░░▒░ ░ ░ ░ ░▒  ░ ░░ ░ ░ ░ ░  ░      ░   ▒   ▒▒ ░░ ░▒ ▒░ ░ ░  ░  ░▒ ░ ▒░\n");
    printf("   ░░░ ░ ░ ░  ░  ░  ░ ░ ░   ░      ░      ░   ▒   ░ ░░ ░    ░     ░░   ░\n");
    printf("     ░           ░  ░       ░             ░  ░░ ░  ░      ░  ░   ░      \n");
    printf("                            ░                                             \n");
    printf("%s", RESET);
    printf("                        %s%s⚡ RAW CORE ENGINE // v1.0.4 ⚡%s\n", BOLD, MAGENTA, RESET);
    printf("             %s🚀 Built by the legends: %sAnshLabs716 %s& %sshozanthebozan%s\n", GRAY, GREEN, GRAY, GREEN, RESET);
    printf("%s──────────────────────────────────────────────────────────────────────────────────%s\n\n", BLUE, RESET);
}

// ========================================
// CORE DIAGNOSTICS & SYSTEM MONITOR
// ========================================
void run_system_profile() {
    printf("%s" BOLD "📊 CORE HOST ENVIRONMENT PROFILE" RESET "\n", CYAN);
    printf("%s─────────────────────────────────────────────────────────%s\n", GRAY, RESET);

    struct sysinfo info;
    if (sysinfo(&info) == 0) {
        double total_ram = (double)info.totalram * info.mem_unit / (1024 * 1024 * 1024);
        double free_ram = (double)info.freeram * info.mem_unit / (1024 * 1024 * 1024);
        printf("%s ► CPU Scheduler Load:%s  %.2f\n", GREEN, RESET, info.loads[0] / 65536.0);
        printf("%s ► Physical Memory:   %s  %.2f GB used / %.2f GB available\n", GREEN, RESET, total_ram - free_ram, total_ram);
    }

    // The fanless rig warning
    printf("\n%s⚠️  THERMAL WARNING:%s If you're running a fanless rig keep htop/btop open in another terminal this will literally cook your ram and cpu. Keep an eye on those temps.%s\n", RED, YELLOW, RESET);
    printf("\n");
}

int is_removable_usb(const char *device) {
    const char *dev_name = device;
    if (strncmp(device, "/dev/", 5) == 0) dev_name = device + 5;

    char sys_path[MAX_PATH];
    snprintf(sys_path, sizeof(sys_path), "/sys/block/%s/removable", dev_name);

    FILE *f = fopen(sys_path, "r");
    if (!f) return 0;

    int removable = 0;
    if (fscanf(f, "%d", &removable) != 1) removable = 0;
    fclose(f);

    return removable;
}

unsigned long long get_device_size(const char *device) {
    int fd = open(device, O_RDONLY);
    if (fd < 0) return 0;

    unsigned long long bytes = 0;
    if (ioctl(fd, BLKGETSIZE64, &bytes) < 0) {
        bytes = 0;
    }
    close(fd);
    return bytes;
}

void show_sysfs_drives() {
    printf("%s" BOLD "💽 TARGETABLE HARDWARE SIGNATURES" RESET "\n", MAGENTA);
    printf("%s─────────────────────────────────────────────────────────%s\n", GRAY, RESET);

    DIR *dp = opendir("/sys/block");
    if (!dp) return;

    struct dirent *entry;
    int count = 0;
    while ((entry = readdir(dp))) {
        if (entry->d_name[0] == '.' || strncmp(entry->d_name, "loop", 4) == 0) continue;

        char node_path[MAX_PATH];
        snprintf(node_path, sizeof(node_path), "/dev/%s", entry->d_name);

        if (is_removable_usb(node_path)) {
            unsigned long long bytes = get_device_size(node_path);
            double gb = (double)bytes / (1024 * 1024 * 1024);
            printf(" %s[USB Target] %s%s %s(%.2f GB)%s\n", GREEN, BOLD, node_path, YELLOW, gb, RESET);
            count++;
        }
    }
    closedir(dp);
    if (count == 0) printf("%s [!] Universal storage alert: No flash storage found.%s\n", YELLOW, RESET);
    printf("\n");
}

// ========================================
// DIRECT BLOCK IO BUFFER ENGINE
// ========================================
void format_time(double seconds, char *buffer) {
    int m = (int)seconds / 60;
    int s = (int)seconds % 60;
    snprintf(buffer, 32, "%02dm %02ds", m, s);
}

int raw_block_flash(const char *iso_path, const char *usb_dev, off_t total_size) {
    int fd_in = open(iso_path, O_RDONLY);
    if (fd_in < 0) {
        perror("Error parsing installation media file descriptor");
        return 1;
    }

    int fd_out = open(usb_dev, O_WRONLY | O_SYNC);
    if (fd_out < 0) {
        perror("Target hardware stream alignment rejected");
        close(fd_in);
        return 1;
    }

    void *buffer = NULL;
    if (posix_memalign(&buffer, 4096, CHUNK_SIZE) != 0) {
        buffer = malloc(CHUNK_SIZE);
    }

    if (!buffer) {
        log_msg("FATAL", RED, "Virtual hardware buffer allocation overflow.");
        close(fd_in); close(fd_out);
        return 1;
    }

    off_t total_written = 0;
    ssize_t read_bytes;
    struct timespec start, current;
    clock_gettime(CLOCK_MONOTONIC, &start);

    log_msg("LAUNCH", BLUE, "Igniting low-level block mirror engine...");
    printf("\n");

    while (keep_running && (read_bytes = read(fd_in, buffer, CHUNK_SIZE)) > 0) {
        ssize_t written = write(fd_out, buffer, read_bytes);
        if (written != read_bytes) {
            log_msg("IO_ERR", RED, "Critical write degradation detected. Flashing broken.");
            break;
        }

        total_written += written;

        clock_gettime(CLOCK_MONOTONIC, &current);
        double delta = (current.tv_sec - start.tv_sec) + (current.tv_nsec - start.tv_nsec) / 1e9;
        double mbps = (total_written / (1024.0 * 1024.0)) / (delta > 0 ? delta : 1);
        double pct = ((double)total_written / total_size) * 100.0;

        char eta[32];
        format_time((pct > 0) ? (total_size - total_written) / (1024.0 * 1024.0) / mbps : 0, eta);

        int width = 30;
        int current_pos = width * pct / 100;
        printf("\r %s⚡ [", MAGENTA);
        for (int i = 0; i < width; i++) {
            if (i < current_pos) printf("█");
            else if (i == current_pos) printf("▓");
            else printf("░");
        }
        printf("] %s%.1f%% %s| %s%.1f MB/s %s| %sETA: %s%s   ", CYAN, pct, GRAY, YELLOW, mbps, GRAY, GREEN, eta, RESET);
        fflush(stdout);
    }

    printf("\n\n");
    log_msg("SYNC", MAGENTA, "Forcing direct file allocation synchronization layer...");
    fsync(fd_out);

    free(buffer);
    close(fd_in);
    close(fd_out);
    return (total_written == total_size) ? 0 : 1;
}

int pipeline_execution() {
    struct stat st;
    if (stat(app_config.iso_path, &st) != 0) {
        log_msg("ABORT", RED, "Target ISO path could not resolve onto local mounting systems.");
        return 1;
    }

    unsigned long long dev_bytes = get_device_size(app_config.target_dev);
    if (dev_bytes > 0 && (unsigned long long)st.st_size > dev_bytes) {
        log_msg("FATAL", RED, "The target flash drive is too small to fit this ISO file image!");
        return 1;
    }

    if (!is_removable_usb(app_config.target_dev) && !app_config.force_mode) {
        log_msg("SHIELD", RED, "Safety block active: Selection points toward an active internal partition!");
        return 1;
    }

    printf("\n%s 🔥 DANGEROUS HIGH-LEVEL OPERATIONAL OVERLAP 🔥%s\n", RED, RESET);
    printf(" Flash Node:  %s%s%s\n", BOLD, app_config.target_dev, RESET);
    printf(" Action Code: Enter '%sFLASH%s' to confirm block rewriting: ", GREEN, RESET);

    char verify[32];
    if (!fgets(verify, sizeof(verify), stdin)) return 1;
    verify[strcspn(verify, "\n")] = 0;

    if (strcmp(verify, "FLASH") != 0) {
        log_msg("STOP", YELLOW, "System abort parsed. No modifications executed.");
        return 0;
    }

    log_msg("MOUNT", BLUE, "Clearing shared runtime mounts...");
    char clear_mount[MAX_PATH];
    snprintf(clear_mount, sizeof(clear_mount), "sudo umount -q %s* 2>/dev/null", app_config.target_dev);
    system(clear_mount);

    return raw_block_flash(app_config.iso_path, app_config.target_dev, st.st_size);
}

// ========================================
// ENTRY SYSTEM ROUTER
// ========================================
int main(int argc, char *argv[]) {
    // The sudo roast
    if (geteuid() != 0) {
        printf("\n%s💀 BRO YOU FORGOT SUDO! Do I look like I have root access naturally? Run it again with sudo.%s\n\n", RED, RESET);
        return 1;
    }

    struct sigaction sa = { .sa_handler = handle_sigint };
    sigaction(SIGINT, &sa, NULL);

    int interactive = 1;
    struct option options[] = {
        {"iso",    required_argument, 0, 'i'},
        {"device", required_argument, 0, 'd'},
        {"force",  no_argument,       0, 'f'},
        {0, 0, 0, 0}
    };

    int opt;
    while ((opt = getopt_long(argc, argv, "i:d:f", options, NULL)) != -1) {
        switch (opt) {
            case 'i': strncpy(app_config.iso_path, optarg, MAX_PATH); interactive = 0; break;
            case 'd': strncpy(app_config.target_dev, optarg, MAX_PATH); interactive = 0; break;
            case 'f': app_config.force_mode = 1; break;
            default: return 1;
        }
    }

    print_banner();
    run_system_profile();

    if (interactive) {
        show_sysfs_drives();

        printf("%s[📂 INPUT] Absolute Path to ISO file image:%s\n> ", YELLOW, RESET);
        char choice_iso[MAX_PATH];
        if (fgets(choice_iso, sizeof(choice_iso), stdin)) {
            choice_iso[strcspn(choice_iso, "\n")] = 0;
            strncpy(app_config.iso_path, choice_iso, MAX_PATH);
        }

        printf("%s[🔌 TARGET] Target Block Node destination (e.g. /dev/sdc):%s\n> ", YELLOW, RESET);
        char choice_dev[MAX_PATH];
        if (fgets(choice_dev, sizeof(choice_dev), stdin)) {
            choice_dev[strcspn(choice_dev, "\n")] = 0;
            strncpy(app_config.target_dev, choice_dev, MAX_PATH);
        }
    }

    if (strlen(app_config.iso_path) == 0 || strlen(app_config.target_dev) == 0) {
        log_msg("FATAL", RED, "Execution sequence missing operating parameters. Run interactive or pass arguments.");
        return 1;
    }

    return pipeline_execution();
}
