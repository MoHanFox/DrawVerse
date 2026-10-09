#pragma once
#include <QObject>
#include <QAbstractNativeEventFilter>
#include <QVariantList>
#include <QHash>
#include <QRect>
#include <QPointer>
#include <QMap>
#include "DockTree.h"
#include "WindowDrag.h"
#include "MenuBlurLayer.h"
class QQuickItem;
class QWindow;

class WorkspaceManager final : public QObject, public QAbstractNativeEventFilter {
    Q_OBJECT
    Q_PROPERTY(bool windowsWindowFrames READ windowsWindowFrames CONSTANT)
    Q_PROPERTY(QVariantList leftGroups READ leftGroups NOTIFY groupsChanged)
    Q_PROPERTY(QVariantList rightGroups READ rightGroups NOTIFY groupsChanged)
    Q_PROPERTY(QVariantList floatingGroups READ floatingGroups NOTIFY groupsChanged)
    Q_PROPERTY(QVariantList floatingWindows READ floatingWindows NOTIFY groupsChanged)
    Q_PROPERTY(QStringList visiblePanels READ visiblePanels NOTIFY groupsChanged)
    Q_PROPERTY(QStringList allPanels READ allPanels NOTIFY groupsChanged)
    Q_PROPERTY(bool leftCollapsed READ leftCollapsed WRITE setLeftCollapsed NOTIFY dockMetricsChanged)
    Q_PROPERTY(bool rightCollapsed READ rightCollapsed WRITE setRightCollapsed NOTIFY dockMetricsChanged)
    Q_PROPERTY(int leftDockWidth READ leftDockWidth WRITE setLeftDockWidth NOTIFY dockMetricsChanged)
    Q_PROPERTY(int rightDockWidth READ rightDockWidth WRITE setRightDockWidth NOTIFY dockMetricsChanged)
    Q_PROPERTY(bool dockingSuppressed READ dockingSuppressed NOTIFY dragModifiersChanged)
    Q_PROPERTY(QString dragTarget READ dragTarget NOTIFY dragModifiersChanged)
    Q_PROPERTY(QString dragPlacement READ dragPlacement NOTIFY dragModifiersChanged)
    Q_PROPERTY(bool dragging READ dragging NOTIFY dragModifiersChanged)
    Q_PROPERTY(QStringList dragGroups READ dragGroups NOTIFY dragModifiersChanged)
    Q_PROPERTY(bool toolsFloating READ toolsFloating NOTIFY toolStripChanged)
    Q_PROPERTY(int toolStripX READ toolStripX NOTIFY toolStripChanged)
    Q_PROPERTY(int toolStripY READ toolStripY NOTIFY toolStripChanged)
public:
    bool windowsWindowFrames() const;
    Q_INVOKABLE QRect availableScreenGeometry(QWindow *window) const;
    Q_INVOKABLE void watchMenuWindow(QWindow *window, bool visible);
    Q_INVOKABLE void watchPanelFlyout(QWindow *window,QQuickItem *owner,bool visible);
    Q_INVOKABLE bool setMenuBarBlur(QWindow *window, bool enabled,int height=28);
    quintptr menuBlurWindowHandle() const {return m_menuBlur.nativeHandle();}
    Q_INVOKABLE bool setWindowCornerRadius(QWindow *window, int radius);
    explicit WorkspaceManager(const QString &settingsFile = {}, QObject *parent = nullptr);
    ~WorkspaceManager() override;
    bool nativeEventFilter(const QByteArray &eventType,void *message,qintptr *result) override;
    QVariantList leftGroups() const { return groupsAt("left"); }
    QVariantList rightGroups() const { return groupsAt("right"); }
    QVariantList floatingGroups() const { return groupsAt("floating"); }
    QVariantList floatingWindows() const;
    Q_INVOKABLE QVariantList layoutItems(const QString &host,int width,int height) const;
    Q_INVOKABLE void setSplitRatio(const QString &host,const QString &split,double ratio);
    Q_INVOKABLE void returnWindow(const QString &host);
    QStringList visiblePanels() const;
    QStringList allPanels() const;
    bool leftCollapsed() const { return m_leftCollapsed; }
    bool rightCollapsed() const { return m_rightCollapsed; }
    int leftDockWidth() const { return m_leftWidth; }
    int rightDockWidth() const { return m_rightWidth; }
    bool dockingSuppressed() const { return m_dockingSuppressed; }
    QString dragTarget() const {return m_dragTarget;}
    QString dragPlacement() const {return m_dragPlacement;}
    bool dragging() const {return m_windowDrag.active();}
    QStringList dragGroups() const {return m_dragGroups;}
    Q_INVOKABLE void registerTarget(const QString &group,QQuickItem *item);
    Q_INVOKABLE void registerWorkspace(QQuickItem *item);
    Q_INVOKABLE QStringList columnGroups(const QString &group) const;
    Q_INVOKABLE void setColumnCollapsed(const QString &group,bool collapsed);
    bool toolsFloating() const {return m_toolsFloating;}
    int toolStripX() const {return m_toolPosition.x();}
    int toolStripY() const {return m_toolPosition.y();}
    Q_INVOKABLE void beginToolStripDrag();
    Q_INVOKABLE void floatToolStrip(int x,int y);
    Q_INVOKABLE void updateToolStripPosition(int x,int y);
    Q_INVOKABLE bool dockToolStrip(const QString &payload,const QString &location="main",const QString &target={},const QString &placement="left");
    void setLeftCollapsed(bool value);
    void setRightCollapsed(bool value);
    void setLeftDockWidth(int value);
    void setRightDockWidth(int value);
    Q_INVOKABLE QVariantMap groupDefinition(const QString &id) const;
    Q_INVOKABLE QVariantMap panelDefinition(const QString &id) const;
    Q_INVOKABLE void setActive(const QString &group, const QString &panel);
    Q_INVOKABLE void swapPanelTabs(const QString &group,const QString &first,const QString &second);
    Q_INVOKABLE void beginDrag(const QString &group, const QString &panel, bool whole);
    Q_INVOKABLE bool dockPayload(const QString &payload, const QString &location, const QString &target = {},
                                const QString &placement = "merge", const QString &beforePanel = {});
    Q_INVOKABLE void updateDockHeight(const QString &group, int height);
    Q_INVOKABLE void hidePanel(const QString &group, const QString &panel = {});
    Q_INVOKABLE void showPanel(const QString &panel);
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
    void dockMetricsChanged();
    void groupStateChanged(const QString &group);
    void dragModifiersChanged();
    void toolStripChanged();
    void layoutChanged();
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
    bool m_dockingSuppressed = false;
    bool m_leftCollapsed = false, m_rightCollapsed = false;
    int m_leftWidth = 230, m_rightWidth = 300;
    int m_uiRevision = 0;
    bool m_toolsFloating = false;
    QPoint m_toolPosition{100,100};
    QPointer<QWindow> m_menuWindow;
    QPointer<QWindow> m_glassWindow;
    quintptr m_glassHandle=0;
    MenuBlurLayer m_menuBlur{this};
    QList<QPointer<QWindow>> m_panelFlyouts;
    QMap<QString,DockTree::Node> m_docks;
    QHash<QString,QRect> m_windowGeometry;
    QSet<QString> m_iconGroups;
    QMap<QString,QPointer<QQuickItem>> m_targets;
    QPointer<QQuickItem> m_workspaceArea;
    WindowDrag m_windowDrag;
    QString m_dragTarget,m_dragPlacement,m_dragHost;
    QStringList m_dragGroups;
    QMap<QString,DockTree::Node> m_beforeDocks;
    QHash<QString,QRect> m_beforeWindows;
    QList<Group> m_beforeGroups;
    QSet<QString> m_beforeIcons;
    bool isIconGroup(const QString &group) const;
    void updateDragTarget(QPoint global,bool suppressed);
    void initializeDocks();
    QString hostFor(const QString &group) const;
    void removeDock(const QString &group);
    void addDock(const QString &group,const QString &location,const QString &target,const QString &edge,bool independentColumn=false);
    void syncToolsLocation();
    QSizeF dockMinimum(const DockTree::Node &node) const;
};
