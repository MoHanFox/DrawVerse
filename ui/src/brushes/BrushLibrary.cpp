#include "brushes/BrushLibrary.h"
#include "brushes/BrushOrganization.h"
#include <QColor>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QSet>
#include <QUuid>
#include <QRegularExpression>
#include <QJsonValue>
#include <algorithm>
#include <cmath>
#include <functional>
#include <limits>

namespace {
QStringList supportedTextureModes(){return {"multiply","subtract","darken","overlay","colorDodge","colorBurn","linearBurn","hardMix","linearHeight","height"};}
QString normalizeTextureMode(const QString &mode){if(mode=="normal" || !supportedTextureModes().contains(mode))return "multiply";return mode;}
QStringList supportedDualModes(){return {"multiply","darken","overlay","colorDodge","colorBurn","linearBurn","hardMix","linearHeight"};}
QString normalizeDualMode(const QString &mode){return supportedDualModes().contains(mode)?mode:"multiply";}
QVariantMap defaultTexture(){return {{"model","periodic-coverage-v1"},{"enabled",false},{"space","document"},{"mode","multiply"},{"scale",1.},{"angle",0.},{"depth",1.},{"invert",false},{"offsetX",0.},{"offsetY",0.},{"resource",QVariant{}}};}
QSet<QString> advancedTextureModes(){return {"linearBurn","colorDodge","linearDodge","overlay","softLight","hardLight","exclusion","hardMix"};}
QSet<QString> latestTextureModes(){return {"linearHeight","height"};}
QSet<QString> dissolveTextureModes(){return {};}
QVariantMap canonicalTexture(QVariantMap data){if(data.contains("mode"))data["mode"]=normalizeTextureMode(data.value("mode").toString());for(const auto &key:{"brightness","contrast","minimumDepth","depthJitter"})if(data.contains(key) && data.value(key).toDouble()==0.)data.remove(key);if(data.contains("depthPressure") && !data.value("depthPressure").toBool())data.remove("depthPressure");return data;}
bool validTexture(const QJsonObject &data){
    static const QRegularExpression identity("^[0-9a-f]{64}$");
    if(data.size()<11 || data.size()>16 || data.value("model").toString()!="periodic-coverage-v1" || !data.value("enabled").isBool() || !data.value("invert").isBool() || !data.value("space").isString() || (data.value("space").toString()!="document" && data.value("space").toString()!="tip") || !data.value("mode").isString() || !supportedTextureModes().contains(data.value("mode").toString()) || !data.contains("resource"))return false;
    const auto resource=data.value("resource");if(!resource.isNull() && (!resource.isString() || !identity.match(resource.toString()).hasMatch()))return false;
    if(data.value("enabled").toBool() && resource.isNull())return false;
    for(const auto &k:{"brightness","contrast"})if(data.contains(k)){const auto v=data.value(k);const auto x=v.toDouble();if(!v.isDouble() || !std::isfinite(x) || x<-1 || x>1)return false;}
    for(const auto &k:{"minimumDepth","depthJitter"})if(data.contains(k)){const auto v=data.value(k);const auto x=v.toDouble();if(!v.isDouble() || !std::isfinite(x) || x<0 || x>1)return false;}
    if(data.contains("depthPressure") && !data.value("depthPressure").isBool())return false;
    for(const auto &k:{"scale","angle","depth","offsetX","offsetY"}){const auto value=data.value(k);const auto x=value.toDouble();const QString key(k);const double lo=key=="scale"?.01:key=="angle"?-180:key=="depth"?0:-1000000;const double hi=key=="scale"?100:key=="angle"?180:key=="depth"?1:1000000;if(!value.isDouble() || !std::isfinite(x) || x<lo || x>hi)return false;}
    return true;
}
QVariantMap defaultColorDynamics(){return {{"model","linear-hsv-v1"},{"enabled",false},{"foregroundBackground",0.},{"hue",0.},{"saturation",0.},{"value",0.},{"perTip",true},{"seed","0000000000000000"}};}
bool validColorDynamics(const QJsonObject &data){static const QRegularExpression seed("^[0-9a-f]{16}$");if(data.size()!=8 || !data.value("model").isString() || data.value("model").toString()!="linear-hsv-v1" || !data.value("enabled").isBool() || !data.value("perTip").isBool() || !data.value("seed").isString() || !seed.match(data.value("seed").toString()).hasMatch())return false;for(const auto &k:{"foregroundBackground","hue","saturation","value"}){const auto v=data.value(k);const auto x=v.toDouble();if(!v.isDouble() || !std::isfinite(x) || x<0 || x>1)return false;}return true;}

QVariantMap defaultShape(){return {{"enabled",false},{"size",0.},{"minimumSize",0.},{"angle",0.},{"roundness",0.},{"minimumRoundness",.01},{"seed","0000000000000000"}};}
bool validShape(const QJsonObject &data){static const QRegularExpression seed("^[0-9a-f]{16}$");if(data.size()!=7 || !data.value("enabled").isBool() || !data.value("seed").isString() || !seed.match(data.value("seed").toString()).hasMatch())return false;for(const auto &k:{"size","minimumSize","angle","roundness","minimumRoundness"}){const auto v=data.value(k);const auto x=v.toDouble();if(!v.isDouble() || !std::isfinite(x) || x<(QString(k)=="minimumRoundness"?.01:0.) || x>1)return false;}return true;}
QVariantMap defaultTransferJitter(){return {{"model","transfer-jitter-v1"},{"size",0.},{"flow",0.},{"opacity",0.}};}
bool validTransferJitter(const QJsonObject &data){if(data.size()!=4 || data.value("model").toString()!="transfer-jitter-v1")return false;for(const auto &key:{"size","flow","opacity"}){const auto value=data.value(key);const auto number=value.toDouble();if(!value.isDouble() || !std::isfinite(number) || number<0 || number>1)return false;}return true;}
QVariantMap canonicalTransferJitter(QVariantMap data){if(data==defaultTransferJitter())data.clear();return data;}

QVariantMap defaultScatter(){return {{"enabled",false},{"amount",0.},{"bothAxes",false},{"count",1},{"countJitter",0.},{"seed","0000000000000000"}};}
bool validScatterControl(const QJsonValue &value){
    if(!value.isObject())return false;
    const auto data=value.toObject();
    if(data.size()<2 || data.size()>3 || !data.value("source").isString() || !data.value("minimum").isDouble())return false;
    const auto source=data.value("source").toString();
    static const QSet<QString> sources={"off","fade","pressure","tilt","direction","rotation"};
    if(!sources.contains(source))return false;
    const double minimum=data.value("minimum").toDouble();
    if(!std::isfinite(minimum) || minimum<0 || minimum>1)return false;
    const auto steps=data.value("fadeSteps");
    const int fadeSteps=steps.isUndefined()?0:steps.toInt(-1);
    if(!steps.isUndefined() && (!steps.isDouble() || fadeSteps<0 || fadeSteps>1000000 || std::floor(steps.toDouble())!=steps.toDouble()))return false;
    if(source=="off")return minimum==0. && fadeSteps==0;
    if(source=="fade")return fadeSteps>=1;
    return fadeSteps==0;
}
bool scatterHasControls(const QVariantMap &data){return data.contains("amountControl") || data.contains("countControl");}
bool validScatter(const QJsonObject &data){
    static const QRegularExpression seed("^[0-9a-f]{16}$");
    const auto amount=data.value("amount"),count=data.value("count"),jitter=data.value("countJitter");
    return data.size()>=6 && data.size()<=8 && data.value("enabled").isBool() && data.value("bothAxes").isBool() && data.value("seed").isString() && seed.match(data.value("seed").toString()).hasMatch()
        && amount.isDouble() && std::isfinite(amount.toDouble()) && amount.toDouble()>=0 && amount.toDouble()<=10
        && count.isDouble() && count.toDouble()>=1 && count.toDouble()<=16 && std::floor(count.toDouble())==count.toDouble()
        && jitter.isDouble() && std::isfinite(jitter.toDouble()) && jitter.toDouble()>=0 && jitter.toDouble()<=1
        && (!data.contains("amountControl") || validScatterControl(data.value("amountControl")))
        && (!data.contains("countControl") || validScatterControl(data.value("countControl")));
}
QVariantMap defaultDevice(){return {{"angleControl","fixed"},{"tiltRoundness",false},{"minimumRoundness",.01}};}
bool validDevice(const QJsonObject &data){
    const QStringList controls={"fixed","direction","pen-rotation","tilt"};
    const auto value=data.value("minimumRoundness");const auto minimum=value.toDouble(-1);
    return data.size()==3 && data.value("angleControl").isString() && controls.contains(data.value("angleControl").toString()) && data.value("tiltRoundness").isBool() && value.isDouble() && std::isfinite(minimum) && minimum>=.01 && minimum<=1;
}
QVariantMap defaultTaperControl(){return {{"curve",QVariantList{QVariantList{0.,0.},QVariantList{1.,1.}}},{"minimum",0.},{"interpolation","linear"}};}
QVariantMap defaultTaper(){const QVariantMap end{{"lengthPx",0.},{"size",defaultTaperControl()},{"opacity",defaultTaperControl()}};return {{"model","distance-envelope-v1"},{"enabled",false},{"start",end},{"end",end}};}
QVariantMap defaultAirbrush(){return {{"model","time-interval-v1"},{"enabled",false},{"intervalMs",50.}};}
bool validAirbrush(const QJsonObject &data){
    if(data.size()!=3 || data.value("model").toString()!="time-interval-v1" || !data.value("enabled").isBool())return false;
    const auto interval=data.value("intervalMs");return interval.isDouble() && std::isfinite(interval.toDouble()) && interval.toDouble()>=1 && interval.toDouble()<=1000;
}
QVariantMap canonicalAirbrush(QVariantMap data){if(data==defaultAirbrush())data.clear();return data;}
QVariantMap linearPressureControl();
QVariantMap defaultSpeedControl(){return linearPressureControl();}
QVariantMap defaultSpeedDynamics(){return {{"model","speed-v1"},{"enabled",false},{"maxSpeed",2000.},{"size",defaultSpeedControl()},{"flow",defaultSpeedControl()},{"opacity",defaultSpeedControl()}};}
bool validSpeedControl(const QJsonObject &data){
    if((data.size()!=2 && data.size()!=3) || !data.value("minimum").isDouble() || !data.value("curve").isArray() || (data.contains("interpolation") && data.value("interpolation").toString()!="smooth-pchip-v1"))return false;
    const double minimum=data.value("minimum").toDouble();if(!std::isfinite(minimum) || minimum<0 || minimum>1)return false;
    const auto points=data.value("curve").toArray();if(points.size()<2 || points.size()>17)return false;double previous=-1;
    for(qsizetype i=0;i<points.size();++i){if(!points[i].isArray())return false;const auto p=points[i].toArray();if(p.size()!=2 || !p[0].isDouble() || !p[1].isDouble())return false;const double x=p[0].toDouble(),y=p[1].toDouble();if(!std::isfinite(x) || !std::isfinite(y) || x<0 || x>1 || y<0 || y>1 || x<=previous || (i==0 && x!=0) || (i==points.size()-1 && x!=1))return false;previous=x;}
    return true;
}
bool validSpeedDynamics(const QJsonObject &data){
    if(data.size()!=6 || data.value("model").toString()!="speed-v1" || !data.value("enabled").isBool() || !data.value("maxSpeed").isDouble())return false;
    const double maximum=data.value("maxSpeed").toDouble();if(!std::isfinite(maximum) || maximum<1 || maximum>100000)return false;
    for(const auto &key:{"size","flow","opacity"})if(!data.value(key).isObject() || !validSpeedControl(data.value(key).toObject()))return false;
    return true;
}
QVariantMap canonicalSpeedDynamics(QVariantMap data){if(data==defaultSpeedDynamics())data.clear();return data;}
QVariantMap defaultTangentDynamics(){return {{"model","tangent-v1"},{"enabled",false},{"size",defaultSpeedControl()},{"flow",defaultSpeedControl()},{"opacity",defaultSpeedControl()}};}
bool validTangentDynamics(const QJsonObject &data){
    if(data.size()!=5 || data.value("model").toString()!="tangent-v1" || !data.value("enabled").isBool())return false;
    for(const auto &key:{"size","flow","opacity"})if(!data.value(key).isObject() || !validSpeedControl(data.value(key).toObject()))return false;
    return true;
}
QVariantMap canonicalTangentDynamics(QVariantMap data){if(data==defaultTangentDynamics())data.clear();return data;}
QVariantMap defaultDistanceDynamics(){return {{"model","distance-v1"},{"enabled",false},{"maxDistance",1000.},{"size",defaultSpeedControl()},{"flow",defaultSpeedControl()},{"opacity",defaultSpeedControl()}};}
bool validDistanceDynamics(const QJsonObject &data){
    if(data.size()!=6 || data.value("model").toString()!="distance-v1" || !data.value("enabled").isBool() || !data.value("maxDistance").isDouble())return false;
    const double maximum=data.value("maxDistance").toDouble();if(!std::isfinite(maximum) || maximum<1 || maximum>100000)return false;
    for(const auto &key:{"size","flow","opacity"})if(!data.value(key).isObject() || !validSpeedControl(data.value(key).toObject()))return false;
    return true;
}
QVariantMap canonicalDistanceDynamics(QVariantMap data){if(data==defaultDistanceDynamics())data.clear();return data;}
QVariantMap defaultTiltDynamics(){return {{"model","tilt-v1"},{"enabled",false},{"size",defaultSpeedControl()},{"flow",defaultSpeedControl()},{"opacity",defaultSpeedControl()}};}
bool validTiltDynamics(const QJsonObject &data){
    if(data.size()!=5 || data.value("model").toString()!="tilt-v1" || !data.value("enabled").isBool())return false;
    for(const auto &key:{"size","flow","opacity"})if(!data.value(key).isObject() || !validSpeedControl(data.value(key).toObject()))return false;
    return true;
}
QVariantMap canonicalTiltDynamics(QVariantMap data){if(data==defaultTiltDynamics())data.clear();return data;}
QVariantMap defaultRotationDynamics(){return {{"model","rotation-v1"},{"enabled",false},{"size",defaultSpeedControl()},{"flow",defaultSpeedControl()},{"opacity",defaultSpeedControl()}};}
bool validRotationDynamics(const QJsonObject &data){
    if(data.size()!=5 || data.value("model").toString()!="rotation-v1" || !data.value("enabled").isBool())return false;
    for(const auto &key:{"size","flow","opacity"})if(!data.value(key).isObject() || !validSpeedControl(data.value(key).toObject()))return false;
    return true;
}
QVariantMap canonicalRotationDynamics(QVariantMap data){if(data==defaultRotationDynamics())data.clear();return data;}
QVariantMap defaultDirectionDynamics(){return {{"model","direction-v1"},{"enabled",false},{"size",defaultSpeedControl()},{"flow",defaultSpeedControl()},{"opacity",defaultSpeedControl()}};}
bool validDirectionDynamics(const QJsonObject &data){
    if(data.size()!=5 || data.value("model").toString()!="direction-v1" || !data.value("enabled").isBool())return false;
    for(const auto &key:{"size","flow","opacity"})if(!data.value(key).isObject() || !validSpeedControl(data.value(key).toObject()))return false;
    return true;
}
QVariantMap canonicalDirectionDynamics(QVariantMap data){if(data==defaultDirectionDynamics())data.clear();return data;}
bool equivalentSettings(const QJsonValue &left, const QJsonValue &right) {
    if(left.isDouble() && right.isDouble())return float(left.toDouble())==float(right.toDouble());
    if(left.isObject() && right.isObject()) {
        const auto a=left.toObject(),b=right.toObject();
        if(a.size()!=b.size())return false;
        for(auto it=a.begin();it!=a.end();++it)
            if(!b.contains(it.key()) || !equivalentSettings(it.value(),b.value(it.key())))return false;
        return true;
    }
    if(left.isArray() && right.isArray()) {
        const auto a=left.toArray(),b=right.toArray();
        if(a.size()!=b.size())return false;
        for(qsizetype i=0;i<a.size();++i)
            if(!equivalentSettings(a[i],b[i]))return false;
        return true;
    }
    return left==right;
}
QVariantMap defaultFadeDynamics(){return {{"model","fade-v1"},{"enabled",false},{"steps",25},{"size",defaultSpeedControl()},{"flow",defaultSpeedControl()},{"opacity",defaultSpeedControl()}};}
bool validFadeDynamics(const QJsonObject &data){
    if(data.size()!=6 || data.value("model").toString()!="fade-v1" || !data.value("enabled").isBool())return false;
    const auto steps=data.value("steps");
    if(!steps.isDouble() || !std::isfinite(steps.toDouble()) || steps.toDouble()<1 || steps.toDouble()>10000 || std::floor(steps.toDouble())!=steps.toDouble())return false;
    for(const auto &key:{"size","flow","opacity"})if(!data.value(key).isObject() || !validSpeedControl(data.value(key).toObject()))return false;
    return true;
}
QVariantMap canonicalFadeDynamics(QVariantMap data){if(data==defaultFadeDynamics())data.clear();return data;}
QVariantMap defaultDynamicsSourceMatrix(){return {{"model","source-matrix-v1"},{"size","all"},{"flow","all"},{"opacity","all"}};}
bool validDynamicsSourceMatrix(const QJsonObject &data){
    static const QStringList sources={"all","speed","tangent","distance","tilt","rotation","direction"};
    if(data.size()!=4 || data.value("model").toString()!="source-matrix-v1")return false;
    for(const auto &key:{"size","flow","opacity"})if(!data.value(key).isString() || !sources.contains(data.value(key).toString()))return false;
    return true;
}
QVariantMap canonicalDynamicsSourceMatrix(QVariantMap data){if(data==defaultDynamicsSourceMatrix())data.clear();return data;}
bool validTaper(const QJsonObject &data){
    if(data.size()!=4 || data.value("model").toString()!="distance-envelope-v1" || !data.value("enabled").isBool())return false;
    for(const auto &key:{"start","end"}){
        if(!data.value(key).isObject())return false;const auto end=data.value(key).toObject();const auto length=end.value("lengthPx");
        if(end.size()!=3 || !length.isDouble() || !std::isfinite(length.toDouble()) || length.toDouble()<0 || length.toDouble()>4096)return false;
        for(const auto &target:{"size","opacity"}){
            if(!end.value(target).isObject())return false;const auto c=end.value(target).toObject();const auto minimum=c.value("minimum");
            if(c.size()!=3 || !minimum.isDouble() || !std::isfinite(minimum.toDouble()) || minimum.toDouble()<0 || minimum.toDouble()>1 || !QStringList{"linear","pchip"}.contains(c.value("interpolation").toString()) || !c.value("curve").isArray())return false;
            const auto points=c.value("curve").toArray();if(points.size()<2 || points.size()>17)return false;double previous=-1;float previousNative=-1;
            for(int i=0;i<points.size();++i){if(!points[i].isArray())return false;const auto point=points[i].toArray();if(point.size()!=2 || !point[0].isDouble() || !point[1].isDouble())return false;const auto x=point[0].toDouble(),y=point[1].toDouble();if(!std::isfinite(x) || !std::isfinite(y) || x<0 || x>1 || y<0 || y>1 || x<=previous || float(x)<=previousNative || (i==0 && (x!=0 || y!=0)) || (i==points.size()-1 && (x!=1 || y!=1)))return false;previous=x;previousNative=float(x);}
        }
    }return true;
}
QVariantMap defaultWetMix(){return {{"model","linear-premul-grid-v1"},{"enabled",false},{"allLayers",false},{"autoClean",true},{"autoLoad",true},{"wetness",.5},{"pickup",.5},{"mix",.5},{"load",1.},{"deposit",1.},{"consumption",.1}};}
bool validWetMix(const QJsonObject &data){if(data.size()!=11 || data.value("model").toString()!="linear-premul-grid-v1")return false;for(const auto &key:{"enabled","allLayers","autoClean","autoLoad"})if(!data.value(key).isBool())return false;for(const auto &key:{"wetness","pickup","mix","load","deposit","consumption"}){const auto v=data.value(key);const auto n=v.toDouble();if(!v.isDouble() || !std::isfinite(n) || n<0 || n>1)return false;}return true;}
QVariantMap canonicalWetMix(QVariantMap data){data.remove("dualColor");const auto defaults=defaultWetMix();for(const auto &key:{"enabled","allLayers","autoClean","autoLoad"})if(data.value(key)!=defaults.value(key))return data;for(const auto &key:{"wetness","pickup","mix","load","deposit","consumption"})if(float(data.value(key).toDouble())!=float(defaults.value(key).toDouble()))return data;return {};}
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
QVariantMap defaultDualSizing(){return {{"model","relative-size-v1"},{"mode","independent"},{"diameterOffset",0.},{"widthRule","follow-pressure"}};}
bool dualHasRatio(const QVariantMap &dual){return dual.value("sizing").toMap().value("model").toString()=="relative-ratio-v1";}
bool validDualSizing(const QJsonObject &data){
    if(data.size()!=4 || !QStringList{"relative-size-v1","relative-ratio-v1"}.contains(data.value("model").toString()) || !QStringList{"independent","relative"}.contains(data.value("mode").toString()) || !QStringList{"follow-pressure","fixed-width"}.contains(data.value("widthRule").toString()))return false;
    const bool ratio=data.value("model").toString()=="relative-ratio-v1";const auto v=data.value(ratio?"diameterRatio":"diameterOffset");return v.isDouble() && std::isfinite(v.toDouble()) && v.toDouble()>=(ratio?.0005:-511) && v.toDouble()<=(ratio?2000:511);
}
QVariantMap defaultDual(){return {{"model","path-coverage-v1"},{"enabled",false},{"mode","multiply"},{"diameter",24.},{"spacing",.25},{"hardness",1.},{"roundness",1.},{"angle",0.},{"flow",1.},{"pressureSize",true},{"pressureFlow",true},{"resource",QVariant{}}};}
bool extendedDualMode(const QString &mode){static const QSet<QString> modes={"colorBurn","linearBurn","colorDodge","linearDodge","overlay","softLight","hardLight","exclusion","hardMix","vividLight","linearLight","pinLight","divide","linearHeight"};return modes.contains(mode);}
bool dualHasExtendedMode(const QVariantMap &data){return extendedDualMode(data.value("mode").toString());}
bool validDual(const QJsonObject &data){
    // An empty payload is the canonical disabled/default second brush.
    if(data.isEmpty())return true;
    static const QSet<QString> allowed={"model","enabled","flip","flipX","flipY","mode","diameter","spacing","hardness","roundness","angle","flow","pressureSize","pressureFlow","resource","pressureDynamics","tipDynamics","scatterDynamics","shapeJitter","sizing"};
    for(auto it=data.begin();it!=data.end();++it)if(!allowed.contains(it.key()))return false;
    for(const auto &key:{"enabled","pressureSize","pressureFlow"})if(!data.value(key).isBool())return false;
    if(data.contains("flip") && !data.value("flip").isBool())return false;
    for(const auto &key:{"flipX","flipY"})if(data.contains(key) && !data.value(key).isBool())return false;
    const auto model=data.value("model").toString();const auto spacing=data.value("spacing");
    if(!data.contains("spacing") || (model=="paired-dab-v1"?!spacing.isNull():model=="path-coverage-v1"?(!spacing.isDouble() || !std::isfinite(spacing.toDouble()) || spacing.toDouble()<.01 || spacing.toDouble()>1):true))return false;
    if(!supportedDualModes().contains(data.value("mode").toString()))return false;
    static const QRegularExpression identity("^[0-9a-f]{64}$");const auto resource=data.value("resource");if(!data.contains("resource") || (!resource.isNull() && (!resource.isString() || !identity.match(resource.toString()).hasMatch())))return false;
    for(const auto &key:{"diameter","hardness","roundness","angle","flow"}){const auto v=data.value(key);const auto x=v.toDouble();const QString k(key);const double lo=k=="diameter"?1:k=="roundness"?.01:k=="angle"?-180:0;const double hi=k=="diameter"?2000:k=="angle"?180:1;if(!v.isDouble() || !std::isfinite(x) || x<lo || x>hi)return false;}
    if(data.contains("pressureDynamics") && (!data.value("pressureDynamics").isObject() || data.value("pressureDynamics").toObject().contains("opacity") || !validPressure(data.value("pressureDynamics").toObject())))return false;
    if(data.contains("tipDynamics") && (!data.value("tipDynamics").isObject() || !validDevice(data.value("tipDynamics").toObject())))return false;
    if(data.contains("scatterDynamics") && (!data.value("scatterDynamics").isObject() || !validScatter(data.value("scatterDynamics").toObject())))return false;
    if(data.contains("shapeJitter") && (!data.value("shapeJitter").isObject() || !validShape(data.value("shapeJitter").toObject())))return false;
    if(data.contains("sizing") && (!data.value("sizing").isObject() || !validDualSizing(data.value("sizing").toObject())))return false;
    return true;
}
QJsonObject organizationForPreset(const QJsonObject &organization,const QString &presetId,const QString &exportId) {
    if(organization.isEmpty())return {};
    const auto groups=organization.value("groups").toArray();
    QHash<QString,QJsonObject> byId;
    for(const auto &value:groups)byId.insert(value.toObject().value("id").toString(),value.toObject());
    QStringList path;
    std::function<bool(const QJsonArray &)> visit=[&](const QJsonArray &items){
        for(const auto &value:items){
            const auto item=value.toObject();
            if(item.value("kind").toString()=="preset" && item.value("id").toString()==presetId)return true;
            if(item.value("kind").toString()=="group"){
                const auto id=item.value("id").toString();
                path.append(id);
                if(visit(byId.value(id).value("items").toArray()))return true;
                path.removeLast();
            }
        }
        return false;
    };
    if(!visit(organization.value("roots").toArray()))return {};
    QJsonArray items{QJsonObject{{"kind","preset"},{"id",exportId}}};
    QJsonArray outGroups;
    for(qsizetype i=path.size();i>0;--i){
        const auto id=path[i-1];
        outGroups.append(QJsonObject{{"id",id},{"name",byId.value(id).value("name")},{"items",items}});
        items=QJsonArray{QJsonObject{{"kind","group"},{"id",id}}};
    }
    return QJsonObject{{"model","ordered-tree-v1"},{"roots",items},{"groups",outGroups}};
}
QVariantMap canonicalDual(QVariantMap data){
    data=QJsonObject::fromVariantMap(data).toVariantMap();
    data.remove("color");
    if(data.contains("mode"))data["mode"]=normalizeDualMode(data.value("mode").toString());
    if(data.contains("color")){auto color=data.value("color").toMap();color["useBackground"]=true;data["color"]=color;}
    for(const auto &item:QList<QPair<QString,QVariantMap>>{{"pressureDynamics",defaultPressure()},{"tipDynamics",defaultDevice()},{"scatterDynamics",defaultScatter()},{"shapeJitter",defaultShape()}})if(data.value(item.first).toMap()==item.second)data.remove(item.first);
    if(data.value("sizing").toMap()==defaultDualSizing())data.remove("sizing");
    if(data.contains("flip") && !data.value("flip").toBool())data.remove("flip");
    for(const auto &key:{"flipX","flipY"})if(data.contains(key) && !data.value(key).toBool())data.remove(key);
    if(QJsonObject::fromVariantMap(data)==QJsonObject::fromVariantMap(defaultDual()))data.clear();return data;
}
}
QVariantMap BrushLibrary::dualBrush() const{const auto &data=m_presets[index(m_selected)].dual;return data.isEmpty()?defaultDual():data;}
bool BrushLibrary::setDualBrush(QVariantMap data){data=canonicalDual(data);if(!validDual(QJsonObject::fromVariantMap(data)))return false;auto &p=m_presets[index(m_selected)];if(p.dual==data)return true;p.dual=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setDualResource(const QString &id,const QString &resource){const int i=index(id);if(i<0)return false;auto &p=m_presets[i];auto data=p.dual.isEmpty()?defaultDual():p.dual;data["resource"]=resource.isEmpty()?QVariant{}:QVariant{resource};if(!validDual(QJsonObject::fromVariantMap(data)))return false;data=canonicalDual(data);if(p.dual==data)return true;p.dual=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setDualPressureControl(const QString &target,const QVariantList &points,qreal minimum){if(target!="size" && target!="flow")return false;auto data=dualBrush();auto controls=data.value("pressureDynamics").toMap();if(controls.isEmpty())controls=defaultPressure();controls[target]=QVariantMap{{"curve",points},{"minimum",minimum},{"interpolation","smooth-pchip-v1"}};data["pressureDynamics"]=controls;return setDualBrush(data);}
void BrushLibrary::resetDualPressureControl(const QString &target){if(target!="size" && target!="flow")return;setDualPressureControl(target,linearPressureControl().value("curve").toList(),0.);}
QVariantMap BrushLibrary::texture() const{const auto &p=m_presets[index(m_selected)].texture;return p.isEmpty()?defaultTexture():p;}
bool BrushLibrary::setTexture(QVariantMap data){data=canonicalTexture(data);if(!validTexture(QJsonObject::fromVariantMap(data)))return false;if(QJsonObject::fromVariantMap(data)==QJsonObject::fromVariantMap(defaultTexture()))data.clear();auto &p=m_presets[index(m_selected)];if(p.texture==data)return true;p.texture=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
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
QVariantMap BrushLibrary::transferJitter() const{const auto &p=m_presets[index(m_selected)].transfer;return p.isEmpty()?defaultTransferJitter():p;}
bool BrushLibrary::setTransferJitter(QVariantMap data){if(!validTransferJitter(QJsonObject::fromVariantMap(data)))return false;data=canonicalTransferJitter(data);auto &p=m_presets[index(m_selected)];if(p.transfer==data)return true;p.transfer=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
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
QVariantMap BrushLibrary::strokeTaper() const{const auto &data=m_presets[index(m_selected)].taper;return data.isEmpty()?defaultTaper():data;}
bool BrushLibrary::setStrokeTaper(QVariantMap data){if(!validTaper(QJsonObject::fromVariantMap(data)))return false;if(data==defaultTaper())data.clear();auto &p=m_presets[index(m_selected)];if(p.taper==data)return true;p.taper=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
QVariantMap BrushLibrary::airbrush() const{const auto &data=m_presets[index(m_selected)].airbrush;return data.isEmpty()?defaultAirbrush():data;}
bool BrushLibrary::setAirbrush(QVariantMap data){if(!validAirbrush(QJsonObject::fromVariantMap(data)))return false;data=canonicalAirbrush(data);auto &p=m_presets[index(m_selected)];if(p.airbrush==data)return true;p.airbrush=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
QVariantMap BrushLibrary::speedDynamics() const{const auto &data=m_presets[index(m_selected)].speed;return data.isEmpty()?defaultSpeedDynamics():data;}
bool BrushLibrary::setSpeedDynamics(QVariantMap data){if(!validSpeedDynamics(QJsonObject::fromVariantMap(data)))return false;data=canonicalSpeedDynamics(data);auto &p=m_presets[index(m_selected)];if(p.speed==data)return true;p.speed=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setSpeedDynamicsEnabled(bool enabled){auto data=speedDynamics();data["enabled"]=enabled;return setSpeedDynamics(data);}
bool BrushLibrary::setSpeedDynamicsMaxSpeed(qreal maximum){auto data=speedDynamics();data["maxSpeed"]=maximum;return setSpeedDynamics(data);}
bool BrushLibrary::speedDynamicsEnabled() const{return speedDynamics().value("enabled").toBool();}
qreal BrushLibrary::speedDynamicsMaxSpeed() const{return speedDynamics().value("maxSpeed").toDouble();}
QVariantMap BrushLibrary::speedSize() const{return speedDynamics().value("size").toMap();}
QVariantMap BrushLibrary::speedFlow() const{return speedDynamics().value("flow").toMap();}
QVariantMap BrushLibrary::speedOpacity() const{return speedDynamics().value("opacity").toMap();}
QVariantList BrushLibrary::speedSizeCurve() const{return speedSize().value("curve").toList();}
qreal BrushLibrary::speedSizeMinimum() const{return speedSize().value("minimum").toDouble();}
QVariantList BrushLibrary::speedFlowCurve() const{return speedFlow().value("curve").toList();}
qreal BrushLibrary::speedFlowMinimum() const{return speedFlow().value("minimum").toDouble();}
QVariantList BrushLibrary::speedOpacityCurve() const{return speedOpacity().value("curve").toList();}
qreal BrushLibrary::speedOpacityMinimum() const{return speedOpacity().value("minimum").toDouble();}
bool BrushLibrary::setSpeedControl(const QString &target,QVariantList points,qreal minimum){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=speedDynamics();auto control=data.value(target).toMap();control["curve"]=points;control["minimum"]=minimum;control["interpolation"]="smooth-pchip-v1";data[target]=control;return setSpeedDynamics(data);}
bool BrushLibrary::resetSpeedControl(const QString &target){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=speedDynamics();data[target]=defaultSpeedControl();return setSpeedDynamics(data);}
QVariantMap BrushLibrary::tangentDynamics() const{const auto &data=m_presets[index(m_selected)].tangent;return data.isEmpty()?defaultTangentDynamics():data;}
bool BrushLibrary::setTangentDynamics(QVariantMap data){if(!validTangentDynamics(QJsonObject::fromVariantMap(data)))return false;data=canonicalTangentDynamics(data);auto &p=m_presets[index(m_selected)];if(p.tangent==data)return true;p.tangent=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setTangentDynamicsEnabled(bool enabled){auto data=tangentDynamics();data["enabled"]=enabled;return setTangentDynamics(data);}
bool BrushLibrary::tangentDynamicsEnabled() const{return tangentDynamics().value("enabled").toBool();}
QVariantMap BrushLibrary::tangentControl(const QString &target) const{return tangentDynamics().value(target).toMap();}
QVariantList BrushLibrary::tangentSizeCurve() const{return tangentControl("size").value("curve").toList();}
qreal BrushLibrary::tangentSizeMinimum() const{return tangentControl("size").value("minimum").toDouble();}
QVariantList BrushLibrary::tangentFlowCurve() const{return tangentControl("flow").value("curve").toList();}
qreal BrushLibrary::tangentFlowMinimum() const{return tangentControl("flow").value("minimum").toDouble();}
QVariantList BrushLibrary::tangentOpacityCurve() const{return tangentControl("opacity").value("curve").toList();}
qreal BrushLibrary::tangentOpacityMinimum() const{return tangentControl("opacity").value("minimum").toDouble();}
bool BrushLibrary::setTangentControl(const QString &target,QVariantList points,qreal minimum){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=tangentDynamics();auto control=data.value(target).toMap();control["curve"]=points;control["minimum"]=minimum;control["interpolation"]="smooth-pchip-v1";data[target]=control;return setTangentDynamics(data);}
bool BrushLibrary::resetTangentControl(const QString &target){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=tangentDynamics();data[target]=defaultSpeedControl();return setTangentDynamics(data);}
QVariantMap BrushLibrary::distanceDynamics() const{const auto &data=m_presets[index(m_selected)].distance;return data.isEmpty()?defaultDistanceDynamics():data;}
bool BrushLibrary::setDistanceDynamics(QVariantMap data){if(!validDistanceDynamics(QJsonObject::fromVariantMap(data)))return false;data=canonicalDistanceDynamics(data);auto &p=m_presets[index(m_selected)];if(p.distance==data)return true;p.distance=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setDistanceDynamicsEnabled(bool enabled){auto data=distanceDynamics();data["enabled"]=enabled;return setDistanceDynamics(data);}
bool BrushLibrary::setDistanceDynamicsMaxDistance(qreal maximum){auto data=distanceDynamics();data["maxDistance"]=maximum;return setDistanceDynamics(data);}
bool BrushLibrary::distanceDynamicsEnabled() const{return distanceDynamics().value("enabled").toBool();}
qreal BrushLibrary::distanceDynamicsMaxDistance() const{return distanceDynamics().value("maxDistance").toDouble();}
QVariantMap BrushLibrary::distanceControl(const QString &target) const{return distanceDynamics().value(target).toMap();}
QVariantList BrushLibrary::distanceSizeCurve() const{return distanceControl("size").value("curve").toList();}
qreal BrushLibrary::distanceSizeMinimum() const{return distanceControl("size").value("minimum").toDouble();}
QVariantList BrushLibrary::distanceFlowCurve() const{return distanceControl("flow").value("curve").toList();}
qreal BrushLibrary::distanceFlowMinimum() const{return distanceControl("flow").value("minimum").toDouble();}
QVariantList BrushLibrary::distanceOpacityCurve() const{return distanceControl("opacity").value("curve").toList();}
qreal BrushLibrary::distanceOpacityMinimum() const{return distanceControl("opacity").value("minimum").toDouble();}
bool BrushLibrary::setDistanceControl(const QString &target,QVariantList points,qreal minimum){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=distanceDynamics();auto control=data.value(target).toMap();control["curve"]=points;control["minimum"]=minimum;control["interpolation"]="smooth-pchip-v1";data[target]=control;return setDistanceDynamics(data);}
bool BrushLibrary::resetDistanceControl(const QString &target){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=distanceDynamics();data[target]=defaultSpeedControl();return setDistanceDynamics(data);}
QVariantMap BrushLibrary::tiltDynamics() const{const auto &data=m_presets[index(m_selected)].tilt;return data.isEmpty()?defaultTiltDynamics():data;}
bool BrushLibrary::setTiltDynamics(QVariantMap data){if(!validTiltDynamics(QJsonObject::fromVariantMap(data)))return false;data=canonicalTiltDynamics(data);auto &p=m_presets[index(m_selected)];if(p.tilt==data)return true;p.tilt=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setTiltDynamicsEnabled(bool enabled){auto data=tiltDynamics();data["enabled"]=enabled;return setTiltDynamics(data);}
bool BrushLibrary::tiltDynamicsEnabled() const{return tiltDynamics().value("enabled").toBool();}
QVariantMap BrushLibrary::tiltControl(const QString &target) const{return tiltDynamics().value(target).toMap();}
QVariantList BrushLibrary::tiltSizeCurve() const{return tiltControl("size").value("curve").toList();}
qreal BrushLibrary::tiltSizeMinimum() const{return tiltControl("size").value("minimum").toDouble();}
QVariantList BrushLibrary::tiltFlowCurve() const{return tiltControl("flow").value("curve").toList();}
qreal BrushLibrary::tiltFlowMinimum() const{return tiltControl("flow").value("minimum").toDouble();}
QVariantList BrushLibrary::tiltOpacityCurve() const{return tiltControl("opacity").value("curve").toList();}
qreal BrushLibrary::tiltOpacityMinimum() const{return tiltControl("opacity").value("minimum").toDouble();}
bool BrushLibrary::setTiltControl(const QString &target,QVariantList points,qreal minimum){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=tiltDynamics();auto control=data.value(target).toMap();control["curve"]=points;control["minimum"]=minimum;control["interpolation"]="smooth-pchip-v1";data[target]=control;return setTiltDynamics(data);}
bool BrushLibrary::resetTiltControl(const QString &target){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=tiltDynamics();data[target]=defaultSpeedControl();return setTiltDynamics(data);}
QVariantMap BrushLibrary::rotationDynamics() const{const auto &data=m_presets[index(m_selected)].rotation;return data.isEmpty()?defaultRotationDynamics():data;}
bool BrushLibrary::setRotationDynamics(QVariantMap data){if(!validRotationDynamics(QJsonObject::fromVariantMap(data)))return false;data=canonicalRotationDynamics(data);auto &p=m_presets[index(m_selected)];if(p.rotation==data)return true;p.rotation=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setRotationDynamicsEnabled(bool enabled){auto data=rotationDynamics();data["enabled"]=enabled;return setRotationDynamics(data);}
bool BrushLibrary::rotationDynamicsEnabled() const{return rotationDynamics().value("enabled").toBool();}
QVariantMap BrushLibrary::rotationControl(const QString &target) const{return rotationDynamics().value(target).toMap();}
QVariantList BrushLibrary::rotationSizeCurve() const{return rotationControl("size").value("curve").toList();}
qreal BrushLibrary::rotationSizeMinimum() const{return rotationControl("size").value("minimum").toDouble();}
QVariantList BrushLibrary::rotationFlowCurve() const{return rotationControl("flow").value("curve").toList();}
qreal BrushLibrary::rotationFlowMinimum() const{return rotationControl("flow").value("minimum").toDouble();}
QVariantList BrushLibrary::rotationOpacityCurve() const{return rotationControl("opacity").value("curve").toList();}
qreal BrushLibrary::rotationOpacityMinimum() const{return rotationControl("opacity").value("minimum").toDouble();}
bool BrushLibrary::setRotationControl(const QString &target,QVariantList points,qreal minimum){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=rotationDynamics();auto control=data.value(target).toMap();control["curve"]=points;control["minimum"]=minimum;control["interpolation"]="smooth-pchip-v1";data[target]=control;return setRotationDynamics(data);}
bool BrushLibrary::resetRotationControl(const QString &target){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=rotationDynamics();data[target]=defaultSpeedControl();return setRotationDynamics(data);}
QVariantMap BrushLibrary::directionDynamics() const{const auto &data=m_presets[index(m_selected)].direction;return data.isEmpty()?defaultDirectionDynamics():data;}
bool BrushLibrary::setDirectionDynamics(QVariantMap data){if(!validDirectionDynamics(QJsonObject::fromVariantMap(data)))return false;data=canonicalDirectionDynamics(data);auto &p=m_presets[index(m_selected)];if(p.direction==data)return true;p.direction=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setDirectionDynamicsEnabled(bool enabled){auto data=directionDynamics();data["enabled"]=enabled;return setDirectionDynamics(data);}
bool BrushLibrary::directionDynamicsEnabled() const{return directionDynamics().value("enabled").toBool();}
QVariantMap BrushLibrary::fadeDynamics() const{const auto &data=m_presets[index(m_selected)].fade;return data.isEmpty()?defaultFadeDynamics():data;}
bool BrushLibrary::setFadeDynamics(QVariantMap data){if(!validFadeDynamics(QJsonObject::fromVariantMap(data)))return false;data=canonicalFadeDynamics(data);auto &p=m_presets[index(m_selected)];if(p.fade==data)return true;p.fade=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
bool BrushLibrary::setFadeDynamicsEnabled(bool enabled){auto data=fadeDynamics();data["enabled"]=enabled;return setFadeDynamics(data);}
bool BrushLibrary::setFadeSteps(int steps){auto data=fadeDynamics();data["steps"]=steps;return setFadeDynamics(data);}
bool BrushLibrary::fadeDynamicsEnabled() const{return fadeDynamics().value("enabled").toBool();}
int BrushLibrary::fadeSteps() const{return fadeDynamics().value("steps").toInt();}
QVariantList BrushLibrary::fadeSizeCurve() const{return fadeDynamics().value("size").toMap().value("curve").toList();}
qreal BrushLibrary::fadeSizeMinimum() const{return fadeDynamics().value("size").toMap().value("minimum").toDouble();}
QVariantList BrushLibrary::fadeFlowCurve() const{return fadeDynamics().value("flow").toMap().value("curve").toList();}
qreal BrushLibrary::fadeFlowMinimum() const{return fadeDynamics().value("flow").toMap().value("minimum").toDouble();}
QVariantList BrushLibrary::fadeOpacityCurve() const{return fadeDynamics().value("opacity").toMap().value("curve").toList();}
qreal BrushLibrary::fadeOpacityMinimum() const{return fadeDynamics().value("opacity").toMap().value("minimum").toDouble();}
bool BrushLibrary::setFadeControl(const QString &target,QVariantList points,qreal minimum){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=fadeDynamics();auto control=data.value(target).toMap();control["curve"]=points;control["minimum"]=minimum;control["interpolation"]="smooth-pchip-v1";data[target]=control;return setFadeDynamics(data);}
bool BrushLibrary::resetFadeControl(const QString &target){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=fadeDynamics();data[target]=defaultSpeedControl();return setFadeDynamics(data);}
QVariantMap BrushLibrary::dynamicsSourceMatrix() const{const auto &data=m_presets[index(m_selected)].matrix;return data.isEmpty()?defaultDynamicsSourceMatrix():data;}
QString BrushLibrary::dynamicsSizeSource() const{return dynamicsSourceMatrix().value("size").toString();}
QString BrushLibrary::dynamicsFlowSource() const{return dynamicsSourceMatrix().value("flow").toString();}
QString BrushLibrary::dynamicsOpacitySource() const{return dynamicsSourceMatrix().value("opacity").toString();}
bool BrushLibrary::setDynamicsSource(const QString &target,const QString &source){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=dynamicsSourceMatrix();data[target]=source;if(!validDynamicsSourceMatrix(QJsonObject::fromVariantMap(data)))return false;data=canonicalDynamicsSourceMatrix(data);auto &p=m_presets[index(m_selected)];if(p.matrix==data)return true;p.matrix=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
QVariantMap BrushLibrary::directionControl(const QString &target) const{return directionDynamics().value(target).toMap();}
QVariantList BrushLibrary::directionSizeCurve() const{return directionControl("size").value("curve").toList();}
qreal BrushLibrary::directionSizeMinimum() const{return directionControl("size").value("minimum").toDouble();}
QVariantList BrushLibrary::directionFlowCurve() const{return directionControl("flow").value("curve").toList();}
qreal BrushLibrary::directionFlowMinimum() const{return directionControl("flow").value("minimum").toDouble();}
QVariantList BrushLibrary::directionOpacityCurve() const{return directionControl("opacity").value("curve").toList();}
qreal BrushLibrary::directionOpacityMinimum() const{return directionControl("opacity").value("minimum").toDouble();}
bool BrushLibrary::setDirectionControl(const QString &target,QVariantList points,qreal minimum){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=directionDynamics();auto control=data.value(target).toMap();control["curve"]=points;control["minimum"]=minimum;control["interpolation"]="smooth-pchip-v1";data[target]=control;return setDirectionDynamics(data);}
bool BrushLibrary::resetDirectionControl(const QString &target){if(target!="size" && target!="flow" && target!="opacity")return false;auto data=directionDynamics();data[target]=defaultSpeedControl();return setDirectionDynamics(data);}
QVariantMap BrushLibrary::wetMix() const{const auto &data=m_presets[index(m_selected)].wet;return data.isEmpty()?defaultWetMix():data;}
bool BrushLibrary::setWetMix(QVariantMap data){data.remove("dualColor");if(!validWetMix(QJsonObject::fromVariantMap(data)))return false;auto &p=m_presets[index(m_selected)];data=canonicalWetMix(data);if(p.wet==data)return true;p.wet=data;p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;}
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
    for(auto &color:m_recentColors)color=QColor(color).name(QColor::HexArgb);
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
QVariantMap BrushLibrary::tipSettings() const{return {{"hardness",hardness()},{"roundness",roundness()},{"angle",angle()},{"flow",flow()},{"pressureSize",pressureSize()},{"pressureFlow",pressureFlow()},{"maskResource",maskResource()},{"pressureDynamics",m_presets[index(m_selected)].pressure},{"tipDynamics",m_presets[index(m_selected)].device},{"scatterDynamics",m_presets[index(m_selected)].scatter},{"shapeJitter",m_presets[index(m_selected)].shape},{"transferJitter",m_presets[index(m_selected)].transfer},{"colorDynamics",m_presets[index(m_selected)].color},{"texture",m_presets[index(m_selected)].texture},{"dualBrush",m_presets[index(m_selected)].dual},{"smoothing",m_presets[index(m_selected)].smoothing},{"wetMix",m_presets[index(m_selected)].wet},{"strokeTaper",m_presets[index(m_selected)].taper},{"airbrush",m_presets[index(m_selected)].airbrush},{"speedDynamics",m_presets[index(m_selected)].speed},{"tangentDynamics",m_presets[index(m_selected)].tangent},{"distanceDynamics",m_presets[index(m_selected)].distance},{"tiltDynamics",m_presets[index(m_selected)].tilt},{"rotationDynamics",m_presets[index(m_selected)].rotation},{"directionDynamics",m_presets[index(m_selected)].direction},{"presetRadius",radius()},{"writeBlend",0}};}
QVariantList BrushLibrary::presets() const {
    QVariantList rows;for(const auto &p:m_presets)rows.append(QVariantMap{{"id",p.id},{"name",p.name},{"radius",p.radius},{"opacity",p.opacity},{"spacing",p.spacing},{"builtin",p.builtin},{"preview",p.preview}});return rows;
}
void BrushLibrary::setWriteBlend(int mode) {
    if(mode<0 || mode>26 || mode==m_writeBlend)return;
    m_writeBlend=mode;
    emit settingsChanged();emit persistRequested(snapshot());
}
void BrushLibrary::changed(){emit settingsChanged();emit presetsChanged();emit selectedPreviewChanged();emit persistRequested(snapshot());}
void BrushLibrary::update(int field,qreal value) {
    if(!std::isfinite(value))return;
    auto &p=m_presets[index(m_selected)];
    qreal *target=nullptr;
    switch(field){case 0:target=&p.radius;value=std::clamp(value,.5,1000.);break;
        case 1:target=&p.opacity;value=std::clamp(value,0.,1.);break;
        case 2:target=&p.spacing;value=std::clamp(value,.02,1.);break;
        case 3:target=&p.hardness;value=std::clamp(value,0.,1.);break;
        case 4:target=&p.roundness;value=std::clamp(value,.01,1.);break;
        case 5:target=&p.angle;value=std::clamp(value,-180.,180.);break;
        case 6:target=&p.flow;value=std::clamp(value,0.,1.);break;
        default:return;}
    if(*target==value)return;
    *target=value;
    // Standard previews use a fixed size, flow and opacity. Keep in-flight jobs valid.
    if(field!=0 && field!=1 && field!=6){p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;}
    changed();
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
void BrushLibrary::setCursorWidth(int value){value=std::clamp(value,0,3);if(m_cursorWidth==value)return;m_cursorWidth=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorBrightness(int value){auto color=QColor(m_cursorColor);color.setHsvF(color.hsvHueF(),color.hsvSaturationF(),std::clamp(value,1,100)/100.);setCursorColor(color.name());}
void BrushLibrary::setCursorImage(bool value){if(m_cursorImage==value)return;m_cursorImage=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorImageOpacity(int value){value=std::clamp(value,1,100);if(m_cursorImageOpacity==value)return;m_cursorImageOpacity=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorComposite(bool value){if(m_cursorComposite==value)return;m_cursorComposite=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorSimple(bool value){if(m_cursorSimple==value)return;m_cursorSimple=value;cursorPreferenceChanged();}
void BrushLibrary::setCursorColor(const QString &value){const QColor color(value);if(!color.isValid() || color.alpha()!=255 || color.name()==m_cursorColor)return;m_cursorColor=color.name();cursorPreferenceChanged();}
void BrushLibrary::setCursorSecondaryColor(const QString &value){const QColor color(value);if(!color.isValid() || color.alpha()!=255 || color.name()==m_cursorSecondaryColor)return;m_cursorSecondaryColor=color.name();cursorPreferenceChanged();}
bool BrushLibrary::select(const QString &id){
    if(index(id)<0)return false;
    const QJsonArray selection{QJsonObject{{"kind","preset"},{"id",id}}};
    const bool selectionWasChanged=selection!=m_selectedItems;m_selectedItems=selection;
    if(selectionWasChanged)emit selectionChanged();
    if(id!=m_selected){m_selected=id;changed();}
    return true;
}
QVariantList BrushLibrary::selectedBrushItems() const {return m_selectedItems.toVariantList();}
bool BrushLibrary::multiBrushSelection() const {return m_selectedItems.size()>1;}
bool BrushLibrary::brushSelectionBlocksPainting() const {
    return m_selectedItems.size()>1 || (m_selectedItems.size()==1 && m_selectedItems.first().toObject().value("kind").toString()!="preset");
}
bool BrushLibrary::selectBrushItems(const QVariantList &items,const QString &activePreset) {
    QJsonArray next;QSet<QString> seen;
    for(const auto &value:items) {
        const auto item=value.toMap();const auto kind=item.value("kind").toString(),id=item.value("id").toString();
        if((kind!="preset" && kind!="group") || id.isEmpty() || seen.contains(kind+":"+id))continue;
        if(kind=="preset") {if(index(id)<0)continue;}
        else {
            bool found=false;for(const auto &group:m_organization.value("groups").toArray())if(group.toObject().value("id").toString()==id){found=true;break;}
            if(!found)continue;
        }
        seen.insert(kind+":"+id);next.append(QJsonObject{{"kind",kind},{"id",id}});
    }
    QString preset=activePreset;
    if(index(preset)<0) {
        preset.clear();
        if(next.size()==1 && next.first().toObject().value("kind").toString()=="preset")preset=next.first().toObject().value("id").toString();
    }
    const bool selectionWasChanged=next!=m_selectedItems;m_selectedItems=next;
    const bool activeWasChanged=!preset.isEmpty() && preset!=m_selected;
    if(activeWasChanged)m_selected=preset;
    if(selectionWasChanged)emit selectionChanged();
    if(activeWasChanged)changed();
    return true;
}
void BrushLibrary::clearBrushSelection() {
    if(m_selectedItems.isEmpty())return;
    m_selectedItems=QJsonArray{};emit selectionChanged();
}
bool BrushLibrary::brushSelectionHasSources() const {
    QSet<QString> groups,presets;
    QHash<QString,QJsonObject> byId;
    for(const auto &value:m_organization.value("groups").toArray()) {const auto group=value.toObject();byId.insert(group.value("id").toString(),group);}
    std::function<void(const QString &)> includeGroup=[&](const QString &id) {
        if(groups.contains(id))return;
        groups.insert(id);
        for(const auto &value:byId.value(id).value("items").toArray()) {
            const auto item=value.toObject();const auto child=item.value("id").toString();
            if(item.value("kind").toString()=="group")includeGroup(child);else presets.insert(child);
        }
    };
    for(const auto &value:m_selectedItems) {
        const auto item=value.toObject();const auto id=item.value("id").toString();
        if(item.value("kind").toString()=="group")includeGroup(id);else presets.insert(id);
    }
    for(const auto &id:presets) {const int i=index(id);if(i>=0 && !m_presets[i].sourceResource.isEmpty())return true;}
    return false;
}
bool BrushLibrary::resetPreset() {
    const int i=index(m_selected);if(i<0 || !m_resetPresets.contains(m_selected))return false;
    auto reset=m_resetPresets.value(m_selected);reset.id=m_selected;reset.name=m_presets[i].name;reset.builtin=m_presets[i].builtin;
    reset.preview.clear();reset.error.clear();reset.token=++m_nextToken;reset.requested=0;
    m_presets[i]=reset;changed();return true;
}
bool BrushLibrary::restoreSourceBaseline(const QVariantMap &info) {
    const int i=index(m_selected);
    if(i<0 || info.value("baseline").userType()!=QMetaType::QVariantMap)return false;
    const auto base=info.value("baseline").toMap();
    const auto radius=base.value("diameter").toDouble()/2;
    const auto opacity=base.value("dabOpacity").toDouble();
    const auto spacing=base.value("spacing").toDouble()*2;
    const auto hardness=base.value("hardness").toDouble();
    const auto roundness=base.value("roundness").toDouble();
    const auto angle=base.value("angle").toDouble();
    const auto flow=base.value("flow").toDouble();
    if(!std::isfinite(radius) || radius<.5 || radius>1000 || !std::isfinite(opacity) || opacity<0 || opacity>1 || !std::isfinite(spacing) || spacing<.02 || spacing>1 || !std::isfinite(hardness) || hardness<0 || hardness>1 || !std::isfinite(roundness) || roundness<.01 || roundness>1 || !std::isfinite(angle) || angle<-180 || angle>180 || !std::isfinite(flow) || flow<0 || flow>1)return false;
    QVariantMap texture;
    if(!info.value("texture").isNull()) {
        if(info.value("texture").userType()!=QMetaType::QVariantMap)return false;
        texture=canonicalTexture(info.value("texture").toMap());
        if(!validTexture(QJsonObject::fromVariantMap(texture)))return false;
    }
    QString mask;
    if(!info.value("tipResource").isNull()) {
        static const QRegularExpression identity("^[0-9a-f]{64}$");
        mask=info.value("tipResource").toString();
        if(!identity.match(mask).hasMatch())return false;
    }
    auto &p=m_presets[i];
    p.radius=radius;p.opacity=opacity;p.spacing=spacing;p.hardness=hardness;p.roundness=roundness;p.angle=angle;p.flow=flow;
    p.pressureSize=base.value("pressureSize").toBool();p.pressureFlow=base.value("pressureFlow").toBool();
    p.pressure=base.value("pressureDynamics").toMap();p.device=base.value("tipDynamics").toMap();p.scatter=base.value("scatterDynamics").toMap();p.shape=base.value("shapeJitter").toMap();p.color=base.value("colorDynamics").toMap();
    p.maskResource=mask;p.texture=texture;
    p.preview.clear();p.error.clear();p.token=++m_nextToken;p.requested=0;changed();return true;
}
bool BrushLibrary::applyOrganization(const QJsonObject &candidate) {
    QStringList ids,order;ids.reserve(m_presets.size());
    for(const auto &preset:m_presets)ids.append(preset.id);
    QJsonObject normalized;
    if(!BrushOrganization::normalize(candidate,ids,normalized,order,false))return false;
    bool orderChanged=order!=ids;
    if(normalized==m_organization && !orderChanged)return true;
    QList<Preset> sorted;sorted.reserve(m_presets.size());
    for(const auto &id:order){const int i=index(id);if(i<0)return false;sorted.append(m_presets[i]);}
    m_presets=std::move(sorted);m_organization=normalized;changed();return true;
}
QString BrushLibrary::createGroup(const QString &name,const QString &parentId) {
    const QString clean=name.trimmed();if(clean.isEmpty() || clean.size()>80 || clean.contains(QChar(0)))return {};
    QJsonObject organization=m_organization;
    if(organization.isEmpty()) {
        QJsonArray roots;for(const auto &preset:m_presets)roots.append(BrushOrganization::presetItem(preset.id));
        organization={{"model","ordered-tree-v1"},{"roots",roots},{"groups",QJsonArray{}}};
    }
    QJsonArray siblings;
    if(parentId.isEmpty())siblings=organization.value("roots").toArray();
    else {
        auto groups=organization.value("groups").toArray();bool found=false;
        for(qsizetype i=0;i<groups.size();++i){auto group=groups[i].toObject();if(group.value("id").toString()==parentId){siblings=group.value("items").toArray();found=true;break;}}
        if(!found)return {};
    }
    const QString id=QUuid::createUuid().toString(QUuid::WithoutBraces);
    siblings.append(QJsonObject{{"kind","group"},{"id",id}});
    if(parentId.isEmpty())organization["roots"]=siblings;
    else {auto groups=organization.value("groups").toArray();for(qsizetype i=0;i<groups.size();++i){auto group=groups[i].toObject();if(group.value("id").toString()==parentId){group["items"]=siblings;groups[i]=group;break;}}organization["groups"]=groups;}
    auto groups=organization.value("groups").toArray();groups.append(QJsonObject{{"id",id},{"name",clean},{"items",QJsonArray{}}});organization["groups"]=groups;
    return applyOrganization(organization)?id:QString();
}
bool BrushLibrary::renameGroup(const QString &id,const QString &name) {
    const auto clean=name.trimmed();if(clean.isEmpty() || clean.size()>80 || clean.contains(QChar(0)))return false;
    auto groups=m_organization.value("groups").toArray();bool found=false;
    for(qsizetype i=0;i<groups.size();++i){auto group=groups[i].toObject();if(group.value("id").toString()==id){group["name"]=clean;groups[i]=group;found=true;break;}}
    if(!found)return false;auto candidate=m_organization;candidate["groups"]=groups;return applyOrganization(candidate);
}
namespace {
bool groupContainsGroup(const QJsonObject &organization,const QString &ancestor,const QString &target) {
    if(ancestor==target)return true;
    const auto groups=organization.value("groups").toArray();
    for(const auto &value:groups) {
        const auto group=value.toObject();
        if(group.value("id").toString()!=ancestor)continue;
        for(const auto &child:group.value("items").toArray())
            if(child.toObject().value("kind").toString()=="group" && groupContainsGroup(organization,child.toObject().value("id").toString(),target))return true;
        return false;
    }
    return false;
}
bool groupContainsItem(const QJsonObject &organization,const QString &group,const QString &kind,const QString &id) {
    for(const auto &value:organization.value("groups").toArray()) {
        const auto entry=value.toObject();
        if(entry.value("id").toString()!=group)continue;
        for(const auto &childValue:entry.value("items").toArray()) {
            const auto child=childValue.toObject();
            if(child.value("kind").toString()==kind && child.value("id").toString()==id)return true;
            if(child.value("kind").toString()=="group" && groupContainsItem(organization,child.value("id").toString(),kind,id))return true;
        }
        return false;
    }
    return false;
}
QString findItemParent(const QJsonObject &organization,const QJsonObject &item,int &index) {
    const auto roots=organization.value("roots").toArray();
    for(qsizetype i=0;i<roots.size();++i)if(roots[i]==item){index=int(i);return {};}
    for(const auto &value:organization.value("groups").toArray()) {
        const auto group=value.toObject();const auto items=group.value("items").toArray();
        for(qsizetype i=0;i<items.size();++i)if(items[i]==item){index=int(i);return group.value("id").toString();}
    }
    return {};
}
void removeOrganizationItem(QJsonObject &organization,const QJsonObject &item) {
    auto roots=organization.value("roots").toArray();for(qsizetype i=roots.size();i>0;--i)if(roots[i-1]==item)roots.removeAt(i-1);organization["roots"]=roots;
    auto groups=organization.value("groups").toArray();
    for(qsizetype i=0;i<groups.size();++i){auto group=groups[i].toObject();auto items=group.value("items").toArray();for(qsizetype j=items.size();j>0;--j)if(items[j-1]==item)items.removeAt(j-1);group["items"]=items;groups[i]=group;}
    organization["groups"]=groups;
}
}
bool BrushLibrary::movePresetToGroup(const QString &presetId,const QString &parentId) {
    return movePresetToGroupAt(presetId,parentId,-1);
}
bool BrushLibrary::movePresetToGroupAt(const QString &presetId,const QString &parentId,int index) {
    if(this->index(presetId)<0)return false;
    auto candidate=m_organization;
    if(candidate.isEmpty()) {
        QJsonArray roots;for(const auto &preset:m_presets)roots.append(BrushOrganization::presetItem(preset.id));
        candidate=QJsonObject{{"model","ordered-tree-v1"},{"roots",roots},{"groups",QJsonArray{}}};
    }
    const QJsonObject item=BrushOrganization::presetItem(presetId);
    int sourceIndex=-1;const QString sourceParent=findItemParent(candidate,item,sourceIndex);
    removeOrganizationItem(candidate,item);
    bool found=false;QJsonArray destination;
    if(parentId.isEmpty()){destination=candidate.value("roots").toArray();found=true;}
    else {
        auto groups=candidate.value("groups").toArray();
        for(qsizetype i=0;i<groups.size();++i){auto group=groups[i].toObject();if(group.value("id").toString()==parentId){destination=group.value("items").toArray();found=true;group["items"]=destination;groups[i]=group;break;}}
        candidate["groups"]=groups;
    }
    if(!found)return false;
    if(sourceParent==parentId && sourceIndex>=0 && sourceIndex<index)--index;
    const int at=index<0?int(destination.size()):std::clamp(index,0,int(destination.size()));
    destination.insert(at,item);
    if(parentId.isEmpty())candidate["roots"]=destination;
    else {auto groups=candidate.value("groups").toArray();for(qsizetype i=0;i<groups.size();++i){auto group=groups[i].toObject();if(group.value("id").toString()==parentId){group["items"]=destination;groups[i]=group;break;}}candidate["groups"]=groups;}
    return applyOrganization(candidate);
}
bool BrushLibrary::moveGroupToGroup(const QString &groupId,const QString &parentId) {
    return moveGroupToGroupAt(groupId,parentId,-1);
}
bool BrushLibrary::moveGroupToGroupAt(const QString &groupId,const QString &parentId,int index) {
    if(m_organization.isEmpty() || groupId==parentId || groupContainsGroup(m_organization,groupId,parentId))return false;
    const auto groups=m_organization.value("groups").toArray();bool exists=false;
    for(const auto &value:groups)if(value.toObject().value("id").toString()==groupId){exists=true;break;}
    if(!exists)return false;
    auto candidate=m_organization;const QJsonObject item{{"kind","group"},{"id",groupId}};
    int sourceIndex=-1;const QString sourceParent=findItemParent(candidate,item,sourceIndex);
    removeOrganizationItem(candidate,item);
    bool found=false;QJsonArray destination;
    if(parentId.isEmpty()){destination=candidate.value("roots").toArray();found=true;}
    else {auto all=candidate.value("groups").toArray();for(qsizetype i=0;i<all.size();++i){auto group=all[i].toObject();if(group.value("id").toString()==parentId){destination=group.value("items").toArray();found=true;group["items"]=destination;all[i]=group;break;}}candidate["groups"]=all;}
    if(!found)return false;
    if(sourceParent==parentId && sourceIndex>=0 && sourceIndex<index)--index;
    const int at=index<0?int(destination.size()):std::clamp(index,0,int(destination.size()));
    destination.insert(at,item);
    if(parentId.isEmpty())candidate["roots"]=destination;
    else {auto all=candidate.value("groups").toArray();for(qsizetype i=0;i<all.size();++i){auto group=all[i].toObject();if(group.value("id").toString()==parentId){group["items"]=destination;all[i]=group;break;}}candidate["groups"]=all;}
    return applyOrganization(candidate);
}
bool BrushLibrary::groupSelectedItems(const QString &name) {
    const auto clean=name.trimmed();
    if(clean.isEmpty() || clean.size()>80 || clean.contains(QChar(0)) || m_selectedItems.isEmpty())return false;
    QJsonObject organization=m_organization;
    if(organization.isEmpty()) {
        QJsonArray roots;for(const auto &preset:m_presets)roots.append(BrushOrganization::presetItem(preset.id));
        organization=QJsonObject{{"model","ordered-tree-v1"},{"roots",roots},{"groups",QJsonArray{}}};
    }
    QSet<QString> selectedGroups;
    for(const auto &value:m_selectedItems) {
        const auto item=value.toObject();
        if(item.value("kind").toString()=="group")selectedGroups.insert(item.value("id").toString());
    }
    QJsonArray moved;QSet<QString> movedKeys;
    for(const auto &value:m_selectedItems) {
        const auto item=value.toObject();const auto kind=item.value("kind").toString(),id=item.value("id").toString(),key=kind+":"+id;
        if((kind!="preset" && kind!="group") || id.isEmpty() || movedKeys.contains(key))continue;
        bool nested=false;
        if(kind=="group") {
            for(const auto &ancestor:m_selectedItems) {
                const auto candidate=ancestor.toObject();
                if(candidate.value("kind").toString()=="group" && candidate.value("id").toString()!=id && groupContainsGroup(m_organization,candidate.value("id").toString(),id)){nested=true;break;}
            }
        } else {
            for(const auto &ancestor:selectedGroups)if(groupContainsItem(m_organization,ancestor,"preset",id)){nested=true;break;}
        }
        if(nested)continue;
        movedKeys.insert(key);moved.append(item);
    }
    if(moved.isEmpty())return false;
    auto candidate=organization;
    for(const auto &value:moved)removeOrganizationItem(candidate,value.toObject());
    const auto id=QUuid::createUuid().toString(QUuid::WithoutBraces);
    auto roots=candidate.value("roots").toArray();roots.append(QJsonObject{{"kind","group"},{"id",id}});candidate["roots"]=roots;
    auto groups=candidate.value("groups").toArray();groups.append(QJsonObject{{"id",id},{"name",clean},{"items",moved}});candidate["groups"]=groups;
    if(!applyOrganization(candidate))return false;
    m_selectedItems=QJsonArray{QJsonObject{{"kind","group"},{"id",id}}};emit selectionChanged();
    return true;
}
bool BrushLibrary::ungroup(const QString &id) {return removeGroup(id,false);}
bool BrushLibrary::removeGroup(const QString &id,bool cascade) {
    if(m_organization.isEmpty())return false;
    const auto groups=m_organization.value("groups").toArray();
    QJsonObject target;bool targetFound=false;
    for(const auto &entry:groups)if(entry.toObject().value("id").toString()==id){target=entry.toObject();targetFound=true;break;}
    if(!targetFound)return false;
    QJsonObject candidate=m_organization;
    auto roots=candidate.value("roots").toArray();
    auto candidateGroups=groups;
    QString parentId;qsizetype position=-1;
    for(qsizetype i=0;i<roots.size();++i){const auto item=roots[i].toObject();if(item.value("kind").toString()=="group" && item.value("id").toString()==id){position=i;break;}}
    if(position>=0)roots.removeAt(position);
    else for(qsizetype i=0;i<candidateGroups.size();++i){auto parent=candidateGroups[i].toObject();auto items=parent.value("items").toArray();for(qsizetype j=0;j<items.size();++j){const auto item=items[j].toObject();if(item.value("kind").toString()=="group" && item.value("id").toString()==id){position=j;parentId=parent.value("id").toString();items.removeAt(j);parent["items"]=items;candidateGroups[i]=parent;break;}}if(position>=0)break;}
    if(position<0)return false;
    QSet<QString> deletingGroups{id},deletingPresets;
    if(cascade){
        QList<QString> pending{id};
        while(!pending.isEmpty()){
            const auto current=pending.takeLast();
            for(const auto &child:([&](){for(const auto &entry:groups)if(entry.toObject().value("id").toString()==current)return entry.toObject().value("items").toArray();return QJsonArray{};})()){
                const auto item=child.toObject();const auto childId=item.value("id").toString();
                if(item.value("kind").toString()=="group"){if(!deletingGroups.contains(childId)){deletingGroups.insert(childId);pending.append(childId);}}
                else deletingPresets.insert(childId);
            }
        }
        for(const auto &presetId:deletingPresets){const int i=index(presetId);if(i<0 || m_presets[i].builtin)return false;}
    } else {
        const auto children=target.value("items").toArray();
        if(parentId.isEmpty()){for(qsizetype i=0;i<children.size();++i)roots.insert(position+i,children[i]);}
        else for(qsizetype i=0;i<candidateGroups.size();++i){auto parent=candidateGroups[i].toObject();if(parent.value("id").toString()==parentId){auto items=parent.value("items").toArray();for(qsizetype j=0;j<children.size();++j)items.insert(position+j,children[j]);parent["items"]=items;candidateGroups[i]=parent;break;}}
    }

    if(cascade){
        for(qsizetype i=roots.size();i>0;--i){const auto item=roots[i-1].toObject();if(item.value("kind").toString()=="preset" && deletingPresets.contains(item.value("id").toString()))roots.removeAt(i-1);}
        for(qsizetype i=0;i<candidateGroups.size();++i){auto group=candidateGroups[i].toObject();if(deletingGroups.contains(group.value("id").toString()))continue;auto items=group.value("items").toArray();for(qsizetype j=items.size();j>0;--j){const auto item=items[j-1].toObject();if(item.value("kind").toString()=="preset" && deletingPresets.contains(item.value("id").toString()))items.removeAt(j-1);}group["items"]=items;candidateGroups[i]=group;}
    }
    QJsonArray finalGroups;for(const auto &entry:candidateGroups)if(!deletingGroups.contains(entry.toObject().value("id").toString()))finalGroups.append(entry);
    candidate["roots"]=roots;candidate["groups"]=finalGroups;

    QList<Preset> nextPresets;auto nextReset=m_resetPresets;QStringList ids,order;
    for(const auto &preset:m_presets)if(!deletingPresets.contains(preset.id)){nextPresets.append(preset);ids.append(preset.id);}else nextReset.remove(preset.id);
    QJsonObject normalized;if(!BrushOrganization::normalize(candidate,ids,normalized,order,false))return false;
    QList<Preset> sorted;sorted.reserve(nextPresets.size());
    for(const auto &presetId:order){auto it=std::find_if(nextPresets.cbegin(),nextPresets.cend(),[&](const Preset &preset){return preset.id==presetId;});if(it==nextPresets.cend())return false;sorted.append(*it);}
    m_presets=std::move(sorted);m_resetPresets=std::move(nextReset);m_organization=normalized;
    if(deletingPresets.contains(m_selected))m_selected="round-pressure";
    m_selectedItems=QJsonArray{QJsonObject{{"kind","preset"},{"id",m_selected}}};emit selectionChanged();
    changed();return true;
}
QString BrushLibrary::saveCopy(const QString &name){
    if(m_presets.size()>=1024 || name.trimmed().isEmpty() || name.size()>80)return {};
    auto p=m_presets[index(m_selected)];p.id=QUuid::createUuid().toString(QUuid::WithoutBraces);p.name=name.trimmed();p.builtin=false;
    QJsonObject organization;QStringList ids,order;for(const auto &preset:m_presets)ids.append(preset.id);ids.append(p.id);
    if(!m_organization.isEmpty() && !BrushOrganization::normalize(BrushOrganization::addPreset(m_organization,p.id,m_selected),ids,organization,order,false))return {};
    p.token=++m_nextToken;p.requested=0;p.error.clear();m_presets.append(p);m_resetPresets.insert(p.id,p);
    if(!organization.isEmpty()){QList<Preset> sorted;for(const auto &id:order)sorted.append(m_presets[index(id)]);m_presets=sorted;}
    m_organization=organization;m_selected=p.id;m_selectedItems=QJsonArray{QJsonObject{{"kind","preset"},{"id",p.id}}};emit selectionChanged();changed();return p.id;
}
QString BrushLibrary::createOutlinePreset(bool fixedWidth) {
    if(m_presets.size()>=1024)return {};
    Preset p;p.id=QUuid::createUuid().toString(QUuid::WithoutBraces);p.name=fixedWidth?QStringLiteral("描边圆笔 · 固定边宽"):QStringLiteral("描边圆笔 · 跟随压感");
    p.spacing=.1;p.pressureFlow=false;
    auto dual=defaultDual();dual["enabled"]=true;dual["model"]="paired-dab-v1";dual["spacing"]=QVariant{};dual["pressureFlow"]=false;
    dual["sizing"]=QVariantMap{{"model","relative-size-v1"},{"mode","relative"},{"diameterOffset",8.},{"widthRule",fixedWidth?"fixed-width":"follow-pressure"}};
    if(!validDual(QJsonObject::fromVariantMap(dual)))return {};
    p.dual=canonicalDual(dual);p.token=++m_nextToken;m_organization=BrushOrganization::addPreset(m_organization,p.id);
    m_presets.append(p);m_resetPresets.insert(p.id,p);m_selected=p.id;m_selectedItems=QJsonArray{QJsonObject{{"kind","preset"},{"id",p.id}}};emit selectionChanged();changed();return p.id;
}
bool BrushLibrary::rename(const QString &id,const QString &name){const int i=index(id);if(i<0 || name.trimmed().isEmpty() || name.size()>80)return false;m_presets[i].name=name.trimmed();changed();return true;}
bool BrushLibrary::remove(const QString &id){
    const int i=index(id);if(i<0 || m_presets[i].builtin)return false;
    m_organization=BrushOrganization::removePreset(m_organization,id);m_presets.removeAt(i);m_resetPresets.remove(id);if(m_selected==id)m_selected="round-pressure";
    QJsonArray selection;for(const auto &value:m_selectedItems){const auto item=value.toObject();if(!(item.value("kind").toString()=="preset" && item.value("id").toString()==id))selection.append(item);}
    const bool selectionWasChanged=selection!=m_selectedItems;m_selectedItems=selection;
    if(selectionWasChanged)emit selectionChanged();changed();return true;
}
void BrushLibrary::setActiveColors(const QColor &foreground,const QColor &background){
    if(!foreground.isValid() || !background.isValid() || (foreground==m_foregroundColor && background==m_backgroundColor))return;
    m_foregroundColor=foreground;m_backgroundColor=background;
    emit activeColorsChanged();emit persistRequested(snapshot());
}
void BrushLibrary::requestPreview(const QString &id){const int i=index(id);if(i<0)return;auto &p=m_presets[i];if(!p.preview.isEmpty() || !p.error.isEmpty() || p.requested==p.token)return;p.requested=p.token;QVariantMap tip={{"hardness",p.hardness},{"roundness",p.roundness},{"angle",p.angle},{"flow",1.},{"pressureSize",p.pressureSize},{"pressureFlow",p.pressureFlow},{"maskResource",p.maskResource},{"pressureDynamics",p.pressure},{"tipDynamics",p.device},{"scatterDynamics",p.scatter},{"shapeJitter",p.shape},{"transferJitter",p.transfer},{"colorDynamics",p.color},{"texture",p.texture},{"dualBrush",p.dual},{"smoothing",p.smoothing},{"wetMix",p.wet},{"strokeTaper",p.taper},{"airbrush",p.airbrush},{"presetRadius",24.},{"writeBlend",0}};if(!p.speed.isEmpty())tip["speedDynamics"]=p.speed;if(!p.tangent.isEmpty())tip["tangentDynamics"]=p.tangent;if(!p.distance.isEmpty())tip["distanceDynamics"]=p.distance;if(!p.tilt.isEmpty())tip["tiltDynamics"]=p.tilt;if(!p.rotation.isEmpty())tip["rotationDynamics"]=p.rotation;if(!p.direction.isEmpty())tip["directionDynamics"]=p.direction;if(!p.matrix.isEmpty())tip["sourceMatrix"]=p.matrix;if(!p.fade.isEmpty())tip["fadeDynamics"]=p.fade;emit previewRequested(p.id,p.token,24.,1.,p.spacing,tip);}
void BrushLibrary::retryPreview(){auto &p=m_presets[index(m_selected)];p.preview.clear();p.error.clear();p.requested=0;p.token=++m_nextToken;requestPreview(m_selected);emit previewChanged(p.id,{});emit selectedPreviewChanged();}
void BrushLibrary::acceptPreview(const QString &id,quint64 token,const QString &image){const int i=index(id);if(i<0 || m_presets[i].token!=token)return;m_presets[i].requested=0;m_presets[i].preview=image;m_presets[i].error=image.isEmpty()?QStringLiteral("预览暂不可用"):QString();emit previewChanged(id,image);if(id==m_selected)emit selectedPreviewChanged();}
int BrushLibrary::textureStateVersion(const QVariantMap &texture) {
    if(texture.isEmpty())return 0;
    int version=11;
    if(texture.contains("brightness") || texture.contains("contrast") || texture.contains("minimumDepth") || texture.contains("depthJitter") || texture.contains("depthPressure"))version=std::max(version,21);
    if(advancedTextureModes().contains(texture.value("mode").toString()))version=std::max(version,30);
    if(latestTextureModes().contains(texture.value("mode").toString()))version=std::max(version,31);
    if(dissolveTextureModes().contains(texture.value("mode").toString()))version=std::max(version,33);
    return version;
}
QJsonObject BrushLibrary::presetRow(const Preset &p,int &version) {
QJsonObject row{{"id",p.id},{"name",p.name},{"radius",p.radius},{"opacity",p.opacity},{"spacing",p.spacing},{"hardness",p.hardness},{"roundness",p.roundness},{"angle",p.angle},{"flow",p.flow},{"pressureSize",p.pressureSize},{"pressureFlow",p.pressureFlow}};if(!p.maskResource.isEmpty()){row["maskResource"]=p.maskResource;version=std::max(version,3);}if(!p.sourceResource.isEmpty()){row["sourceResource"]=p.sourceResource;version=std::max(version,4);}if(!p.pressure.isEmpty()){row["pressureDynamics"]=QJsonObject::fromVariantMap(p.pressure);version=std::max(version,p.pressure.contains("opacity")?6:5);}if(!p.device.isEmpty()){row["tipDynamics"]=QJsonObject::fromVariantMap(p.device);version=std::max(version,7);}if(!p.scatter.isEmpty()){row["scatterDynamics"]=QJsonObject::fromVariantMap(p.scatter);version=std::max(version,scatterHasControls(p.scatter)?35:8);}if(!p.shape.isEmpty()){row["shapeJitter"]=QJsonObject::fromVariantMap(p.shape);version=std::max(version,9);}if(!p.transfer.isEmpty()){row["transferJitter"]=QJsonObject::fromVariantMap(p.transfer);version=std::max(version,37);}if(!p.color.isEmpty()){row["colorDynamics"]=QJsonObject::fromVariantMap(p.color);version=std::max(version,10);}if(!p.texture.isEmpty()){row["texture"]=QJsonObject::fromVariantMap(p.texture);version=std::max(version,textureStateVersion(p.texture));}if(!p.dual.isEmpty()){row["dualBrush"]=QJsonObject::fromVariantMap(p.dual);version=std::max(version,12);if(scatterHasControls(p.dual.value("scatterDynamics").toMap()))version=std::max(version,35);if(dualHasExtendedMode(p.dual))version=std::max(version,36);}if(smoothPressure(p.pressure,p.dual))version=std::max(version,13);if(!p.smoothing.isEmpty()){row["smoothing"]=QJsonObject::fromVariantMap(p.smoothing);version=std::max(version,14);}if(p.dual.contains("sizing"))version=std::max(version,dualHasRatio(p.dual)?38:15);if(!p.wet.isEmpty()){row["wetMix"]=QJsonObject::fromVariantMap(p.wet);version=std::max(version,17);}if(!p.taper.isEmpty()){row["strokeTaper"]=QJsonObject::fromVariantMap(p.taper);version=std::max(version,18);}if(!p.airbrush.isEmpty()){row["airbrush"]=QJsonObject::fromVariantMap(p.airbrush);version=std::max(version,23);}if(!p.speed.isEmpty()){row["speedDynamics"]=QJsonObject::fromVariantMap(p.speed);version=std::max(version,24);}if(!p.tangent.isEmpty()){row["tangentDynamics"]=QJsonObject::fromVariantMap(p.tangent);version=std::max(version,25);}if(!p.distance.isEmpty()){row["distanceDynamics"]=QJsonObject::fromVariantMap(p.distance);version=std::max(version,26);}if(!p.tilt.isEmpty()){row["tiltDynamics"]=QJsonObject::fromVariantMap(p.tilt);version=std::max(version,27);}if(!p.rotation.isEmpty()){row["rotationDynamics"]=QJsonObject::fromVariantMap(p.rotation);version=std::max(version,28);}if(!p.direction.isEmpty()){row["directionDynamics"]=QJsonObject::fromVariantMap(p.direction);version=std::max(version,29);}if(!p.matrix.isEmpty()){row["sourceMatrix"]=QJsonObject::fromVariantMap(p.matrix);version=std::max(version,32);}if(!p.fade.isEmpty()){row["fadeDynamics"]=QJsonObject::fromVariantMap(p.fade);version=std::max(version,34);}return row;
}
QByteArray BrushLibrary::snapshot() const {
    QJsonArray rows,baselines;int version=2,resetVersion=2;
    for(const auto &p:m_presets){rows.append(presetRow(p,version));baselines.append(presetRow(m_resetPresets.value(p.id,p),resetVersion));}
    QJsonObject resetState{{"version",resetVersion},{"selected",m_selected},{"presets",baselines}};
    if(!m_organization.isEmpty()){version=std::max(version,20);resetState["version"]=std::max(resetVersion,20);resetState["organization"]=m_organization;}
    QJsonObject root{{"version",version},{"selected",m_selected},{"presets",rows},{"cursor",QJsonObject{{"outline",m_cursorOutline},{"center",m_cursorCenter},{"hoverOnly",m_cursorHoverOnly},{"width",m_cursorWidth},{"simple",m_cursorSimple},{"composite",m_cursorComposite},{"image",m_cursorImage},{"imageOpacity",m_cursorImageOpacity},{"color",m_cursorColor},{"secondaryColor",m_cursorSecondaryColor}}},
                                     {"recentColors",QJsonArray::fromStringList(m_recentColors)},{"colorState",QJsonObject{{"foreground",m_foregroundColor.name(QColor::HexArgb)},{"background",m_backgroundColor.name(QColor::HexArgb)}}},{"resetState",resetState},{"brushTool",QJsonObject{{"writeBlend",m_writeBlend}}}};
    if(!m_organization.isEmpty())root["organization"]=m_organization;
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}
QJsonObject BrushLibrary::exportPreset(const Preset &p,bool includeSource,const QString &id) {
    if(!p.sourceResource.isEmpty() && !includeSource)return {};
    const QStringList builtins={"round-pressure","round-fine","round-translucent","round-wide"};
    QJsonObject parameters{{"diameter",p.radius*2},{"spacing",p.spacing/2},{"dabOpacity",p.opacity},{"hardness",p.hardness},{"roundness",p.roundness},{"angle",p.angle},{"flow",p.flow},{"pressureSize",p.pressureSize},{"pressureFlow",p.pressureFlow}};
    QJsonObject preset{{"id",id},{"name",p.name},{"engineProfile","drawverse.brush"},{"behaviorVersion",1},{"requiredCapabilities",QJsonArray{p.maskResource.isEmpty()?"procedural-tip":"image-tip","pressure","dab-flow"}},{"parameters",parameters}};
    if(!p.pressure.isEmpty()){parameters["pressureDynamics"]=QJsonObject::fromVariantMap(p.pressure);preset["parameters"]=parameters;preset["behaviorVersion"]=p.pressure.contains("opacity")?3:2;auto caps=preset.value("requiredCapabilities").toArray();caps.append("pressure-curves");if(p.pressure.contains("opacity"))caps.append("opacity-pressure");preset["requiredCapabilities"]=caps;}
    if(!p.device.isEmpty()){parameters["tipDynamics"]=QJsonObject::fromVariantMap(p.device);preset["parameters"]=parameters;preset["behaviorVersion"]=4;auto caps=preset.value("requiredCapabilities").toArray();caps.append("tip-dynamics");preset["requiredCapabilities"]=caps;}
    if(!p.scatter.isEmpty()){parameters["scatterDynamics"]=QJsonObject::fromVariantMap(p.scatter);preset["parameters"]=parameters;preset["behaviorVersion"]=scatterHasControls(p.scatter)?27:5;auto caps=preset.value("requiredCapabilities").toArray();caps.append("scatter-stamps");if(scatterHasControls(p.scatter))caps.append("scatter-controls");preset["requiredCapabilities"]=caps;}
    if(!p.shape.isEmpty()){parameters["shapeJitter"]=QJsonObject::fromVariantMap(p.shape);preset["parameters"]=parameters;preset["behaviorVersion"]=6;auto caps=preset.value("requiredCapabilities").toArray();caps.append("shape-jitter");preset["requiredCapabilities"]=caps;}
    if(!p.transfer.isEmpty()){parameters["transferJitter"]=QJsonObject::fromVariantMap(p.transfer);preset["parameters"]=parameters;auto caps=preset.value("requiredCapabilities").toArray();caps.append("transfer-jitter");preset["requiredCapabilities"]=caps;}
    if(!p.color.isEmpty()){parameters["colorDynamics"]=QJsonObject::fromVariantMap(p.color);preset["parameters"]=parameters;preset["behaviorVersion"]=7;auto caps=preset.value("requiredCapabilities").toArray();caps.append("color-dynamics");preset["requiredCapabilities"]=caps;}
    if(!p.texture.isEmpty()){preset["texture"]=QJsonObject::fromVariantMap(p.texture);preset["behaviorVersion"]=8;auto caps=preset.value("requiredCapabilities").toArray();caps.append("texture-cover");preset["requiredCapabilities"]=caps;}
    if(!p.dual.isEmpty()){preset["dualBrush"]=QJsonObject::fromVariantMap(p.dual);preset["behaviorVersion"]=dualHasExtendedMode(p.dual)?28:(scatterHasControls(p.dual.value("scatterDynamics").toMap())?27:9);auto caps=preset.value("requiredCapabilities").toArray();caps.append("dual-cover");if(scatterHasControls(p.dual.value("scatterDynamics").toMap()))caps.append("scatter-controls");if(dualHasExtendedMode(p.dual))caps.append("dual-cover-extended");preset["requiredCapabilities"]=caps;}
    if(smoothPressure(p.pressure,p.dual)){preset["behaviorVersion"]=10;auto caps=preset.value("requiredCapabilities").toArray();caps.append("pressure-smooth");preset["requiredCapabilities"]=caps;}
    if(!p.smoothing.isEmpty()){preset["smoothing"]=QJsonObject::fromVariantMap(p.smoothing);preset["behaviorVersion"]=11;auto caps=preset.value("requiredCapabilities").toArray();caps.append("path-smoothing");preset["requiredCapabilities"]=caps;}
    if(p.dual.contains("sizing")){preset["behaviorVersion"]=12;auto caps=preset.value("requiredCapabilities").toArray();caps.append("dual-relative-size");preset["requiredCapabilities"]=caps;}
    if(!p.wet.isEmpty()){preset["wetMix"]=QJsonObject::fromVariantMap(p.wet);preset["behaviorVersion"]=14;auto caps=preset.value("requiredCapabilities").toArray();caps.append("wet-mix");preset["requiredCapabilities"]=caps;}
    if(!p.taper.isEmpty()){preset["strokeTaper"]=QJsonObject::fromVariantMap(p.taper);preset["behaviorVersion"]=15;auto caps=preset.value("requiredCapabilities").toArray();caps.append("stroke-taper");preset["requiredCapabilities"]=caps;}
    if(!p.airbrush.isEmpty()){parameters["airbrush"]=QJsonObject::fromVariantMap(p.airbrush);preset["parameters"]=parameters;preset["behaviorVersion"]=18;auto caps=preset.value("requiredCapabilities").toArray();caps.append("airbrush");preset["requiredCapabilities"]=caps;}
    if(!p.speed.isEmpty()){parameters["speedDynamics"]=QJsonObject::fromVariantMap(p.speed);preset["parameters"]=parameters;preset["behaviorVersion"]=19;auto caps=preset.value("requiredCapabilities").toArray();caps.append("speed-dynamics");preset["requiredCapabilities"]=caps;}
    if(!p.tangent.isEmpty()){parameters["tangentDynamics"]=QJsonObject::fromVariantMap(p.tangent);preset["parameters"]=parameters;preset["behaviorVersion"]=20;auto caps=preset.value("requiredCapabilities").toArray();caps.append("tangent-dynamics");preset["requiredCapabilities"]=caps;}
    if(!p.distance.isEmpty()){parameters["distanceDynamics"]=QJsonObject::fromVariantMap(p.distance);preset["parameters"]=parameters;preset["behaviorVersion"]=21;auto caps=preset.value("requiredCapabilities").toArray();caps.append("distance-dynamics");preset["requiredCapabilities"]=caps;}
    if(!p.tilt.isEmpty()){parameters["tiltDynamics"]=QJsonObject::fromVariantMap(p.tilt);preset["parameters"]=parameters;preset["behaviorVersion"]=22;auto caps=preset.value("requiredCapabilities").toArray();caps.append("tilt-dynamics");preset["requiredCapabilities"]=caps;}
    if(!p.rotation.isEmpty()){parameters["rotationDynamics"]=QJsonObject::fromVariantMap(p.rotation);preset["parameters"]=parameters;preset["behaviorVersion"]=23;auto caps=preset.value("requiredCapabilities").toArray();caps.append("rotation-dynamics");preset["requiredCapabilities"]=caps;}
    if(!p.direction.isEmpty()){parameters["directionDynamics"]=QJsonObject::fromVariantMap(p.direction);preset["parameters"]=parameters;preset["behaviorVersion"]=24;auto caps=preset.value("requiredCapabilities").toArray();caps.append("direction-dynamics");preset["requiredCapabilities"]=caps;}
    if(!p.matrix.isEmpty()){parameters["sourceMatrix"]=QJsonObject::fromVariantMap(p.matrix);preset["parameters"]=parameters;preset["behaviorVersion"]=25;auto caps=preset.value("requiredCapabilities").toArray();caps.append("dynamics-source-matrix");preset["requiredCapabilities"]=caps;}
    if(!p.fade.isEmpty()){parameters["fadeDynamics"]=QJsonObject::fromVariantMap(p.fade);preset["parameters"]=parameters;preset["behaviorVersion"]=26;auto caps=preset.value("requiredCapabilities").toArray();caps.append("fade-dynamics");preset["requiredCapabilities"]=caps;}
    if(scatterHasControls(p.scatter) || scatterHasControls(p.dual.value("scatterDynamics").toMap()))preset["behaviorVersion"]=27;
    if(dualHasExtendedMode(p.dual))preset["behaviorVersion"]=28;
    if(!p.transfer.isEmpty())preset["behaviorVersion"]=29;
    if(dualHasRatio(p.dual)){preset["behaviorVersion"]=30;auto caps=preset.value("requiredCapabilities").toArray();caps.append("dual-size-ratio");preset["requiredCapabilities"]=caps;}
    if(!p.maskResource.isEmpty())preset["tipResource"]=p.maskResource;
    if(!p.sourceResource.isEmpty())preset["sourceResource"]=p.sourceResource;
    return preset;
}
QByteArray BrushLibrary::exportPackage(bool includeSource) const {
    const auto &p=m_presets[index(m_selected)];
    const QStringList builtins={"round-pressure","round-fine","round-translucent","round-wide"};
    const int builtin=builtins.indexOf(p.id);
    const auto id=builtin<0?p.id:QString("dba00000-0000-4000-8000-%1").arg(builtin+1,12,16,QChar('0'));
    const auto preset=exportPreset(p,includeSource,id);
    if(preset.isEmpty())return {};
    const QJsonObject package{{"packageId",QUuid::createUuid().toString(QUuid::WithoutBraces)},{"brushes",QJsonArray{preset}}};
    const auto organization=organizationForPreset(m_organization,p.id,id);
    if(organization.isEmpty())return QJsonDocument(package).toJson(QJsonDocument::Compact);
    return QJsonDocument(QJsonObject{{"model","ordered-package-v1"},{"package",package},{"organization",organization}}).toJson(QJsonDocument::Compact);
}
QByteArray BrushLibrary::exportAllPackage(bool includeSource) const {
    if(m_presets.isEmpty() || m_presets.size()>1024)return {};
    const QStringList builtins={"round-pressure","round-fine","round-translucent","round-wide"};
    QHash<QString,QString> mapping;QJsonArray brushes;QStringList order;QSet<QString> emitted;
    for(const auto &preset:m_presets) {
        const int builtin=builtins.indexOf(preset.id);
        QString id=builtin<0?preset.id:QString("dba00000-0000-4000-8000-%1").arg(builtin+1,12,16,QChar('0'));
        if(id.isEmpty() || QUuid(id).isNull() || emitted.contains(id))return {};
        const auto row=exportPreset(preset,includeSource,id);
        if(row.isEmpty())return {};
        mapping.insert(preset.id,id);emitted.insert(id);order.append(id);brushes.append(row);
    }
    auto remapItems=[&](const QJsonArray &items) {
        QJsonArray result;
        for(const auto &value:items) {
            auto item=value.toObject();
            if(item.value("kind").toString()=="preset") {
                const auto id=mapping.value(item.value("id").toString());
                if(id.isEmpty())return QJsonArray{};
                item["id"]=id;
            }
            result.append(item);
        }
        return result;
    };
    QJsonObject organization;
    if(m_organization.isEmpty()) {
        QJsonArray roots;for(const auto &id:order)roots.append(QJsonObject{{"kind","preset"},{"id",id}});
        organization=QJsonObject{{"model","ordered-tree-v1"},{"roots",roots},{"groups",QJsonArray{}}};
    } else {
        organization=m_organization;
        const auto roots=remapItems(organization.value("roots").toArray());
        if(roots.isEmpty() && !organization.value("roots").toArray().isEmpty())return {};
        auto groups=organization.value("groups").toArray();
        for(qsizetype i=0;i<groups.size();++i){auto group=groups[i].toObject();const auto items=remapItems(group.value("items").toArray());if(items.isEmpty() && !group.value("items").toArray().isEmpty())return {};group["items"]=items;groups[i]=group;}
        organization["roots"]=roots;organization["groups"]=groups;
    }
    const QJsonObject package{{"packageId",QUuid::createUuid().toString(QUuid::WithoutBraces)},{"brushes",brushes}};
    return QJsonDocument(QJsonObject{{"model","ordered-package-v1"},{"package",package},{"organization",organization}}).toJson(QJsonDocument::Compact);
}
QByteArray BrushLibrary::exportSelectionPackage(bool includeSource) const {
    if(m_selectedItems.isEmpty() || m_presets.size()>1024)return {};
    QHash<QString,QJsonObject> groupsById;
    for(const auto &value:m_organization.value("groups").toArray()) {const auto group=value.toObject();groupsById.insert(group.value("id").toString(),group);}
    QSet<QString> selectedGroups,includedGroups,includedPresets;
    QJsonArray roots;
    std::function<void(const QString &)> includeGroup=[&](const QString &id) {
        if(includedGroups.contains(id) || !groupsById.contains(id))return;
        includedGroups.insert(id);
        for(const auto &value:groupsById.value(id).value("items").toArray()) {
            const auto item=value.toObject();const auto child=item.value("id").toString();
            if(item.value("kind").toString()=="group")includeGroup(child);else includedPresets.insert(child);
        }
    };
    for(const auto &value:m_selectedItems)if(value.toObject().value("kind").toString()=="group")selectedGroups.insert(value.toObject().value("id").toString());
    for(const auto &value:m_selectedItems) {
        const auto item=value.toObject();const auto kind=item.value("kind").toString(),id=item.value("id").toString();
        if(kind=="group") {
            bool nested=false;for(const auto &ancestor:selectedGroups)if(ancestor!=id && groupContainsGroup(m_organization,ancestor,id)){nested=true;break;}
            if(nested)continue;
            includeGroup(id);roots.append(QJsonObject{{"kind","group"},{"id",id}});
        } else if(kind=="preset" && index(id)>=0) {
            bool nested=false;for(const auto &group:selectedGroups)if(groupContainsItem(m_organization,group,"preset",id)){nested=true;break;}
            if(nested)continue;
            includedPresets.insert(id);roots.append(QJsonObject{{"kind","preset"},{"id",id}});
        }
    }
    if(includedPresets.isEmpty())return {};
    QJsonArray groups;
    for(const auto &value:m_organization.value("groups").toArray()) {
        const auto group=value.toObject();
        if(includedGroups.contains(group.value("id").toString()))groups.append(group);
    }
    const QJsonObject candidate{{"model","ordered-tree-v1"},{"roots",roots},{"groups",groups}};
    QStringList ids;for(const auto &id:includedPresets)ids.append(id);
    QJsonObject normalized;QStringList order;
    if(!BrushOrganization::normalize(candidate,ids,normalized,order,false) || order.isEmpty())return {};
    const QStringList builtins={"round-pressure","round-fine","round-translucent","round-wide"};
    QHash<QString,QString> mapping;QJsonArray brushes;QSet<QString> emitted;
    for(const auto &presetId:order) {
        const int i=index(presetId);if(i<0)return {};
        const int builtin=builtins.indexOf(presetId);
        QString id=builtin<0?presetId:QString("dba00000-0000-4000-8000-%1").arg(builtin+1,12,16,QChar('0'));
        if(id.isEmpty() || QUuid(id).isNull() || emitted.contains(id))return {};
        const auto row=exportPreset(m_presets[i],includeSource,id);if(row.isEmpty())return {};
        mapping.insert(presetId,id);emitted.insert(id);brushes.append(row);
    }
    auto remapItems=[&](const QJsonArray &items) {
        QJsonArray result;
        for(const auto &value:items) {
            auto item=value.toObject();
            if(item.value("kind").toString()=="preset") {
                const auto id=mapping.value(item.value("id").toString());
                if(id.isEmpty())return QJsonArray{};
                item["id"]=id;
            }
            result.append(item);
        }
        return result;
    };
    if(normalized.isEmpty()) {
        QJsonArray mappedRoots;for(const auto &presetId:order)mappedRoots.append(QJsonObject{{"kind","preset"},{"id",mapping.value(presetId)}});
        normalized=QJsonObject{{"model","ordered-tree-v1"},{"roots",mappedRoots},{"groups",QJsonArray{}}};
    } else {
        normalized["roots"]=remapItems(normalized.value("roots").toArray());
        auto normalizedGroups=normalized.value("groups").toArray();
        for(qsizetype i=0;i<normalizedGroups.size();++i) {auto group=normalizedGroups[i].toObject();group["items"]=remapItems(group.value("items").toArray());normalizedGroups[i]=group;}
        normalized["groups"]=normalizedGroups;
    }
    const QJsonObject package{{"packageId",QUuid::createUuid().toString(QUuid::WithoutBraces)},{"brushes",brushes}};
    return QJsonDocument(QJsonObject{{"model","ordered-package-v1"},{"package",package},{"organization",normalized}}).toJson(QJsonDocument::Compact);
}
QByteArray BrushLibrary::catalogSnapshot() const {
    const auto root=QJsonDocument::fromJson(snapshot()).object();
    QJsonObject executing{{"version",root.value("version")},{"selected",root.value("selected")},{"presets",root.value("presets")}};
    if(root.contains("organization"))executing["organization"]=root.value("organization");
    return QJsonDocument(executing).toJson(QJsonDocument::Compact);
}
bool BrushLibrary::applyNativeCatalog(const QByteArray &json) {
    const auto native=QJsonDocument::fromJson(json).object();
    if((native.value("version").toInt()!=2 && native.value("version").toInt()!=3 && native.value("version").toInt()!=4 && native.value("version").toInt()!=5 && native.value("version").toInt()!=6 && native.value("version").toInt()!=7 && native.value("version").toInt()!=8 && native.value("version").toInt()!=9 && native.value("version").toInt()!=10 && native.value("version").toInt()!=11 && native.value("version").toInt()!=12 && native.value("version").toInt()!=13 && native.value("version").toInt()!=14 && native.value("version").toInt()!=15 && native.value("version").toInt()!=16 && native.value("version").toInt()!=17 && native.value("version").toInt()!=18 && native.value("version").toInt()!=19 && native.value("version").toInt()!=20 && native.value("version").toInt()!=21 && native.value("version").toInt()!=22 && native.value("version").toInt()!=23 && native.value("version").toInt()!=24 && native.value("version").toInt()!=25 && native.value("version").toInt()!=26 && native.value("version").toInt()!=27 && native.value("version").toInt()!=28 && native.value("version").toInt()!=29 && native.value("version").toInt()!=30 && native.value("version").toInt()!=31 && native.value("version").toInt()!=32 && native.value("version").toInt()!=33 && native.value("version").toInt()!=34 && native.value("version").toInt()!=35 && native.value("version").toInt()!=36 && native.value("version").toInt()!=37 && native.value("version").toInt()!=38) || !native.value("presets").isArray() || !native.value("selected").isString())return false;
    if(QJsonDocument(native).toJson(QJsonDocument::Compact)==catalogSnapshot())return true;
    auto state=QJsonDocument::fromJson(snapshot()).object();state["version"]=native.value("version");state["selected"]=native.value("selected");state["presets"]=native.value("presets");if(native.contains("organization"))state["organization"]=native.value("organization");else state.remove("organization");
    return restore(QJsonDocument(state).toJson(QJsonDocument::Compact));
}
bool BrushLibrary::importPackage(const QByteArray &json,const QString &groupName) {
    if(json.size()>2*1024*1024)return false;
    const auto root=QJsonDocument::fromJson(json).object();
    const bool ordered=root.value("model").toString()=="ordered-package-v1";
    if(ordered && (!root.value("package").isObject() || !root.value("organization").isObject() || root.value("organization").toObject().value("model").toString()!="ordered-tree-v1"))return false;
    const auto package=ordered?root.value("package").toObject():root;
    const auto inputOrganization=ordered?root.value("organization").toObject():QJsonObject{};
    if(!package.value("brushes").isArray())return false;
    const auto brushes=package.value("brushes").toArray();
    if(brushes.isEmpty() || brushes.size()+m_presets.size()>1024)return false;
    auto state=QJsonDocument::fromJson(snapshot()).object();auto rows=state.value("presets").toArray();QSet<QString> ids;
    for(const auto &p:m_presets)ids.insert(p.id);
    const qsizetype existingCount=m_presets.size();
    QHash<QString,QString> importedIds;
    QString selected;
    QJsonObject inputTree;
    const auto cleanGroupName=groupName.trimmed();
    if(!cleanGroupName.isEmpty() && (cleanGroupName.size()>80 || cleanGroupName.contains(QChar(0))))return false;
    if(ordered) {
        QStringList inputIds;
        for(const auto &value:brushes)inputIds.append(value.toObject().value("id").toString());
        QStringList inputOrder;
        if(!BrushOrganization::normalize(inputOrganization,inputIds,inputTree,inputOrder) || inputOrder!=inputIds)return false;
    }
    for(const auto &value:brushes){
        const auto b=value.toObject();const auto params=b.value("parameters").toObject();const auto sourceId=b.value("id").toString();QString id=sourceId;
        // VBR stays at 1.0 and older packages may carry stale behavior metadata.
        // Execute the fields that are present instead of rejecting the package.
        if(QUuid(id).isNull() || (b.value("engineProfile").toString()!="drawverse.brush" && b.value("engineProfile").toString()!="legacy.round"))return false;
        if(importedIds.contains(sourceId))return false;
        if(ids.contains(id))id=QUuid::createUuid().toString(QUuid::WithoutBraces);
        ids.insert(id);importedIds.insert(sourceId,id);if(selected.isEmpty())selected=id;
        QJsonObject row{{"id",id},{"name",b.value("name")},{"radius",params.value("diameter").toDouble()/2},{"opacity",params.value("dabOpacity")},{"spacing",params.value("spacing").toDouble()*2}};
        for(const auto &key:{"hardness","roundness","angle","flow","pressureSize","pressureFlow"})row[key]=params.value(key);
        if(b.contains("tipResource")){row["maskResource"]=b.value("tipResource");state["version"]=std::max(state.value("version").toInt(),3);}
        if(b.contains("sourceResource")){row["sourceResource"]=b.value("sourceResource");state["version"]=std::max(state.value("version").toInt(),4);}
        if(params.contains("pressureDynamics")){row["pressureDynamics"]=params.value("pressureDynamics");state["version"]=std::max(state.value("version").toInt(),params.value("pressureDynamics").toObject().contains("opacity")?6:5);}
        if(params.contains("tipDynamics")){row["tipDynamics"]=params.value("tipDynamics");state["version"]=std::max(state.value("version").toInt(),7);}
        if(params.contains("scatterDynamics")){row["scatterDynamics"]=params.value("scatterDynamics");state["version"]=std::max(state.value("version").toInt(),scatterHasControls(params.value("scatterDynamics").toObject().toVariantMap())?35:8);}
        if(params.contains("shapeJitter")){row["shapeJitter"]=params.value("shapeJitter");state["version"]=std::max(state.value("version").toInt(),9);}
        if(params.contains("transferJitter")){row["transferJitter"]=params.value("transferJitter");state["version"]=std::max(state.value("version").toInt(),37);}
        if(params.contains("colorDynamics")){row["colorDynamics"]=params.value("colorDynamics");state["version"]=std::max(state.value("version").toInt(),10);}
        if(b.contains("texture")){auto texture=b.value("texture").toObject();texture["mode"]=normalizeTextureMode(texture.value("mode").toString());row["texture"]=texture;state["version"]=std::max(state.value("version").toInt(),11);if(advancedTextureModes().contains(texture.value("mode").toString()))state["version"]=std::max(state.value("version").toInt(),30);}
        if(b.contains("dualBrush")){auto dual=b.value("dualBrush").toObject();dual["mode"]=normalizeDualMode(dual.value("mode").toString());dual.remove("color");row["dualBrush"]=dual;state["version"]=std::max(state.value("version").toInt(),12);if(scatterHasControls(dual.toVariantMap().value("scatterDynamics").toMap()))state["version"]=std::max(state.value("version").toInt(),35);if(dualHasExtendedMode(dual.toVariantMap()))state["version"]=std::max(state.value("version").toInt(),36);}
        if(smoothPressure(params.value("pressureDynamics").toObject().toVariantMap(),b.value("dualBrush").toObject().toVariantMap()))state["version"]=std::max(state.value("version").toInt(),13);
        if(b.contains("smoothing")){row["smoothing"]=b.value("smoothing");state["version"]=std::max(state.value("version").toInt(),14);}
        if(b.value("dualBrush").toObject().contains("sizing"))state["version"]=std::max(state.value("version").toInt(),dualHasRatio(b.value("dualBrush").toObject().toVariantMap())?38:15);
        if(b.contains("wetMix")){auto wet=b.value("wetMix").toObject();wet.remove("dualColor");row["wetMix"]=wet;state["version"]=std::max(state.value("version").toInt(),17);}
        if(b.contains("strokeTaper")){row["strokeTaper"]=b.value("strokeTaper");state["version"]=std::max(state.value("version").toInt(),18);}
        if(params.contains("airbrush")){if(!validAirbrush(params.value("airbrush").toObject()))return false;row["airbrush"]=params.value("airbrush");state["version"]=std::max(state.value("version").toInt(),23);}
        if(params.contains("speedDynamics")){if(!validSpeedDynamics(params.value("speedDynamics").toObject()))return false;row["speedDynamics"]=params.value("speedDynamics");state["version"]=std::max(state.value("version").toInt(),24);}
        if(params.contains("tangentDynamics")){if(!validTangentDynamics(params.value("tangentDynamics").toObject()))return false;row["tangentDynamics"]=params.value("tangentDynamics");state["version"]=std::max(state.value("version").toInt(),25);}
        if(params.contains("distanceDynamics")){if(!validDistanceDynamics(params.value("distanceDynamics").toObject()))return false;row["distanceDynamics"]=params.value("distanceDynamics");state["version"]=std::max(state.value("version").toInt(),26);}
        if(params.contains("tiltDynamics")){if(!validTiltDynamics(params.value("tiltDynamics").toObject()))return false;row["tiltDynamics"]=params.value("tiltDynamics");state["version"]=std::max(state.value("version").toInt(),27);}
        if(params.contains("rotationDynamics")){if(!validRotationDynamics(params.value("rotationDynamics").toObject()))return false;row["rotationDynamics"]=params.value("rotationDynamics");state["version"]=std::max(state.value("version").toInt(),28);}
        if(params.contains("directionDynamics")){if(!validDirectionDynamics(params.value("directionDynamics").toObject()))return false;row["directionDynamics"]=params.value("directionDynamics");state["version"]=std::max(state.value("version").toInt(),29);}
        if(params.contains("sourceMatrix")){if(!params.value("sourceMatrix").isObject() || !validDynamicsSourceMatrix(params.value("sourceMatrix").toObject()))return false;row["sourceMatrix"]=params.value("sourceMatrix");state["version"]=std::max(state.value("version").toInt(),32);}
        if(params.contains("fadeDynamics")){if(!params.value("fadeDynamics").isObject() || !validFadeDynamics(params.value("fadeDynamics").toObject()))return false;row["fadeDynamics"]=params.value("fadeDynamics");state["version"]=std::max(state.value("version").toInt(),34);}
        rows.append(row);
    }
    if(ordered) {
        QSet<QString> usedGroups;for(const auto &value:m_organization.value("groups").toArray())usedGroups.insert(value.toObject().value("id").toString());
        QHash<QString,QString> groupMap;const auto inputGroups=inputTree.value("groups").toArray();
        for(const auto &value:inputGroups){const auto oldId=value.toObject().value("id").toString();QString id;do{id=QUuid::createUuid().toString(QUuid::WithoutBraces);}while(usedGroups.contains(id));usedGroups.insert(id);groupMap.insert(oldId,id);}
        auto remapItems=[&](const QJsonArray &items){QJsonArray result;for(const auto &value:items){auto item=value.toObject();const auto kind=item.value("kind").toString(),old=item.value("id").toString();QString id=kind=="preset"?importedIds.value(old):kind=="group"?groupMap.value(old):QString();if(id.isEmpty())return QJsonArray{};item["id"]=id;result.append(item);}return result;};
        QJsonArray importedRoots;
        if(inputTree.isEmpty()){for(const auto &value:brushes){const auto id=importedIds.value(value.toObject().value("id").toString());if(id.isEmpty())return false;importedRoots.append(QJsonObject{{"kind","preset"},{"id",id}});}}
        else {importedRoots=remapItems(inputTree.value("roots").toArray());if(importedRoots.isEmpty() && !inputTree.value("roots").toArray().isEmpty())return false;}
        QJsonArray importedGroups;
        for(const auto &value:inputGroups){const auto group=value.toObject();const auto items=remapItems(group.value("items").toArray());if(items.isEmpty() && !group.value("items").toArray().isEmpty())return false;importedGroups.append(QJsonObject{{"id",groupMap.value(group.value("id").toString())},{"name",group.value("name")},{"items",items}});}
        QJsonArray roots,groups;
        if(m_organization.isEmpty()){for(qsizetype i=0;i<existingCount;++i)roots.append(QJsonObject{{"kind","preset"},{"id",rows[i].toObject().value("id").toString()}});}
        else {roots=m_organization.value("roots").toArray();groups=m_organization.value("groups").toArray();}
        if(cleanGroupName.isEmpty()){for(const auto &value:importedRoots)roots.append(value);}
        else {
            const auto outerId=QUuid::createUuid().toString(QUuid::WithoutBraces);
            roots.append(QJsonObject{{"kind","group"},{"id",outerId}});
            groups.append(QJsonObject{{"id",outerId},{"name",cleanGroupName},{"items",importedRoots}});
        }
        for(const auto &value:importedGroups)groups.append(value);
        const QJsonObject merged{{"model","ordered-tree-v1"},{"roots",roots},{"groups",groups}};
        QStringList allIds;for(const auto &value:rows)allIds.append(value.toObject().value("id").toString());QJsonObject normalized;QStringList order;
        if(!BrushOrganization::normalize(merged,allIds,normalized,order))return false;
        if(normalized.isEmpty())state.remove("organization");else {state["organization"]=normalized;state["version"]=std::max(state.value("version").toInt(),20);}
    } else if(!cleanGroupName.isEmpty()) {
        QJsonArray roots,groups;
        if(m_organization.isEmpty()){for(qsizetype i=0;i<existingCount;++i)roots.append(QJsonObject{{"kind","preset"},{"id",rows[i].toObject().value("id").toString()}});}
        else {roots=m_organization.value("roots").toArray();groups=m_organization.value("groups").toArray();}
        QJsonArray items;for(qsizetype i=existingCount;i<rows.size();++i)items.append(QJsonObject{{"kind","preset"},{"id",rows[i].toObject().value("id").toString()}});
        const auto outerId=QUuid::createUuid().toString(QUuid::WithoutBraces);
        roots.append(QJsonObject{{"kind","group"},{"id",outerId}});
        groups.append(QJsonObject{{"id",outerId},{"name",cleanGroupName},{"items",items}});
        const QJsonObject merged{{"model","ordered-tree-v1"},{"roots",roots},{"groups",groups}};
        QStringList allIds;for(const auto &value:rows)allIds.append(value.toObject().value("id").toString());QJsonObject normalized;QStringList order;
        if(!BrushOrganization::normalize(merged,allIds,normalized,order))return false;
        state["organization"]=normalized;state["version"]=std::max(state.value("version").toInt(),20);
    } else if(!m_organization.isEmpty()){auto organization=m_organization;for(qsizetype i=m_presets.size();i<rows.size();++i)organization=BrushOrganization::addPreset(organization,rows[i].toObject().value("id").toString());state["organization"]=organization;}
    state["presets"]=rows;state["selected"]=selected;
    if(!restore(QJsonDocument(state).toJson(QJsonDocument::Compact)))return false;
    emit persistRequested(snapshot());return true;
}
bool BrushLibrary::importAbr(const QByteArray &json,const QString &groupName) {
    if(json.size()>2*1024*1024)return false;
    const auto handoff=QJsonDocument::fromJson(json).object();
    if((handoff.value("version").toInt()!=1 && handoff.value("version").toInt()!=2 && handoff.value("version").toInt()!=3 && handoff.value("version").toInt()!=4 && handoff.value("version").toInt()!=5 && handoff.value("version").toInt()!=6 && handoff.value("version").toInt()!=7 && handoff.value("version").toInt()!=8 && handoff.value("version").toInt()!=9 && handoff.value("version").toInt()!=10 && handoff.value("version").toInt()!=11 && handoff.value("version").toInt()!=12 && handoff.value("version").toInt()!=13) || handoff.value("format").toString()!="abr" || !handoff.value("presets").isArray())return false;
    const auto imported=handoff.value("presets").toArray();
    if(imported.isEmpty() || imported.size()+m_presets.size()>1024)return false;
    auto state=QJsonDocument::fromJson(snapshot()).object();auto rows=state.value("presets").toArray();QString selected;
    for(const auto &value:imported) {
        if(!value.isObject())return false;const auto preset=value.toObject();const auto parameters=preset.value("parameters").toObject();
        if(!preset.value("partial").isBool() || !preset.value("sourceResource").isString())return false;
        const auto id=QUuid::createUuid().toString(QUuid::WithoutBraces);if(selected.isEmpty())selected=id;
        QJsonObject row{{"id",id},{"name",preset.value("name")},{"radius",parameters.value("diameter").toDouble()/2},{"opacity",parameters.value("dabOpacity")},{"spacing",parameters.value("spacing").toDouble()*2},{"sourceResource",preset.value("sourceResource")}};
        for(const auto &key:{"hardness","roundness","angle","flow","pressureSize","pressureFlow"})row[key]=parameters.value(key);
        if(parameters.contains("tipDynamics")) {
            if(handoff.value("version").toInt()<6 || !parameters.value("tipDynamics").isObject())return false;
            row["tipDynamics"]=parameters.value("tipDynamics");state["version"]=std::max(state.value("version").toInt(),7);
        }
        if(parameters.contains("shapeJitter")) {
            if(handoff.value("version").toInt()<5 || !parameters.value("shapeJitter").isObject())return false;
            row["shapeJitter"]=parameters.value("shapeJitter");state["version"]=std::max(state.value("version").toInt(),9);
        }
        if(parameters.contains("transferJitter")) {
            if(handoff.value("version").toInt()<12 || !parameters.value("transferJitter").isObject() || !validTransferJitter(parameters.value("transferJitter").toObject()))return false;
            row["transferJitter"]=parameters.value("transferJitter");state["version"]=std::max(state.value("version").toInt(),37);
        }
        if(parameters.contains("colorDynamics")) {
            if(handoff.value("version").toInt()<4 || !parameters.value("colorDynamics").isObject())return false;
            row["colorDynamics"]=parameters.value("colorDynamics");state["version"]=std::max(state.value("version").toInt(),10);
        }
        if(parameters.contains("scatterDynamics")) {
            if(handoff.value("version").toInt()<3 || !parameters.value("scatterDynamics").isObject())return false;
            const auto scatter=parameters.value("scatterDynamics").toObject();
            if(scatterHasControls(scatter.toVariantMap()) && handoff.value("version").toInt()<11)return false;
            row["scatterDynamics"]=parameters.value("scatterDynamics");state["version"]=std::max(state.value("version").toInt(),scatterHasControls(scatter.toVariantMap())?35:8);
        }
        if(parameters.contains("pressureDynamics")) {
            if(handoff.value("version").toInt()<2 || !parameters.value("pressureDynamics").isObject())return false;
            row["pressureDynamics"]=parameters.value("pressureDynamics");state["version"]=std::max(state.value("version").toInt(),parameters.value("pressureDynamics").toObject().contains("opacity")?6:5);
        }
        if(parameters.contains("tiltDynamics")) {
            if(handoff.value("version").toInt()<8 || !parameters.value("tiltDynamics").isObject() || !validTiltDynamics(parameters.value("tiltDynamics").toObject()))return false;
            row["tiltDynamics"]=parameters.value("tiltDynamics");state["version"]=std::max(state.value("version").toInt(),27);
        }
        if(parameters.contains("rotationDynamics")) {
            if(handoff.value("version").toInt()<8 || !parameters.value("rotationDynamics").isObject() || !validRotationDynamics(parameters.value("rotationDynamics").toObject()))return false;
            row["rotationDynamics"]=parameters.value("rotationDynamics");state["version"]=std::max(state.value("version").toInt(),28);
        }
        if(parameters.contains("directionDynamics")) {
            if(handoff.value("version").toInt()<8 || !parameters.value("directionDynamics").isObject() || !validDirectionDynamics(parameters.value("directionDynamics").toObject()))return false;
            row["directionDynamics"]=parameters.value("directionDynamics");state["version"]=std::max(state.value("version").toInt(),29);
        }
        if(parameters.contains("sourceMatrix")) {
            if(handoff.value("version").toInt()<8 || !parameters.value("sourceMatrix").isObject() || !validDynamicsSourceMatrix(parameters.value("sourceMatrix").toObject()))return false;
            row["sourceMatrix"]=parameters.value("sourceMatrix");state["version"]=std::max(state.value("version").toInt(),32);
        }
        if(parameters.contains("fadeDynamics")) {
            if(handoff.value("version").toInt()<9 || !parameters.value("fadeDynamics").isObject() || !validFadeDynamics(parameters.value("fadeDynamics").toObject()))return false;
            row["fadeDynamics"]=parameters.value("fadeDynamics");state["version"]=std::max(state.value("version").toInt(),34);
        }
        if(preset.contains("dualBrush") && !preset.value("dualBrush").isNull()) {
            if(handoff.value("version").toInt()<10 || !preset.value("dualBrush").isObject() || !validDual(preset.value("dualBrush").toObject()))return false;
            const auto dualControls=preset.value("dualBrush").toObject().value("scatterDynamics").toObject().toVariantMap();
            if(scatterHasControls(dualControls) && handoff.value("version").toInt()<11)return false;
            const auto dual=preset.value("dualBrush").toObject().toVariantMap();
            if(dualHasExtendedMode(dual) && handoff.value("version").toInt()<11)return false;
            if(dualHasRatio(dual) && handoff.value("version").toInt()<13)return false;
            row["dualBrush"]=preset.value("dualBrush");state["version"]=std::max(state.value("version").toInt(),dualHasRatio(dual)?38:(dualHasExtendedMode(dual)?36:(scatterHasControls(dualControls)?35:12)));
        }
        if(!preset.value("tipResource").isNull())row["maskResource"]=preset.value("tipResource");
        if(preset.contains("texture") && !preset.value("texture").isNull()) {
            if(handoff.value("version").toInt()<7 || !preset.value("texture").isObject())return false;
            auto texture=preset.value("texture").toObject();texture["mode"]=normalizeTextureMode(texture.value("mode").toString());
            if(!validTexture(texture))return false;
            row["texture"]=texture;state["version"]=std::max(state.value("version").toInt(),textureStateVersion(texture.toVariantMap()));
        }
        rows.append(row);
    }
    const auto cleanGroupName=groupName.trimmed();
    if(!cleanGroupName.isEmpty() && (cleanGroupName.size()>80 || cleanGroupName.contains(QChar(0))))return false;
    if(!cleanGroupName.isEmpty()) {
        const qsizetype existingCount=m_presets.size();
        QJsonArray roots,groups;
        if(m_organization.isEmpty()){for(qsizetype i=0;i<existingCount;++i)roots.append(QJsonObject{{"kind","preset"},{"id",rows[i].toObject().value("id").toString()}});}
        else {roots=m_organization.value("roots").toArray();groups=m_organization.value("groups").toArray();}
        QJsonArray items;for(qsizetype i=existingCount;i<rows.size();++i)items.append(QJsonObject{{"kind","preset"},{"id",rows[i].toObject().value("id").toString()}});
        const auto outerId=QUuid::createUuid().toString(QUuid::WithoutBraces);
        roots.append(QJsonObject{{"kind","group"},{"id",outerId}});
        groups.append(QJsonObject{{"id",outerId},{"name",cleanGroupName},{"items",items}});
        const QJsonObject merged{{"model","ordered-tree-v1"},{"roots",roots},{"groups",groups}};
        QStringList allIds;for(const auto &value:rows)allIds.append(value.toObject().value("id").toString());QJsonObject normalized;QStringList order;
        if(!BrushOrganization::normalize(merged,allIds,normalized,order))return false;
        state["organization"]=normalized;state["version"]=std::max(state.value("version").toInt(),20);
    } else if(!m_organization.isEmpty()){auto organization=m_organization;for(qsizetype i=m_presets.size();i<rows.size();++i)organization=BrushOrganization::addPreset(organization,rows[i].toObject().value("id").toString());state["organization"]=organization;}
    state["presets"]=rows;state["selected"]=selected;state["version"]=std::max(state.value("version").toInt(),4);
    if(!restore(QJsonDocument(state).toJson(QJsonDocument::Compact)))return false;
    emit persistRequested(snapshot());return true;
}
bool BrushLibrary::restore(const QByteArray &json) {
    if(json.size()>4*1024*1024+16384)return false;
    const auto root=QJsonDocument::fromJson(json).object();const auto rows=root.value("presets").toArray();
    QJsonObject executing{{"version",root.value("version")},{"selected",root.value("selected")},{"presets",root.value("presets")}};
    if(root.contains("organization"))executing["organization"]=root.value("organization");
    if(QJsonDocument(executing).toJson(QJsonDocument::Compact).size()>2*1024*1024)return false;
    const int version=root.value("version").toInt();
    if((version<1 || version>38) || !root.value("version").isDouble() || root.value("version").toDouble()!=version || rows.size()<4 || rows.size()>1024)return false;
    QJsonObject organization;QStringList organizationIds,organizationOrder;
    for(const auto &row:rows)organizationIds.append(row.toObject().value("id").toString());
    if(root.contains("organization") && (version<20 || !root.value("organization").isObject() || !BrushOrganization::normalize(root.value("organization").toObject(),organizationIds,organization,organizationOrder)))return false;
    const QSet<QString> builtins{"round-pressure","round-fine","round-translucent","round-wide"};QSet<QString> seen;QList<Preset> restored;
    for(const auto &row:rows){const auto p=row.toObject();Preset b;b.id=p.value("id").toString();b.name=p.value("name").toString();b.radius=p.value("radius").toDouble(std::numeric_limits<double>::quiet_NaN());b.opacity=p.value("opacity").toDouble(std::numeric_limits<double>::quiet_NaN());b.spacing=p.value("spacing").toDouble(std::numeric_limits<double>::quiet_NaN());
        if(b.id.isEmpty() || b.id.size()>64 || seen.contains(b.id) || b.name.trimmed().isEmpty() || b.name.size()>80 || !std::isfinite(b.radius) || b.radius<.5 || b.radius>1000 || !std::isfinite(b.opacity) || b.opacity<0 || b.opacity>1 || !std::isfinite(b.spacing) || b.spacing<.02 || b.spacing>1)return false;
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
        if(p.contains("scatterDynamics")){if(version<8 || !p.value("scatterDynamics").isObject() || !validScatter(p.value("scatterDynamics").toObject()) || (version<35 && scatterHasControls(p.value("scatterDynamics").toObject().toVariantMap())))return false;b.scatter=p.value("scatterDynamics").toObject().toVariantMap();if(b.scatter==defaultScatter())b.scatter.clear();}
        if(p.contains("shapeJitter")){if(version<9 || !p.value("shapeJitter").isObject() || !validShape(p.value("shapeJitter").toObject()))return false;b.shape=p.value("shapeJitter").toObject().toVariantMap();if(b.shape==defaultShape())b.shape.clear();}
        if(p.contains("transferJitter")){if(version<37 || !p.value("transferJitter").isObject() || !validTransferJitter(p.value("transferJitter").toObject()))return false;b.transfer=canonicalTransferJitter(p.value("transferJitter").toObject().toVariantMap());}
        if(p.contains("colorDynamics")){if(version<10 || !p.value("colorDynamics").isObject() || !validColorDynamics(p.value("colorDynamics").toObject()))return false;b.color=p.value("colorDynamics").toObject().toVariantMap();if(b.color==defaultColorDynamics())b.color.clear();}
        if(p.contains("texture")){if(version<11 || !p.value("texture").isObject())return false;auto texture=p.value("texture").toObject();texture["mode"]=normalizeTextureMode(texture.contains("mode")?texture.value("mode").toString():"normal");if(!validTexture(texture))return false;if((texture.contains("brightness") || texture.contains("contrast") || texture.contains("minimumDepth") || texture.contains("depthJitter") || texture.contains("depthPressure")) && version<21)return false;if(advancedTextureModes().contains(texture.value("mode").toString()) && version<30)return false;b.texture=canonicalTexture(texture.toVariantMap());if(QJsonObject::fromVariantMap(b.texture)==QJsonObject::fromVariantMap(defaultTexture()))b.texture.clear();}
        if(p.contains("dualBrush")){auto dual=p.value("dualBrush").toObject();dual["mode"]=normalizeDualMode(dual.value("mode").toString());dual.remove("color");if(version<12 || (version<15 && dual.contains("sizing")) || (version<38 && dualHasRatio(dual.toVariantMap())) || (version<35 && scatterHasControls(dual.value("scatterDynamics").toObject().toVariantMap())) || (version<36 && dualHasExtendedMode(dual.toVariantMap())) || !p.value("dualBrush").isObject() || !validDual(dual))return false;b.dual=canonicalDual(dual.toVariantMap());}
        if(p.contains("smoothing")){if(version<14 || !p.value("smoothing").isObject() || !validSmoothing(p.value("smoothing").toObject()))return false;b.smoothing=p.value("smoothing").toObject().toVariantMap();if(b.smoothing==defaultSmoothing())b.smoothing.clear();}
        if(p.contains("wetMix")){auto wet=p.value("wetMix").toObject();wet.remove("dualColor");if(version<17 || !p.value("wetMix").isObject() || !validWetMix(wet))return false;b.wet=canonicalWetMix(wet.toVariantMap());}
        if(p.contains("strokeTaper")){if(version<18 || !p.value("strokeTaper").isObject() || !validTaper(p.value("strokeTaper").toObject()))return false;b.taper=p.value("strokeTaper").toObject().toVariantMap();if(b.taper==defaultTaper())b.taper.clear();}
        if(p.contains("airbrush")){if(version<23 || !p.value("airbrush").isObject() || !validAirbrush(p.value("airbrush").toObject()))return false;b.airbrush=canonicalAirbrush(p.value("airbrush").toObject().toVariantMap());}
        if(p.contains("speedDynamics")){if(version<24 || !p.value("speedDynamics").isObject() || !validSpeedDynamics(p.value("speedDynamics").toObject()))return false;b.speed=canonicalSpeedDynamics(p.value("speedDynamics").toObject().toVariantMap());}
        if(p.contains("tangentDynamics")){if(version<25 || !p.value("tangentDynamics").isObject() || !validTangentDynamics(p.value("tangentDynamics").toObject()))return false;b.tangent=canonicalTangentDynamics(p.value("tangentDynamics").toObject().toVariantMap());}
        if(p.contains("distanceDynamics")){if(version<26 || !p.value("distanceDynamics").isObject() || !validDistanceDynamics(p.value("distanceDynamics").toObject()))return false;b.distance=canonicalDistanceDynamics(p.value("distanceDynamics").toObject().toVariantMap());}
        if(p.contains("tiltDynamics")){if(version<27 || !p.value("tiltDynamics").isObject() || !validTiltDynamics(p.value("tiltDynamics").toObject()))return false;b.tilt=canonicalTiltDynamics(p.value("tiltDynamics").toObject().toVariantMap());}
        if(p.contains("rotationDynamics")){if(version<28 || !p.value("rotationDynamics").isObject() || !validRotationDynamics(p.value("rotationDynamics").toObject()))return false;b.rotation=canonicalRotationDynamics(p.value("rotationDynamics").toObject().toVariantMap());}
        if(p.contains("directionDynamics")){if(version<29 || !p.value("directionDynamics").isObject() || !validDirectionDynamics(p.value("directionDynamics").toObject()))return false;b.direction=canonicalDirectionDynamics(p.value("directionDynamics").toObject().toVariantMap());}
        if(p.contains("sourceMatrix")){if(version<32 || !p.value("sourceMatrix").isObject() || !validDynamicsSourceMatrix(p.value("sourceMatrix").toObject()))return false;b.matrix=canonicalDynamicsSourceMatrix(p.value("sourceMatrix").toObject().toVariantMap());}
        if(p.contains("fadeDynamics")){if(version<34 || !p.value("fadeDynamics").isObject() || !validFadeDynamics(p.value("fadeDynamics").toObject()))return false;b.fade=canonicalFadeDynamics(p.value("fadeDynamics").toObject().toVariantMap());}
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
        if(!std::isfinite(brush) || !std::isfinite(eraser) || brush<.5 || brush>1000 || eraser<.5 || eraser>1000)return false;
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
    QColor foreground=m_foregroundColor,background=m_backgroundColor;
    if(root.contains("colorState")) {
        if(!root.value("colorState").isObject())return false;
        const auto colors=root.value("colorState").toObject();
        if(colors.size()!=2 || !colors.value("foreground").isString() || !colors.value("background").isString())return false;
        foreground=QColor(colors.value("foreground").toString());background=QColor(colors.value("background").toString());
        if(!foreground.isValid() || !background.isValid())return false;
    }
    bool outline=true,center=false,hoverOnly=false,simple=false,composite=false,image=false;int width=0,imageOpacity=25;
    QString color="#000000",secondaryColor="#ffffff";
    if(root.contains("cursor")){
        if(!root.value("cursor").isObject())return false;
        const auto cursor=root.value("cursor").toObject();
        for(const auto &key:{"outline","center","hoverOnly"})if(!cursor.value(key).isBool())return false;
        const double line=cursor.value("width").toDouble(0);
        if(!cursor.value("width").isDouble() || !std::isfinite(line) || line<0 || line>3 || std::floor(line)!=line)return false;
        outline=cursor.value("outline").toBool();center=cursor.value("center").toBool();hoverOnly=cursor.value("hoverOnly").toBool();width=int(line);
        if(cursor.contains("image")){if(!cursor.value("image").isBool())return false;image=cursor.value("image").toBool();}
        if(cursor.contains("imageOpacity")){const auto v=cursor.value("imageOpacity");const auto number=v.toDouble(0);if(!v.isDouble() || !std::isfinite(number) || number<1 || number>100 || std::floor(number)!=number)return false;imageOpacity=int(number);}
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
        auto previewSettings=[](QJsonObject row){for(const auto &key:{"name","radius","opacity","flow"})row.remove(key);return row;};
        if(current>=0 && equivalentSettings(previewSettings(presetRow(m_presets[current],oldVersion)),previewSettings(presetRow(preset,newVersion)))) {
            const auto &old=m_presets[current];preset.preview=old.preview;preset.error=old.error;preset.token=old.token;preset.requested=old.requested;
        } else preset.token=++m_nextToken;
    }
    m_presets=restored;m_organization=organization;m_selected=selected;m_recentColors=recent;m_writeBlend=writeBlend;
    m_selectedItems=QJsonArray{QJsonObject{{"kind","preset"},{"id",selected}}};
    const bool colorsChanged=foreground!=m_foregroundColor || background!=m_backgroundColor;
    m_foregroundColor=foreground;m_backgroundColor=background;
    if(colorsChanged)emit activeColorsChanged();
    m_cursorOutline=outline;m_cursorCenter=center;m_cursorHoverOnly=hoverOnly;m_cursorWidth=width;m_cursorSimple=simple;m_cursorComposite=composite;m_cursorImage=image;m_cursorImageOpacity=imageOpacity;m_cursorColor=color;m_cursorSecondaryColor=secondaryColor;emit cursorChanged();
    emit settingsChanged();emit presetsChanged();emit selectedPreviewChanged();emit selectionChanged();emit recentColorsChanged();return true;
}
