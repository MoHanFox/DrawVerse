# 复刻主体落笔描边验收（2026-10-20）

对应[设计](../../../brush/core/brush-stroke-replay-outline-design.md)。

## 第一步：纯数值算子

paint-ops::replay_outline 已实现两尺度落笔、四通道灰度max并集、差集与可选主体几何裁剪。单次至多64×64、输出合成至多8Mi像素；返回私有候选，失败保留输入。共享来源复用主采样，两个通道均满覆盖时跳过无效重复计算。CPU参考，不依赖文档/Qt/颜色，未新增GPU计算。

7项新测试通过：Outside/Center/Inside圆环逐像素参考，重复和交叉印章顺序无关，独立形状允许溢出，亚像素软边变换，负世界坐标与分块一致，内半径退化，非法参数/中途采样失败不修改输入。paint-ops完整最小包共8单元+5算子+10 SDF回归通过；严格Clippy和格式通过。

当前尚未接入实际笔触、尾部回放、预览和设置；未移除旧UI/算法，state/VBR/ABI保持。不能把此步视作用户新描边需求已完成。Release程序仍为上一项通过的完整子树变换版本，未将未接入算子宣传为可用功能。

## 2026-10-21 核心实际印章接入

`document_stroke_replay_outline` 新5项通过：真实软边/旋转扁平/图片笔尖三位置逐像素与独立两尺度算子对照，单历史撤销重做；独立笔尖溢出与主体来源等价；压力/收笔/交叉路径分页与直接一致、取消无残留；低瓦片配额失败回滚；画布外几何不沿视口边缘描边。既有document_stroke_outline9、existing1、preview1及内部缓存/尾部3通过。paint-core/all-targets严格Clippy与fmt通过。ABI/格式未变，尚未触及Qt或重建Release，不把该小项称为UI描边完成。

## 2026-10-21 IO、Qt与Release

IO描边交换7项通过，新增四来源（含已有图片笔尖+纹理）×开关×溢出状态的state45/VBR37往返、真实像素一致，拒绝旧state携带新模型及缺失/null/错误类型allowOverflow；保留旧编码。原生async_session描边准备/真实像素/历史回归1项含新Replay通过。paint-io/paint-ffi全目标严格Clippy、fmt及生成头一致性通过。

Qt workflow软件/D3D11各8行通过：旧Path/Sdf迁移与失败原子性；真实鼠标点击溢出开关改变像素和标准预览、单笔历史及撤销重做；自身独立纹理新路径和旧SDF迁移两行；异步导入角色捕获；无主纹理选择描边纹理；整数控件及大范围像素滑条。UiTests原描边设置/复制重置、预览/像素/交换重开、资源迁移三项在软件/D3D11各通过。日志 `artifacts/replay-{workflow,ui}-{software,d3d11}.txt`；D3D11设置截图 `artifacts/replay-outline-d3d11.png`已检查，算法入口消失，溢出/大小及来源可见。

迁移测试初次失败是QVariant数字/null的JSON往返类型差异，改为按QJsonObject比较完整语义后通过，未删字段断言。完整Release及Qt目标构建、C11/C++20静态/共享四ABI通过；ABI1.75 bit63、state45、behavior37，产物 `build/qt/bin/Release`。未跑全量check/跨平台/实体连续输入；新功能限干画笔普通像素层、原预算保持，无新增GPU。旧UI入口已移除，原生旧算法兼容执行尚在，不宣称旧代码删除完成。
