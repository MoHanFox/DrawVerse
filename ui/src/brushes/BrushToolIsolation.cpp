#include "brushes/BrushLibrary.h"
#include <QJsonDocument>

void BrushLibrary::captureToolProfile() {
    auto &profile=m_toolProfiles[m_toolContext];
    profile.presets.clear();
    for(const auto &p:m_presets)profile.presets.insert(p.id,p);
    profile.selected=m_selected;profile.writeBlend=m_writeBlend;
}

void BrushLibrary::setToolIsolation(int mode) {
    if(mode<0 || mode>2 || mode==m_toolIsolation)return;
    m_toolIsolation=mode;
    m_toolProfiles={};
    if(mode) {
        captureToolProfile();
        const auto initial=m_toolProfiles[m_toolContext];
        for(auto &profile:m_toolProfiles)profile=initial;
    }
    changed();
}

void BrushLibrary::setToolContext(int tool,bool persist) {
    if(tool<0 || tool>2 || tool==m_toolContext)return;
    if(m_toolIsolation)captureToolProfile();
    m_toolContext=tool;
    if(!m_toolIsolation) {
        m_snapshotCache.clear();
        if(!m_snapshotObjectCache.isEmpty()) {
            auto isolation=m_snapshotObjectCache.value("toolIsolation").toObject();isolation["context"]=tool;
            m_snapshotObjectCache["toolIsolation"]=isolation;
        }
        return;
    }
    const auto &profile=m_toolProfiles[tool];
    for(auto &p:m_presets) {
        const auto found=profile.presets.constFind(p.id);
        if(found==profile.presets.cend())continue;
        if(m_toolIsolation==1) {
            p.radius=found->radius;p.opacity=found->opacity;p.flow=found->flow;
        } else {
            auto next=*found;
            // Catalog identity, naming and import provenance belong to the shared library.
            next.id=p.id;next.name=p.name;next.builtin=p.builtin;next.sourceResource=p.sourceResource;
            int oldVersion=2,newVersion=2;
            auto oldRow=presetRow(p,oldVersion),newRow=presetRow(next,newVersion);
            for(const auto &key:{"name","radius","opacity","flow"}){oldRow.remove(key);newRow.remove(key);}
            if(oldRow==newRow) {
                next.preview=p.preview;next.error=p.error;next.token=p.token;next.requested=p.requested;
            } else {
                next.preview.clear();next.error.clear();next.token=++m_nextToken;next.requested=0;
            }
            p=std::move(next);
        }
    }
    if(m_toolIsolation==2) {
        m_selected=index(profile.selected)>=0?profile.selected:QStringLiteral("round-pressure");
        m_writeBlend=profile.writeBlend;
        m_selectedItems=QJsonArray{QJsonObject{{"kind","preset"},{"id",m_selected}}};
        emit selectionChanged();
    }
    if(persist)changed(true);
    else {
        m_snapshotCache.clear();m_catalogCache.clear();m_snapshotObjectCache={};m_catalogObjectCache={};
        emit settingsChanged();emit presetsChanged();emit selectedPreviewChanged();
    }
}

QJsonObject BrushLibrary::toolIsolationState() const {
    QJsonArray profiles;
    if(m_toolIsolation)for(int tool=0;tool<3;++tool) {
        const auto &profile=m_toolProfiles[tool];
        QJsonArray rows;QJsonObject editors;int version=2;
        for(const auto &current:m_presets) {
            auto p=tool==m_toolContext?current:profile.presets.value(current.id,current);
            p.name=current.name;p.sourceResource=current.sourceResource;
            rows.append(presetRow(p,version));
            if(p.dualExisting || !p.dualSourceName.isEmpty())editors[p.id]=QJsonObject{{"existing",p.dualExisting},{"sourceName",p.dualSourceName}};
        }
        auto selected=tool==m_toolContext?m_selected:profile.selected;
        if(index(selected)<0)selected="round-pressure";
        QJsonObject state{{"version",version},{"selected",selected},{"presets",rows},
                          {"brushTool",QJsonObject{{"writeBlend",tool==m_toolContext?m_writeBlend:profile.writeBlend}}}};
        if(!editors.isEmpty())state["dualEditors"]=editors;
        profiles.append(state);
    }
    return {{"version",1},{"mode",m_toolIsolation},{"context",m_toolContext},{"profiles",profiles}};
}

bool BrushLibrary::parseToolIsolation(const QJsonValue &value,int &mode,int &context,std::array<ToolProfile,3> &profiles,RestoreRows &decoded) {
    mode=0;context=0;
    if(value.isUndefined())return true;
    if(!value.isObject())return false;
    const auto state=value.toObject();
    if(state.size()!=4 || state.value("version")!=QJsonValue(1) || !state.value("profiles").isArray())return false;
    const auto m=state.value("mode"),c=state.value("context");
    if(!m.isDouble() || !c.isDouble() || m.toDouble()!=m.toInt(-1) || c.toDouble()!=c.toInt(-1) || m.toInt()<0 || m.toInt()>2 || c.toInt()<0 || c.toInt()>2)return false;
    mode=m.toInt();context=c.toInt();const auto rows=state.value("profiles").toArray();
    if(rows.size()!=(mode?3:0))return false;
    for(int tool=0;tool<rows.size();++tool) {
        if(!rows[tool].isObject())return false;
        const auto row=rows[tool].toObject();
        for(auto it=row.begin();it!=row.end();++it)if(it.key()!="version" && it.key()!="selected" && it.key()!="presets" && it.key()!="dualEditors" && it.key()!="brushTool")return false;
        const auto json=QJsonDocument(row).toJson(QJsonDocument::Compact);
        if(json.size()>2*1024*1024)return false;
        BrushLibrary validator;
        if(!validator.restoreObject(row,false,decoded,true))return false;
        auto &profile=profiles[tool];profile.selected=validator.m_selected;profile.writeBlend=validator.m_writeBlend;
        for(const auto &p:validator.m_presets)profile.presets.insert(p.id,p);
    }
    return true;
}
