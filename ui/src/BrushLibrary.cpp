#include "BrushLibrary.h"
#include <QColor>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QSet>
#include <QUuid>
#include <QRegularExpression>
#include <algorithm>
#include <cmath>
#include <limits>

namespace {
QVariantMap defaultTexture(){return {{"model","periodic-coverage-v1"},{"enabled",false},{"space","document"},{"scale",1.},{"angle",0.},{"depth",1.},{"invert",false},{"offsetX",0.},{"offsetY",0.},{"resource",QVariant{}}};}
bool validTexture(const QJsonObject &data){
    static const QRegularExpression identity("^[0-9a-f]{64}$");
    if(data.size()!=10 || data.value("model").toString()!="periodic-coverage-v1" || !data.value("enabled").isBool() || !data.value("invert").isBool() || !data.value("space").isString() || (data.value("space").toString()!="document" && data.value("space").toString()!="tip") || !data.contains("resource"))return false;
    const auto resource=data.value("resource");if(!resource.isNull() && (!resource.isString() || !identity.match(resource.toString()).hasMatch()))return false;
    if(data.value("enabled").toBool() && resource.isNull())return false;
    for(const auto &k:{"scale","angle","depth","offsetX","offsetY"}){const auto value=data.value(k);const auto x=value.toDouble();const QString key(k);const double lo=key=="scale"?.01:key=="angle"?-180:key=="depth"?0:-1000000;const double hi=key=="scale"?100:key=="angle"?180:key=="depth"?1:1000000;if(!value.isDouble() || !std::isfinite(x) || x<lo || x>hi)return false;}
    return true;
}
QVariantMap defaultColorDynamics(){return {{"model","linear-hsv-v1"},{"enabled",false},{"foregroundBackground",0.},{"hue",0.},{"saturation",0.},{"value",0.},{"perTip",true},{"seed","0000000000000000"}};}
bool validColorDynamics(const QJsonObject &data){static const QRegularExpression seed("^[0-9a-f]{16}$");if(data.size()!=8 || !data.value("model").isString() || data.value("model").toString()!="linear-hsv-v1" || !data.value("enabled").isBool() || !data.value("perTip").isBool() || !data.value("seed").isString() || !seed.match(data.value("seed").toString()).hasMatch())return false;for(const auto &k:{"foregroundBackground","hue","saturation","value"}){const auto v=data.value(k);const auto x=v.toDouble();if(!v.isDouble() || !std::isfinite(x) || x<0 || x>1)return false;}return true;}

QVariantMap defaultShape(){return {{"enabled",false},{"size",0.},{"minimumSize",0.},{"angle",0.},{"roundness",0.},{"minimumRoundness",.01},{"seed","0000000000000000"}};}
bool validShape(const QJsonObject &data){static const QRegularExpression seed("^[0-9a-f]{16}$");if(data.size()!=7 || !data.value("enabled").isBool() || !data.value("seed").isString() || !seed.match(data.value("seed").toString()).hasMatch())return false;for(const auto &k:{"size","minimumSize","angle","roundness","minimumRoundness"}){const auto v=data.value(k);const auto x=v.toDouble();if(!v.isDouble() || !std::isfinite(x) || x<(QString(k)=="minimumRoundness"?.01:0.) || x>1)return false;}return true;}

QVariantMap defaultScatter(){return {{"enabled",false},{"amount",0.},{"bothAxes",false},{"count",1},{"countJitter",0.},{"seed","0000000000000000"}};}
bool validScatter(const QJsonObject &data){
    static const QRegularExpression seed("^[0-9a-f]{16}$");
    const auto amount=data.value("amount"),count=data.value("count"),jitter=data.value("countJitter");
    return data.size()==6 && data.value("enabled").isBool() && data.value("bothAxes").isBool() && data.value("seed").isString() && seed.match(data.value("seed").toString()).hasMatch()
        && amount.isDouble() && std::isfinite(amount.toDouble()) && amount.toDouble()>=0 && amount.toDouble()<=10
        && count.isDouble() && count.toDouble()>=1 && count.toDouble()<=16 && std::floor(count.toDouble())==count.toDouble()
        && jitter.isDouble() && std::isfinite(jitter.toDouble()) && jitter.toDouble()>=0 && jitter.toDouble()<=1;
}
QVariantMap defaultDevice(){return {{"angleControl","fixed"},{"tiltRoundness",false},{"minimumRoundness",.01}};}
bool validDevice(const QJsonObject &data){
    const QStringList controls={"fixed","direction","pen-rotation","tilt"};
    const auto value=data.value("minimumRoundness");const auto minimum=value.toDouble(-1);
    return data.size()==3 && data.value("angleControl").isString() && controls.contains(data.value("angleControl").toString()) && data.value("tiltRoundness").isBool() && value.isDouble() && std::isfinite(minimum) && minimum>=.01 && minimum<=1;
}
QVariantMap defaultWetMix(){return {{"model","linear-premul-grid-v1"},{"enabled",false},{"allLayers",false},{"autoClean",true},{"autoLoad",true},{"wetness",.5},{"pickup",.5},{"mix",.5},{"load",1.},{"deposit",1.},{"consumption",.1}};}
bool validWetMix(const QJsonObject &data){if(data.size()!=11 || data.value("model").toString()!="linear-premul-grid-v1")return false;for(const auto &key:{"enabled","allLayers","autoClean","autoLoad"})if(!data.value(key).isBool())return false;for(const auto &key:{"wetness","pickup","mix","load","deposit","consumption"}){const auto v=data.value(key);const auto n=v.toDouble();if(!v.isDouble() || !std::isfinite(n) || n<0 || n>1)return false;}return true;}
QVariantMap canonicalWetMix(QVariantMap data){const auto defaults=defaultWetMix();for(const auto &key:{"enabled","allLayers","autoClean","autoLoad"})if(data.value(key)!=defaults.value(key))return data;for(const auto &key:{"wetness","pickup","mix","load","deposit","consumption"})if(float(data.value(key).toDouble())!=float(defaults.value(key).toDouble()))return data;return {};}
bool wetDualConflict(const QVariantMap &wet,const QVariantMap &dual){return wet.value("enabled").toBool() && dual.value("enabled").toBool() && dual.value("color").toMap().value("enabled").toBool();}
QVariantMap defaultSmoothing(){return {{"model","time-grid-v1"},{"enabled",false},{"positionMs",12.},{"pressureMs",8.},{"speedResponse",.5},{"stabilizerPx",0.}};}
bool validSmoothing(const QJsonObject &data){if(data.size()!=6 || !data.value("model").isString() || data.value("model").toString()!="time-grid-v1" || !data.value("enabled").isBool())return false;for(const auto &key:{"positionMs","pressureMs","speedResponse","stabilizerPx"}){const auto v=data.value(key);const double n=v.toDouble(-1);const double maximum=QString(key)=="speedResponse"?1.:QString(key)=="stabilizerPx"?128.:250.;if(!v.isDouble() || !std::isfinite(n) || n<0 || n>maximum)return false;}return true;}
QVariantMap linearPressureControl(){return {{"curve",QVariantList{QVariantList{0.,0.},QVariantList{1.,1.}}},{"minimum",0.}};}
bool smoothPressure(const QVariantMap &data){for(const auto &target:{"size","flow","opacity"})if(data.value(target).toMap().value("interpolation").toString()=="smooth-pchip-v1")return true;return false;}
bool smoothPressure(const QVariantMap &data,const QVariantMap &dual){return smoothPressure(data) || smoothPressure(dual.value("pressureDynamics").toMap());}
QVariantMap defaultPressure(){return {{"size",linearPressureControl()},{"flow",linearPressureControl()}};}
QVariantMap opacityControl(){auto c=linearPressureControl();c["enabled"]=false;return c;}
QVariantMap canonicalPressure(QVariantMap data){if(data.value("opacity").toMap()==opacityControl())data.remove("opacity");if(data==defaultPressure())data.clear();return data;}
bool validPressure(const QJsonObject &data) {
    if((data.size()!=2 && data.size()!=3) || !data.contains("size") || !data.contains("flow") || (data.size()==3 && !data.contains("opacity")))return false;
    for(const auto &target:{"size","flow","opacity"}){
        if(QString(target)=="opacity" && !data.contains(target))continue;
        if(!data.value(target).isObject())return false;const auto c=data.value(target).toObject();
        if((c.contains("interpolation") && c.value("interpolation").toString()!="smooth-pchip-v1") || c.size()!=(QString(target)=="opacity"?3:2)+(c.contains("interpolation")?1:0) || (QString(target)=="opacity" && !c.value("enabled").isBool()) || !c.value("minimum").isDouble() || !c.value("curve").isArray())return false;
        const double minimum=c.value("minimum").toDouble();if(!std::isfinite(minimum) || minimum<0 || minimum>1)return false;
        const auto points=c.value("curve").toArray();if(points.size()<2 || points.size()>17)return false;double previous=-1;float previousNative=-1;
        for(int i=0;i<points.size();++i){if(!points[i].isArray())return false;const auto p=points[i].toArray();if(p.size()!=2 || !p[0].isDouble() || !p[1].isDouble())return false;const auto x=p[0].toDouble(),y=p[1].toDouble();if(!std::isfinite(x) || !std::isfinite(y) || x<0 || x>1 || y<0 || y>1 || x<=previous || static_cast<float>(x)<=previousNative || (i==0 && x!=0) || (i==points.size()-1 && x!=1))return false;previous=x;previousNative=static_cast<float>(x);}
    }return true;
}
}
namespace {
QVariantMap defaultDualColor(){return {{"model","separated-stroke-v1"},{"enabled",false},{"useBackground",false},{"overflow",false},{"order","second-behind"},{"primaryBlend","normal"},{"secondaryBlend","normal"}};}
bool validDualColor(const QJsonObject &data){
    if(data.size()!=7 || data.value("model").toString()!="separated-stroke-v1" || !QStringList{"second-behind","second-in-front"}.contains(data.value("order").toString()))return false;
    for(const auto &key:{"enabled","useBackground","overflow"})if(!data.value(key).isBool())return false;
    const QStringList modes={"normal","dissolve","darken","multiply","color-burn","linear-burn","darker-color","lighten","screen","color-dodge","linear-dodge","lighter-color","overlay","soft-light","hard-light","vivid-light","linear-light","pin-light","hard-mix","difference","exclusion","subtract","divide","hue","saturation","color","luminosity"};
    return modes.contains(data.value("primaryBlend").toString()) && modes.contains(data.value("secondaryBlend").toString());
}
QVariantMap defaultDualSizing(){return {{"model","relative-size-v1"},{"mode","independent"},{"diameterOffset",0.},{"widthRule","follow-pressure"}};}
bool validDualSizing(const QJsonObject &data){
    if(data.size()!=4 || data.value("model").toString()!="relative-size-v1" || !QStringList{"independent","relative"}.contains(data.value("mode").toString()) || !QStringList{"follow-pressure","fixed-width"}.contains(data.value("widthRule").toString()))return false;
    const auto v=data.value("diameterOffset");return v.isDouble() && std::isfinite(v.toDouble()) && v.toDouble()>=-511 && v.toDouble()<=511;
}
QVariantMap defaultDual(){return {{"model","path-coverage-v1"},{"enabled",false},{"mode","multiply"},{"diameter",24.},{"spacing",.25},{"hardness",1.},{"roundness",1.},{"angle",0.},{"flow",1.},{"pressureSize",true},{"pressureFlow",true},{"resource",QVariant{}}};}
bool validDual(const QJsonObject &data){
    static const QSet<QString> allowed={"model","enabled","mode","diameter","spacing","hardness","roundness","angle","flow","pressureSize","pressureFlow","resource","pressureDynamics","tipDynamics","scatterDynamics","shapeJitter","sizing","color"};
    for(auto it=data.begin();it!=data.end();++it)if(!allowed.contains(it.key()))return false;
    for(const auto &key:{"enabled","pressureSize","pressureFlow"})if(!data.value(key).isBool())return false;
    const auto model=data.value("model").toString();const auto spacing=data.value("spacing");
    if(!data.contains("spacing") || (model=="paired-dab-v1"?!spacing.isNull():model=="path-coverage-v1"?(!spacing.isDouble() || !std::isfinite(spacing.toDouble()) || spacing.toDouble()<.01 || spacing.toDouble()>1):true))return false;
    if(!QStringList{"multiply","darken","lighten","subtract","difference","screen"}.contains(data.value("mode").toString()))return false;
    static const QRegularExpression identity("^[0-9a-f]{64}$");const auto resource=data.value("resource");if(!data.contains("resource") || (!resource.isNull() && (!resource.isString() || !identity.match(resource.toString()).hasMatch())))return false;
    for(const auto &key:{"diameter","hardness","roundness","angle","flow"}){const auto v=data.value(key);const auto x=v.toDouble();const QString k(key);const double lo=k=="diameter"?1:k=="roundness"?.01:k=="angle"?-180:0;const double hi=k=="diameter"?512:k=="angle"?180:1;if(!v.isDouble() || !std::isfinite(x) || x<lo || x>hi)return false;}
    if(data.contains("pressureDynamics") && (!data.value("pressureDynamics").isObject() || data.value("pressureDynamics").toObject().contains("opacity") || !validPressure(data.value("pressureDynamics").toObject())))return false;
    if(data.contains("tipDynamics") && (!data.value("tipDynamics").isObject() || !validDevice(data.value("tipDynamics").toObject())))return false;
    if(data.contains("scatterDynamics") && (!data.value("scatterDynamics").isObject() || !validScatter(data.value("scatterDynamics").toObject())))return false;
    if(data.contains("shapeJitter") && (!data.value("shapeJitter").isObject() || !validShape(data.value("shapeJitter").toObject())))return false;
    if(data.contains("sizing") && (!data.value("sizing").isObject() || !validDualSizing(data.value("sizing").toObject())))return false;
    if(data.contains("color") && (!data.value("color").isObject() || !validDualColor(data.value("color").toObject())))return false;
    return true;
}
QVariantMap canonicalDual(QVariantMap data){
    data=QJsonObject::fromVariantMap(data).toVariantMap();
    for(const auto &item:QList<QPair<QString,QVariantMap>>{{"pressureDynamics",defaultPressure()},{"tipDynamics",defaultDevice()},{"scatterDynamics",defaultScatter()},{"shapeJitter",defaultShape()}})if(data.value(item.first).toMap()==item.second)data.remove(item.first);
    if(data.value("color").toMap()==defaultDualColor())data.remove("color");
    if(data.value("sizing").toMap()==defaultDualSizing())data.remove("sizing");
    if(QJsonObject::fromVariantMap(data)==QJsonObject::fromVariantMap(defaultDual()))data.clear();return data;
}
}
QVariantMap BrushLibrary::dualBrush() const{const auto &data=m_presets[index(m_selected)].dual;return data.isEmpty()?defaultDual():data;}
bool BrushLibrary::setDualBrush(QVariantMap data){if(!validDual(QJsonObject::fromVariantMap(data)) || wetDualConflict(wetMix(),data))return false;data=canonicalDual(data);auto &p=m_presets[index(m_selected)];if(p.dual==data)return true;p.dual=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setDualResource(const QString &id,const QString &resource){const int i=index(id);if(i<0)return false;auto &p=m_presets[i];auto data=p.dual.isEmpty()?defaultDual():p.dual;data["resource"]=resource.isEmpty()?QVariant{}:QVariant{resource};if(!validDual(QJsonObject::fromVariantMap(data)))return false;data=canonicalDual(data);if(p.dual==data)return true;p.dual=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setDualPressureControl(const QString &target,const QVariantList &points,qreal minimum){if(target!="size" && target!="flow")return false;auto data=dualBrush();auto controls=data.value("pressureDynamics").toMap();if(controls.isEmpty())controls=defaultPressure();controls[target]=QVariantMap{{"curve",points},{"minimum",minimum},{"interpolation","smooth-pchip-v1"}};data["pressureDynamics"]=controls;return setDualBrush(data);}
void BrushLibrary::resetDualPressureControl(const QString &target){if(target!="size" && target!="flow")return;setDualPressureControl(target,linearPressureControl().value("curve").toList(),0.);}
QVariantMap BrushLibrary::texture() const{const auto &p=m_presets[index(m_selected)].texture;return p.isEmpty()?defaultTexture():p;}
bool BrushLibrary::setTexture(QVariantMap data){if(!validTexture(QJsonObject::fromVariantMap(data)))return false;if(QJsonObject::fromVariantMap(data)==QJsonObject::fromVariantMap(defaultTexture()))data.clear();auto &p=m_presets[index(m_selected)];if(p.texture==data)return true;p.texture=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setTextureResource(const QString &id,const QString &resource){
    const int i=index(id);if(i<0)return false;auto &p=m_presets[i];auto data=p.texture.isEmpty()?defaultTexture():p.texture;
    data["resource"]=resource.isEmpty()?QVariant{}:QVariant{resource};if(resource.isEmpty())data["enabled"]=false;
    if(!validTexture(QJsonObject::fromVariantMap(data)))return false;if(QJsonObject::fromVariantMap(data)==QJsonObject::fromVariantMap(defaultTexture()))data.clear();if(p.texture==data)return true;
    p.texture=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;
}
QVariantMap BrushLibrary::colorDynamics() const{const auto &p=m_presets[index(m_selected)].color;return p.isEmpty()?defaultColorDynamics():p;}
bool BrushLibrary::setColorDynamics(QVariantMap data){if(!validColorDynamics(QJsonObject::fromVariantMap(data)))return false;if(data==defaultColorDynamics())data.clear();auto &p=m_presets[index(m_selected)];if(p.color==data)return true;p.color=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::colorDynamicsEnabled() const{return colorDynamics().value("enabled").toBool();}
void BrushLibrary::setColorDynamicsEnabled(bool value){auto data=colorDynamics();data["enabled"]=value;setColorDynamics(data);}
bool BrushLibrary::colorPerTip() const{return colorDynamics().value("perTip").toBool();}
void BrushLibrary::setColorPerTip(bool value){auto data=colorDynamics();data["perTip"]=value;setColorDynamics(data);}
qreal BrushLibrary::foregroundBackgroundJitter() const{return colorDynamics().value("foregroundBackground").toDouble();}
void BrushLibrary::setForegroundBackgroundJitter(qreal value){auto data=colorDynamics();data["foregroundBackground"]=value;setColorDynamics(data);}
qreal BrushLibrary::hueJitter() const{return colorDynamics().value("hue").toDouble();}
void BrushLibrary::setHueJitter(qreal value){auto data=colorDynamics();data["hue"]=value;setColorDynamics(data);}
qreal BrushLibrary::saturationJitter() const{return colorDynamics().value("saturation").toDouble();}
void BrushLibrary::setSaturationJitter(qreal value){auto data=colorDynamics();data["saturation"]=value;setColorDynamics(data);}
qreal BrushLibrary::valueJitter() const{return colorDynamics().value("value").toDouble();}
void BrushLibrary::setValueJitter(qreal value){auto data=colorDynamics();data["value"]=value;setColorDynamics(data);}
QVariantMap BrushLibrary::shapeJitter() const{const auto &p=m_presets[index(m_selected)].shape;return p.isEmpty()?defaultShape():p;}
bool BrushLibrary::setShapeJitter(QVariantMap data){if(!validShape(QJsonObject::fromVariantMap(data)))return false;if(data==defaultShape())data.clear();auto &p=m_presets[index(m_selected)];if(p.shape==data)return true;p.shape=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::shapeJitterEnabled() const{return shapeJitter().value("enabled").toBool();}
void BrushLibrary::setShapeJitterEnabled(bool value){auto data=shapeJitter();data["enabled"]=value;setShapeJitter(data);}
qreal BrushLibrary::shapeSizeJitter() const{return shapeJitter().value("size").toDouble();}
void BrushLibrary::setShapeSizeJitter(qreal value){auto data=shapeJitter();data["size"]=value;setShapeJitter(data);}
qreal BrushLibrary::shapeMinimumSize() const{return shapeJitter().value("minimumSize").toDouble();}
void BrushLibrary::setShapeMinimumSize(qreal value){auto data=shapeJitter();data["minimumSize"]=value;setShapeJitter(data);}
qreal BrushLibrary::shapeAngleJitter() const{return shapeJitter().value("angle").toDouble();}
void BrushLibrary::setShapeAngleJitter(qreal value){auto data=shapeJitter();data["angle"]=value;setShapeJitter(data);}
qreal BrushLibrary::shapeRoundnessJitter() const{return shapeJitter().value("roundness").toDouble();}
void BrushLibrary::setShapeRoundnessJitter(qreal value){auto data=shapeJitter();data["roundness"]=value;setShapeJitter(data);}
qreal BrushLibrary::shapeMinimumRoundness() const{return shapeJitter().value("minimumRoundness").toDouble();}
void BrushLibrary::setShapeMinimumRoundness(qreal value){auto data=shapeJitter();data["minimumRoundness"]=value;setShapeJitter(data);}
QVariantMap BrushLibrary::scatterDynamics() const{const auto &p=m_presets[index(m_selected)].scatter;return p.isEmpty()?defaultScatter():p;}
bool BrushLibrary::scatterEnabled() const{return scatterDynamics().value("enabled").toBool();}
bool BrushLibrary::scatterBothAxes() const{return scatterDynamics().value("bothAxes").toBool();}
qreal BrushLibrary::scatterAmount() const{return scatterDynamics().value("amount").toDouble();}
int BrushLibrary::scatterCount() const{return scatterDynamics().value("count").toInt();}
qreal BrushLibrary::scatterCountJitter() const{return scatterDynamics().value("countJitter").toDouble();}
bool BrushLibrary::setScatterDynamics(QVariantMap data){if(!validScatter(QJsonObject::fromVariantMap(data)))return false;if(data==defaultScatter())data.clear();auto &p=m_presets[index(m_selected)];if(p.scatter==data)return true;p.scatter=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
void BrushLibrary::setScatterEnabled(bool enabled){auto data=scatterDynamics();data["enabled"]=enabled;setScatterDynamics(data);}
void BrushLibrary::setScatterBothAxes(bool enabled){auto data=scatterDynamics();data["bothAxes"]=enabled;setScatterDynamics(data);}
void BrushLibrary::setScatterAmount(qreal amount){auto data=scatterDynamics();data["amount"]=amount;setScatterDynamics(data);}
void BrushLibrary::setScatterCount(int count){auto data=scatterDynamics();data["count"]=count;setScatterDynamics(data);}
void BrushLibrary::setScatterCountJitter(qreal jitter){auto data=scatterDynamics();data["countJitter"]=jitter;setScatterDynamics(data);}
QVariantMap BrushLibrary::tipDynamics() const{const auto &p=m_presets[index(m_selected)].device;return p.isEmpty()?defaultDevice():p;}
QString BrushLibrary::angleControl() const{return tipDynamics().value("angleControl").toString();}
bool BrushLibrary::tiltRoundness() const{return tipDynamics().value("tiltRoundness").toBool();}
qreal BrushLibrary::minimumRoundness() const{return tipDynamics().value("minimumRoundness").toDouble();}
void BrushLibrary::setTipDynamics(QVariantMap data){if(!validDevice(QJsonObject::fromVariantMap(data)))return;if(data==defaultDevice())data.clear();auto &p=m_presets[index(m_selected)];if(p.device==data)return;p.device=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();}
void BrushLibrary::setAngleControl(const QString &control){auto data=tipDynamics();data["angleControl"]=control;setTipDynamics(data);}
void BrushLibrary::setTiltRoundness(bool enabled){auto data=tipDynamics();data["tiltRoundness"]=enabled;setTipDynamics(data);}
void BrushLibrary::setMinimumRoundness(qreal minimum){auto data=tipDynamics();data["minimumRoundness"]=minimum;setTipDynamics(data);}
QVariantMap BrushLibrary::wetMix() const{const auto &data=m_presets[index(m_selected)].wet;return data.isEmpty()?defaultWetMix():data;}
bool BrushLibrary::setWetMix(QVariantMap data){if(!validWetMix(QJsonObject::fromVariantMap(data)))return false;auto &p=m_presets[index(m_selected)];if(wetDualConflict(data,p.dual))return false;data=canonicalWetMix(data);if(p.wet==data)return true;p.wet=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
QVariantMap BrushLibrary::smoothing() const{const auto &data=m_presets[index(m_selected)].smoothing;return data.isEmpty()?defaultSmoothing():data;}
bool BrushLibrary::setSmoothing(QVariantMap data){if(!validSmoothing(QJsonObject::fromVariantMap(data)))return false;if(QJsonObject::fromVariantMap(data)==QJsonObject::fromVariantMap(defaultSmoothing()))data.clear();auto &p=m_presets[index(m_selected)];if(p.smoothing==data)return true;p.smoothing=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
QVariantMap BrushLibrary::pressureDynamics() const {const auto &p=m_presets[index(m_selected)].pressure;auto result=p.isEmpty()?defaultPressure():p;if(!result.contains("opacity"))result["opacity"]=opacityControl();return result;}
bool BrushLibrary::setPressureControl(const QString &target,const QVariantList &points,qreal minimum) {
    if(target!="size" && target!="flow" && target!="opacity")return false;
    auto data=pressureDynamics();auto c=QVariantMap{{"curve",points},{"minimum",minimum},{"interpolation","smooth-pchip-v1"}};if(target=="opacity")c["enabled"]=pressureOpacity();data[target]=c;
    if(!validPressure(QJsonObject::fromVariantMap(data)))return false;
    data=canonicalPressure(data);auto &p=m_presets[index(m_selected)];if(p.pressure==data)return true;
    p.pressure=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;
}
void BrushLibrary::resetPressureControl(const QString &target){if(target=="size" || target=="flow" || target=="opacity")setPressureControl(target,linearPressureControl().value("curve").toList(),0.);}
BrushLibrary::BrushLibrary(QObject *parent):QObject(parent) {
    m_presets={{"round-pressure",QStringLiteral("压感圆笔"),12,1,.15,true,{},1,0,{}},
               {"round-fine",QStringLiteral("细线圆笔"),3,1,.15,true,{},1,0,{}},
               {"round-translucent",QStringLiteral("透明圆笔"),18,.25,.15,true,{},1,0,{}},
               {"round-wide",QStringLiteral("宽幅圆笔"),48,1,.25,true,{},1,0,{}}};
    for(const auto &p:m_presets)m_resetPresets.insert(p.id,p);
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
QString BrushLibrary::sourceResource() const{return m_presets[index(m_selected)].sourceResource;}
QString BrushLibrary::maskResource() const{return m_presets[index(m_selected)].maskResource;}
bool BrushLibrary::setMaskResource(const QString &id,const QString &resource) {
    const int i=index(id);
    static const QRegularExpression identity("^[0-9a-f]{64}$");
    if(i<0 || (!resource.isEmpty() && !identity.match(resource).hasMatch()))return false;
    auto &p=m_presets[i];if(p.maskResource==resource)return true;
    p.maskResource=resource;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;
}
QString BrushLibrary::previewMessage() const{const auto &p=m_presets[index(m_selected)];return p.error.isEmpty()?QStringLiteral("生成预览…"):p.error;}
qreal BrushLibrary::radius() const{return m_presets[index(m_selected)].radius;}
qreal BrushLibrary::opacity() const{return m_presets[index(m_selected)].opacity;}
qreal BrushLibrary::spacing() const{return m_presets[index(m_selected)].spacing;}
qreal BrushLibrary::hardness() const{return m_presets[index(m_selected)].hardness;}
qreal BrushLibrary::roundness() const{return m_presets[index(m_selected)].roundness;}
qreal BrushLibrary::angle() const{return m_presets[index(m_selected)].angle;}
qreal BrushLibrary::flow() const{return m_presets[index(m_selected)].flow;}
bool BrushLibrary::pressureSize() const{return m_presets[index(m_selected)].pressureSize;}
bool BrushLibrary::pressureOpacity() const{return m_presets[index(m_selected)].pressure.value("opacity").toMap().value("enabled").toBool();}
void BrushLibrary::setPressureOpacity(bool enabled){auto data=pressureDynamics();auto c=data.value("opacity").toMap();c["enabled"]=enabled;data["opacity"]=c;data=canonicalPressure(data);auto &p=m_presets[index(m_selected)];if(p.pressure==data)return;p.pressure=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();}
bool BrushLibrary::pressureFlow() const{return m_presets[index(m_selected)].pressureFlow;}
QVariantMap BrushLibrary::tipSettings() const{return {{"hardness",hardness()},{"roundness",roundness()},{"angle",angle()},{"flow",flow()},{"pressureSize",pressureSize()},{"pressureFlow",pressureFlow()},{"maskResource",maskResource()},{"pressureDynamics",m_presets[index(m_selected)].pressure},{"tipDynamics",m_presets[index(m_selected)].device},{"scatterDynamics",m_presets[index(m_selected)].scatter},{"shapeJitter",m_presets[index(m_selected)].shape},{"colorDynamics",m_presets[index(m_selected)].color},{"texture",m_presets[index(m_selected)].texture},{"dualBrush",m_presets[index(m_selected)].dual},{"smoothing",m_presets[index(m_selected)].smoothing},{"wetMix",m_presets[index(m_selected)].wet},{"presetRadius",radius()},{"writeBlend",0}};}
QVariantList BrushLibrary::presets() const {
    QVariantList rows;for(const auto &p:m_presets)rows.append(QVariantMap{{"id",p.id},{"name",p.name},{"radius",p.radius},{"opacity",p.opacity},{"spacing",p.spacing},{"builtin",p.builtin},{"preview",p.preview}});return rows;
}
void BrushLibrary::setWriteBlend(int mode) {
    if(mode<0 || mode>26 || mode==m_writeBlend)return;
    m_writeBlend=mode;
    emit settingsChanged();emit persistRequested(snapshot());
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
    update(0,value);
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
void BrushLibrary::setCursorComposite(bool value){if(m_cursorComposite==value)return;m_cursorComposite=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorSimple(bool value){if(m_cursorSimple==value)return;m_cursorSimple=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorColor(const QString &value){const QColor color(value);if(!color.isValid() || color.alpha()!=255 || color.name()==m_cursorColor)return;m_cursorColor=color.name();cursorPreferenceChanged();}
void BrushLibrary::setCursorSecondaryColor(const QString &value){const QColor color(value);if(!color.isValid() || color.alpha()!=255 || color.name()==m_cursorSecondaryColor)return;m_cursorSecondaryColor=color.name();cursorPreferenceChanged();}
bool BrushLibrary::select(const QString &id){if(index(id)<0)return false;if(id!=m_selected){m_selected=id;changed();}return true;}
bool BrushLibrary::resetPreset() {
    const int i=index(m_selected);if(i<0 || !m_resetPresets.contains(m_selected))return false;
    auto reset=m_resetPresets.value(m_selected);reset.id=m_selected;reset.name=m_presets[i].name;reset.builtin=m_presets[i].builtin;
    reset.preview.clear();reset.error.clear();reset.token=++m_nextToken;reset.requested=0;
    m_presets[i]=reset;changed();return true;
}
QString BrushLibrary::saveCopy(const QString &name){if(m_presets.size()>=1024 || name.trimmed().isEmpty() || name.size()>80)return {};auto p=m_presets[index(m_selected)];p.id=QUuid::createUuid().toString(QUuid::WithoutBraces);p.name=name.trimmed();p.builtin=false;p.token=++m_nextToken;p.requested=0;p.error.clear();m_presets.append(p);m_resetPresets.insert(p.id,p);m_selected=p.id;changed();return p.id;}
QString BrushLibrary::createOutlinePreset(bool fixedWidth) {
    if(m_presets.size()>=1024)return {};
    Preset p;p.id=QUuid::createUuid().toString(QUuid::WithoutBraces);p.name=fixedWidth?QStringLiteral("描边圆笔 · 固定边宽"):QStringLiteral("描边圆笔 · 跟随压感");
    p.spacing=.1;p.pressureFlow=false;
    auto dual=defaultDual();dual["enabled"]=true;dual["model"]="paired-dab-v1";dual["spacing"]=QVariant{};dual["pressureFlow"]=false;
    dual["sizing"]=QVariantMap{{"model","relative-size-v1"},{"mode","relative"},{"diameterOffset",8.},{"widthRule",fixedWidth?"fixed-width":"follow-pressure"}};
    auto color=defaultDualColor();color["enabled"]=true;color["useBackground"]=true;color["overflow"]=true;dual["color"]=color;
    if(!validDual(QJsonObject::fromVariantMap(dual)))return {};
    p.dual=canonicalDual(dual);p.token=++m_nextToken;
    m_presets.append(p);m_resetPresets.insert(p.id,p);m_selected=p.id;changed();return p.id;
}
bool BrushLibrary::rename(const QString &id,const QString &name){const int i=index(id);if(i<0 || name.trimmed().isEmpty() || name.size()>80)return false;m_presets[i].name=name.trimmed();changed();return true;}
bool BrushLibrary::remove(const QString &id){const int i=index(id);if(i<0 || m_presets[i].builtin)return false;m_presets.removeAt(i);m_resetPresets.remove(id);if(m_selected==id)m_selected="round-pressure";changed();return true;}
void BrushLibrary::setPreviewColors(const QColor &foreground,const QColor &background){
    if(!foreground.isValid() || !background.isValid() || (foreground==m_previewForeground && background==m_previewBackground))return;
    m_previewForeground=foreground;m_previewBackground=background;bool invalidated=false;
    for(auto &p:m_presets)if(p.wet.value("enabled").toBool() || (p.dual.value("enabled").toBool() && p.dual.value("color").toMap().value("enabled").toBool())){p.preview.clear();p.error.clear();p.requested=0;p.token=++m_nextToken;invalidated=true;}
    if(invalidated){emit presetsChanged();requestPreview(m_selected);}
}
void BrushLibrary::requestPreview(const QString &id){const int i=index(id);if(i<0)return;auto &p=m_presets[i];if(!p.preview.isEmpty() || !p.error.isEmpty() || p.requested==p.token)return;p.requested=p.token;QVariantMap tip={{"hardness",p.hardness},{"roundness",p.roundness},{"angle",p.angle},{"flow",p.flow},{"pressureSize",p.pressureSize},{"pressureFlow",p.pressureFlow},{"maskResource",p.maskResource},{"pressureDynamics",p.pressure},{"tipDynamics",p.device},{"scatterDynamics",p.scatter},{"shapeJitter",p.shape},{"colorDynamics",p.color},{"texture",p.texture},{"dualBrush",p.dual},{"smoothing",p.smoothing},{"wetMix",p.wet},{"presetRadius",p.radius},{"writeBlend",0}};if(p.wet.value("enabled").toBool() || (p.dual.value("enabled").toBool() && p.dual.value("color").toMap().value("enabled").toBool())){tip["previewForeground"]=m_previewForeground;tip["previewBackground"]=m_previewBackground;}emit previewRequested(p.id,p.token,24.,p.opacity,p.spacing,tip);}
void BrushLibrary::retryPreview(){auto &p=m_presets[index(m_selected)];p.preview.clear();p.error.clear();p.requested=0;p.token=++m_nextToken;requestPreview(m_selected);emit presetsChanged();}
void BrushLibrary::acceptPreview(const QString &id,quint64 token,const QString &image){const int i=index(id);if(i<0 || m_presets[i].token!=token)return;m_presets[i].requested=0;m_presets[i].preview=image;m_presets[i].error=image.isEmpty()?QStringLiteral("预览暂不可用"):QString();emit presetsChanged();}
QJsonObject BrushLibrary::presetRow(const Preset &p,int &version) {
    QJsonObject row{{"id",p.id},{"name",p.name},{"radius",p.radius},{"opacity",p.opacity},{"spacing",p.spacing},{"hardness",p.hardness},{"roundness",p.roundness},{"angle",p.angle},{"flow",p.flow},{"pressureSize",p.pressureSize},{"pressureFlow",p.pressureFlow}};if(!p.maskResource.isEmpty()){row["maskResource"]=p.maskResource;version=std::max(version,3);}if(!p.sourceResource.isEmpty()){row["sourceResource"]=p.sourceResource;version=std::max(version,4);}if(!p.pressure.isEmpty()){row["pressureDynamics"]=QJsonObject::fromVariantMap(p.pressure);version=std::max(version,p.pressure.contains("opacity")?6:5);}if(!p.device.isEmpty()){row["tipDynamics"]=QJsonObject::fromVariantMap(p.device);version=std::max(version,7);}if(!p.scatter.isEmpty()){row["scatterDynamics"]=QJsonObject::fromVariantMap(p.scatter);version=std::max(version,8);}if(!p.shape.isEmpty()){row["shapeJitter"]=QJsonObject::fromVariantMap(p.shape);version=std::max(version,9);}if(!p.color.isEmpty()){row["colorDynamics"]=QJsonObject::fromVariantMap(p.color);version=std::max(version,10);}if(!p.texture.isEmpty()){row["texture"]=QJsonObject::fromVariantMap(p.texture);version=std::max(version,11);}if(!p.dual.isEmpty()){row["dualBrush"]=QJsonObject::fromVariantMap(p.dual);version=std::max(version,12);}if(smoothPressure(p.pressure,p.dual))version=std::max(version,13);if(!p.smoothing.isEmpty()){row["smoothing"]=QJsonObject::fromVariantMap(p.smoothing);version=std::max(version,14);}if(p.dual.contains("sizing"))version=std::max(version,15);if(p.dual.contains("color"))version=std::max(version,16);if(!p.wet.isEmpty()){row["wetMix"]=QJsonObject::fromVariantMap(p.wet);version=std::max(version,17);}return row;
}
QByteArray BrushLibrary::snapshot() const {
    QJsonArray rows,baselines;int version=2,resetVersion=2;
    for(const auto &p:m_presets){rows.append(presetRow(p,version));baselines.append(presetRow(m_resetPresets.value(p.id,p),resetVersion));}
    const QJsonObject resetState{{"version",resetVersion},{"selected",m_selected},{"presets",baselines}};
    return QJsonDocument(QJsonObject{{"version",version},{"selected",m_selected},{"presets",rows},{"cursor",QJsonObject{{"outline",m_cursorOutline},{"center",m_cursorCenter},{"hoverOnly",m_cursorHoverOnly},{"width",m_cursorWidth},{"simple",m_cursorSimple},{"composite",m_cursorComposite},{"color",m_cursorColor},{"secondaryColor",m_cursorSecondaryColor}}},
                                     {"recentColors",QJsonArray::fromStringList(m_recentColors)},{"resetState",resetState},{"brushTool",QJsonObject{{"writeBlend",m_writeBlend}}}}).toJson(QJsonDocument::Compact);
}
QByteArray BrushLibrary::exportPackage(bool includeSource) const {
    const auto &p=m_presets[index(m_selected)];
    if(!p.sourceResource.isEmpty() && !includeSource)return {};
    const QStringList builtins={"round-pressure","round-fine","round-translucent","round-wide"};
    const int builtin=builtins.indexOf(p.id);
    const auto id=builtin<0?p.id:QString("dba00000-0000-4000-8000-%1").arg(builtin+1,12,16,QChar('0'));
    QJsonObject parameters{{"diameter",p.radius*2},{"spacing",p.spacing/2},{"dabOpacity",p.opacity},{"hardness",p.hardness},{"roundness",p.roundness},{"angle",p.angle},{"flow",p.flow},{"pressureSize",p.pressureSize},{"pressureFlow",p.pressureFlow}};
    QJsonObject preset{{"id",id},{"name",p.name},{"engineProfile","drawverse.brush"},{"behaviorVersion",1},{"requiredCapabilities",QJsonArray{p.maskResource.isEmpty()?"procedural-tip":"image-tip","pressure","dab-flow"}},{"parameters",parameters}};
    if(!p.pressure.isEmpty()){parameters["pressureDynamics"]=QJsonObject::fromVariantMap(p.pressure);preset["parameters"]=parameters;preset["behaviorVersion"]=p.pressure.contains("opacity")?3:2;auto caps=preset.value("requiredCapabilities").toArray();caps.append("pressure-curves");if(p.pressure.contains("opacity"))caps.append("opacity-pressure");preset["requiredCapabilities"]=caps;}
    if(!p.device.isEmpty()){parameters["tipDynamics"]=QJsonObject::fromVariantMap(p.device);preset["parameters"]=parameters;preset["behaviorVersion"]=4;auto caps=preset.value("requiredCapabilities").toArray();caps.append("tip-dynamics");preset["requiredCapabilities"]=caps;}
    if(!p.scatter.isEmpty()){parameters["scatterDynamics"]=QJsonObject::fromVariantMap(p.scatter);preset["parameters"]=parameters;preset["behaviorVersion"]=5;auto caps=preset.value("requiredCapabilities").toArray();caps.append("scatter-stamps");preset["requiredCapabilities"]=caps;}
    if(!p.shape.isEmpty()){parameters["shapeJitter"]=QJsonObject::fromVariantMap(p.shape);preset["parameters"]=parameters;preset["behaviorVersion"]=6;auto caps=preset.value("requiredCapabilities").toArray();caps.append("shape-jitter");preset["requiredCapabilities"]=caps;}
    if(!p.color.isEmpty()){parameters["colorDynamics"]=QJsonObject::fromVariantMap(p.color);preset["parameters"]=parameters;preset["behaviorVersion"]=7;auto caps=preset.value("requiredCapabilities").toArray();caps.append("color-dynamics");preset["requiredCapabilities"]=caps;}
    if(!p.texture.isEmpty()){preset["texture"]=QJsonObject::fromVariantMap(p.texture);preset["behaviorVersion"]=8;auto caps=preset.value("requiredCapabilities").toArray();caps.append("texture-cover");preset["requiredCapabilities"]=caps;}
    if(!p.dual.isEmpty()){preset["dualBrush"]=QJsonObject::fromVariantMap(p.dual);preset["behaviorVersion"]=9;auto caps=preset.value("requiredCapabilities").toArray();caps.append("dual-cover");preset["requiredCapabilities"]=caps;}
    if(smoothPressure(p.pressure,p.dual)){preset["behaviorVersion"]=10;auto caps=preset.value("requiredCapabilities").toArray();caps.append("pressure-smooth");preset["requiredCapabilities"]=caps;}
    if(!p.smoothing.isEmpty()){preset["smoothing"]=QJsonObject::fromVariantMap(p.smoothing);preset["behaviorVersion"]=11;auto caps=preset.value("requiredCapabilities").toArray();caps.append("path-smoothing");preset["requiredCapabilities"]=caps;}
    if(p.dual.contains("sizing")){preset["behaviorVersion"]=12;auto caps=preset.value("requiredCapabilities").toArray();caps.append("dual-relative-size");preset["requiredCapabilities"]=caps;}
    if(p.dual.contains("color")){preset["behaviorVersion"]=13;auto caps=preset.value("requiredCapabilities").toArray();caps.append("dual-color-stroke");preset["requiredCapabilities"]=caps;}
    if(!p.wet.isEmpty()){preset["wetMix"]=QJsonObject::fromVariantMap(p.wet);preset["behaviorVersion"]=14;auto caps=preset.value("requiredCapabilities").toArray();caps.append("wet-mix");preset["requiredCapabilities"]=caps;}
    if(!p.maskResource.isEmpty())preset["tipResource"]=p.maskResource;
    if(!p.sourceResource.isEmpty())preset["sourceResource"]=p.sourceResource;
    return QJsonDocument(QJsonObject{{"packageId",QUuid::createUuid().toString(QUuid::WithoutBraces)},{"brushes",QJsonArray{preset}}}).toJson(QJsonDocument::Compact);
}
QByteArray BrushLibrary::catalogSnapshot() const {
    const auto root=QJsonDocument::fromJson(snapshot()).object();
    return QJsonDocument(QJsonObject{{"version",root.value("version")},{"selected",root.value("selected")},{"presets",root.value("presets")}}).toJson(QJsonDocument::Compact);
}
bool BrushLibrary::applyNativeCatalog(const QByteArray &json) {
    const auto native=QJsonDocument::fromJson(json).object();
    if((native.value("version").toInt()!=2 && native.value("version").toInt()!=3 && native.value("version").toInt()!=4 && native.value("version").toInt()!=5 && native.value("version").toInt()!=6 && native.value("version").toInt()!=7 && native.value("version").toInt()!=8 && native.value("version").toInt()!=9 && native.value("version").toInt()!=10 && native.value("version").toInt()!=11 && native.value("version").toInt()!=12 && native.value("version").toInt()!=13 && native.value("version").toInt()!=14 && native.value("version").toInt()!=15 && native.value("version").toInt()!=16 && native.value("version").toInt()!=17) || !native.value("presets").isArray() || !native.value("selected").isString())return false;
    if(QJsonDocument(native).toJson(QJsonDocument::Compact)==catalogSnapshot())return true;
    auto state=QJsonDocument::fromJson(snapshot()).object();state["version"]=native.value("version");state["selected"]=native.value("selected");state["presets"]=native.value("presets");
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
        if(QUuid(id).isNull() || b.value("behaviorVersion").toInt()!=(b.contains("wetMix")?14:b.value("dualBrush").toObject().contains("color")?13:b.value("dualBrush").toObject().contains("sizing")?12:b.contains("smoothing")?11:smoothPressure(params.value("pressureDynamics").toObject().toVariantMap(),b.value("dualBrush").toObject().toVariantMap())?10:b.contains("dualBrush")?9:b.contains("texture")?8:params.contains("colorDynamics")?7:params.contains("shapeJitter")?6:params.contains("scatterDynamics")?5:params.contains("tipDynamics")?4:params.value("pressureDynamics").toObject().contains("opacity")?3:params.contains("pressureDynamics")?2:1) || (b.value("engineProfile").toString()!="drawverse.brush" && b.value("engineProfile").toString()!="legacy.round"))return false;
        if(ids.contains(id))id=QUuid::createUuid().toString(QUuid::WithoutBraces);
        ids.insert(id);if(selected.isEmpty())selected=id;
        QJsonObject row{{"id",id},{"name",b.value("name")},{"radius",params.value("diameter").toDouble()/2},{"opacity",params.value("dabOpacity")},{"spacing",params.value("spacing").toDouble()*2}};
        for(const auto &key:{"hardness","roundness","angle","flow","pressureSize","pressureFlow"})row[key]=params.value(key);
        if(b.contains("tipResource")){row["maskResource"]=b.value("tipResource");state["version"]=std::max(state.value("version").toInt(),3);}
        if(b.contains("sourceResource")){row["sourceResource"]=b.value("sourceResource");state["version"]=std::max(state.value("version").toInt(),4);}
        if(params.contains("pressureDynamics")){row["pressureDynamics"]=params.value("pressureDynamics");state["version"]=std::max(state.value("version").toInt(),params.value("pressureDynamics").toObject().contains("opacity")?6:5);}
        if(params.contains("tipDynamics")){row["tipDynamics"]=params.value("tipDynamics");state["version"]=std::max(state.value("version").toInt(),7);}
        if(params.contains("scatterDynamics")){row["scatterDynamics"]=params.value("scatterDynamics");state["version"]=std::max(state.value("version").toInt(),8);}
        if(params.contains("shapeJitter")){row["shapeJitter"]=params.value("shapeJitter");state["version"]=std::max(state.value("version").toInt(),9);}
        if(params.contains("colorDynamics")){row["colorDynamics"]=params.value("colorDynamics");state["version"]=std::max(state.value("version").toInt(),10);}
        if(b.contains("texture")){row["texture"]=b.value("texture");state["version"]=std::max(state.value("version").toInt(),11);}
        if(b.contains("dualBrush")){row["dualBrush"]=b.value("dualBrush");state["version"]=std::max(state.value("version").toInt(),12);}
        if(smoothPressure(params.value("pressureDynamics").toObject().toVariantMap(),b.value("dualBrush").toObject().toVariantMap()))state["version"]=std::max(state.value("version").toInt(),13);
        if(b.contains("smoothing")){row["smoothing"]=b.value("smoothing");state["version"]=std::max(state.value("version").toInt(),14);}
        if(b.value("dualBrush").toObject().contains("sizing"))state["version"]=std::max(state.value("version").toInt(),15);
        if(b.value("dualBrush").toObject().contains("color"))state["version"]=std::max(state.value("version").toInt(),16);
        if(b.contains("wetMix")){row["wetMix"]=b.value("wetMix");state["version"]=std::max(state.value("version").toInt(),17);}
        rows.append(row);
    }
    state["presets"]=rows;state["selected"]=selected;
    if(!restore(QJsonDocument(state).toJson(QJsonDocument::Compact)))return false;
    emit persistRequested(snapshot());return true;
}
bool BrushLibrary::importAbr(const QByteArray &json) {
    if(json.size()>2*1024*1024)return false;
    const auto handoff=QJsonDocument::fromJson(json).object();
    if((handoff.value("version").toInt()!=1 && handoff.value("version").toInt()!=2 && handoff.value("version").toInt()!=3 && handoff.value("version").toInt()!=4) || handoff.value("format").toString()!="abr" || !handoff.value("presets").isArray())return false;
    const auto imported=handoff.value("presets").toArray();
    if(imported.isEmpty() || imported.size()+m_presets.size()>1024)return false;
    auto state=QJsonDocument::fromJson(snapshot()).object();auto rows=state.value("presets").toArray();QString selected;
    for(const auto &value:imported) {
        if(!value.isObject())return false;const auto preset=value.toObject();const auto parameters=preset.value("parameters").toObject();
        if(!preset.value("partial").isBool() || !preset.value("sourceResource").isString())return false;
        const auto id=QUuid::createUuid().toString(QUuid::WithoutBraces);if(selected.isEmpty())selected=id;
        QJsonObject row{{"id",id},{"name",preset.value("name")},{"radius",parameters.value("diameter").toDouble()/2},{"opacity",parameters.value("dabOpacity")},{"spacing",parameters.value("spacing").toDouble()*2},{"sourceResource",preset.value("sourceResource")}};
        for(const auto &key:{"hardness","roundness","angle","flow","pressureSize","pressureFlow"})row[key]=parameters.value(key);
        if(parameters.contains("colorDynamics")) {
            if(handoff.value("version").toInt()<4 || !parameters.value("colorDynamics").isObject())return false;
            row["colorDynamics"]=parameters.value("colorDynamics");state["version"]=std::max(state.value("version").toInt(),10);
        }
        if(parameters.contains("scatterDynamics")) {
            if(handoff.value("version").toInt()<3 || !parameters.value("scatterDynamics").isObject())return false;
            row["scatterDynamics"]=parameters.value("scatterDynamics");state["version"]=std::max(state.value("version").toInt(),8);
        }
        if(parameters.contains("pressureDynamics")) {
            if(handoff.value("version").toInt()<2 || !parameters.value("pressureDynamics").isObject())return false;
            row["pressureDynamics"]=parameters.value("pressureDynamics");state["version"]=std::max(state.value("version").toInt(),parameters.value("pressureDynamics").toObject().contains("opacity")?6:5);
        }
        if(!preset.value("tipResource").isNull())row["maskResource"]=preset.value("tipResource");
        rows.append(row);
    }
    state["presets"]=rows;state["selected"]=selected;state["version"]=std::max(state.value("version").toInt(),4);
    if(!restore(QJsonDocument(state).toJson(QJsonDocument::Compact)))return false;
    emit persistRequested(snapshot());return true;
}
bool BrushLibrary::restore(const QByteArray &json) {
    if(json.size()>4*1024*1024+16384)return false;
    const auto root=QJsonDocument::fromJson(json).object();const auto rows=root.value("presets").toArray();
    const QJsonObject executing{{"version",root.value("version")},{"selected",root.value("selected")},{"presets",root.value("presets")}};
    if(QJsonDocument(executing).toJson(QJsonDocument::Compact).size()>2*1024*1024)return false;
    const int version=root.value("version").toInt();
    if((version<1 || version>17) || rows.size()<4 || rows.size()>1024)return false;
    const QSet<QString> builtins{"round-pressure","round-fine","round-translucent","round-wide"};QSet<QString> seen;QList<Preset> restored;
    for(const auto &row:rows){const auto p=row.toObject();Preset b;b.id=p.value("id").toString();b.name=p.value("name").toString();b.radius=p.value("radius").toDouble(std::numeric_limits<double>::quiet_NaN());b.opacity=p.value("opacity").toDouble(std::numeric_limits<double>::quiet_NaN());b.spacing=p.value("spacing").toDouble(std::numeric_limits<double>::quiet_NaN());
        if(b.id.isEmpty() || b.id.size()>64 || seen.contains(b.id) || b.name.trimmed().isEmpty() || b.name.size()>80 || !std::isfinite(b.radius) || b.radius<.5 || b.radius>256 || !std::isfinite(b.opacity) || b.opacity<0 || b.opacity>1 || !std::isfinite(b.spacing) || b.spacing<.02 || b.spacing>1)return false;
        if(p.contains("maskResource")) {
            static const QRegularExpression identity("^[0-9a-f]{64}$");
            if(version<3 || !p.value("maskResource").isString() || !identity.match(p.value("maskResource").toString()).hasMatch())return false;
            b.maskResource=p.value("maskResource").toString();
        }
        if(p.contains("sourceResource")) {
            static const QRegularExpression identity("^[0-9a-f]{64}$");
            if(version<4 || !p.value("sourceResource").isString() || !identity.match(p.value("sourceResource").toString()).hasMatch())return false;
            b.sourceResource=p.value("sourceResource").toString();
        }
        if(p.contains("pressureDynamics")){if(version<(p.value("pressureDynamics").toObject().contains("opacity")?6:5) || !p.value("pressureDynamics").isObject() || !validPressure(p.value("pressureDynamics").toObject()))return false;b.pressure=p.value("pressureDynamics").toObject().toVariantMap();b.pressure=canonicalPressure(b.pressure);}
        if(p.contains("tipDynamics")){if(version<7 || !p.value("tipDynamics").isObject() || !validDevice(p.value("tipDynamics").toObject()))return false;b.device=p.value("tipDynamics").toObject().toVariantMap();if(b.device==defaultDevice())b.device.clear();}
        if(p.contains("scatterDynamics")){if(version<8 || !p.value("scatterDynamics").isObject() || !validScatter(p.value("scatterDynamics").toObject()))return false;b.scatter=p.value("scatterDynamics").toObject().toVariantMap();if(b.scatter==defaultScatter())b.scatter.clear();}
        if(p.contains("shapeJitter")){if(version<9 || !p.value("shapeJitter").isObject() || !validShape(p.value("shapeJitter").toObject()))return false;b.shape=p.value("shapeJitter").toObject().toVariantMap();if(b.shape==defaultShape())b.shape.clear();}
        if(p.contains("colorDynamics")){if(version<10 || !p.value("colorDynamics").isObject() || !validColorDynamics(p.value("colorDynamics").toObject()))return false;b.color=p.value("colorDynamics").toObject().toVariantMap();if(b.color==defaultColorDynamics())b.color.clear();}
        if(p.contains("texture")){if(version<11 || !p.value("texture").isObject() || !validTexture(p.value("texture").toObject()))return false;b.texture=p.value("texture").toObject().toVariantMap();if(QJsonObject::fromVariantMap(b.texture)==QJsonObject::fromVariantMap(defaultTexture()))b.texture.clear();}
        if(p.contains("dualBrush")){if(version<12 || (version<16 && p.value("dualBrush").toObject().contains("color")) || (version<15 && p.value("dualBrush").toObject().contains("sizing")) || !p.value("dualBrush").isObject() || !validDual(p.value("dualBrush").toObject()))return false;b.dual=canonicalDual(p.value("dualBrush").toObject().toVariantMap());}
        if(p.contains("smoothing")){if(version<14 || !p.value("smoothing").isObject() || !validSmoothing(p.value("smoothing").toObject()))return false;b.smoothing=p.value("smoothing").toObject().toVariantMap();if(b.smoothing==defaultSmoothing())b.smoothing.clear();}
        if(p.contains("wetMix")){if(version<17 || !p.value("wetMix").isObject() || !validWetMix(p.value("wetMix").toObject()))return false;b.wet=canonicalWetMix(p.value("wetMix").toObject().toVariantMap());}
        if(wetDualConflict(b.wet,b.dual))return false;
        if(version<13 && smoothPressure(b.pressure,b.dual))return false;
        if(version>=2){
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
    bool outline=true,center=false,hoverOnly=false,simple=false,composite=false;int width=1;
    QString color="#000000",secondaryColor="#ffffff";
    if(root.contains("cursor")){
        if(!root.value("cursor").isObject())return false;
        const auto cursor=root.value("cursor").toObject();
        for(const auto &key:{"outline","center","hoverOnly"})if(!cursor.value(key).isBool())return false;
        const double line=cursor.value("width").toDouble(0);
        if(line<1 || line>3 || std::floor(line)!=line)return false;
        outline=cursor.value("outline").toBool();center=cursor.value("center").toBool();hoverOnly=cursor.value("hoverOnly").toBool();width=int(line);
        if(cursor.contains("composite")){if(!cursor.value("composite").isBool())return false;composite=cursor.value("composite").toBool();}
        if(cursor.contains("simple")){if(!cursor.value("simple").isBool())return false;simple=cursor.value("simple").toBool();}
        if(cursor.contains("color")){const QColor parsed(cursor.value("color").toString());if(!parsed.isValid() || parsed.alpha()!=255)return false;color=parsed.name();}
        if(cursor.contains("secondaryColor")){const QColor parsed(cursor.value("secondaryColor").toString());if(!parsed.isValid() || parsed.alpha()!=255)return false;secondaryColor=parsed.name();}
    }
    int writeBlend=m_writeBlend;
    if(root.contains("brushTool")) {
        if(!root.value("brushTool").isObject())return false;
        const auto tool=root.value("brushTool").toObject();const auto mode=tool.value("writeBlend");
        if(tool.size()!=1 || !mode.isDouble() || !std::isfinite(mode.toDouble()) || mode.toDouble()<0 || mode.toDouble()>26 || std::floor(mode.toDouble())!=mode.toDouble())return false;
        writeBlend=int(mode.toDouble());
    }
    QHash<QString,Preset> baselines;
    BrushLibrary factory;
    if(root.contains("resetState")) {
        if(!root.value("resetState").isObject())return false;
        const auto saved=root.value("resetState").toObject();
        if(saved.contains("resetState") || QJsonDocument(saved).toJson(QJsonDocument::Compact).size()>2*1024*1024)return false;
        BrushLibrary validator;
        if(!validator.restore(QJsonDocument(saved).toJson(QJsonDocument::Compact)))return false;
        for(const auto &p:validator.m_presets)baselines.insert(p.id,p);
    }
    for(const auto &p:restored) {
        if(p.builtin)baselines[p.id]=factory.m_presets[factory.index(p.id)];
        else if(!baselines.contains(p.id))baselines[p.id]=p;
    }
    for(auto it=baselines.begin();it!=baselines.end();)if(!seen.contains(it.key()))it=baselines.erase(it);else ++it;
    m_resetPresets=baselines;
    for(auto &preset:restored) {
        const int current=index(preset.id);int oldVersion=2,newVersion=2;
        if(current>=0 && presetRow(m_presets[current],oldVersion)==presetRow(preset,newVersion)) {
            const auto &old=m_presets[current];preset.preview=old.preview;preset.error=old.error;preset.token=old.token;preset.requested=old.requested;
        } else preset.token=++m_nextToken;
    }
    m_presets=restored;m_selected=selected;m_recentColors=recent;m_writeBlend=writeBlend;
    m_cursorOutline=outline;m_cursorCenter=center;m_cursorHoverOnly=hoverOnly;m_cursorWidth=width;m_cursorSimple=simple;m_cursorComposite=composite;m_cursorColor=color;m_cursorSecondaryColor=secondaryColor;emit cursorChanged();
    emit settingsChanged();emit presetsChanged();emit recentColorsChanged();return true;
}
