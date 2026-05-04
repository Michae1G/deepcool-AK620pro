"""
LibreHardwareMonitor 数据提供者
通过标准输出为 Rust 主程序提供真实的 CPU 温度和功耗数据
输出格式: TEMP:xx.x|POWER:xx.x
"""

import sys
import os
import time

# 添加 LibreHardwareMonitor DLL 路径
LHM_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'LibreHardwareMonitor')

def init_lhm():
    """初始化 LibreHardwareMonitor"""
    if not os.path.exists(LHM_PATH):
        return None, None, None
    
    try:
        sys.path.insert(0, LHM_PATH)
        import clr
        clr.AddReference(os.path.join(LHM_PATH, 'LibreHardwareMonitorLib.dll'))
        clr.AddReference(os.path.join(LHM_PATH, 'HidSharp.dll'))
        
        from LibreHardwareMonitor.Hardware import Computer
        
        lhm = Computer()
        lhm.IsCpuEnabled = True
        lhm.Open()
        
        temp_sensor = None
        power_sensor = None
        
        # 查找 CPU 温度和功耗传感器
        for hardware in lhm.Hardware:
            if hardware.HardwareType.ToString() == 'Cpu':
                hardware.Update()
                for sensor in hardware.Sensors:
                    sensor_type = sensor.SensorType.ToString()
                    name = sensor.Name.lower()
                    if sensor_type == 'Temperature' and temp_sensor is None:
                        if 'core' in name or 'package' in name or 'tdie' in name or 'tctl' in name:
                            temp_sensor = sensor
                    elif sensor_type == 'Power' and power_sensor is None:
                        if 'package' in name or 'cpu' in name or 'soc' in name:
                            power_sensor = sensor
        
        return lhm, temp_sensor, power_sensor
    except Exception as e:
        print(f"INIT_ERROR:{e}", file=sys.stderr, flush=True)
        return None, None, None


def get_data(lhm, temp_sensor, power_sensor):
    """获取温度和功耗数据"""
    temp = None
    power = None
    
    if lhm and temp_sensor:
        try:
            temp_sensor.Hardware.Update()
            val = float(temp_sensor.Value)
            if val is not None and val == val and 0 < val < 150:  # NaN check + range check
                temp = val
        except:
            pass
    
    if lhm and power_sensor:
        try:
            power_sensor.Hardware.Update()
            val = float(power_sensor.Value)
            if val is not None and val == val and 0 < val < 500:  # NaN check + range check
                power = val
        except:
            pass
    
    return temp, power


def main():
    # 初始化
    lhm, temp_sensor, power_sensor = init_lhm()
    
    if lhm:
        # 输出就绪信号
        print("READY", flush=True)
        
        # 等待 Rust 确认
        time.sleep(0.1)
        
        # 持续输出数据
        while True:
            try:
                temp, power = get_data(lhm, temp_sensor, power_sensor)
                temp_str = f"{temp:.1f}" if temp is not None else "N/A"
                power_str = f"{power:.1f}" if power is not None else "N/A"
                print(f"TEMP:{temp_str}|POWER:{power_str}", flush=True)
                time.sleep(1)
            except KeyboardInterrupt:
                break
            except Exception as e:
                print(f"ERROR:{e}", file=sys.stderr, flush=True)
                time.sleep(1)
        
        lhm.Close()
    else:
        print("NO_LHM", flush=True)
        sys.exit(1)


if __name__ == "__main__":
    main()
