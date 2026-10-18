#pragma once
#include <QString>
class PaintCoreClient;
class QQuickWindow;
void startPaintingBenchmark(PaintCoreClient &client,QQuickWindow &window,const QString &output);
