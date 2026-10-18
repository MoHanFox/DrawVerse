# 双笔刷状态与 VBR 交换验收

2026-10-14，state12 / VBR1.10 / behavior9 / dual-cover，ABI仍1.42。独立预设级dualBrush保留配对/路径模型、必需null间距及资源、六种模式、第二大小/笔尖/压感曲线/设备动态/散布/形状及完整seed、启用和关闭配置。Scalar数值f64范围检查后编译；只允许已执行的第二size/flow压力，不接受第二opacity/颜色模块。

主图片/纹理/第二图片/原始来源基线并集已接IO；同源PNG和覆盖Arc共享，目录预算包含周期mip。源ABR基线和原字节保持独立。Schema见 `core/crates/paint-io/tests/fixtures/schema-history/dual/vbr-dual-v1.10.schema.json`，PNG仍使用覆盖角色tip-mask；新清单6144/ZIP7169边界通过。旧对象保持最低版本。

通过的最小验证：

- `paint-io --test brush_dual_exchange` 3项：24组（配对/路径×启用/关闭×六模式）state12/VBR1.10两笔逐像素还原，完整第二动态/最低1%间距、三角色同源、预算边界和旧minor拒绝；模型/模式/舍入前范围/必需null字段/错误资源/能力版本/第二opacity拒绝；Schema模型与版本上限。
- `paint-io --test brush_dual_io` 3项：程序、非默认关闭及路径模型准确往返，state11拒绝新对象，默认不升级；匿名第二图片拒绝保存，来源基线继续拒绝丢配置；1024个复杂预设超过2MiB时编码明确返回Limit，避免写出无法重开的状态。
- 纹理交换3、来源包6、颜色交换5、旧状态4、VBR图片4项通过，本轮IO共28项。来源组合增加真实独立第二笔尖，与主图片/纹理同源，原始字节/报告/当前像素保持；1024条目边界增加独占第二PNG，4096PNG/6144资源/7169ZIP完整写入解码通过。
- `paint-ffi --test brush_source` 9项通过：新临时边界读取required=0、不改caller、不采用包内第二PNG到已绑定目录；写入不改已有目标。旧纹理、颜色、设备、散布、形状、压力和来源包接口回归通过。
- core/io/ffi all-target clippy `-D warnings`、cargo fmt、git diff --check通过。Schema已解析并核对实际模型；未运行第三方JSON Schema验证器，不将字段断言声称完整Schema验证。

Qt仍只保存state11，库package对dualBrush暂返回Unsupported，旧程序包接口仍明确拒绝。原生第二资源采用/闲置修订释放与Qt分类、真实预览、组合光标提示继续后续，不宣称原生用户入口完成。未运行全量tools/check.py、Qt构建、跨平台、实机或PS兼容验收；完整系统的平滑、湿混、ABR动态及VBR分组扩展继续后续。开发版未更新。
