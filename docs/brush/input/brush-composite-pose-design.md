# 复合笔尖动态几何接口

2026-10-15，复合光标分两步接通。本阶段实现Rust共用几何与原生捕获读取；下一阶段接Qt主/第二图片轮廓和显示。ABI1.49新增PAINT_FEATURE_COMPOSITE_BRUSH_POSE与paint_brush_pose_read_composite，现有pose句柄/绑定/单笔尖入口不变，不增加state/VBR字段。

Core cursor_poses返回主/第二基础几何、图片标志和有效溢出。第二关闭时没有几何；开启时旧灰度覆盖及双色都能读取第二基础笔尖，但只有双色enabled且overflow允许显示超出主笔触的范围。相对跟随模式使用传入主标称半径加偏移并应用第二独立大小曲线；固定边宽先计算当前主压感半径再加偏移，不再套第二尺寸压感。未知悬停压力使用标称半径；方向/旋转/倾斜共用真实geometry，不抽取随机散布/形状抖动，也不模拟平滑队列或独立路径的后续印章。这是基础笔尖轮廓，不宣称表示全部随机印章/纹理和整笔面积。

PaintCompositeTipPose为64字节：struct_size/flags、两个已有24字节PaintTipPose、8字节保留。flags: SECONDARY=1、PRIMARY_IMAGE=2、SECONDARY_IMAGE=4、SECONDARY_OVERFLOW=8，其余位保留。没有第二时对应几何为零但嵌套struct_size有效；所有错误/Busy清除两套几何及flags。与现有入口相同的point/flags/vector校验，绑定非阻塞，不访问文件，不等待编译，资源与修订由已有pose捕获保持。库更新、销毁不改变已捕获结果；clear/destroy释放。

验收：旧主几何一致、关闭/独立/相对正负和夹限、两种压力规则/未知压力及传入主半径覆盖、方向与设备回退、第二图片/双色溢出标志；实际无抖动配对dab半径一致；FFI准备Busy/错误清零/旧修订与库销毁保持/释放及DTO布局，生成头检查、相关Rust/ABI smoke。Qt尚未使用时不得宣称复合光标已完成。
