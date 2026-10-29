# 方向角稳定验收（2026-10-29）

每笔新增私有方向anchor/滤波角；主副dab进入时以真实落点累计0.75px有效位移。初移模式此时锁定；绘制方向以最短角差、距离指数响应平滑（半径×0.3、长度1–8px）。只给笔尖几何用过滤角；散布/大小/流量等保留原方向。公共数据、参数和格式不变。

- 新核心两项：微抖不初始化/首次有效方向/新笔重置；跨±π连续/真实急转随位移收敛，build/direction-unit-tests.log。
- device11、direction3通过；新增实际像素对比两种模式在微抖/反向/拐弯轨迹的直接与单事件分页完全一致，build/direction-tests-final.log。
- dual12、dual_path7、taper11、outline_replay5、smudge4通过，build/direction-related-tests.log。
- release paint-ffi构建、核心严格Clippy/fmt、生成头/diff检查及标准Release构建、实际D3D11应用冒烟通过。未跑总check、所有笔刷矩阵或新增实体手绘笔体验验收。

与边框修复同包发布在build/bin/Release/drawverse.exe。先前GPU核优化仍在工作区；本节测试不取代其性能结论，不宣称角度处理端到端延迟改善。
