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
        return None, None, None, None, None

    try:
        sys.path.insert(0, LHM_PATH)
        import clr
        clr.AddReference(os.path.join(LHM_PATH, 'LibreHardwareMonitorLib.dll'))
        clr.AddReference(os.path.join(LHM_PATH, 'HidSharp.dll'))

        from LibreHardwareMonitor.Hardware import Computer

        lhm = Computer()
        lhm.IsCpuEnabled = True
        lhm.IsGpuEnabled = True
        lhm.Open()

        cpu_temp_sensor = None
        cpu_power_sensor = None
        gpu_temp_sensor = None
        gpu_power_sensor = None

        # 查找 CPU 和 GPU 温度/功耗传感器
        for hardware in lhm.Hardware:
            hw_type = hardware.HardwareType.ToString()
            hardware.Update()

            if hw_type == 'Cpu':
                for sensor in hardware.Sensors:
                    sensor_type = sensor.SensorType.ToString()
                    name = sensor.Name.lower()
                    if sensor_type == 'Temperature' and cpu_temp_sensor is None:
                        if 'core' in name or 'package' in name or 'tdie' in name or 'tctl' in name:
                            # 检查值是否有效
                            val = sensor.Value
                            if val is not None and val > 0 and val < 150:
                                cpu_temp_sensor = sensor
                    elif sensor_type == 'Power' and cpu_power_sensor is None:
                        if 'package' in name or 'cpu' in name or 'soc' in name:
                            val = sensor.Value
                            if val is not None and val > 0 and val < 500:
                                cpu_power_sensor = sensor

            elif hw_type == 'GpuNvidia' or hw_type == 'GpuAmd':
                for sensor in hardware.Sensors:
                    sensor_type = sensor.SensorType.ToString()
                    name = sensor.Name.lower()
                    if sensor_type == 'Temperature' and gpu_temp_sensor is None:
                        if 'core' in name or 'gpu' in name:
                            val = sensor.Value
                            if val is not None and val > 0 and val < 150:
                                gpu_temp_sensor = sensor
                    elif sensor_type == 'Power' and gpu_power_sensor is None:
                        if 'package' in name or 'gpu' in name:
                            val = sensor.Value
                            if val is not None and val > 0 and val < 600:
                                gpu_power_sensor = sensor

        return lhm, cpu_temp_sensor, cpu_power_sensor, gpu_temp_sensor, gpu_power_sensor
    except Exception as e:
        print(f"INIT_ERROR:{e}", file=sys.stderr, flush=True)
        return None, None, None, None, None


def get_data(lhm, cpu_temp_sensor, cpu_power_sensor, gpu_temp_sensor, gpu_power_sensor):
    """获取温度和功耗数据"""
    cpu_temp = None
    cpu_power = None
    gpu_temp = None
    gpu_power = None

    if lhm:
        # CPU
        if cpu_temp_sensor:
            try:
                cpu_temp_sensor.Hardware.Update()
                val = float(cpu_temp_sensor.Value)
                if val is not None and val == val and 0 < val < 150:
                    cpu_temp = val
            except:
                pass

        if cpu_power_sensor:
            try:
                cpu_power_sensor.Hardware.Update()
                val = float(cpu_power_sensor.Value)
                if val is not None and val == val and 0 < val < 500:
                    cpu_power = val
            except:
                pass

        # GPU
        if gpu_temp_sensor:
            try:
                gpu_temp_sensor.Hardware.Update()
                val = float(gpu_temp_sensor.Value)
                if val is not None and val == val and 0 < val < 150:
                    gpu_temp = val
            except:
                pass

        if gpu_power_sensor:
            try:
                gpu_power_sensor.Hardware.Update()
                val = float(gpu_power_sensor.Value)
                if val is not None and val == val and 0 < val < 600:
                    gpu_power = val
            except:
                pass

    return cpu_temp, cpu_power, gpu_temp, gpu_power


def main():
    # 初始化
    lhm, cpu_temp_sensor, cpu_power_sensor, gpu_temp_sensor, gpu_power_sensor = init_lhm()

    if lhm:
        # 输出就绪信号
        print("READY", flush=True)

        # 等待 Rust 确认
        time.sleep(0.1)

        # 持续输出数据
        # 格式: CPU_TEMP:xx.x|CPU_POWER:xx.x|GPU_TEMP:xx.x|GPU_POWER:xx.x
        while True:
            try:
                cpu_temp, cpu_power, gpu_temp, gpu_power = get_data(
                    lhm, cpu_temp_sensor, cpu_power_sensor, gpu_temp_sensor, gpu_power_sensor
                )
                cpu_temp_str = f"{cpu_temp:.1f}" if cpu_temp is not None else "N/A"
                cpu_power_str = f"{cpu_power:.1f}" if cpu_power is not None else "N/A"
                gpu_temp_str = f"{gpu_temp:.1f}" if gpu_temp is not None else "N/A"
                gpu_power_str = f"{gpu_power:.1f}" if gpu_power is not None else "N/A"
                print(f"CPU_TEMP:{cpu_temp_str}|CPU_POWER:{cpu_power_str}|GPU_TEMP:{gpu_temp_str}|GPU_POWER:{gpu_power_str}", flush=True)
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
