# Melange DB 测试数据索引

## 测试记录

### 2025年9月17日 - Surface Book 2性能测试
- **文件**: [2025-09-17_surface_book_2_performance_test.md](2025-09-17_surface_book_2_performance_test.md)
- **硬件**: Intel Core i7-8650U / 16GB内存 / Windows 11
- **测试内容**: 高端移动设备AVX2优化验证测试
- **主要结果**: 写入3.25 µs/条，读取1.38 µs/条，AVX2指令集确认正常工作
- **特别提醒**: 测试在高性能电源模式下进行，节能模式性能可能下降

### 2025年9月16日 - 树莓派3B+性能测试
- **文件**: [2025-09-16_raspberry_pi_3b_plus_performance_test.md](2025-09-16_raspberry_pi_3b_plus_performance_test.md)
- **硬件**: ARM Cortex-A53 / 1GB内存 / Raspberry Pi OS
- **测试内容**: 针对低功耗ARM设备的适配优化测试
- **主要结果**: 写入39.04 µs/条，读取9.06 µs/条，成功适配低资源环境

### 2025年9月16日 - 低端x86设备性能测试
- **文件**: [2025-09-16_low_end_x86_performance_test.md](2025-09-16_low_end_x86_performance_test.md)
- **硬件**: Intel Celeron J1800 / 2GB内存 / Debian 12
- **测试内容**: 针对低端x86设备的性能优化测试
- **主要结果**: 写入9.13 µs/条，读取2.56 µs/条，优化效果显著

### 2024年9月15日 - Apple M1性能测试
- **文件**: [2024-09-15_apple_m1_performance_test.md](2024-09-15_apple_m1_performance_test.md)
- **硬件**: Apple M1 / 8GB内存 / macOS 15.6
- **测试内容**: 高端ARM设备性能基准测试
- **主要结果**: 写入1.23 µs/条，读取0.42 µs/条，相比RocksDB提升4倍写入性能

## 性能对比总结

| 设备类型 | 写入性能 | 读取性能 | 优化特点 |
|---------|---------|---------|---------|
| **Apple M1** | 1.23 µs/条 | 0.42 µs/条 | 高端ARM，最佳性能 |
| **Surface Book 2** | 3.25 µs/条 | 1.38 µs/条 | 高端x86，AVX2优化 |
| **Intel Celeron J1800** | 9.13 µs/条 | 2.56 µs/条 | 低端x86，智能flush优化 |
| **树莓派3B+** | 39.04 µs/条 | 9.06 µs/条 | 低功耗ARM，存储适配优化 |

## 测试环境说明
所有测试均在真实硬件环境下进行，记录了完整的硬件配置、测试参数和性能指标。测试结果用于指导Melange DB在不同硬件配置下的优化工作。