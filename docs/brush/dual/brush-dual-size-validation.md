# 第二笔尖相对尺寸内核验收

2026-10-14，ABI1.46/state14/VBR1.12不变。核心新增DualSizing/DualSizeMode/DualWidthRule及完整不可变捕获，独立/相对/跟随/固定宽度规则真实执行；不表示双色绘制或外描边已完成。

- brush_dual_size 4项：[-511,-8,0,8,511]直径偏移夹限，配对/独立路径与显式第二半径逐像素对照、输入拆分、单条历史及撤销重做；固定宽度在0/0.25/0.7/1压力下与显式实际第二半径对照，第二曲线保留但不重复执行；独立固定路径含主尺寸抖动拆分一致；非法值及关闭保留。
- sizing_tests 2项：独立步长、固定尺寸忽略第二大小抖动、主压力变化检测/最小半径；起笔独立第二覆盖场4096像素与真实主序号0形状样本的计划相同。
- brush_dual 9项、brush_dual_path 7项旧行为通过，含图片孔洞/方向/选区/蒙版/锁/实际像素、分页、取消/撤销重做、预算和存储失败原子回滚。
- brush_dual_size_boundary 1项：旧IO拒绝启用或关闭、独立或相对所保留的非默认尺寸配置；默认DTO绑定回核心保持默认。
- paint-core及paint-io all-targets Clippy -D warnings通过。对应日志brush-dual-size-core/unit/io/clippy.log。没有运行全仓tools/check.py或Qt交互测试，本阶段没有Qt新入口。
- Release Qt应用及Rust FFI构建通过（brush-dual-size-build.log）；应用未运行时更新开发目录exe/DLL，核对DLL与验证构建SHA256一致。

后续继续显式state/VBR版本、原生准备捕获、Qt相对滑条与有效尺寸显示；双色颜色贡献、叠放/溢出/整笔合成、自动描边预设及复合光标仍未完成。固定宽度的均匀描边保证依然只针对同步路径与几何，独立动态不承诺均匀轮廓。
