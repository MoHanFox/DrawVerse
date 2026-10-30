# 选区右键菜单（2026-10-30）

存在选区且文档空闲时，画布右键打开 GlassMenu。菜单绑定所属 CanvasPane 的 ownClient，浮动画布与多文档不通过全局活动客户端执行。仅接收右键，不截获绘画左键；编辑、文件操作及选区消失后关闭。右键本身不进入历史。

第一批实际接入取消选择、反向选择、自由变换、扭曲与透视；后三项进入原权威变换会话并设置现有控制层模式。参考图中羽化、通过复制/剪切创建图层、变换选区、填充与描边仍需核心事务，不能用整层复制或画笔替代，也不能用当前选区工具的羽化参数冒充已存在选区的羽化。

验收：无选区无菜单；有选区真实右键显示且不增加历史；取消选择执行核心操作并关闭菜单；变换沿既有会话/确认取消规则。新增 Qt 用例 selectionRightClickMenuUsesOwningDocumentAndDeselects；其余待补。

验证记录：qt_ui_tests Release 构建成功；offscreen/software 新右键用例通过（3 passed 含初始化和清理）。先前两种级联菜单行也通过。首次新增用例错误使用 QQuickItem::isEnabled() 检查 MouseArea 自有 enabled 属性失败，改读实际属性后通过，原日志 selection-context-validation.txt / selection-context-recheck.txt 保留在 build。最终日志 selection-context-final.txt。最终与调整画布一起复验软件及实际 D3D11 均通过；TapHandler 仅接收右键且不覆盖画布工具/空格导航光标。未做实体右键或参考图其余核心功能验收。

按用户后续要求移除右键“扭曲”和“透视”，保留取消选择、反向选择、自由变换。其余参考项仍待实际事务接入。
