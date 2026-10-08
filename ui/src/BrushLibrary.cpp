#include "BrushLibrary.h"
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QSet>
#include <QUuid>
#include <algorithm>
#include <cmath>
#include <limits>

BrushLibrary::BrushLibrary(QObject *parent):QObject(parent) {
    m_presets={{"round-pressure",QStringLiteral("压感圆笔"),12,1,.15,true,{},1,0,{}},
               {"round-fine",QStringLiteral("细线圆笔"),3,1,.15,true,{},1,0,{}},
               {"round-translucent",QStringLiteral("透明圆笔"),18,.25,.15,true,{},1,0,{}},
               {"round-wide",QStringLiteral("宽幅圆笔"),48,1,.25,true,{},1,0,{}}};
}
int BrushLibrary::index(const QString &id) const {for(int i=0;i<m_presets.size();++i)if(m_presets[i].id==id)return i;return -1;}
QString BrushLibrary::selectedName() const{return m_presets[index(m_selected)].name;}
QString BrushLibrary::selectedPreview() const{return m_presets[index(m_selected)].preview;}
QString BrushLibrary::previewMessage() const{const auto &p=m_presets[index(m_selected)];return p.error.isEmpty()?QStringLiteral("生成预览…"):p.error;}
qreal BrushLibrary::radius() const{return m_presets[index(m_selected)].radius;}
qreal BrushLibrary::opacity() const{return m_presets[index(m_selected)].opacity;}
qreal BrushLibrary::spacing() const{return m_presets[index(m_selected)].spacing;}
QVariantList BrushLibrary::presets() const {
    QVariantList rows;for(const auto &p:m_presets)rows.append(QVariantMap{{"id",p.id},{"name",p.name},{"radius",p.radius},{"opacity",p.opacity},{"spacing",p.spacing},{"builtin",p.builtin},{"preview",p.preview}});return rows;
}
void BrushLibrary::changed(){emit settingsChanged();emit presetsChanged();emit persistRequested(snapshot());}
void BrushLibrary::update(int field,qreal value) {
    if(!std::isfinite(value))return;
    auto &p=m_presets[index(m_selected)];qreal *target=field==0?&p.radius:field==1?&p.opacity:&p.spacing;
    value=field==0?std::clamp(value,.5,256.):field==1?std::clamp(value,0.,1.):std::clamp(value,.05,1.);
    if(*target==value)return;
    *target=value;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();
}
void BrushLibrary::setRadius(qreal value){update(0,value);}
void BrushLibrary::setOpacity(qreal value){update(1,value);}
void BrushLibrary::setSpacing(qreal value){update(2,value);}
bool BrushLibrary::select(const QString &id){if(index(id)<0)return false;if(id!=m_selected){m_selected=id;changed();}return true;}
QString BrushLibrary::saveCopy(const QString &name){if(m_presets.size()>=64 || name.trimmed().isEmpty() || name.size()>80)return {};auto p=m_presets[index(m_selected)];p.id=QUuid::createUuid().toString(QUuid::WithoutBraces);p.name=name.trimmed();p.builtin=false;p.token=++m_nextToken;p.requested=0;p.error.clear();m_presets.append(p);m_selected=p.id;changed();return p.id;}
bool BrushLibrary::rename(const QString &id,const QString &name){const int i=index(id);if(i<0 || name.trimmed().isEmpty() || name.size()>80)return false;m_presets[i].name=name.trimmed();changed();return true;}
bool BrushLibrary::remove(const QString &id){const int i=index(id);if(i<0 || m_presets[i].builtin)return false;m_presets.removeAt(i);if(m_selected==id)m_selected="round-pressure";changed();return true;}
void BrushLibrary::requestPreview(const QString &id){const int i=index(id);if(i<0)return;auto &p=m_presets[i];if(!p.preview.isEmpty() || !p.error.isEmpty() || p.requested==p.token)return;p.requested=p.token;emit previewRequested(p.id,p.token,p.radius,p.opacity,p.spacing);}
void BrushLibrary::retryPreview(){auto &p=m_presets[index(m_selected)];p.preview.clear();p.error.clear();p.requested=0;p.token=++m_nextToken;requestPreview(m_selected);emit presetsChanged();}
void BrushLibrary::acceptPreview(const QString &id,quint64 token,const QString &image){const int i=index(id);if(i<0 || m_presets[i].token!=token)return;m_presets[i].requested=0;m_presets[i].preview=image;m_presets[i].error=image.isEmpty()?QStringLiteral("预览暂不可用"):QString();emit presetsChanged();}
QByteArray BrushLibrary::snapshot() const {
    QJsonArray rows;for(const auto &p:m_presets)rows.append(QJsonObject{{"id",p.id},{"name",p.name},{"radius",p.radius},{"opacity",p.opacity},{"spacing",p.spacing}});
    return QJsonDocument(QJsonObject{{"version",1},{"selected",m_selected},{"presets",rows}}).toJson(QJsonDocument::Compact);
}
bool BrushLibrary::restore(const QByteArray &json) {
    if(json.size()>65536)return false;
    const auto root=QJsonDocument::fromJson(json).object();const auto rows=root.value("presets").toArray();
    if(root.value("version").toInt()!=1 || rows.size()<4 || rows.size()>64)return false;
    const QSet<QString> builtins{"round-pressure","round-fine","round-translucent","round-wide"};QSet<QString> seen;QList<Preset> restored;
    for(const auto &row:rows){const auto p=row.toObject();Preset b;b.id=p.value("id").toString();b.name=p.value("name").toString();b.radius=p.value("radius").toDouble(std::numeric_limits<double>::quiet_NaN());b.opacity=p.value("opacity").toDouble(std::numeric_limits<double>::quiet_NaN());b.spacing=p.value("spacing").toDouble(std::numeric_limits<double>::quiet_NaN());
        if(b.id.isEmpty() || b.id.size()>64 || seen.contains(b.id) || b.name.trimmed().isEmpty() || b.name.size()>80 || !std::isfinite(b.radius) || b.radius<.5 || b.radius>256 || !std::isfinite(b.opacity) || b.opacity<0 || b.opacity>1 || !std::isfinite(b.spacing) || b.spacing<.05 || b.spacing>1)return false;
        b.builtin=builtins.contains(b.id);b.token=++m_nextToken;seen.insert(b.id);restored.append(b);}
    const auto selected=root.value("selected").toString();if(!seen.contains(selected) || !seen.contains(builtins))return false;
    m_presets=restored;m_selected=selected;emit settingsChanged();emit presetsChanged();return true;
}
