# 复合笔尖原生几何验收

2026-10-15，Windows/MSVC，ABI1.49。仅相关最小回归，未运行完整tools/check.py。

- paint-core brush_composite_pose：3个用例通过。主几何与旧入口一致、第二关闭/开启、灰度/双色有效溢出、主/第二图片角色、独立旋转倾斜与能力回退、相对半径使用绑定主标称大小、固定边宽使用主实际压力、独立PCHIP曲线与关闭压感配置、无效几何输入拒绝。
- 内部composite_cursor_radius_matches_actual_base_dabs通过：2种尺寸模式×2种边宽规则×5个正负/零偏移×3个标称半径×4个压力，共240组；已知与未知悬停压力均和真实无抖动配对dab半径比较（按DTO f32精度）。
- paint-ffi brush_pose：2个用例通过。新旧读取同主姿态、复合64字节布局、两种压力已知状态/绑定半径覆盖、所有标志/向量及无效point错误清空两套结果；库销毁后保留捕获，clear与失效句柄清空。准备Busy分支若发生也检查清空，不声称强制触发Busy调度。
- C/C++静态/共享4项ABI smoke通过，新入口实际链接调用、64字节/字段偏移断言与clear后输出清零检查。
- Release构建、cbindgen生成头--check、paint-core/paint-ffi全目标clippy -D warnings通过。Qt旧动态数位笔/在途姿态与描边模板真实像素/历史2项回归通过。

日志：artifacts/brush-composite-pose-{core,dabs,ffi,clippy,header-check,build,abi}.log及brush-composite-pose-qt.txt。FFI调用者内存只经pointers读写，先清输出再验证，注册表锁在姿态snapshot前释放；不保留调用者内存。

这是基础几何接口阶段；Qt尚未接入双轮廓/第二图片轮廓，不包含随机散布/形状抖动、时间平滑队列与独立路径当前印章推演，也不宣称完整复合光标完成。湿混、收尖、非分隔绘制、高级ABR及其他全系统范围继续后续。
