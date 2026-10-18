# 第二笔尖相对尺寸原生与Qt验收

2026-10-15，ABI1.47/state15/VBR1.13/behavior12。新能力PAINT_FEATURE_DUAL_RELATIVE_SIZE，生成头更新，无DTO/导出签名变化；解除state准备/库包临时拒绝。Qt严格完整sizing镜像，完全默认省略，关闭/独立保留偏移和规则；snapshot/resetState/原生/包/预览最高版本保持。

- Qt dualSizingRetainedModesHighestVersionAndResetBaselines通过：关闭非默认、独立保留、副本完整重置基准、后续state14平滑行不降低state15、恢复、非法舍入前范围/类型/null及旧state14原子拒绝、VBR行为版本12与完整对象恢复。
- Qt dualSizingControlsPreviewCaptureExchangeAndReopen在DPR1/2通过：真实双笔刷开关与偏移输入、尺寸/宽度下拉的activated路径、窄列居中滑条/有效标称直径；真实例笔触预览变化；库包真实导出/独立客户端导入，同一实际笔触像素一致；中途改变偏移不影响捕获定义、单条历史/撤销重做、关闭配置磁盘保存与重开；QML无警告。
- 截图brush-dual-size-ui-1/2.png已检查，显示相对偏移-8px、有效标称4px、固定宽度与真实预览。没有实体数位板或跨平台交互证据，不把Qt软件窗口测试当作设备测试。
- 原生native_relative_sizing_preparation_capture_and_package_entry通过：准确state15准备/采用、编辑后旧捕获保留、含图片纹理资源导入、公开库包写入/文件恢复保持尺寸。原生测试先发现未绑定资源目录，补完整测试环境后通过。
- 相关Qt DPR2双笔刷图片/预览/像素历史/交换、平滑完整控件与保存、最高版本/重置三项通过。对应结果brush-dual-size-ui-1/2/related.txt；native/header/build/clippy/abi.log记录原生、生成头、Release应用、paint-ffi all-targets Clippy -D warnings及4个C/C++静态/共享ABI烟测通过。
- 应用未运行时更新开发目录exe/DLL，DLL与验证构建SHA256一致。

未运行全仓tools/check.py。界面尺寸与压力只声明灰度第二覆盖能力，双色贡献/上层下层模式/允许溢出/整笔分隔合成、自动描边预设及复合光标仍待完成；收尖、湿混和高级ABR也继续后续。
