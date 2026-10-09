#include "FrostedSurface.h"
#include <QPainter>
#include <QPainterPath>
#include <algorithm>
#include <array>
#include <cmath>

namespace {
QImage blurPass(const QImage &source,bool horizontal) {
    QImage result(source.size(),QImage::Format_ARGB32_Premultiplied);
    constexpr int radius=3;
    static const auto weights=[] {
        std::array<qreal,2*radius+1> values{};
        qreal total=0;
        for(int offset=-radius;offset<=radius;++offset) {
            const qreal value=std::exp(-offset*offset/(2.*1.6*1.6));
            values[offset+radius]=value; total+=value;
        }
        for(auto &value:values) value/=total;
        return values;
    }();
    const int lines=horizontal?source.height():source.width();
    const int length=horizontal?source.width():source.height();
    for(int line=0;line<lines;++line) {
        const auto pixel=[&](int position) {
            position=std::clamp(position,0,length-1);
            return source.pixel(horizontal?position:line,horizontal?line:position);
        };
        for(int position=0;position<length;++position) {
            std::array<qreal,4> sums{};
            for(int offset=-radius;offset<=radius;++offset) {
                const QRgb color=pixel(position+offset);
                const qreal weight=weights[offset+radius];
                sums[0]+=weight*qRed(color); sums[1]+=weight*qGreen(color);
                sums[2]+=weight*qBlue(color); sums[3]+=weight*qAlpha(color);
            }
            result.setPixel(horizontal?position:line,horizontal?line:position,
                qRgba(qRound(sums[0]),qRound(sums[1]),qRound(sums[2]),qRound(sums[3])));
        }
    }
    return result;
}
}
FrostedSurface::FrostedSurface(QQuickItem *parent):QQuickPaintedItem(parent) {
    setAcceptedMouseButtons(Qt::NoButton); setAntialiasing(true);
    setImplicitWidth(200); setImplicitHeight(40);
}
void FrostedSurface::setRadius(qreal value) {
    if(!std::isfinite(value)) return;
    value=std::clamp(value,qreal(0),qreal(64));
    if(m_radius==value) return;
    m_radius=value; update(); emit materialChanged();
}
void FrostedSurface::setTint(QColor value) {
    if(!value.isValid() || value==m_tint) return;
    m_tint=value; update(); emit materialChanged();
}
void FrostedSurface::setTopCornersOnly(bool value) {
    if(m_topCornersOnly==value) return;
    m_topCornersOnly=value; update(); emit materialChanged();
}
QImage FrostedSurface::blurredBackdrop(const QImage &image) {
    if(image.isNull()) return {};
    QImage reduced=image;
    if(image.width()>512 || image.height()>512)
        reduced=image.scaled(512,512,Qt::KeepAspectRatio,Qt::SmoothTransformation);
    reduced=reduced.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    reduced.setDevicePixelRatio(1);
    return blurPass(blurPass(reduced,true),false);
}
void FrostedSurface::capture(QQuickWindow *source) {
    clear();
    if(!source || !source->isVisible()) return;
    m_backdrop=blurredBackdrop(source->grabWindow());
    m_source=source; m_sourceSize=source->size();
    relocate(); emit backdropChanged();
}
void FrostedSurface::relocate() {
    if(m_source) m_origin=m_source->mapFromGlobal(mapToGlobal(QPointF()));
    update();
}
void FrostedSurface::clear() {
    const bool existed=hasBackdrop();
    m_backdrop={}; m_source=nullptr; m_sourceSize={};
    update(); if(existed) emit backdropChanged();
}
void FrostedSurface::paint(QPainter *painter) {
    if(width()<=0 || height()<=0) return;
    const QRectF rect(0,0,width(),height());
    QPainterPath shape; shape.addRoundedRect(rect,m_radius,m_radius);
    if(m_topCornersOnly) {
        shape.setFillRule(Qt::WindingFill);
        shape.addRect(0,height()/2,width(),height()/2);
    }
    painter->setRenderHint(QPainter::Antialiasing);
    painter->save(); painter->setClipPath(shape);
    if(hasBackdrop() && !m_sourceSize.isEmpty()) {
        const qreal sx=qreal(m_backdrop.width())/m_sourceSize.width();
        const qreal sy=qreal(m_backdrop.height())/m_sourceSize.height();
        painter->setRenderHint(QPainter::SmoothPixmapTransform);
        painter->drawImage(rect,m_backdrop,QRectF(m_origin.x()*sx,m_origin.y()*sy,width()*sx,height()*sy));
    }
    painter->fillRect(rect,m_tint);
    painter->restore();
}
