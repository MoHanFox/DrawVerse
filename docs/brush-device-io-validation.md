# 设备笔尖动态保存与交换验收

2026-10-14。状态v7和VBR1.5/behavior4/tip-dynamics已执行，tipDynamics独立于pressureDynamics。所有角度来源、倾斜圆度/最低比例可持久化，恢复时组合成同一内核配置；旧默认状态仍输出旧版本。来源converter1基线继续禁止动态编辑对象。

相关最小验证通过：

- paint-io `brush_device_io` 4项：四种角度来源+倾斜圆度与压力/不透明度共存，state/VBR恢复后与原始定义逐像素完全一致；只有几何时没有伪造压力载荷；旧状态不升级；坏枚举/布尔/范围/额外字段/null、state6和VBR1.4携带新数据、缺能力/错行为拒绝；公开Schema的枚举、单位、能力和原始来源角色检查。
- `brush_pressure` 5、`brush_state` 4、`vbr` 4、`vbr_image` 4、`vbr_source` 6项通过。新来源测试验证动态编辑后的图片笔触、完整原ABR字节、来源/图片引用保持；converter1原基线拒绝新对象。完整旧程序/图片/来源包的缺项、变更hash、路径、额外条目、取消/原子目标保护继续通过。
- 两处旧未来版本测试仍把已实现minor4/5当作未知版本，已改用公开MAX_EXECUTABLE_MINOR+1/+2，保留未来版本拒绝与全部资源损坏断言；设备数据降级测试仍明确使用minor4拒绝。
- paint-ffi `brush_source` 4项通过：旧压力/来源交换保持；新增新设备包在现有入口返回Unsupported，required清零、caller缓冲未动、原目标文件不覆盖。
- 核心/IO/FFI all-target clippy `-D warnings`通过。

ABI和Qt未升级：现有Qt模型只理解到state6/behavior3，package临时边界防止导入后丢失设置。本阶段没有新增Qt操作界面或动态光标，不能宣称GUI功能完成。下一阶段接原生能力/准确准备与Qt设置、实际输入光标，并解除文件入口临时边界。

Schema为 `contracts/vbr-tip-dynamics-v1.5.schema.json`，此轮Rust读取并检查公开字段，未运行第三方Schema验证器。未跑全量tools/check.py、Qt测试、设备实机、跨平台或性能基准。其余随机/纹理/散布/双笔刷/颜色/平滑/湿混、ABR动态与分组扩展继续后续，完整系统未完成。
