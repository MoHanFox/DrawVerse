#include "ColorWheelItem.h"
#include <QPainter>
#include <QMouseEvent>
#include <QConicalGradient>
#include <array>
#include <algorithm>
#include <cmath>

namespace {
constexpr int resolution=256;
const QPointF a{.80,0}, b{-.40,-.692820323}, c{-.40,.692820323};
std::array<qreal,3> barycentric(QPointF p) {
    const qreal v=(.8-p.x())/1.2;
    const qreal w=(p.y()+.692820323*v)/1.385640646;
    return {1-v,v-w,w};
}
QPointF nearestTriangle(QPointF p) {
    const auto uv=barycentric(p);
    if(*std::min_element(uv.begin(),uv.end())>=0) return p;
    QPointF best; qreal distance=1e9;
    const std::array<QPointF,3> vertices{a,b,c};
    for(int i=0;i<3;++i) {
        const auto start=vertices[i], edge=vertices[(i+1)%3]-start;
        const qreal t=std::clamp(QPointF::dotProduct(p-start,edge)/QPointF::dotProduct(edge,edge),qreal(0),qreal(1));
        const auto candidate=start+edge*t;
        const qreal d=QPointF::dotProduct(p-candidate,p-candidate);
        if(d<distance){distance=d;best=candidate;}
    }
    return best;
}
}
ColorWheelItem::ColorWheelItem(QQuickItem *parent):QQuickPaintedItem(parent) {
    setAcceptedMouseButtons(Qt::LeftButton); setAntialiasing(true);
}
void ColorWheelItem::setColor(const QColor &color) {
    if(!color.isValid() || color==m_color) return;
    m_color=color;
    if(color.hsvHueF()>=0) m_hue=color.hsvHueF();
    update(); emit colorChanged();
}
void ColorWheelItem::rebuildTriangle() {
    if(m_cachedHue==m_hue) return;
    m_cachedHue=m_hue;
    m_triangle=QImage(resolution,resolution,QImage::Format_ARGB32_Premultiplied);
    m_triangle.fill(Qt::transparent);
    const QColor hue=QColor::fromHsvF(m_hue,1,1);
    for(int y=0;y<resolution;++y) {
        auto *row=reinterpret_cast<QRgb*>(m_triangle.scanLine(y));
        for(int x=0;x<resolution;++x) {
            const auto uv=barycentric({(x+.5-128)/126,(y+.5-128)/126});
            if(*std::min_element(uv.begin(),uv.end())<0) continue;
            row[x]=qRgb(qRound(hue.red()*uv[0]+255*uv[1]),qRound(hue.green()*uv[0]+255*uv[1]),qRound(hue.blue()*uv[0]+255*uv[1]));
        }
    }
}
void ColorWheelItem::pickHsv(qreal hue,qreal saturation,qreal value) {
    if(!std::isfinite(hue) || !std::isfinite(saturation) || !std::isfinite(value)) return;
    const qreal previous=m_hue;
    m_hue=std::clamp(hue,qreal(0),qreal(1));
    const auto color=QColor::fromHsvF(m_hue,std::clamp(saturation,qreal(0),qreal(1)),std::clamp(value,qreal(0),qreal(1)));
    if(color==m_color && previous!=m_hue) { update(); emit colorChanged(); }
    else setColor(color);
    emit picked(color);
}
void ColorWheelItem::paint(QPainter *painter) {
    rebuildTriangle();
    const qreal size=std::min(width(),height()), radius=(size-6)/2;
    const QPointF center{width()/2,height()/2};
    painter->setRenderHint(QPainter::Antialiasing);
    QConicalGradient gradient(center,0);
    for(int i=0;i<=6;++i) gradient.setColorAt(i/6.,QColor::fromHsvF(i==6?0:i/6.,1,1));
    QPen pen(QBrush(gradient),radius*.15); painter->setPen(pen); painter->setBrush(Qt::NoBrush);
    painter->drawEllipse(center,radius*.925,radius*.925);
    const QRectF square(center.x()-radius,center.y()-radius,2*radius,2*radius);
    painter->setRenderHint(QPainter::SmoothPixmapTransform); painter->drawImage(square,m_triangle);
    const qreal s=m_color.hsvSaturationF(), v=m_color.valueF();
    const QPointF triangle=a*(s*v)+b*((1-s)*v)+c*(1-v);
    const qreal angle=-m_hue*2*3.141592653589793;
    const QPointF ring{std::cos(angle)*.925,std::sin(angle)*.925};
    for(const auto &point:std::array<QPointF,2>{ring,triangle}) {
        const QPointF at=center+point*radius;
        painter->setPen(QPen(QColor("#151515"),3)); painter->drawEllipse(at,4.5,4.5);
        painter->setPen(QPen(Qt::white,1.6)); painter->drawEllipse(at,4.5,4.5);
    }
}
void ColorWheelItem::pick(QPointF point) {
    const qreal radius=(std::min(width(),height())-6)/2;
    if(radius<=0) return;
    const QPointF p=(point-QPointF(width()/2,height()/2))/radius;
    QColor color;
    if(m_ring) {
        m_hue=std::fmod(-std::atan2(p.y(),p.x())/(2*3.141592653589793)+1,1);
        color=QColor::fromHsvF(m_hue,m_color.hsvSaturationF(),m_color.valueF());
    } else {
        const auto uv=barycentric(nearestTriangle(p));
        const qreal value=std::clamp(uv[0]+uv[1],qreal(0),qreal(1));
        color=QColor::fromHsvF(m_hue,value>0?std::clamp(uv[0]/value,qreal(0),qreal(1)):0,value);
    }
    setColor(color); update(); emit picked(color);
}
void ColorWheelItem::mousePressEvent(QMouseEvent *event) {
    const QPointF p=event->position()-QPointF(width()/2,height()/2);
    m_ring=std::hypot(p.x(),p.y())>(std::min(width(),height())-6)*.425;
    pick(event->position()); event->accept();
}
void ColorWheelItem::mouseMoveEvent(QMouseEvent *event) { pick(event->position()); event->accept(); }
