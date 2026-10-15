#include "CanvasItem.h"
#include <QQuickWindow>
#include <QSGSimpleTextureNode>
#include <QTabletEvent>
#include <QMouseEvent>
#include <QWheelEvent>
#include <QKeyEvent>
#include <QHoverEvent>
#include <QGuiApplication>
#include <QPointingDevice>
#include <QScreen>
#include <QLineF>
#include <algorithm>
#include <cmath>
#ifdef Q_OS_WIN
#include <QtCore/qt_windows.h>
#endif

namespace {
class CanvasNode final : public QSGSimpleTextureNode {
public:
    qint64 imageKey = 0;
};
// Application-wide space-pan hold: the canvas that owns keyboard focus raises it, every canvas observes it.
bool spaceHeld=false;
}

CanvasItem::CanvasItem(QQuickItem *parent) : QQuickItem(parent) {
    setFlag(ItemHasContents); setAcceptedMouseButtons(Qt::LeftButton | Qt::MiddleButton);
    setCursor(Qt::CrossCursor);setAcceptHoverEvents(true);qApp->installEventFilter(this);
#ifdef Q_OS_WIN
    if(QGuiApplication::platformName()=="windows")m_capsLock=(GetKeyState(VK_CAPITAL)&1)!=0;
#endif
    setClip(true); m_clock.start();
    m_poseRetry.setInterval(16);connect(&m_poseRetry,&QTimer::timeout,this,&CanvasItem::refreshBrushCursor);
    m_liquifyTimer.setInterval(16);
    connect(&m_liquifyTimer,&QTimer::timeout,this,[this]{if(!m_stroke || !m_client || !m_client->liquifyActive() || !isEnabled() || !isVisible()){m_liquifyTimer.stop();return;}m_client->liquifyTick();});
    m_viewTimer.setSingleShot(true); m_viewTimer.setInterval(16);
    connect(&m_viewTimer, &QTimer::timeout, this, &CanvasItem::requestView);
    connect(this, &CanvasItem::viewChanged, this, [this] { if (!m_viewTimer.isActive()) m_viewTimer.start(); });
    connect(this,&CanvasItem::viewChanged,this,[this]{if(m_havePoseSample && (!m_client || !m_client->drawing()))m_poseSample.position=documentPoint(m_cursorPosition);refreshBrushCursor();});
    const auto observeWindow=[this](QQuickWindow *window) {
        if (m_observedWindow) { m_observedWindow->removeEventFilter(this); disconnect(m_observedWindow, nullptr, this, nullptr); }
        m_observedWindow = window;
        if (window) {
            connect(window, &QQuickWindow::screenChanged, this, &CanvasItem::observeScreen);
        }
        observeScreen(window ? window->screen() : nullptr);
        m_viewTimer.start();
    };
    connect(this, &QQuickItem::windowChanged, this, observeWindow);
    connect(this,&QQuickItem::enabledChanged,this,[this] {if(!isEnabled())endPicking();});
    connect(this,&QQuickItem::visibleChanged,this,[this] {if(!isVisible())endPicking();});
    observeWindow(window());
}
void CanvasItem::observeScreen(QScreen *screen) {
    disconnect(m_dpiConnection);
    if (screen) m_dpiConnection = connect(screen, &QScreen::logicalDotsPerInchChanged, this, [this] { m_viewTimer.start(); });
    m_viewTimer.start();
}
CanvasItem::~CanvasItem() {
    qApp->removeEventFilter(this);
    if (m_observedWindow) m_observedWindow->removeEventFilter(this);
    if (m_client && !m_interactive) m_client->requestViewport(1, {}, {}, false);
}
void CanvasItem::setClient(PaintCoreClient *client) {
    if (m_client == client) return;
    endPicking();
    if (m_client) disconnect(m_client, nullptr, this, nullptr);
    m_poseRetry.stop();m_havePoseSample=false;m_poseDirectionKnown=false;m_client = client; m_fitPending = true;
    if (client) {
        connect(client, &PaintCoreClient::liquifyOptionsChanged,this,&CanvasItem::refreshBrushCursor);
        connect(client, &PaintCoreClient::stateChanged,this,&CanvasItem::refreshBrushCursor);
        connect(client, &PaintCoreClient::brushCursorPreferencesChanged,this,&CanvasItem::refreshBrushCursor);
        connect(client, &PaintCoreClient::frameChanged, this, &CanvasItem::refresh);
        connect(client, &PaintCoreClient::brushChanged, this, [this] { cancelSelectionDrag();if(m_picking && !m_altHeld && !m_client->eyedropperTool())endPicking();refreshBrushCursor(); });
        connect(client, &PaintCoreClient::stateChanged, this, [this] { refresh(); emit viewChanged(); });
        connect(client, &PaintCoreClient::samplingRingEnabledChanged, this, &CanvasItem::refreshBrushCursor);
        connect(client, &QObject::destroyed, this, [this] { m_client = nullptr;endPicking();refreshBrushCursor(); });
        refresh();
    }
    refreshBrushCursor();emit clientChanged();
}
void CanvasItem::setInteractive(bool enabled) {
    if(!enabled)endPicking();
    m_interactive = enabled; setAcceptedMouseButtons(enabled ? Qt::LeftButton | Qt::MiddleButton : Qt::NoButton);
    m_fitPending = true; refresh();refreshBrushCursor(); emit interactiveChanged(); emit viewChanged();
}
QRectF CanvasItem::brushCursorRect() const {
    const qreal radius=m_client?(m_client->liquifyActive()?m_client->liquifySize()*.5:m_client->brushLibrary()->cursorSimple()?m_client->baseCursorPose().radius:m_pose.radius)*m_zoom:0;
    return {m_cursorPosition-QPointF(radius,radius),QSizeF(radius*2,radius*2)};
}
QRectF CanvasItem::secondaryCursorRect() const {
    const auto radius=m_client?m_pose.secondaryRadius*m_zoom:0;
    return {m_cursorPosition-QPointF(radius,radius),QSizeF(radius*2,radius*2)};
}
bool CanvasItem::secondaryCursorVisible() const {
    return brushCursorVisible() && m_client && !m_client->eraser() && !m_client->smudgeTool() && !m_client->liquifyActive() && m_client->brushLibrary()->cursorOutline() && m_client->brushLibrary()->cursorComposite() && m_pose.ready && m_pose.secondary;
}
bool CanvasItem::brushCursorVisible() const {
    if(m_client && !m_client->liquifyActive()){
        const auto *library=m_client->brushLibrary();
        if((!library->cursorOutline() && !library->cursorCenter()) || (library->cursorHoverOnly() && m_client->drawing()))return false;
    }
    // Alt is the eyedropper: the pointer becomes a crosshair and the brush ring goes away, so the
    // user can see exactly which pixel will be sampled.
    return m_cursorInside && !(m_client && m_client->transformActive()) && !m_capsLock && !m_altHeld && !m_ctrlHeld && !m_moving && m_interactive && m_client && (m_client->liquifyActive() || (!m_client->eyedropperTool() && !m_client->bucketTool() && !m_client->moveTool() && !m_client->selectionTool())) && !m_space && !spaceHeld && !m_panning && isVisible();
}
bool CanvasItem::spacePanning() const { return m_space || spaceHeld; }
bool CanvasItem::eyedropperCursorVisible() const {
    return m_cursorInside && m_interactive && m_client && !m_client->liquifyActive() && (m_altHeld || m_client->eyedropperTool()) && !m_client->selectionTool()
        && !m_client->moveTool() && !spacePanning() && !m_panning && isVisible();
}
bool CanvasItem::samplingRingVisible() const {
    return m_picking && m_sampledColor.isValid() && m_client && m_client->samplingRingEnabled() && eyedropperCursorVisible() && documentRect().contains(m_cursorPosition);
}
void CanvasItem::beginPicking(QPointF local) {
    m_previousSampleColor=m_client->brushColor();m_picking=true;updatePicking(local);
}
void CanvasItem::updatePicking(QPointF local) {
    m_cursorPosition=local;m_cursorInside=contains(local);m_sampledColor=pickColorAt(local);
    if(m_sampledColor.isValid())m_client->setBrushColor(m_sampledColor);
    refreshBrushCursor();
}
void CanvasItem::endPicking() {
    if(!m_picking)return;
    m_picking=false;m_tablet=false;m_sampledColor=QColor();emit brushCursorChanged();
}
void CanvasItem::setCursorSample(const InputSample &sample,bool resetDirection){
    if(resetDirection){m_havePoseSample=false;m_poseDirectionKnown=false;}
    if(m_havePoseSample){const auto delta=sample.position-m_poseSample.position;if(std::hypot(delta.x(),delta.y())>1e-9){m_poseDirection=delta;m_poseDirectionKnown=true;}}
    m_poseSample=sample;m_havePoseSample=true;
}
InputSample CanvasItem::tabletSample(QTabletEvent *e,QPointF local) const {
    InputSample s;s.position=documentPoint(local);s.pressure=static_cast<float>(e->pressure());s.tiltX=static_cast<float>(e->xTilt());s.tiltY=static_cast<float>(e->yTilt());s.rotation=static_cast<float>(e->rotation());s.tangentialPressure=static_cast<float>(e->tangentialPressure());s.buttons=static_cast<quint32>(e->buttons());s.tool=e->pointerType()==QPointingDevice::PointerType::Eraser?2:1;s.timestamp=static_cast<quint64>(m_clock.nsecsElapsed());s.capabilities=16;
    const auto cap=e->pointingDevice()->capabilities();if(cap.testFlag(QInputDevice::Capability::Pressure))s.capabilities|=1;else s.pressure=1;if(cap.testFlag(QInputDevice::Capability::XTilt)||cap.testFlag(QInputDevice::Capability::YTilt))s.capabilities|=2;if(cap.testFlag(QInputDevice::Capability::Rotation))s.capabilities|=4;if(cap.testFlag(QInputDevice::Capability::TangentialPressure))s.capabilities|=8;return s;
}
void CanvasItem::refreshBrushCursor() {
    if(m_client){m_pose=m_client->baseCursorPose();if(m_havePoseSample && brushCursorVisible() && !m_client->liquifyActive())m_pose=m_client->brushCursorPose(m_poseSample,m_poseDirection,m_poseDirectionKnown,m_client->drawing() && !m_client->brushLibrary()->cursorSimple());}
    if(m_client && !m_pose.ready && m_havePoseSample && brushCursorVisible() && !m_client->liquifyActive())m_poseRetry.start();else m_poseRetry.stop();
    setCursor(m_space || spaceHeld || m_panning?Qt::OpenHandCursor:m_client && (m_client->moveTool() || m_ctrlHeld || m_moving) && !m_client->liquifyActive() && !m_client->transformActive()?Qt::SizeAllCursor:eyedropperCursorVisible()?Qt::BlankCursor:brushCursorVisible()?Qt::BlankCursor:Qt::CrossCursor);
    emit brushCursorChanged();
}
void CanvasItem::setSpacePanning(bool held) {
    // Space panning is application wide: every canvas reports the shared hold.
    const bool changed=(m_space || spaceHeld)!=(m_space || held);spaceHeld=held;
    if(changed)emit spacePanningChanged();
}
void CanvasItem::setKeySpace(bool held) {
    // The focused canvas owns this half of the state; the global hold covers every other canvas.
    const bool changed=(m_space || spaceHeld)!=(held || spaceHeld);m_space=held;
    if(changed)emit spacePanningChanged();
}
namespace {
bool tabletHoverEvent(const QHoverEvent *event) {
    const auto *device=event->pointingDevice();
    return device && (device->type()==QInputDevice::DeviceType::Stylus || device->type()==QInputDevice::DeviceType::Airbrush);
}
}
void CanvasItem::hoverEnterEvent(QHoverEvent *event){if(m_tablet || tabletHoverEvent(event))return;m_cursorInside=true;m_cursorPosition=event->position();setCursorSample(mouseSample(event->position(),Qt::NoButton));refreshBrushCursor();}
void CanvasItem::hoverMoveEvent(QHoverEvent *event){if(m_tablet || tabletHoverEvent(event))return;m_cursorInside=true;m_cursorPosition=event->position();setCursorSample(mouseSample(event->position(),Qt::NoButton));refreshBrushCursor();if(m_selecting && m_selectionKind==3)updateSelection(event->position(),event->modifiers());}
void CanvasItem::hoverLeaveEvent(QHoverEvent *event){if(m_tablet || tabletHoverEvent(event))return;m_cursorInside=false;refreshBrushCursor();}
void CanvasItem::refresh() {
    if (!m_client) return;
    if (m_generation != m_client->generation()) { endPicking();cancelSelectionDrag(); m_generation = m_client->generation(); m_fitPending = true; }
    const int view = m_interactive ? 0 : 1;
    m_image = m_client->frame(view); m_imageRegion = m_client->frameRegion(view);
    if (m_fitPending) fitToView();
    update();
}
void CanvasItem::requestView() {
    if (!m_client || !m_client->ready() || !window() || width() < 1 || height() < 1) return;
    const QRectF document(0, 0, m_client->documentWidth(), m_client->documentHeight());
    const QRectF visible(documentPoint({0, 0}), QSizeF(width() / m_zoom, height() / m_zoom));
    const QRectF region = m_interactive ? visible.intersected(document) : document;
    const qreal dpr = window()->devicePixelRatio();
    const QSize pixels(qCeil(region.width() * m_zoom * dpr), qCeil(region.height() * m_zoom * dpr));
    m_client->requestViewport(m_interactive ? 0 : 1, region, pixels, !region.isEmpty());
}
QRectF CanvasItem::documentRect() const {
    if (!m_client) return {};
    const QSizeF size(m_client->documentWidth() * m_zoom, m_client->documentHeight() * m_zoom);
    return {QPointF((width() - size.width()) / 2, (height() - size.height()) / 2) + m_pan, size};
}
QPointF CanvasItem::documentPoint(QPointF local) const { return (local - documentRect().topLeft()) / m_zoom; }
QRectF CanvasItem::visibleDocumentRect() const {
    if(!m_client || width()<=0 || height()<=0 || m_zoom<=0)return {};
    return QRectF(documentPoint({0,0}),QSizeF(width()/m_zoom,height()/m_zoom)).intersected(QRectF(0,0,m_client->documentWidth(),m_client->documentHeight()));
}
void CanvasItem::fitToView() {
    if (!m_client || width() < 1 || height() < 1) return;
    const qreal margin = m_interactive ? 64 : 8;
    m_zoom = std::clamp(std::min((width() - margin) / m_client->documentWidth(), (height() - margin) / m_client->documentHeight()), .00001, 8.);
    if(m_fitPending) m_zoom*=m_initialFitRatio;
    m_pan = {}; m_fitPending = false; emit viewChanged(); update();
}
void CanvasItem::setInitialFitRatio(qreal ratio) {
    if(std::isfinite(ratio)) m_initialFitRatio=std::clamp(ratio,qreal(.1),qreal(1));
}
void CanvasItem::actualSize() { m_zoom = 1; m_pan = {}; emit viewChanged(); update(); }
void CanvasItem::zoomBy(qreal factor) {zoomAround(factor,{width()/2,height()/2});}
void CanvasItem::zoomAround(qreal factor,QPointF position) {
    if(!m_interactive || !m_client || !std::isfinite(factor) || factor<=0 || width()<=0 || height()<=0)return;
    const auto doc=documentPoint(position);
    const auto next=std::clamp(m_zoom*factor,.00001,8.);
    if(next==m_zoom)return;
    m_zoom=next;m_pan+=position-(documentRect().topLeft()+doc*m_zoom);
    emit viewChanged();update();
}
void CanvasItem::geometryChange(const QRectF &geometry, const QRectF &old) {
    QQuickItem::geometryChange(geometry, old);
    if (!m_interactive || m_fitPending) fitToView();
    else if(m_layoutCaptured) m_pan+=QPointF((old.width()-geometry.width())/2,(old.height()-geometry.height())/2);
    emit viewChanged(); update();
}
void CanvasItem::captureLayoutPosition() {
    if(m_layoutCaptured || !m_interactive || m_fitPending || !m_client || !window())return;
    m_layoutCaptured=true;m_layoutOrigin=mapToScene(documentRect().topLeft());m_layoutWindow=window();m_layoutGeneration=m_client->generation();
}
void CanvasItem::restoreLayoutPosition() {
    if(!m_layoutCaptured)return;
    m_layoutCaptured=false;
    if(window()!=m_layoutWindow || !m_client || m_client->generation()!=m_layoutGeneration || m_fitPending)return;
    m_pan+=m_layoutOrigin-mapToScene(documentRect().topLeft());emit viewChanged();update();
}
QSGNode *CanvasItem::updatePaintNode(QSGNode *old, UpdatePaintNodeData *) {
    auto *node = static_cast<CanvasNode *>(old);
    if (m_image.isNull() || !window()) { delete node; return nullptr; }
    if (!node) { node = new CanvasNode; node->setOwnsTexture(true); node->setFiltering(QSGTexture::Linear); }
    // The scene graph owns GPU resources; no graphics objects cross the GUI/worker boundary.
    if (node->imageKey != m_image.cacheKey()) {
        QSGTexture *texture = window()->createTextureFromImage(m_image);
        if (!texture) { delete node; return nullptr; }
        // setTexture deletes the previous texture when ownsTexture is true.
        node->setTexture(texture); node->imageKey = m_image.cacheKey();
    }
    // A viewport frame covers only its document-space region, never the whole canvas.
    node->setRect(QRectF(documentRect().topLeft() + m_imageRegion.topLeft() * m_zoom,
                                 m_imageRegion.size() * m_zoom));
    return node;
}
InputSample CanvasItem::mouseSample(QPointF local, Qt::MouseButtons buttons) {
    InputSample s; s.position = documentPoint(local); s.buttons = static_cast<quint32>(buttons);
    s.capabilities = 16; s.timestamp = static_cast<quint64>(m_clock.nsecsElapsed()); return s;
}
bool CanvasItem::beginLayerMove(QPointF local, bool pick) {
    if(!m_client || !m_client->ready() || m_client->drawing() || m_client->layerEditBusy()) return false;
    m_moveStart=documentPoint(local); m_moveLayer=m_client->activeLayer(); m_moveGeneration=m_client->generation();
    m_pickMoving=pick;
    if(pick && !m_client->pickLayer(m_moveStart))return false;
    m_moving=true; return true;
}
void CanvasItem::finishLayerMove(QPointF local) {
    if(!m_moving) return;
    m_moving=false;
    if(!m_client || m_client->generation()!=m_moveGeneration) return;
    const auto delta=documentPoint(local)-m_moveStart;
    if(std::isfinite(delta.x()) && std::isfinite(delta.y()) && std::abs(delta.x())<=1000000 && std::abs(delta.y())<=1000000) {
        if(m_pickMoving) {
            if(std::round(delta.x())!=0 || std::round(delta.y())!=0)
                m_client->pickLayer(m_moveStart,static_cast<int>(std::round(delta.x())),static_cast<int>(std::round(delta.y())));
        } else m_client->moveLayer(m_moveLayer,static_cast<int>(std::round(delta.x())),static_cast<int>(std::round(delta.y())));
    }
}
bool CanvasItem::beginSelection(QPointF local,Qt::KeyboardModifiers modifiers) {
    if(!m_client || !m_client->ready() || m_client->drawing() || m_client->layerEditBusy() || m_client->fileBusy()) return false;
    if(m_selecting && m_selectionKind==3) {
        const auto point=documentPoint(local);
        if(m_selectionPoints.size()>=3 && QLineF(point,m_selectionPoints.first()).length()*m_zoom<=6)finishPolygon();
        else {appendSelectionPoint(point);m_selectionEnd=point;emit selectionDragChanged();}
        return true;
    }
    m_selectionStart=m_selectionEnd=documentPoint(local);m_selectionGeneration=m_client->generation();
    m_selectionPoints.clear();
    m_selectionKind=m_client->selectionTool()-1;
    // Shift adds to the selection; the square/circle constraint comes from Shift during the drag,
    // matching the usual "hold Shift while dragging" gesture but never blocking additive selection.
    const bool add=modifiers.testFlag(Qt::ShiftModifier),subtract=modifiers.testFlag(Qt::AltModifier);
    m_selectionOperation=add ? (subtract ? 3 : 1) : (subtract ? 2 : m_client->selectionOperation());
    m_selectionConstrained=false;
    if(m_selectionKind>=2)appendSelectionPoint(m_selectionStart);
    m_selecting=true;emit selectionDragChanged();return true;
}
void CanvasItem::updateSelection(QPointF local,Qt::KeyboardModifiers modifiers) {
    if(!m_selecting || !m_client) return;
    const auto point=documentPoint(local);
    m_selectionEnd={std::clamp(point.x(),qreal(0),qreal(m_client->documentWidth())),std::clamp(point.y(),qreal(0),qreal(m_client->documentHeight()))};
    if(m_selectionKind==2)appendSelectionPoint(m_selectionEnd);
    const bool constrained=m_selectionKind<2 && modifiers.testFlag(Qt::ShiftModifier);
    if(constrained!=m_selectionConstrained){m_selectionConstrained=constrained;emit selectionDragChanged();}
    else emit selectionDragChanged();
}
QRectF CanvasItem::constrainedRect(QPointF anchor,QPointF corner) {    // Square/circle: the pressed corner stays put and the box grows along the drag direction until
    // both sides match, so the shape follows the pointer instead of drifting.
    const qreal side=qMax(qAbs(corner.x()-anchor.x()),qAbs(corner.y()-anchor.y()));
    const qreal x=corner.x()>=anchor.x()?anchor.x():anchor.x()-side;
    const qreal y=corner.y()>=anchor.y()?anchor.y():anchor.y()-side;
    return QRectF(x,y,side,side);
}
QRectF CanvasItem::selectionPreviewRect() const {    auto rect=QRectF(m_selectionStart,m_selectionEnd).normalized();
    if(!m_selectionConstrained)return rect;
    return constrainedRect(m_selectionStart,m_selectionEnd);
}

QColor CanvasItem::pickColorAt(QPointF local) const {
    if(!m_client) return {};
    // The ABI maps document coordinates into the current composited viewport frame.
    // Read-only: no history entry, no revision change.
    const auto document = documentPoint(local);
    if(document.x() < 0 || document.y() < 0 || document.x() >= m_client->documentWidth() || document.y() >= m_client->documentHeight()) return {};
    return m_client->sampleDocumentPixel(document.x(), document.y());
}void CanvasItem::finishSelection(QPointF local,Qt::KeyboardModifiers modifiers) {
    if(!m_selecting) return;
    // Keep the shift constraint rather than resetting the modifiers: the release position decides
    // the committed square/circle together with the anchor.
    updateSelection(local,modifiers);
    if(m_selectionKind==3)return;
    if(m_selectionKind==2) {
        const auto points=m_selectionPoints;cancelSelectionDrag();
        if(m_client && m_client->generation()==m_selectionGeneration && points.size()>=3)m_client->editSelectionPath(points,m_selectionOperation,false);
        return;
    }
    const auto rect=selectionPreview();cancelSelectionDrag();
    if(m_client && m_client->generation()==m_selectionGeneration && rect.width()>=1 && rect.height()>=1) m_client->editSelection(rect,m_selectionKind,m_selectionOperation);
}
void CanvasItem::appendSelectionPoint(QPointF point) {
    if(m_selectionPoints.size()>4096)return;
    if(m_selectionPoints.isEmpty() || QLineF(point,m_selectionPoints.last()).length()*m_zoom>=1)m_selectionPoints.append(point);
}
QVariantList CanvasItem::selectionPreviewPoints() const {
    QVariantList points;if(!m_selecting || m_selectionKind<2)return points;
    for(const auto &point:m_selectionPoints)points.append(point);
    if(m_selectionKind==3)points.append(m_selectionEnd);
    return points;
}
void CanvasItem::finishPolygon() {
    if(!m_selecting || m_selectionKind!=3)return;
    const auto points=m_selectionPoints;cancelSelectionDrag();
    if(m_client && m_client->generation()==m_selectionGeneration && points.size()>=3)m_client->editSelectionPath(points,m_selectionOperation,true);
}
void CanvasItem::cancelSelectionDrag(){if(m_selecting){m_selecting=false;m_selectionPoints.clear();emit selectionDragChanged();}}
void CanvasItem::mousePressEvent(QMouseEvent *e) {
    if (!m_interactive || !m_client || e->source() != Qt::MouseEventNotSynthesized) { e->ignore(); return; }
    forceActiveFocus(); m_last = e->position();
    m_cursorInside=contains(e->position());m_cursorPosition=e->position();setCursorSample(mouseSample(e->position(),e->buttons()),e->type()==QEvent::MouseButtonPress);refreshBrushCursor();
    if (e->button() == Qt::MiddleButton || m_space || spaceHeld) { m_panning = true; e->accept(); return; }
    if(m_client->liquifyActive()) {
        if(e->button()==Qt::LeftButton && documentRect().contains(e->position())) {m_stroke=m_client->beginStroke(mouseSample(e->position(),e->buttons()));if(m_stroke)m_liquifyTimer.start();}
        e->accept();return;
    }
    if(e->button()==Qt::LeftButton && e->modifiers().testFlag(Qt::ControlModifier) && !m_client->transformActive() && documentRect().contains(e->position())) {
        e->setAccepted(beginLayerMove(e->position(),true));return;
    }
    // Alt is the eyedropper: sample the rendered frame and adopt the colour as the foreground.
    // Nothing is written to the document, so this never creates history.
    if (e->button() == Qt::LeftButton && (e->modifiers().testFlag(Qt::AltModifier) || m_client->eyedropperTool()) && !m_client->selectionTool() && !m_client->moveTool()) {
        if(e->modifiers().testFlag(Qt::AltModifier))setAltHeld(true);
        beginPicking(e->position());
        e->accept(); return;
    }
    if(e->button()!=Qt::LeftButton){e->ignore();return;}
    if(m_client->selectionTool()==5 && documentRect().contains(e->position())) {const bool add=e->modifiers().testFlag(Qt::ShiftModifier),sub=e->modifiers().testFlag(Qt::AltModifier);e->setAccepted(m_client->selectWand(documentPoint(e->position()),add?(sub?3:1):(sub?2:m_client->wandOperation())));return;}
    if(m_client->bucketTool() && documentRect().contains(e->position())) {e->setAccepted(m_client->bucketFill(documentPoint(e->position())));return;}
    if(m_client->selectionTool() && documentRect().contains(e->position())) {e->setAccepted(beginSelection(e->position(),e->modifiers()));return;}
    if (m_client->moveTool() && documentRect().contains(e->position())) { e->setAccepted(beginLayerMove(e->position())); return; }
    if (documentRect().contains(e->position())) m_stroke = m_client->beginStroke(mouseSample(e->position(), e->buttons()));
    e->setAccepted(m_stroke);
}
void CanvasItem::mouseDoubleClickEvent(QMouseEvent *e) {
    if(m_selecting && m_selectionKind==3 && e->button()==Qt::LeftButton){appendSelectionPoint(documentPoint(e->position()));finishPolygon();e->accept();return;}
    QQuickItem::mouseDoubleClickEvent(e);
}
void CanvasItem::mouseMoveEvent(QMouseEvent *e) {
    if(m_tablet){e->accept();return;}
    m_cursorInside=contains(e->position());m_cursorPosition=e->position();setCursorSample(mouseSample(e->position(),e->buttons()),e->type()==QEvent::MouseButtonPress);refreshBrushCursor();
    if (m_panning) { m_pan += e->position() - m_last; m_last = e->position(); emit viewChanged(); update(); }
    else if(m_picking && !m_tablet) updatePicking(e->position());
    else if(m_selecting && !m_tablet) updateSelection(e->position(),e->modifiers());
    else if (m_stroke && !m_tablet) m_client->strokeTo(mouseSample(e->position(), e->buttons()));
    e->accept();
}
void CanvasItem::mouseReleaseEvent(QMouseEvent *e) {
    if(!m_tablet){m_cursorPosition=e->position();setCursorSample(mouseSample(e->position(),e->buttons()));}
    if(m_picking && !m_tablet && e->button()==Qt::LeftButton){updatePicking(e->position());endPicking();e->accept();return;}
    if(!m_panning && m_selecting && !m_tablet) finishSelection(e->position(),e->modifiers());
    if(m_moving && !m_tablet) finishLayerMove(e->position());
    if (m_stroke && !m_tablet) { m_client->strokeTo(mouseSample(e->position(), e->buttons())); m_client->endStroke(); m_stroke = false; }
    m_panning = false;refreshBrushCursor(); e->accept();
}
void CanvasItem::mouseUngrabEvent() {
    endPicking();
    if (m_stroke && m_client) m_client->cancelStroke();
    m_stroke = false; m_panning = false; m_moving=false;if(m_selectionKind!=3)cancelSelectionDrag();refreshBrushCursor();
}
void CanvasItem::wheelEvent(QWheelEvent *e) {
    if (!m_interactive) { e->ignore(); return; }
    zoomAround(std::pow(1.15,e->angleDelta().y()/120.),e->position());e->accept();
}
void CanvasItem::keyPressEvent(QKeyEvent *e) {
    if (e->key() == Qt::Key_Space) {setKeySpace(true);refreshBrushCursor(); e->accept(); }
    else if(m_selecting && m_selectionKind==3 && (e->key()==Qt::Key_Return || e->key()==Qt::Key_Enter)) {finishPolygon();e->accept();}
    else if(m_selecting && m_selectionKind==3 && e->key()==Qt::Key_Backspace) {if(!m_selectionPoints.isEmpty())m_selectionPoints.removeLast();if(m_selectionPoints.isEmpty())cancelSelectionDrag();else emit selectionDragChanged();e->accept();}
    else if (e->key() == Qt::Key_Alt) {setAltHeld(true); e->accept(); }
    else if(m_client && e->key()==Qt::Key_G && e->modifiers()==Qt::NoModifier) {m_client->activateBucket();e->accept();}
    else if(m_client && e->key()==Qt::Key_W && e->modifiers()==Qt::NoModifier) {m_client->activateWand();e->accept();}
    else if(m_client && e->key()==Qt::Key_R && e->modifiers()==Qt::NoModifier) {m_client->activateSmudge();e->accept();}
    else if (e->key() == Qt::Key_Escape) { if(m_client && m_client->liquifyActive()){m_client->cancelStroke();m_client->finishLiquify(false);}else if(m_selecting) cancelSelectionDrag();else if (m_client) m_client->cancelStroke(); m_stroke = false; m_tablet = false; m_moving=false; e->accept(); }
    else if(m_client && m_client->moveTool() && !m_moving && e->key()>=Qt::Key_Left && e->key()<=Qt::Key_Down) {
        const int step=e->modifiers().testFlag(Qt::ShiftModifier) ? 10 : 1;
        m_client->moveLayer(m_client->activeLayer(),e->key()==Qt::Key_Left ? -step : e->key()==Qt::Key_Right ? step : 0,
            e->key()==Qt::Key_Up ? -step : e->key()==Qt::Key_Down ? step : 0); e->accept();
    }
    else QQuickItem::keyPressEvent(e);
}
void CanvasItem::keyReleaseEvent(QKeyEvent *e) {
    if (e->key() == Qt::Key_Space) {setKeySpace(false);setSpacePanning(false);refreshBrushCursor();}
    else if (e->key() == Qt::Key_Alt) {setAltHeld(false); e->accept();}
    else QQuickItem::keyReleaseEvent(e);
}
void CanvasItem::setAltHeld(bool held) {
    if(m_altHeld==held) return;
    m_altHeld=held;
    if(!held && m_client && !m_client->eyedropperTool())endPicking();
    // The eyedropper takes over the pointer: a crosshair instead of the brush outline.
    refreshBrushCursor();
}
bool CanvasItem::eventFilter(QObject *watched, QEvent *event) {
    if(event->type()==QEvent::ApplicationDeactivate){endPicking();setKeySpace(false);setSpacePanning(false);setAltHeld(false);m_ctrlHeld=false;refreshBrushCursor();}
    if(m_interactive && isVisible() && (event->type()==QEvent::KeyPress || event->type()==QEvent::KeyRelease)) {
        auto *key=static_cast<QKeyEvent*>(event);
        if(key->key()==Qt::Key_Control && !key->isAutoRepeat()){m_ctrlHeld=event->type()==QEvent::KeyPress;refreshBrushCursor();}
        if(key->key()==Qt::Key_Alt && !key->isAutoRepeat())setAltHeld(event->type()==QEvent::KeyPress);
        if(key->key()==Qt::Key_Space && !key->isAutoRepeat()) {
            auto *focused=qobject_cast<QQuickWindow*>(QGuiApplication::focusWindow());
            auto *item=focused?focused->activeFocusItem():nullptr;
            if(event->type()==QEvent::KeyPress && item && item->flags().testFlag(QQuickItem::ItemAcceptsInputMethod))return false;
            setKeySpace(event->type()==QEvent::KeyPress);setSpacePanning(event->type()==QEvent::KeyPress);refreshBrushCursor();key->accept();return true;
        }
    }
    if(event->type()==QEvent::TabletLeaveProximity){m_havePoseSample=false;m_poseDirectionKnown=false;endPicking();m_cursorInside=false;refreshBrushCursor();return false;}
    const bool tabletEvent=event->type()==QEvent::TabletPress || event->type()==QEvent::TabletMove || event->type()==QEvent::TabletRelease;
    if(tabletEvent) {
        const auto item=qobject_cast<QQuickItem*>(watched);
        const auto receiver=item?item->window():qobject_cast<QQuickWindow*>(watched);
        if(receiver!=m_observedWindow)return false;
    } else if (watched != m_observedWindow) return false;
    if (event->type() == QEvent::ScreenChangeInternal || event->type() == QEvent::Resize) m_viewTimer.start();
#if QT_VERSION >= QT_VERSION_CHECK(6, 6, 0)
    if (event->type() == QEvent::DevicePixelRatioChange) m_viewTimer.start();
#endif
    if (!m_interactive || !m_client) return false;
    if(event->type()==QEvent::KeyPress && static_cast<QKeyEvent*>(event)->key()==Qt::Key_CapsLock && !static_cast<QKeyEvent*>(event)->isAutoRepeat()) {m_capsLock=!m_capsLock;refreshBrushCursor();}
#ifdef Q_OS_WIN
    if(event->type()==QEvent::WindowActivate && QGuiApplication::platformName()=="windows"){m_capsLock=(GetKeyState(VK_CAPITAL)&1)!=0;refreshBrushCursor();}
#endif
    if (event->type() == QEvent::WindowDeactivate) {
        if (m_stroke) m_client->cancelStroke();
        m_stroke = m_tablet = m_panning = m_moving = false;
        endPicking();setAltHeld(false);m_ctrlHeld=false;
        setKeySpace(false);setSpacePanning(false);
        m_cursorInside=false;m_havePoseSample=false;m_poseDirectionKnown=false;refreshBrushCursor();
        cancelSelectionDrag();
    }
    if (event->type() != QEvent::TabletPress && event->type() != QEvent::TabletMove && event->type() != QEvent::TabletRelease) return false;
    if (!isEnabled() || !isVisible()) return false;
    // Let Qt Quick synthesize the same pointer gestures used by transform handles.
    if (m_client->transformActive()) { event->ignore(); return false; }
    auto *e = static_cast<QTabletEvent *>(event);
    // Qt Quick may deliver tablet input to an item, whose position is item-local.
    // The global position is shared by both native-window and item delivery paths.
    const QPointF local = mapFromGlobal(e->globalPosition());
    const InputSample s=tabletSample(e,local);
    m_cursorInside=contains(local);m_cursorPosition=local;setCursorSample(s,event->type()==QEvent::TabletPress);refreshBrushCursor();
    if(event->type()!=QEvent::TabletRelease)setAltHeld(e->modifiers().testFlag(Qt::AltModifier));
    if(!m_tablet && event->type()==QEvent::TabletMove) {
        if(!m_cursorInside)return false;
        if(m_selecting && m_selectionKind==3)updateSelection(local,e->modifiers());
        e->accept();return true; // avoid synthesized mouse hover replacing pen position
    }
    if (!m_tablet && (event->type() != QEvent::TabletPress || !documentRect().contains(local))) return false;
    if(m_panning || spacePanning()) {
        if(event->type()==QEvent::TabletPress){m_last=local;m_panning=m_tablet=true;}
        else if(event->type()==QEvent::TabletMove){m_pan+=local-m_last;m_last=local;emit viewChanged();update();}
        else {m_tablet=m_panning=false;refreshBrushCursor();}
        e->accept();return true;
    }
    if(!m_client->liquifyActive() && ((m_pickMoving && m_moving) || (event->type()==QEvent::TabletPress && e->modifiers().testFlag(Qt::ControlModifier)))) {
        if(event->type()==QEvent::TabletPress){forceActiveFocus();m_tablet=beginLayerMove(local,true);}
        else if(event->type()==QEvent::TabletRelease){finishLayerMove(local);m_tablet=false;}
        e->accept();return true;
    }
    if(!m_client->liquifyActive() && (m_picking || ((m_altHeld || m_client->eyedropperTool()) && !m_client->selectionTool() && !m_client->moveTool()))) {
        if(event->type()==QEvent::TabletPress){beginPicking(local);m_tablet=true;}
        else if(event->type()==QEvent::TabletMove)updatePicking(local);
        else if(event->type()==QEvent::TabletRelease){updatePicking(local);endPicking();}
        e->accept();return true;
    }
    if(!m_client->liquifyActive() && (m_client->selectionTool()==5)) {
        if(event->type()==QEvent::TabletPress){const bool add=e->modifiers().testFlag(Qt::ShiftModifier),sub=e->modifiers().testFlag(Qt::AltModifier);forceActiveFocus();m_client->selectWand(documentPoint(local),add?(sub?3:1):(sub?2:m_client->wandOperation()));m_tablet=true;}
        else if(event->type()==QEvent::TabletRelease)m_tablet=false;
        e->accept();return true;
    }
    if(!m_client->liquifyActive() && (m_client->selectionTool() || m_selecting)) {
        if(event->type()==QEvent::TabletPress) {forceActiveFocus();m_tablet=beginSelection(local,e->modifiers());}
        else if(event->type()==QEvent::TabletMove) updateSelection(local,e->modifiers());
        else if(event->type()==QEvent::TabletRelease) {finishSelection(local,e->modifiers());m_tablet=false;}
        e->accept();return true;
    }
    if(!m_client->liquifyActive() && (m_client->bucketTool())) {
        if(event->type()==QEvent::TabletPress){forceActiveFocus();m_client->bucketFill(documentPoint(local));m_tablet=true;}
        else if(event->type()==QEvent::TabletRelease)m_tablet=false;
        e->accept();return true;
    }
    if(!m_client->liquifyActive() && (m_client->moveTool() || m_moving)) {
        if(event->type()==QEvent::TabletPress) m_tablet=beginLayerMove(local);
        else if(event->type()==QEvent::TabletRelease) { finishLayerMove(local); m_tablet=false; }
        e->accept(); return true;
    }
    if (event->type() == QEvent::TabletPress) {m_tablet = m_stroke = m_client->beginStroke(s);if(m_stroke && m_client->liquifyActive())m_liquifyTimer.start();}
    else if (event->type() == QEvent::TabletMove && m_stroke) m_client->strokeTo(s);
    else if (event->type() == QEvent::TabletRelease) { if (m_stroke) m_client->endStroke(); m_tablet = m_stroke = false; }
    refreshBrushCursor();e->accept(); return true;
}
