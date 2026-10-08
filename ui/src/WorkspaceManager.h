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
    Q_PROPERTY(QStringList visiblePanels READ visiblePanels NOTIFY groupsChanged)
    Q_PROPERTY(QStringList allPanels READ allPanels NOTIFY groupsChanged)
    Q_PROPERTY(bool leftCollapsed READ leftCollapsed WRITE setLeftCollapsed NOTIFY dockMetricsChanged)
    Q_PROPERTY(bool rightCollapsed READ rightCollapsed WRITE setRightCollapsed NOTIFY dockMetricsChanged)
    Q_PROPERTY(int leftDockWidth READ leftDockWidth WRITE setLeftDockWidth NOTIFY dockMetricsChanged)
    Q_PROPERTY(int rightDockWidth READ rightDockWidth WRITE setRightDockWidth NOTIFY dockMetricsChanged)
    Q_PROPERTY(bool dockingSuppressed READ dockingSuppressed NOTIFY dragModifiersChanged)
    Q_PROPERTY(bool toolsFloating READ toolsFloating NOTIFY toolStripChanged)
    Q_PROPERTY(int toolStripX READ toolStripX NOTIFY toolStripChanged)
    Q_PROPERTY(int toolStripY READ toolStripY NOTIFY toolStripChanged)
public:
    explicit WorkspaceManager(const QString &settingsFile = {}, QObject *parent = nullptr);
    QVariantList leftGroups() const { return groupsAt("left"); }
    QVariantList rightGroups() const { return groupsAt("right"); }
    QVariantList floatingGroups() const { return groupsAt("floating"); }
    QStringList visiblePanels() const;
    QStringList allPanels() const;
    bool leftCollapsed() const { return m_leftCollapsed; }
    bool rightCollapsed() const { return m_rightCollapsed; }
    int leftDockWidth() const { return m_leftWidth; }
    int rightDockWidth() const { return m_rightWidth; }
    bool dockingSuppressed() const { return m_dockingSuppressed; }
    bool toolsFloating() const {return m_toolsFloating;}
    int toolStripX() const {return m_toolPosition.x();}
    int toolStripY() const {return m_toolPosition.y();}
    Q_INVOKABLE void beginToolStripDrag();
    Q_INVOKABLE void floatToolStrip(int x,int y);
    Q_INVOKABLE void updateToolStripPosition(int x,int y);
    Q_INVOKABLE bool dockToolStrip(const QString &payload);
    void setLeftCollapsed(bool value);
    void setRightCollapsed(bool value);
    void setLeftDockWidth(int value);
    void setRightDockWidth(int value);
    Q_INVOKABLE QVariantMap groupDefinition(const QString &id) const;
    Q_INVOKABLE QVariantMap panelDefinition(const QString &id) const;
    Q_INVOKABLE void setActive(const QString &group, const QString &panel);
    Q_INVOKABLE void beginDrag(const QString &group, const QString &panel, bool whole);
    Q_INVOKABLE bool dockPayload(const QString &payload, const QString &location, const QString &target = {},
                                const QString &placement = "merge", const QString &beforePanel = {});
    Q_INVOKABLE void setGroupCollapsed(const QString &group, bool collapsed);
    Q_INVOKABLE void updateDockHeight(const QString &group, int height);
    Q_INVOKABLE void hidePanel(const QString &group, const QString &panel = {});
    Q_INVOKABLE void showPanel(const QString &panel);
    Q_INVOKABLE void detachGroup(const QString &group);
    Q_INVOKABLE void detachPanel(const QString &group, const QString &panel);
    Q_INVOKABLE void returnGroup(const QString &group);
    Q_INVOKABLE QString addCustomPanel(const QString &title, const QString &kind);
    Q_INVOKABLE void updateGeometry(const QString &group, int x, int y, int width, int height);
    Q_INVOKABLE void resetLayout();
    Q_INVOKABLE void applyReferenceLayout(int x,int y,int width,int height);
    bool needsReferenceLayout() const { return m_uiRevision<2; }
    Q_INVOKABLE void saveLayout() const;
    bool restoreLayout();
signals:
    void groupsChanged();
    void dockMetricsChanged();
    void groupStateChanged(const QString &group);
    void dragModifiersChanged();
    void toolStripChanged();
protected:
    bool eventFilter(QObject *, QEvent *event) override;
private:
    struct Group {
        QString id, location;
        QStringList panels;
        QString active;
        QRect geometry{100,100,320,440};
        QString home = "right";
        bool collapsed = false;
        int dockHeight = 320;
    };
    QVariantList groupsAt(const QString &location) const;
    QString payload(const QString &group, const QString &panel, bool whole) const;
    int index(const QString &id) const;
    QRect safeGeometry(QRect geometry) const;
    QString m_settingsFile;
    QList<Group> m_groups;
    QHash<QString,QVariantMap> m_panels;
    bool m_dragCancelled = false;
    bool m_dragActive = false, m_dockingSuppressed = false;
    bool m_leftCollapsed = false, m_rightCollapsed = false;
    int m_leftWidth = 230, m_rightWidth = 300;
    int m_uiRevision = 0;
    bool m_toolsFloating = false;
    QPoint m_toolPosition{100,100};
};
