# VibeBuddy OS Product Requirements Document

**文档编号**: PRD-VBOS-001
**版本**: v1.0
**状态**: Draft
**日期**: 2026-09-08
**负责人**: IClawMini 产品团队

> 本 PRD 由 `../../Vibebuddy os.md` 蓝图落地而来，是 VibeBuddy OS 多形态产品化的核心对齐文档。
> 技术架构详见 [docs/design/architecture.md](../design/architecture.md)。

## 1. 概述

### 1.1 文档目的

定义 VibeBuddy OS（VBOS）的产品需求。VBOS 是从 [Microduck](https://github.com/pollen-robotics/microduck)
软件分叉并重构而来的通用机器人操作系统，驱动 Vibe 家族全系列机器人（Vibe Chicken、Vibe Cat、Vibe Doge）。

### 1.2 适用范围

覆盖 VBOS v1.0 的全部功能需求、非功能需求及验收标准。

### 1.3 术语表

| 术语 | 定义 |
| - | - |
| **VBOS** | VibeBuddy OS，本项目的机器人操作系统 |
| **Daemon** | 守护进程，长期运行的后台服务 |
| **robotd** | 控制守护进程，唯一拥有电机控制权限的服务 |
| **HAL** | 硬件抽象层（Hardware Abstraction Layer） |
| **硬件描述文件** | 描述特定机型硬件拓扑的配置文件 |
| **Sim2Real** | 从仿真环境到真机部署的迁移流程 |
| **NDJSON** | Newline Delimited JSON，每行一个 JSON 对象 |

## 2. 产品定位

**一句话定位**: VibeBuddy OS 是 Vibe 机器人宇宙的"大脑"——驱动 Chicken 走路、Cat 爬行、Doge 卖萌的通用软件平台。

**愿景**: 成为桌面级机器人领域的"Linux + Arduino 综合体"——软件开源可定制，硬件模块可扩展。

### 2.1 目标用户

| 用户类型 | 描述 | 核心诉求 |
| - | - | - |
| RL 算法研究员 | 高校/企业强化学习研究者 | Sim2Real 训练管线、策略部署 |
| 机器人专业学生 | 自动化/机器人专业本科生 | 完整教学案例、可读代码 |
| 创客/极客 | 开源硬件爱好者 | 硬件改装、自定义行为开发 |
| STEAM 教育者 | 中小学/培训机构教师 | 低门槛、开箱即用、可视化界面 |

### 2.2 核心价值

1. **硬件无关**: 同一份 OS 镜像驱动 Chicken/Cat/Doge 多种形态
2. **服务化架构**: 独立守护进程，模块解耦，故障隔离
3. **安全优先**: 只有 robotd 拥有电机权限，所有指令经安全校验
4. **OTA 可靠**: 签名验证 + 自动回滚，杜绝"变砖"
5. **Sim2Real 就绪**: 内置 RL 训练管线，策略可热切换

## 3. 功能需求

### 3.1 核心服务模块（7 个守护进程）

| 服务 | 功能描述 | 优先级 |
| - | - | - |
| **robotd** | 50Hz 实时控制循环、RL 策略执行、安全控制、舵机/IMU 驱动 | P0 |
| **updaterd** | OTA 升级、签名验证、失败自动回滚、版本管理 | P0 |
| **configd** | Wi-Fi 配置、设备身份管理、网络状态 | P0 |
| **btd** | 蓝牙 LE 通信、手机配对与控制 | P1 |
| **padd** | 游戏手柄支持（USB/BLE） | P1 |
| **mediad** | 摄像头视频流、音频采集、WebRTC 推流 | P1 |
| **tofd** | 8×8 ToF 深度传感器数据读取与发布 | P2 |

> 代码实现：`daemons/` 目录即对应上述服务（`robotd/`、`updater/`、`configd/`、`btd/`、`padd/`、`mediad/`、`tof/`）。
> 本仓库从 Microduck 分叉，这些 crate 名与设计保持兼容。

### 3.2 硬件抽象层（HAL）

- **REQ-HAL-001**: 定义标准硬件接口 `Motor`、`Imu`、`Camera`、`ToF`
- **REQ-HAL-002**: 每种机型提供独立硬件描述文件（YAML/TOML）
- **REQ-HAL-003**: 描述文件包含舵机 ID 映射、IMU 方向、关节限位、默认 PID
- **REQ-HAL-004**: OS 启动时根据硬件描述文件加载对应驱动和参数
- **REQ-HAL-005**: 支持多硬件形态，通过描述文件适配不同机型

### 3.3 仿真与训练

- **REQ-SIM-001**: 基于 MuJoCo 的仿真环境（见 `docs/design/simulation.md`）
- **REQ-SIM-002**: 精确模拟电机摩擦、电池电压、控制延迟、舵机齿隙
- **REQ-SIM-003**: 支持 PPO 算法训练
- **REQ-SIM-004**: 领域随机化缩小 Sim2Real 差距
- **REQ-SIM-005**: 策略导出 ONNX 部署到真机
- **REQ-SIM-006**: 统一观测接口，多套策略可热切换（`robot.policy.switch`）

### 3.4 用户交互

- **REQ-UI-001**: 手机 App 通过 BLE 控制（行走、姿态、行为触发）
- **REQ-UI-002**: 手机 App 通过 WebRTC 查看实时摄像头
- **REQ-UI-003**: 游戏手柄控制（USB 有线 / BLE 无线）
- **REQ-UI-004**: 命令行工具 `vibectl`（laptop-side）与 `robotctl`（on-board）

## 4. 非功能需求

### 4.1 性能

| 指标 | 要求 | 优先级 |
| - | - | - |
| 控制循环频率 | ≥ 50 Hz，抖动 < 1ms | P0 |
| 舵机总线速率 | 1 Mbps | P0 |
| 策略推理延迟 | < 10ms（端到端） | P0 |
| 系统冷启动 | < 30 秒 | P1 |
| OTA 更新 | < 5 分钟 | P1 |

### 4.2 可靠性与安全

- robotd 崩溃自动重启，不影响其他服务（P0）
- OTA 失败 100% 自动回滚（P0）
- OTA 更新包数字签名验证（REQ-SEC-001）
- 只有 robotd 拥有电机控制权限（REQ-SEC-002）
- 舵机电压/温度实时监测 + 跌倒检测 + 关节限位（REQ-SEC-004/005）

### 4.3 兼容性

| 机型 | 规格 |
| - | - |
| Vibe Chicken | 15 DOF，双足 |
| Vibe Cat | 13+ DOF，四足/匍匐 |
| Vibe Doge | 7+ DOF + LED 表情屏 |

扩展新机型 ≤ 3 个月（通过硬件描述文件机制）。

## 5. 验收标准

| 类别 | 验收项 | 标准 |
| - | - | - |
| 功能 | robotd 50Hz 控制循环 | 连续 1 小时，抖动 < 1ms |
| 功能 | OTA 升级 | 升级+回滚各 10 次，成功率 100% |
| 功能 | 多形态 | 同一镜像在 Chicken/Cat/Doge 上均可启动并执行基础动作 |
| 功能 | Sim2Real | 仿真行走策略部署真机后可稳定行走 10 米 |
| 性能 | 策略推理 | < 10ms（RK3566） |
| 安全 | 安全层 | 非法指令 100% 被拦截 |

## 6. 路线图

| 版本 | 里程碑 | 核心交付 |
| - | - | - |
| v0.5 | 第 1-2 月 | 代码分叉、品牌替换、基础 HAL 定义 |
| v1.0-beta | 第 3-4 月 | 7 个守护进程重构完成、支持 Chicken 硬件 |
| v1.0 | 第 5-6 月 | 多形态支持（Cat/Doge）、OTA 完整流程、仿真环境 |
| v1.1 | 第 7-9 月 | 策略商店集成、社区贡献机制 |
| v2.0 | 第 10-12 月 | 完整 SDK、开发者工具链、企业级功能 |