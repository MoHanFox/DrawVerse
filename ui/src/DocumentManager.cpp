#include "DocumentManager.h"
#include <QQmlEngine>
#include <QQuickWindow>
#include <QGuiApplication>
#include <QCursor>
#include <QTimer>
#include <QUuid>
#include <QScreen>
#include <algorithm>

namespace {QString uid(){return QUuid::createUuid().toString(QUuid::WithoutBraces);}}
DocumentManager::DocumentManager(QObject *parent):QObject(parent),m_drag(this) {
    m_groups.insert("main",{});
    connect(&m_drag,&WindowDrag::moved,this,[this](QPoint global,bool suppressed) {
        QString target;
        if(!suppressed)for(auto it=m_targets.cbegin();it!=m_targets.cend();++it) {
            auto *item=it.value().data();if(!item || !item->isVisible() || !item->window() || !item->window()->isVisible() || it.key()==m_dragHost)continue;
            const QRectF area(item->mapToGlobal({0,0}),QSizeF(item->width(),std::min(qreal(28),item->height())));
            if(area.contains(global)){target=it.key();break;}
        }
        if(target!=m_dragTarget){m_dragTarget=target;emit dragChanged();}
    });
    connect(&m_drag,&WindowDrag::finished,this,[this](bool cancelled) {
        const auto host=m_dragHost,target=m_dragTarget;m_dragHost.clear();m_dragTarget.clear();emit dragChanged();
        if(cancelled){m_groups=m_dragGroups;m_documents=m_dragDocuments;emit groupsChanged();emit documentsChanged();return;}
        if(!target.isEmpty()) {
            const auto ids=m_groups.value(host).documents;
            for(const auto &id:ids)moveDocument(id,target);
        }
    });
}
void DocumentManager::track(PaintCoreClient *client) {
    // Invokable getters expose these objects to JS; ownership remains with the
    // application/manager, including the externally supplied first client.
    QQmlEngine::setObjectOwnership(client,QQmlEngine::CppOwnership);
    m_clients.append(client);
    connect(client,&PaintCoreClient::stateChanged,this,&DocumentManager::documentsChanged);
    connect(client,&PaintCoreClient::stopped,this,[this,client] {
        m_stopped.insert(client);
        if(m_quitting && std::all_of(m_clients.cbegin(),m_clients.cend(),[this](const auto &c){return !c || m_stopped.contains(c);}))emit stopped();
    });
}
void DocumentManager::configure(PaintCoreClient *initial,QObject *root) {
    if(m_initial || !initial || !root)return;
    m_initial=initial;
    if(auto *context=qmlContext(root))m_context=context->engine()->rootContext();
    track(initial);activate(add(initial));
}
int DocumentManager::index(const QString &id) const {for(int i=0;i<m_documents.size();++i)if(m_documents[i].id==id)return i;return -1;}
QString DocumentManager::add(PaintCoreClient *client) {
    const auto id=uid();m_documents.append({id,"main",client});
    auto &main=m_groups["main"];main.documents.append(id);main.active=id;
    emit documentsChanged();emit groupsChanged();return id;
}
PaintCoreClient *DocumentManager::client(const QString &id) const {const int at=index(id);return at<0?nullptr:m_documents[at].client.data();}
PaintCoreClient *DocumentManager::activeClient() const {auto *value=client(m_active);return value?value:m_initial.data();}
QVariantList DocumentManager::documents() const {
    QVariantList result;for(const auto &d:m_documents)if(d.client)result.append(QVariantMap{{"id",d.id},{"host",d.host},{"client",QVariant::fromValue(d.client.data())},{"title",d.client->documentName()},{"modified",d.client->modified()}});return result;
}
QVariantMap DocumentManager::group(const QString &host) const {
    const auto g=m_groups.value(host);return {{"id",host},{"documents",g.documents},{"active",g.active},{"x",g.geometry.x()},{"y",g.geometry.y()},{"width",g.geometry.width()},{"height",g.geometry.height()}};
}
QVariantList DocumentManager::windows() const {QVariantList result;for(auto it=m_groups.cbegin();it!=m_groups.cend();++it)if(it.key()!="main")result.append(group(it.key()));return result;}
bool DocumentManager::activate(const QString &id) {
    auto *value=client(id);if(!value)return false;
    if(m_interactionBlocked && !m_active.isEmpty() && id!=m_active)return false;
    if(id==m_active && m_groups.value(m_documents[index(id)].host).active==id)return true;
    if(auto *old=client(m_active);old && old->drawing())return id==m_active;
    m_active=id;m_groups[m_documents[index(id)].host].active=id;
    if(m_context && m_context->contextProperty("PaintClient").value<QObject*>()!=value)m_context->setContextProperty("PaintClient",value);
    emit activeChanged();emit groupsChanged();return true;
}
QString DocumentManager::create(int width,int height,bool transparent,const QUrl &url) {
    if(m_quitting || !m_initial || width<1 || height<1 || width>1000000 || height>1000000)return {};
    if(auto *active=activeClient();active && active->drawing())return {};
    auto *created=new PaintCoreClient(this,m_initial->settingsFile());track(created);
    const auto id=add(created);auto initialized=std::make_shared<bool>(false);
    connect(created,&PaintCoreClient::stateChanged,this,[created,width,height,transparent,url,initialized] {
        if(*initialized || !created->ready())return;
        *initialized=true;
        if(!url.isEmpty())created->openDocument(url);
        else if(transparent)created->newTransparentDocument(width,height);else created->newDocument(width,height);
    });
    if(!url.isEmpty()) {
        const auto completed=std::make_shared<bool>(false);
        connect(created,&PaintCoreClient::fileFinished,this,[this,id,created,completed](bool success) {
            if(*completed)return;
            *completed=true;
            if(!success){emit fileFailed(created->lastError());closeDocument(id,true);}
        });
    }
    activate(id);return id;
}
QString DocumentManager::newDocument(int width,int height,bool transparent){return create(width,height,transparent,{});}
QString DocumentManager::openDocument(const QUrl &url){if(url.isEmpty())return {};return create(960,640,false,url);}
bool DocumentManager::moveDocument(const QString &id,const QString &host) {
    const int at=index(id);if(at<0 || !m_groups.contains(host) || m_documents[at].client->drawing())return false;
    if(auto *active=activeClient();active && active->drawing())return false;
    const auto old=m_documents[at].host;if(old==host){activate(id);return true;}
    auto &source=m_groups[old];source.documents.removeAll(id);
    if(source.active==id)source.active=source.documents.value(0);
    if(source.documents.isEmpty() && old!="main")m_groups.remove(old);
    auto &target=m_groups[host];target.documents.append(id);target.active=id;m_documents[at].host=host;
    emit documentsChanged();emit groupsChanged();activate(id);return true;
}
void DocumentManager::floatDocument(const QString &id) {
    if(m_interactionBlocked || !client(id) || client(id)->drawing())return;
    if(auto *active=activeClient();active && active->drawing())return;
    const auto host=uid();m_groups.insert(host,{});
    auto &geometry=m_groups[host].geometry;geometry.moveTopLeft(QCursor::pos()-QPoint(24,12));
    if(auto *screen=QGuiApplication::screenAt(QCursor::pos()))geometry.moveTopLeft(screen->availableGeometry().topLeft()+QPoint(40,40));
    moveDocument(id,host);
}
void DocumentManager::returnWindow(const QString &host){const auto ids=m_groups.value(host).documents;for(const auto &id:ids)moveDocument(id,"main");}
void DocumentManager::requestClose(const QString &id){if(!m_interactionBlocked && client(id))emit closeRequested(id);}
bool DocumentManager::closeDocument(const QString &id,bool discard) {
    const int at=index(id);if(at<0)return false;auto *value=m_documents[at].client.data();
    if(!value || value->drawing() || (!discard && (value->modified() || value->fileBusy())))return false;
    const auto host=m_documents[at].host;auto &g=m_groups[host];g.documents.removeAll(id);if(g.active==id)g.active=g.documents.value(0);
    if(g.documents.isEmpty() && host!="main")m_groups.remove(host);m_documents.removeAt(at);
    if(m_active==id){m_active.clear();if(!m_documents.isEmpty())activate(m_documents.last().id);else {if(m_context)m_context->setContextProperty("PaintClient",m_initial.data());emit activeChanged();}}
    emit documentsChanged();emit groupsChanged();value->shutdown();return true;
}
void DocumentManager::shutdown() {
    if(m_quitting)return;m_quitting=true;m_drag.finish(true);
    for(const auto &c:m_clients)if(c && !m_stopped.contains(c))c->shutdown();
    if(std::all_of(m_clients.cbegin(),m_clients.cend(),[this](const auto &c){return !c || m_stopped.contains(c);}))emit stopped();
}
void DocumentManager::updateGeometry(const QString &host,int x,int y,int width,int height){if(host!="main" && m_groups.contains(host))m_groups[host].geometry=QRect(x,y,std::clamp(width,320,4000),std::clamp(height,200,3000));}
void DocumentManager::registerTarget(const QString &host,QQuickItem *item){m_targets[host]=item;}
void DocumentManager::beginDrag(const QString &id,bool whole) {
    const int at=index(id);if(at<0 || m_interactionBlocked || m_drag.active() || !m_dragHost.isEmpty() || m_documents[at].client->drawing())return;
    if(auto *active=activeClient();active && active->drawing())return;
    m_dragGroups=m_groups;m_dragDocuments=m_documents;
    auto host=m_documents[at].host;
    if(host=="main" || !whole) {floatDocument(id);host=m_documents[index(id)].host;}
    m_dragHost=host;
    const auto cursor=QCursor::pos();
    QTimer::singleShot(0,this,[this,host,cursor] {
        for(auto *window:QGuiApplication::allWindows())if(window->objectName()=="documentWindow:"+host) {
            const auto offset=m_dragGroups.contains(host)?cursor-m_dragGroups.value(host).geometry.topLeft():QPoint(24,12);
            m_drag.start(window,cursor,offset);break;
        }
    });
}
