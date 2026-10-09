#include "MenuSurface.h"
#include <QPainter>
#include <QPainterPath>
#include <algorithm>
#include <cmath>

MenuSurface::MenuSurface(QQuickItem *parent):QQuickPaintedItem(parent) {
    setAcceptedMouseButtons(Qt::NoButton); setAntialiasing(true);
    setFillColor(Qt::transparent); setOpaquePainting(false);
    setImplicitWidth(200); setImplicitHeight(40);
}
void MenuSurface::setRadius(qreal value) {
    if(!std::isfinite(value)) return;
    value=std::clamp(value,qreal(0),qreal(64));
    if(m_radius==value) return;
    m_radius=value; update(); emit materialChanged();
}
void MenuSurface::setTint(QColor value) {
    if(!value.isValid() || value==m_tint) return;
    m_tint=value; update(); emit materialChanged();
}
void MenuSurface::setTopCornersOnly(bool value) {
    if(m_topCornersOnly==value) return;
    m_topCornersOnly=value; update(); emit materialChanged();
}
void MenuSurface::setTopRightCornerOnly(bool value) {
    if(m_topRightCornerOnly==value) return;
    m_topRightCornerOnly=value; update(); emit materialChanged();
}
void MenuSurface::paint(QPainter *painter) {
    const QRectF rect(0,0,width(),height());
    painter->save();
    painter->setCompositionMode(QPainter::CompositionMode_Source);
    painter->fillRect(rect,Qt::transparent);
    painter->setCompositionMode(QPainter::CompositionMode_SourceOver);
    QPainterPath shape; shape.addRoundedRect(rect,m_radius,m_radius);
    if(m_topCornersOnly || m_topRightCornerOnly) {
        shape.setFillRule(Qt::WindingFill);
        shape.addRect(0,height()/2,width(),height()/2);
        if(m_topRightCornerOnly) shape.addRect(0,0,width()/2,height());
    }
    painter->setRenderHint(QPainter::Antialiasing);
    painter->fillPath(shape,m_tint); painter->restore();
}
