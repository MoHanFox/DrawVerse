#include "canvas/CanvasItem.h"
#include "canvas/LiquifyPreviewNode.h"
#include "canvas/ResidentPreviewTextures.h"
#include "platform/TabletInput.h"
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
class CanvasNode final : public QSGNode {
public:
    ~CanvasNode() override { delete image;for(auto *tile:overview)delete tile;delete overviewTexture; }
    std::array<QSGSimpleTextureNode*,4> overview{};
    QSGTexture *overviewTexture=nullptr;
    qint64 overviewKey=0;
    qint64 imageKey = 0;
    QSGSimpleTextureNode *image=nullptr;
    LiquifyPreviewNode *preview=nullptr;
    LiquifyPreviewNode *moveWarmup=nullptr;
    std::array<std::shared_ptr<ResidentPreviewTextures>,2> sources{};
    std::shared_ptr<ResidentPreviewTextures> preparedSource;
};
// Application-wide space-pan hold: the canvas that owns keyboard focus raises it, every canvas observes it.
bool spaceHeld=false;
QPointer<CanvasItem> tabletCursorOwner;
}

CanvasItem::CanvasItem(QQuickItem *parent) : QQuickItem(parent) {
    setFlag(ItemHasContents); setAcceptedMouseButtons(Qt::LeftButton | Qt::MiddleButton);
    setCursor(Qt::CrossCursor);setAcceptHoverEvents(true);qApp->installEventFilter(this);
#ifdef Q_OS_WIN
    if(QGuiApplication::platformName()=="windows")m_capsLock=(GetKeyState(VK_CAPITAL)&1)!=0;
#endif
    setClip(true); m_clock.start();
    m_navigationClock.start();m_navigationTimer.setInterval(8);
    connect(&m_navigationTimer,&QTimer::timeout,this,&CanvasItem::animateNavigation);
    m_tabletCursorRefresh.setSingleShot(true);m_tabletCursorRefresh.setInterval(0);
    connect(&m_tabletCursorRefresh,&QTimer::timeout,this,[this]{
        if(tabletCursorOwner==this && m_observedWindow && m_cursorInside && isVisible() && isEnabled() && m_interactive)
            m_observedWindow->setCursor(cursor());
    });
    m_poseRetry.setInterval(16);connect(&m_poseRetry,&QTimer::timeout,this,&CanvasItem::refreshBrushCursor);
    m_liquifyTimer.setInterval(16);
    connect(&m_liquifyTimer,&QTimer::timeout,this,[this]{if(!m_stroke || !m_client || !m_client->liquifyActive() || !isEnabled() || !isVisible()){m_liquifyTimer.stop();return;}m_client->liquifyTick();});
    m_viewTimer.setSingleShot(true); m_viewTimer.setInterval(16);
    connect(&m_viewTimer, &QTimer::timeout, this, &CanvasItem::requestView);
    connect(this, &CanvasItem::viewChanged, this, [this] { if (!m_viewTimer.isActive()) m_viewTimer.start(); });
    connect(this,&CanvasItem::viewChanged,this,[this]{if(m_havePoseSample && (!m_client || !m_client->drawing()))m_poseSample.position=documentPoint(m_cursorPosition);refreshBrushCursor();});
    const auto observeWindow=[this](QQuickWindow *window) {
        cancelLayerMove();stopNavigation();
        setTabletWindowCursor(false);
        if (m_observedWindow) { m_observedWindow->removeEventFilter(this); disconnect(m_observedWindow, nullptr, this, nullptr); }
        m_observedWindow = window;
        if (window) {
            connect(window, &QQuickWindow::screenChanged, this, &CanvasItem::observeScreen);
            connect(window,&QQuickWindow::sceneGraphInitialized,this,[this]{emit gpuDisplayChanged();refresh();},Qt::QueuedConnection);
            connect(window,&QQuickWindow::frameSwapped,this,[this]{
                if(m_moveWarmupPending && m_movePreparation && m_movePreparation->uploads->load()){
                    m_moveWarmupPending=false;update();
                }
            });
        }
        emit gpuDisplayChanged();
        observeScreen(window ? window->screen() : nullptr);
        m_viewTimer.start();
    };
    connect(this, &QQuickItem::windowChanged, this, observeWindow);
    connect(this,&QQuickItem::enabledChanged,this,[this] {if(!isEnabled()){cancelLayerMove();stopNavigation();endPicking();setTabletWindowCursor(false);}});
    connect(this,&QQuickItem::visibleChanged,this,[this] {if(!isVisible()){cancelLayerMove();stopNavigation();endPicking();setTabletWindowCursor(false);}});
    observeWindow(window());
}
void CanvasItem::observeScreen(QScreen *screen) {
    disconnect(m_dpiConnection);
    if (screen) m_dpiConnection = connect(screen, &QScreen::logicalDotsPerInchChanged, this, [this] { m_viewTimer.start(); });
    m_viewTimer.start();
}
CanvasItem::~CanvasItem() {
    cancelLayerMove();
    setTabletWindowCursor(false);
    qApp->removeEventFilter(this);
    if (m_observedWindow) m_observedWindow->removeEventFilter(this);
    if (m_client && !m_interactive && !m_previewPart) m_client->requestViewport(1, {}, {}, false);
}
void CanvasItem::setClient(PaintCoreClient *client) {
    if (m_client == client) return;
    cancelLayerMove();
    stopNavigation();m_image={};m_overview={};m_residentPreview={};m_movePreparation={};m_moveWarmupPending=false;
    emit residentPresentationChanged();
    setTabletWindowCursor(false);
    endPicking();
    if (m_client) disconnect(m_client, nullptr, this, nullptr);
    m_poseRetry.stop();m_havePoseSample=false;m_poseDirectionKnown=false;m_client = client; m_fitPending = true;
    if (client) {
        connect(client, &PaintCoreClient::liquifyOptionsChanged,this,&CanvasItem::refreshBrushCursor);
        connect(client, &PaintCoreClient::stateChanged,this,&CanvasItem::refreshBrushCursor);
        connect(client, &PaintCoreClient::brushCursorImageChanged,this,&CanvasItem::refreshBrushCursor);
        connect(client, &PaintCoreClient::brushCursorPreferencesChanged,this,&CanvasItem::refreshBrushCursor);
        connect(client, &PaintCoreClient::frameChanged, this, &CanvasItem::refresh);
        connect(client,&PaintCoreClient::movePreparationChanged,this,[this]{
            m_movePreparation=m_client?m_client->movePreparation():ResidentPreviewAssetsPtr{};
            m_moveWarmupPending=m_movePreparation && m_movePreparation->uploads->load()==0;
            update();
        });
        connect(client, &PaintCoreClient::brushChanged, this, [this] { cancelSelectionDrag();if(m_picking && !m_altHeld && !m_client->eyedropperTool())endPicking();refreshBrushCursor(); });
        connect(client, &PaintCoreClient::stateChanged, this, [this] { refresh(); emit viewChanged(); });
        connect(client, &PaintCoreClient::samplingRingEnabledChanged, this, &CanvasItem::refreshBrushCursor);
        connect(client, &QObject::destroyed, this, [this] { m_client = nullptr;endPicking();refreshBrushCursor(); });
        refresh();
    }
    refreshBrushCursor();emit clientChanged();
}
void CanvasItem::setInteractive(bool enabled) {
    if(!enabled){cancelLayerMove();endPicking();setTabletWindowCursor(false);}
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
    return brushCursorVisible() && m_client && !m_client->eraser() && !m_client->smudgeTool() && !m_client->liquifyActive() && (m_client->brushLibrary()->cursorOutline() || m_client->brushLibrary()->cursorImage()) && m_client->brushLibrary()->cursorComposite() && m_pose.ready && m_pose.secondary;
}
bool CanvasItem::brushCursorVisible() const {
    if(m_client && !m_client->liquifyActive()){
        const auto *library=m_client->brushLibrary();
        if((!library->cursorOutline() && !library->cursorCenter() && !(library->cursorImage() && !library->cursorSimple() && !m_client->cursorTipImage().isEmpty())) || (library->cursorHoverOnly() && m_client->drawing()))return false;
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
    if(tabletCursorOwner==this && m_observedWindow)m_observedWindow->setCursor(cursor());
    emit brushCursorChanged();
}
void CanvasItem::setTabletWindowCursor(bool active) {
    if(active && m_observedWindow && m_interactive && isEnabled() && isVisible() && m_cursorInside) {
        if(tabletCursorOwner!=this) {
            if(tabletCursorOwner)tabletCursorOwner->setTabletWindowCursor(false);
            m_savedWindowCursor=m_observedWindow->cursor();tabletCursorOwner=this;
        }
        // Item cursors are normally resolved by Qt Quick mouse delivery; tablet-only
        // entry may never run that path. Scope the native cursor to this canvas.
        m_observedWindow->setCursor(cursor());
    } else if(tabletCursorOwner==this) {
        m_tabletCursorRefresh.stop();
        if(m_observedWindow)m_observedWindow->setCursor(m_savedWindowCursor);
        tabletCursorOwner.clear();
    }
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
void CanvasItem::updateTabletHover(QHoverEvent *event) {
    // Some tablet backends deliver only Quick hover events between strokes.
    // Keep device geometry from TabletMove, but always use the current position.
    m_penOwnsHover=true;
    syncShortcutModifiers(event->modifiers(),true);
    InputSample sample=m_havePoseSample && (m_poseSample.tool==1 || m_poseSample.tool==2)?m_poseSample:InputSample{};
    sample.position=documentPoint(event->position());sample.pressure=0;
    sample.tool=event->pointingDevice()->pointerType()==QPointingDevice::PointerType::Eraser?2:1;
    sample.capabilities|=16;sample.buttons=0;sample.timestamp=static_cast<quint64>(m_clock.nsecsElapsed());
    m_cursorInside=contains(event->position());m_cursorPosition=event->position();
    // Save the window cursor before setCursor() can make Quick resolve its
    // cached mouse target and replace the value we need to restore on handoff.
    setTabletWindowCursor(m_cursorInside);setCursorSample(sample);refreshBrushCursor();
}
bool CanvasItem::ignoreCachedMouseHover(QHoverEvent *event) {
    // Qt Quick frame-sync hover has the primary mouse device and cached mouse
    // position even while Windows Ink is delivering current TabletMove events.
    // A real mouse event releases this ownership in eventFilter before Quick
    // constructs its hover; an unaccompanied hover must not take it from the pen.
    if(!m_penOwnsHover || tabletHoverEvent(event))return false;
    // Quick resolves the window cursor after hover delivery. Reassert ours once
    // that dispatch has returned, without generating another dirty frame.
    if(tabletCursorOwner==this && !m_tabletCursorRefresh.isActive())m_tabletCursorRefresh.start();
    return true;
}
void CanvasItem::hoverEnterEvent(QHoverEvent *event){
    if(ignoreCachedMouseHover(event) || m_tablet)return;
    if(tabletHoverEvent(event)){updateTabletHover(event);return;}
    syncShortcutModifiers(event->modifiers(),true);
    setTabletWindowCursor(false);m_cursorInside=true;m_cursorPosition=event->position();setCursorSample(mouseSample(event->position(),Qt::NoButton));refreshBrushCursor();
}
void CanvasItem::hoverMoveEvent(QHoverEvent *event){
    if(ignoreCachedMouseHover(event) || m_tablet)return;
    if(tabletHoverEvent(event)){updateTabletHover(event);return;}
    syncShortcutModifiers(event->modifiers(),true);
    setTabletWindowCursor(false);m_cursorInside=true;m_cursorPosition=event->position();setCursorSample(mouseSample(event->position(),Qt::NoButton));refreshBrushCursor();if(m_selecting && m_selectionKind==3)updateSelection(event->position(),event->modifiers());
}
void CanvasItem::hoverLeaveEvent(QHoverEvent *event){if(ignoreCachedMouseHover(event) || m_tablet)return;setTabletWindowCursor(false);m_cursorInside=false;refreshBrushCursor();}
void CanvasItem::refresh() {
    if (!m_client) return;
    if(m_previewPart){
        m_residentPreview=m_client->residentPreview();
        const auto image=m_residentPreview?m_residentPreview->images[size_t(m_previewPart)]:QImage();
        m_image=image;update();
        return;
    }
    if(m_interactive && window())m_client->setResidentGpuPreviewAvailable(gpuDisplay());
    if (m_generation != m_client->generation()) { stopNavigation();m_overview={};endPicking();cancelSelectionDrag(); m_generation = m_client->generation(); m_fitPending = true; }
    const int view = m_interactive ? 0 : 1;
    m_image = m_client->frame(view); m_imageRegion = m_client->frameRegion(view);
    m_movePreparation=m_client->movePreparation();
    m_moveWarmupPending=m_movePreparation && m_movePreparation->uploads->load()==0;
    const QRectF document(0,0,m_client->documentWidth(),m_client->documentHeight());
    if(m_interactive && m_client->frameRegion(1)==document)m_overview=m_client->frame(1);
    if(m_interactive && m_overview.isNull() && !m_viewTimer.isActive())m_viewTimer.start();
    const bool wasForward=forwardPreview();
    m_residentPreview=m_client->residentPreview();
    if(wasForward!=forwardPreview())emit residentPresentationChanged();
    if (m_fitPending) fitToView();
    update();
}
void CanvasItem::requestView() {
    if(m_previewPart)return;
    if (!m_client || !m_client->ready() || !window() || width() < 1 || height() < 1) return;
    if(m_interactive)m_client->setResidentGpuPreviewAvailable(gpuDisplay());
    const QRectF document(0, 0, m_client->documentWidth(), m_client->documentHeight());
    const QRectF region = m_interactive ? prefetchedDocumentRect() : document;
    const qreal dpr = window()->devicePixelRatio();
    const QSize pixels(qCeil(region.width() * m_zoom * dpr), qCeil(region.height() * m_zoom * dpr));
    m_client->requestViewport(m_interactive ? 0 : 1, region, pixels, !region.isEmpty());
    if(m_interactive)m_client->ensureOverview();
}
bool CanvasItem::gpuDisplay() const {
    const auto api=window()?window()->rendererInterface()->graphicsApi():QSGRendererInterface::Unknown;
    return api!=QSGRendererInterface::Software && api!=QSGRendererInterface::Unknown && api!=QSGRendererInterface::Null;
}
QRectF CanvasItem::prefetchedDocumentRect() const {
    if(!m_client || width()<=0 || height()<=0)return {};
    const qreal margin=128/m_zoom;
    const auto ahead=m_velocity*.12/m_zoom;
    QRectF region(documentPoint({0,0}),QSizeF(width()/m_zoom,height()/m_zoom));
    region.adjust(-margin-std::max(0.,ahead.x()),-margin-std::max(0.,ahead.y()),
                  margin-std::min(0.,ahead.x()),margin-std::min(0.,ahead.y()));
    return region.intersected(QRectF(0,0,m_client->documentWidth(),m_client->documentHeight()));
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
    stopNavigation();
    const qreal margin = m_interactive ? 64 : 8;
    m_zoom = std::clamp(std::min((width() - margin) / m_client->documentWidth(), (height() - margin) / m_client->documentHeight()), .00001, 8.);
    if(m_fitPending) m_zoom*=m_initialFitRatio;
    m_pan = {}; m_fitPending = false; emit viewChanged(); update();
}
void CanvasItem::setInitialFitRatio(qreal ratio) {
    if(std::isfinite(ratio)) m_initialFitRatio=std::clamp(ratio,qreal(.1),qreal(1));
}
void CanvasItem::actualSize() { stopNavigation();m_zoom = 1; m_pan = {}; emit viewChanged(); update(); }
void CanvasItem::zoomBy(qreal factor) {zoomAround(factor,{width()/2,height()/2});}
void CanvasItem::zoomAround(qreal factor,QPointF position) {
    if(!m_interactive || !m_client || !std::isfinite(factor) || factor<=0 || width()<=0 || height()<=0)return;
    const auto target=std::clamp((m_zoomAnimating?m_zoomTarget:m_zoom)*factor,.00001,8.);
    m_velocity={};m_zoomFrom=m_zoom;m_zoomTarget=target;m_zoomAnchor=position;m_zoomDocument=documentPoint(position);
    m_zoomStarted=m_navigationClock.elapsed();m_navigationLast=m_zoomStarted;m_zoomAnimating=true;m_navigationTimer.start();
}
void CanvasItem::stopNavigation() {
    m_navigationTimer.stop();m_zoomAnimating=false;m_velocity={};
}
void CanvasItem::animateNavigation() {
    if(!isVisible() || !isEnabled() || !m_client){stopNavigation();return;}
    const auto now=m_navigationClock.elapsed();
    const auto dt=std::clamp((now-m_navigationLast)/1000.,0.,.032);m_navigationLast=now;
    if(m_zoomAnimating){
        const auto t=std::clamp((now-m_zoomStarted)/140.,0.,1.);
        const auto blend=1-std::pow(1-t,3);
        m_zoom=std::exp(std::log(m_zoomFrom)+(std::log(m_zoomTarget)-std::log(m_zoomFrom))*blend);
        if(t>=1){m_zoom=m_zoomTarget;m_zoomAnimating=false;}
        m_pan+=m_zoomAnchor-(documentRect().topLeft()+m_zoomDocument*m_zoom);
    }else{
        const auto damping=std::exp(-8.*dt);
        m_pan+=m_velocity*((1-damping)/8.);m_velocity*=damping;
        if(QLineF({},m_velocity).length()<8 || now-m_inertiaStarted>1000)m_velocity={};
    }
    if(!m_zoomAnimating && m_velocity.isNull())m_navigationTimer.stop();
    emit viewChanged();update();
}
void CanvasItem::beginPan(QPointF position) {
    stopNavigation();m_last=position;m_panning=true;m_panClock.restart();refreshBrushCursor();
}
void CanvasItem::movePan(QPointF position) {
    const auto delta=position-m_last;
    const auto dt=std::max(.004,m_panClock.restart()/1000.);
    const auto speed=delta/dt;const auto length=QLineF({},speed).length();
    m_velocity=length>3000?speed*(3000/length):speed;
    m_pan+=delta;m_last=position;emit viewChanged();update();
}
void CanvasItem::endPan() {
    m_panning=false;
    if(m_panClock.isValid() && m_panClock.elapsed()<80 && QLineF({},m_velocity).length()>40){
        m_inertiaStarted=m_navigationLast=m_navigationClock.elapsed();m_navigationTimer.start();
    }else m_velocity={};
    refreshBrushCursor();
}
void CanvasItem::geometryChange(const QRectF &geometry, const QRectF &old) {
    QQuickItem::geometryChange(geometry, old);
    if (!m_interactive || m_fitPending) fitToView();
    else if(m_layoutCaptured) m_pan+=QPointF((old.width()-geometry.width())/2,(old.height()-geometry.height())/2);
    emit viewChanged(); update();
}
void CanvasItem::captureLayoutPosition() {
    if(m_layoutCaptured || !m_interactive || m_fitPending || !m_client || !window())return;
    stopNavigation();
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
    if ((!m_residentPreview && !m_movePreparation && m_image.isNull() && m_overview.isNull()) || !window()) { delete node; return nullptr; }
    if(!node)node=new CanvasNode;
    if(m_previewPart){
        if(!m_residentPreview){delete node;return nullptr;}
        const auto previous=node->sources;
        if(!node->sources[0] || node->sources[0]->sourceId!=m_residentPreview->sourceId){
            auto current=ResidentPreviewTextures::acquire(window(),*m_residentPreview);
            auto prepared=m_residentPreview->prepared?ResidentPreviewTextures::acquire(window(),*m_residentPreview->prepared):nullptr;
            if(!current || (m_residentPreview->prepared && !prepared)){delete node;return nullptr;}
            if(node->image && node->image->ownsTexture()){delete node->image;node->image=nullptr;}
            node->sources={std::move(current),std::move(prepared)};
        }
        if(!node->image){node->image=new QSGSimpleTextureNode;node->appendChildNode(node->image);}
        node->image->setTexture(node->sources[0]->images[size_t(m_previewPart)]);node->image->setFiltering(QSGTexture::Linear);
        node->image->setRect(m_presentationRect);return node;
    }
    if(node->sources[0]){delete node->image;node->image=nullptr;node->imageKey=0;node->sources={};}
    const bool warmMove=!m_residentPreview && m_movePreparation && m_previewPart==0;
    if(warmMove){
        if(!node->preparedSource || node->preparedSource->sourceId!=m_movePreparation->sourceId)
            node->preparedSource=ResidentPreviewTextures::acquire(window(),*m_movePreparation);
        if(node->preparedSource && m_movePreparation->uploads->load()==0){
            if(!node->moveWarmup){
                node->moveWarmup=new LiquifyPreviewNode;
                if(node->image)node->insertChildNodeBefore(node->moveWarmup,node->image);
                else node->appendChildNode(node->moveWarmup);
            }
            ResidentPreviewFrame frame;
            static_cast<ResidentPreviewAssets&>(frame)=*m_movePreparation;
            const auto document=documentRect();
            frame.documentSize=QSize(m_client?m_client->documentWidth():int(document.width()),
                m_client?m_client->documentHeight():int(document.height()));
            frame.columns=2;frame.rows=2;
            const float w=float(frame.documentSize.width()),h=float(frame.documentSize.height());
            frame.vertices={{0,0,0,0},{w,0,1,0},{0,h,0,1},{w,h,1,1}};
            if(!node->moveWarmup->sync(window(),frame,document)){delete node;return nullptr;}
        }else if(node->moveWarmup){
            delete node->moveWarmup;node->moveWarmup=nullptr;
        }
    }else{
        if(node->moveWarmup){delete node->moveWarmup;node->moveWarmup=nullptr;}
        if(!m_movePreparation)node->preparedSource.reset();
    }
    if(m_residentPreview){
        for(auto *tile:node->overview)if(tile)tile->setRect({});
        if(node->image){delete node->image;node->image=nullptr;node->imageKey=0;}
        if(!node->preview){node->preview=new LiquifyPreviewNode;node->appendChildNode(node->preview);}
        if(!node->preview->sync(window(),*m_residentPreview,documentRect())){qWarning("Unable to allocate resident preview textures");delete node;return nullptr;}
        return node;
    }
    if(node->preview){delete node->preview;node->preview=nullptr;}
    if(node->overviewKey!=m_overview.cacheKey()){
        for(auto *&tile:node->overview){delete tile;tile=nullptr;}
        delete node->overviewTexture;node->overviewTexture=nullptr;
        if(!m_overview.isNull())node->overviewTexture=window()->createTextureFromImage(m_overview);
        node->overviewKey=m_overview.cacheKey();
    }
    if(node->overviewTexture){
        const QRectF document(0,0,m_client->documentWidth(),m_client->documentHeight());
        const auto cut=m_image.isNull()?QRectF():m_imageRegion.intersected(document);
        std::array<QRectF,4> regions{document,QRectF(),QRectF(),QRectF()};
        if(!cut.isEmpty())regions={QRectF(0,0,document.width(),cut.top()),
            QRectF(0,cut.bottom(),document.width(),document.height()-cut.bottom()),
            QRectF(0,cut.top(),cut.left(),cut.height()),
            QRectF(cut.right(),cut.top(),document.width()-cut.right(),cut.height())};
        for(size_t i=0;i<regions.size();++i){
            auto *&tile=node->overview[i];
            if(!tile){tile=new QSGSimpleTextureNode;tile->setTexture(node->overviewTexture);tile->setFiltering(QSGTexture::Linear);node->prependChildNode(tile);}
            tile->setTexture(node->overviewTexture);
            const auto &r=regions[i];tile->setRect(QRectF(documentRect().topLeft()+r.topLeft()*m_zoom,r.size()*m_zoom));
            tile->setSourceRect(QRectF(r.x()/document.width()*m_overview.width(),r.y()/document.height()*m_overview.height(),r.width()/document.width()*m_overview.width(),r.height()/document.height()*m_overview.height()));
        }
    }else for(auto *tile:node->overview)if(tile)tile->setRect({});
    if(m_image.isNull()){if(node->image){delete node->image;node->image=nullptr;node->imageKey=0;}return node;}
    if(!node->image){
        auto *texture=window()->createTextureFromImage(m_image);if(!texture){delete node;return nullptr;}
        node->image=new QSGSimpleTextureNode;node->image->setTexture(texture);node->imageKey=m_image.cacheKey();
        node->image->setOwnsTexture(true);node->image->setFiltering(QSGTexture::Linear);node->appendChildNode(node->image);
    }
    // The scene graph owns GPU resources; no graphics objects cross the GUI/worker boundary.
    if (node->imageKey != m_image.cacheKey()) {
        QSGTexture *texture = window()->createTextureFromImage(m_image);
        if (!texture) { delete node; return nullptr; }
        // setTexture deletes the previous texture when ownsTexture is true.
        node->image->setTexture(texture); node->imageKey = m_image.cacheKey();
    }
    // A viewport frame covers only its document-space region, never the whole canvas.
    node->image->setRect(QRectF(documentRect().topLeft() + m_imageRegion.topLeft() * m_zoom,
                                 m_imageRegion.size() * m_zoom));
    return node;
}
InputSample CanvasItem::mouseSample(QPointF local, Qt::MouseButtons buttons) {
    InputSample s; s.position = documentPoint(local); s.buttons = static_cast<quint32>(buttons);
    s.capabilities = 16; s.timestamp = static_cast<quint64>(m_clock.nsecsElapsed()); return s;
}
bool CanvasItem::beginLayerMove(QPointF local, bool pick) {
    if(!m_client || !m_client->beginLayerMove(documentPoint(local),pick))return false;
    m_moveStart=documentPoint(local);m_moveGeneration=m_client->generation();
    m_pickMoving=pick;m_moving=true;return true;
}
void CanvasItem::updateLayerMove(QPointF local) {
    if(!m_moving || !m_client || m_client->generation()!=m_moveGeneration)return;
    const auto delta=documentPoint(local)-m_moveStart;
    if(std::isfinite(delta.x()) && std::isfinite(delta.y()) && std::abs(delta.x())<=1000000 && std::abs(delta.y())<=1000000)
        m_client->updateLayerMove(QPoint(qRound(delta.x()),qRound(delta.y())));
}
void CanvasItem::finishLayerMove(QPointF local) {
    if(!m_moving)return;
    updateLayerMove(local);m_moving=false;
    if(!m_client || m_client->generation()!=m_moveGeneration)return;
    const auto delta=documentPoint(local)-m_moveStart;
    const bool valid=std::isfinite(delta.x()) && std::isfinite(delta.y()) && std::abs(delta.x())<=1000000 && std::abs(delta.y())<=1000000;
    m_client->finishLayerMove(valid?QPoint(qRound(delta.x()),qRound(delta.y())):QPoint{},valid);
}
void CanvasItem::cancelLayerMove() {
    if(m_moving && m_client && m_client->generation()==m_moveGeneration)m_client->finishLayerMove({},false);
    m_moving=false;
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
    syncShortcutModifiers(e->modifiers()|QGuiApplication::queryKeyboardModifiers(),false);
    stopNavigation();
    forceActiveFocus(); m_last = e->position();
    m_cursorInside=contains(e->position());m_cursorPosition=e->position();setCursorSample(mouseSample(e->position(),e->buttons()),e->type()==QEvent::MouseButtonPress);refreshBrushCursor();
    if (e->button() == Qt::MiddleButton || m_space || spaceHeld) { beginPan(e->position());e->accept(); return; }
    if(m_client->liquifyActive()) {
        if(e->button()==Qt::LeftButton) {m_stroke=m_client->beginStroke(mouseSample(e->position(),e->buttons()));if(m_stroke)m_liquifyTimer.start();}
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
    if (!m_client->selectionTool() && !m_client->bucketTool() && !m_client->moveTool() && !m_client->transformActive()) {
        if(!m_stroke || !m_client->drawing()) {
            if(m_client->drawing())m_client->endStroke();
            m_stroke=m_client->beginStroke(mouseSample(e->position(),e->buttons()));
        }
    }
    e->setAccepted(m_stroke || m_client->drawing());
}
void CanvasItem::mouseDoubleClickEvent(QMouseEvent *e) {
    if(m_selecting && m_selectionKind==3 && e->button()==Qt::LeftButton){appendSelectionPoint(documentPoint(e->position()));finishPolygon();e->accept();return;}
    QQuickItem::mouseDoubleClickEvent(e);
}
void CanvasItem::mouseMoveEvent(QMouseEvent *e) {
    if(m_tablet){e->accept();return;}
    syncShortcutModifiers(e->modifiers(),true);
    m_cursorInside=contains(e->position());m_cursorPosition=e->position();setCursorSample(mouseSample(e->position(),e->buttons()),e->type()==QEvent::MouseButtonPress);refreshBrushCursor();
    if (m_panning) { movePan(e->position()); }
    else if(m_moving)updateLayerMove(e->position());
    else if(m_picking && !m_tablet) updatePicking(e->position());
    else if(m_selecting && !m_tablet) updateSelection(e->position(),e->modifiers());
    else if ((m_stroke || m_client->drawing()) && !m_tablet){m_stroke=true;m_client->strokeTo(mouseSample(e->position(),e->buttons()));}
    e->accept();
}
void CanvasItem::mouseReleaseEvent(QMouseEvent *e) {
    syncShortcutModifiers(e->modifiers(),false);
    if(!m_tablet){m_cursorPosition=e->position();setCursorSample(mouseSample(e->position(),e->buttons()));}
    if(m_picking && !m_tablet && e->button()==Qt::LeftButton){updatePicking(e->position());endPicking();e->accept();return;}
    if(!m_panning && m_selecting && !m_tablet) finishSelection(e->position(),e->modifiers());
    if(m_moving && !m_tablet) finishLayerMove(e->position());
    if ((m_stroke || m_client->drawing()) && !m_tablet) { m_client->strokeTo(mouseSample(e->position(), e->buttons())); m_client->endStroke(); m_stroke = false; }
    if(m_panning)endPan();refreshBrushCursor(); e->accept();
}
void CanvasItem::mouseUngrabEvent() {
    cancelLayerMove();
    if(m_panning || m_stroke)stopNavigation();
    endPicking();
    if (m_client && (m_stroke || m_client->drawing())) m_client->cancelStroke();
    m_stroke = false; m_tablet = false; m_panning = false; m_moving=false;if(m_selectionKind!=3)cancelSelectionDrag();refreshBrushCursor();
}
void CanvasItem::wheelEvent(QWheelEvent *e) {
    if (!m_interactive) { e->ignore(); return; }
    const auto delta=e->pixelDelta().isNull()?e->angleDelta().y()/120.:e->pixelDelta().y()/100.;
    zoomAround(std::pow(1.15,delta),e->position());e->accept();
}
void CanvasItem::keyPressEvent(QKeyEvent *e) {
    if (e->key() == Qt::Key_Space) {setKeySpace(true);refreshBrushCursor(); e->accept(); }
    else if(m_selecting && m_selectionKind==3 && (e->key()==Qt::Key_Return || e->key()==Qt::Key_Enter)) {finishPolygon();e->accept();}
    else if(m_selecting && m_selectionKind==3 && e->key()==Qt::Key_Backspace) {if(!m_selectionPoints.isEmpty())m_selectionPoints.removeLast();if(m_selectionPoints.isEmpty())cancelSelectionDrag();else emit selectionDragChanged();e->accept();}
    else if (e->key() == Qt::Key_Alt) {setAltHeld(true); e->accept(); }
    else if(m_client && e->key()==Qt::Key_G && e->modifiers()==Qt::NoModifier) {m_client->activateBucket();e->accept();}
    else if(m_client && e->key()==Qt::Key_W && e->modifiers()==Qt::NoModifier) {m_client->activateWand();e->accept();}
    else if(m_client && e->key()==Qt::Key_R && e->modifiers()==Qt::NoModifier) {m_client->activateSmudge();e->accept();}
    else if (e->key() == Qt::Key_Escape) { if(m_moving)cancelLayerMove();else if(m_client && m_client->liquifyActive()){m_client->cancelStroke();m_client->finishLiquify(false);}else if(m_selecting) cancelSelectionDrag();else if (m_client) m_client->cancelStroke(); m_stroke = false; m_tablet = false; m_moving=false; e->accept(); }
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
void CanvasItem::syncShortcutModifiers(Qt::KeyboardModifiers modifiers,bool latchOnly) {
    const bool alt=modifiers.testFlag(Qt::AltModifier),ctrl=modifiers.testFlag(Qt::ControlModifier);
    if(!latchOnly || alt)setAltHeld(alt);
    if(!latchOnly || ctrl)m_ctrlHeld=ctrl;
}
bool CanvasItem::eventFilter(QObject *watched, QEvent *event) {
    if((m_penOwnsHover || tabletCursorOwner==this) && (event->type()==QEvent::MouseMove || event->type()==QEvent::MouseButtonPress)) {
        const auto *mouse=static_cast<QMouseEvent*>(event);
        if(!m_tablet && mouse->source()==Qt::MouseEventNotSynthesized && mouse->pointingDevice()->type()==QInputDevice::DeviceType::Mouse){m_penOwnsHover=false;setTabletWindowCursor(false);}
    }
    if(watched==m_observedWindow && event->type()==QEvent::Leave && !m_tablet) {setTabletWindowCursor(false);m_cursorInside=false;refreshBrushCursor();}

    if(event->type()==QEvent::ApplicationDeactivate){cancelLayerMove();stopNavigation();setTabletWindowCursor(false);endPicking();setKeySpace(false);setSpacePanning(false);syncShortcutModifiers(QGuiApplication::queryKeyboardModifiers(),false);refreshBrushCursor();}
    if(watched==m_observedWindow && m_observedWindow && m_observedWindow->isActive() && m_interactive && isVisible() && isEnabled() && m_client && m_client->ready() && !m_client->drawing() && !m_client->textInputFocused()
        && (event->type()==QEvent::KeyPress || event->type()==QEvent::ShortcutOverride)) {
        auto *key=static_cast<QKeyEvent*>(event);const auto modifiers=key->modifiers();
        const bool zoomKey=key->key()==Qt::Key_Plus || key->key()==Qt::Key_Equal || key->key()==Qt::Key_Minus;
        if(zoomKey && modifiers.testFlag(Qt::ControlModifier) && !(modifiers & (Qt::AltModifier|Qt::MetaModifier))) {
            if(event->type()==QEvent::KeyPress)zoomBy(key->key()==Qt::Key_Minus?1./1.25:1.25);
            key->accept();return true;
        }
    }
    if(m_interactive && isVisible() && (event->type()==QEvent::KeyPress || event->type()==QEvent::KeyRelease)) {
        auto *key=static_cast<QKeyEvent*>(event);
        if((key->key()==Qt::Key_Control || key->key()==Qt::Key_Alt) && !key->isAutoRepeat()) {
            auto modifiers=key->modifiers();
            const bool pressed=event->type()==QEvent::KeyPress;
            if(key->key()==Qt::Key_Control)modifiers.setFlag(Qt::ControlModifier,pressed);
            if(key->key()==Qt::Key_Alt)modifiers.setFlag(Qt::AltModifier,pressed);
            syncShortcutModifiers(modifiers,false);
        }
        refreshBrushCursor();
        if(key->key()==Qt::Key_Space && !key->isAutoRepeat()) {
            auto *focused=qobject_cast<QQuickWindow*>(QGuiApplication::focusWindow());
            auto *item=focused?focused->activeFocusItem():nullptr;
            if(event->type()==QEvent::KeyPress && item && item->flags().testFlag(QQuickItem::ItemAcceptsInputMethod))return false;
            setKeySpace(event->type()==QEvent::KeyPress);setSpacePanning(event->type()==QEvent::KeyPress);refreshBrushCursor();key->accept();return true;
        }
    }
    if(event->type()==QEvent::TabletEnterProximity || event->type()==QEvent::TabletLeaveProximity) {
        auto *tablet=static_cast<QTabletEvent*>(event);
        if(!acceptTabletInputDevice(activeTabletInputBackend(),tablet->pointingDevice())){event->accept();return true;}
    }
    if(event->type()==QEvent::TabletLeaveProximity){
        // Drivers can report proximity loss without a final contact release.
        // Finish the accepted samples once, then release every pen-owned gesture.
        if(m_tablet || (m_client && m_client->drawing())){
            if(m_client && m_client->drawing())m_client->endStroke();
            m_liquifyTimer.stop();m_stroke=false;
            cancelLayerMove();stopNavigation();cancelSelectionDrag();m_tablet=false;
        }
        setTabletWindowCursor(false);m_havePoseSample=false;m_poseDirectionKnown=false;endPicking();m_cursorInside=false;refreshBrushCursor();return false;
    }
    const bool tabletEvent=event->type()==QEvent::TabletPress || event->type()==QEvent::TabletMove || event->type()==QEvent::TabletRelease;
    if(tabletEvent) {
        const auto item=qobject_cast<QQuickItem*>(watched);
        const auto receiver=item?item->window():qobject_cast<QQuickWindow*>(watched);
        if(receiver!=m_observedWindow)return false;
    } else if (watched != m_observedWindow) return false;
    if(tabletEvent) {
        auto *eventTablet=static_cast<QTabletEvent*>(event);
        if(!acceptTabletInputDevice(activeTabletInputBackend(),eventTablet->pointingDevice())){event->accept();return true;}
    }
    if (event->type() == QEvent::ScreenChangeInternal || event->type() == QEvent::Resize) m_viewTimer.start();
#if QT_VERSION >= QT_VERSION_CHECK(6, 6, 0)
    if (event->type() == QEvent::DevicePixelRatioChange) m_viewTimer.start();
#endif
    if (!m_interactive || !m_client) return false;
    if(event->type()==QEvent::KeyPress && static_cast<QKeyEvent*>(event)->key()==Qt::Key_CapsLock && !static_cast<QKeyEvent*>(event)->isAutoRepeat()) {m_capsLock=!m_capsLock;refreshBrushCursor();}
    if(event->type()==QEvent::WindowActivate)syncShortcutModifiers(QGuiApplication::queryKeyboardModifiers(),false);
#ifdef Q_OS_WIN
    if(event->type()==QEvent::WindowActivate && QGuiApplication::platformName()=="windows"){m_capsLock=(GetKeyState(VK_CAPITAL)&1)!=0;refreshBrushCursor();}
#endif
    if (event->type() == QEvent::WindowDeactivate) {
        cancelLayerMove();
        stopNavigation();
        setTabletWindowCursor(false);
        if (m_stroke || m_client->drawing()) m_client->cancelStroke();
        m_stroke = m_tablet = m_panning = m_moving = false;
        endPicking();syncShortcutModifiers(QGuiApplication::queryKeyboardModifiers(),false);
        setKeySpace(false);setSpacePanning(false);
        m_cursorInside=false;m_havePoseSample=false;m_poseDirectionKnown=false;refreshBrushCursor();
        cancelSelectionDrag();
    }
    if (event->type() != QEvent::TabletPress && event->type() != QEvent::TabletMove && event->type() != QEvent::TabletRelease) return false;
    if (!isEnabled() || !isVisible()) return false;
    if(!m_tablet)for(auto *popup:QGuiApplication::allWindows())if(popup->isVisible() && (popup->type()==Qt::Popup || popup->property("drawverseMenuOpen").toBool())) {
        // Windows Ink can retain the owner window as tablet target while a
        // native popup covers its canvas. Let menu routing/Quick see it first.
        m_penOwnsHover=true;
        if(m_cursorInside || tabletCursorOwner==this){setTabletWindowCursor(false);m_cursorInside=false;refreshBrushCursor();}
        return false;
    }
    // Let Qt Quick synthesize the same pointer gestures used by transform handles.
    if (m_client->transformActive()) { event->ignore(); return false; }
    auto *e = static_cast<QTabletEvent *>(event);
    const auto tabletModifiers=e->modifiers()|(event->type()==QEvent::TabletPress?QGuiApplication::queryKeyboardModifiers():Qt::KeyboardModifiers{});
    syncShortcutModifiers(tabletModifiers,event->type()==QEvent::TabletMove);
    m_penOwnsHover=true;
    // Qt Quick may deliver tablet input to an item, whose position is item-local.
    // The global position is shared by both native-window and item delivery paths.
    const QPointF local = mapFromGlobal(e->globalPosition());
    if(event->type()==QEvent::TabletPress)stopNavigation();
    const InputSample s=tabletSample(e,local);
    m_cursorInside=contains(local);m_cursorPosition=local;setTabletWindowCursor(m_cursorInside);setCursorSample(s,event->type()==QEvent::TabletPress);refreshBrushCursor();
    if(!m_tablet && event->type()==QEvent::TabletMove) {
        if(!m_cursorInside)return false;
        if(m_selecting && m_selectionKind==3)updateSelection(local,e->modifiers());
        e->accept();return true; // avoid synthesized mouse hover replacing pen position
    }
    const bool brushInput=!m_client->liquifyActive() && !m_client->selectionTool() && !m_client->bucketTool() && !m_client->moveTool() && !m_client->eyedropperTool() && !spacePanning() && !e->modifiers().testFlag(Qt::ControlModifier) && !e->modifiers().testFlag(Qt::AltModifier);
    if (!m_tablet && (event->type() != QEvent::TabletPress || !contains(local) || (!documentRect().contains(local) && !brushInput && !m_client->liquifyActive()))) return false;
    if(m_panning || spacePanning()) {
        if(event->type()==QEvent::TabletPress){beginPan(local);m_tablet=true;}
        else if(event->type()==QEvent::TabletMove){movePan(local);}
        else {m_tablet=false;endPan();}
        e->accept();return true;
    }
    if(!m_client->liquifyActive() && ((m_pickMoving && m_moving) || (event->type()==QEvent::TabletPress && e->modifiers().testFlag(Qt::ControlModifier)))) {
        if(event->type()==QEvent::TabletPress){forceActiveFocus();m_tablet=beginLayerMove(local,true);}
        else if(event->type()==QEvent::TabletMove)updateLayerMove(local);
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
        else if(event->type()==QEvent::TabletMove)updateLayerMove(local);
        else if(event->type()==QEvent::TabletRelease) { finishLayerMove(local); m_tablet=false; }
        e->accept(); return true;
    }
    if (event->type() == QEvent::TabletPress) {
        forceActiveFocus();
        if(!m_stroke || !m_client->drawing()) {
            if(m_client->drawing())m_client->endStroke();
            m_stroke=m_client->beginStroke(s);
        }
        m_tablet=m_stroke;
        if(m_stroke && m_client->liquifyActive())m_liquifyTimer.start();
    } else if (event->type() == QEvent::TabletMove && (m_stroke || m_client->drawing())) {
        // A duplicate press must not detach the canvas from an accepted stroke.
        m_stroke=true;m_tablet=true;m_client->strokeTo(s);
    } else if (event->type() == QEvent::TabletRelease) {
        if(m_client->drawing())m_client->endStroke();
        m_tablet=m_stroke=false;
    }
    refreshBrushCursor();e->accept(); return true;
}
