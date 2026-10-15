# 湿混统一设置与载色策略验收

2026-10-15，Windows/MSVC/Rust及Qt6.8.3 Release。WetSettings进入Rust BrushSettings/BrushDefinition/BrushProgram，尚未接JSON/ABI/Qt设置页；本阶段只验证对应范围，未运行完整tools/check.py。

- 新增brush_wet_settings七项真实内核/文档回归全部通过，日志artifacts/brush-wet-settings-final.log。四种autoClean/autoLoad组合产生预期实际颜色和载色量，起笔捕获颜色；保留载体跨笔移动不消耗、大小改变重采样、load改变不缩放原持有量、下次装载使用新load；首次空载拾色/移动沉积/真实undo/redo，取消和错误输入整笔恢复且调用方载体不变。
- 同组验证二维网格新格点中心与旧网格双线性采样一致，同尺寸不改像素；旧预设修订与关闭参数完整捕获。配置笔触与旧手动载体入口的整条实际像素一致；关闭非默认策略保持普通笔刷像素。取样开关用两层实际拾色证明只写目标，蒙版合成取样及非Paint/有效双色/错误入口明确拒绝，不伪装成功。
- 既有brush_wet_carrier七项、brush_wet_color五项、brush_wet_document十二项通过，日志brush-wet-settings-core.log；brush_program三项、brush_mask五项通过，日志brush-wet-settings-legacy.log。默认笔刷、图片/纹理/动态、压感、灰度双路径、选区/蒙版/锁、分页/平滑和历史保持既有回归。
- IO新增两项通过（brush-wet-settings-io-new.log）：启用与关闭非默认湿混均被旧state及ABR来源单项/批次基线明确拒绝；恢复默认后state字节及旧状态往返保持。既有颜色交换五项通过（brush-wet-settings-io.log），覆盖真实state/VBR参数与像素。
- 原生native_smoothing_preparation_capture_and_package_entry通过（brush-wet-settings-native.log），验证准备捕获/当前设置变更隔离及包入口。去重47项相关Rust用例通过。为保持队列命令尺寸，Operation::Begin使用Box持有完整定义，Move不因预设扩大；无ABI布局/函数变化。
- paint-core/paint-ffi全目标clippy -D warnings、Release构建及构建内生成头--check、四项C/C++静态/共享ABI smoke通过，日志brush-wet-settings-{clippy,build,abi}.log。Qt真实颜色设置/捕获/预览/VBR/保存重开/绘画历史回归通过，日志brush-wet-settings-qt.txt。Qt部署工具提示SDK缺少translations/catalogs.json；构建使用--no-translations并成功，未宣称翻译部署已验收。

载体属于工具状态，文档undo/redo只恢复真实像素。私有重配/清洗/装载在起笔执行，只有成功end返回可采用的下一笔载体；下一笔配置不会倒改当前笔触。

后续仍需完整湿混state/VBR字段、关闭值和resetState基准、资源/来源交换、原生准备桥和Qt设置/真实有色预览、长期工具载体所有权与手动操作入口；有效双色湿混、收尖、非分隔绘制、高级ABR及设备/跨平台/性能验收尚未完成。本阶段不是用户可用的完整湿混工具。
