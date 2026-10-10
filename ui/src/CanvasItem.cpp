#include "CanvasItem.h"
#include <QQuickWindow>
#include <QSGSimpleTextureNode>
#include <QSGTransformNode>
#include <QMatrix4x4>
#include <QTabletEvent>
#include <QMouseEvent>
#include <QWheelEvent>
#include <QKeyEvent>
#include <QHoverEvent>
#include <QGuiApplication>
#include <QPointingDevice>
#include <QScreen>
#include <algorithm>
#include <cmath>
#ifdef Q_OS_WIN
#include <QtCore/qt_windows.h>
#endif

namespace {
// A transform node wraps the texture node so the view rotation is a scene-graph matrix: the document
// pixels are never copied or resampled on the CPU to rotate the view.
class CanvasNode final : public QSGTransformNode {
public:
    CanvasNode() { appendChildNode(&texture); }
    QSGSimpleTextureNode texture;
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
    m_viewTimer.setSingleShot(true); m_viewTimer.setInterval(16);
    connect(&m_viewTimer, &QTimer::timeout, this, &CanvasItem::requestView);
    connect(this, &CanvasItem::viewChanged, this, [this] { if (!m_viewTimer.isActive()) m_viewTimer.start(); });
    connect(this,&CanvasItem::viewChanged,this,&CanvasItem::refreshBrushCursor);
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
    if (m_client) disconnect(m_client, nullptr, this, nullptr);
    m_client = client; m_fitPending = true;
    if (client) {
        connect(client, &PaintCoreClient::frameChanged, this, &CanvasItem::refresh);
        connect(client, &PaintCoreClient::brushChanged, this, [this] { cancelSelectionDrag();refreshBrushCursor(); });
        connect(client, &PaintCoreClient::stateChanged, this, [this] { refresh(); emit viewChanged(); });
        connect(client, &QObject::destroyed, this, [this] { m_client = nullptr; });
        refresh();
    }
    refreshBrushCursor();emit clientChanged();
}
void CanvasItem::setInteractive(bool enabled) {
    m_interactive = enabled; setAcceptedMouseButtons(enabled ? Qt::LeftButton | Qt::MiddleButton : Qt::NoButton);
    m_fitPending = true; refresh();refreshBrushCursor(); emit interactiveChanged(); emit viewChanged();
}
QRectF CanvasItem::brushCursorRect() const {
    const qreal radius=m_client?m_client->brushRadius()*m_zoom:0;
    return {m_cursorPosition-QPointF(radius,radius),QSizeF(radius*2,radius*2)};
}
bool CanvasItem::brushCursorVisible() const {
    return m_cursorInside && !m_capsLock && m_interactive && m_client && !m_client->moveTool() && !m_client->selectionTool() && !m_space && !spaceHeld && !m_panning && isVisible();
}
bool CanvasItem::spacePanning() const { return m_space || spaceHeld; }
void CanvasItem::refreshBrushCursor() {
    setCursor(m_space || spaceHeld || m_panning?Qt::OpenHandCursor:m_client && m_client->moveTool()?Qt::SizeAllCursor:brushCursorVisible()?Qt::BlankCursor:Qt::CrossCursor);
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
void CanvasItem::hoverEnterEvent(QHoverEvent *event){m_cursorInside=true;m_cursorPosition=event->position();refreshBrushCursor();}
void CanvasItem::hoverMoveEvent(QHoverEvent *event){m_cursorInside=true;m_cursorPosition=event->position();refreshBrushCursor();}
void CanvasItem::hoverLeaveEvent(QHoverEvent *event){Q_UNUSED(event);m_cursorInside=false;refreshBrushCursor();}
void CanvasItem::refresh() {
    if (!m_client) return;
    if (m_generation != m_client->generation()) { cancelSelectionDrag(); m_generation = m_client->generation(); m_fitPending = true; }
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
QPointF CanvasItem::documentPoint(QPointF local) const {
    // Every input path goes through here, so the view rotation has to be undone first: otherwise the
    // brush would paint where the rotated picture is not.
    if (m_viewRotation != 0.) {
        const auto centre = localCentre();
        const auto offset = local - centre;
        const auto radians = qDegreesToRadians(-m_viewRotation);
        const auto cos = std::cos(radians), sin = std::sin(radians);
        local = centre + QPointF(offset.x() * cos - offset.y() * sin, offset.x() * sin + offset.y() * cos);
    }
    return (local - documentRect().topLeft()) / m_zoom;
}
QPointF CanvasItem::scenePoint(QPointF document) const {
    auto local = documentRect().topLeft() + document * m_zoom;
    if (m_viewRotation != 0.) {
        const auto centre = localCentre();
        const auto offset = local - centre;
        const auto radians = qDegreesToRadians(m_viewRotation);
        const auto cos = std::cos(radians), sin = std::sin(radians);
        local = centre + QPointF(offset.x() * cos - offset.y() * sin, offset.x() * sin + offset.y() * cos);
    }
    return local;
}
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
    if (!node) { node = new CanvasNode; node->texture.setOwnsTexture(true); node->texture.setFiltering(QSGTexture::Linear); }
    // The scene graph owns GPU resources; no graphics objects cross the GUI/worker boundary.
    if (node->imageKey != m_image.cacheKey()) {
        QSGTexture *texture = window()->createTextureFromImage(m_image);
        if (!texture) { delete node; return nullptr; }
        // setTexture deletes the previous texture when ownsTexture is true.
        node->texture.setTexture(texture); node->imageKey = m_image.cacheKey();
    }
    // A viewport frame covers only its document-space region, never the whole canvas.
    node->texture.setRect(QRectF(documentRect().topLeft() + m_imageRegion.topLeft() * m_zoom,
                                 m_imageRegion.size() * m_zoom));
    // Rotate the whole canvas around the viewport centre. documentPoint() undoes exactly this, so
    // painting still lands under the pointer at any angle.
    if (m_viewRotation != 0.) {
        const auto centre = localCentre();
        QMatrix4x4 matrix;
        matrix.translate(centre.x(), centre.y());
        matrix.rotate(m_viewRotation, 0., 0., 1.);
        matrix.translate(-centre.x(), -centre.y());
        node->setMatrix(matrix);
    } else {
        node->setMatrix(QMatrix4x4());
    }
    return node;
}
InputSample CanvasItem::mouseSample(QPointF local, Qt::MouseButtons buttons) {
    InputSample s; s.position = documentPoint(local); s.buttons = static_cast<quint32>(buttons);
    s.capabilities = 16; s.timestamp = static_cast<quint64>(m_clock.nsecsElapsed()); return s;
}
void CanvasItem::setViewRotation(qreal degrees) {
    if (!std::isfinite(degrees)) return;
    // Keep the angle in a friendly range so the status readout does not drift to 359.99.
    auto normalised = std::fmod(degrees, 360.);
    if (normalised > 180.) normalised -= 360.;
    if (normalised < -180.) normalised += 360.;
    if (qFuzzyCompare(normalised + 1., m_viewRotation + 1.)) return;
    m_viewRotation = normalised;
    update();
    emit viewChanged();
}
void CanvasItem::rotateViewBy(qreal degrees) { setViewRotation(m_viewRotation + degrees); }
void CanvasItem::resetViewRotation() { setViewRotation(0.); }
bool CanvasItem::beginLayerMove(QPointF local) {
    if(!m_client || !m_client->ready() || m_client->drawing() || m_client->layerEditBusy()) return false;
    m_moveStart=documentPoint(local); m_moveLayer=m_client->activeLayer(); m_moveGeneration=m_client->generation();
    m_moving=true; return true;
}
void CanvasItem::finishLayerMove(QPointF local) {
    if(!m_moving) return;
    m_moving=false;
    if(!m_client || m_client->generation()!=m_moveGeneration) return;
    const auto delta=documentPoint(local)-m_moveStart;
    if(std::isfinite(delta.x()) && std::isfinite(delta.y()) && std::abs(delta.x())<=1000000 && std::abs(delta.y())<=1000000)
        m_client->moveLayer(m_moveLayer,static_cast<int>(std::round(delta.x())),static_cast<int>(std::round(delta.y())));
}
bool CanvasItem::beginSelection(QPointF local,Qt::KeyboardModifiers modifiers) {
    if(!m_client || !m_client->ready() || m_client->drawing() || m_client->layerEditBusy() || m_client->fileBusy()) return false;
    m_selectionStart=m_selectionEnd=documentPoint(local);m_selectionGeneration=m_client->generation();
    m_selectionKind=m_client->selectionTool()-1;
    // Shift adds to the selection; the square/circle constraint comes from Shift during the drag,
    // matching the usual "hold Shift while dragging" gesture but never blocking additive selection.
    const bool add=modifiers.testFlag(Qt::ShiftModifier),subtract=modifiers.testFlag(Qt::AltModifier);
    m_selectionOperation=add ? (subtract ? 3 : 1) : (subtract ? 2 : 0);
    m_selectionConstrained=false;
    m_selecting=true;emit selectionDragChanged();return true;
}
void CanvasItem::updateSelection(QPointF local,Qt::KeyboardModifiers modifiers) {
    if(!m_selecting || !m_client) return;
    const auto point=documentPoint(local);
    m_selectionEnd={std::clamp(point.x(),qreal(0),qreal(m_client->documentWidth())),std::clamp(point.y(),qreal(0),qreal(m_client->documentHeight()))};
    const bool constrained=modifiers.testFlag(Qt::ShiftModifier);
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
QVariantList CanvasItem::pathPoints() const {
    QVariantList rows;for(const auto &point:m_pathPoints){rows.append(QVariantList{point.x(),point.y()});}return rows;
}
QColor CanvasItem::pickColorAt(QPointF local) const {
    if(!m_client) return {};
    // Sampling goes through the ABI so it reads the composited document, not a possibly downscaled
    // or not-yet-rendered frame. Read-only: no history entry, no revision change.
    const auto document = documentPoint(local);
    if(document.x() < 0 || document.y() < 0 || document.x() >= m_client->documentWidth() || document.y() >= m_client->documentHeight()) return {};
    return m_client->sampleDocumentPixel(document.x(), document.y());
}void CanvasItem::finishSelection(QPointF local,Qt::KeyboardModifiers modifiers) {
    if(!m_selecting) return;
    // Keep the shift constraint rather than resetting the modifiers: the release position decides
    // the committed square/circle together with the anchor.
    updateSelection(local,modifiers);const auto rect=selectionPreview();cancelSelectionDrag();
    if(m_client && m_client->generation()==m_selectionGeneration && rect.width()>=1 && rect.height()>=1) m_client->editSelection(rect,m_selectionKind,m_selectionOperation);
}
void CanvasItem::cancelSelectionDrag(){if(m_selecting){m_selecting=false;emit selectionDragChanged();}}
void CanvasItem::mousePressEvent(QMouseEvent *e) {
    if (!m_interactive || !m_client || e->source() != Qt::MouseEventNotSynthesized) { e->ignore(); return; }
    if(m_client->selectionTool()>2 && e->button()==Qt::LeftButton && documentRect().contains(e->position())) {
        // Lasso collects a freehand path; the magic wand resolves one seed region. Both are
        // submitted to the session worker on release so the UI thread never touches pixels.
        if(!m_client->ready() || m_client->drawing() || m_client->layerEditBusy() || m_client->fileBusy()) {e->ignore();return;}
        forceActiveFocus();
        const bool add=e->modifiers().testFlag(Qt::ShiftModifier),subtract=e->modifiers().testFlag(Qt::AltModifier);
        m_pathOperation=add?(subtract?3:1):(subtract?2:0);
        m_pathPoints.clear();m_pathPoints.append(documentPoint(e->position()));
        m_collectingPath=m_client->selectionTool()==3;
        m_pathKind=m_client->selectionTool();
        emit selectionDragChanged();
        if(!m_collectingPath) {m_pathOperation=m_pathOperation;m_wandPoint=documentPoint(e->position());}
        e->accept();return;
    }
    forceActiveFocus(); m_last = e->position();
    // The view-rotation tool drags the view angle. Alt resets it to zero, which is how the same tool
    // gets back to an upright canvas without touching the document.
    if (m_client->rotateViewTool() && e->button() == Qt::LeftButton) {
        if (e->modifiers().testFlag(Qt::AltModifier)) { resetViewRotation(); e->accept(); return; }
        m_rotateViewing = true;
        const auto centre = localCentre();
        m_rotateStartAngle = std::atan2(e->position().y() - centre.y(), e->position().x() - centre.x());
        m_rotateStartRotation = m_viewRotation;
        e->accept(); return;
    }
    // The paint bucket fills on press: no drag state, and the region is resolved on the worker.
    if (m_client->bucketTool() && e->button() == Qt::LeftButton) {
        if (!documentRect().contains(e->position())) { e->accept(); return; }
        const auto document = documentPoint(e->position());
        m_client->fillRegion(document.x(), document.y(), m_client->bucketTolerance(),
                             m_client->bucketContiguous(), m_client->brushOpacity());
        e->accept(); return;
    }
    // Alt is the eyedropper: sample the rendered frame and adopt the colour as the foreground.
    // Nothing is written to the document, so this never creates history.
    if (e->button() == Qt::LeftButton && e->modifiers().testFlag(Qt::AltModifier) && !m_client->selectionTool()) {
        const auto picked = pickColorAt(e->position());
        if (picked.isValid()) { m_client->setBrushColor(picked); emit viewChanged(); }
        e->accept(); return;
    }
    if (e->button() == Qt::MiddleButton || m_space || spaceHeld) { m_panning = true; e->accept(); return; }
    if(m_client->selectionTool() && documentRect().contains(e->position())) {e->setAccepted(beginSelection(e->position(),e->modifiers()));return;}
    if (m_client->moveTool() && documentRect().contains(e->position())) { e->setAccepted(beginLayerMove(e->position())); return; }
    if (documentRect().contains(e->position())) m_stroke = m_client->beginStroke(mouseSample(e->position(), e->buttons()));
    e->setAccepted(m_stroke);
}
void CanvasItem::mouseMoveEvent(QMouseEvent *e) {
    if (m_rotateViewing) {
        const auto centre = localCentre();
        // Too close to the pivot the angle would jump wildly, so ignore that sliver.
        if (QLineF(centre, e->position()).length() > 1.) {
            const auto angle = std::atan2(e->position().y() - centre.y(), e->position().x() - centre.x());
            setViewRotation(m_rotateStartRotation + qRadiansToDegrees(angle - m_rotateStartAngle));
        }
        e->accept(); return;
    }
    if(m_pathKind>2 && (m_collectingPath || m_pathKind==4) && (e->buttons()&Qt::LeftButton)) {
        const auto point=documentPoint(e->position());
        m_cursorPosition=e->position();m_cursorInside=true;
        if(m_collectingPath) {
            // Sample by screen distance so a slow drag does not burn the point budget.
            const auto &last=m_pathPoints.last();
            if(QLineF(last,point).length()>=1.5 && m_pathPoints.size()<4096) m_pathPoints.append(point);
        }
        m_wandPoint=point;
        emit selectionDragChanged();
        e->accept();return;
    }
    m_cursorInside=contains(e->position());m_cursorPosition=e->position();refreshBrushCursor();
    if (m_panning) { m_pan += e->position() - m_last; m_last = e->position(); emit viewChanged(); update(); }
    else if(m_selecting && !m_tablet) updateSelection(e->position(),e->modifiers());
    else if (m_stroke && !m_tablet) m_client->strokeTo(mouseSample(e->position(), e->buttons()));
    e->accept();
}
void CanvasItem::mouseReleaseEvent(QMouseEvent *e) {
    if (m_rotateViewing) { m_rotateViewing=false; e->accept(); return; }
    if(m_pathKind>2) {
        const bool committed=m_pathKind==3 ? (m_pathPoints.size()>=3 && m_client->editSelectionPath(pathPoints(),m_pathOperation))
                                           : m_client->magicWandSelection(m_wandPoint.x(),m_wandPoint.y(),m_wandTolerance,m_pathOperation);
        m_pathKind=0;m_collectingPath=false;m_pathPoints.clear();
        emit selectionDragChanged();
        e->setAccepted(committed);return;
    }
    if(m_selecting && !m_tablet) finishSelection(e->position(),e->modifiers());
    if(m_moving && !m_tablet) finishLayerMove(e->position());
    if (m_stroke && !m_tablet) { m_client->strokeTo(mouseSample(e->position(), e->buttons())); m_client->endStroke(); m_stroke = false; }
    m_panning = false;refreshBrushCursor(); e->accept();
}
void CanvasItem::mouseUngrabEvent() {
    if (m_stroke && m_client) m_client->cancelStroke();
    m_stroke = false; m_panning = false; m_moving=false;cancelSelectionDrag();refreshBrushCursor();
}
void CanvasItem::wheelEvent(QWheelEvent *e) {
    if (!m_interactive) { e->ignore(); return; }
    zoomAround(std::pow(1.15,e->angleDelta().y()/120.),e->position());e->accept();
}
void CanvasItem::keyPressEvent(QKeyEvent *e) {
    if (e->key() == Qt::Key_Space) {setKeySpace(true);refreshBrushCursor(); e->accept(); }
    else if (e->key() == Qt::Key_Escape) { if(m_selecting) cancelSelectionDrag();else if (m_client) m_client->cancelStroke(); m_stroke = false; m_tablet = false; m_moving=false; e->accept(); }
    else if(m_client && m_client->moveTool() && !m_moving && e->key()>=Qt::Key_Left && e->key()<=Qt::Key_Down) {
        const int step=e->modifiers().testFlag(Qt::ShiftModifier) ? 10 : 1;
        m_client->moveLayer(m_client->activeLayer(),e->key()==Qt::Key_Left ? -step : e->key()==Qt::Key_Right ? step : 0,
            e->key()==Qt::Key_Up ? -step : e->key()==Qt::Key_Down ? step : 0); e->accept();
    }
    else QQuickItem::keyPressEvent(e);
}
void CanvasItem::keyReleaseEvent(QKeyEvent *e) { if (e->key() == Qt::Key_Space) {setKeySpace(false);setSpacePanning(false);refreshBrushCursor();} else QQuickItem::keyReleaseEvent(e); }
bool CanvasItem::eventFilter(QObject *watched, QEvent *event) {
    if(event->type()==QEvent::ApplicationDeactivate){setKeySpace(false);setSpacePanning(false);refreshBrushCursor();}
    if(m_interactive && isVisible() && (event->type()==QEvent::KeyPress || event->type()==QEvent::KeyRelease)) {
        auto *key=static_cast<QKeyEvent*>(event);
        if(key->key()==Qt::Key_Space && !key->isAutoRepeat()) {
            auto *focused=qobject_cast<QQuickWindow*>(QGuiApplication::focusWindow());
            auto *item=focused?focused->activeFocusItem():nullptr;
            if(event->type()==QEvent::KeyPress && item && item->flags().testFlag(QQuickItem::ItemAcceptsInputMethod))return false;
            setKeySpace(event->type()==QEvent::KeyPress);setSpacePanning(event->type()==QEvent::KeyPress);refreshBrushCursor();key->accept();return true;
        }
    }
    if(event->type()==QEvent::TabletLeaveProximity){m_cursorInside=false;refreshBrushCursor();return false;}
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
        setKeySpace(false);setSpacePanning(false);
        m_cursorInside=false;refreshBrushCursor();
        cancelSelectionDrag();
    }
    if (event->type() != QEvent::TabletPress && event->type() != QEvent::TabletMove && event->type() != QEvent::TabletRelease) return false;
    if (!isEnabled() || !isVisible()) return false;
    auto *e = static_cast<QTabletEvent *>(event);
    // Qt Quick may deliver tablet input to an item, whose position is item-local.
    // The global position is shared by both native-window and item delivery paths.
    const QPointF local = mapFromGlobal(e->globalPosition());
    m_cursorInside=contains(local);m_cursorPosition=local;refreshBrushCursor();
    if(!m_tablet && event->type()==QEvent::TabletMove) {
        if(!m_cursorInside)return false;
        e->accept();return true; // avoid synthesized mouse hover replacing pen position
    }
    if (!m_tablet && (event->type() != QEvent::TabletPress || !documentRect().contains(local))) return false;
    if(m_client->selectionTool() || m_selecting) {
        if(event->type()==QEvent::TabletPress) {forceActiveFocus();m_tablet=beginSelection(local,e->modifiers());}
        else if(event->type()==QEvent::TabletMove) updateSelection(local,e->modifiers());
        else if(event->type()==QEvent::TabletRelease) {finishSelection(local,e->modifiers());m_tablet=false;}
        e->accept();return true;
    }
    if(m_client->moveTool() || m_moving) {
        if(event->type()==QEvent::TabletPress) m_tablet=beginLayerMove(local);
        else if(event->type()==QEvent::TabletRelease) { finishLayerMove(local); m_tablet=false; }
        e->accept(); return true;
    }
    InputSample s; s.position = documentPoint(local); s.pressure = static_cast<float>(e->pressure());
    s.tiltX = static_cast<float>(e->xTilt()); s.tiltY = static_cast<float>(e->yTilt());
    s.rotation = static_cast<float>(e->rotation()); s.tangentialPressure = static_cast<float>(e->tangentialPressure());
    s.buttons = static_cast<quint32>(e->buttons()); s.tool = e->pointerType() == QPointingDevice::PointerType::Eraser ? 2 : 1;
    s.timestamp = static_cast<quint64>(m_clock.nsecsElapsed()); s.capabilities = 16;
    const auto capabilities = e->pointingDevice()->capabilities();
    if (capabilities.testFlag(QInputDevice::Capability::Pressure)) s.capabilities |= 1; else s.pressure = 1;
    if (capabilities.testFlag(QInputDevice::Capability::XTilt) || capabilities.testFlag(QInputDevice::Capability::YTilt)) s.capabilities |= 2;
    if (capabilities.testFlag(QInputDevice::Capability::Rotation)) s.capabilities |= 4;
    if (capabilities.testFlag(QInputDevice::Capability::TangentialPressure)) s.capabilities |= 8;
    if (event->type() == QEvent::TabletPress) m_tablet = m_stroke = m_client->beginStroke(s);
    else if (event->type() == QEvent::TabletMove && m_stroke) m_client->strokeTo(s);
    else if (event->type() == QEvent::TabletRelease) { if (m_stroke) m_client->endStroke(); m_tablet = m_stroke = false; }
    e->accept(); return true;
}
