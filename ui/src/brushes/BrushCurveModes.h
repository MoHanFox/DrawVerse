#pragma once
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>

namespace BrushCurveModes {
inline bool present(const QJsonValue &value) {
    if(value.isObject()) {
        const auto object=value.toObject();
        if(object.contains("nodeModes"))return true;
        for(auto it=object.begin();it!=object.end();++it)if(present(it.value()))return true;
    } else if(value.isArray()) {
        for(const auto &item:value.toArray())if(present(item))return true;
    }
    return false;
}
inline bool valid(const QJsonObject &control, const QString &smoothModel) {
    if(!control.contains("nodeModes"))return true;
    if(control.value("interpolation").toString()!=smoothModel || !control.value("nodeModes").isArray())return false;
    const auto modes=control.value("nodeModes").toArray();
    if(modes.size()<2 || modes.size()>17 || modes.size()!=control.value("curve").toArray().size())return false;
    for(const auto &mode:modes)if(!mode.isBool())return false;
    return true;
}
}
