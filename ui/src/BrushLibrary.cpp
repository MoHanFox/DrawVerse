#include "BrushLibrary.h"
#include <QColor>
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
    m_recentColors={QStringLiteral("#000000"),QStringLiteral("#3a3a3a"),QStringLiteral("#6e6e6e"),QStringLiteral("#a8a8a8"),
                    QStringLiteral("#d8d8d8"),QStringLiteral("#ffffff"),QStringLiteral("#8c2b2b"),QStringLiteral("#d6453f"),
                    QStringLiteral("#f08a3c"),QStringLiteral("#f5d14e"),QStringLiteral("#7fb84b"),QStringLiteral("#3e9e7a")};
}
QVariantList BrushLibrary::recentColors() const {QVariantList rows;for(const auto &c:m_recentColors)rows.append(c);return rows;}
bool BrushLibrary::useColor(const QString &color) {
    const QColor parsed(color);
    if(!parsed.isValid())return false;
    const auto normalized=parsed.name(QColor::HexArgb);
    // Already the most recent colour: nothing to reorder or persist.
    if(!m_recentColors.isEmpty() && m_recentColors.first()==normalized)return false;
    m_recentColors.removeAll(normalized);
    m_recentColors.prepend(normalized);
    while(m_recentColors.size()>12)m_recentColors.removeLast();
    emit recentColorsChanged();
    emit persistRequested(snapshot());
    return true;
}
int BrushLibrary::index(const QString &id) const {for(int i=0;i<m_presets.size();++i)if(m_presets[i].id==id)return i;return -1;}
QString BrushLibrary::selectedName() const{return m_presets[index(m_selected)].name;}
QString BrushLibrary::selectedPreview() const{return m_presets[index(m_selected)].preview;}
QString BrushLibrary::previewMessage() const{const auto &p=m_presets[index(m_selected)];return p.error.isEmpty()?QStringLiteral("生成预览…"):p.error;}
qreal BrushLibrary::radius() const{return m_toolRadii[m_eraser?1:0];}
qreal BrushLibrary::opacity() const{return m_presets[index(m_selected)].opacity;}
qreal BrushLibrary::spacing() const{return m_presets[index(m_selected)].spacing;}
qreal BrushLibrary::hardness() const{return m_presets[index(m_selected)].hardness;}
qreal BrushLibrary::roundness() const{return m_presets[index(m_selected)].roundness;}
qreal BrushLibrary::angle() const{return m_presets[index(m_selected)].angle;}
qreal BrushLibrary::flow() const{return m_presets[index(m_selected)].flow;}
bool BrushLibrary::pressureSize() const{return m_presets[index(m_selected)].pressureSize;}
bool BrushLibrary::pressureFlow() const{return m_presets[index(m_selected)].pressureFlow;}
QVariantMap BrushLibrary::tipSettings() const{return {{"hardness",hardness()},{"roundness",roundness()},{"angle",angle()},{"flow",flow()},{"pressureSize",pressureSize()},{"pressureFlow",pressureFlow()}};}
QVariantList BrushLibrary::presets() const {
    QVariantList rows;for(const auto &p:m_presets)rows.append(QVariantMap{{"id",p.id},{"name",p.name},{"radius",p.radius},{"opacity",p.opacity},{"spacing",p.spacing},{"builtin",p.builtin},{"preview",p.preview}});return rows;
}
void BrushLibrary::changed(){emit settingsChanged();emit presetsChanged();emit persistRequested(snapshot());}
void BrushLibrary::update(int field,qreal value) {
    if(!std::isfinite(value))return;
    auto &p=m_presets[index(m_selected)];
    qreal *target=nullptr;
    switch(field){case 0:target=&p.radius;value=std::clamp(value,.5,256.);break;
        case 1:target=&p.opacity;value=std::clamp(value,0.,1.);break;
        case 2:target=&p.spacing;value=std::clamp(value,.02,1.);break;
        case 3:target=&p.hardness;value=std::clamp(value,0.,1.);break;
        case 4:target=&p.roundness;value=std::clamp(value,.01,1.);break;
        case 5:target=&p.angle;value=std::clamp(value,-180.,180.);break;
        case 6:target=&p.flow;value=std::clamp(value,0.,1.);break;
        default:return;}
    if(*target==value)return;
    *target=value;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();
}
void BrushLibrary::setRadius(qreal value){
    if(!std::isfinite(value))return;
    value=std::clamp(value,.5,256.);auto &radius=m_toolRadii[m_eraser?1:0];if(radius==value)return;
    radius=value;emit settingsChanged();emit persistRequested(snapshot());
}
void BrushLibrary::setEraser(bool eraser){if(m_eraser!=eraser){m_eraser=eraser;emit settingsChanged();}}
void BrushLibrary::setOpacity(qreal value){update(1,value);}
void BrushLibrary::setSpacing(qreal value){update(2,value);}
void BrushLibrary::setHardness(qreal value){update(3,value);}
void BrushLibrary::setRoundness(qreal value){update(4,value);}
void BrushLibrary::setAngle(qreal value){update(5,value);}
void BrushLibrary::setFlow(qreal value){update(6,value);}
void BrushLibrary::setPressureSize(bool value){auto &p=m_presets[index(m_selected)];if(p.pressureSize==value)return;p.pressureSize=value;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();}
void BrushLibrary::setPressureFlow(bool value){auto &p=m_presets[index(m_selected)];if(p.pressureFlow==value)return;p.pressureFlow=value;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();}
void BrushLibrary::cursorPreferenceChanged(){emit cursorChanged();emit persistRequested(snapshot());}
void BrushLibrary::setCursorOutline(bool value){if(m_cursorOutline==value)return;m_cursorOutline=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorCenter(bool value){if(m_cursorCenter==value)return;m_cursorCenter=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorHoverOnly(bool value){if(m_cursorHoverOnly==value)return;m_cursorHoverOnly=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorWidth(int value){value=std::clamp(value,1,3);if(m_cursorWidth==value)return;m_cursorWidth=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorSimple(bool value){if(m_cursorSimple==value)return;m_cursorSimple=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorColor(const QString &value){const QColor color(value);if(!color.isValid() || color.alpha()!=255 || color.name()==m_cursorColor)return;m_cursorColor=color.name();cursorPreferenceChanged();}
void BrushLibrary::setCursorSecondaryColor(const QString &value){const QColor color(value);if(!color.isValid() || color.alpha()!=255 || color.name()==m_cursorSecondaryColor)return;m_cursorSecondaryColor=color.name();cursorPreferenceChanged();}
bool BrushLibrary::select(const QString &id){if(index(id)<0)return false;if(id!=m_selected){m_selected=id;changed();}return true;}
QString BrushLibrary::saveCopy(const QString &name){if(m_presets.size()>=1024 || name.trimmed().isEmpty() || name.size()>80)return {};auto p=m_presets[index(m_selected)];p.id=QUuid::createUuid().toString(QUuid::WithoutBraces);p.name=name.trimmed();p.builtin=false;p.token=++m_nextToken;p.requested=0;p.error.clear();m_presets.append(p);m_selected=p.id;changed();return p.id;}
bool BrushLibrary::rename(const QString &id,const QString &name){const int i=index(id);if(i<0 || name.trimmed().isEmpty() || name.size()>80)return false;m_presets[i].name=name.trimmed();changed();return true;}
bool BrushLibrary::remove(const QString &id){const int i=index(id);if(i<0 || m_presets[i].builtin)return false;m_presets.removeAt(i);if(m_selected==id)m_selected="round-pressure";changed();return true;}
void BrushLibrary::requestPreview(const QString &id){const int i=index(id);if(i<0)return;auto &p=m_presets[i];if(!p.preview.isEmpty() || !p.error.isEmpty() || p.requested==p.token)return;p.requested=p.token;emit previewRequested(p.id,p.token,24.,p.opacity,p.spacing,{{"hardness",p.hardness},{"roundness",p.roundness},{"angle",p.angle},{"flow",p.flow},{"pressureSize",p.pressureSize},{"pressureFlow",p.pressureFlow}});}
void BrushLibrary::retryPreview(){auto &p=m_presets[index(m_selected)];p.preview.clear();p.error.clear();p.requested=0;p.token=++m_nextToken;requestPreview(m_selected);emit presetsChanged();}
void BrushLibrary::acceptPreview(const QString &id,quint64 token,const QString &image){const int i=index(id);if(i<0 || m_presets[i].token!=token)return;m_presets[i].requested=0;m_presets[i].preview=image;m_presets[i].error=image.isEmpty()?QStringLiteral("预览暂不可用"):QString();emit presetsChanged();}
QByteArray BrushLibrary::snapshot() const {
    QJsonArray rows;for(const auto &p:m_presets)rows.append(QJsonObject{{"id",p.id},{"name",p.name},{"radius",p.radius},{"opacity",p.opacity},{"spacing",p.spacing},{"hardness",p.hardness},{"roundness",p.roundness},{"angle",p.angle},{"flow",p.flow},{"pressureSize",p.pressureSize},{"pressureFlow",p.pressureFlow}});
    return QJsonDocument(QJsonObject{{"version",2},{"selected",m_selected},{"presets",rows},{"cursor",QJsonObject{{"outline",m_cursorOutline},{"center",m_cursorCenter},{"hoverOnly",m_cursorHoverOnly},{"width",m_cursorWidth},{"simple",m_cursorSimple},{"color",m_cursorColor},{"secondaryColor",m_cursorSecondaryColor}}},{"toolRadii",QJsonArray{m_toolRadii[0],m_toolRadii[1]}},
                                     {"recentColors",QJsonArray::fromStringList(m_recentColors)}}).toJson(QJsonDocument::Compact);
}
QByteArray BrushLibrary::exportPackage() const {
    const auto &p=m_presets[index(m_selected)];
    const QStringList builtins={"round-pressure","round-fine","round-translucent","round-wide"};
    const int builtin=builtins.indexOf(p.id);
    const auto id=builtin<0?p.id:QString("dba00000-0000-4000-8000-%1").arg(builtin+1,12,16,QChar('0'));
    const QJsonObject parameters{{"diameter",p.radius*2},{"spacing",p.spacing/2},{"dabOpacity",p.opacity},{"hardness",p.hardness},{"roundness",p.roundness},{"angle",p.angle},{"flow",p.flow},{"pressureSize",p.pressureSize},{"pressureFlow",p.pressureFlow}};
    const QJsonObject preset{{"id",id},{"name",p.name},{"engineProfile","drawverse.brush"},{"behaviorVersion",1},{"requiredCapabilities",QJsonArray{"procedural-tip","pressure","dab-flow"}},{"parameters",parameters}};
    return QJsonDocument(QJsonObject{{"packageId",QUuid::createUuid().toString(QUuid::WithoutBraces)},{"brushes",QJsonArray{preset}}}).toJson(QJsonDocument::Compact);
}
QByteArray BrushLibrary::catalogSnapshot() const {
    const auto root=QJsonDocument::fromJson(snapshot()).object();
    return QJsonDocument(QJsonObject{{"version",2},{"selected",root.value("selected")},{"presets",root.value("presets")}}).toJson(QJsonDocument::Compact);
}
bool BrushLibrary::applyNativeCatalog(const QByteArray &json) {
    const auto native=QJsonDocument::fromJson(json).object();
    if(native.value("version").toInt()!=2 || !native.value("presets").isArray() || !native.value("selected").isString())return false;
    if(QJsonDocument(native).toJson(QJsonDocument::Compact)==catalogSnapshot())return true;
    auto state=QJsonDocument::fromJson(snapshot()).object();state["version"]=2;state["selected"]=native.value("selected");state["presets"]=native.value("presets");
    return restore(QJsonDocument(state).toJson(QJsonDocument::Compact));
}
bool BrushLibrary::importPackage(const QByteArray &json) {
    if(json.size()>2*1024*1024)return false;
    const auto package=QJsonDocument::fromJson(json).object();
    if(!package.value("brushes").isArray())return false;
    const auto brushes=package.value("brushes").toArray();
    if(brushes.isEmpty() || brushes.size()+m_presets.size()>1024)return false;
    auto state=QJsonDocument::fromJson(snapshot()).object();auto rows=state.value("presets").toArray();QSet<QString> ids;
    for(const auto &p:m_presets)ids.insert(p.id);
    QString selected;
    for(const auto &value:brushes){
        const auto b=value.toObject();const auto params=b.value("parameters").toObject();QString id=b.value("id").toString();
        if(QUuid(id).isNull() || b.value("behaviorVersion").toInt()!=1 || (b.value("engineProfile").toString()!="drawverse.brush" && b.value("engineProfile").toString()!="legacy.round"))return false;
        if(ids.contains(id))id=QUuid::createUuid().toString(QUuid::WithoutBraces);
        ids.insert(id);if(selected.isEmpty())selected=id;
        QJsonObject row{{"id",id},{"name",b.value("name")},{"radius",params.value("diameter").toDouble()/2},{"opacity",params.value("dabOpacity")},{"spacing",params.value("spacing").toDouble()*2}};
        for(const auto &key:{"hardness","roundness","angle","flow","pressureSize","pressureFlow"})row[key]=params.value(key);
        rows.append(row);
    }
    state["presets"]=rows;state["selected"]=selected;
    if(!restore(QJsonDocument(state).toJson(QJsonDocument::Compact)))return false;
    emit persistRequested(snapshot());return true;
}
bool BrushLibrary::restore(const QByteArray &json) {
    if(json.size()>2*1024*1024)return false;
    const auto root=QJsonDocument::fromJson(json).object();const auto rows=root.value("presets").toArray();
    const int version=root.value("version").toInt();
    if((version!=1 && version!=2) || rows.size()<4 || rows.size()>1024)return false;
    const QSet<QString> builtins{"round-pressure","round-fine","round-translucent","round-wide"};QSet<QString> seen;QList<Preset> restored;
    for(const auto &row:rows){const auto p=row.toObject();Preset b;b.id=p.value("id").toString();b.name=p.value("name").toString();b.radius=p.value("radius").toDouble(std::numeric_limits<double>::quiet_NaN());b.opacity=p.value("opacity").toDouble(std::numeric_limits<double>::quiet_NaN());b.spacing=p.value("spacing").toDouble(std::numeric_limits<double>::quiet_NaN());
        if(b.id.isEmpty() || b.id.size()>64 || seen.contains(b.id) || b.name.trimmed().isEmpty() || b.name.size()>80 || !std::isfinite(b.radius) || b.radius<.5 || b.radius>256 || !std::isfinite(b.opacity) || b.opacity<0 || b.opacity>1 || !std::isfinite(b.spacing) || b.spacing<.02 || b.spacing>1)return false;
        if(version==2){
            for(const auto &key:{"hardness","roundness","angle","flow"})if(!p.value(key).isDouble())return false;
            if(!p.value("pressureSize").isBool() || !p.value("pressureFlow").isBool())return false;
            b.hardness=p.value("hardness").toDouble();b.roundness=p.value("roundness").toDouble();b.angle=p.value("angle").toDouble();b.flow=p.value("flow").toDouble();
            if(!std::isfinite(b.hardness) || b.hardness<0 || b.hardness>1 || !std::isfinite(b.roundness) || b.roundness<.01 || b.roundness>1 || !std::isfinite(b.angle) || b.angle<-180 || b.angle>180 || !std::isfinite(b.flow) || b.flow<0 || b.flow>1)return false;
            b.pressureSize=p.value("pressureSize").toBool();b.pressureFlow=p.value("pressureFlow").toBool();
        }
        b.builtin=builtins.contains(b.id);seen.insert(b.id);restored.append(b);}
    const auto selected=root.value("selected").toString();if(!seen.contains(selected) || !seen.contains(builtins))return false;
    const auto sizes=root.value("toolRadii").toArray();
    qreal brush=12,eraser=12;
    for(const auto &preset:restored)if(preset.id==selected)brush=eraser=preset.radius;
    if(root.contains("toolRadii")) {
        if(!root.value("toolRadii").isArray() || sizes.size()!=2)return false;
        brush=sizes[0].toDouble(std::numeric_limits<double>::quiet_NaN());eraser=sizes[1].toDouble(std::numeric_limits<double>::quiet_NaN());
        if(!std::isfinite(brush) || !std::isfinite(eraser) || brush<.5 || brush>256 || eraser<.5 || eraser>256)return false;
    }
    QStringList recent=m_recentColors;
    // Optional: older brush state has no recent colours and keeps the built-in defaults.
    if(root.contains("recentColors")) {
        if(!root.value("recentColors").isArray())return false;
        recent.clear();
        for(const auto &value:root.value("recentColors").toArray()) {
            const QColor parsed(value.toString());
            if(!parsed.isValid() || recent.size()>=12)return false;
            recent.append(parsed.name(QColor::HexArgb));
        }
    }
    bool outline=true,center=false,hoverOnly=false,simple=false;int width=1;
    QString color="#000000",secondaryColor="#ffffff";
    if(root.contains("cursor")){
        if(!root.value("cursor").isObject())return false;
        const auto cursor=root.value("cursor").toObject();
        for(const auto &key:{"outline","center","hoverOnly"})if(!cursor.value(key).isBool())return false;
        const double line=cursor.value("width").toDouble(0);
        if(line<1 || line>3 || std::floor(line)!=line)return false;
        outline=cursor.value("outline").toBool();center=cursor.value("center").toBool();hoverOnly=cursor.value("hoverOnly").toBool();width=int(line);
        if(cursor.contains("simple")){if(!cursor.value("simple").isBool())return false;simple=cursor.value("simple").toBool();}
        if(cursor.contains("color")){const QColor parsed(cursor.value("color").toString());if(!parsed.isValid() || parsed.alpha()!=255)return false;color=parsed.name();}
        if(cursor.contains("secondaryColor")){const QColor parsed(cursor.value("secondaryColor").toString());if(!parsed.isValid() || parsed.alpha()!=255)return false;secondaryColor=parsed.name();}
    }
    for(auto &preset:restored)preset.token=++m_nextToken;
    m_presets=restored;m_selected=selected;m_toolRadii[0]=brush;m_toolRadii[1]=eraser;m_recentColors=recent;
    m_cursorOutline=outline;m_cursorCenter=center;m_cursorHoverOnly=hoverOnly;m_cursorWidth=width;m_cursorSimple=simple;m_cursorColor=color;m_cursorSecondaryColor=secondaryColor;emit cursorChanged();
    emit settingsChanged();emit presetsChanged();emit recentColorsChanged();return true;
}
