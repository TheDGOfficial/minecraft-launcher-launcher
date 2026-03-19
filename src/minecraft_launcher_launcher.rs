use std::env;
use std::fs;
use std::process;

use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitCode;
use std::thread;
use std::io;
use std::io::Write;
use std::time::Duration;
use std::sync::mpsc::channel;

use crate::utils;
use colored::Colorize;

use sysinfo::Pid;
use sysinfo::Process;
use sysinfo::ProcessRefreshKind;
use sysinfo::ProcessesToUpdate;
use sysinfo::System;
use sysinfo::UpdateKind;

#[cfg(target_os = "linux")]
use sudo::RunningAs;

#[cfg(target_os = "linux")]
use cnproc::PidMonitor;

#[cfg(target_os = "linux")]
use cnproc::PidEvent;

#[cfg(target_os = "linux")]
use nix::unistd::Pid as NixPid;

#[cfg(target_os = "linux")]
use nix::sys::signal::Signal;

#[cfg(target_os = "linux")]
use nix::sys::signal;

use std::sync::LazyLock;

fn notify_error(description: &str) {
    eprintln!("{}", description.red());
}

#[cfg(not(target_os = "linux"))]
#[inline]
pub(crate) fn launch() -> ExitCode {
    notify_error("Minecraft Launcher launcher is only supported on Linux");

    ExitCode::FAILURE
}

// Minecraft Launcher Launcher launches Minecraft Launcher with various
// environment variables to fix bugs and improve performance, while also making
// it so when the game starts, all launcher processes will be killed. This will
// usually save around 300MB of memory from being wasted due to the launcher
// using an embedded browser to render its pages. After setting environment
// variables and launching the Launcher, this app will wait in the background
// for you to press Play and start the game in the launcher. Once you do that
// all launcher processes will be killed to save resources, and then after that
// this app will also quit, so only the java runtime (the actual game) is
// running.

// It will also automatically delete JavaCheck.jar to let you launch any game
// version with any Java version you desire.

// NOTE: Environment variables that are not supported, force disabled, etc.
// will just be ignored or not do anything at all.

// NOTE 2: Minecraft Launcher Launcher requires sudo. This not convenient, so
// you should do VISUAL=gnome-text-editor EDITOR="$VISUAL" sudo -E visudo and
// add yourusername ALL = (root) NOPASSWD: /usr/bin/minecraft-launcher to the
// last line. change gnome-text-editor with gedit if using old ubuntu versions,
// or another editor.

// NOTE 2.1: Despite that, it requires running it without sudo AND then
// escalating to sudo privileges because the launcher itself and java checker
// MUST run without sudo. Only the PID monitoring for the starting of Java
// process (game process) requires, and will, use sudo.

// TODO Future plans include checking for Bootstrap launcher updates.

/// Location of PID file in /tmp (safe, automatically cleared on reboot)
const LAUNCHER_PID_FILE: &str = "/tmp/minecraft-launcher.pid";

#[inline]
#[must_use]
fn write_pid_file(pid: u32) -> io::Result<()> {
    let mut file = fs::File::create(LAUNCHER_PID_FILE)?;
    write!(file, "{}", pid)?;
    file.sync_all()?;
    Ok(())
}

#[inline]
#[must_use]
fn read_pid_file_blocking() -> u32 {
    loop {
        match fs::read_to_string(LAUNCHER_PID_FILE) {
            Ok(contents) => {
                if let Ok(pid) = contents.trim().parse::<u32>() {
                    return pid;
                } else {
                    eprintln!("{}", "Launcher PID file malformed, retrying...".red());
                }
            }
            Err(_) => {
                // PID file doesn't exist yet, wait a bit
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

#[inline]
fn renice_launcher(pid: u32) {
    fn run(cmd: &str, args: &[&str], label: &str, pid: u32) {
        let status = Command::new(cmd)
            .args(args)
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("{} for PID {} succeeded", label, pid);
            }
            Ok(s) => {
                eprintln!("{} for PID {} failed: exited with {}", label, pid, s);
            }
            Err(e) => {
                eprintln!("{} for PID {} failed: {}", label, pid, e);
            }
        }
    }

    let pid_str = pid.to_string();
    let pid_ref = pid_str.as_str();

    // CPU priority
    run(
        "renice",
        &["-n", "-6", "-p", pid_ref],
        "renice (-6)",
        pid,
    );

    // I/O priority (best-effort, highest)
    run(
        "ionice",
        &["-c", "2", "-n", "0", "-p", pid_ref],
        "ionice (class 2, prio 0)",
        pid,
    );
}

#[inline]
#[cfg(target_os = "linux")]
pub(crate) fn launch() -> ExitCode {
    let user = sudo::check() == RunningAs::User;

    if user && find_launcher_processes(System::new(), false, true) {
        println!("Already open, exiting.");

        return ExitCode::SUCCESS;
    }

    if user {
        remove_javacheck();

        // remove existing PID file for safety
        let _ = fs::remove_file(LAUNCHER_PID_FILE);

        // launch launcher and get PID
        let rx = launch_launcher();

        if let Ok(pid_u32) = rx.recv() {
            if let Err(e) = write_pid_file(pid_u32) {
                notify_error(&format!("Failed to write PID file: {e}"));
            }
        } else {
            notify_error("Failed to receive launcher PID");
        }
    }

    if !escalate_if_needed() {
        return ExitCode::FAILURE;
    }

    // read pid_u32 from file, if file does not exist block till it exists
    let pid_u32 = read_pid_file_blocking();

    // set priority
    renice_launcher(pid_u32);

    // cleanup PID file
    let _ = fs::remove_file(LAUNCHER_PID_FILE);

    start_watching_java_process();

    ExitCode::SUCCESS
}

static KILLING_IN_PROGRESS: LazyLock<AtomicBool> =
    LazyLock::new(|| AtomicBool::new(false));

const LAUNCHER_PROFILES_MLLBACKUP_FILE: &str =
    "launcher_profiles.json.mllbackup";

fn get_launcher_profiles_path() -> Option<PathBuf> {
    utils::get_minecraft_dir().map_or_else(
        || {
            notify_error("can't find launcher path");

            None
        },
        |launcher_path| {
            let launcher_profiles_path =
                launcher_path.join("launcher_profiles.json");

            Some(launcher_profiles_path)
        },
    )
}

fn backup_launcher_profiles() {
    println!("Backing up launcher profiles...");
    if let Some(path) = get_launcher_profiles_path()
        && let Some(contents) = utils::read_file(&path)
    {
        if let Some(parent) = &path.parent() {
            if utils::write_file(
                &parent.join(LAUNCHER_PROFILES_MLLBACKUP_FILE),
                &contents,
            ) {
                println!("Backed up launcher profiles.");
            } // Error will be printed by the util method if write fails.
        } else {
            notify_error(&format!("no parent for {}", path.to_string_lossy()));
        }
    } // error will be printed by the read method if None
    // error will be printed by the get method if None
}

fn restore_launcher_profiles() {
    println!("Restoring launcher profiles from backup...");
    if let Some(path) = get_launcher_profiles_path() {
        if let Some(parent) = &path.parent() {
            let backup_file_path =
                parent.join(LAUNCHER_PROFILES_MLLBACKUP_FILE);
            if let Some(contents) = utils::read_file(&backup_file_path) {
                if utils::write_file(&path, &contents) {
                    println!("Restored launcher profiles from backup.");
                } // Error will be printed by the util method if write fails.

                if let Err(e) = fs::remove_file(backup_file_path) {
                    notify_error(&format!(
                        "error while removing {LAUNCHER_PROFILES_MLLBACKUP_FILE}: {e}"
                    ));
                }
            } // error will be printed by the read method if None
        } else {
            notify_error(&format!("no parent for {}", path.to_string_lossy()));
        }
    } // error will be printed by the get method if None
}

#[cfg(not(target_os = "linux"))]
#[inline]
fn kill_launcher_process(launcher_process: &Process) {
    if launcher_process.kill() {
        println!("Killed process successfully");
    } else {
        eprintln!(
            "Couldn't kill Minecraft Launcher process named {} with PID {}. Already killed?",
            launcher_process.name(),
            launcher_process.pid()
        );
        // Can happen if already killed, not a fatal error.
    }
}

#[cfg(target_os = "linux")]
#[inline]
fn kill_launcher_process(launcher_process: &Process) {
    let pid = i32::try_from(launcher_process.pid().as_u32());

    if let Ok(pid_i32) = pid {
        if signal::kill(NixPid::from_raw(pid_i32), Signal::SIGTERM).is_ok() {
            println!("Killed process successfully");
        } else {
            eprintln!(
                "Couldn't kill Minecraft Launcher process named {} with PID {}. Already killed?",
                launcher_process.name().to_string_lossy(),
                launcher_process.pid()
            );
            // Can happen if already killed, not a fatal error.
        }
    }
}

#[inline]
fn find_launcher_processes(
    mut sys: System,
    kill: bool,
    check_parent: bool,
) -> bool {
    if kill
        && KILLING_IN_PROGRESS.compare_exchange(
            false,
            true,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) != Ok(false)
    {
        notify_error("atomic operation failure (expected false, got true)");
    }

    let _ = sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_cmd(UpdateKind::OnlyIfNotSet)
            .with_user(UpdateKind::OnlyIfNotSet),
    );

    let mut found = false;
    let self_pid = process::id();

    for launcher_process in sys.processes_by_name(
        OsStr::new("minecraft-launc"), /* Not a typo, process names
                                        * are limited
                                        * to 15
                                        * characters in Linux as
                                        * docs on the
                                        * processes_by_name method
                                        * suggests. */
    ) {
        if launcher_process.pid().as_u32() != self_pid
            && (!check_parent
                || launcher_process.parent().map(Pid::as_u32)
                    != Some(self_pid))
            && launcher_process
                .effective_user_id()
                .is_some_and(|uid| **uid != 0)
        {
            if launcher_process.cmd().iter().any(|arg| {
                arg.to_string_lossy().contains("minecraft-launcher-launcher")
            }) {
                // This is definitely our process, we don't want to kill
                // ourselves.
                continue;
            }
            println!(
                "Found launcher process {}. PID: {}",
                launcher_process.name().to_string_lossy(),
                launcher_process.pid(),
            );

            if kill {
                kill_launcher_process(launcher_process);
            }

            found = true;
        }
    }

    // Workaround to also kill that one
    // process remaining that doesn't
    // use minecraft-launc name, but
    // uses exe
    for possible_stealth_launcher_process in sys.processes().values() {
        if possible_stealth_launcher_process.name() == "exe"
            && possible_stealth_launcher_process.pid().as_u32() != self_pid
            && (!check_parent
                || possible_stealth_launcher_process.parent().map(Pid::as_u32)
                    != Some(self_pid))
            && possible_stealth_launcher_process
                .effective_user_id()
                .is_some_and(|uid| **uid != 0)
            && possible_stealth_launcher_process.cmd().iter().any(|element| {
                element.to_string_lossy().contains("--launcherui")
            })
        {
            if possible_stealth_launcher_process.cmd().iter().any(|arg| {
                arg.to_string_lossy().contains("minecraft-launcher-launcher")
            }) {
                // This is definitely our process, we don't want to kill
                // ourselves.
                continue;
            }
            println!(
                "Found stealth launcher process {}. PID: {}",
                possible_stealth_launcher_process.name().to_string_lossy(),
                possible_stealth_launcher_process.pid(),
            );

            if kill {
                kill_launcher_process(possible_stealth_launcher_process);
            }

            found = true;
        }
    }

    if kill
        && KILLING_IN_PROGRESS.compare_exchange(
            true,
            false,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) != Ok(true)
    {
        notify_error("atomic operation failure (expected true, got false)");
    }

    found
}

#[inline]
fn start_watching_java_process() {
    println!("Starting monitoring");

    match PidMonitor::new() {
        Ok(mut monitor) => {
            let mut sys = System::new();

            loop {
                if let Some(e) = monitor.recv() {
                    match e {
                        PidEvent::Exec { process_pid, .. } => {
                            let id = process_pid;
                            if let Ok(id_u32) = u32::try_from(id) {
                                let pid = Pid::from_u32(id_u32);

                                if sys.refresh_processes_specifics(
                                    ProcessesToUpdate::Some(&[pid]),
                                    true,
                                    ProcessRefreshKind::nothing()
                                        .with_cmd(UpdateKind::OnlyIfNotSet),
                                ) == 1
                                    && let Some(process) = sys.process(pid)
                                {
                                    let name = process.name();

                                    if name == "java"
                                            && process.cmd().iter().any(
                                                |element| {
                                                    element.to_string_lossy()
                                                        .contains("-Dminecraft.launcher.brand=minecraft-launcher")
                                                },
                                            )
                                        {
                                            backup_launcher_profiles();
                                            let _ = find_launcher_processes(sys, true, false);
                                            restore_launcher_profiles();
                                            break;
                                        }
                                }
                            } else {
                                notify_error(&format!(
                                    "Can't convert i32 PID to usize PID: {id}"
                                ));
                            }
                        },

                        PidEvent::Exit { process_pid, .. } => {
                            let id = process_pid;
                            if let Ok(id_u32) = u32::try_from(id) {
                                let pid = Pid::from_u32(id_u32);

                                if let Some(process) = sys.process(pid) {
                                    let name = process.name();

                                    if name == "minecraft-launc"
                                        && !KILLING_IN_PROGRESS
                                            .load(Ordering::Relaxed)
                                    {
                                        break;
                                    }
                                }
                            } else {
                                notify_error(&format!(
                                    "Can't convert i32 PID to usize PID: {id}"
                                ));
                            }
                        },

                        PidEvent::Fork { .. } | PidEvent::Coredump { .. } => {
                        },
                    }
                } else {
                    notify_error("no events to receive");
                }
            }
        },

        Err(e) => {
            notify_error(&format!(
                "error while trying to create process event watcher: {e}"
            ));
        },
    }
}

#[inline]
pub(crate) fn remove_javacheck() {
    if let Some(launcher_path) = utils::get_minecraft_dir() {
        let javacheck_path = launcher_path
            .join("launcher")
            .join("JavaCheck.jar");

        if javacheck_path.exists() {
            println!("Removing JavaCheck.jar");

            if let Err(e) = fs::remove_file(javacheck_path) {
                notify_error(&format!(
                    "error while removing JavaCheck.jar: {e}"
                ));
            }
        }
    } else {
        notify_error("can't find launcher path");
    }
}

#[inline]
fn launch_launcher() -> std::sync::mpsc::Receiver<u32> {
    let (tx, rx) = channel();

    let _ = thread::spawn(move || {
        let mut envs = HashMap::from([
            ("vblank_mode", "0"), // Improves performance
            ("__GL_SYNC_TO_VBLANK", "0"), /* Same as the above
                                   * environment variable, but
                                   * also works on NVIDIA closed
                                   * source drivers. */
            ("ALSOFT_DRIVERS", "pipewire"), // Use native pipewire
            ("LIBGL_DRI2_DISABLE", "true"), // Force use of DRI3 if available
            ("MESA_NO_ERROR", "true"),      /* Disable error checking for
                                             * performance */
            ("MESA_GL_VERSION_OVERRIDE", "4.3"), /* Force increase
                                                  * advertised GL version
                                                  * for performance */
            ("MESA_GLES_VERSION_OVERRIDE", "3.2"), // ^^
            ("MESA_GLSL_VERSION_OVERRIDE", "430"), // ^^
            ("DRI_NO_MSAA", "true"),               /* Disable MSAA for
                                                    * performance */
            ("MESA_SHADER_CACHE_DISABLE", "false"), /* Force enable Shader
                                                     * Cache */
            ("MESA_SHADER_CACHE_MAX_SIZE", "4G"), /* Use a big value as limit for Shader Cache */
            ("LD_PRELOAD", "/usr/local/lib/libmimalloc.so.2.2"), /* Use mimalloc to increase memory/GC performance */
        ]);

        if let Ok(value) =
            env::var("MC_LAUNCHER_LAUNCHER_NO_GL_VERSION_OVERRIDE")
            && value == "true"
        {
            println!("Not overriding advertised GL versions.");

            let _ = envs.remove("MESA_GL_VERSION_OVERRIDE");
            let _ = envs.remove("MESA_GLES_VERSION_OVERRIDE");
            let _ = envs.remove("MESA_GLSL_VERSION_OVERRIDE");
        }

        let _ = envs.remove("LD_PRELOAD"); // Temporary; mimalloc causes launcher to never load.

        match Command::new("minecraft-launcher-real").envs(envs).spawn() {
            Ok(child) => {
                // Send PID back to main thread
                let _ = tx.send(child.id());
            },
            Err(e) => notify_error(&format!(
                "error while trying to start Minecraft Launcher: {e}"
            )),
        }
    });

    rx
}

#[inline]
fn escalate_if_needed() -> bool {
    if let Err(e) = sudo::escalate_if_needed() {
        notify_error(&format!(
            "error while trying to escalate to root permissions automatically: {e}"
        ));

        return false;
    }

    true
}

#[inline]
#[cfg(not(target_os = "linux"))]
pub(crate) fn install(_: &str, _: &[String]) -> ExitCode {
    notify_error("Minecraft Launcher launcher is only supported on Linux");

    ExitCode::FAILURE
}

// This function installs the binary running this program itself to the
// /usr/bin/minecraft-launcher.
#[inline]
#[cfg(target_os = "linux")]
pub(crate) fn install(binary_file_name: &str, args: &[String]) -> ExitCode {
    fn fail(msg: &str) -> ExitCode {
        eprintln!("{}", msg.red());
        ExitCode::FAILURE
    }

    if !escalate_if_needed() {
        return ExitCode::FAILURE;
    }

    println!("Starting install");

    let self_path = match env::current_exe() {
        Ok(p) => p,
        Err(e) =>
            return fail(&format!(
                "error when getting current executable path: {e}"
            )),
    };

    if !self_path.exists() {
        return fail("Current executable deleted, can't continue");
    }

    let Some(self_file_name) =
        self_path.file_name().map(|n| n.to_string_lossy())
    else {
        return fail("can't get file name from current executable path");
    };

    if *binary_file_name != self_file_name {
        return fail(&format!(
            "error: current executable name ({self_file_name}) and original ({binary_file_name}) differ"
        ));
    }

    if self_file_name.contains(char::REPLACEMENT_CHARACTER) {
        return fail("non-unicode characters in executable path");
    }

    let bin_dir = Path::new("/usr/bin");
    if !bin_dir.exists() {
        return fail("bin directory doesn't exist, can't continue");
    }

    let launcher_path = bin_dir.join("minecraft-launcher");
    if !launcher_path.exists() {
        return fail(
            "Minecraft Launcher doesn't exist, can't continue. Please install it first.",
        );
    }

    println!("Checking if already installed. This might take some time..");

    match utils::is_same_file(&self_path, &launcher_path) {
        Ok(true) => {
            println!("Already installed, nothing to do.");
            ExitCode::SUCCESS
        },
        Ok(false) => {
            backup_launcher_profiles();
            if find_launcher_processes(System::new(), true, true) {
                println!(
                    "Killed launcher to proceed with install. Please restart it after install if desired."
                );
            }
            restore_launcher_profiles();

            let real_launcher_path = bin_dir.join("minecraft-launcher-real");

            if args.contains(&"--upgrade".to_owned())
                && let Err(e) = fs::remove_file(&real_launcher_path)
            {
                eprintln!(
                    "{}{}",
                    "error while removing real launcher: ".red(),
                    e
                );
            }

            if !real_launcher_path.exists()
                && !utils::copy(&launcher_path, &real_launcher_path)
            {
                return fail("Install failed");
            } else if real_launcher_path.exists() {
                println!(
                    "Real launcher already exists, skipping. Use --upgrade to overwrite."
                );
            } else {
                println!(
                    "Copied real launcher from {} to {} successfully",
                    launcher_path.to_string_lossy(),
                    real_launcher_path.to_string_lossy()
                );
            }

            if !utils::copy(&self_path, &launcher_path) {
                return fail("Install failed");
            }

            println!("Install successful");
            ExitCode::SUCCESS
        },
        Err(e) => fail(&format!(
            "error comparing current executable with launcher path: {e}"
        )),
    }
}
