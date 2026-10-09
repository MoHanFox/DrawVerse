#include "WorkspaceManager.h"
#include <QGuiApplication>
#include <QScreen>
#include <QWindow>
#include <QCursor>
#include <QMimeData>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QSettings>
#include <QUuid>
#include <QMouseEvent>
#include <QTouchEvent>
#include <QTabletEvent>
#include <QKeyEvent>
#include <QSet>
#include <QDropEvent>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTimer>
#include <functional>
#include <algorithm>
#ifdef Q_OS_WIN
#include <QtCore/qt_windows.h>
#endif

namespace { QString newId() { return QUuid::createUuid().toString(QUuid::WithoutBraces); } }
bool WorkspaceManager::windowsWindowFrames() const {
    return QGuiApplication::platformName()==QStringLiteral("windows");
}
QRect WorkspaceManager::availableScreenGeometry(QWindow *window) const {
    auto *screen=window?window->screen():QGuiApplication::primaryScreen();
    return screen?screen->availableGeometry():QRect{};
}
void WorkspaceManager::watchMenuWindow(QWindow *window,bool visible) {
    if(visible) m_menuWindow=window;
    else if(m_menuWindow==window) m_menuWindow.clear();
}
void WorkspaceManager::watchPanelFlyout(QWindow *window,QQuickItem *owner,bool visible) {
    if(visible) {
        if(m_panelFlyout && m_panelFlyout!=window){m_panelFlyout->hide();m_panelFlyout->close();}
        m_panelFlyout=window;m_flyoutOwner=owner;
        ++m_flyoutRevision;
        // Reinstall last so transient dismissal precedes other application input filters.
        qApp->removeEventFilter(this);qApp->installEventFilter(this);
    } else if(m_panelFlyout==window){m_panelFlyout.clear();m_flyoutOwner.clear();}
}
bool WorkspaceManager::isIconGroup(const QString &group) const {
    const int at=index(group);
    return at>=0 && (m_iconGroups.contains(group) || (m_groups[at].location=="left" && m_leftCollapsed) || (m_groups[at].location=="right" && m_rightCollapsed));
}
bool WorkspaceManager::setWindowCornerRadius(QWindow *window,int radius) {
#ifdef Q_OS_WIN
    if(!window || QGuiApplication::platformName()!=QStringLiteral("windows")) return false;
    const auto handle=reinterpret_cast<HWND>(window->winId());
    if(window==m_glassWindow)m_menuBlur.setRadius(radius);
    // An iconic HWND has a tiny shell rectangle; applying a region there breaks
    // its restore placement and can leave only the Windows minimized caption.
    if(IsIconic(handle) || window->visibility()==QWindow::Minimized)return false;
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
bool WorkspaceManager::setMenuBarBlur(QWindow *window,bool enabled,int height) {
#ifdef Q_OS_WIN
    if(!window || QGuiApplication::platformName()!=QStringLiteral("windows")) return false;
    m_glassWindow=window;m_glassHandle=window->winId();
    return m_menuBlur.setEnabled(window,enabled,height);
#else
    Q_UNUSED(window); Q_UNUSED(enabled);Q_UNUSED(height);
    return false;
#endif
}
WorkspaceManager::WorkspaceManager(const QString &settingsFile, QObject *parent) : QObject(parent), m_settingsFile(settingsFile),m_windowDrag(this) {
    qApp->installEventFilter(this);
    qApp->installNativeEventFilter(this);
    connect(&m_windowDrag,&WindowDrag::moved,this,[this](QPoint global,bool suppressed) {
        updateDragTarget(global,suppressed);
    });
    connect(&m_windowDrag,&WindowDrag::finished,this,[this](bool cancelled) {
        const auto target=m_dragTarget,placement=m_dragPlacement;const auto moving=m_dragGroups;
        m_dragHost.clear();m_dragGroups.clear();m_dragTarget.clear();m_dragPlacement.clear();m_dockingSuppressed=false;emit dragModifiersChanged();
        if(cancelled){m_docks=m_beforeDocks;m_windowGeometry=m_beforeWindows;m_groups=m_beforeGroups;m_iconGroups=m_beforeIcons;syncToolsLocation();emit groupsChanged();return;}
        if(target.isEmpty())return;
        if(target.startsWith("__workspace_")) {
            const auto side=target.endsWith("left")?QStringLiteral("left"):QStringLiteral("right");
            QString previous;
            for(const auto &id:moving){dockPayload(payload(id,{},true),side,previous,"after");previous=id;}
            return;
        }
        QString previous=target;
        for(const auto &id:moving) {
            const auto location=groupDefinition(previous).value("location").toString();
            const auto mode=id==moving.first()?placement:QStringLiteral("after");
            dockPayload(payload(id,{},true),location,previous,mode);
            if(mode!="merge")previous=id;
        }
    });
    resetLayout(); restoreLayout();
}
WorkspaceManager::~WorkspaceManager() {
    qApp->removeNativeEventFilter(this);
}
bool WorkspaceManager::nativeEventFilter(const QByteArray &eventType,void *message,qintptr *result) {
#ifdef Q_OS_WIN
    if(eventType!="windows_generic_MSG" || !m_glassWindow)return false;
    const auto *msg=static_cast<MSG*>(message);
    if(reinterpret_cast<quintptr>(msg->hwnd)!=m_glassHandle)return false;
    if(msg->message==WM_SYSCOMMAND && (msg->wParam&0xfff0)==SC_MOVE &&
       (IsZoomed(msg->hwnd) || m_glassWindow->visibility()==QWindow::Maximized || m_glassWindow->visibility()==QWindow::FullScreen)) {*result=0;return true;}
    if((msg->message==WM_SYSCOMMAND && (msg->wParam&0xfff0)==SC_MINIMIZE) ||
       (msg->message==WM_WINDOWPOSCHANGING && (reinterpret_cast<WINDOWPOS*>(msg->lParam)->flags&SWP_HIDEWINDOW)) ||
       (msg->message==WM_SIZE && msg->wParam==SIZE_MINIMIZED) || (msg->message==WM_SHOWWINDOW && !msg->wParam))m_menuBlur.hide();
    else if(msg->message==WM_WINDOWPOSCHANGED || msg->message==WM_SIZE || msg->message==WM_ACTIVATE || msg->message==WM_DPICHANGED)m_menuBlur.sync();
    // DefWindowProc paints an iconic caption into even a frameless alpha
    // surface during minimize/restore. DWM keeps those pixels behind our menu.
    // The QML title bar owns this window's entire non-client presentation.
    if(msg->message==WM_NCPAINT || msg->message==WM_NCCALCSIZE) {*result=0;return true;}
    if(msg->message==WM_NCACTIVATE) {*result=TRUE;return true;}
#else
    Q_UNUSED(eventType);Q_UNUSED(message);Q_UNUSED(result);
#endif
    return false;
}
void WorkspaceManager::updateDragTarget(QPoint global,bool suppressed) {
    QString target,placement; qreal best=25;
    auto *moving=m_windowDrag.window();
    const QRectF movingRect=moving?QRectF(moving->geometry()):QRectF();
    const auto exposed=[&](QWindow *window,QPointF point) {
#ifdef Q_OS_WIN
        if(QGuiApplication::platformName()=="windows") {
            const auto all=QGuiApplication::allWindows();
            for(auto handle=GetTopWindow(nullptr);handle;handle=GetWindow(handle,GW_HWNDNEXT))
                for(auto *w:all)if(w!=moving && w->isVisible() && w->visibility()!=QWindow::Minimized && reinterpret_cast<HWND>(w->winId())==handle && w->geometry().contains(point.toPoint()))return w==window;
        }
#endif
        Q_UNUSED(window); Q_UNUSED(point);return true;
    };
    const auto consider=[&](const QString &id,QQuickItem *item,bool workspace) {
        if(!item || !item->isVisible() || !item->window() || !item->window()->isVisible() || item->window()->visibility()==QWindow::Minimized)return;
        const QRectF area(item->mapToGlobal({0,0}),QSizeF(item->width(),item->height()));
        const auto offer=[&](qreal distance,const QString &edge,QPointF contact) {
            contact.setX(std::clamp(contact.x(),area.left()+1,area.right()-1));
            contact.setY(std::clamp(contact.y(),area.top()+1,area.bottom()-1));
            if(distance<=24 && distance<best && exposed(item->window(),contact)) {
                best=distance;target=workspace?"__workspace_"+edge:id;placement=edge;
            }
        };
        const auto at=QPointF(global)-area.topLeft();
        if(area.adjusted(-24,-24,24,24).contains(global)) {
            if(global.y()>=area.top() && global.y()<=area.bottom()) {
                offer(std::abs(at.x()),"left",{area.left(),qreal(global.y())});
                offer(std::abs(at.x()-area.width()),"right",{area.right(),qreal(global.y())});
            }
            if(!workspace && global.x()>=area.left() && global.x()<=area.right()) {
                // Tabs remain a deliberate merge target; the panel body never is.
                if(!m_dragGroups.contains("__toolstrip") && at.y()>=8 && at.y()<28 && at.x()>24 && at.x()<area.width()-24)offer(0,"merge",global);
                else offer(std::abs(at.y()),"before",{qreal(global.x()),area.top()});
                offer(std::abs(at.y()-area.height()),"after",{qreal(global.x()),area.bottom()});
            }
        }
        // The dragged window edge can reach a target before its pointer does.
        const auto overlapY=std::min(area.bottom(),movingRect.bottom())-std::max(area.top(),movingRect.top());
        const auto overlapX=std::min(area.right(),movingRect.right())-std::max(area.left(),movingRect.left());
        if(overlapY>12) {
            if(movingRect.center().x()<area.left())offer(std::abs(movingRect.right()-area.left()),"left",{area.left(),std::clamp(movingRect.center().y(),area.top(),area.bottom())});
            if(movingRect.center().x()>area.right())offer(std::abs(movingRect.left()-area.right()),"right",{area.right(),std::clamp(movingRect.center().y(),area.top(),area.bottom())});
        }
        if(!workspace && overlapX>12) {
            if(movingRect.center().y()<area.top())offer(std::abs(movingRect.bottom()-area.top()),"before",{std::clamp(movingRect.center().x(),area.left(),area.right()),area.top()});
            if(movingRect.center().y()>area.bottom())offer(std::abs(movingRect.top()-area.bottom()),"after",{std::clamp(movingRect.center().x(),area.left(),area.right()),area.bottom()});
        }
    };
    if(!suppressed) {
        // Main outer edges win ties, but covered edges do not steal another host.
        consider({},m_workspaceArea,true);
        for(auto it=m_targets.cbegin();it!=m_targets.cend();++it)
            if(!m_dragGroups.contains(it.key()) && hostFor(it.key())!=m_dragHost)consider(it.key(),it.value(),false);
    }
    m_dockingSuppressed=suppressed;m_dragTarget=target;m_dragPlacement=placement;emit dragModifiersChanged();
}
void WorkspaceManager::resetLayout() {
    m_iconGroups.clear();
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
int WorkspaceManager::index(const QString &id) const { for (int i=0;i<m_groups.size();++i) if(m_groups[i].id==id) return i; return -1; }
QString WorkspaceManager::hostFor(const QString &group) const {
    for(auto it=m_docks.cbegin();it!=m_docks.cend();++it) if(DockTree::leaves(it.value()).contains(group)) return it.key();
    return {};
}
QStringList WorkspaceManager::columnGroups(const QString &group) const {
    QStringList result{group};
    std::function<bool(const DockTree::Node&)> vertical=[&](const auto &node) {
        if(node.isEmpty())return false;
        if(node.contains("group"))return index(node.value("group").toString())>=0;
        return node.value("axis")=="vertical" && vertical(node.value("first").toObject()) && vertical(node.value("second").toObject());
    };
    std::function<void(const DockTree::Node&)> visit=[&](const auto &node) {
        if(node.isEmpty() || !DockTree::leaves(node).contains(group))return;
        if(vertical(node)){result=DockTree::leaves(node);return;}
        if(!node.contains("group")){visit(node.value("first").toObject());visit(node.value("second").toObject());}
    };
    visit(m_docks.value(hostFor(group)));return result;
}
void WorkspaceManager::setColumnCollapsed(const QString &group,bool collapsed) {
    for(const auto &id:columnGroups(group))if(index(id)>=0){if(collapsed)m_iconGroups.insert(id);else m_iconGroups.remove(id);}
    const auto location=groupDefinition(group).value("location").toString();
    if(!collapsed){if(location=="left")m_leftCollapsed=false;if(location=="right")m_rightCollapsed=false;}
    emit dockMetricsChanged();emit groupsChanged();
}
void WorkspaceManager::registerTarget(const QString &group,QQuickItem *item){if(group!="__canvas")m_targets[group]=item;}
void WorkspaceManager::registerWorkspace(QQuickItem *item){m_workspaceArea=item;}
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
    if(group!="__toolstrip" && !beside.isEmpty()) { DockTree::insert(m_docks["main"],beside,group,"after"); return; }
    if(mainLeaves.contains("__canvas") && group!="__toolstrip") {
        DockTree::insert(m_docks["main"],"__canvas",group,location=="left"?"left":"right");return;
    }
    const bool before=location=="left" || (group=="__toolstrip" && location!="right");
    auto &main=m_docks["main"];
    main=DockTree::split(before?DockTree::leaf(group):main,before?main:DockTree::leaf(group),"horizontal",before?.25:.75,
                         group=="__toolstrip"?(before?"toolsFirst":"toolsSecond"):QString());
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
        const bool collapsed=std::all_of(ids.cbegin(),ids.cend(),[this](const auto &id){return isIconGroup(id);});
        const bool icons=std::all_of(ids.cbegin(),ids.cend(),[this](const auto &id){return m_iconGroups.contains(id);});
        result.append(QVariantMap{{"id",it.key()},{"groups",ids},{"panels",panels},{"single",ids.size()==1},{"icons",icons},
            {"collapsed",collapsed},{"compactHeight",minimum.height()+2},
            {"x",geometry.x()},{"y",geometry.y()},{"width",geometry.width()},{"height",geometry.height()},
            {"minimumWidth",minimum.width()+2},{"minimumHeight",minimum.height()+2}});
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
        const bool first=columnGroups(id).first()==id;
        if(isIconGroup(id)) return {28,qreal((first?12:0)+28*g.panels.size())};
        const auto kind=panelDefinition(g.active).value("kind").toString();
        return {g.location=="right"?190.:150.,kind=="layers"?250.:kind=="brush-settings"?260.:120.};
    }
    const auto first=dockMinimum(node.value("first").toObject()),second=dockMinimum(node.value("second").toObject());
    if(node.value("axis")=="horizontal")return {first.width()+second.width()+1,std::max(first.height(),second.height())};
    return {std::max(first.width(),second.width()),first.height()+second.height()+1};
}
QVariantList WorkspaceManager::layoutItems(const QString &host,int width,int height) const {
    QVariantList result;
    const auto rail=[&](const QString &id) {return isIconGroup(id);};
    std::function<bool(const DockTree::Node&,bool)> compact=[&](const auto &node,bool) {
        const auto ids=DockTree::leaves(node);
        return !ids.isEmpty() && std::all_of(ids.cbegin(),ids.cend(),[&](const auto &id){return rail(id);});
    };
    std::function<void(const DockTree::Node&,QRectF)> visit=[&](const auto &node,QRectF rect) {
        if(node.isEmpty()) return;
        if(node.contains("group")) {
            auto data=groupDefinition(node.value("group").toString()); data.insert("kind","leaf");data.insert("rail",rail(data.value("id").toString()));
            const auto id=data.value("id").toString();data.insert("columnFirst",columnGroups(id).first()==id);
            if(compact(node,false))rect.setHeight(std::min(rect.height(),dockMinimum(node).height()));
            data.insert("rect",rect); result.append(data); return;
        }
        const bool horizontal=node.value("axis")=="horizontal";
        const auto first=node.value("first").toObject(),second=node.value("second").toObject();
        const auto amin=dockMinimum(first),bmin=dockMinimum(second);
        if(!horizontal && compact(node,false))rect.setHeight(std::min(rect.height(),amin.height()+bmin.height()+1));
        const qreal space=std::max(qreal(0),(horizontal?rect.width():rect.height())-1);
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
        if(horizontal) {ar.setWidth(a);handle.setX(rect.x()+a);handle.setWidth(1);br.setX(rect.x()+a+1);br.setWidth(std::max(qreal(0),space-a));}
        else {ar.setHeight(a);handle.setY(rect.y()+a);handle.setHeight(1);br.setY(rect.y()+a+1);br.setHeight(std::max(qreal(0),space-a));}
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
            {"collapsed",false},{"icons",isIconGroup(g.id)},{"dockHeight",g.dockHeight}};
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
void WorkspaceManager::swapPanelTabs(const QString &group,const QString &first,const QString &second) {
    const int at=index(group);if(at<0 || first==second)return;
    auto &panels=m_groups[at].panels;const auto a=panels.indexOf(first),b=panels.indexOf(second);
    if(a<0 || b<0)return;
    panels.swapItemsAt(a,b);emit groupsChanged();
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
    if(source=="__canvas" || target=="__canvas")return false;
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
    const bool targetIcons=isIconGroup(target) || (target.isEmpty() && std::any_of(m_groups.cbegin(),m_groups.cend(),[&](const auto &g){return g.location==location && isIconGroup(g.id);}));
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
        if(targetIcons)m_iconGroups.insert(g.id);
        addDock(g.id,location,target,mode);
    } else {
        for(const auto &id:moving) m_groups[from].panels.removeAll(id);
        if(m_groups[from].panels.isEmpty()) { removeDock(source);m_iconGroups.remove(source);m_groups.removeAt(from); }
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
            if(targetIcons)m_iconGroups.insert(g.id);
            addDock(g.id,location,target,mode);
        }
    }
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
        if(g.location!="hidden") { setActive(g.id,panel);
            return; }
        const auto home=g.home;
        QString target;
        for(const auto &candidate:m_groups) if(candidate.location==home){target=candidate.id;break;}
        dockPayload(payload(g.id,panel,false),home,target); return;
    }
}
void WorkspaceManager::beginDrag(const QString &group,const QString &panel,bool whole) {
    if(m_windowDrag.active() || !m_dragHost.isEmpty() || group=="__canvas" || (index(group)<0 && group!="__toolstrip"))return;
    m_beforeDocks=m_docks;m_beforeWindows=m_windowGeometry;m_beforeGroups=m_groups;m_beforeIcons=m_iconGroups;
    const auto oldHost=hostFor(group);const auto cursor=QCursor::pos();
    QString host=oldHost;
    const bool one=DockTree::leaves(m_docks.value(oldHost)).size()==1;
    if(oldHost=="main" || (!whole && (!one || isIconGroup(group) || (index(group)>=0 && m_groups[index(group)].panels.size()>1)))) {
        if(whole && group!="__toolstrip") {
            const auto column=columnGroups(group);QString first;
            for(const auto &id:column) {
                if(first.isEmpty()){if(!dockPayload(payload(id,{},true),"floating"))return;first=id;}
                else dockPayload(payload(id,{},true),"floating",first,"after");
            }
            host=hostFor(first);
        } else {
            if(!dockPayload(payload(group,panel,whole),"floating"))return;
            if(whole)host=hostFor(group);
            else for(const auto &g:m_groups)if(g.location=="floating" && g.panels.contains(panel)){host=hostFor(g.id);break;}
        }
    }
    m_dragHost=host;m_dragGroups=DockTree::leaves(m_docks.value(host));
    QTimer::singleShot(0,this,[this,host,cursor,oldHost] {
        for(auto *window:QGuiApplication::allWindows())if(window->objectName()=="floatingDock:"+host ||
            (m_dragGroups==QStringList{"__toolstrip"} && window->objectName()=="floatingToolStrip")) {
            const auto offset=host==oldHost?cursor-m_beforeWindows.value(host).topLeft():QPoint(24,4);
            m_windowDrag.start(window,cursor,offset);break;
        }
    });
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
bool WorkspaceManager::eventFilter(QObject *watched,QEvent *event) {
    if(watched==m_glassWindow && event->type()==QEvent::Hide)m_menuBlur.hide();
    if(m_menuWindow && m_menuWindow->isVisible() && event->type()==QEvent::MouseButtonPress) {
        const auto *press=static_cast<QMouseEvent*>(event);
        if(!m_menuWindow->geometry().contains(press->globalPosition().toPoint())) {
            m_menuWindow->close();
            return true;
        }
    }
    if(m_panelFlyout && m_panelFlyout->isVisible()) {
        if(event->type()==QEvent::KeyPress && static_cast<QKeyEvent*>(event)->key()==Qt::Key_Escape) {m_panelFlyout->close();return true;}
        if((watched==m_panelFlyout && event->type()==QEvent::WindowDeactivate) || event->type()==QEvent::ApplicationDeactivate) {
            const auto previous=m_panelFlyout;const auto revision=m_flyoutRevision;
            QTimer::singleShot(0,this,[this,previous,revision]{if(previous && previous==m_panelFlyout && revision==m_flyoutRevision)previous->close();});
        }
        if(event->type()==QEvent::MouseButtonPress || event->type()==QEvent::TabletPress || event->type()==QEvent::TouchBegin) {
            QPoint global;
            if(event->type()==QEvent::MouseButtonPress)global=static_cast<QMouseEvent*>(event)->globalPosition().toPoint();
            else if(event->type()==QEvent::TabletPress)global=static_cast<QTabletEvent*>(event)->globalPosition().toPoint();
            else {const auto *touch=static_cast<QTouchEvent*>(event);if(touch->points().isEmpty())return false;global=touch->points().first().globalPosition().toPoint();}
            if(!m_panelFlyout->geometry().contains(global)) {
                if(m_flyoutOwner) {
                    const QRectF rail(m_flyoutOwner->mapToGlobal({0,0}),QSizeF(m_flyoutOwner->width(),m_flyoutOwner->height()));
                    if(rail.contains(global))return false;
                    for(auto it=m_targets.cbegin();it!=m_targets.cend();++it)if(isIconGroup(it.key()) && it.value() && it.value()->isVisible() && it.value()->window() && it.value()->window()->isVisible()) {
                        auto *item=it.value().data();const QRectF other(item->mapToGlobal({0,0}),QSizeF(item->width(),item->height()));
                        if(other.contains(global))return false;
                    }
                }
                m_panelFlyout->close();return true;
            }
        }
    }
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
    const auto home=m_groups[i].home;
    for(const auto &g:m_groups)if(g.location==home && isIconGroup(g.id)){m_iconGroups.insert(group);break;}
    removeDock(group);m_groups[i].location=home;addDock(group,home,{},"after");syncToolsLocation();emit groupsChanged();
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
                                                        {"active",g.active},{"home",g.home},{"collapsed",false},{"dockHeight",g.dockHeight},{"x",g.geometry.x()},{"y",g.geometry.y()},{"width",g.geometry.width()},{"height",g.geometry.height()}});
    QJsonObject panels; for(auto it=m_panels.cbegin();it!=m_panels.cend();++it) panels.insert(it.key(),QJsonObject::fromVariantMap(it.value()));
    QSettings settings(m_settingsFile.isEmpty()?QSettings::NativeFormat:QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse");
    QJsonObject docks,windows;
    for(auto it=m_docks.cbegin();it!=m_docks.cend();++it) docks.insert(it.key(),it.value());
    for(auto it=m_windowGeometry.cbegin();it!=m_windowGeometry.cend();++it) { const auto r=it.value();windows.insert(it.key(),QJsonObject{{"x",r.x()},{"y",r.y()},{"width",r.width()},{"height",r.height()}}); }
    const QJsonObject layout{{"version",6},{"iconGroups",QJsonArray::fromStringList(m_iconGroups.values())},{"docks",docks},{"windows",windows},{"groups",groups},{"panels",panels},{"uiRevision",m_uiRevision},{"leftWidth",m_leftWidth},{"rightWidth",m_rightWidth},{"leftCollapsed",m_leftCollapsed},{"rightCollapsed",m_rightCollapsed},{"toolsFloating",m_toolsFloating},{"toolX",m_toolPosition.x()},{"toolY",m_toolPosition.y()}};
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
    if(!QList<int>{1,2,3,4,5,6}.contains(root.value("version").toInt()) || panels.size()>64 || panels.size()<5 || groups.isEmpty() || groups.size()>64) return false;
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
    bool migrated=root.value("version").toInt()!=6;
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
    if(root.value("version").toInt()>=5) {
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
    QSet<QString> icons;
    if(root.value("version").toInt()==6) {
        if(!root.value("iconGroups").isArray())return false;
        for(const auto &v:root.value("iconGroups").toArray()) {
            const auto id=v.toString();if(!groupIds.contains(id) || icons.contains(id))return false;icons.insert(id);
        }
        if(!DockTree::leaves(docks.value("main")).contains("__canvas"))return false;
    } else if(root.value("version").toInt()==5) {
        // Separate the old single canvas from any mixed utility floating tree.
        const bool inMain=DockTree::leaves(docks.value("main")).contains("__canvas");
        for(auto it=docks.begin();it!=docks.end();) {
            if(it.key()!="main")DockTree::remove(it.value(),"__canvas");
            if(it.key()!="main" && it.value().isEmpty()){windows.remove(it.key());it=docks.erase(it);}else ++it;
        }
        if(!inMain)docks["main"]=DockTree::split(docks.value("main"),DockTree::leaf("__canvas"),"horizontal",.25);
    }
    for(auto &g:restored)if(g.collapsed){icons.insert(g.id);g.collapsed=false;migrated=true;}
    m_iconGroups=icons;
    m_panels=restoredPanels; m_groups=restored;
    m_uiRevision=root.value("uiRevision").toInt(0);
    m_leftWidth=std::clamp(root.value("leftWidth").toInt(180),150,520);
    m_rightWidth=std::clamp(root.value("rightWidth").toInt(190),190,520);
    m_leftCollapsed=root.value("leftCollapsed").toBool(false);
    m_rightCollapsed=root.value("rightCollapsed").toBool(false);
    updateToolStripPosition(root.value("toolX").toInt(100),root.value("toolY").toInt(100));
    m_toolsFloating=root.value("toolsFloating").toBool(false);emit toolStripChanged();
    if(root.value("version").toInt()>=5) {m_docks=docks;m_windowGeometry=windows;syncToolsLocation();}
    else initializeDocks();
    const auto oldIcons=m_iconGroups;
    for(const auto &id:oldIcons)for(const auto &member:columnGroups(id))m_iconGroups.insert(member);
    migrated=migrated || oldIcons!=m_iconGroups;
    emit dockMetricsChanged();
    if(migrated) saveLayout();
    emit groupsChanged(); return true;
}
