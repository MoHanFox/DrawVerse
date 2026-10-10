#include "SelectionOverlay.h"
#include <QPainter>
#include <QQuickWindow>
#include <cmath>

namespace {
// Dash pattern in device pixels. The white line is stroked in screen space so the pattern stays a
// constant size at any zoom; a document-space pattern stretches with zoom into a near-solid line.
// A short dash against a longer gap keeps the ants clearly dotted instead of reading as a line.
constexpr int kDashOn = 4;
constexpr int kDashGap = 4;
constexpr int kDashPeriod = kDashOn + kDashGap;
constexpr int kPhaseStep = 1;
// One step every 60ms: a calm crawl rather than a busy strobing outline.
constexpr int kTickMs = 80;
}

SelectionOverlay::SelectionOverlay(QQuickItem *parent):QQuickPaintedItem(parent){
    setAcceptedMouseButtons(Qt::NoButton);setAntialiasing(true);setClip(true);
    m_ants.setInterval(kTickMs);m_ants.setTimerType(Qt::CoarseTimer);
    connect(&m_ants,&QTimer::timeout,this,[this]{
        m_phase=(m_phase+kPhaseStep)%kDashPeriod;
        emit dashPhaseChanged();
        update();
    });
}
void SelectionOverlay::componentComplete(){
    QQuickPaintedItem::componentComplete();
    syncAnimation();
}
void SelectionOverlay::setSteps(const QVariantList &steps){if(steps==m_steps)return;m_steps=steps;rebuild();}
void SelectionOverlay::setEnabledSelection(bool enabled){if(m_enabled==enabled)return;m_enabled=enabled;syncAnimation();update();emit geometryChanged();}
void SelectionOverlay::setDocumentRect(QRectF rect){if(rect==m_documentRect)return;const bool sizeChanged=rect.size()!=m_documentRect.size();m_documentRect=rect;if(sizeChanged)rebuild();else{update();emit geometryChanged();}}
void SelectionOverlay::setPreview(QRectF rect){if(rect==m_preview)return;m_preview=rect;syncAnimation();update();emit geometryChanged();}
void SelectionOverlay::setPreviewKind(int kind){if(kind==m_previewKind)return;m_previewKind=kind;update();emit geometryChanged();}
void SelectionOverlay::setZoom(qreal zoom){if(!std::isfinite(zoom)||zoom<=0||zoom==m_zoom)return;m_zoom=zoom;rebuild();}
void SelectionOverlay::syncAnimation(){
    // Marching ants only run while the owning window is actually on screen, so a hidden window,
    // a minimized window or a torn-down test scene never repaints forever.
    const bool hasOutline=m_enabled||!m_preview.isEmpty();
    QQuickWindow *host=window();
    const bool shown=host && host->isVisible() && host->visibility()!=QWindow::Minimized;
    const bool active=hasOutline&&shown;
    if(active && !m_ants.isActive())m_ants.start();
    else if(!active && m_ants.isActive())m_ants.stop();
}
void SelectionOverlay::itemChange(ItemChange change,const ItemChangeData &value){
    QQuickPaintedItem::itemChange(change,value);
    if(change!=ItemSceneChange)return;
    syncAnimation();
    // Track the hosting window so hiding, minimizing or closing it stops the animation instead of
    // repainting a window nobody can see.
    if(m_hostWindow)m_hostWindow->disconnect(this);
    m_hostWindow=window();
    if(m_hostWindow){
        connect(m_hostWindow,&QWindow::visibleChanged,this,[this]{syncAnimation();});
        connect(m_hostWindow,&QWindow::visibilityChanged,this,[this]{syncAnimation();});
    }
}
void SelectionOverlay::rebuild(){
    QPainterPath full;full.addRect(QRectF(QPointF(),m_documentRect.size()/m_zoom));
    m_path={};
    for(const auto &entry:m_steps){
        const auto step=entry.toMap();const int operation=step.value("operation").toInt();
        QPainterPath shape;const QRectF rect(step.value("x").toDouble(),step.value("y").toDouble(),step.value("width").toDouble(),step.value("height").toDouble());
        if(step.value("shape").toInt()==1)shape.addEllipse(rect);else shape.addRect(rect);
        switch(operation){case 0:m_path=shape;break;case 1:m_path=m_path.united(shape);break;case 2:m_path=m_path.subtracted(shape);break;case 3:m_path=m_path.intersected(shape);break;case 4:m_path=full.subtracted(m_path);break;default:break;}
    }
    m_path=m_path.intersected(full);syncAnimation();update();emit geometryChanged();
}
void SelectionOverlay::paint(QPainter *painter){
    if(!m_enabled && m_preview.isEmpty())return;
    painter->setRenderHint(QPainter::Antialiasing);
    painter->setBrush(Qt::NoBrush);
    // Solid black underlay in document space: it hugs the selection exactly.
    painter->save();
    painter->translate(m_documentRect.topLeft());painter->scale(m_zoom,m_zoom);
    QPen black(Qt::black,2);black.setCosmetic(true);
    painter->setPen(black);
    if(m_enabled)painter->drawPath(m_path);
    if(!m_preview.isEmpty())painter->drawPath(previewPath(m_preview,m_previewKind));
    painter->restore();
    // Dashed white ants in screen space, so the dash length and the marching speed do not depend
    // on the document zoom.
    QTransform toScreen;toScreen.translate(m_documentRect.left(),m_documentRect.top());toScreen.scale(m_zoom,m_zoom);
    QPen white(Qt::white,1);white.setCosmetic(true);
    white.setDashPattern({qreal(kDashOn),qreal(kDashGap)});
    white.setDashOffset(m_phase);
    painter->setPen(white);
    if(m_enabled)painter->drawPath(toScreen.map(m_path));
    if(!m_preview.isEmpty())painter->drawPath(toScreen.map(previewPath(m_preview,m_previewKind)));
}
QPainterPath SelectionOverlay::previewPath(const QRectF &rect,int kind){
    QPainterPath path;
    if(kind==1)path.addEllipse(rect);else path.addRect(rect);
    return path;
}
