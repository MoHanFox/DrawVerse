# 二维湿混载色内核

2026-10-15。湿混依次实现二维数值载体、Document逐印章事务/预算和历史、预设/state/VBR/原生及Qt。当前阶段只提供真正可执行的独立Rust数值内核，不展示未接通的UI，不宣称Document湿混绘画或完整系统完成。无ABI/现有格式变化，不往旧BrushSettings静默添加会丢失的字段。

模型linear-premul-grid-v1，沿用线性预乘RGBA，明确不是颜料光谱或物理体积模型。WetMix为六个不可变0..1值：wetness、pickup、mix、load、deposit、consumption；所有数值严格有限。WetCarrier按初始标称直径1..512选择2..64方格，颜色随笔尖归一化坐标存储，压力尺寸/圆度/角度改变采样映射而不更换载色。颜色alpha同时表达可沉积的透明度/载量，不另乘一次alpha。初始及显式reload为前景RGBA乘load；clean为全透明；reset_path只清沿距离基准，允许每笔清洗/重新载色或保留多色的后续调度。

每个prepare_dab先完整读取所有格点，再产生沉积与下一载色状态，绝不修改当前载体或目标。调用者提供只读采样回调WetSample{color,pickup_weight}；weight为0..1的有效笔尖取样覆盖，与输出选区不同。先按移动距离消耗：decay=(1-consumption)^(distance/当前直径)，零距离为1、consumption=1且距离非零为0。carry=原载色×decay；pickup_t=wetness×pickup×weight；next=carry.mix(sample,pickup_t)；output=next.mix(sample,wetness×mix×weight)×deposit。所有mix为预乘RGBA线性插值，透明取样会稀释，载量/透明度不会被平方。乾笔（wetness=0）不访问取样回调。

输出WetDab.sample(u,v)双线性取样，仅归一化范围内有效，超范围透明。世界取样用当前笔尖中心/直径/圆度/角度映射格点中心。拾色/混合都不移除画布像素，不宣称质量守恒；后续绘画的流量、选区、整笔opacity、蒙版、锁和blend由Document统一应用，不在此重复套用。采样源（目标/可见合成/蒙版灰度）由下一阶段Document决定。

prepare携带基础修订和Arc身份；commit只接受同一载体基础状态的准备结果，过期/异源拒绝。清洗/重新载色/path reset也使旧准备失效，失败不改变状态。载体可clone作为事务快照，输出/next缓冲不可变，每个准备最多4096格，准备期间为原/下一/输出三个Pixel网格及一份WetSample源网格，源在Vec转Arc前释放，64阶约272KiB峰值（不含调用者另存的快照），不使用unsafe。下一阶段必须在写入/历史成功时提交载体，取消或暂存失败恢复像素和载体。

验收：二维红蓝分布而非均匀平均、旋转/圆度/压力映射、预乘透明颜色和透明底、干笔零取样、拾色/混合/沉积/消耗边界、距离拆分在f32容差内一致/驻点不消耗、全部读取在输出前完成且下一印章可取前次写入、回调/几何错误原子性、异源/过期及清洗重载、最大网格和内存边界。相关Rust测试/clippy；Document/历史、所有图层、选区、图层锁、VBR/ABR与Qt均仍待接入。
