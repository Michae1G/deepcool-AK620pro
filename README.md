# Table of Contents
- [About](#about)
- [Installation](#installation)
- [Supported Devices](#supported-devices)
    - [MYSTIQUE Series](#mystique-series)
- [Usage](#usage)
- [Automatic Start](#automatic-start)
    - [Windows](#windows)
    - [Systemd](#systemd-arch-debian-ubuntu-fedora-etc)
    - [OpenRC](#openrc-gentoo-artix-linux-etc)
- [Building from Source](#building-from-source)
- [Device List](#more-information)

# About
This is a **Windows-compatible fork** of the original `DeepCool Digital Linux` program.

This CLI program replicates the functionality of the original `DeepCool Digital` Windows program 
for Linux and now **Windows** as well.

## Windows Support
This fork adds native Windows support with:
- ✅ HID device communication via `hidapi`
- ✅ CPU usage monitoring via Windows API
- ✅ CPU frequency reading from registry
- ⚠️ CPU temperature (estimated based on usage, requires WMI/Admin for real values)
- ⚠️ CPU power (estimated based on TDP, as Windows doesn't expose RAPL)

> [!NOTE]
> For accurate temperature monitoring on Windows, run as Administrator.

# Installation

## Windows
1. Download the latest Windows release from [releases](https://github.com/Michae1G/deepcool-AK620pro/releases)
2. Run the executable (no installation required)
3. **Run as Administrator** for best results

## Linux
Simply download the latest [release](https://github.com/Nortank12/deepcool-digital-linux/releases)
and make it executable:
```bash
chmod +x deepcool-digital-linux
```
You will need root permission to send data to the device.

> [!TIP]
> For more accurate CPU temperature monitoring, you can use the [zenpower3](https://github.com/PutinVladimir/zenpower3)
> or [asus-ec-sensors](https://github.com/zeule/asus-ec-sensors) kernel modules on supported hardware.

> [!NOTE]
> On Intel's Arc GPUs, you have to use kernel version 6.13 or higher for proper temperature monitoring.

### Rootless Mode <sup>(optional - Linux only)</sup>
If you need to run the program without root privilege, you can create a `udev` rule to access all necessary resources as a user.

1. Locate your directory, it can be `/lib/udev/rules.d` or `/etc/udev/rules.d`
```bash
cd /lib/udev/rules.d
```
2. Create a new file called `99-deepcool-digital.rules`
```bash
sudo nano 99-deepcool-digital.rules
```
3. Insert the following:
```bash
# Intel RAPL energy usage file
ACTION=="add", SUBSYSTEM=="powercap", KERNEL=="intel-rapl:0", RUN+="/bin/chmod 444 /sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj"

# DeepCool HID raw devices
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3633", MODE="0666"

# CH510 MESH DIGITAL
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="34d3", ATTRS{idProduct}=="1100", MODE="0666"
```
4. Reboot your computer

# Supported Devices

### CPU Air Coolers
<table>
    <tr>
        <th>Name</th>
        <th>Supported</th>
    </tr>
    <tr>
        <td>AK620 DIGITAL PRO</td>
        <td align="center">✅</td>
    </tr>
    <tr>
        <td>AK500 DIGITAL PRO</td>
        <td align="center">✅</td>
    </tr>
    <tr>
        <td>AK400 DIGITAL PRO</td>
        <td align="center">✅</td>
    </tr>
    <tr>
        <td>AK620 DIGITAL</td>
        <td align="center">✅</td>
    </tr>
    <tr>
        <td>AK500 DIGITAL</td>
        <td align="center">✅</td>
    </tr>
    <tr>
        <td>AK400 DIGITAL</td>
        <td align="center">✅</td>
    </tr>
    <tr>
        <td>AG Series</td>
        <td align="center">✅</td>
    </tr>
    <tr>
        <td>ASSASSIN IV VC VISION</td>
        <td align="center">✅</td>
    </tr>
</table>

### CPU Liquid Coolers
<table>
    <tr>
        <th>Name</th>
        <th>Supported</th>
    </tr>
    <tr>
        <td>LD240/LD360</td>
        <td align="center">✅</td>
    </tr>
    <tr>
        <td>LP240/LP360</td>
        <td align="center">✔️</td>
    </tr>
    <tr>
        <td>LQ240/LQ360</td>
        <td align="center">✅</td>
    </tr>
    <tr>
        <td>LS520/LS720 SE DIGITAL</td>
        <td align="center">✅</td>
    </tr>
</table>

### Cases
<table>
    <tr>
        <th>Name</th>
        <th>Supported</th>
    </tr>
    <tr>
        <td>CH360/510/560 DIGITAL</td>
        <td align="center">✅</td>
    </tr>
    <tr>
        <td>CH170/270/690 DIGITAL</td>
        <td align="center">✔️</td>
    </tr>
    <tr>
        <td>MORPHEUS</td>
        <td align="center">✅</td>
    </tr>
</table>

**✅: Fully supported**

**✔️: Partially supported**<br>
*Some display modes are unavailable due to resource limitations.*

**⚠️: Not tested &nbsp; ❓: Not added**

# Usage

## Windows
Run as Administrator for best results:
```powershell
# Run directly
.\deepcool-digital-windows.exe

# Or with options
.\deepcool-digital-windows.exe --update 500 --fahrenheit
```

## Linux
You can run the program with or without providing any options.
```bash
sudo ./deepcool-digital-linux [OPTIONS]
```
```
Options:
  -m, --mode <MODE>       Change the display mode of your device
  -s, --secondary <MODE>  Change the secondary display mode of your device (if supported)
      --pid <ID>          Specify the Product ID if multiple devices are connected
      --gpuid <VENDOR:ID> Specify the nth GPU of a specific vendor to monitor (use ID 0 for integrated GPU)

  -u, --update <MILLISEC> Change the update interval of the display [default: 1000]
  -f, --fahrenheit        Change the temperature unit to °F
  -a, --alarm             Enable the alarm
  -r, --rotate <DEGREE>   Rotate the display (LP Series only)
  -z, --zeros             Display leading zeros (LD Series only)

Commands:
  -l, --list         Print Product ID of the connected devices
  -g, --gpulist      Print all available GPUs
  -h, --help         Print help
  -v, --version      Print version
```

# Automatic Start

## Windows
### Task Scheduler (Recommended)
1. Open Task Scheduler (`taskschd.msc`)
2. Create a new task:
   - **General**: Run with highest privileges
   - **Triggers**: At startup
   - **Action**: Start program → `deepcool-digital-windows.exe`
3. Enable the task

### Startup Folder (Simple)
Copy the executable to:
```
%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\
```
Note: This won't run as Administrator automatically.

## Systemd (Arch, Debian, Ubuntu, Fedora, etc.)
1. Copy the `deepcool-digital-linux` to the `/usr/sbin/` folder
```bash
sudo cp ./deepcool-digital-linux /usr/sbin/
```
2. Create the service file in the `/etc/systemd/system/` folder
```bash
sudo nano /etc/systemd/system/deepcool-digital.service
```
3. Insert the following:
```properties
[Unit]
Description=DeepCool Digital

[Service]
ExecStart=/usr/sbin/deepcool-digital-linux
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```
4. Enable the service
```bash
sudo systemctl enable deepcool-digital
```

## OpenRC (Gentoo, Artix Linux, etc.)
1. Copy the binary and create service file as shown above
2. Enable with:
```bash
sudo rc-update add deepcool-digital default
```

# Building from Source

## Windows
1. Install Rust: https://rustup.rs/
2. Install Visual Studio Build Tools (C++ toolchain)
3. Clone and build:
```powershell
git clone https://github.com/Michae1G/deepcool-AK620pro
cd deepcool-AK620pro
cargo build --release
```
The executable will be in `./target/release/deepcool-digital-linux.exe`

## Linux
### Dependencies
<details>
<summary><b>Arch-based distributions</b></summary>

```bash
sudo pacman -S base-devel rustup
```
</details>

<details>
<summary><b>Debian-based distributions</b></summary>

```bash
sudo apt install build-essential pkg-config libudev-dev curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
```
</details>

### Building
```bash
git clone https://github.com/Michae1G/deepcool-AK620pro
cd deepcool-AK620pro
cargo build --release
```

# More Information
[Device List and USB Mapping Tables](device-list/README.md)