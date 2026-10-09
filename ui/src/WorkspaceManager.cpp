#include "WorkspaceManager.h"
#include <QGuiApplication>
#include <QScreen>
#include <QWindow>
#include <QCursor>
#include <QDrag>
#include <QMimeData>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QSettings>
#include <QUuid>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QSet>
#include <QPainter>
#include <QPixmap>
#include <QDropEvent>
#include <functional>
#include <algorithm>
#include <bit>
#ifdef Q_OS_WIN
#include <QtCore/qt_windows.h>
#endif

namespace { QString newId() { return QUuid::createUuid().toString(QUuid::WithoutBraces); } }
QRect WorkspaceManager::availableScreenGeometry(QWindow *window) const {
    auto *screen=window?window->screen():QGuiApplication::primaryScreen();
    return screen?screen->availableGeometry():QRect{};
}
void WorkspaceManager::watchMenuWindow(QWindow *window,bool visible) {
    if(visible) m_menuWindow=window;
    else if(m_menuWindow==window) m_menuWindow.clear();
}
bool WorkspaceManager::setWindowCornerRadius(QWindow *window,int radius) {
#ifdef Q_OS_WIN
    if(!window || QGuiApplication::platformName()!=QStringLiteral("windows")) return false;
    const auto handle=reinterpret_cast<HWND>(window->winId());
    if(radius<=0) return SetWindowRgn(handle,nullptr,TRUE)!=0;
    RECT bounds{};
    if(!GetWindowRect(handle,&bounds)) return false;
    const int diameter=qRound(2*std::clamp(radius,0,64)*window->devicePixelRatio());
    // Regions use physical outer-window coordinates and also clip compositor blur.
    const auto region=CreateRoundRectRgn(0,0,bounds.right-bounds.left+1,bounds.bottom-bounds.top+1,diameter,diameter);
    if(!region) return false;
    if(SetWindowRgn(handle,region,TRUE)!=0) return true; // Windows owns region now.
    DeleteObject(region);
    return false;
#else
    Q_UNUSED(window); Q_UNUSED(radius);
    return false;
#endif
}
bool WorkspaceManager::setMenuBarBlur(QWindow *window,bool enabled) {
#ifdef Q_OS_WIN
    if(!window || QGuiApplication::platformName()!=QStringLiteral("windows")) return false;
    // The Windows compositor blurs live content behind the alpha surface.
    // Opaque workspace pixels cover the effect; only the menu bar reveals it.
    struct AccentPolicy { int state,flags; DWORD gradient; int animation; };
    struct CompositionData { int attribute; void *data; SIZE_T size; };
    using SetComposition=BOOL(WINAPI *)(HWND,const CompositionData *);
    const auto module=GetModuleHandleW(L"user32.dll");
    const auto apply=std::bit_cast<SetComposition>(GetProcAddress(module,"SetWindowCompositionAttribute"));
    if(!apply) return false;
    AccentPolicy accent{enabled?3:0,0,0,0}; // ACCENT_ENABLE_BLURBEHIND / ACCENT_DISABLED
    CompositionData data{19,&accent,sizeof(accent)}; // WCA_ACCENT_POLICY
    return apply(reinterpret_cast<HWND>(window->winId()),&data)!=FALSE;
#else
    Q_UNUSED(window); Q_UNUSED(enabled);
    return false;
#endif
}
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
    initializeDocks();
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
    initializeDocks();
    emit dockMetricsChanged(); emit groupsChanged();
}
int WorkspaceManager::index(const QString &id) const { for (int i=0;i<m_groups.size();++i) if(m_groups[i].id==id) return i; return -1; }
QString WorkspaceManager::hostFor(const QString &group) const {
    for(auto it=m_docks.cbegin();it!=m_docks.cend();++it) if(DockTree::leaves(it.value()).contains(group)) return it.key();
    return {};
}
void WorkspaceManager::initializeDocks() {
    m_docks.clear(); m_windowGeometry.clear();
    DockTree::Node left,right;
    int leftHeight=0,rightHeight=0;
    for(const auto &g:m_groups) {
        if(g.location=="floating") { m_docks.insert(g.id,DockTree::leaf(g.id)); m_windowGeometry.insert(g.id,g.geometry); }
        else if(g.location=="left") { left=DockTree::split(left,DockTree::leaf(g.id),"vertical",leftHeight?double(leftHeight)/(leftHeight+g.dockHeight):.5);leftHeight+=g.dockHeight; }
        else if(g.location=="right") { right=DockTree::split(right,DockTree::leaf(g.id),"vertical",rightHeight?double(rightHeight)/(rightHeight+g.dockHeight):.5);rightHeight+=g.dockHeight; }
    }
    auto main=DockTree::split(DockTree::leaf("__canvas"),right,"horizontal",.8,"right");
    main=DockTree::split(left,main,"horizontal",.15,"left");
    if(m_toolsFloating) { m_docks.insert("__toolstrip",DockTree::leaf("__toolstrip"));m_windowGeometry.insert("__toolstrip",QRect(m_toolPosition,QSize(38,304))); }
    else main=DockTree::split(DockTree::leaf("__toolstrip"),main,"horizontal",.05,"toolsFirst");
    m_docks.insert("main",main);
}
void WorkspaceManager::removeDock(const QString &group) {
    const auto host=hostFor(group); if(host.isEmpty()) return;
    auto &tree=m_docks[host]; DockTree::remove(tree,group);
    if(tree.isEmpty() && host!="main") { m_docks.remove(host); m_windowGeometry.remove(host); }
}
void WorkspaceManager::addDock(const QString &group,const QString &location,const QString &target,const QString &edge) {
    if(!target.isEmpty()) {
        const auto host=hostFor(target);
        if(!host.isEmpty() && DockTree::insert(m_docks[host],target,group,edge)) {
            if(host!="main") {
                auto geometry=m_windowGeometry.value(host);const auto minimum=dockMinimum(m_docks[host]).toSize()+QSize(4,4);
                geometry.setSize(geometry.size().expandedTo(minimum));m_windowGeometry[host]=safeGeometry(geometry);
            }
            return;
        }
    }
    if(location=="floating") {
        const int at=index(group);
        const auto size=at>=0?m_groups[at].geometry.size():group=="__canvas"?QSize(900,650):QSize(38,304);
        auto geometry=safeGeometry(QRect(QCursor::pos()-QPoint(24,12),size));
        if(group=="__toolstrip") geometry=QRect(m_toolPosition,QSize(38,304));
        const auto host=m_docks.contains(group)?newId():group;
        m_docks.insert(host,DockTree::leaf(group));m_windowGeometry.insert(host,geometry);
        if(at>=0) m_groups[at].geometry=geometry;
        return;
    }
    const auto mainLeaves=DockTree::leaves(m_docks.value("main"));
    QString beside;
    for(const auto &id:mainLeaves) { const int at=index(id); if(at>=0 && m_groups[at].location==location) beside=id; }
    if(!beside.isEmpty()) { DockTree::insert(m_docks["main"],beside,group,"after"); return; }
    if(mainLeaves.contains("__canvas") && group!="__toolstrip") {
        DockTree::insert(m_docks["main"],"__canvas",group,location=="left"?"left":"right");return;
    }
    const bool before=location=="left" || group=="__toolstrip";
    auto &main=m_docks["main"];
    main=DockTree::split(before?DockTree::leaf(group):main,before?main:DockTree::leaf(group),"horizontal",before?.25:.75,
                         group=="__toolstrip"?"toolsFirst":QString());
}
void WorkspaceManager::syncToolsLocation() {
    const auto host=hostFor("__toolstrip"); const bool floating=host!="main";
    if(floating) m_toolPosition=m_windowGeometry.value(host,QRect(m_toolPosition,QSize(38,304))).topLeft();
    m_toolsFloating=floating; emit toolStripChanged();
}
QVariantList WorkspaceManager::floatingWindows() const {
    QVariantList result;
    for(auto it=m_docks.cbegin();it!=m_docks.cend();++it) {
        if(it.key()=="main") continue;
        const auto ids=DockTree::leaves(it.value()); if(ids.isEmpty()) continue;
        const auto geometry=m_windowGeometry.value(it.key(),QRect(100,100,320,440));
        const auto minimum=dockMinimum(it.value());
        QStringList panels;
        for(const auto &id:ids) panels.append(groupDefinition(id).value("panels").toStringList());
        result.append(QVariantMap{{"id",it.key()},{"groups",ids},{"panels",panels},{"single",ids.size()==1},
            {"collapsed",ids.size()==1 && groupDefinition(ids.first()).value("collapsed").toBool()},
            {"x",geometry.x()},{"y",geometry.y()},{"width",geometry.width()},{"height",geometry.height()},
            {"minimumWidth",minimum.width()+4},{"minimumHeight",minimum.height()+4}});
    }
    return result;
}
QSizeF WorkspaceManager::dockMinimum(const DockTree::Node &node) const {
    if(node.isEmpty()) return {};
    if(node.contains("group")) {
        const auto id=node.value("group").toString();
        if(id=="__canvas") return {320,200};
        if(id=="__toolstrip") return {36,280};
        const int at=index(id);if(at<0)return {};
        const auto &g=m_groups[at];
        if((g.location=="left" && m_leftCollapsed) || (g.location=="right" && m_rightCollapsed)) return {28,qreal(12+28*g.panels.size())};
        if(g.collapsed)return {150,qreal(8+22*g.panels.size())};
        const auto kind=panelDefinition(g.active).value("kind").toString();
        return {g.location=="right"?190.:150.,kind=="layers"?250.:kind=="brush-settings"?260.:120.};
    }
    const auto first=dockMinimum(node.value("first").toObject()),second=dockMinimum(node.value("second").toObject());
    if(node.value("axis")=="horizontal")return {first.width()+second.width()+4,std::max(first.height(),second.height())};
    return {std::max(first.width(),second.width()),first.height()+second.height()+4};
}
QVariantList WorkspaceManager::layoutItems(const QString &host,int width,int height) const {
    QVariantList result;
    const auto rail=[&](const QString &id) { const int at=index(id); return at>=0 && ((m_groups[at].location=="left" && m_leftCollapsed) || (m_groups[at].location=="right" && m_rightCollapsed)); };
    std::function<bool(const DockTree::Node&,bool)> compact=[&](const auto &node,bool horizontal) {
        const auto ids=DockTree::leaves(node);
        return !ids.isEmpty() && std::all_of(ids.cbegin(),ids.cend(),[&](const auto &id){const int at=index(id);return rail(id) || (!horizontal && at>=0 && m_groups[at].collapsed);});
    };
    std::function<void(const DockTree::Node&,QRectF)> visit=[&](const auto &node,QRectF rect) {
        if(node.isEmpty()) return;
        if(node.contains("group")) {
            auto data=groupDefinition(node.value("group").toString()); data.insert("kind","leaf");data.insert("rail",rail(data.value("id").toString()));
            data.insert("rect",rect); result.append(data); return;
        }
        const bool horizontal=node.value("axis")=="horizontal";
        const auto first=node.value("first").toObject(),second=node.value("second").toObject();
        const auto amin=dockMinimum(first),bmin=dockMinimum(second);
        const qreal space=std::max(qreal(0),(horizontal?rect.width():rect.height())-4);
        qreal a=space*node.value("ratio").toDouble(.5);
        const auto preferred=node.value("preferred").toString();
        if(horizontal && preferred=="left") a=m_leftCollapsed?28:m_leftWidth;
        if(horizontal && preferred=="right") a=space-(m_rightCollapsed?28:m_rightWidth);
        if(horizontal && (preferred=="toolsFirst" || first.value("group")=="__toolstrip")) a=36;
        if(horizontal && (preferred=="toolsSecond" || second.value("group")=="__toolstrip")) a=space-36;
        const qreal low=horizontal?amin.width():amin.height(),high=horizontal?bmin.width():bmin.height();
        if(compact(first,horizontal)) a=low;
        else if(compact(second,horizontal)) a=space-high;
        if(space>=low+high) a=std::clamp(a,low,space-high);
        else a=low+high>0?space*low/(low+high):space/2;
        auto ar=rect,br=rect,handle=rect;
        if(horizontal) {ar.setWidth(a);handle.setX(rect.x()+a);handle.setWidth(4);br.setX(rect.x()+a+4);br.setWidth(std::max(qreal(0),space-a));}
        else {ar.setHeight(a);handle.setY(rect.y()+a);handle.setHeight(4);br.setY(rect.y()+a+4);br.setHeight(std::max(qreal(0),space-a));}
        result.append(QVariantMap{{"id",node.value("id").toString()},{"kind","split"},{"axis",horizontal?"horizontal":"vertical"},{"rect",handle},{"area",rect}});
        visit(first,ar);visit(second,br);
    };
    visit(m_docks.value(host),QRectF(0,0,std::max(0,width),std::max(0,height)));return result;
}
void WorkspaceManager::setSplitRatio(const QString &host,const QString &split,double ratio) {
    if(!std::isfinite(ratio) || !m_docks.contains(host)) return;
    if(DockTree::setRatio(m_docks[host],split,ratio)) emit layoutChanged();
}
QVariantMap WorkspaceManager::groupDefinition(const QString &id) const {
    if(id=="__canvas" || id=="__toolstrip") {
        const auto host=hostFor(id); const auto geometry=m_windowGeometry.value(host,id=="__canvas"?QRect(100,100,900,650):QRect(100,100,38,304));
        return {{"id",id},{"host",host},{"location",host=="main"?"center":"floating"},{"panels",QStringList{id}},{"active",id},
                {"x",geometry.x()},{"y",geometry.y()},{"width",geometry.width()},{"height",geometry.height()},{"collapsed",false}};
    }
    const int i=index(id); if(i<0) return {};
    const auto &g=m_groups[i];
    return {{"id",g.id},{"host",hostFor(id)},{"location",g.location},{"panels",g.panels},{"active",g.active},
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
QVariantMap WorkspaceManager::panelDefinition(const QString &id) const {
    if(id=="__canvas") return {{"title",QStringLiteral("画布")},{"kind","canvas"}};
    if(id=="__toolstrip") return {{"title",QStringLiteral("工具条")},{"kind","tools"}};
    return m_panels.value(id);
}
void WorkspaceManager::setActive(const QString &group,const QString &panel) {
    const int i=index(group); if(i<0 || !m_groups[i].panels.contains(panel) || m_groups[i].active==panel) return;
    m_groups[i].active=panel; emit groupStateChanged(group);
    // Selection and collapse never destroy floating windows or active controls.
}
void WorkspaceManager::setGroupCollapsed(const QString &group,bool collapsed) {
    const int i=index(group); if(i<0 || m_groups[i].collapsed==collapsed) return;
    m_groups[i].collapsed=collapsed; emit groupStateChanged(group);emit layoutChanged();
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
    if(data.size()>1024 || !QStringList{"left","right","floating","center","main"}.contains(location) ||
       !QStringList{"merge","before","after","left","right","top","bottom"}.contains(placement)) return false;
    const auto object=QJsonDocument::fromJson(data.toUtf8()).object();
    const QString source=object.value("group").toString(), panel=object.value("panel").toString();
    int from=index(source), to=index(target);
    const bool special=source=="__canvas" || source=="__toolstrip";
    const bool targetSpecial=target=="__canvas" || target=="__toolstrip";
    if((from<0 && !special) || (!target.isEmpty() && ((!targetSpecial && to<0) || groupDefinition(target).value("location").toString()!=location))) return false;
    if(!target.isEmpty() && hostFor(target).isEmpty()) return false;
    if(special) {
        if(source==target || !beforePanel.isEmpty() || hostFor(source).isEmpty() || !object.value("whole").toBool() || !object.value("panel").toString().isEmpty()) return false;
        removeDock(source);
        addDock(source,location,target,placement=="merge"?"right":placement);
        syncToolsLocation(); emit groupsChanged(); return true;
    }
    const QString mode=targetSpecial && placement=="merge"?QStringLiteral("right"):placement;
    const auto original=m_groups[from];
    const bool whole=object.value("whole").toBool(false);
    if(!whole && !original.panels.contains(panel)) return false;
    if(!beforePanel.isEmpty() && (to<0 || mode!="merge" || !m_groups[to].panels.contains(beforePanel))) return false;
    const QStringList moving=whole?original.panels:QStringList{panel};
    if(source==target) {
        if(whole) return false;
        if(mode=="merge") {
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
    if(whole && (to<0 || mode!="merge")) {
        removeDock(source);
        auto g=m_groups.takeAt(from);
        g.location=location;
        if(location=="left" || location=="right") g.home=location;
        else g.geometry=safeGeometry(QRect(QCursor::pos()-QPoint(30,10),g.geometry.size()));
        to=index(target);
        const int at=to<0?m_groups.size():to+(mode=="after" || mode=="bottom" || mode=="right"?1:0);
        m_groups.insert(at,g);
        addDock(g.id,location,target,mode);
    } else {
        for(const auto &id:moving) m_groups[from].panels.removeAll(id);
        if(m_groups[from].panels.isEmpty()) { removeDock(source);m_groups.removeAt(from); }
        else if(!m_groups[from].panels.contains(m_groups[from].active)) m_groups[from].active=m_groups[from].panels.first();
        to=index(target);
        if(to>=0 && mode=="merge") {
            auto &g=m_groups[to];
            int at=beforePanel.isEmpty()?g.panels.size():g.panels.indexOf(beforePanel);
            for(const auto &id:moving) g.panels.insert(at++,id);
            g.active=moving.first(); g.collapsed=false;
        } else {
            QRect geometry=original.geometry;
            if(location=="floating") geometry=safeGeometry(QRect(QCursor::pos()-QPoint(30,10),geometry.size()));
            Group g{newId(),location,moving,moving.first(),geometry,
                    location=="left" || location=="right"?location:original.home,false,original.dockHeight};
            const int at=to<0?m_groups.size():to+(mode=="after" || mode=="bottom" || mode=="right"?1:0);
            m_groups.insert(at,g);
            addDock(g.id,location,target,mode);
        }
    }
    if(location=="left") setLeftCollapsed(false);
    if(location=="right") setRightCollapsed(false);
    syncToolsLocation();emit groupsChanged(); return true;
}
void WorkspaceManager::hidePanel(const QString &group,const QString &panel) {
    const int i=index(group); if(i<0 || m_groups[i].location=="hidden") return;
    if(panel.isEmpty() || m_groups[i].panels.size()==1) {
        if(!panel.isEmpty() && !m_groups[i].panels.contains(panel)) return;
        removeDock(group);
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
    if(!tools && group!="__canvas" && index(group)<0) return;
    const QString data=tools?QStringLiteral("drawverse-tools-v1"):payload(group,panel,whole);
    // Native Qt drag-and-drop crosses QQuickWindow boundaries without UI-thread model work.
    QDrag drag(this); auto *mime=new QMimeData;
    mime->setData(tools?"application/x-drawverse-tool-strip":"application/x-drawverse-panel",data.toUtf8()); drag.setMimeData(mime);
    QPixmap ghost(220,48); ghost.fill(Qt::transparent);
    { QPainter painter(&ghost); painter.setRenderHint(QPainter::Antialiasing);
      painter.setBrush(QColor(28,30,33,230)); painter.setPen(QColor("#23b5ee")); painter.drawRoundedRect(ghost.rect().adjusted(1,1,-1,-1),3,3);
      painter.setPen(QColor("#e6e6e6"));
      QString title=QStringLiteral("工具条");
      if(group=="__canvas") title=QStringLiteral("画布");
      else if(!tools){const auto &g=m_groups[index(group)];QStringList titles;for(const auto &id:g.panels)titles.append(panelDefinition(id).value("title").toString());title=whole?titles.join(" / "):panelDefinition(panel).value("title").toString();}
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
void WorkspaceManager::floatToolStrip(int x,int y){updateToolStripPosition(x,y);dockPayload(payload("__toolstrip",{},true),"floating");}
bool WorkspaceManager::dockToolStrip(const QString &data,const QString &location,const QString &target,const QString &placement){
    if(data!="drawverse-tools-v1")return false;
    return dockPayload(payload("__toolstrip",{},true),location,target,placement);
}
bool WorkspaceManager::eventFilter(QObject *,QEvent *event) {
    if(m_menuWindow && m_menuWindow->isVisible() && event->type()==QEvent::MouseButtonPress) {
        const auto *press=static_cast<QMouseEvent*>(event);
        if(!m_menuWindow->geometry().contains(press->globalPosition().toPoint())) {
            m_menuWindow->close();
            return true;
        }
    }
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
    if(group=="__canvas" || group=="__toolstrip") { dockPayload(payload(group,{},true),"main");return; }
    const int i=index(group); if(i<0) return;
    const auto home=m_groups[i].home;removeDock(group);m_groups[i].location=home;addDock(group,home,{},"after");syncToolsLocation();emit groupsChanged();
}
void WorkspaceManager::returnWindow(const QString &host) {
    if(host=="main") return;
    const auto ids=DockTree::leaves(m_docks.value(host));
    for(const auto &id:ids) returnGroup(id);
}
QString WorkspaceManager::addCustomPanel(const QString &title,const QString &kind) {
    if(m_panels.size()>=64 || (kind!="palette" && kind!="brush")) return {};
    const QString id=newId(); const QString name=title.trimmed().left(80);
    m_panels.insert(id,{{"title",name.isEmpty()?QStringLiteral("自定义面板"):name},{"kind",kind},{"custom",true},{"content",QString()}});
    // Add a tab rather than an unbounded number of dock columns.
    for(auto &g:m_groups) if(g.location=="right") { g.panels.append(id); g.active=id; emit groupsChanged(); return id; }
    const auto group=newId();m_groups.append({group,"right",{id},id,{100,100,320,440}});addDock(group,"right",{},"after");emit groupsChanged();return id;
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
    if(m_windowGeometry.contains(id)) {
        m_windowGeometry[id]=QRect(x,y,std::clamp(w,36,3000),std::clamp(h,60,2000));
        if(hostFor("__toolstrip")==id) {m_toolPosition={x,y};emit toolStripChanged();}
    }
}
void WorkspaceManager::saveLayout() const {
    QJsonArray groups;
    for(const auto &g:m_groups) groups.append(QJsonObject{{"id",g.id},{"location",g.location},{"panels",QJsonArray::fromStringList(g.panels)},
                                                        {"active",g.active},{"home",g.home},{"collapsed",g.collapsed},{"dockHeight",g.dockHeight},{"x",g.geometry.x()},{"y",g.geometry.y()},{"width",g.geometry.width()},{"height",g.geometry.height()}});
    QJsonObject panels; for(auto it=m_panels.cbegin();it!=m_panels.cend();++it) panels.insert(it.key(),QJsonObject::fromVariantMap(it.value()));
    QSettings settings(m_settingsFile.isEmpty()?QSettings::NativeFormat:QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse");
    QJsonObject docks,windows;
    for(auto it=m_docks.cbegin();it!=m_docks.cend();++it) docks.insert(it.key(),it.value());
    for(auto it=m_windowGeometry.cbegin();it!=m_windowGeometry.cend();++it) { const auto r=it.value();windows.insert(it.key(),QJsonObject{{"x",r.x()},{"y",r.y()},{"width",r.width()},{"height",r.height()}}); }
    const QJsonObject layout{{"version",5},{"docks",docks},{"windows",windows},{"groups",groups},{"panels",panels},{"uiRevision",m_uiRevision},{"leftWidth",m_leftWidth},{"rightWidth",m_rightWidth},{"leftCollapsed",m_leftCollapsed},{"rightCollapsed",m_rightCollapsed},{"toolsFloating",m_toolsFloating},{"toolX",m_toolPosition.x()},{"toolY",m_toolPosition.y()}};
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
    if(!QList<int>{1,2,3,4,5}.contains(root.value("version").toInt()) || panels.size()>64 || panels.size()<5 || groups.isEmpty() || groups.size()>64) return false;
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
        if(g.id.isEmpty() || g.id=="__canvas" || g.id=="__toolstrip" || g.id=="main" || g.id.size()>128 || groupIds.contains(g.id) || !QStringList{"left","right","center","floating","hidden"}.contains(g.location)) return false;
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
    bool migrated=root.value("version").toInt()!=5;
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
    QMap<QString,DockTree::Node> docks;QHash<QString,QRect> windows;
    if(root.value("version").toInt()==5) {
        const auto savedDocks=root.value("docks").toObject(),savedWindows=root.value("windows").toObject();
        if(!savedDocks.contains("main") || savedDocks.size()>67 || savedWindows.size()!=savedDocks.size()-1) return false;
        QSet<QString> allowed{"__canvas","__toolstrip"},seenLeaves,splitIds;
        for(const auto &g:restored) if(g.location!="hidden") allowed.insert(g.id);
        for(auto it=savedDocks.constBegin();it!=savedDocks.constEnd();++it) {
            if(it.key().isEmpty() || it.key().size()>128 || !it.value().isObject() ||
               !DockTree::validate(it.value().toObject(),allowed,seenLeaves,splitIds)) return false;
            const auto ids=DockTree::leaves(it.value().toObject());
            if(it.key()!="main" && (ids.isEmpty() || !savedWindows.contains(it.key()))) return false;
            for(const auto &id:ids) for(const auto &g:restored) if(g.id==id && ((it.key()=="main")== (g.location=="floating"))) return false;
            docks.insert(it.key(),it.value().toObject());
            if(it.key()!="main") {
                const auto geometry=savedWindows.value(it.key()).toObject();
                if(geometry.isEmpty() || !geometry.value("x").isDouble() || !geometry.value("y").isDouble() ||
                   geometry.value("width").toInt()<36 || geometry.value("width").toInt()>3000 ||
                   geometry.value("height").toInt()<60 || geometry.value("height").toInt()>2000) return false;
                auto r=safeGeometry({geometry.value("x").toInt(),geometry.value("y").toInt(),geometry.value("width").toInt(320),geometry.value("height").toInt(440)});
                if(ids==QStringList{"__toolstrip"}) r.setWidth(38);
                windows.insert(it.key(),r);
            }
        }
        if(seenLeaves!=allowed) return false;
    }
    m_panels=restoredPanels; m_groups=restored;
    m_uiRevision=root.value("uiRevision").toInt(0);
    m_leftWidth=std::clamp(root.value("leftWidth").toInt(180),150,520);
    m_rightWidth=std::clamp(root.value("rightWidth").toInt(190),190,520);
    m_leftCollapsed=root.value("leftCollapsed").toBool(false);
    m_rightCollapsed=root.value("rightCollapsed").toBool(false);
    updateToolStripPosition(root.value("toolX").toInt(100),root.value("toolY").toInt(100));
    m_toolsFloating=root.value("toolsFloating").toBool(false);emit toolStripChanged();
    if(root.value("version").toInt()==5) {m_docks=docks;m_windowGeometry=windows;syncToolsLocation();}
    else initializeDocks();
    emit dockMetricsChanged();
    if(migrated) saveLayout();
    emit groupsChanged(); return true;
}
