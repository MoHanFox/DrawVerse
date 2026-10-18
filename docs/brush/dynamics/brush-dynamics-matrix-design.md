# 动态来源矩阵设计

2026-10-18。此前速度、切向压力、笔触距离、倾斜幅度、笔旋转和笔触方向六类动态同时启用时，尺寸、流量和整笔不透明度都会把六条曲线相乘。新矩阵让每个目标通道独立选择响应来源，同时保留默认的全来源行为。

## 语义

`DynamicsSourceMatrix` 包含 `size`、`flow`、`opacity` 三个选择，每个选择为：

- `All`：六个来源在该通道上的因子相乘，等价于旧行为。
- `Speed`、`Tangent`、`Distance`、`Tilt`、`Rotation`、`Direction`：只取所选来源在该通道上的因子；该通道曲线为恒等线性曲线时使用来源的归一化输入，避免把线性设备响应误判为无效果。

来源关闭时因子为 1；设备缺少倾斜、旋转或方向信息时也保持恒等。默认 `All` 下恒等曲线仍贡献 1，保持旧笔刷行为。压力曲线不进入矩阵，仍按原逻辑作用。矩阵只改变动态因子的来源，不改变整笔不透明度、流量累积、选区、蒙版或历史边界。

主笔尖与配对第二笔尖共用主预设的矩阵因子；第二笔尖自身仍拒绝非默认矩阵，避免双重选择。普通绘画、橡皮、干湿混和涂抹路径继续通过同一个 `factor_all` 入口取得三个目标因子。

## 保存与兼容

非默认矩阵写入画笔状态 `state32`，VBR 使用 `VBR1.38`、behavior25 和能力名 `dynamics-source-matrix`。对象为：

```json
{
  "model": "source-matrix-v1",
  "size": "speed",
  "flow": "distance",
  "opacity": "tilt"
}
```

默认 `All` 不写入对象，旧 state/VBR 的版本和字节保持不变。旧解码器遇到 state31/VBR1.37 中的 `sourceMatrix` 明确拒绝，避免静默丢失选择。Schema 为 `core/crates/paint-io/tests/fixtures/schema-history/dynamics/vbr-dynamics-matrix-v1.38.schema.json`。

动态专用预设必须走带版本资源的 image VBR 路径。编码器把 airbrush、六类动态和 sourceMatrix 都视为需要 image bundle 的对象；否则只有动态的包会误走旧平包并写出 minor0 manifest，无法表达对应版本门控。

## Qt

“传递”页增加尺寸、流量、不透明度三个来源下拉，默认“全部来源相乘”。选择立即进入当前预设、真实白色预览、复制/重置基准和持久化状态。预览状态构建提升到 state32，并与画布使用同一原生执行路径。

## 边界

本阶段没有把 ABR 的速度、距离、渐隐等来源映射进矩阵；矩阵只组合现有六类设备/笔触来源。实体数位板的倾斜、旋转和切向压力手感仍由用户验收，跨平台和 GPU 后端不在本阶段范围。
