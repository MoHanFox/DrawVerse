# 湿混双载色状态验收

2026-10-16，Windows。对应[设计](../../../brush/wet/brush-wet-pair-design.md)。新增 Rust WetInk 的主/可选第二载体、按角色起笔重配置/清洗/自动载色，WetTool 的完整状态快照、独占租约和原子采用，以及双角色手动装载。旧主载体接口仍保留第二载色，清洗同时处理两个现有角色。最多两个64阶载体，不保存到预设或历史。

最小相关验证通过：

- `brush_wet_pair` 新增5项：不同颜色/alpha/尺寸、保留载色时修改load不重乘；两条独立时间表的真实二维拾色/消耗/输出，对照两个原单载体，输出网格和载体采样逐值相等，另一角色完全不推进；非均匀二维结果保持。配置尺寸重采样及路径重置、新第二角色从透明开始、清洗两者；第二配置无效不改变原状态。跨线程共享租约Busy、取消不采用、成功整对采用、旧主接口和清洗、第二手动装载超界失败不发布主变化均通过。
- 既有 `brush_wet_settings` 7项、`brush_wet_tool` 3项、`brush_wet_color` 5项、`brush_wet_smudge` 6项通过；含实际文档笔触、载色所有权、取消、像素历史、组合算子和失败恢复。共26项Rust集成测试通过，未重跑无关全量。
- paint-core 全目标 Clippy（-D warnings）通过；Windows Release 和既有生成头一致性检查通过；C/C++ static/shared 四项 ABI 通过，无ABI/格式版本变化。
- Qt6.8.3 windows / software Quick / 环境倍率1：现有湿混设置页手动清洗/载色与画布隔离、湿混涂抹独立强度/手指策略和撤销两项通过（初始化清理共4 passed、0 failed）。本轮没有添加不存在的双色湿混界面测试。

日志：`artifacts/brush-wet-pair-{core,regression,clippy,release,abi}.log` 与 `artifacts/brush-wet-pair-qt.txt`，本机产物不入库。确认程序已关闭后同步开发版，包含前一轮空图层栈改动。跨平台和真实硬件验收后置。

本阶段是工具双载色事务的真实实现与数值验证，尚未将 WetInk 接入 Document 双贡献场、Engine 完整双载色结束返回、taper 双载色检查点或 IO/ABI/Qt组合入口，现有湿混双色拒绝仍保留。现有载体1–512尺寸域未扩大；后续实际第二相对尺寸/溢出需明确核对。不能据此宣布已能在画布画湿混双色或完整笔刷系统完成。
