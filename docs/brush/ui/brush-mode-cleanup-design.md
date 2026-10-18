# 双色绘制删除与纹理/双笔刷模式收窄

2026-10-18。按用户要求删除双色绘制入口，并把纹理与双重笔刷的覆盖模式限制为明确有用的集合。旧 state/VBR/ABR 仍可读取，Qt 载入旧预设时会剥离双色配置和退役模式，避免历史文件导致整个笔刷库无法打开。

## 双色绘制

设置页删除“双色绘制”子页、启用开关、前后顺序、溢出和贡献混合模式控件。载入旧 state、VBR 包或 native catalog 时，`dualBrush.color` 与 `wetMix.dualColor` 会被移除；实际绘画只使用前景色。底层 Rust 的旧双色解析代码暂保留，用于读取历史资源，不构成用户入口。

## 纹理模式

下拉和笔刷库校验只接受以下十种：

| 名称 | 代码 |
|---|---|
| 正片叠底 | `multiply` |
| 减去 | `subtract` |
| 变暗 | `darken` |
| 叠加 | `overlay` |
| 颜色减淡 | `colorDodge` |
| 颜色加深 | `colorBurn` |
| 线性加深 | `linearBurn` |
| 实色混合 | `hardMix` |
| 线性高度 | `linearHeight` |
| 高度 | `height` |

`height`/`linearHeight` 采用 Krita 遮罩纹理的高度覆盖公式，`depth` 作为纹理强度。旧值 `normal`、`linearDodge`、`lighten`、`softLight`、`screen` 等不在新集合内，载入时回落为 `multiply`。纹理深度、亮度、对比度、最低深度和深度动态仍保留。

## 双重笔刷模式

下拉和笔刷库校验只接受八种：

| 名称 | 代码 |
|---|---|
| 正片叠底 | `multiply` |
| 变暗 | `darken` |
| 叠加 | `overlay` |
| 颜色减淡 | `colorDodge` |
| 颜色加深 | `colorBurn` |
| 线性加深 | `linearBurn` |
| 实色混合 | `hardMix` |
| 线性高度 | `linearHeight` |

其他旧灰度组合在载入时回落为 `multiply`。双笔刷的尺寸、散布、形状抖动、资源和压力动态不变。

## 版本与兼容

这次不新增 DTO 字段；纹理 `height`/`linearHeight` 归入现有最新纹理模式范围（state31/VBR1.37），双重笔刷 `linearHeight` 归入现有扩展覆盖范围（state36/VBR1.43）。最新 Schema 已补充这两个枚举值。旧文件中的退役值由 Qt 在进入 FFI 前规范化，原生层仍需保留历史枚举以完成读取。
