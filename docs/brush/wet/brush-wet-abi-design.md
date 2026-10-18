# 湿混原生入口 ABI1.50

2026-10-15。新增WET_BRUSH能力及不透明PaintWetTool：独立create/destroy、idle clean/read、使用准确prepared预设和32字节PaintWetLoadDesc手动装载。40字节PaintWetToolInfo返回初始化标志/载体revision/网格edge/中心线性预乘RGBA（不是整格平均）；busy/error输出清零，读取不等待准备。加载描述含struct_size/reserved/radius/reserved_tail/直通线性RGBA，范围与已有笔刷相同，预设必须启用湿混。

新增会话绑定命令，复制共享Arc，检查core/session归属；占用会话不可换工具，但不同闲置会话可绑定同一工具。销毁外部句柄只撤销令牌，不取消已绑定/在途使用。最多64个仍由句柄或会话持有的外部工具，Weak追踪避免销毁令牌绕过限制；令牌保留既有tombstone规则。默认会话独立工具保持兼容，文档历史不拥有工具。

新增显式prepare_state_wet及library_package_read_wet/write_wet入口，携带完整state17/VBR1.15。旧prepare/包入口继续拒绝新配置，不能让旧Qt丢字段。prepared state/pose/已有with_blend起笔复用准确票据，Engine执行配置湿混与新绑定工具。关闭湿混仍保存完整配置且走普通绘画。准备拒绝与资源交接规则不变，不在UI实现算法。

新增接口不扩写旧DTO，不改变现有调用顺序；状态/清洗/装载按worker-thread契约使用，取资源Arc后释放全局注册表锁。手动动作busy期间拒绝，错误参数/过期/错类型和跨core准确报错，无输出或旧状态污染。

验收：C/C++ DTO尺寸及导出真实调用；公开API真实prepare/捕获、手动清洗装载、共享绑定/销毁令牌寿命，真实异步两笔/像素历史与载色，busy/取消/错误/输出清零；state17/库包完整关闭配置及旧入口拒绝；限额含销毁后仍绑定者。Qt尚未接参数/基准/有色预览，本阶段不宣称用户界面完成。
