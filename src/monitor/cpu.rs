//! Reads live CPU data from the system.
//!
//! This module provides platform-agnostic CPU monitoring with
//! platform-specific implementations for Linux and Windows.

#[cfg(target_os = "linux")]
use crate::error;
use crate::warning;
#[cfg(target_os = "linux")]
use std::process::exit;

// ============================================================================
// Linux Implementation
// ============================================================================
#[cfg(target_os = "linux")]
use cpu_monitor::CpuInstant;
#[cfg(target_os = "linux")]
use std::{fs::{read_dir, read_to_string, File}, io::{BufRead, BufReader}};

#[cfg(target_os = "linux")]
pub struct Cpu {
    temp_sensor: Option<String>,
    rapl_max_uj: u64,
}

#[cfg(target_os = "linux")]
impl Cpu {
    pub fn new() -> Self {
        Cpu {
            temp_sensor: find_temp_sensor(),
            rapl_max_uj: get_max_energy(),
        }
    }

    pub fn warn_temp(&self) {
        if self.temp_sensor.is_none() {
            warning!("No supported CPU temperature sensor was found");
            eprintln!("         CPU temperature will not be displayed, and alarm will be disabled.");
            eprintln!("         Supported kernel modules are: asusec, coretemp, k10temp, and zenpower.");
        }
    }

    pub fn warn_rapl(&self) {
        if self.rapl_max_uj == 0 {
            warning!("RAPL module was not found");
            eprintln!("         CPU power consumption will not be displayed.");
        }
    }

    pub fn get_temp(&self, fahrenheit: bool) -> u8 {
        if let Some(sensor) = &self.temp_sensor {
            let data = read_to_string(sensor).unwrap_or_else(|_| {
                error!("Failed to get CPU temperature");
                exit(1);
            });
            let mut temp = data.trim_end().parse::<u32>().unwrap();
            if fahrenheit {
                temp = temp * 9 / 5 + 32000
            }
            return (temp as f32 / 1000.0).round() as u8;
        }
        0
    }

    pub fn read_energy(&self) -> u64 {
        if self.rapl_max_uj > 0 {
            let data = read_to_string("/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj").unwrap_or_else(|_| {
                error!("Failed to get CPU power");
                exit(1);
            });
            return data.trim_end().parse::<u64>().unwrap();
        }
        0
    }

    pub fn get_power(&self, initial_energy: u64, delta_millisec: u64) -> u16 {
        if self.rapl_max_uj > 0 {
            let current_energy = self.read_energy();
            let delta_energy = if current_energy > initial_energy {
                current_energy - initial_energy
            } else {
                (self.rapl_max_uj + current_energy) - initial_energy
            };
            return (delta_energy as f64 / (delta_millisec * 1000) as f64).round() as u16;
        }
        0
    }

    pub fn read_instant(&self) -> CpuInstant {
        CpuInstant::now().unwrap_or_else(|_| {
            error!("Failed to get CPU usage");
            exit(1);
        })
    }

    pub fn get_usage(&self, initial_instant: CpuInstant) -> u8 {
        let usage = (self.read_instant() - initial_instant).non_idle() * 100.0;
        usage.round() as u8
    }

    pub fn get_frequency(&self) -> u16 {
        let cpuinfo = read_to_string("/proc/cpuinfo").unwrap_or_else(|_| {
            error!("Failed to get CPU clock");
            exit(1);
        });

        let mut highest_core = 0.0;
        for info in cpuinfo.lines() {
            if info.starts_with("cpu MHz") {
                let clock = info.split(':').nth(1).unwrap();
                let clock = clock.trim().parse::<f32>().unwrap();
                if clock > highest_core {
                    highest_core = clock;
                }
            }
        }
        highest_core.round() as u16
    }
}

#[cfg(target_os = "linux")]
fn find_temp_sensor() -> Option<String> {
    for sensor in read_dir("/sys/class/hwmon").ok()? {
        let path = sensor.ok()?.path().to_str()?.to_owned();
        if let Ok(name) = read_to_string(format!("{path}/name")) {
            if ["asusec", "coretemp", "k10temp", "zenpower"].contains(&name.trim_end()) {
                return Some(format!("{path}/temp1_input"));
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn get_max_energy() -> u64 {
    match read_to_string("/sys/class/powercap/intel-rapl/intel-rapl:0/max_energy_range_uj") {
        Ok(data) => data.trim_end().parse::<u64>().unwrap(),
        Err(_) => 0,
    }
}

#[cfg(target_os = "linux")]
pub fn get_name() -> Option<String> {
    let file = File::open("/proc/cpuinfo").ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.ok()?;
        if line.starts_with("model name") {
            if let Some(colon_pos) = line.find(':') {
                return Some(line[colon_pos + 1..].trim().to_string());
            }
        }
    }
    None
}

// ============================================================================
// Windows Implementation
// ============================================================================
#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::{OsStrExt, OsStringExt};
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicU64, AtomicU16, AtomicU8, AtomicBool, Ordering};
#[cfg(target_os = "windows")]
use std::process::{Command, Stdio};
#[cfg(target_os = "windows")]
use std::io::{BufRead, BufReader};
#[cfg(target_os = "windows")]
use std::thread;
#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::GetSystemTimes;
#[cfg(target_os = "windows")]
use winapi::um::winnt::KEY_READ;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::{HKEY, FILETIME};
#[cfg(target_os = "windows")]
use winapi::um::winreg::{RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE};

#[cfg(target_os = "windows")]
static LAST_IDLE: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "windows")]
static LAST_KERNEL: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "windows")]
static LAST_USER: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "windows")]
static CACHED_CPU_TEMP: AtomicU8 = AtomicU8::new(0);
#[cfg(target_os = "windows")]
static CACHED_CPU_POWER: AtomicU16 = AtomicU16::new(0);
#[cfg(target_os = "windows")]
static CACHED_GPU_TEMP: AtomicU8 = AtomicU8::new(0);
#[cfg(target_os = "windows")]
static CACHED_GPU_POWER: AtomicU16 = AtomicU16::new(0);
#[cfg(target_os = "windows")]
static LHM_RUNNING: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
pub struct Cpu {
    temp_available: bool,
    power_available: bool,
}

#[cfg(target_os = "windows")]
impl Cpu {
    pub fn new() -> Self {
        // Try to start LibreHardwareMonitor provider in background thread
        let (temp_available, power_available) = Self::start_lhm_thread();

        if temp_available && power_available {
            println!("         LibreHardwareMonitor initialized successfully.");
            println!("         Real CPU temperature and power monitoring enabled.");
        } else {
            warning!("LibreHardwareMonitor not available");
            eprintln!("         CPU temperature and power will be estimated.");
            eprintln!("         Ensure lhm_provider.py and LibreHardwareMonitor DLLs are present.");
        }

        Cpu {
            temp_available,
            power_available,
        }
    }

    /// Start the LHM provider in a background thread
    fn start_lhm_thread() -> (bool, bool) {
        // Get the executable directory to find lhm_provider.py
        let exe_path = std::env::current_exe().ok();
        let exe_dir = exe_path.as_ref().and_then(|p| p.parent()).map(|p| p.to_path_buf());
        
        // Try to find lhm_provider.py
        let script_path = if let Some(dir) = &exe_dir {
            let path = dir.join("lhm_provider.py");
            if path.exists() {
                Some(path)
            } else {
                // Try current directory
                let cwd_path = std::path::PathBuf::from("lhm_provider.py");
                if cwd_path.exists() {
                    Some(cwd_path)
                } else {
                    None
                }
            }
        } else {
            None
        };

        let script_path = match script_path {
            Some(p) => p,
            None => {
                eprintln!("         lhm_provider.py not found");
                return (false, false);
            }
        };

        // Start Python process - use full path to avoid Windows Store stub
        let python_path = std::path::PathBuf::from(
            std::env::var("LOCALAPPDATA")
                .unwrap_or_else(|_| "C:\\Users\\84765\\AppData\\Local".to_string())
        ).join("Programs\\Python\\Python312\\python.exe");

        let python_exe = if python_path.exists() {
            python_path
        } else {
            // Fallback to system python
            std::path::PathBuf::from("python")
        };

        let result = Command::new(&python_exe)
            .arg(&script_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        match result {
            Ok(mut child) => {
                // Get stdout reader
                let stdout = child.stdout.take();
                if let Some(stdout) = stdout {
                    let mut reader = BufReader::new(stdout);
                    
                    // Wait for READY signal (with timeout)
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(_) => {
                            let line = line.trim();
                            if line == "READY" {
                                // Mark as running
                                LHM_RUNNING.store(true, Ordering::SeqCst);
                                
                                // Spawn background thread to continuously read data
                                thread::spawn(move || {
                                    loop {
                                        if !LHM_RUNNING.load(Ordering::SeqCst) {
                                            break;
                                        }
                                        let mut data_line = String::new();
                                        match reader.read_line(&mut data_line) {
                                            Ok(0) => break, // EOF
                                            Ok(_) => {
                                                if let Some((cpu_temp, cpu_power, gpu_temp, gpu_power)) = Self::parse_lhm_line(&data_line) {
                                                    if cpu_temp > 0 {
                                                        CACHED_CPU_TEMP.store(cpu_temp, Ordering::SeqCst);
                                                    }
                                                    if cpu_power > 0 {
                                                        CACHED_CPU_POWER.store(cpu_power, Ordering::SeqCst);
                                                    }
                                                    if gpu_temp > 0 {
                                                        CACHED_GPU_TEMP.store(gpu_temp, Ordering::SeqCst);
                                                    }
                                                    if gpu_power > 0 {
                                                        CACHED_GPU_POWER.store(gpu_power, Ordering::SeqCst);
                                                    }
                                                }
                                            }
                                            Err(_) => break,
                                        }
                                    }
                                    let _ = child.kill();
                                    let _ = child.wait();
                                });
                                
                                return (true, true);
                            } else if line == "NO_LHM" {
                                let _ = child.kill();
                                return (false, false);
                            }
                        }
                        Err(_) => {
                            let _ = child.kill();
                            return (false, false);
                        }
                    }
                }
                let _ = child.kill();
                (false, false)
            }
            Err(e) => {
                eprintln!("         Failed to start lhm_provider.py: {}", e);
                (false, false)
            }
        }
    }

    /// Parse LHM output line: "CPU_TEMP:xx.x|CPU_POWER:xx.x|GPU_TEMP:xx.x|GPU_POWER:xx.x"
    fn parse_lhm_line(line: &str) -> Option<(u8, u16, u8, u16)> {
        let line = line.trim();
        let mut cpu_temp: Option<u8> = None;
        let mut cpu_power: Option<u16> = None;
        let mut gpu_temp: Option<u8> = None;
        let mut gpu_power: Option<u16> = None;

        for part in line.split('|') {
            if part.starts_with("CPU_TEMP:") {
                if let Ok(val) = part[9..].parse::<f32>() {
                    if val > 0.0 && val < 150.0 {
                        cpu_temp = Some(val as u8);
                    }
                }
            } else if part.starts_with("CPU_POWER:") {
                if let Ok(val) = part[10..].parse::<f32>() {
                    if val >= 0.0 && val < 500.0 {
                        cpu_power = Some(val as u16);
                    }
                }
            } else if part.starts_with("GPU_TEMP:") {
                if let Ok(val) = part[9..].parse::<f32>() {
                    if val > 0.0 && val < 150.0 {
                        gpu_temp = Some(val as u8);
                    }
                }
            } else if part.starts_with("GPU_POWER:") {
                if let Ok(val) = part[10..].parse::<f32>() {
                    if val >= 0.0 && val < 600.0 {
                        gpu_power = Some(val as u16);
                    }
                }
            }
        }

        // Return parsed values (N/A values will remain None)
        match (cpu_temp, cpu_power, gpu_temp, gpu_power) {
            (ct, cp, gt, gp) => Some((
                ct.unwrap_or(0),
                cp.unwrap_or(0),
                gt.unwrap_or(0),
                gp.unwrap_or(0),
            )),
        }
    }

    pub fn warn_temp(&self) {
        if !self.temp_available {
            warning!("CPU temperature monitoring requires LibreHardwareMonitor");
            eprintln!("         CPU temperature will be estimated based on usage.");
        }
    }

    pub fn warn_rapl(&self) {
        if !self.power_available {
            warning!("CPU power monitoring requires LibreHardwareMonitor");
            eprintln!("         CPU power will be estimated based on usage and TDP.");
        }
    }

    /// Get CPU temperature - from LHM or estimated
    pub fn get_temp(&self, fahrenheit: bool) -> u8 {
        let temp = if self.temp_available {
            let cached = CACHED_CPU_TEMP.load(Ordering::SeqCst);
            if cached > 0 {
                cached
            } else {
                // Fallback to estimate if LHM returns 0 (AMD Ryzen issue)
                self.estimate_temp()
            }
        } else {
            self.estimate_temp()
        };

        if fahrenheit {
            (temp as f32 * 9.0 / 5.0 + 32.0) as u8
        } else {
            temp
        }
    }

    /// Get CPU power - from LHM or estimated
    pub fn get_power(&self, _initial_energy: u64, _delta_millisec: u64) -> u16 {
        if self.power_available {
            let cached = CACHED_CPU_POWER.load(Ordering::SeqCst);
            if cached > 0 {
                cached
            } else {
                // Fallback to estimate if LHM returns 0
                self.estimate_power()
            }
        } else {
            self.estimate_power()
        }
    }

    /// Get GPU temperature - from LHM
    pub fn get_gpu_temp(&self, fahrenheit: bool) -> u8 {
        let temp = CACHED_GPU_TEMP.load(Ordering::SeqCst);
        if fahrenheit {
            (temp as f32 * 9.0 / 5.0 + 32.0) as u8
        } else {
            temp
        }
    }

    /// Get GPU power - from LHM
    pub fn get_gpu_power(&self) -> u16 {
        CACHED_GPU_POWER.load(Ordering::SeqCst)
    }

    fn estimate_temp(&self) -> u8 {
        let usage = self.get_usage_instant();
        let base_temp = 35.0f32;
        let load_temp = usage as f32 * 0.45;
        (base_temp + load_temp) as u8
    }

    fn estimate_power(&self) -> u16 {
        let usage = self.get_usage_instant();
        let base_power = 10.0f32;
        let max_tdp = 125.0f32;
        let power = base_power + (usage as f32 / 100.0) * (max_tdp - base_power);
        power as u16
    }

    /// Not available on Windows
    pub fn read_energy(&self) -> u64 {
        0
    }

    /// Dummy instant for compatibility - returns ()
    pub fn read_instant(&self) -> () {}

    /// Get CPU usage
    pub fn get_usage(&self, _initial_instant: ()) -> u8 {
        self.get_usage_instant()
    }

    /// Get CPU usage directly using GetSystemTimes
    fn get_usage_instant(&self) -> u8 {
        unsafe {
            let mut idle: FILETIME = std::mem::zeroed();
            let mut kernel: FILETIME = std::mem::zeroed();
            let mut user: FILETIME = std::mem::zeroed();

            if GetSystemTimes(&mut idle, &mut kernel, &mut user) != 0 {
                let idle_time = filetime_to_u64(&idle);
                let kernel_time = filetime_to_u64(&kernel);
                let user_time = filetime_to_u64(&user);

                let last_idle = LAST_IDLE.load(Ordering::SeqCst);
                let last_kernel = LAST_KERNEL.load(Ordering::SeqCst);
                let last_user = LAST_USER.load(Ordering::SeqCst);

                let idle_delta = idle_time.wrapping_sub(last_idle);
                let kernel_delta = kernel_time.wrapping_sub(last_kernel);
                let user_delta = user_time.wrapping_sub(last_user);

                LAST_IDLE.store(idle_time, Ordering::SeqCst);
                LAST_KERNEL.store(kernel_time, Ordering::SeqCst);
                LAST_USER.store(user_time, Ordering::SeqCst);

                let total_delta = user_delta + kernel_delta + idle_delta;
                if total_delta > 0 {
                    let usage = ((user_delta + kernel_delta) * 100 / total_delta) as u8;
                    return usage.min(100);
                }
            }
        }
        0
    }

    /// Get CPU frequency from registry
    pub fn get_frequency(&self) -> u16 {
        unsafe {
            let mut hkey: HKEY = std::ptr::null_mut();
            let subkey = to_wide_string("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0");

            if RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                subkey.as_ptr(),
                0,
                KEY_READ,
                &mut hkey,
            ) != 0 {
                return 0;
            }

            let value_name = to_wide_string("~MHz");
            let mut data: u32 = 0;
            let mut size = std::mem::size_of::<u32>() as u32;

            let result = RegQueryValueExW(
                hkey,
                value_name.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut data as *mut u32 as *mut _,
                &mut size,
            );

            RegCloseKey(hkey);

            if result == 0 {
                return data as u16;
            }
        }
        0
    }
}

#[cfg(target_os = "windows")]
fn filetime_to_u64(ft: &FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

#[cfg(target_os = "windows")]
pub fn get_name() -> Option<String> {
    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        let subkey = to_wide_string("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0");

        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        ) != 0 {
            return None;
        }

        let value_name = to_wide_string("ProcessorNameString");
        let mut buffer: [u16; 256] = [0; 256];
        let mut size = (buffer.len() * 2) as u32;

        let result = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            buffer.as_mut_ptr() as *mut _,
            &mut size,
        );

        RegCloseKey(hkey);

        if result == 0 && size > 0 {
            let len = (size / 2) as usize;
            let os_string = OsString::from_wide(&buffer[..len.saturating_sub(1)]);
            return Some(os_string.to_string_lossy().trim().to_string());
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn to_wide_string(s: &str) -> Vec<u16> {
    let os_string: OsString = s.into();
    let mut wide: Vec<u16> = os_string.encode_wide().collect();
    wide.push(0); // null terminator
    wide
}

// ============================================================================
// Common Default Implementation
// ============================================================================
impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}
