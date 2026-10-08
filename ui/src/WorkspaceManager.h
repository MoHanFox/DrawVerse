#pragma once
#include <QObject>
#include <QVariantList>
#include <QHash>
#include <QRect>

class WorkspaceManager final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantList leftGroups READ leftGroups NOTIFY groupsChanged)
    Q_PROPERTY(QVariantList rightGroups READ rightGroups NOTIFY groupsChanged)
    Q_PROPERTY(QVariantList floatingGroups READ floatingGroups NOTIFY groupsChanged)
public:
    explicit WorkspaceManager(const QString &settingsFile = {}, QObject *parent = nullptr);
    QVariantList leftGroups() const { return groupsAt("left"); }
    QVariantList rightGroups() const { return groupsAt("right"); }
    QVariantList floatingGroups() const { return groupsAt("floating"); }
    Q_INVOKABLE QVariantMap panelDefinition(const QString &id) const;
    Q_INVOKABLE void setActive(const QString &group, const QString &panel);
    Q_INVOKABLE void beginDrag(const QString &group, const QString &panel, bool whole);
    Q_INVOKABLE bool dockPayload(const QString &payload, const QString &location, const QString &target = {});
    Q_INVOKABLE void detachGroup(const QString &group);
    Q_INVOKABLE void detachPanel(const QString &group, const QString &panel);
    Q_INVOKABLE void returnGroup(const QString &group);
    Q_INVOKABLE QString addCustomPanel(const QString &title, const QString &kind);
    Q_INVOKABLE void updateGeometry(const QString &group, int x, int y, int width, int height);
    Q_INVOKABLE void resetLayout();
    Q_INVOKABLE void saveLayout() const;
    bool restoreLayout();
signals:
    void groupsChanged();
protected:
    bool eventFilter(QObject *, QEvent *event) override;
private:
    struct Group { QString id, location; QStringList panels; QString active; QRect geometry{100,100,320,440}; };
    QVariantList groupsAt(const QString &location) const;
    QString payload(const QString &group, const QString &panel, bool whole) const;
    int index(const QString &id) const;
    QRect safeGeometry(QRect geometry) const;
    QString m_settingsFile;
    QList<Group> m_groups;
    QHash<QString,QVariantMap> m_panels;
    bool m_dragCancelled = false;
};
