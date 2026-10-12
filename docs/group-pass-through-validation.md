# 组穿透验收（2026-10-12）

按用户此前要求执行最小定向验证，没有运行 tools/check.py 全量检查。

- 核心：pass_through 集成测试 3/3，既有 standard_mix_reference_values_and_transparent_premultiplied_boundaries 1/1。覆盖颜色减淡对组外 backdrop、正常隔离对照、模式撤销/重做、组 opacity/fill 整体影响、逐行与单像素一致、嵌套隔离边界、组蒙版隐藏/撤销、组内剪贴链与未分组结果一致、穿透组作为剪贴基底及被剪贴、空组不改变背景，像素层/预览拒绝值 27。旧模式编号与数值边界保留，未知值从 28 起拒绝。
- IO：files 中 group 过滤的三项测试 3/3，加 hostile_or_unsupported 一项 1/1。穿透组透明度/fill 保存及加载保持自有 mode 27，XML isolation=auto，逐像素结果在 ORA 8-bit 精度容差内回环，外部 auto 正常组可导入；既有嵌套隔离、深度/未知 isolation/恶意 XML/非法像素层 mode 27 拒绝继续通过。
- 渲染：pass_through_switch_preview_and_mask_invalidate_cached_backdrop 1/1。缓存视口与直接 CPU 渲染在 128/16/1 尺寸一致；切换模式、撤销重做、蒙版修改更新真实像素，不复用过期隔离结果。
- FFI：pass_through_group_appearance_validation_publication_and_undo 1/1。ABI 1.19/能力位 65536、值 27 只允许组、实际 publication 查询及一次历史/撤销；像素层请求入队后明确 INVALID_ARGUMENT 且不增加历史。
- Qt：groupPassThroughIsGroupOnlyAndChangesBackdropWithUndo 一项在 Windows 原生窗口 100%/200% 缩放各通过（含初始化/清理各 3/3）。组菜单 28 项、像素层仍 27 项、组穿透真实帧与未分组颜色减淡结果一致、悬停预览不改变历史/修订、清除预览恢复隔离、提交/撤销/重做及像素层 UI 拒绝。无失败、跳过或 QML 警告，截图检查组 1 选中及穿透文字。

Rust fmt、core/ffi/io/render all-targets clippy -D warnings、Release 构建及生成头严格字节检查通过；ABI C/C++ smoke 编译链接，本轮未运行其测试。日志 artifacts/pass-build.log、pass-native-1.txt、pass-native-2.txt；截图 artifacts/pass-ui-1.png、pass-ui-2.png，生成物不纳入提交。

保留现有组默认正常/隔离，不迁移旧文件；使用方式是选中组，将混合模式设为“穿透”。穿透与普通组共用属性历史/现有布局，不新增虚假的 fx 实现。ORA 自有扩展精确保留穿透 opacity/fill；只解释标准 stack 的外部应用会按其 isolation/opacity 规则显示，可能与自有扩展合成不同，mergedimage.png 使用本应用的真实结果。本轮未做全量性能基准和独立真实硬件验收。
