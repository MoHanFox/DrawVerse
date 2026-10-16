# Windows悬停与模式菜单验收

2026-10-16。对应[Windows Ink设计](brush-windows-ink-hover-design.md)和[菜单路由设计](tablet-menu-routing-design.md)。Windows11 / Qt6.8.3 / Release / windows平台与software Quick后端。无Rust、ABI、state或VBR变化。

数位笔最后实际输入持有悬停位置，Quick缓存的默认鼠标Hover不能覆盖笔的位置、设备姿态或恢复系统箭头。真实鼠标Move/Press接管；数位笔绘画中的伴随鼠标保持隔离。离开后不被旧Hover重新显示，合并的零延迟任务只重申原生光标且释放时取消，不生成重绘循环。实体驱动未录制，该路径为与用户反馈吻合的代码与受控Qt回归。

模式菜单同时修复鼠标和数位笔：原生与兼容窗口完整登记、置顶、透明模态输入屏障，仍送原窗口的点击按全局坐标交给实际菜单；菜单外点击只关闭。兼容宿主不激活，避免ComboBox失焦立即关闭。测试把菜单移到真实文档上方，将事件故意送原窗口，要求模式真实切换、文档revision/modified/历史及真实像素保持，关闭后下一次独立落笔正常。

最小相关验证：

- Release构建及生成头字节检查通过（artifacts/windows-input-menu-release.log），临时QML输入诊断日志已移除。
- 环境倍率1：7个功能函数、含数据行及初始化清理13 passed、0 failed（artifacts/windows-input-menu-final-1.txt）。包括新帧同步悬停、原生/兼容×鼠标/数位笔四种菜单路由，既有菜单跨原生浮窗置顶及Escape/选择、直接Hover进入/窗口和Item入口、设备动态/在途姿态、可选复合光标。
- 环境倍率2：4个功能函数、含数据行及初始化清理9 passed、0 failed（artifacts/windows-input-menu-final-2.txt）。同样覆盖两个新问题、菜单四种配置及直接进入/鼠标接管/原光标恢复。
- 悬停回归使用实际Quick窗口Enter缓存及后续真实帧，并追加默认鼠标HoverEnter/Move/Leave。原生桌面鼠标移动仅在这一受控等待期间局部隔离；合成测试鼠标及所有Tablet/Quick Hover照常传递。原始光标恢复用应用过滤阶段探针精确断言，随后Qt为新目标解析的正常Cross等形状不强制等于之前的Wait；并独立核对窗口离开、隐藏和离开感应范围都不保留Blank。

环境倍率不等于实体硬件DPR；兼容路径在Qt6.8.3上通过强制禁用Popup.Window来验证，不代表实际Qt6.5或macOS/Linux已验证。未运行无关全量测试。Windows Ink实体数位板/驱动仍需用户实机复测，不能把受控Qt输入称为该硬件认证。完整笔刷系统其余范围保持进行中。
