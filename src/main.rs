use directories::UserDirs;
use std::os::windows::fs::MetadataExt;
use std::process::Command;
use sysinfo::System;
use std::fs;

fn main() {
    // Get which processes are running
    let mut sys = System::new_all();
    sys.refresh_all();
    let vrmonitor_process = sys.processes_by_exact_name("vrmonitor.exe".as_ref()).next();
    let vrchat_process = sys.processes_by_exact_name("VRChat.exe".as_ref()).next();

    // Get the last visited world ID
    let vrc_world_id: Option<String> = match vrchat_process {
        None => None,
        Some(_) => get_last_world_id(),
    };

    println!("VRMonitor running: {:#?}", vrmonitor_process.is_some());
    println!("VRChat running: {:#?}", vrchat_process.is_some());
    println!("World ID: {:#?}", vrc_world_id);

    // Kill VRChat
    if let Some(vrchat_process) = vrchat_process {
        println!("Killing VRChat.exe");
        vrchat_process.kill();
        println!("Waiting 5s...");
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    // Kill SteamVR
    if let Some(vrmonitor_process) = vrmonitor_process {
        println!("Killing vrmonitor.exe");
        vrmonitor_process.kill();
        println!("Waiting 10s...");
        std::thread::sleep(std::time::Duration::from_secs(10));
    }

    // Start SteamVR
    println!("Starting SteamVR...");
    {
        let mut command = Command::new("cmd");
        let executable_path =
            "C:\\Program Files (x86)\\Steam\\steamapps\\common\\SteamVR\\bin\\win64\\vrmonitor.exe";
        command.args(&["/C", "start", "", executable_path]);
        let _ = command.spawn();
        println!("Waiting 10s...");
        std::thread::sleep(std::time::Duration::from_secs(10));
    }

    // Launch VRChat back into the last visited world
    if let Some(world_id) = vrc_world_id {
        println!("Launching VRChat in world {}", world_id);
        let _ = Command::new("cmd")
            .args(&["/c", "start", format!("vrchat://launch?id={}", world_id).as_str()])
            .spawn();
    }

    println!("DONE");
}

fn get_last_world_id() -> Option<String> {
    match get_latest_log_path() {
        None => None,
        Some(latest_log_path) => {
            let log_file = match fs::read_to_string(&latest_log_path) {
                Ok(log_file) => log_file,
                Err(_) => return None,
            };
            let lines: Vec<String> = log_file
                .lines()
                .map(|line| line.to_string())
                .rev()
                .collect();
            let mut instance_id = None;
            for line in lines {
                if line.contains("[Behaviour] Joining ")
                    && !line.contains("] Joining or Creating Room: ")
                    && !line.contains("] Joining friend: ")
                {
                    let mut offset = match line.rfind("] Joining ") {
                        Some(v) => v,
                        None => return None,
                    };
                    offset += 10;
                    if offset >= line.len() {
                        return None;
                    }
                    instance_id = Some(line[offset..].to_string());
                    break;
                }
            }
            instance_id
        }
    }
}

fn get_latest_log_path() -> Option<String> {
    let user_dirs = match UserDirs::new() {
        Some(user_dirs) => user_dirs,
        None => return None,
    };
    // Get all files in the log directory
    let dir = match fs::read_dir(
        user_dirs
            .home_dir()
            .join("AppData\\LocalLow\\VRChat\\VRChat"),
    ) {
        Ok(dir) => dir,
        Err(_) => return None,
    };
    // Get the latest log file
    dir
        // Only get log files
        .filter(|entry| {
            let name = match entry.as_ref() {
                Ok(entry) => {
                    let entry = entry.clone();
                    let file_name = entry.file_name();
                    file_name.to_str().unwrap().to_string()
                }
                Err(_) => return false,
            };
            name.starts_with("output_log_") && name.ends_with(".txt")
        })
        // Find most recent log file
        .max_by_key(|entry| entry.as_ref().unwrap().metadata().unwrap().creation_time())
        // Get the path for it
        .map(|entry| String::from(entry.unwrap().path().to_str().unwrap()))
}
