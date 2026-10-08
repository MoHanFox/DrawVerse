#include "SelectionOverlay.h"
#include <QPainter>
#include <cmath>

SelectionOverlay::SelectionOverlay(QQuickItem *parent):QQuickPaintedItem(parent){setAcceptedMouseButtons(Qt::NoButton);setAntialiasing(true);setClip(true);}
void SelectionOverlay::setSteps(const QVariantList &steps){if(steps==m_steps)return;m_steps=steps;rebuild();}
void SelectionOverlay::setEnabledSelection(bool enabled){if(m_enabled==enabled)return;m_enabled=enabled;update();emit geometryChanged();}
void SelectionOverlay::setDocumentRect(QRectF rect){if(rect==m_documentRect)return;const bool sizeChanged=rect.size()!=m_documentRect.size();m_documentRect=rect;if(sizeChanged)rebuild();else{update();emit geometryChanged();}}
void SelectionOverlay::setPreview(QRectF rect){if(rect==m_preview)return;m_preview=rect;update();emit geometryChanged();}
void SelectionOverlay::setPreviewKind(int kind){if(kind==m_previewKind)return;m_previewKind=kind;update();emit geometryChanged();}
void SelectionOverlay::setZoom(qreal zoom){if(!std::isfinite(zoom)||zoom<=0||zoom==m_zoom)return;m_zoom=zoom;rebuild();}
void SelectionOverlay::rebuild(){
    QPainterPath full;full.addRect(QRectF(QPointF(),m_documentRect.size()/m_zoom));
    m_path={};
    for(const auto &entry:m_steps){
        const auto step=entry.toMap();const int operation=step.value("operation").toInt();
        QPainterPath shape;const QRectF rect(step.value("x").toDouble(),step.value("y").toDouble(),step.value("width").toDouble(),step.value("height").toDouble());
        if(step.value("shape").toInt()==1)shape.addEllipse(rect);else shape.addRect(rect);
        switch(operation){case 0:m_path=shape;break;case 1:m_path=m_path.united(shape);break;case 2:m_path=m_path.subtracted(shape);break;case 3:m_path=m_path.intersected(shape);break;case 4:m_path=full.subtracted(m_path);break;default:break;}
    }
    m_path=m_path.intersected(full);update();emit geometryChanged();
}
void SelectionOverlay::paint(QPainter *painter){
    if(!m_enabled && m_preview.isEmpty())return;
    painter->setRenderHint(QPainter::Antialiasing);painter->translate(m_documentRect.topLeft());painter->scale(m_zoom,m_zoom);
    painter->setBrush(Qt::NoBrush);
    const auto outline=[&](const QPainterPath &path){QPen black(Qt::black,2);black.setCosmetic(true);painter->setPen(black);painter->drawPath(path);QPen white(Qt::white,1);white.setCosmetic(true);white.setDashPattern({4,4});painter->setPen(white);painter->drawPath(path);};
    if(m_enabled)outline(m_path);
    if(!m_preview.isEmpty()){QPainterPath path;if(m_previewKind==1)path.addEllipse(m_preview);else path.addRect(m_preview);outline(path);}
}
