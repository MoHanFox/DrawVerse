#pragma once
#include "PaintCoreClient.h"
#include <QQuickItem>
#include <QElapsedTimer>
#include <QTimer>
#include <QPointer>
class QScreen;

class CanvasItem : public QQuickItem {
    Q_OBJECT
    Q_PROPERTY(PaintCoreClient *client READ client WRITE setClient NOTIFY clientChanged)
    Q_PROPERTY(qreal zoom READ zoom NOTIFY viewChanged)
    Q_PROPERTY(QRectF documentRect READ documentRect NOTIFY viewChanged)
    Q_PROPERTY(QRectF visibleDocumentRect READ visibleDocumentRect NOTIFY viewChanged)
    Q_PROPERTY(bool interactive READ interactive WRITE setInteractive NOTIFY interactiveChanged)
    Q_PROPERTY(qreal initialFitRatio READ initialFitRatio WRITE setInitialFitRatio NOTIFY viewChanged)
    Q_PROPERTY(QRectF selectionPreview READ selectionPreview NOTIFY selectionDragChanged)
    Q_PROPERTY(int selectionPreviewKind READ selectionPreviewKind NOTIFY selectionDragChanged)
public:
    explicit CanvasItem(QQuickItem *parent = nullptr);
    ~CanvasItem() override;
    PaintCoreClient *client() const { return m_client; }
    void setClient(PaintCoreClient *client);
    qreal zoom() const { return m_zoom; }
    QRectF documentRect() const;
    QRectF visibleDocumentRect() const;
    bool interactive() const { return m_interactive; }
    void setInteractive(bool enabled);
    qreal initialFitRatio() const { return m_initialFitRatio; }
    void setInitialFitRatio(qreal ratio);
    QPointF documentPoint(QPointF local) const;
    Q_INVOKABLE void fitToView();
    Q_INVOKABLE void actualSize();
    Q_INVOKABLE void zoomBy(qreal factor);
    QRectF selectionPreview() const { return m_selecting ? QRectF(m_selectionStart,m_selectionEnd).normalized() : QRectF(); }
    int selectionPreviewKind() const { return m_selectionKind; }
    Q_INVOKABLE void cancelSelectionDrag();
signals:
    void clientChanged();
    void viewChanged();
    void interactiveChanged();
    void selectionDragChanged();
protected:
    QSGNode *updatePaintNode(QSGNode *old, UpdatePaintNodeData *) override;
    void geometryChange(const QRectF &geometry, const QRectF &old) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void mouseUngrabEvent() override;
    void wheelEvent(QWheelEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void keyReleaseEvent(QKeyEvent *event) override;
    bool eventFilter(QObject *watched, QEvent *event) override;
private:
    InputSample mouseSample(QPointF local, Qt::MouseButtons buttons);
    void refresh();
    void requestView();
    void observeScreen(QScreen *screen);
    void zoomAround(qreal factor,QPointF position);
    bool beginLayerMove(QPointF local);
    void finishLayerMove(QPointF local);
    bool beginSelection(QPointF local, Qt::KeyboardModifiers modifiers);
    void updateSelection(QPointF local);
    void finishSelection(QPointF local);
    PaintCoreClient *m_client = nullptr;
    QPointer<QQuickWindow> m_observedWindow;
    QImage m_image;
    QRectF m_imageRegion;
    quint64 m_generation = 0;
    QTimer m_viewTimer;
    QMetaObject::Connection m_dpiConnection;
    qreal m_zoom = 1;
    qreal m_initialFitRatio = 1;
    QPointF m_pan, m_last;
    QPointF m_moveStart;
    bool m_moving = false;
    quint64 m_moveLayer = 0, m_moveGeneration = 0;
    QPointF m_selectionStart, m_selectionEnd;
    quint64 m_selectionGeneration = 0;
    int m_selectionKind = 0, m_selectionOperation = 0;
    bool m_selecting = false;
    bool m_interactive = true, m_space = false, m_panning = false, m_stroke = false, m_tablet = false, m_fitPending = true;
    QElapsedTimer m_clock;
};
