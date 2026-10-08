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
#include <QPainter>
#include <QPixmap>
#include <QDropEvent>
#include <algorithm>
#ifdef Q_OS_WIN
#include <QtCore/qt_windows.h>
#endif

namespace { QString newId() { return QUuid::createUuid().toString(QUuid::WithoutBraces); } }
WorkspaceManager::WorkspaceManager(const QString &settingsFile, QObject *parent) : QObject(parent), m_settingsFile(settingsFile) {
    qApp->installEventFilter(this);
    resetLayout(); if(!restoreLayout()) m_uiRevision=0;
}
void WorkspaceManager::resetLayout() {
    m_panels.clear();
    const QStringList ids{"color","brush","layers","history","navigator","brush-settings"};
    const QStringList titles{QStringLiteral("颜色"),QStringLiteral("画笔"),QStringLiteral("图层"),QStringLiteral("历史记录"),QStringLiteral("导航"),QStringLiteral("画笔设置")};
    for (int i=0; i<ids.size(); ++i) m_panels.insert(ids[i], {{"title",titles[i]},{"kind",ids[i]}});
    m_groups = {{newId(),"right",{"color"},"color",{100,100,190,300},"right",false,300},
                {newId(),"right",{"layers","history","navigator"},"layers",{160,160,240,400},"right",false,460},
                {newId(),"left",{"brush","brush-settings"},"brush-settings",{100,100,180,420},"left",false,420}};
    m_leftCollapsed=false; m_rightCollapsed=false; m_leftWidth=180; m_rightWidth=190;
    m_uiRevision=2;
    m_toolsFloating=false;emit toolStripChanged();
    emit dockMetricsChanged();
    emit groupsChanged();
}
void WorkspaceManager::applyReferenceLayout(int x,int y,int width,int height) {
    const QStringList builtins{"color","brush","brush-settings","layers","history","navigator"};
    // Keep custom panels and their geometry while arranging the built-in workspace.
    for(auto it=m_groups.begin();it!=m_groups.end();) {
        it->panels.removeIf([&](const QString &id){return builtins.contains(id);});
        if(it->panels.isEmpty()){it=m_groups.erase(it);continue;}
        if(!it->panels.contains(it->active))it->active=it->panels.first();
        ++it;
    }
    m_groups.append({newId(),"right",{"history"},"history",{x+width-210,y+60,190,220},"right",false,qRound(height*.29)});
    m_groups.append({newId(),"right",{"layers","navigator"},"layers",{x+width-210,y+290,190,400},"right",false,qRound(height*.65)});
    m_groups.append({newId(),"floating",{"brush","brush-settings"},"brush-settings",safeGeometry({x+70,y+100,150,360}),"left",true,360});
    m_groups.append({newId(),"floating",{"color"},"color",safeGeometry({x+qRound(width*.18),y+70,155,235}),"right",false,235});
    m_leftCollapsed=false; m_rightCollapsed=false; m_leftWidth=180; m_rightWidth=190; m_uiRevision=2;
    m_toolsFloating=false;emit toolStripChanged();
    emit dockMetricsChanged(); emit groupsChanged();
}
int WorkspaceManager::index(const QString &id) const { for (int i=0;i<m_groups.size();++i) if(m_groups[i].id==id) return i; return -1; }
QVariantMap WorkspaceManager::groupDefinition(const QString &id) const {
    const int i=index(id); if(i<0) return {};
    const auto &g=m_groups[i];
    return {{"id",g.id},{"location",g.location},{"panels",g.panels},{"active",g.active},
            {"x",g.geometry.x()},{"y",g.geometry.y()},{"width",g.geometry.width()},{"height",g.geometry.height()},
            {"collapsed",g.collapsed},{"dockHeight",g.dockHeight}};
}
QVariantList WorkspaceManager::groupsAt(const QString &location) const {
    QVariantList result;
    for(const auto &g:m_groups) if(g.location==location) result.append(groupDefinition(g.id));
    return result;
}
QStringList WorkspaceManager::visiblePanels() const {
    QStringList result;
    for(const auto &g:m_groups) if(g.location!="hidden") result.append(g.panels);
    return result;
}
QStringList WorkspaceManager::allPanels() const {
    QStringList result{"brush","color","layers","history","navigator"};
    auto custom=m_panels.keys(); custom.sort();
    for(const auto &id:custom) if(!result.contains(id)) result.append(id);
    return result;
}
void WorkspaceManager::setLeftCollapsed(bool value) { if(m_leftCollapsed==value) return; m_leftCollapsed=value; emit dockMetricsChanged(); }
void WorkspaceManager::setRightCollapsed(bool value) { if(m_rightCollapsed==value) return; m_rightCollapsed=value; emit dockMetricsChanged(); }
void WorkspaceManager::setLeftDockWidth(int value) { value=std::clamp(value,150,520); if(value==m_leftWidth)return; m_leftWidth=value; emit dockMetricsChanged(); }
void WorkspaceManager::setRightDockWidth(int value) { value=std::clamp(value,190,520); if(value==m_rightWidth)return; m_rightWidth=value; emit dockMetricsChanged(); }
QVariantMap WorkspaceManager::panelDefinition(const QString &id) const { return m_panels.value(id); }
void WorkspaceManager::setActive(const QString &group,const QString &panel) {
    const int i=index(group); if(i<0 || !m_groups[i].panels.contains(panel) || m_groups[i].active==panel) return;
    m_groups[i].active=panel; emit groupStateChanged(group);
    // Selection and collapse never destroy floating windows or active controls.
}
void WorkspaceManager::setGroupCollapsed(const QString &group,bool collapsed) {
    const int i=index(group); if(i<0 || m_groups[i].collapsed==collapsed) return;
    m_groups[i].collapsed=collapsed; emit groupStateChanged(group);
}
void WorkspaceManager::updateDockHeight(const QString &group,int height) {
    const int i=index(group); if(i>=0 && !m_groups[i].collapsed && m_groups[i].location!="floating" && height>80)
        m_groups[i].dockHeight=std::clamp(height,100,2000);
}
QString WorkspaceManager::payload(const QString &group,const QString &panel,bool whole) const {
    return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",group},{"panel",panel},{"whole",whole}}).toJson(QJsonDocument::Compact));
}
bool WorkspaceManager::dockPayload(const QString &data,const QString &location,const QString &target,
                                   const QString &placement,const QString &beforePanel) {
    if(data.size()>1024 || !QStringList{"left","right","floating"}.contains(location) ||
       !QStringList{"merge","before","after"}.contains(placement)) return false;
    const auto object=QJsonDocument::fromJson(data.toUtf8()).object();
    const QString source=object.value("group").toString(), panel=object.value("panel").toString();
    int from=index(source), to=index(target);
    if(from<0 || (!target.isEmpty() && (to<0 || m_groups[to].location!=location))) return false;
    const auto original=m_groups[from];
    const bool whole=object.value("whole").toBool(false);
    if(!whole && !original.panels.contains(panel)) return false;
    if(!beforePanel.isEmpty() && (to<0 || placement!="merge" || !m_groups[to].panels.contains(beforePanel))) return false;
    const QStringList moving=whole?original.panels:QStringList{panel};
    if(source==target) {
        if(whole) return false;
        if(placement=="merge") {
            auto &g=m_groups[from];
            if(beforePanel==panel) return true;
            g.panels.removeAll(panel);
            const int at=beforePanel.isEmpty()?g.panels.size():g.panels.indexOf(beforePanel);
            g.panels.insert(at,panel); g.active=panel;
            emit groupsChanged(); return true;
        }
        if(original.panels.size()==1) return false;
    }
    // Whole groups retain identity, collapse state and preferred size when moved.
    if(whole && (to<0 || placement!="merge")) {
        auto g=m_groups.takeAt(from);
        g.location=location;
        if(location!="floating") g.home=location;
        else g.geometry=safeGeometry(QRect(QCursor::pos()-QPoint(30,10),g.geometry.size()));
        to=index(target);
        const int at=to<0?m_groups.size():to+(placement=="after"?1:0);
        m_groups.insert(at,g);
    } else {
        for(const auto &id:moving) m_groups[from].panels.removeAll(id);
        if(m_groups[from].panels.isEmpty()) m_groups.removeAt(from);
        else if(!m_groups[from].panels.contains(m_groups[from].active)) m_groups[from].active=m_groups[from].panels.first();
        to=index(target);
        if(to>=0 && placement=="merge") {
            auto &g=m_groups[to];
            int at=beforePanel.isEmpty()?g.panels.size():g.panels.indexOf(beforePanel);
            for(const auto &id:moving) g.panels.insert(at++,id);
            g.active=moving.first(); g.collapsed=false;
        } else {
            QRect geometry=original.geometry;
            if(location=="floating") geometry=safeGeometry(QRect(QCursor::pos()-QPoint(30,10),geometry.size()));
            Group g{newId(),location,moving,moving.first(),geometry,
                    location=="floating"?original.home:location,false,original.dockHeight};
            const int at=to<0?m_groups.size():to+(placement=="after"?1:0);
            m_groups.insert(at,g);
        }
    }
    if(location=="left") setLeftCollapsed(false);
    if(location=="right") setRightCollapsed(false);
    emit groupsChanged(); return true;
}
void WorkspaceManager::hidePanel(const QString &group,const QString &panel) {
    const int i=index(group); if(i<0 || m_groups[i].location=="hidden") return;
    if(panel.isEmpty() || m_groups[i].panels.size()==1) {
        if(!panel.isEmpty() && !m_groups[i].panels.contains(panel)) return;
        m_groups[i].location="hidden";
    } else {
        if(!m_groups[i].panels.contains(panel)) return;
        auto g=m_groups[i];
        m_groups[i].panels.removeAll(panel);
        if(m_groups[i].active==panel) m_groups[i].active=m_groups[i].panels.first();
        g.id=newId(); g.location="hidden"; g.panels={panel}; g.active=panel;
        m_groups.append(g);
    }
    emit groupsChanged();
}
void WorkspaceManager::showPanel(const QString &panel) {
    for(const auto &g:m_groups) if(g.panels.contains(panel)) {
        if(g.location!="hidden") { setActive(g.id,panel); setGroupCollapsed(g.id,false);
            if(g.location=="left")setLeftCollapsed(false); if(g.location=="right")setRightCollapsed(false); return; }
        const auto home=g.home;
        QString target;
        for(const auto &candidate:m_groups) if(candidate.location==home){target=candidate.id;break;}
        dockPayload(payload(g.id,panel,false),home,target); return;
    }
}
void WorkspaceManager::beginDrag(const QString &group,const QString &panel,bool whole) {
    const bool tools=group=="__toolstrip" && whole;
    if(!tools && index(group)<0) return;
    const QString data=tools?QStringLiteral("drawverse-tools-v1"):payload(group,panel,whole);
    // Native Qt drag-and-drop crosses QQuickWindow boundaries without UI-thread model work.
    QDrag drag(this); auto *mime=new QMimeData;
    mime->setData(tools?"application/x-drawverse-tool-strip":"application/x-drawverse-panel",data.toUtf8()); drag.setMimeData(mime);
    QPixmap ghost(220,48); ghost.fill(Qt::transparent);
    { QPainter painter(&ghost); painter.setRenderHint(QPainter::Antialiasing);
      painter.setBrush(QColor(28,30,33,230)); painter.setPen(QColor("#23b5ee")); painter.drawRoundedRect(ghost.rect().adjusted(1,1,-1,-1),3,3);
      painter.setPen(QColor("#e6e6e6"));
      QString title=QStringLiteral("工具条");
      if(!tools){const auto &g=m_groups[index(group)];QStringList titles;for(const auto &id:g.panels)titles.append(panelDefinition(id).value("title").toString());title=whole?titles.join(" / "):panelDefinition(panel).value("title").toString();}
      painter.drawText(ghost.rect().adjusted(12,0,-12,0),Qt::AlignVCenter,title); }
    drag.setPixmap(ghost); drag.setHotSpot({24,12});
    m_dragCancelled=false; m_dragActive=true;
    const auto action=drag.exec(Qt::MoveAction); m_dragActive=false;
#ifdef Q_OS_WIN
    // OLE consumes Escape before Qt's key filter sees it. Cancelled drags retain
    // the held mouse button; only a released drop may turn into a floating panel.
    m_dragCancelled=m_dragCancelled || (GetAsyncKeyState(VK_ESCAPE)&0x8000) || (GetAsyncKeyState(VK_LBUTTON)&0x8000);
#endif
    if(m_dockingSuppressed) { m_dockingSuppressed=false; emit dragModifiersChanged(); }
    if(action==Qt::IgnoreAction && !m_dragCancelled) {
        if(tools)floatToolStrip(QCursor::pos().x()-18,QCursor::pos().y()-5);
        else dockPayload(data,"floating");
    }
}
void WorkspaceManager::beginToolStripDrag(){beginDrag("__toolstrip",{},true);}
void WorkspaceManager::updateToolStripPosition(int x,int y) {
    QPoint position(x,y);QScreen *screen=QGuiApplication::screenAt(position);if(!screen)screen=QGuiApplication::primaryScreen();
    if(screen){const auto rect=screen->availableGeometry();position.setX(std::clamp(x,rect.left(),std::max(rect.left(),rect.right()-37)));position.setY(std::clamp(y,rect.top(),std::max(rect.top(),rect.bottom()-303)));}
    if(m_toolPosition==position)return;
    m_toolPosition=position;emit toolStripChanged();
}
void WorkspaceManager::floatToolStrip(int x,int y){updateToolStripPosition(x,y);if(!m_toolsFloating){m_toolsFloating=true;emit toolStripChanged();}}
bool WorkspaceManager::dockToolStrip(const QString &data){if(data!="drawverse-tools-v1")return false;if(m_toolsFloating){m_toolsFloating=false;emit toolStripChanged();}return true;}
bool WorkspaceManager::eventFilter(QObject *,QEvent *event) {
    if(m_dragActive && event->type()==QEvent::KeyPress && static_cast<QKeyEvent*>(event)->key()==Qt::Key_Escape) m_dragCancelled=true;
    if(event->type()==QEvent::DragEnter || event->type()==QEvent::DragMove || event->type()==QEvent::Drop) {
        const auto *drop=static_cast<QDropEvent*>(event);
        if(drop->mimeData()->hasFormat("application/x-drawverse-panel") || drop->mimeData()->hasFormat("application/x-drawverse-tool-strip")) {
#ifdef Q_OS_MACOS
            const bool suppressed=drop->modifiers().testFlag(Qt::MetaModifier);
#else
            const bool suppressed=drop->modifiers().testFlag(Qt::ControlModifier);
#endif
            if(suppressed!=m_dockingSuppressed){m_dockingSuppressed=suppressed;emit dragModifiersChanged();}
        }
    }
    return false;
}
void WorkspaceManager::detachGroup(const QString &group) { dockPayload(payload(group,{},true),"floating"); }
void WorkspaceManager::detachPanel(const QString &group,const QString &panel) { dockPayload(payload(group,panel,false),"floating"); }
void WorkspaceManager::returnGroup(const QString &group) {
    const int i=index(group); if(i<0) return;
    m_groups[i].location=m_groups[i].home; emit groupsChanged();
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
    rect.setWidth(std::clamp(rect.width(),150,1200)); rect.setHeight(std::clamp(rect.height(),60,1200));
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
    const int i=index(id); if(i>=0) m_groups[i].geometry=QRect(x,y,std::clamp(w,150,1200),std::clamp(h,60,1200));
}
void WorkspaceManager::saveLayout() const {
    QJsonArray groups;
    for(const auto &g:m_groups) groups.append(QJsonObject{{"id",g.id},{"location",g.location},{"panels",QJsonArray::fromStringList(g.panels)},
                                                        {"active",g.active},{"home",g.home},{"collapsed",g.collapsed},{"dockHeight",g.dockHeight},{"x",g.geometry.x()},{"y",g.geometry.y()},{"width",g.geometry.width()},{"height",g.geometry.height()}});
    QJsonObject panels; for(auto it=m_panels.cbegin();it!=m_panels.cend();++it) panels.insert(it.key(),QJsonObject::fromVariantMap(it.value()));
    QSettings settings(m_settingsFile.isEmpty()?QSettings::NativeFormat:QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse");
    const QJsonObject layout{{"version",4},{"groups",groups},{"panels",panels},{"uiRevision",m_uiRevision},{"leftWidth",m_leftWidth},{"rightWidth",m_rightWidth},{"leftCollapsed",m_leftCollapsed},{"rightCollapsed",m_rightCollapsed},{"toolsFloating",m_toolsFloating},{"toolX",m_toolPosition.x()},{"toolY",m_toolPosition.y()}};
    if(!m_settingsFile.isEmpty()) { QSettings file(m_settingsFile,QSettings::IniFormat); file.setValue("workspace",QJsonDocument(layout).toJson(QJsonDocument::Compact)); }
    else settings.setValue("workspace",QJsonDocument(layout).toJson(QJsonDocument::Compact));
}
bool WorkspaceManager::restoreLayout() {
    QSettings settings(QSettings::NativeFormat,QSettings::UserScope,"DrawVerse","DrawVerse");
    QByteArray bytes;
    if(m_settingsFile.isEmpty()) bytes=settings.value("workspace").toByteArray();
    else { QSettings file(m_settingsFile,QSettings::IniFormat); bytes=file.value("workspace").toByteArray(); }
    if(bytes.isEmpty() || bytes.size()>5*1024*1024) return false;
    const auto root=QJsonDocument::fromJson(bytes).object();
    const auto panels=root.value("panels").toObject(); const auto groups=root.value("groups").toArray();
    if(!QList<int>{1,2,3,4}.contains(root.value("version").toInt()) || panels.size()>64 || panels.size()<5 || groups.isEmpty() || groups.size()>64) return false;
    QHash<QString,QVariantMap> restoredPanels;
    for(auto it=panels.begin();it!=panels.end();++it) {
        const auto p=it.value().toObject().toVariantMap(); const auto kind=p.value("kind").toString();
        if(it.key().size()>128 || p.value("title").toString().size()>80 || p.value("content").toString().size()>65536 ||
           !QStringList{"color","brush","layers","history","navigator","notes","palette","brush-settings"}.contains(kind)) return false;
        restoredPanels.insert(it.key(),p);
    }
    for(const auto &id:QStringList{"color","brush","layers","history","navigator"}) if(!restoredPanels.contains(id) || restoredPanels[id].value("kind")!=id) return false;
    QList<Group> restored; QSet<QString> seen,groupIds;
    for(const auto &entry:groups) {
        const auto object=entry.toObject(); Group g;
        g.id=object.value("id").toString(); g.location=object.value("location").toString(); g.active=object.value("active").toString();
        if(g.id.isEmpty() || g.id.size()>128 || groupIds.contains(g.id) || !QStringList{"left","right","floating","hidden"}.contains(g.location)) return false;
        groupIds.insert(g.id);
        for(const auto &p:object.value("panels").toArray()) {
            const auto id=p.toString(); if(!restoredPanels.contains(id) || seen.contains(id)) return false;
            seen.insert(id); g.panels.append(id);
        }
        if(g.panels.isEmpty() || !g.panels.contains(g.active)) return false;
        g.home=object.value("home").toString(g.location=="left"?"left":"right");
        if(g.home!="left" && g.home!="right") return false;
        g.collapsed=object.value("collapsed").toBool(false);
        g.dockHeight=std::clamp(object.value("dockHeight").toInt(320),100,2000);
        g.geometry=safeGeometry({object.value("x").toInt(),object.value("y").toInt(),object.value("width").toInt(320),object.value("height").toInt(440)});
        restored.append(g);
    }
    if(seen.size()!=restoredPanels.size()) return false;
    // Validate the entire old layout first, then remove retired notes without
    // resetting unrelated panel groups or their floating window geometry.
    bool migrated=root.value("version").toInt()!=4;
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
    if(!restoredPanels.contains("brush-settings")) {
        restoredPanels.insert("brush-settings",{{"title",QStringLiteral("画笔设置")},{"kind","brush-settings"}});
        for(auto &g:restored)if(g.panels.contains("brush")){g.panels.append("brush-settings");break;}
        migrated=true;
    }
    m_panels=restoredPanels; m_groups=restored;
    m_uiRevision=root.value("uiRevision").toInt(0);
    m_leftWidth=std::clamp(root.value("leftWidth").toInt(180),150,520);
    m_rightWidth=std::clamp(root.value("rightWidth").toInt(190),190,520);
    m_leftCollapsed=root.value("leftCollapsed").toBool(false);
    m_rightCollapsed=root.value("rightCollapsed").toBool(false);
    updateToolStripPosition(root.value("toolX").toInt(100),root.value("toolY").toInt(100));
    m_toolsFloating=root.value("toolsFloating").toBool(false);emit toolStripChanged();
    emit dockMetricsChanged();
    if(migrated) saveLayout();
    emit groupsChanged(); return true;
}
