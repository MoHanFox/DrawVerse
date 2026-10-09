#pragma once
#include "PaintCoreClient.h"
#include "WindowDrag.h"
#include <QPointer>
#include <QQuickItem>
#include <QQmlContext>
#include <QMap>

class DocumentManager : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantList documents READ documents NOTIFY documentsChanged)
    Q_PROPERTY(QVariantList windows READ windows NOTIFY groupsChanged)
    Q_PROPERTY(QString activeId READ activeId NOTIFY activeChanged)
    Q_PROPERTY(PaintCoreClient* activeClient READ activeClient NOTIFY activeChanged)
    Q_PROPERTY(QString dragTarget READ dragTarget NOTIFY dragChanged)
    Q_PROPERTY(bool dragging READ dragging NOTIFY dragChanged)
    Q_PROPERTY(bool interactionBlocked MEMBER m_interactionBlocked)
public:
    explicit DocumentManager(QObject *parent=nullptr);
    Q_INVOKABLE void configure(PaintCoreClient *initial,QObject *root);
    QVariantList documents() const;
    QVariantList windows() const;
    QString activeId() const {return m_active;}
    PaintCoreClient *activeClient() const;
    QString dragTarget() const {return m_dragTarget;}
    bool dragging() const {return m_drag.active();}
    Q_INVOKABLE QVariantMap group(const QString &host) const;
    Q_INVOKABLE PaintCoreClient *client(const QString &id) const;
    Q_INVOKABLE bool activate(const QString &id);
    Q_INVOKABLE QString newDocument(int width,int height,bool transparent=false);
    Q_INVOKABLE QString openDocument(const QUrl &url);
    Q_INVOKABLE bool moveDocument(const QString &id,const QString &host);
    Q_INVOKABLE void floatDocument(const QString &id);
    Q_INVOKABLE void returnWindow(const QString &host);
    Q_INVOKABLE void requestClose(const QString &id);
    Q_INVOKABLE bool closeDocument(const QString &id,bool discard=false);
    Q_INVOKABLE void shutdown();
    Q_INVOKABLE void updateGeometry(const QString &host,int x,int y,int width,int height);
    Q_INVOKABLE void registerTarget(const QString &host,QQuickItem *item);
    Q_INVOKABLE void beginDrag(const QString &id,bool whole=false);
signals:
    void documentsChanged();
    void groupsChanged();
    void activeChanged();
    void dragChanged();
    void closeRequested(QString id);
    void fileFailed(QString message);
    void stopped();
private:
    struct Document {QString id,host;QPointer<PaintCoreClient> client;};
    struct Group {QStringList documents;QString active;QRect geometry{100,100,900,650};};
    QList<Document> m_documents;
    QMap<QString,Group> m_groups;
    QList<QPointer<PaintCoreClient>> m_clients;
    QSet<PaintCoreClient*> m_stopped;
    QPointer<PaintCoreClient> m_initial;
    QPointer<QQmlContext> m_context;
    QMap<QString,QPointer<QQuickItem>> m_targets;
    QString m_active,m_dragTarget,m_dragHost;
    QMap<QString,Group> m_dragGroups;
    QList<Document> m_dragDocuments;
    WindowDrag m_drag;
    bool m_quitting=false;
    bool m_interactionBlocked=false;
    int index(const QString &id) const;
    QString add(PaintCoreClient *client);
    QString create(int width,int height,bool transparent,const QUrl &url);
    void track(PaintCoreClient *client);
};
