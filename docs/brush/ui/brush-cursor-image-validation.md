# 半透明图片光标：数值与原生验收

2026-10-17，Windows。对应[设计](brush-cursor-image-design.md)，源码已包含在用户提交`646648b`中，本记录补齐实际验证。当前完成BrushMask显示图、资源缓存及ABI1.53原生查询；Qt图片显示、透明度偏好和当前/在途四身份消费尚未接入，不能据提交标题或本阶段测试宣称界面功能完成。

内核对基础双线性覆盖做有理面积积分，重复边界样本、单位直径居中透明留边，256×256 straight RGBA8/RGB固定白色。放大不采用方块状最近像素，缩小保留面积贡献；不是特定画布尺度/纹理/散布后的最终印章。每轴线性基函数积分采用整数权重，最大资源的u64累计、访问及输出有界。

资源按不可变ID缓存已编码PNG，8项/4MiB、单项512KiB；只保留编码结果，原BrushMask及mip仍可释放。缓存未命中走原内容哈希校验，读取/编码错误或取消不安装部分结果；命中复用已验证结果，不承诺每次重新检查磁盘。取消仅在有界生成/编码前后检查。

`paint_brush_library_tip_cursor_png`沿用caller-owned字节缓冲：容量查询/不足只返回完整required，不复制部分内容；其他失败required清零。新feature bit48与固定图边常量已发布，生成头来自paint-api-gen。没有新DTO或state/VBR变化，不修改图层、画布历史、预设修订或载色工具。

实际结果：

- 新内核5项通过：0/1/128/254/255覆盖、非方形完整256²像素对照双线性参考、奇数比例边缘积分对称、512²棋盘细节平均、4096×1/1×4096薄笔尖非零面积、权重单位分解及4096²最大资源不溢出。原覆盖保持不变，RGB全为白色。
- 资源9项通过（新2/旧7）：三种输入解释生成后PNG解码与内核全部像素一致；LRU淘汰/同身份Arc复用/原mask弱引用及驻留释放、取消、非法ID、缺失和损坏资源拒绝、修复磁盘后重新生成成功；原始PNG字节不变。旧资源导入/预算/轮廓用例保持。
- 公开资源ABI3项通过（新2/旧1）：1.53/bit48能力、字节结果、PNG尺寸、容量哨兵/无部分写入、空buffer及异常长度、错误required归零、预设数量/revision保持；源文件删除、原库销毁后共享资源库仍可查询，陈旧句柄拒绝。原图片导入/轮廓及资源共享用例通过。
- 旧Rust图片轮廓4项通过。paint-core/paint-io/paint-ffi全目标Clippy `-D warnings`通过。自动生成头已重新生成，Windows最终Release/头字节检查及4项C/C++ static/shared ABI通过；C/C++均调用新增接口检查旧库未绑定资源时的错误，C检查新版本/能力。
- 既有Qt两项通过（含init/cleanup为4 passed）：`imageBrushCursorPreservesHolesTransformsAndScreenWidth`与`compositeCursorOptionalPressureImagesAndCapturedStroke`，检查旧图片孔洞/变换/屏幕线宽、复合压力和在途图片捕获。它们仍显示旧轮廓，不是新增半透明图片Qt功能验收；未运行无关Qt全套、跨平台或整体硬件验收。

首轮新增ABI测试误用不存在的库信息count字段，编译明确失败，改为实际preset_count后通过。数值初稿为源常量格面积平均；复核放大小图片会产生方块后，在发布验收前改为基础双线性重建的精确面积积分，并重新运行核心/IO/公开接口及Clippy。没有把旧测试结果用于宣称新算法通过。

本机日志：`artifacts/brush-cursor-image-{core,io,public,clippy,header,old-outline,release}.log`，不入库。整体Windows设备手感由用户自行验收，跨平台后置。下一阶段接Qt偏好、后台身份与白色叠加，完成实际界面验证后再更新[剩余清单](../../planning/brush-system-remaining.md)为用户可用。
