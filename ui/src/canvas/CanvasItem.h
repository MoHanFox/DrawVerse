#pragma once
#include <QCursor>
#include "engine/PaintCoreClient.h"
#include <QQuickItem>
#include <QVector>
#include <QElapsedTimer>
#include <QTimer>
#include <QPointer>
class QScreen;
class QTabletEvent;
struct CanvasPresentationObservation;

class CanvasItem : public QQuickItem {
    Q_OBJECT
    Q_PROPERTY(PaintCoreClient *client READ client WRITE setClient NOTIFY clientChanged)
    Q_PROPERTY(qreal zoom READ zoom NOTIFY viewChanged)
    Q_PROPERTY(QRectF documentRect READ documentRect NOTIFY viewChanged)
    Q_PROPERTY(QRectF visibleDocumentRect READ visibleDocumentRect NOTIFY viewChanged)
    Q_PROPERTY(QRectF prefetchedDocumentRect READ prefetchedDocumentRect NOTIFY viewChanged)
    Q_PROPERTY(bool gpuDisplay READ gpuDisplay NOTIFY gpuDisplayChanged)
    Q_PROPERTY(bool forwardPreview READ forwardPreview NOTIFY residentPresentationChanged)
    Q_PROPERTY(int previewPart READ previewPart WRITE setPreviewPart NOTIFY residentPresentationChanged)
    Q_PROPERTY(QRectF presentationRect READ presentationRect WRITE setPresentationRect NOTIFY residentPresentationChanged)
    Q_PROPERTY(QRectF secondaryCursorRect READ secondaryCursorRect NOTIFY brushCursorChanged)
    Q_PROPERTY(qreal secondaryCursorAngle READ secondaryCursorAngle NOTIFY brushCursorChanged)
    Q_PROPERTY(qreal secondaryCursorRoundness READ secondaryCursorRoundness NOTIFY brushCursorChanged)
    Q_PROPERTY(bool secondaryCursorVisible READ secondaryCursorVisible NOTIFY brushCursorChanged)
    Q_PROPERTY(qreal brushCursorAngle READ brushCursorAngle NOTIFY brushCursorChanged)
    Q_PROPERTY(qreal brushCursorRoundness READ brushCursorRoundness NOTIFY brushCursorChanged)
    Q_PROPERTY(QRectF brushCursorRect READ brushCursorRect NOTIFY brushCursorChanged)
    Q_PROPERTY(bool brushCursorVisible READ brushCursorVisible NOTIFY brushCursorChanged)
    Q_PROPERTY(bool eyedropperCursorVisible READ eyedropperCursorVisible NOTIFY brushCursorChanged)
    Q_PROPERTY(bool samplingRingVisible READ samplingRingVisible NOTIFY brushCursorChanged)
    Q_PROPERTY(QColor sampledColor READ sampledColor NOTIFY brushCursorChanged)
    Q_PROPERTY(QColor previousSampleColor READ previousSampleColor NOTIFY brushCursorChanged)
    Q_PROPERTY(QPointF cursorPosition READ cursorPosition NOTIFY brushCursorChanged)
    Q_PROPERTY(bool spacePanning READ spacePanning NOTIFY spacePanningChanged)
    Q_PROPERTY(bool interactive READ interactive WRITE setInteractive NOTIFY interactiveChanged)
    Q_PROPERTY(qreal initialFitRatio READ initialFitRatio WRITE setInitialFitRatio NOTIFY viewChanged)
    Q_PROPERTY(QRectF selectionPreview READ selectionPreview NOTIFY selectionDragChanged)
    Q_PROPERTY(QVariantList selectionPreviewPoints READ selectionPreviewPoints NOTIFY selectionDragChanged)
    Q_PROPERTY(int selectionPreviewKind READ selectionPreviewKind NOTIFY selectionDragChanged)
public:
    explicit CanvasItem(QQuickItem *parent = nullptr);
    ~CanvasItem() override;
    PaintCoreClient *client() const { return m_client; }
    void setClient(PaintCoreClient *client);
    // GUI-thread opt-in; updatePaintNode observes only during Quick sync.
    void observePresentation(std::shared_ptr<CanvasPresentationObservation> observer) {m_presentationObserver=std::move(observer);}
    qreal zoom() const { return m_zoom; }
    QRectF documentRect() const;
    QRectF visibleDocumentRect() const;
    QRectF prefetchedDocumentRect() const;
    bool gpuDisplay() const;
    Q_INVOKABLE QVariantMap compositionStatistics()const;
    void observeCompositionReadback(std::shared_ptr<CompositionReadback> value){m_compositionCounters->readback=std::move(value);}
    bool forwardPreview() const {return !m_previewPart && m_residentPreview && m_residentPreview->forward;}
    int previewPart() const {return m_previewPart;}
    void setPreviewPart(int part) {if(part<0 || part>2 || part==m_previewPart)return;m_previewPart=part;refresh();emit residentPresentationChanged();}
    QRectF presentationRect() const {return m_presentationRect;}
    void setPresentationRect(QRectF rect) {m_presentationRect=rect;update();emit residentPresentationChanged();}
    QRectF brushCursorRect() const;
    QRectF secondaryCursorRect() const;
    qreal secondaryCursorAngle() const {return m_pose.secondaryAngle;}
    qreal secondaryCursorRoundness() const {return m_pose.secondaryRoundness;}
    bool secondaryCursorVisible() const;
    qreal brushCursorAngle() const {return m_pose.angle;}
    qreal brushCursorRoundness() const {return m_pose.roundness;}
    bool brushCursorVisible() const;
    bool eyedropperCursorVisible() const;
    bool samplingRingVisible() const;
    QColor sampledColor() const { return m_sampledColor; }
    QColor previousSampleColor() const { return m_previousSampleColor; }
    QPointF cursorPosition() const {return m_cursorPosition;}
    // Application wide space-pan state (combines the focused key with the global hold).
    bool spacePanning() const;
    bool interactive() const { return m_interactive; }
    void setInteractive(bool enabled);
    qreal initialFitRatio() const { return m_initialFitRatio; }
    void setInitialFitRatio(qreal ratio);
    QPointF documentPoint(QPointF local) const;
    Q_INVOKABLE void fitToView();
    Q_INVOKABLE void actualSize();
    Q_INVOKABLE void zoomBy(qreal factor);
    Q_INVOKABLE void captureLayoutPosition();
    Q_INVOKABLE void restoreLayoutPosition();
    /// Normalized drag rectangle; Shift constrains it to a square (circle for the ellipse tool).
    QRectF selectionPreview() const { return m_selecting ? selectionPreviewRect() : QRectF(); }
    /// Shift constraint: keep the anchor corner and grow the shorter side to the longer one.
    static QRectF constrainedRect(QPointF anchor,QPointF corner);
    QVariantList selectionPreviewPoints() const;
    int selectionPreviewKind() const { return m_selectionKind; }
    Q_INVOKABLE void cancelSelectionDrag();
signals:
    void residentPresentationChanged();
    void gpuDisplayChanged();
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
    void mouseDoubleClickEvent(QMouseEvent *event) override;
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
    friend class CompositionStabilityTests;
    void resetCompositionDevice();
    void compositionFallback(QString reason,CompositionPacketPtr packet);
    quint64 compositionDemand()const;
    void retryCompositionIfPossible();
    quint64 m_failedCompositionDemand=0,m_failedCompositionRelease=0,m_compositionRetries=0;
    bool m_failedCompositionPreview=false;
    CompositionPacketPtr m_compositionPacket;
    std::shared_ptr<CompositionCounters> m_compositionCounters=std::make_shared<CompositionCounters>();
    quint64 m_compositionEpoch=0;
    bool m_compositionFailed=false;
    QString m_compositionFailureReason;
    std::shared_ptr<CanvasPresentationObservation> m_presentationObserver;
    int m_previewPart=0;
    QRectF m_presentationRect;
    InputSample mouseSample(QPointF local, Qt::MouseButtons buttons);
    void refresh();
    void requestView();
    void observeScreen(QScreen *screen);
    void refreshBrushCursor();
    void setTabletWindowCursor(bool active);
    void updateTabletHover(QHoverEvent *event);
    bool ignoreCachedMouseHover(QHoverEvent *event);
    QTimer m_tabletCursorRefresh;
    bool m_penOwnsHover = false;
    QPointer<const QPointingDevice> m_penDevice;
    QCursor m_savedWindowCursor;
    void setSpacePanning(bool held);
    void setKeySpace(bool held);
    void zoomAround(qreal factor,QPointF position);
    void stopNavigation();
    void animateNavigation();
    void beginPan(QPointF position);
    void movePan(QPointF position);
    void endPan();
    bool beginLayerMove(QPointF local, bool pick = false);
    bool m_pickMoving = false;
    void finishLayerMove(QPointF local);
    void updateLayerMove(QPointF local);
    void cancelLayerMove(const char *reason="move.cancel.other");
    bool beginSelection(QPointF local, Qt::KeyboardModifiers modifiers);
    void updateSelection(QPointF local,Qt::KeyboardModifiers modifiers);
    QRectF selectionPreviewRect() const;
    /// Eyedropper: sample the already rendered frame at a local point. Returns an invalid colour
    /// outside the document. Read-only, never touches history or the document revision.
    Q_INVOKABLE QColor pickColorAt(QPointF local) const;
    void finishPolygon();
    void appendSelectionPoint(QPointF point);
    QVector<QPointF> m_selectionPoints;
    void finishSelection(QPointF local,Qt::KeyboardModifiers modifiers);
    PaintCoreClient *m_client = nullptr;
    QPointer<QQuickWindow> m_observedWindow;
    QImage m_image;
    quint64 m_imageRevision=0;
    QImage m_overview;
    ResidentPreviewPtr m_residentPreview;
    ResidentPreviewAssetsPtr m_movePreparation;
    bool m_moveWarmupPending=false;
    QRectF m_imageRegion;
    quint64 m_generation = 0;
    bool m_residentEditing=true;
    QSize m_displayDocumentSize,m_pendingDocumentSize;
    quint64 m_sizeRevision=0;
    QTimer m_liquifyTimer;
    QTimer m_viewTimer;
    QTimer m_navigationTimer;
    QElapsedTimer m_navigationClock, m_panClock;
    qreal m_zoomFrom=1, m_zoomTarget=1;
    qint64 m_zoomStarted=0, m_navigationLast=0, m_inertiaStarted=0;
    QPointF m_zoomAnchor, m_zoomDocument, m_velocity;
    bool m_zoomAnimating=false;
    QMetaObject::Connection m_dpiConnection;
    qreal m_zoom = 1;
    qreal m_initialFitRatio = 1;
    QPointF m_pan, m_last;
    QPointF m_layoutOrigin;
    QRectF m_layoutGeometry;
    QPointer<QQuickWindow> m_layoutWindow;
    quint64 m_layoutGeneration=0;
    bool m_layoutCaptured=false;
    void resetCursorSample();
    void setCursorSample(const InputSample &sample,const QPointingDevice *device,bool resetDirection=false);
    InputSample tabletSample(QTabletEvent *event,QPointF local) const;
    QPointer<const QPointingDevice> m_poseDevice;quint64 m_poseGeneration=0;
    InputSample m_poseSample;BrushCursorPose m_pose;QPointF m_poseDirection,m_poseDirectionAnchor;bool m_havePoseSample=false,m_poseDirectionKnown=false;QTimer m_poseRetry;
    QPointF m_cursorPosition;
    bool m_cursorInside=false,m_capsLock=false;
    bool m_picking=false;
    bool m_pendingSample=false;
    QPointF m_pendingSamplePoint;
    quint64 m_sampleGeneration=0,m_sampleRevision=0;
    QColor m_pendingSampleColor;
    QColor m_sampledColor, m_previousSampleColor;
    void beginPicking(QPointF local);
    void updatePicking(QPointF local);
    void endPicking(bool cancel=true);
    void refreshPendingSample();
    void updateSamplingDemand();
    QPointF m_moveStart;
    bool m_moving = false;
    quint64 m_moveGeneration = 0;
    QPointF m_selectionStart, m_selectionEnd;
    quint64 m_selectionGeneration = 0;
    int m_selectionKind = 0, m_selectionOperation = 0;
    bool m_selectionConstrained = false;
    /// Alt is held: the pointer acts as the eyedropper and shows a crosshair.
    bool m_altHeld = false, m_ctrlHeld = false;
    Qt::KeyboardModifiers m_cursorShortcutModifiers;
    QTimer m_shortcutStateTimer;int m_altReleaseTicks=0,m_ctrlReleaseTicks=0,m_spaceReleaseTicks=0;
    void setAltHeld(bool held);
    void setCtrlHeld(bool held);
    void syncShortcutModifiers(Qt::KeyboardModifiers modifiers,bool latchOnly);
    Qt::KeyboardModifiers shortcutCursorModifiers() const;
    Qt::KeyboardModifiers readShortcutCursorModifiers() const;
    QCursor effectiveCursor() const;
    void refreshShortcutCursor();
    void applyShortcutCursor();
    void syncWindowCursor();
    void updateCursorPoll();
    bool m_selecting = false;
    bool m_interactive = true, m_space = false, m_panning = false, m_stroke = false, m_tablet = false, m_fitPending = true;
    QElapsedTimer m_clock;
};
