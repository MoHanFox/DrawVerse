# Rust / C++ / QML 源码目录设计

日期：2026-10-18。目录按现有代码的责任组织；依赖事实以各 crate 的 `Cargo.toml`、Rust 模块声明和 `ui/CMakeLists.txt` 为准。目录本身不构成额外的编译隔离。

## 跨语言边界

2026-10-19 新增 `paint-ops`（有界算子 DTO、校验、CPU 数值执行与 WGSL 数据编码）和 `paint-gpu`（设备、队列、融合计算、读回及故障回退）。FFI 装配执行器，`paint-core` 仅依赖 `paint-ops`，不持有 wgpu 设备类型；液化的不可变源、网格、历史和发布归 `document/tools`。见[GPU 与液化设计](../tools/liquify-gpu-operators-design.md)。

```text
ui/src/app                         应用装配、QML 类型注册、启动及绘画基准入口
  ├─ documents / canvas            多文档生命周期、画布呈现及输入
  ├─ workspace / platform          停靠布局、原生窗口及菜单表面
  ├─ color                         Qt 颜色控件
  └─ engine / brushes              Qt 客户端、笔刷库与参数状态
           ↓ ui/include/paint_api.h（生成的唯一 C 边界）
core/crates/paint-ffi              ABI 校验、句柄、异步会话与服务装配
  ├─ paint-io                     有界格式、资源与原子文件操作
  │    ├─ paint-render             快照合成
  │    ├─ paint-task               取消与任务基础设施
  │    ├─ paint-core               文档、笔刷、图层与历史模型
  │    └─ paint-storage            临时磁盘资源管理
  ├─ paint-render → paint-core / paint-task
  ├─ paint-core → paint-storage / paint-ops
  ├─ paint-gpu → paint-ops          可选 wgpu 数值执行器，不管理文档
  ├─ paint-task
  └─ paint-storage
```

箭头表示调用/编译依赖方向。`paint-ffi` 直接依赖全部五个基础 crate。`paint-task` 和 `paint-storage` 不依赖其他项目 crate；`paint-core` 不依赖 IO、渲染、FFI 或 Qt。C++ 通过 C ABI 使用 Rust，不直接包含 Rust 实现。

## Rust：模型、文档执行、交换与边界各自负责

```text
core/crates/
├─ paint-core/src/
│  ├─ lib.rs                       公开类型门面、模块路由
│  ├─ brushes/
│  │  ├─ model/                    笔刷定义、程序、笔尖、纹理、双笔刷及光标形状
│  │  ├─ dynamics/                 压力/设备曲线、散布、随机与形状/传递抖动
│  │  ├─ stroke/                   输入调度、平滑、起收笔、喷枪的独立数值模型
│  │  ├─ wet/                      湿混载体、载色、设置及工具租约
│  │  └─ library/                  预设目录、分组与组织
│  ├─ document/
│  │  ├─ mod.rs                    权威文档、事务、活动笔触与历史提交
│  │  ├─ stroke/                   在文档上执行分页、双笔刷、湿混和起收笔
│  │  ├─ tools/                    油漆桶、魔棒、涂抹、液化的文档操作
│  │  └─ *.rs                      图层树操作、批量操作、变换
│  ├─ layers/                      图层数据、外观及剪贴规则
│  ├─ history/                     历史命令及历史磁盘后备
│  ├─ pixels/                      像素、瓦片与驻留分页
│  ├─ selection/                   选区几何、遮罩及组合
│  └─ error.rs                     模型公共错误
├─ paint-io/src/
│  ├─ lib.rs                       文件载入/保存入口、公共错误与限额
│  ├─ abr/                         Photoshop 描述符、资源解码、关联与转换
│  ├─ vbr_format/                  VBR 容器、图片、来源交接及分组包
│  ├─ brushes/
│  │  ├─ dynamics/                 动力学参数 DTO 与验证/转换
│  │  ├─ settings/                 双笔刷、湿混、纹理、平滑等设置 DTO
│  │  └─ library/                  目录状态、分组、资源与来源持久化
│  └─ document/                    ORA 与平面图片编解码
├─ paint-ffi/src/
│  ├─ lib.rs                       C 导出门面及 core/document 基础入口
│  ├─ abi/                         C DTO、指针访问、错误转换及基础 ABI 单元用例
│  ├─ engine/                      句柄注册、文档 actor、session 与存储配置入口
│  ├─ brushes/                     笔刷库、准备票据、资源及笔刷工具入口
│  ├─ document/                    图层、选区、变换与绘画工具入口
│  └─ files/                       文件入口及后台文件作业
├─ paint-render/src/               快照渲染，当前单一实现文件
├─ paint-task/src/                 通用线程池、串行 worker
└─ paint-storage/src/              临时磁盘资源，当前单一实现文件
```

`brushes` 保存可独立配置/编译的模型；`document/stroke` 负责将这些模型应用到真实瓦片，并承担活动笔触、失败回滚与单次历史提交。新增动力学数值不应直接放进文档事务文件，新增文档绘画执行也不应放进笔刷参数模型。IO 负责格式解释，FFI 负责调用边界，二者不承接新的绘画像素算法。

各目录的 `mod.rs` 声明它负责的模块。`lib.rs` 保留已有根类型和 `paint_io::brush_*` / `paint_io::abr_*` / `paint_io::vbr*` 路径，避免文件整理强迫调用方迁移。文档笔触和工具仍是文档子模块；少量 `#[path]` 将子模块定位到 `stroke/`、`tools/`，保持既有 `super`、私有状态和事务所有权。

Rust 集成测试和示例保留 Cargo 原生的 `tests/`、`examples/` 自动发现入口；外置单元测试随其实现移动。不新增仅为搬文件的 crate，也不拆开当前较小的单文件实现。

## C++：从应用装配到核心桥接

| 目录 | 负责 | 当前直接使用的项目目录 |
|---|---|---|
| `ui/src/app` | 应用启动、QML 注册、确定性预览及基准装配 | canvas、color、documents、workspace、platform、engine |
| `ui/src/documents` | Qt 文档条目、客户端生命周期 | engine、platform |
| `ui/src/canvas` | 输入坐标、画布显示、选区显示 | engine |
| `ui/src/engine` | Rust ABI 客户端、工作线程、发布帧及参数桥接 | brushes、`ui/include/paint_api.h` |
| `ui/src/brushes` | 笔刷库 Qt 状态与组织操作 | 本目录 |
| `ui/src/workspace` | 停靠树、工作区持久化、浮动窗口编排 | platform |
| `ui/src/platform` | 窗口拖动、原生菜单/模糊表面、UI 缩放 | 本目录 |
| `ui/src/color` | 色轮的 Qt Quick 绘制与交互 | 本目录 |

同一类的 `.h/.cpp` 放在同一目录。项目头统一用相对 `ui/src` 的显式路径，例如 `#include "engine/PaintCoreClient.h"`。CMake 仅提供 `ui/src` 这一项目源码包含根，不把全部子目录加入搜索路径，跨目录依赖可以从 include 直接读出。源文件仍显式列入 CMake，新增文件要在所属目标登记。

QML 类型名、C++ 注册名、`ui/include/paint_api.h` 及 Rust crate 名保持稳定。

## QML：应用装配、功能面板与公共基础组件

```text
ui/qml/
├─ app/                            Main、工作区状态栏；组合各功能入口
├─ workspace/                      停靠、面板容器、浮动面板、面板内容装配
├─ documents/                      文档标签区和文档窗口
├─ canvas/                         画布、变换/吸色叠加、透明网格
├─ brushes/                        画笔库、设置面板、压力/速度曲线编辑器
├─ layers/                         图层面板、锁和缩略图
├─ color/                          HSV 滑条
├─ tools/                          工具条、变体菜单和各工具选项栏
├─ preferences/                    偏好、存储设置和快捷键对话框
├─ controls/                       紧凑输入控件、图标、百分比与组合框弹窗
├─ menus/                          通用菜单、菜单项和独立子菜单窗口
├─ windows/                        通用窗口基类、边缘缩放和窗口描边
├─ theme/                          Theme 单例及其 qmldir
└─ assets/                         图片资源
```

`app` 和 `workspace/PanelContent.qml` 承担功能装配；功能面板引用基础控件，基础控件不导入业务面板。`controls/LayerPercent.qml` 已被图层与工具选项共用，因此归入公共控件，保留原类型名。`canvas/TransparencyGrid.qml` 被画布及图层预览共用。目录 import 的实际方向为：

```text
app → workspace / documents / canvas / tools / preferences / menus / controls / windows / theme
workspace → brushes / layers / canvas / color / tools / menus / controls / windows / theme
documents → tools / controls / windows / theme
brushes → controls / menus / theme
layers → canvas / controls / menus / theme
tools → controls / theme
canvas / preferences / controls / menus / windows → theme
color / theme → 无项目内 import
```

这些目录之间没有循环 import。每个 QML 文件只显式导入自己使用的其他目录，例如 `import "../controls"`；同目录类型依赖使用 QML 的本地解析。Theme 通过 `theme/qmldir` 注册单例，避免为每个功能复制主题对象。动态创建的 `PanelFlyout.qml` 和它的调用方同属 `workspace`。

物理路径与资源路径一致，应用入口为 `qrc:/qml/app/Main.qml`，C++ 启动与 Qt 用例一起更新；主窗 logo 改为相对 `app` 的 `../assets/logo.png`。CMake 中 `_drawverse_ui_resources` 同时供资源打包和重编译依赖使用，新增 QML/资源只登记一次，不维护隐藏的旧路径别名。

## contracts：只展示当前稳定边界

```text
contracts/
├─ README.md                       当前契约路由与版本口径
├─ ffi/                            ABI 所有权、线程、指针和缓冲区契约
│  └─ history/                     早期 ABI 头文件草案
└─ brush/
   ├─ vbr/                         VBR 1.0 格式与写出外壳 schema
   ├─ library/                     VBR 1.0 有序组树、包交接与示例
   ├─ wet/fixtures/                VBR 1.0 湿混和起收笔参数示例
   └─ abr/fixtures/                VBR 1.0 引用的 ABR 来源记录示例

core/crates/paint-io/tests/fixtures/schema-history/
└─ {vbr,dynamics,dual,texture,wet,abr}/   开发迭代快照，仅供旧格式回归
```

43 个开发迭代 schema 移出 contracts，保留在测试数据目录，测试编译引用同步迁移。避免把旧的局部字段快照改名后冒充当前完整协议。当前两个 VBR schema 明确只约束写出外壳；嵌套参数、缺项补全、资源和组织语义由 Rust 验证。当前交接 schema 只依赖当前预设外壳和组树。

当前契约使用无版本后缀的稳定文件名，描述统一为 VBR 1.0。湿混示例删除测试迭代号，ABR 示例以 `converter-N` 明确独立转换器版本；`$id` 和 `$ref` 保持引用身份。`vbr.rs` 统一定义 `VBR_MAJOR = 1` / `VBR_MINOR = 0`，程序与资源包写出共用。ABI/state/behavior/source/converter 保持独立版本。契约索引见 [contracts](../../contracts/README.md)。

## 验证入口

本次范围为可构建运行，执行记录见 [目录调整验收](../validations/platform/source-layout-validation.md)。常规完整验证仍使用根目录 `python tools/check.py`；环境初始化见 [构建复现](build-reproducibility.md)。目录调整不另增第二套验证框架。
