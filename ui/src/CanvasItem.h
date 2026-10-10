#pragma once
#include "PaintCoreClient.h"
#include <QQuickItem>
#include <QVector>
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
    Q_PROPERTY(QRectF brushCursorRect READ brushCursorRect NOTIFY brushCursorChanged)
    Q_PROPERTY(bool brushCursorVisible READ brushCursorVisible NOTIFY brushCursorChanged)
    Q_PROPERTY(bool spacePanning READ spacePanning NOTIFY spacePanningChanged)
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
    QRectF brushCursorRect() const;
    bool brushCursorVisible() const;
    // Application wide space-pan state (combines the focused key with the global hold).
    bool spacePanning() const;
    bool interactive() const { return m_interactive; }
    void setInteractive(bool enabled);
    qreal initialFitRatio() const { return m_initialFitRatio; }
    void setInitialFitRatio(qreal ratio);
    QPointF documentPoint(QPointF local) const;
    /// Local position of a document point, including the view rotation. Only for drawing overlays
    /// that have to follow the rotated view; edits still go through documentPoint.
    Q_INVOKABLE QPointF scenePoint(QPointF document) const;
    /// View rotation in degrees; drawing and painting follow it, the document does not change.
    Q_PROPERTY(qreal viewRotation READ viewRotation WRITE setViewRotation NOTIFY viewChanged)
    qreal viewRotation() const { return m_viewRotation; }
    void setViewRotation(qreal degrees);
    Q_INVOKABLE void rotateViewBy(qreal degrees);
    Q_INVOKABLE void resetViewRotation();
    Q_INVOKABLE void fitToView();
    Q_INVOKABLE void actualSize();
    Q_INVOKABLE void zoomBy(qreal factor);
    Q_INVOKABLE void captureLayoutPosition();
    Q_INVOKABLE void restoreLayoutPosition();
    /// Normalized drag rectangle; Shift constrains it to a square (circle for the ellipse tool).
    QRectF selectionPreview() const { return m_selecting ? selectionPreviewRect() : QRectF(); }
    /// Shift constraint: keep the anchor corner and grow the shorter side to the longer one.
    static QRectF constrainedRect(QPointF anchor,QPointF corner);
    int selectionPreviewKind() const { return m_selectionKind; }
    Q_INVOKABLE void cancelSelectionDrag();
signals:
    void clientChanged();
    void viewChanged();
    void interactiveChanged();
    void selectionDragChanged();
    void brushCursorChanged();
    void spacePanningChanged();
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
    void hoverEnterEvent(QHoverEvent *event) override;
    void hoverMoveEvent(QHoverEvent *event) override;
    void hoverLeaveEvent(QHoverEvent *event) override;
    bool eventFilter(QObject *watched, QEvent *event) override;
private:
    InputSample mouseSample(QPointF local, Qt::MouseButtons buttons);
    void refresh();
    void requestView();
    void observeScreen(QScreen *screen);
    void refreshBrushCursor();
    void setSpacePanning(bool held);
    void setKeySpace(bool held);
    void zoomAround(qreal factor,QPointF position);
    bool beginLayerMove(QPointF local);
    void finishLayerMove(QPointF local);
    bool beginSelection(QPointF local, Qt::KeyboardModifiers modifiers);
    void updateSelection(QPointF local,Qt::KeyboardModifiers modifiers);
    QRectF selectionPreviewRect() const;
    /// Lasso path in document coordinates, for the client to rasterize on the session worker.
    Q_INVOKABLE QVariantList pathPoints() const;
    Q_PROPERTY(QVariantList pathPreview READ pathPoints NOTIFY selectionDragChanged)
    /// Eyedropper: sample the already rendered frame at a local point. Returns an invalid colour
    /// outside the document. Read-only, never touches history or the document revision.
    Q_INVOKABLE QColor pickColorAt(QPointF local) const;
    void finishSelection(QPointF local,Qt::KeyboardModifiers modifiers);
    PaintCoreClient *m_client = nullptr;
    QPointer<QQuickWindow> m_observedWindow;
    QImage m_image;
    QRectF m_imageRegion;
    quint64 m_generation = 0;
    QTimer m_viewTimer;
    QMetaObject::Connection m_dpiConnection;
    qreal m_zoom = 1;
    qreal m_viewRotation = 0;
    bool m_rotateViewing = false;
    qreal m_rotateStartAngle = 0, m_rotateStartRotation = 0;
    QPointF localCentre() const { return QPointF(width() / 2., height() / 2.); }
    qreal m_initialFitRatio = 1;
    QPointF m_pan, m_last;
    QPointF m_layoutOrigin;
    QPointer<QQuickWindow> m_layoutWindow;
    quint64 m_layoutGeneration=0;
    bool m_layoutCaptured=false;
    QPointF m_cursorPosition;
    bool m_cursorInside=false,m_capsLock=false;
    QPointF m_moveStart;
    bool m_moving = false;
    quint64 m_moveLayer = 0, m_moveGeneration = 0;
    QPointF m_selectionStart, m_selectionEnd;
    quint64 m_selectionGeneration = 0;
    int m_selectionKind = 0, m_selectionOperation = 0;
    bool m_selectionConstrained = false;
    // Lasso (3) / magic wand (4) capture state; `m_wandTolerance` mirrors the panel setting.
    QVector<QPointF> m_pathPoints;
    QPointF m_wandPoint;
    int m_pathKind = 0, m_pathOperation = 0, m_wandTolerance = 32;
    bool m_collectingPath = false;
    bool m_selecting = false;
    bool m_interactive = true, m_space = false, m_panning = false, m_stroke = false, m_tablet = false, m_fitPending = true;
    QElapsedTimer m_clock;
};
