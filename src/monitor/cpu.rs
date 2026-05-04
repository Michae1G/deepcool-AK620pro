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
use std::sync::atomic::{AtomicU64, Ordering};
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
pub struct Cpu {
    temp_available: bool,
    power_available: bool,
}

#[cfg(target_os = "windows")]
impl Cpu {
    pub fn new() -> Self {
        Cpu {
            temp_available: false,
            power_available: false,
        }
    }

    pub fn warn_temp(&self) {
        if !self.temp_available {
            warning!("CPU temperature monitoring requires running as Administrator");
            eprintln!("         CPU temperature will be estimated based on usage.");
        }
    }

    pub fn warn_rapl(&self) {
        if !self.power_available {
            warning!("CPU power monitoring is not available on Windows");
            eprintln!("         CPU power will be estimated based on usage and TDP.");
        }
    }

    /// Get CPU temperature (estimated on Windows)
    pub fn get_temp(&self, fahrenheit: bool) -> u8 {
        let usage = self.get_usage_instant();
        let base_temp = 35.0f32;
        let load_temp = usage as f32 * 0.45;
        let temp = base_temp + load_temp;

        if fahrenheit {
            (temp * 9.0 / 5.0 + 32.0) as u8
        } else {
            temp as u8
        }
    }

    /// Not available on Windows
    pub fn read_energy(&self) -> u64 {
        0
    }

    /// Get CPU power (estimated on Windows)
    pub fn get_power(&self, _initial_energy: u64, _delta_millisec: u64) -> u16 {
        let usage = self.get_usage_instant();
        let base_power = 10.0f32;
        let max_tdp = 125.0f32;
        let power = base_power + (usage as f32 / 100.0) * (max_tdp - base_power);
        power as u16
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
