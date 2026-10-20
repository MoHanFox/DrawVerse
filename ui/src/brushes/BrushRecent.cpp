#include "brushes/BrushLibrary.h"
#include "brushes/BrushOrganization.h"
#include <QJsonDocument>
#include <QUuid>

QByteArray BrushLibrary::captureBrushSnapshot() const {
    const auto &p=m_presets[index(m_selected)];int version=2;
    const auto row=presetRow(p,version);
    return QJsonDocument(QJsonObject{{"version",version},
        {"preset",row},{"writeBlend",m_writeBlend},
        {"dualEditor",QJsonObject{{"existing",p.dualExisting},{"sourceName",p.dualSourceName}}}})
        .toJson(QJsonDocument::Compact);
}

bool BrushLibrary::decodeBrushSnapshot(const QJsonObject &state,Preset &preset,int &blend) {
    if(state.size()!=4 || !state.value("preset").isObject() || !state.value("dualEditor").isObject())return false;
    // Only a single preset can enter this wrapper; catalog/reset/preference recursion is impossible.
    BrushLibrary validator;auto catalog=QJsonDocument::fromJson(validator.catalogSnapshot()).object();
    auto rows=catalog.value("presets").toArray();const auto p=state.value("preset").toObject();const auto id=p.value("id").toString();bool replaced=false;
    for(int i=0;i<rows.size();++i)if(rows[i].toObject().value("id").toString()==id){rows[i]=p;replaced=true;break;}
    if(!replaced)rows.append(p);
    catalog["version"]=state.value("version");catalog["presets"]=rows;catalog["selected"]=id;
    catalog["dualEditors"]=QJsonObject{{id,state.value("dualEditor")}};
    catalog["brushTool"]=QJsonObject{{"writeBlend",state.value("writeBlend")}};
    if(!validator.restore(QJsonDocument(catalog).toJson(QJsonDocument::Compact)))return false;
    preset=validator.m_presets[validator.index(id)];blend=validator.m_writeBlend;return true;
}

bool BrushLibrary::recordRecentBrush(const QByteArray &snapshot,bool persist) {
    if(snapshot.size()>2*1024*1024)return false;
    Preset p;int blend=0;if(!decodeBrushSnapshot(QJsonDocument::fromJson(snapshot).object(),p,blend))return false;
    if(!m_recentBrushSnapshots.isEmpty() && sameBrushSnapshot(snapshot,m_recentBrushSnapshots.first()))return true;
    for(int i=m_recentBrushSnapshots.size()-1;i>=0;--i)if(sameBrushSnapshot(snapshot,m_recentBrushSnapshots[i]))m_recentBrushSnapshots.removeAt(i);
    m_recentBrushSnapshots.prepend(snapshot);while(m_recentBrushSnapshots.size()>8)m_recentBrushSnapshots.removeLast();
    if(persist)queuePersistence(false);else m_snapshotCache.clear();
    emit recentBrushesChanged();if(persist)emit recentBrushRecorded(snapshot);return true;
}

QVariantList BrushLibrary::recentBrushes() const {
    QVariantList result;
    for(const auto &snapshot:m_recentBrushSnapshots) {
        const auto p=QJsonDocument::fromJson(snapshot).object().value("preset").toObject();
        result.append(QVariantMap{{"id",p.value("id").toString()},{"name",p.value("name").toString()},
            {"diameter",p.value("radius").toDouble()*2},{"resource",p.value("maskResource").toString()},
            {"hardness",p.value("hardness").toDouble()},{"roundness",p.value("roundness").toDouble()},{"angle",p.value("angle").toDouble()}});
    }
    return result;
}

bool BrushLibrary::applyRecentBrush(int at) {
    if(at<0 || at>=m_recentBrushSnapshots.size())return false;
    Preset p;int blend=0;if(!decodeBrushSnapshot(QJsonDocument::fromJson(m_recentBrushSnapshots[at]).object(),p,blend))return false;
    int target=index(p.id);
    if(target<0) {
        if(m_presets.size()>=1024)return false;
        p.id=QUuid::createUuid().toString(QUuid::WithoutBraces);p.builtin=false;
        m_organization=BrushOrganization::addPreset(m_organization,p.id);
        m_resetPresets.insert(p.id,p);target=m_presets.size();m_presets.append(p);
    } else {
        p.name=m_presets[target].name;p.builtin=m_presets[target].builtin;p.sourceResource=m_presets[target].sourceResource;
    }
    p.token=++m_nextToken;p.requested=0;p.preview.clear();p.error.clear();m_presets[target]=p;
    m_selected=p.id;m_writeBlend=blend;m_selectedItems=QJsonArray{QJsonObject{{"kind","preset"},{"id",p.id}}};
    emit selectionChanged();changed(true);return true;
}

QJsonObject BrushLibrary::recentBrushState() const {
    QJsonArray items;for(const auto &snapshot:m_recentBrushSnapshots)items.append(QJsonDocument::fromJson(snapshot).object());
    return {{"version",1},{"items",items}};
}

bool BrushLibrary::parseRecentBrushes(const QJsonValue &value,QList<QByteArray> &snapshots) {
    if(value.isUndefined())return true;
    if(!value.isObject())return false;const auto root=value.toObject();
    if(root.size()!=2 || root.value("version")!=QJsonValue(1) || !root.value("items").isArray())return false;
    const auto items=root.value("items").toArray();if(items.size()>8)return false;
    for(const auto &item:items) {
        if(!item.isObject())return false;const auto bytes=QJsonDocument(item.toObject()).toJson(QJsonDocument::Compact);
        Preset p;int blend=0;if(bytes.size()>2*1024*1024 || !decodeBrushSnapshot(item.toObject(),p,blend))return false;
        snapshots.append(bytes);
    }
    return true;
}
