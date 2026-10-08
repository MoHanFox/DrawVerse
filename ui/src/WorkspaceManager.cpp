#include "WorkspaceManager.h"
#include <QGuiApplication>
#include <QScreen>
#include <QCursor>
#include <QDrag>
#include <QMimeData>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QSettings>
#include <QUuid>
#include <QKeyEvent>
#include <QSet>
#include <algorithm>

namespace { QString newId() { return QUuid::createUuid().toString(QUuid::WithoutBraces); } }
WorkspaceManager::WorkspaceManager(const QString &settingsFile, QObject *parent) : QObject(parent), m_settingsFile(settingsFile) {
    resetLayout(); restoreLayout();
}
void WorkspaceManager::resetLayout() {
    m_panels.clear();
    const QStringList ids{"color","brush","layers","history","navigator"};
    const QStringList titles{QStringLiteral("颜色"),QStringLiteral("画笔"),QStringLiteral("图层"),QStringLiteral("历史"),QStringLiteral("导航")};
    for (int i=0; i<ids.size(); ++i) m_panels.insert(ids[i], {{"title",titles[i]},{"kind",ids[i]}});
    m_groups = {{newId(),"right",{"color","brush"},"color",{100,100,320,360}},
                {newId(),"right",{"layers","history","navigator"},"layers",{160,160,320,440}}};
    emit groupsChanged();
}
int WorkspaceManager::index(const QString &id) const { for (int i=0;i<m_groups.size();++i) if(m_groups[i].id==id) return i; return -1; }
QVariantList WorkspaceManager::groupsAt(const QString &location) const {
    QVariantList result;
    for (const auto &g:m_groups) if(g.location==location)
        result.append(QVariantMap{{"id",g.id},{"location",g.location},{"panels",g.panels},{"active",g.active},
                                  {"x",g.geometry.x()},{"y",g.geometry.y()},{"width",g.geometry.width()},{"height",g.geometry.height()}});
    return result;
}
QVariantMap WorkspaceManager::panelDefinition(const QString &id) const { return m_panels.value(id); }
void WorkspaceManager::setActive(const QString &group,const QString &panel) {
    const int i=index(group); if(i>=0 && m_groups[i].panels.contains(panel)) m_groups[i].active=panel;
    // Tab changes never rebuild a floating window or interrupt a text editor.
}
QString WorkspaceManager::payload(const QString &group,const QString &panel,bool whole) const {
    return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",group},{"panel",panel},{"whole",whole}}).toJson(QJsonDocument::Compact));
}
bool WorkspaceManager::dockPayload(const QString &data,const QString &location,const QString &target) {
    if(data.size()>1024 || (location!="left" && location!="right" && location!="floating")) return false;
    const auto object=QJsonDocument::fromJson(data.toUtf8()).object();
    const QString source=object.value("group").toString(), panel=object.value("panel").toString();
    const int from=index(source), to=index(target);
    if(from<0 || (!target.isEmpty() && to<0) || source==target) return false;
    const auto original=m_groups[from];
    const bool whole=object.value("whole").toBool(false);
    if(!whole && !original.panels.contains(panel)) return false;
    const QStringList moving=whole?original.panels:QStringList{panel};
    if(to>=0) {
        m_groups[to].panels.append(moving); m_groups[to].active=moving.first();
    } else {
        QRect geometry=original.geometry;
        if(location=="floating") geometry=safeGeometry(QRect(QCursor::pos(),geometry.size()));
        m_groups.append({newId(),location,moving,moving.first(),geometry});
    }
    for(const auto &id:moving) m_groups[from].panels.removeAll(id);
    if(m_groups[from].panels.isEmpty()) m_groups.removeAt(from);
    else if(!m_groups[from].panels.contains(m_groups[from].active)) m_groups[from].active=m_groups[from].panels.first();
    emit groupsChanged(); return true;
}
void WorkspaceManager::beginDrag(const QString &group,const QString &panel,bool whole) {
    if(index(group)<0) return;
    const QString data=payload(group,panel,whole);
    // Native Qt drag-and-drop crosses QQuickWindow boundaries without UI-thread model work.
    QDrag drag(this); auto *mime=new QMimeData;
    mime->setData("application/x-drawverse-panel",data.toUtf8()); drag.setMimeData(mime);
    m_dragCancelled=false; qApp->installEventFilter(this);
    const auto action=drag.exec(Qt::MoveAction); qApp->removeEventFilter(this);
    if(action==Qt::IgnoreAction && !m_dragCancelled) dockPayload(data,"floating");
}
bool WorkspaceManager::eventFilter(QObject *,QEvent *event) {
    if(event->type()==QEvent::KeyPress && static_cast<QKeyEvent*>(event)->key()==Qt::Key_Escape) m_dragCancelled=true;
    return false;
}
void WorkspaceManager::detachGroup(const QString &group) { dockPayload(payload(group,{},true),"floating"); }
void WorkspaceManager::detachPanel(const QString &group,const QString &panel) { dockPayload(payload(group,panel,false),"floating"); }
void WorkspaceManager::returnGroup(const QString &group) {
    const int i=index(group); if(i<0) return;
    m_groups[i].location="right"; emit groupsChanged();
}
QString WorkspaceManager::addCustomPanel(const QString &title,const QString &kind) {
    if(m_panels.size()>=64 || (kind!="palette" && kind!="brush")) return {};
    const QString id=newId(); const QString name=title.trimmed().left(80);
    m_panels.insert(id,{{"title",name.isEmpty()?QStringLiteral("自定义面板"):name},{"kind",kind},{"custom",true},{"content",QString()}});
    // Add a tab rather than an unbounded number of dock columns.
    for(auto &g:m_groups) if(g.location=="right") { g.panels.append(id); g.active=id; emit groupsChanged(); return id; }
    m_groups.append({newId(),"right",{id},id,{100,100,320,440}}); emit groupsChanged(); return id;
}
QRect WorkspaceManager::safeGeometry(QRect rect) const {
    rect.setWidth(std::clamp(rect.width(),240,1200)); rect.setHeight(std::clamp(rect.height(),200,1200));
    QScreen *screen=QGuiApplication::screenAt(rect.center()); if(!screen) screen=QGuiApplication::primaryScreen();
    if(screen) {
        const QRect available=screen->availableGeometry();
        rect.setSize(rect.size().boundedTo(available.size()));
        rect.moveLeft(std::clamp(rect.x(),available.left(),available.right()-rect.width()+1));
        rect.moveTop(std::clamp(rect.y(),available.top(),available.bottom()-rect.height()+1));
    }
    return rect;
}
void WorkspaceManager::updateGeometry(const QString &id,int x,int y,int w,int h) {
    const int i=index(id); if(i>=0) m_groups[i].geometry=QRect(x,y,std::clamp(w,240,1200),std::clamp(h,200,1200));
}
void WorkspaceManager::saveLayout() const {
    QJsonArray groups;
    for(const auto &g:m_groups) groups.append(QJsonObject{{"id",g.id},{"location",g.location},{"panels",QJsonArray::fromStringList(g.panels)},
                                                        {"active",g.active},{"x",g.geometry.x()},{"y",g.geometry.y()},{"width",g.geometry.width()},{"height",g.geometry.height()}});
    QJsonObject panels; for(auto it=m_panels.cbegin();it!=m_panels.cend();++it) panels.insert(it.key(),QJsonObject::fromVariantMap(it.value()));
    QSettings settings(m_settingsFile.isEmpty()?QSettings::NativeFormat:QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse");
    if(!m_settingsFile.isEmpty()) { QSettings file(m_settingsFile,QSettings::IniFormat); file.setValue("workspace",QJsonDocument(QJsonObject{{"version",2},{"groups",groups},{"panels",panels}}).toJson(QJsonDocument::Compact)); }
    else settings.setValue("workspace",QJsonDocument(QJsonObject{{"version",2},{"groups",groups},{"panels",panels}}).toJson(QJsonDocument::Compact));
}
bool WorkspaceManager::restoreLayout() {
    QSettings settings(QSettings::NativeFormat,QSettings::UserScope,"DrawVerse","DrawVerse");
    QByteArray bytes;
    if(m_settingsFile.isEmpty()) bytes=settings.value("workspace").toByteArray();
    else { QSettings file(m_settingsFile,QSettings::IniFormat); bytes=file.value("workspace").toByteArray(); }
    if(bytes.isEmpty() || bytes.size()>5*1024*1024) return false;
    const auto root=QJsonDocument::fromJson(bytes).object();
    const auto panels=root.value("panels").toObject(); const auto groups=root.value("groups").toArray();
    if(!QList<int>{1,2}.contains(root.value("version").toInt()) || panels.size()>64 || panels.size()<5 || groups.isEmpty() || groups.size()>64) return false;
    QHash<QString,QVariantMap> restoredPanels;
    for(auto it=panels.begin();it!=panels.end();++it) {
        const auto p=it.value().toObject().toVariantMap(); const auto kind=p.value("kind").toString();
        if(it.key().size()>128 || p.value("title").toString().size()>80 || p.value("content").toString().size()>65536 ||
           !QStringList{"color","brush","layers","history","navigator","notes","palette"}.contains(kind)) return false;
        restoredPanels.insert(it.key(),p);
    }
    for(const auto &id:QStringList{"color","brush","layers","history","navigator"}) if(!restoredPanels.contains(id) || restoredPanels[id].value("kind")!=id) return false;
    QList<Group> restored; QSet<QString> seen,groupIds;
    for(const auto &entry:groups) {
        const auto object=entry.toObject(); Group g;
        g.id=object.value("id").toString(); g.location=object.value("location").toString(); g.active=object.value("active").toString();
        if(g.id.isEmpty() || g.id.size()>128 || groupIds.contains(g.id) || !QStringList{"left","right","floating"}.contains(g.location)) return false;
        groupIds.insert(g.id);
        for(const auto &p:object.value("panels").toArray()) {
            const auto id=p.toString(); if(!restoredPanels.contains(id) || seen.contains(id)) return false;
            seen.insert(id); g.panels.append(id);
        }
        if(g.panels.isEmpty() || !g.panels.contains(g.active)) return false;
        g.geometry=safeGeometry({object.value("x").toInt(),object.value("y").toInt(),object.value("width").toInt(320),object.value("height").toInt(440)});
        restored.append(g);
    }
    if(seen.size()!=restoredPanels.size()) return false;
    // Validate the entire old layout first, then remove retired notes without
    // resetting unrelated panel groups or their floating window geometry.
    bool migrated=root.value("version").toInt()!=2;
    for(auto it=restoredPanels.begin();it!=restoredPanels.end();) {
        if(it.value().value("kind")=="notes") { it=restoredPanels.erase(it); migrated=true; }
        else ++it;
    }
    for(auto it=restored.begin();it!=restored.end();) {
        auto &g=*it;
        g.panels.removeIf([&](const QString &id) { return !restoredPanels.contains(id); });
        if(g.panels.isEmpty()) { it=restored.erase(it); continue; }
        if(!g.panels.contains(g.active)) g.active=g.panels.first();
        ++it;
    }
    m_panels=restoredPanels; m_groups=restored;
    if(migrated) saveLayout();
    emit groupsChanged(); return true;
}
