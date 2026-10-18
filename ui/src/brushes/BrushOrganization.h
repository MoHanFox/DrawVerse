#pragma once
#include <QJsonArray>
#include <QJsonObject>
#include <QMap>
#include <QSet>
#include <QRegularExpression>

namespace BrushOrganization {
inline bool uuid(const QString &id) {
    static const QRegularExpression pattern("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$");
    return pattern.match(id).hasMatch() && id!="00000000-0000-0000-0000-000000000000";
}
inline bool normalize(const QJsonObject &input,const QStringList &presets,QJsonObject &output,QStringList &order,bool exact=true) {
    if(input.size()!=3 || !input.value("model").isString() || input.value("model").toString()!="ordered-tree-v1" || !input.value("roots").isArray() || !input.value("groups").isArray())return false;
    const auto roots=input.value("roots").toArray(), groups=input.value("groups").toArray();
    if(groups.size()>256 || roots.size()>1280 || presets.size()>1024)return false;
    QMap<QString,QJsonObject> records;qsizetype count=roots.size();
    for(const auto &value:groups) {
        if(!value.isObject())return false;auto group=value.toObject();
        if(group.size()!=3 || !group.value("id").isString() || !group.value("name").isString() || !group.value("items").isArray())return false;
        const auto id=group.value("id").toString(), name=group.value("name").toString().trimmed();
        if(!uuid(id) || records.contains(id) || name.isEmpty() || name.contains(QChar(0)) || name.toUcs4().size()>80)return false;
        count+=group.value("items").toArray().size();if(count>1280)return false;
        group["name"]=name;records.insert(id,group);
    }
    QSet<QString> expected(presets.begin(),presets.end()),seenPresets,seenGroups;
    if(expected.size()!=presets.size())return false;
    const QSet<QString> builtins{"round-pressure","round-fine","round-translucent","round-wide"};
    QList<QPair<QJsonValue,int>> pending;for(qsizetype i=roots.size();i>0;--i)pending.append({roots[i-1],0});
    order.clear();
    while(!pending.isEmpty()) {
        const auto current=pending.takeLast();if(!current.first.isObject())return false;const auto item=current.first.toObject();
        if(item.size()!=2 || !item.value("kind").isString() || !item.value("id").isString())return false;
        const auto kind=item.value("kind").toString(),id=item.value("id").toString();
        if(kind=="preset") {
            if((!uuid(id) && !builtins.contains(id)) || records.contains(id) || !expected.contains(id) || seenPresets.contains(id))return false;
            seenPresets.insert(id);order.append(id);
        } else if(kind=="group") {
            if(current.second>=8 || !records.contains(id) || seenGroups.contains(id))return false;
            seenGroups.insert(id);const auto items=records[id].value("items").toArray();
            for(qsizetype i=items.size();i>0;--i)pending.append({items[i-1],current.second+1});
        } else return false;
    }
    if(seenPresets!=expected || seenGroups.size()!=records.size() || (exact && order!=presets))return false;
    QJsonArray canonical;for(const auto &group:records)canonical.append(group);
    output=records.isEmpty()?QJsonObject{}:QJsonObject{{"model","ordered-tree-v1"},{"roots",roots},{"groups",canonical}};return true;
}
inline QJsonObject presetItem(const QString &id) {return {{"kind","preset"},{"id",id}};}
// Copy appends to the source's immediate group; ordinary imports append to roots.
inline QJsonObject addPreset(QJsonObject organization,const QString &id,const QString &source={}) {
    if(organization.isEmpty())return organization;
    auto groups=organization.value("groups").toArray();
    for(qsizetype i=0;i<groups.size();++i) {
        auto group=groups[i].toObject();auto items=group.value("items").toArray();
        if(!source.isEmpty() && items.contains(presetItem(source))) {
            items.append(presetItem(id));group["items"]=items;groups[i]=group;organization["groups"]=groups;return organization;
        }
    }
    auto roots=organization.value("roots").toArray();roots.append(presetItem(id));organization["roots"]=roots;return organization;
}
inline QJsonObject removePreset(QJsonObject organization,const QString &id) {
    if(organization.isEmpty())return organization;
    const auto strip=[&](QJsonArray items){for(qsizetype i=items.size();i>0;--i)if(items[i-1]==presetItem(id))items.removeAt(i-1);return items;};
    organization["roots"]=strip(organization.value("roots").toArray());
    auto groups=organization.value("groups").toArray();for(qsizetype i=0;i<groups.size();++i){auto group=groups[i].toObject();group["items"]=strip(group.value("items").toArray());groups[i]=group;}organization["groups"]=groups;return organization;
}
}
