#pragma once
#include "engine/LiquifyPreview.h"
#include <QRectF>
#include <QSGGeometryNode>
class QQuickWindow;

class LiquifyPreviewNode final : public QSGGeometryNode {
public:
    LiquifyPreviewNode();
    bool sync(QQuickWindow *window,const LiquifyPreviewFrame &frame,const QRectF &rect);
private:
    quint64 m_source=0,m_revision=0;
    QPoint m_presentationDelta;
    QRectF m_rect;
};
