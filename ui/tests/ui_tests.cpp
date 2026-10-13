#include "CanvasItem.h"
#include "ColorWheelItem.h"
#include "SelectionOverlay.h"
#include "WorkspaceManager.h"
#include "DocumentManager.h"
#include "UiScale.h"
#include "MenuSurface.h"
#include <QtTest>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQmlProperty>
#include <QQuickWindow>
#include <QQuickItemGrabResult>
#include <QQuickStyle>
#include <QTemporaryDir>
#include <QSettings>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QPointingDevice>
#include <QTabletEvent>
#include <QDir>
#include <QFileInfo>
#include <QMimeData>
#include <QDropEvent>
#include <QDragEnterEvent>
#include <QDragMoveEvent>
#include <QWheelEvent>
#include <QHoverEvent>
#include <QStyleHints>
#include <QPainter>
#include <QScreen>
#include <QScopeGuard>
#include <cmath>
#include <limits>
#ifdef Q_OS_WIN
#include <QtCore/qt_windows.h>
#include <thread>
#endif

namespace {
class BlurTestBackdrop final : public QQuickPaintedItem {
public:
    explicit BlurTestBackdrop(QQuickItem *parent):QQuickPaintedItem(parent) { setOpaquePainting(true); }
    void showCornerPattern() {m_cornerPattern=true;update();}
    void paint(QPainter *painter) override {
        for(int x=0;x<width();x+=80)
            painter->fillRect(QRectF(x,0,80,height()),(x/80)%2?QColor(230,40,210):QColor(30,230,60));
        if(m_cornerPattern)for(int y=0;y<24;y+=2)for(int x=0;x<24;x+=2)
            painter->fillRect(QRectF(x,y,2,2),(x/2+y/2)%2?Qt::white:Qt::black);
    }
private:
    bool m_cornerPattern=false;
};
QWindowList applicationWindows(){auto windows=QGuiApplication::allWindows();windows.removeIf([](QWindow *window){return window->objectName()=="applicationOutlineWindow";});return windows;}
int visibleWindowsCount(){int count=0;for(auto *window:applicationWindows())if(window->isVisible())++count;return count;}
QQuickItem *findVisualItem(QQuickItem *root,const QString &name) {
    if(root->objectName()==name) return root;
    for(auto *item:root->childItems()) if(auto *found=findVisualItem(item,name)) return found;
    return nullptr;
}
}

class UiTests final : public QObject {
    Q_OBJECT
    QTemporaryDir m_preferences;
private slots:
    void initTestCase() {
        QVERIFY(m_preferences.isValid());
        QSettings::setPath(QSettings::IniFormat,QSettings::UserScope,m_preferences.path());
        QSettings::setPath(QSettings::IniFormat,QSettings::SystemScope,m_preferences.path());
    }
    void init() {
        QSettings settings(QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse");
        QVERIFY(settings.fileName().startsWith(m_preferences.path()+"/"));
        settings.clear();settings.sync();QCOMPARE(settings.status(),QSettings::NoError);
    }
    void foldedNeighboursKeepTheirSizes_data() {
        QTest::addColumn<int>("toolWidth");for(int width:{36,55,72})QTest::newRow(qPrintable(QString::number(width)))<<width;
    }
    void foldedNeighboursKeepTheirSizes() {
        QFETCH(int,toolWidth);QTemporaryDir temp;const auto path=temp.filePath("layout.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());QTRY_VERIFY(client.ready());
        const auto brush=workspace.groupForPanel("brush");workspace.detachGroup(brush);workspace.setToolStripWidth(toolWidth);QVERIFY(workspace.dockToolStrip("drawverse-tools-v1","floating",brush,"left"));
        QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+brush)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());floating->resize(420,650);QTest::qWait(50);
        auto *tools=findVisualItem(floating->contentItem(),"dockTile:__toolstrip");QVERIFY(tools);QTRY_COMPARE(tools->width(),qreal(toolWidth));const auto expanded=floating->geometry();
        workspace.setColumnCollapsed(brush,true);QTRY_COMPARE(tools->width(),qreal(toolWidth));QTRY_COMPARE(floating->width(),toolWidth+31);QCOMPARE(floating->position(),expanded.topLeft());QCOMPARE(floating->height(),expanded.height());
        const auto preview=qEnvironmentVariable("DRAWVERSE_FOLD_SIZE_PREVIEW");if(!preview.isEmpty()){QTest::qWait(50);QVERIFY(floating->grabWindow().save(preview+"."+QString::number(toolWidth)+".png"));}
        const auto adjusted=toolWidth==36?72:36;workspace.setToolStripWidth(adjusted);QTRY_COMPARE(tools->width(),qreal(adjusted));QTRY_COMPARE(floating->width(),adjusted+31);workspace.setToolStripWidth(toolWidth);QTRY_COMPARE(floating->width(),toolWidth+31);
        workspace.saveLayout();WorkspaceManager restored(path);const auto saved=restored.floatingWindows().first().toMap();QCOMPARE(saved.value("width").toInt(),toolWidth+31);restored.setColumnCollapsed(brush,false);QCOMPARE(restored.floatingWindows().first().toMap().value("width").toInt(),expanded.width());
        QSettings settings(path,QSettings::IniFormat);auto legacy=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();legacy.remove("retainedWidths");auto windows=legacy.value("windows").toObject();auto oldWindow=windows.value(brush).toObject();oldWindow.insert("width",expanded.width());windows.insert(brush,oldWindow);legacy.insert("windows",windows);settings.setValue("workspace",QJsonDocument(legacy).toJson());settings.sync();WorkspaceManager migrated(path);QCOMPARE(migrated.floatingWindows().first().toMap().value("width").toInt(),toolWidth+31);migrated.setColumnCollapsed(brush,false);QCOMPARE(migrated.floatingWindows().first().toMap().value("width").toInt(),std::min(expanded.width(),floating->screen()->availableGeometry().width()));
        for(int i=0;i<3;++i){workspace.setColumnCollapsed(brush,false);QTRY_COMPARE(floating->geometry(),expanded);QTRY_COMPARE(tools->width(),qreal(toolWidth));workspace.setColumnCollapsed(brush,true);QTRY_COMPARE(floating->width(),toolWidth+31);}
        workspace.setColumnCollapsed(brush,false);QTRY_COMPARE(floating->geometry(),expanded);QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void independentFoldedColumnsKeepExpandedNeighboursAndRestore() {
        QTemporaryDir temp;const auto path=temp.filePath("layout.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());QTRY_VERIFY(client.ready());
        const auto brush=workspace.groupForPanel("brush"),color=workspace.groupForPanel("color");workspace.detachGroup(brush);workspace.detachGroup(color);
        const auto payload=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",color},{"whole",true}}).toJson());QVERIFY(workspace.dockPayload(payload,"floating",brush,"column-right"));
        QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+brush)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());floating->resize(610,550);QTest::qWait(50);
        auto *a=findVisualItem(floating->contentItem(),"dockTile:"+brush),*b=findVisualItem(floating->contentItem(),"dockTile:"+color);QVERIFY(a && b);QTRY_COMPARE(floating->size(),QSize(610,550));QTRY_COMPARE(a->height(),qreal(548));QTRY_COMPARE(b->height(),qreal(548));QTRY_VERIFY(std::abs(a->width()+b->width()-607)<1);const auto original=floating->geometry();const auto as=a->size(),bs=b->size();QCOMPARE(workspace.columnGroups(brush),QStringList{brush});
        // Native host sizes are integral; split ratios can give fractional logical pixels.
        const auto sameSize=[](QSizeF x,QSizeF y){return std::abs(x.width()-y.width())<1 && std::abs(x.height()-y.height())<1;};
        workspace.setColumnCollapsed(brush,true);QTRY_VERIFY2(sameSize(b->size(),bs),qPrintable(QString("neighbour %1x%2 expected %3x%4; host %5x%6").arg(b->width()).arg(b->height()).arg(bs.width()).arg(bs.height()).arg(floating->width()).arg(floating->height())));QTRY_COMPARE(floating->width(),qRound(bs.width())+31);QCOMPARE(floating->height(),original.height());
        workspace.setColumnCollapsed(color,true);QTRY_COMPARE(floating->width(),59);workspace.saveLayout();WorkspaceManager restored(path);restored.setColumnCollapsed(brush,false);restored.setColumnCollapsed(color,false);QCOMPARE(restored.floatingWindows().first().toMap().value("width").toInt(),original.width());
        workspace.setColumnCollapsed(brush,false);QTRY_VERIFY(sameSize(a->size(),as));workspace.setColumnCollapsed(color,false);QTRY_COMPARE(floating->geometry(),original);QTRY_VERIFY(sameSize(b->size(),bs));
        workspace.setColumnCollapsed(brush,true);QTRY_VERIFY(sameSize(b->size(),bs));const auto expandedHeight=b->height();floating->resize(floating->width()+30,floating->height()+40);QTRY_VERIFY(b->width()>bs.width());QTRY_VERIFY(b->height()>expandedHeight);QTRY_COMPARE(b->height(),qreal(floating->height()-2));const auto manual=b->size();workspace.setColumnCollapsed(brush,false);QTRY_VERIFY(sameSize(b->size(),manual));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void categorizedMenusHoverIntoCompactNativeSubmenus_data() {
        QTest::addColumn<bool>("fallback");QTest::newRow("window-popups")<<false;QTest::newRow("side-window-fallback")<<true;
    }
    void categorizedMenusHoverIntoCompactNativeSubmenus() {
        QFETCH(bool,fallback);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));
        QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto area=main->screen()->availableGeometry();main->setPosition(area.topLeft());main->resize(std::min(1100,area.width()),std::min(700,area.height()));
        const auto hover=[&](QObject *menu,const QString &text) -> QQuickItem* {
            auto *content=qobject_cast<QQuickItem*>(menu->property("contentItem").value<QObject*>());if(!content || !content->window())return nullptr;
            auto *row=findVisualItem(content,"glassMenuItem:"+text);if(!row)return nullptr;
            QTest::qWait(30);
            // On a narrow high-DPI screen a submenu may flip left and cover its parent's
            // center. Hover the exposed trailing edge, where the category arrow lives.
            QTest::mouseMove(content->window(),row->mapToScene({row->width()-12,row->height()/2}).toPoint());return row;
        };
        auto *image=main->findChild<QObject*>("imageMenu"),*adjust=main->findChild<QObject*>("imageAdjustSubmenu"),*rotate=main->findChild<QObject*>("imageRotateSubmenu");QVERIFY(image && adjust && rotate);
        if(fallback)for(const auto &name:{"imageMenu","imageAdjustSubmenu","imageRotateSubmenu","filterMenu","filterBlurSubmenu"})main->findChild<QObject*>(name)->setProperty("windowPopupsAvailable",false);
        auto *entry=findVisualItem(main->contentItem(),"menuEntry:图像");QVERIFY(entry);
        QTRY_COMPARE(main->position(),area.topLeft());QTRY_VERIFY(main->isExposed());QTest::qWait(50);
        if(QGuiApplication::platformName()!="offscreen"){QCursor::setPos(entry->mapToGlobal({entry->width()/2,14}).toPoint());QTest::qWait(30);}
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,entry->mapToScene({entry->width()/2,14}).toPoint());QTRY_VERIFY(image->property("opened").toBool());auto *row=hover(image,"调整");QVERIFY(row);QTRY_VERIFY(row->property("hovered").toBool());QTRY_VERIFY(adjust->property("opened").toBool());
        auto *arrow=qobject_cast<QQuickItem*>(row->property("arrow").value<QObject*>());QVERIFY(arrow);QCOMPARE(arrow->width(),qreal(6));QCOMPARE(arrow->height(),qreal(8));QVERIFY(row->height()<30);
        auto *parentContent=qobject_cast<QQuickItem*>(image->property("contentItem").value<QObject*>()),*childContent=qobject_cast<QQuickItem*>(adjust->property("contentItem").value<QObject*>());QVERIFY(parentContent && childContent);QVERIFY(childContent->window()!=parentContent->window());
        if(fallback){QVERIFY(image->property("sideWindow").value<QObject*>());QVERIFY(adjust->property("sideWindow").value<QObject*>());}
        const auto preview=qEnvironmentVariable("DRAWVERSE_MENU_LAYER_PREVIEW");if(!preview.isEmpty() && !fallback){QTest::qWait(50);QVERIFY(childContent->window()->grabWindow().save(preview+".menu.png"));}
        const auto childGeometry=childContent->window()->geometry();QVERIFY(area.adjusted(-1,-1,1,1).contains(childGeometry));
        // Cross the first row's edge gradually, with the menu bar actively owning the chain.
        const auto start=row->mapToGlobal({row->width()-12,row->height()/2}).toPoint();
        const auto finish=childContent->mapToGlobal({20,10}).toPoint();
        QPointer<QQuickWindow> parentHost=parentContent->window(),childHost=childContent->window();
        const int distance=std::max(std::abs(finish.x()-start.x()),std::abs(finish.y()-start.y()));
        for(int step=1;step<=distance;++step) {
            const auto global=start+(finish-start)*qreal(step)/distance;
            auto *host=childGeometry.contains(global)?childHost.data():parentHost.data();QVERIFY2(host,"menu host vanished during first-row traversal");
            QTest::mouseMove(host,host->mapFromGlobal(global),1);
            QVERIFY2(image->property("opened").toBool(),qPrintable(QString("first-row move %1/%2 at %3,%4 (start %5,%6 child %7,%8) closed parent").arg(step).arg(distance).arg(global.x()).arg(global.y()).arg(start.x()).arg(start.y()).arg(childGeometry.x()).arg(childGeometry.y())));
            QVERIFY(!main->findChild<QObject*>("fileMenuPopup")->property("visible").toBool());
        }
        if(QGuiApplication::platformName()!="offscreen") {
            // Exercise real native hover dispatch, including popup mouse grabs.
            QCursor::setPos(start);QTest::qWait(30);
            for(int step=1;step<=distance;++step) {
                QCursor::setPos(start+(finish-start)*qreal(step)/distance);QTest::qWait(5);
                QVERIFY2(image->property("opened").toBool(),qPrintable(QString("native first-row move %1/%2 closed parent").arg(step).arg(distance)));
                QVERIFY(!main->findChild<QObject*>("fileMenuPopup")->property("visible").toBool());
            }
        }
        QTest::mouseMove(childContent->window(),childContent->mapToScene({20,10}).toPoint());QVERIFY(image->property("opened").toBool());QVERIFY(adjust->property("opened").toBool());
        auto *rotateRow=hover(image,"画布旋转");QVERIFY(rotateRow);QTRY_VERIFY(rotateRow->property("hovered").toBool());QTRY_VERIFY(rotate->property("opened").toBool());QTRY_VERIFY(!adjust->property("visible").toBool());
        QVERIFY(!main->findChild<QObject*>("canvasRotateCwAction")->property("enabled").toBool());
        auto *fileEntry=findVisualItem(main->contentItem(),"menuEntry:文件");QVERIFY(fileEntry);
        auto *rotateContent=qobject_cast<QQuickItem*>(rotate->property("contentItem").value<QObject*>());QVERIFY(rotateContent && rotateContent->window());
        QTest::mouseMove(rotateContent->window(),rotateContent->window()->mapFromGlobal(fileEntry->mapToGlobal({fileEntry->width()/2,14}).toPoint()));
        auto *file=main->findChild<QObject*>("fileMenuPopup");QTRY_VERIFY(file->property("opened").toBool());QTRY_VERIFY(!image->property("visible").toBool());QTRY_VERIFY(!rotate->property("visible").toBool());
        QVERIFY(QMetaObject::invokeMethod(file,"close"));
        auto *filter=main->findChild<QObject*>("filterMenu"),*blur=main->findChild<QObject*>("filterBlurSubmenu");QVERIFY(filter && blur);
        QVERIFY(QMetaObject::invokeMethod(filter,"open"));QTRY_VERIFY(filter->property("opened").toBool());QVERIFY(hover(filter,"模糊"));QTRY_VERIFY(blur->property("opened").toBool());
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()){auto *content=qobject_cast<QQuickItem*>(blur->property("contentItem").value<QObject*>());QVERIFY(GetWindowLongPtr(reinterpret_cast<HWND>(content->window()->winId()),GWL_EXSTYLE)&WS_EX_TOPMOST);}
#endif
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,QPoint(main->width()*.8,main->height()*.8));QTRY_VERIFY(!filter->property("visible").toBool());QTRY_VERIFY(!blur->property("visible").toBool());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void selectedLayersDeleteDragAndGroupChoicesAreAtomic() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));
        QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        for(int i=0;i<3;++i){client.addLayer(QString("Layer %1").arg(i));QTRY_COMPARE(client.layers().size(),i+2);QTRY_VERIFY(!client.layerEditBusy());}
        client.selectLayers(2,0);QTRY_VERIFY(!client.layerEditBusy());client.selectLayers(4,Qt::ControlModifier);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectedLayers().size(),2);
        const auto payload=client.layerDragPayload(2);QCOMPARE(client.layerDragIds(payload).size(),2);QCOMPARE(client.layerDragIds("0:2,4").size(),0);QCOMPARE(client.layerDragIds(QString::number(client.generation())+":2,2").size(),0);
        const int depth=client.undoDepth();QQuickItem *target=nullptr;QTRY_VERIFY((target=findVisualItem(main->contentItem(),"layerRow:1")));
        const auto point=target->mapToScene({target->width()*.72,target->height()-3}).toPoint();QMimeData mime;mime.setData("application/x-drawverse-layer",payload.toUtf8());
        QDragEnterEvent enter(point,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(main,&enter);
        QDragMoveEvent move(point,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(main,&move);
        QDropEvent drop(point,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(main,&drop);QVERIFY(drop.isAccepted());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);
        QCOMPARE(client.layers()[2].toMap().value("id").toULongLong(),quint64(4));QCOMPARE(client.layers()[3].toMap().value("id").toULongLong(),quint64(2));QCOMPARE(client.selectedLayers().size(),2);
        auto *trash=findVisualItem(main->contentItem(),"deleteLayer");QTRY_VERIFY((trash=findVisualItem(main->contentItem(),"deleteLayer")));QVERIFY(QMetaObject::invokeMethod(trash,"clicked"));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+2);
        client.undo();QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());
        client.selectLayers(2,0);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.groupLayer(2,"Group"));QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();QCOMPARE(client.selectedLayers(),QVariantList{QVariant::fromValue(group)});
        auto *dialog=main->findChild<QObject*>("groupDeleteDialog");QVERIFY(dialog);const auto revision=client.revision();
        client.requestSelectedLayerRemoval();QTRY_VERIFY(dialog->property("opened").toBool());
        const auto preview=qEnvironmentVariable("DRAWVERSE_MENU_LAYER_PREVIEW");if(!preview.isEmpty()){QTest::qWait(50);QVERIFY(main->grabWindow().save(preview+".group.png"));}
        QVERIFY(QMetaObject::invokeMethod(main->findChild<QObject*>("deleteGroupCancel"),"clicked"));QTRY_VERIFY(!dialog->property("visible").toBool());QCOMPARE(client.revision(),revision);
        client.requestSelectedLayerRemoval();QTRY_VERIFY(dialog->property("opened").toBool());QVERIFY(QMetaObject::invokeMethod(main->findChild<QObject*>("deleteGroupKeepChildren"),"clicked"));QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());
        const auto kept=client.layers();QVERIFY(std::any_of(kept.begin(),kept.end(),[](const QVariant &v){return v.toMap().value("id").toULongLong()==2;}));
        client.undo();QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());client.selectLayers(group,0);QTRY_VERIFY(!client.layerEditBusy());
        client.requestSelectedLayerRemoval();QTRY_VERIFY(dialog->property("opened").toBool());QVERIFY(QMetaObject::invokeMethod(main->findChild<QObject*>("deleteGroupSubtree"),"clicked"));QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());
        client.undo();QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());client.selectLayers(group,0);QTRY_VERIFY(!client.layerEditBusy());client.requestSelectedLayerRemoval();
        client.newTransparentDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());QVERIFY(!client.confirmSelectedLayerRemoval(2));QVERIFY(QMetaObject::invokeMethod(dialog,"close"));QCOMPARE(client.layers().size(),1);
        QVERIFY(!client.acceptLayerDrop(payload,0,0));QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void newGroupContainsWholeSelectionFromBothUiEntrypoints() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        for(int i=0;i<3;++i){client.addLayer(QString("L%1").arg(i));QTRY_COMPARE(client.layers().size(),i+2);QTRY_VERIFY(!client.layerEditBusy());}
        for(bool menu:{false,true}) {
            client.selectLayers(2,0);QTRY_VERIFY(!client.layerEditBusy());client.selectLayers(4,Qt::ControlModifier);QTRY_VERIFY(!client.layerEditBusy());
            QCOMPARE(client.selectedLayers().size(),2);const auto depth=client.undoDepth();
            if(menu) {auto *action=main->findChild<QObject*>("newLayerGroupAction");QVERIFY(action);QVERIFY(QMetaObject::invokeMethod(action,"trigger"));}
            else {QQuickItem *button=nullptr;QTRY_VERIFY((button=findVisualItem(main->contentItem(),"groupLayer")));QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,button->mapToScene({button->width()/2,button->height()/2}).toPoint());}
            QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);
            const auto group=client.activeLayer();QCOMPARE(client.selectedLayers(),QVariantList{QVariant::fromValue(group)});
            for(const auto &value:client.layers()) {const auto layer=value.toMap();const auto id=layer.value("id").toULongLong();if(id==2 || id==4)QCOMPARE(layer.value("parent").toULongLong(),group);if(id==1 || id==3)QCOMPARE(layer.value("parent").toULongLong(),quint64(0));}
            client.undo();QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());
            for(const auto &value:client.layers())QCOMPARE(value.toMap().value("parent").toULongLong(),quint64(0));
            client.redo();QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());client.undo();QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());
        }
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void firstLayersPanelOpenIncubatesWithoutBlockingTheEventLoop() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(client.ready());
        for(int i=0;i<79;++i){client.addLayer(QString("Layer %1").arg(i));QTRY_COMPARE(client.layers().size(),i+2);QTRY_VERIFY(!client.layerEditBusy());}
        WorkspaceManager workspace(temp.filePath("layout.ini"));workspace.hidePanel(workspace.groupForPanel("layers"),"layers");workspace.saveLayout();QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTest::qWait(100);
        QVERIFY(!findVisualItem(main->contentItem(),"layersPanel"));QElapsedTimer timer;timer.start();qint64 previous=0,longest=0;int ticks=0;QTimer heartbeat;heartbeat.setInterval(5);
        connect(&heartbeat,&QTimer::timeout,this,[&]{const auto now=timer.elapsed();longest=std::max(longest,now-previous);previous=now;++ticks;});heartbeat.start();
        workspace.showPanel("layers");const auto dispatch=timer.elapsed();QVERIFY2(dispatch<100,qPrintable(QString("opening dispatch blocked for %1 ms").arg(dispatch)));
        QTRY_VERIFY(findVisualItem(main->contentItem(),"layerRow:80"));QTest::qWait(100);heartbeat.stop();
        auto *loader=findVisualItem(main->contentItem(),"panelContentLoader:layers");QVERIFY(loader);QVERIFY(loader->property("asynchronous").toBool());QVERIFY(ticks>1);QVERIFY2(longest<200,qPrintable(QString("opening blocked heartbeat for %1 ms").arg(longest)));
        int instantiated=0;std::function<void(QQuickItem*)> count=[&](QQuickItem *item){if(item->objectName().startsWith("layerRow:"))++instantiated;for(auto *child:item->childItems())count(child);};count(main->contentItem());QVERIFY(instantiated>0 && instantiated<40);
        qInfo()<<"first layers panel dispatch"<<dispatch<<"ms; longest heartbeat gap"<<longest<<"ms; rows"<<instantiated;
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void automaticClippingReleaseKeepsMetadataAndHistoryLive() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(client.ready());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        client.addLayer("Clip");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto clip=client.activeLayer();
        QVERIFY(client.setLayerClipping(clip,true));QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(client.dropLayer(1,clip,1));QTRY_VERIFY(!client.layerEditBusy());
        QCOMPARE(client.historyEntries().last().toMap().value("title").toString(),QString("释放剪贴蒙版"));
        QCOMPARE(client.historyEntries().last().toMap().value("icon").toString(),QString("layers"));
        const auto released=[&]{for(const auto &value:client.layers()){const auto layer=value.toMap();if(layer.value("id").toULongLong()==clip)return !layer.value("clipped").toBool();}return false;};
        QVERIFY(released());client.undo();QTRY_VERIFY(!client.layerEditBusy());QVERIFY(!released());client.redo();QTRY_VERIFY(!client.layerEditBusy());QVERIFY(released());
        client.addLayer("Still responsive");QTRY_COMPARE(client.layers().size(),3);QVERIFY(client.lastError().isEmpty());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void layerThumbnailsUpdateWithoutRebuildingRows() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        for(int i=0;i<7;++i){client.addLayer(QString("Preview %1").arg(i));QTRY_COMPARE(client.layers().size(),i+2);QTRY_VERIFY(!client.layerEditBusy());}
        QPointer<QQuickItem> row=findVisualItem(main->contentItem(),"layerRow:"+QString::number(client.activeLayer()));QVERIFY(row);
        QSignalSpy structure(&client,&PaintCoreClient::layersChanged),thumbnails(&client,&PaintCoreClient::layerThumbnailsChanged);
        QTRY_VERIFY_WITH_TIMEOUT(thumbnails.size()>=4,10000);
        QCOMPARE(structure.size(),0);QVERIFY(row);QCOMPARE(findVisualItem(main->contentItem(),row->objectName()),row.data());
        QTRY_COMPARE_WITH_TIMEOUT(client.layerThumbnails().size(),8,10000);const auto cached=client.layerThumbnails();
        QElapsedTimer reopen;reopen.start();const auto group=workspace.groupForPanel("layers");workspace.setActive(group,"history");QTest::qWait(50);workspace.setActive(group,"layers");
        QTRY_VERIFY(findVisualItem(main->contentItem(),"layerRow:"+QString::number(client.activeLayer())));QVERIFY(reopen.elapsed()<1000);QCOMPARE(client.layerThumbnails(),cached);
        const auto generation=client.generation();client.newTransparentDocument(32,32);QTRY_VERIFY(client.generation()>generation);QTRY_VERIFY(client.ready());QVERIFY(client.layerThumbnails().size()<=1);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void closingDockedCategoryKeepsOtherHeightsAndFullToolColumn() {
        QTemporaryDir temp;const auto path=temp.filePath("layout.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.groupForPanel("color"),layers=workspace.groupForPanel("layers");QVERIFY(color!=layers);
        QVERIFY(workspace.dockToolStrip("drawverse-tools-v1","right",layers,"left"));QTest::qWait(60);
        auto *remaining=findVisualItem(main->contentItem(),"dockTile:"+layers),*tools=findVisualItem(main->contentItem(),"dockTile:__toolstrip");QVERIFY(remaining && tools);const auto height=remaining->height(),toolHeight=tools->height();
        workspace.hidePanel(color);QTRY_VERIFY(!findVisualItem(main->contentItem(),"dockTile:"+color));QCOMPARE(remaining->height(),height);QCOMPARE(tools->height(),toolHeight);QVERIFY(tools->height()>remaining->height()+50);
        auto *background=findVisualItem(main->contentItem(),"__panelBackground:"+layers);QVERIFY(background);QVERIFY(background->height()>remaining->height());
        const auto shot=main->grabWindow();const auto bottom=background->mapToScene({background->width()/2,background->height()-2});const auto scale=shot.width()/qreal(main->width());QCOMPARE(shot.pixelColor(qRound(bottom.x()*scale),qRound(bottom.y()*scale)),QColor("#1c1e21"));
        workspace.saveLayout();WorkspaceManager restored(path);const auto *area=background->parentItem();bool found=false;
        for(const auto &value:restored.layoutItems("main",qRound(area->width()),qRound(area->height()))){const auto item=value.toMap();if(item.value("id")==layers){QCOMPARE(item.value("rect").toRectF().height(),height);found=true;}}QVERIFY(found);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void historyUsesRealToolLabelsAndHidesEvictedInitialState() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());for(auto *canvas:window->findChildren<CanvasItem*>())canvas->setClient(nullptr);
        QTRY_COMPARE(client.historyEntries().size(),1);QCOMPARE(client.historyEntries().first().toMap().value("title").toString(),QString("初始状态"));
        client.setBrushRadius(1);InputSample sample;sample.position={16,16};sample.pressure=1;
        for(int i=0;i<150;++i) {
            const auto previous=client.revision();client.setBrushColor(i%2?Qt::black:Qt::white);QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_VERIFY(client.revision()>previous);QTRY_VERIFY(!client.layerEditBusy());
        }
        QVERIFY(client.undoDepth()>0 && client.undoDepth()<=100);QCOMPARE(client.redoDepth(),0);
        QTRY_COMPARE(client.historyEntries().size(),client.undoDepth());
        for(const auto &value:client.historyEntries()){const auto record=value.toMap();QCOMPARE(record.value("title").toString(),QString("画笔"));QCOMPARE(record.value("icon").toString(),QString("brush"));QVERIFY(record.value("depth").toInt()>0);}
        client.selectAll();QTRY_COMPARE(client.historyEntries().last().toMap().value("title").toString(),QString("全选"));
        client.setEraser(true);QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.historyEntries().last().toMap().value("title").toString(),QString("橡皮擦"));QCOMPARE(client.historyEntries().last().toMap().value("icon").toString(),QString("eraser"));
        const auto records=client.historyEntries();client.undo();QTRY_COMPARE(client.redoDepth(),1);QCOMPARE(client.historyEntries(),records);client.redo();QTRY_COMPARE(client.redoDepth(),0);QCOMPARE(client.historyEntries(),records);
        const auto group=workspace.groupForPanel("history");workspace.setActive(group,"history");QQuickItem *title=nullptr;QTRY_VERIFY((title=findVisualItem(window->contentItem(),"historyTitle:"+QString::number(client.undoDepth()))));QCOMPARE(title->property("text").toString(),QString("橡皮擦"));
        auto *icon=findVisualItem(window->contentItem(),"historyIcon:"+QString::number(client.undoDepth()));QVERIFY(icon);QCOMPARE(icon->property("name").toString(),QString("eraser"));QVERIFY(!findVisualItem(window->contentItem(),"historyEntry:0"));
        QQuickItem *previous=nullptr;QTRY_VERIFY((previous=findVisualItem(window->contentItem(),"historyEntry:"+QString::number(client.undoDepth()-1))));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,previous->mapToScene({20,12}).toPoint());QTRY_COMPARE(client.redoDepth(),1);QCOMPARE(client.historyEntries(),records);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void tabletHoverTracksOutlineThroughWindowAndItemDelivery() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();client.setBrushRadius(7);
        QHoverEvent mouse(QEvent::HoverMove,{50,60},canvas->mapToGlobal({50,60}),{40,50});QCoreApplication::sendEvent(canvas,&mouse);const bool initiallyVisible=canvas->brushCursorVisible();if(!initiallyVisible)QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());
        // QGuiApplication retains the latest tablet device between delivered events.
        auto *pen=new QPointingDevice("Hover pen",888,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,2);pen->setParent(qApp);
        for(const bool itemDelivery:{false,true}) {
            const QPointF local=itemDelivery?QPointF(123.5,180.25):QPointF(80.25,100.5);
            QTabletEvent hover(QEvent::TabletMove,pen,itemDelivery?local:canvas->mapToScene(local),canvas->mapToGlobal(local),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);
            QCoreApplication::sendEvent(itemDelivery?static_cast<QObject*>(canvas):static_cast<QObject*>(window),&hover);QVERIFY(hover.isAccepted());QVERIFY(canvas->brushCursorVisible());QCOMPARE(canvas->brushCursorRect().center(),local);QCOMPARE(client.undoDepth(),0);
        }
        QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(!canvas->brushCursorVisible());QCOMPARE(canvas->cursor().shape(),Qt::CrossCursor);
        QTabletEvent precise(QEvent::TabletMove,pen,{200,150},canvas->mapToGlobal({200,150}),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(canvas,&precise);QCOMPARE(canvas->brushCursorRect().center(),QPointF(200,150));QCOMPARE(canvas->cursor().shape(),Qt::CrossCursor);
        QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());QTabletEvent leave(QEvent::TabletLeaveProximity,pen,{200,150},canvas->mapToGlobal({200,150}),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(window,&leave);QVERIFY(!canvas->brushCursorVisible());
        if(!initiallyVisible)QTest::keyClick(window,Qt::Key_CapsLock);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void floatingSplitRemovalKeepsRemainingPaneSizeAndPosition() {
        for(const auto &edge:QStringList{"left","right","before","after"}) {
            QTemporaryDir temp;WorkspaceManager workspace(temp.filePath("layout.ini"));
            workspace.detachPanel(workspace.groupForPanel("brush"),"brush");
            workspace.detachPanel(workspace.groupForPanel("color"),"color");
            const auto brush=workspace.groupForPanel("brush"),color=workspace.groupForPanel("color");
            auto data=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",brush},{"whole",true}}).toJson());
            QVERIFY(workspace.dockPayload(data,"floating",color,edge));
            const auto host=workspace.groupDefinition(brush).value("host").toString();QVERIFY(!host.isEmpty());
            workspace.updateGeometry(host,100,100,600,600);
            QRectF remaining;
            for(const auto &value:workspace.layoutItems(host,598,598)) {const auto item=value.toMap();if(item.value("id")==brush && item.value("kind")=="leaf")remaining=item.value("rect").toRectF();}
            QVERIFY(!remaining.isEmpty());workspace.detachPanel(color,"color");
            QVariantMap window;
            for(const auto &value:workspace.floatingWindows())if(value.toMap().value("groups").toStringList().contains(brush))window=value.toMap();
            QVERIFY(!window.isEmpty());
            QCOMPARE(window.value("width").toInt(),remaining.size().toSize().width()+2);
            QCOMPARE(window.value("height").toInt(),remaining.size().toSize().height()+2);
            QCOMPARE(window.value("x").toInt(),100+remaining.topLeft().toPoint().x());
            QCOMPARE(window.value("y").toInt(),100+remaining.topLeft().toPoint().y());
        }
    }
    void layerBlendHoverPreviewsWithoutHistoryAndCommitsOnce_data() {
        QTest::addColumn<bool>("floating");QTest::newRow("docked")<<false;QTest::newRow("floating")<<true;
    }
    void layerBlendHoverPreviewsWithoutHistoryAndCommitsOnce() {
        QFETCH(bool,floating);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());
        client.setBrushRadius(8);client.setBrushColor(Qt::black);InputSample sample;sample.position={16,16};sample.pressure=1;QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);
        client.addLayer("red");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(Qt::red);QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();
        // Keep the real canvas connected: hover must repaint without any viewport scroll.
        QTRY_VERIFY(!client.frame().isNull());
        const auto center=[&](int view){const auto image=client.frame(view);return image.isNull()?QColor():image.pixelColor(image.width()/2,image.height()/2);};
        QTRY_COMPARE(center(0),QColor(Qt::red));
        client.requestViewport(1,{0,0,32,32},{32,32});QTRY_COMPARE(center(1),QColor(Qt::red));
        const auto revision=client.revision();const auto depth=client.undoDepth();const auto top=client.activeLayer();
        QQuickWindow *panelWindow=window;
        if(floating) {
            workspace.detachPanel(workspace.groupForPanel("layers"),"layers");const auto group=workspace.groupForPanel("layers");panelWindow=nullptr;
            QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+group && w->isVisible())panelWindow=qobject_cast<QQuickWindow*>(w);return panelWindow!=nullptr;}());
        }
        QQuickItem *combo=nullptr;QTRY_VERIFY((combo=findVisualItem(panelWindow->contentItem(),"layerBlendMode")));auto *popup=combo->property("popup").value<QObject*>();QVERIFY(popup);
        auto hoverMultiply=[&]()->QQuickItem* {
            if(!QMetaObject::invokeMethod(popup,"open"))return nullptr;
            auto *content=qobject_cast<QQuickItem*>(popup->property("contentItem").value<QObject*>());if(!content)return nullptr;
            QTest::qWait(30);auto *option=findVisualItem(content,"layerBlendModeOption:3");if(option)QTest::mouseMove(panelWindow,option->mapToScene({20,10}).toPoint());return option;
        };
        auto *option=hoverMultiply();QVERIFY(option);QCOMPARE(option->height(),qreal(20));
        QTRY_COMPARE(center(0),QColor(Qt::black));QTRY_COMPARE(center(1),QColor(Qt::black));QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),depth);
        auto *content=qobject_cast<QQuickItem*>(popup->property("contentItem").value<QObject*>());QVERIFY(content);
        for(int index:{0,3,0,3}) {
            auto *next=findVisualItem(content,"layerBlendModeOption:"+QString::number(index));QVERIFY(next);
            QTest::mouseMove(panelWindow,next->mapToScene({20,10}).toPoint());
            QTRY_COMPARE(center(0),index==3?QColor(Qt::black):QColor(Qt::red));
            QTRY_COMPARE(center(1),index==3?QColor(Qt::black):QColor(Qt::red));
            QCOMPARE(client.undoDepth(),depth);QCOMPARE(client.revision(),revision);
        }
        // Opening on a later current item can move the list under a stationary pointer.
        // Exercise that placement/scroll change without relying on a wheel event to refresh hover.
        auto *normal=findVisualItem(content,"layerBlendModeOption:0");QVERIFY(normal);
        QTest::mouseMove(panelWindow,normal->mapToScene({20,10}).toPoint());QTRY_COMPARE(center(0),QColor(Qt::red));
        QVERIFY(content->setProperty("contentY",qreal(60)));QTRY_COMPARE(content->property("contentY").toReal(),qreal(60));
        QTRY_COMPARE(center(0),QColor(Qt::black));QTRY_COMPARE(center(1),QColor(Qt::black));
        QVERIFY(content->setProperty("contentY",qreal(0)));QTRY_COMPARE(center(0),QColor(Qt::red));QTRY_COMPARE(center(1),QColor(Qt::red));
        QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),depth);
        for(const auto &value:client.layers())if(value.toMap().value("id").toULongLong()==top)QCOMPARE(value.toMap().value("blendMode").toInt(),0);
        QVERIFY(QMetaObject::invokeMethod(popup,"close"));QTRY_COMPARE(center(0),QColor(Qt::red));QTRY_COMPARE(center(1),QColor(Qt::red));QCOMPARE(client.undoDepth(),depth);
        option=hoverMultiply();QVERIFY(option);QTRY_COMPARE(center(0),QColor(Qt::black));QTest::mouseClick(panelWindow,Qt::LeftButton,Qt::NoModifier,option->mapToScene({20,10}).toPoint());
        QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(center(0),QColor(Qt::black));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void compactLayerSliderAndHsvMarkersUseSharedStyles() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        QQuickItem *opacity=nullptr;QTRY_VERIFY((opacity=findVisualItem(window->contentItem(),"layerOpacity")));QVERIFY(QMetaObject::invokeMethod(opacity,"openSlider"));
        auto *popup=opacity->findChild<QObject*>("layerOpacityPopup");QVERIFY(popup);QCOMPARE(popup->property("width").toReal(),qreal(127.5));QCOMPARE(popup->property("padding").toReal(),qreal(7.5));
        auto *slider=qobject_cast<QQuickItem*>(popup->property("contentItem").value<QObject*>());QVERIFY(slider);QCOMPARE(slider->implicitHeight(),qreal(16.5));
        auto *handle=qobject_cast<QQuickItem*>(slider->property("handle").value<QObject*>());QVERIFY(handle);QCOMPARE(handle->width(),qreal(6.75));QCOMPARE(handle->height(),qreal(9));QVERIFY(QMetaObject::invokeMethod(popup,"close"));
        for(int axis=0;axis<3;++axis) {
            auto *marker=findVisualItem(window->contentItem(),"hsvMarker:"+QString::number(axis));QVERIFY(marker);auto *control=marker->parentItem();QVERIFY(control);auto *gradient=qobject_cast<QQuickItem*>(control->property("background").value<QObject*>());QVERIFY(gradient);QVERIFY(marker->y()>gradient->y()+gradient->height());
        }
        auto *bar=findVisualItem(window->contentItem(),"brushOptionsBar");QVERIFY(bar);
        for(auto *child:bar->childItems())for(auto *item:child->childItems())QVERIFY(!(item->width()==16 && item->height()==16));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void maximizedTitleBarDragRestoresAndFollowsPointer() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        window->resize(1050,670);window->setPosition(window->screen()->availableGeometry().topLeft()+QPoint(80,70));
        QTRY_COMPARE(window->property("normalGeometry").toRect(),window->geometry());
        const auto normalSize=window->size();
        auto *grip=findVisualItem(window->contentItem(),"mainWindowDragArea");QVERIFY(grip);
        const auto titleAt=[&](qreal fraction) {
            qreal first=30;
            for(const auto &title:QStringList{"文件","编辑","图像","图层","选择","滤镜","窗口"})if(auto *entry=findVisualItem(window->contentItem(),"menuEntry:"+title))first=std::max(first,entry->mapToScene({entry->width(),0}).x()+8);
            const qreal last=grip->width()-106;
            // The offscreen two-DPI screen can be narrower than the app's
            // minimum width. Its logo-side gap remains a real draggable area.
            return QPoint(first<=last?qRound(first+(last-first)*fraction):(fraction<.65?2:6),14);
        };
        for(qreal fraction:{.55,.75}) {
            window->showMaximized();QTRY_COMPARE(window->visibility(),QWindow::Maximized);QTest::qWait(80);
            const auto at=titleAt(fraction);
            const auto pressGlobal=window->mapToGlobal(at);
            QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at);QVERIFY(grip->property("pressed").toBool());QCOMPARE(window->visibility(),QWindow::Maximized);
            QTest::mouseMove(window,at+QPoint(1,1));QCOMPARE(window->visibility(),QWindow::Maximized);
            const auto global=pressGlobal+QPoint(90,70),offset=QPoint(qRound(normalSize.width()*qreal(at.x())/grip->width()),14);
            QTest::mouseMove(window,window->mapFromGlobal(global));QTRY_COMPARE(window->visibility(),QWindow::Windowed);QCOMPARE(window->size(),normalSize);QTRY_COMPARE(window->position(),global-offset);
            const auto next=global+QPoint(-40,30);QTest::mouseMove(window,window->mapFromGlobal(next));QTRY_COMPARE(window->position(),next-offset);
            QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,window->mapFromGlobal(next));
            // A deferred continuation after release must finish immediately, not retain a
            // stale pressed flag from the global drag's consumed release event.
            workspace.continueRestoredWindowDrag(window,next,offset);
            QTest::mouseMove(window,window->mapFromGlobal(next+QPoint(5,5)));QCOMPARE(window->position(),next-offset);
            QTRY_COMPARE(window->property("normalGeometry").toRect(),window->geometry());
            const auto moved=window->geometry();window->showMinimized();QTRY_COMPARE(window->visibility(),QWindow::Minimized);
#ifdef Q_OS_WIN
            if(workspace.windowsWindowFrames())SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_RESTORE,0);else
#endif
                window->showNormal();
            QTRY_COMPARE(window->visibility(),QWindow::Windowed);QTRY_COMPARE(window->geometry(),moved);
        }
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            window->showMaximized();QTRY_COMPARE(window->visibility(),QWindow::Maximized);
            const auto handle=reinterpret_cast<HWND>(window->winId());
            SetWindowPos(handle,HWND_TOPMOST,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE);window->requestActivate();QTRY_VERIFY(window->isActive());
            POINT physical{qRound(grip->width()*.65*window->devicePixelRatio()),qRound(14*window->devicePixelRatio())};ClientToScreen(handle,&physical);SetCursorPos(physical.x,physical.y);
            INPUT down{};down.type=INPUT_MOUSE;down.mi.dwFlags=MOUSEEVENTF_LEFTDOWN;
            const auto release=qScopeGuard([]{INPUT up{};up.type=INPUT_MOUSE;up.mi.dwFlags=MOUSEEVENTF_LEFTUP;SendInput(1,&up,sizeof(INPUT));});
            QCOMPARE(SendInput(1,&down,sizeof(INPUT)),UINT(1));QTRY_VERIFY(grip->property("pressed").toBool());QCOMPARE(window->visibility(),QWindow::Maximized);
            SetCursorPos(physical.x+qRound(100*window->devicePixelRatio()),physical.y+qRound(90*window->devicePixelRatio()));
            QTRY_COMPARE(window->visibility(),QWindow::Windowed);QCOMPARE(window->size(),normalSize);
            const auto offset=QPoint(qRound(normalSize.width()*grip->property("pressFraction").toReal()),qRound(grip->property("pressY").toReal()));
            QTRY_VERIFY((window->position()-(QCursor::pos()-offset)).manhattanLength()<=2);
            const auto firstPosition=window->position(),firstCursor=QCursor::pos();GetCursorPos(&physical);
            SetCursorPos(physical.x+qRound(40*window->devicePixelRatio()),physical.y+qRound(25*window->devicePixelRatio()));
            QTRY_VERIFY((window->position()-(firstPosition+QCursor::pos()-firstCursor)).manhattanLength()<=2);
            const auto rounded=[](HWND hwnd) {const auto region=CreateRectRgn(0,0,0,0);const bool result=GetWindowRgn(hwnd,region)==COMPLEXREGION && !PtInRegion(region,0,0);DeleteObject(region);return result;};
            const auto blur=reinterpret_cast<HWND>(workspace.menuBlurWindowHandle());QVERIFY(blur);QTRY_VERIFY(rounded(handle) && rounded(blur));
            RECT mainBounds{},blurBounds{};QVERIFY(GetWindowRect(handle,&mainBounds));QVERIFY(GetWindowRect(blur,&blurBounds));QCOMPARE(blurBounds.left,mainBounds.left);QCOMPARE(blurBounds.top,mainBounds.top);QCOMPARE(blurBounds.right,mainBounds.right);
        }
#endif
        window->showMaximized();QTRY_COMPARE(window->visibility(),QWindow::Maximized);QTest::qWait(80);
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            const auto at=titleAt(.5);POINT point{qRound(at.x()*window->devicePixelRatio()),qRound(at.y()*window->devicePixelRatio())};ClientToScreen(reinterpret_cast<HWND>(window->winId()),&point);SetCursorPos(point.x,point.y);
            INPUT clicks[4]{};for(int i=0;i<4;++i){clicks[i].type=INPUT_MOUSE;clicks[i].mi.dwFlags=i%2?MOUSEEVENTF_LEFTUP:MOUSEEVENTF_LEFTDOWN;}QCOMPARE(SendInput(4,clicks,sizeof(INPUT)),UINT(4));
        } else
#endif
            QTest::mouseDClick(window,Qt::LeftButton,Qt::NoModifier,titleAt(.5));
        QTRY_COMPARE(window->visibility(),QWindow::Windowed);QCOMPARE(window->size(),normalSize);
        window->showFullScreen();QTest::qWait(80);const auto fullGeometry=window->geometry();const auto fullAt=titleAt(.25);
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,fullAt);QTest::mouseMove(window,fullAt+QPoint(60,25));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,fullAt+QPoint(60,25));QCOMPARE(window->visibility(),QWindow::FullScreen);QCOMPARE(window->geometry(),fullGeometry);
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            QVERIFY(PostMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_MOVE,0));QTest::qWait(50);QCOMPARE(window->visibility(),QWindow::FullScreen);
        }
#endif
        window->showNormal();QTRY_COMPARE(window->visibility(),QWindow::Windowed);
        const auto normalGeometry=window->geometry();const auto normalAt=titleAt(.5);QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,normalAt);QTest::mouseMove(window,normalAt+QPoint(1,1));QCOMPARE(window->geometry(),normalGeometry);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,normalAt+QPoint(1,1));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void nativeMenuBlurLayerFollowsWindowAndHidesBeforeMinimize() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            const auto main=reinterpret_cast<HWND>(window->winId()),blur=reinterpret_cast<HWND>(workspace.menuBlurWindowHandle());QVERIFY(blur);QVERIFY(IsWindow(blur));
            const auto style=GetWindowLongPtrW(blur,GWL_EXSTYLE);QVERIFY(style&WS_EX_TOOLWINDOW);QVERIFY(style&WS_EX_NOACTIVATE);QVERIFY(style&WS_EX_TRANSPARENT);QVERIFY(!GetWindow(blur,GW_OWNER));
            const auto correct=[&](bool rounded) {
                RECT a{},b{};if(!GetWindowRect(main,&a) || !GetWindowRect(blur,&b))return false;
                const auto region=CreateRectRgn(0,0,0,0);const int kind=GetWindowRgn(blur,region);
                const bool shape=kind!=ERROR && bool(PtInRegion(region,0,0))==!rounded && PtInRegion(region,0,b.bottom-b.top-1) && PtInRegion(region,(b.right-b.left)/2,0);
                DeleteObject(region);
                return IsWindowVisible(blur) && a.left==b.left && a.top==b.top && a.right==b.right && b.bottom-b.top==qRound(28*window->devicePixelRatio()) && shape && GetWindow(main,GW_HWNDNEXT)==blur;
            };
            QTRY_VERIFY(correct(true));window->setPosition(window->position()+QPoint(37,29));window->resize(1050,670);QTRY_VERIFY(correct(true));
            for(bool maximized:{false,true})for(int cycle=0;cycle<3;++cycle) {
                if(maximized)window->showMaximized();else window->showNormal();QTRY_VERIFY(correct(!maximized));
                SendMessageW(main,WM_SYSCOMMAND,SC_MINIMIZE,0);QVERIFY(!IsWindowVisible(blur));QVERIFY(!IsIconic(blur));
                SendMessageW(main,WM_SYSCOMMAND,SC_RESTORE,0);QTRY_COMPARE(window->visibility(),maximized?QWindow::Maximized:QWindow::Windowed);QTRY_VERIFY(correct(!maximized));
                window->showNormal();QTRY_VERIFY(correct(true));
            }
            window->hide();QVERIFY(!IsWindowVisible(blur));window->show();QTRY_VERIFY(correct(true));
            QVERIFY(workspace.setMenuBarBlur(window,false));QVERIFY(!IsWindowVisible(blur));QVERIFY(workspace.setMenuBarBlur(window,true));QTRY_VERIFY(correct(true));
            engine.clearComponentCache();engine.rootObjects().first()->deleteLater();QTRY_VERIFY(!IsWindow(blur));
        } else
#endif
            QCOMPARE(workspace.menuBlurWindowHandle(),quintptr(0));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void brushPresetClicksKeepEraserAndShareActualStrokeSettings() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();QVERIFY(library);
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);
        const auto stroke=[&] {InputSample sample;sample.position={32,64};sample.pressure=1;sample.tool=1;if(!client.beginStroke(sample))return false;sample.position={96,64};client.strokeTo(sample);client.endStroke();return true;};
        const auto rendered=[&] {client.requestViewport(0,{0,0,128,128},{128,128});return client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,128);};
        library->setRadius(12);client.setBrushColor(Qt::black);QVERIFY(stroke());QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(rendered());const int painted=client.frame().pixelColor(64,64).alpha();QVERIFY(painted>200);
        client.setEraser(true);
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()) {
            const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush"))workspace.setActive(group.value("id").toString(),"brush");
        }
        QQuickItem *fine=nullptr;QTRY_VERIFY((fine=findVisualItem(window->contentItem(),"brushPreset:round-fine")));
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,fine->mapToScene({fine->width()/2,fine->height()/2}).toPoint());QTRY_COMPARE(library->selectedId(),QString("round-fine"));QVERIFY(client.eraser());
        library->setRadius(8);library->setOpacity(.5);library->setSpacing(.25);QCOMPARE(client.brushRadius(),qreal(8));QCOMPARE(client.brushOpacity(),qreal(.5));QCOMPARE(client.brushSpacing(),qreal(.25));
        QVERIFY(stroke());QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(rendered());const int erased=client.frame().pixelColor(64,64).alpha();QVERIFY(erased<painted);
        auto *brush=findVisualItem(window->contentItem(),"brushTool");QVERIFY(brush);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,brush->mapToScene({brush->width()/2,brush->height()/2}).toPoint());QVERIFY(!client.eraser());QCOMPARE(library->selectedId(),QString("round-fine"));QCOMPARE(client.brushRadius(),qreal(12));QCOMPARE(client.brushOpacity(),qreal(.5));QCOMPARE(client.brushSpacing(),qreal(.25));
        QVERIFY(stroke());QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(rendered());QVERIFY(client.frame().pixelColor(64,64).alpha()>erased);
        canvas->setClient(&client);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void proceduralBrushTipPersistencePreviewCursorAndPixels() {
        BrushLibrary model;model.setHardness(.3);model.setRoundness(.2);model.setAngle(90);model.setFlow(.4);model.setPressureSize(false);model.setPressureFlow(false);
        const auto state=model.snapshot();BrushLibrary restored;QVERIFY(restored.restore(state));QCOMPARE(restored.hardness(),.3);QCOMPARE(restored.roundness(),.2);QCOMPARE(restored.angle(),90.);QCOMPARE(restored.flow(),.4);QVERIFY(!restored.pressureSize() && !restored.pressureFlow());
        const auto restoredState=restored.snapshot();auto bad=QJsonDocument::fromJson(state).object();bad["selected"]="round-fine";bad["recentColors"]=QJsonArray{"invalid color"};QVERIFY(!restored.restore(QJsonDocument(bad).toJson()));QCOMPARE(restored.snapshot(),restoredState);
        auto old=QJsonDocument::fromJson(state).object();old["version"]=1;QVERIFY(restored.restore(QJsonDocument(old).toJson()));QCOMPARE(restored.hardness(),1.);QCOMPARE(restored.roundness(),1.);QVERIFY(restored.pressureSize() && restored.pressureFlow());
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();library->setRadius(10);library->setRoundness(.2);library->setAngle(90);library->setPressureSize(false);library->setPressureFlow(false);
        library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto first=library->selectedPreview();library->setHardness(.2);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(first!=library->selectedPreview());library->setHardness(1);
        client.newTransparentDocument(40,40);QTRY_COMPARE(client.documentWidth(),40);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);
        InputSample sample;sample.position={20.5,20.5};sample.pressure=.1f;sample.tool=1;QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        client.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(40,40));QVERIFY(client.frame().pixelColor(20,27).alpha()>200);QCOMPARE(client.frame().pixelColor(27,20).alpha(),0);
        canvas->setClient(&client);auto *cursor=findVisualItem(window->contentItem(),"brushCursorOutline");QVERIFY(cursor);QCOMPARE(cursor->property("rotation").toDouble(),90.);QVERIFY(std::abs(cursor->height()/cursor->width()-.2)<1e-6);
        const auto screenshot=qEnvironmentVariable("DRAWVERSE_BRUSH_PREVIEW");if(!screenshot.isEmpty()){auto group=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(group);for(auto *w:applicationWindows())if(auto *q=qobject_cast<QQuickWindow*>(w))if(findVisualItem(q->contentItem(),"selectedBrushName")){q->resize(350,440);QTest::qWait(200);QVERIFY(q->grabWindow().save(screenshot+".tip.png"));}}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void imageBrushImportPreviewPersistenceAndCapturedTarget() {
        QTemporaryDir temp;const auto settingsPath=temp.filePath("brush.ini");
        QImage tip(16,16,QImage::Format_RGBA8888);tip.fill(Qt::white);
        for(int y=6;y<10;++y)for(int x=6;x<10;++x)tip.setPixelColor(x,y,Qt::transparent);
        const auto source=temp.filePath("tip.png");QVERIFY(tip.save(source));
        PaintCoreClient client(nullptr,settingsPath);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        QSignalSpy imported(&client,&PaintCoreClient::brushFileFinished);
        QVERIFY(client.importBrushTip(QUrl::fromLocalFile(source),1));library->select("round-fine");
        QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QVERIFY(library->maskResource().isEmpty());
        library->select("round-pressure");const auto identity=library->maskResource();QCOMPARE(identity.size(),64);
        QVERIFY(QFileInfo::exists(temp.filePath("brush-resources/"+identity+".png")));
        const auto copy=library->saveCopy("图片副本");QVERIFY(!copy.isEmpty());QCOMPARE(library->maskResource(),identity);
        library->setRadius(8);library->setPressureSize(false);library->setPressureFlow(false);
        library->requestPreview(copy);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto preview=library->selectedPreview();
        QVERIFY(!client.exportBrush(QUrl::fromLocalFile(temp.filePath("image.vbr"))));QVERIFY(!QFileInfo::exists(temp.filePath("image.vbr")));
        client.newTransparentDocument(40,40);QTRY_COMPARE(client.documentWidth(),40);QTRY_VERIFY(client.ready());
        InputSample sample;sample.position={20.5,20.5};sample.pressure=1;sample.tool=1;QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);
        client.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(40,40));
        QCOMPARE(client.frame().pixelColor(20,20).alpha(),0);QVERIFY(client.frame().pixelColor(25,20).alpha()>200);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);QVERIFY(QFile::remove(source));
        PaintCoreClient recovered(nullptr,settingsPath);QTRY_VERIFY(recovered.ready());auto *restored=recovered.brushLibrary();QCOMPARE(restored->maskResource(),identity);QCOMPARE(restored->selectedId(),copy);
        restored->requestPreview(copy);QTRY_VERIFY_WITH_TIMEOUT(!restored->selectedPreview().isEmpty(),10000);QCOMPARE(restored->selectedPreview(),preview);
        restored->useProceduralTip();QVERIFY(restored->maskResource().isEmpty());QVERIFY(!restored->exportPackage().isEmpty());restored->requestPreview(copy);QTRY_VERIFY_WITH_TIMEOUT(!restored->selectedPreview().isEmpty(),10000);QVERIFY(restored->selectedPreview()!=preview);
        QSignalSpy recoveredStopped(&recovered,&PaintCoreClient::stopped);recovered.shutdown();QTRY_COMPARE(recoveredStopped.size(),1);
        QByteArray persisted;{QSettings settings(settingsPath,QSettings::IniFormat);persisted=settings.value("brushes/state").toByteArray();}
        QVERIFY(QFile::remove(temp.filePath("brush-resources/"+identity+".png")));
        PaintCoreClient missing(nullptr,settingsPath);QTRY_VERIFY(missing.ready());QTRY_VERIFY_WITH_TIMEOUT(!missing.lastError().isEmpty(),10000);
        {QSettings settings(settingsPath,QSettings::IniFormat);QCOMPARE(settings.value("brushes/state").toByteArray(),persisted);}
        QSignalSpy missingStopped(&missing,&PaintCoreClient::stopped);missing.shutdown();QTRY_COMPARE(missingStopped.size(),1);
    }
    void vbrPackageImportExportIsAtomicAndPreservesParameters() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();library->setHardness(.25);library->setRoundness(.3);library->setAngle(-35);library->setFlow(.6);library->setPressureSize(false);library->setSpacing(.02);
        const auto revision=client.revision();const auto undo=client.undoDepth();const auto path=QUrl::fromLocalFile(temp.filePath("brush.vbr"));QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);
        QVERIFY(client.exportBrush(path));QVERIFY(client.brushFileBusy());QVERIFY(!client.exportBrush(path));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QVERIFY(!client.brushFileBusy());QVERIFY(QFileInfo::exists(path.toLocalFile()));
        library->setHardness(1);QVERIFY(client.importBrushes(path));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QCOMPARE(library->presets().size(),5);QCOMPARE(library->hardness(),.25);QCOMPARE(library->roundness(),.3);QCOMPARE(library->angle(),-35.);QCOMPARE(library->flow(),.6);QCOMPARE(library->spacing(),.02);QVERIFY(!library->pressureSize());
        const auto imported=library->selectedId();QVERIFY(client.importBrushes(path));QTRY_COMPARE(finished.size(),3);QVERIFY(finished.last()[0].toBool());QCOMPARE(library->presets().size(),6);QVERIFY(library->selectedId()!=imported);
        const auto originalState=library->snapshot();QFile bad(temp.filePath("bad.vbr"));QVERIFY(bad.open(QIODevice::WriteOnly));bad.write("invalid archive");bad.close();QVERIFY(client.importBrushes(QUrl::fromLocalFile(bad.fileName())));QTRY_COMPARE(finished.size(),4);QVERIFY(!finished.last()[0].toBool());QCOMPARE(library->snapshot(),originalState);QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),undo);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void nativeBrushRevisionStartsImmediatelyAndBadMigrationKeepsFile() {
        QTemporaryDir temp;const auto path=temp.filePath("state.ini");
        PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());client.newTransparentDocument(40,40);QTRY_COMPARE(client.documentWidth(),40);QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();library->setRadius(10);library->setPressureSize(false);library->setPressureFlow(false);library->setFlow(1);
        library->setRoundness(.2);library->setAngle(0);
        InputSample sample;sample.position={20.5,20.5};sample.pressure=.1f;sample.tool=1;
        QVERIFY(client.beginStroke(sample));library->setRoundness(1);library->setFlow(0);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        client.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(40,40));QVERIFY(client.frame().pixelColor(27,20).alpha()>200);QCOMPARE(client.frame().pixelColor(20,27).alpha(),0);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);library->setRoundness(.2);library->setAngle(90);library->setFlow(1);QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.frame().pixelColor(20,27).alpha()>200);QCOMPARE(client.frame().pixelColor(27,20).alpha(),0);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        BrushLibrary old;const auto copy=old.saveCopy("Old custom");QVERIFY(!copy.isEmpty());auto invalid=QJsonDocument::fromJson(old.snapshot()).object();auto rows=invalid.value("presets").toArray();auto row=rows.last().toObject();row["id"]="not-a-uuid";rows[rows.size()-1]=row;invalid["presets"]=rows;invalid["selected"]="not-a-uuid";const auto bad=QJsonDocument(invalid).toJson(QJsonDocument::Compact);const auto badPath=temp.filePath("bad.ini");{QSettings settings(badPath,QSettings::IniFormat);settings.setValue("brushes/state",bad);settings.sync();}
        PaintCoreClient recovered(nullptr,badPath);QTRY_VERIFY(recovered.ready());QTRY_VERIFY_WITH_TIMEOUT(recovered.lastError().contains("UUID"),10000);QTRY_COMPARE(recovered.brushLibrary()->presets().size(),4);{QSettings settings(badPath,QSettings::IniFormat);QCOMPARE(settings.value("brushes/state").toByteArray(),bad);}
        QVERIFY(recovered.beginStroke(sample));recovered.endStroke();QTRY_COMPARE(recovered.undoDepth(),1);QSignalSpy recoveredStopped(&recovered,&PaintCoreClient::stopped);recovered.shutdown();QTRY_COMPARE(recoveredStopped.size(),1);
    }
    void brushCursorPreferencesAreGlobalPersistentAndNeverExported() {
        BrushLibrary model;QSignalSpy preview(&model,&BrushLibrary::previewRequested);QSignalSpy settings(&model,&BrushLibrary::settingsChanged);
        model.setCursorCenter(true);model.setCursorHoverOnly(true);model.setCursorWidth(3);model.setCursorOutline(false);model.setCursorSimple(true);model.setCursorColor("#123456");model.setCursorSecondaryColor("#abcdef");QCOMPARE(settings.size(),0);QCOMPARE(preview.size(),0);
        BrushLibrary restored;QVERIFY(restored.restore(model.snapshot()));QVERIFY(restored.cursorCenter() && restored.cursorHoverOnly());QVERIFY(!restored.cursorOutline());QCOMPARE(restored.cursorWidth(),3);QVERIFY(restored.cursorSimple());QCOMPARE(restored.cursorColor(),QString("#123456"));QCOMPARE(restored.cursorSecondaryColor(),QString("#abcdef"));restored.setCursorColor("invalid");QCOMPARE(restored.cursorColor(),QString("#123456"));
        const auto state=restored.snapshot();auto invalid=QJsonDocument::fromJson(state).object();invalid["cursor"]=QJsonObject{{"outline",true},{"center",false},{"hoverOnly",false},{"width",8}};QVERIFY(!restored.restore(QJsonDocument(invalid).toJson()));QCOMPARE(restored.snapshot(),state);
        QVERIFY(!model.exportPackage().contains("cursor"));const auto copy=model.saveCopy("Cursor independent");QVERIFY(!copy.isEmpty());QVERIFY(!model.cursorOutline());auto old=QJsonDocument::fromJson(state).object();old.remove("cursor");QVERIFY(restored.restore(QJsonDocument(old).toJson()));QVERIFY(restored.cursorOutline() && !restored.cursorCenter());QCOMPARE(restored.cursorWidth(),1);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();
        QHoverEvent hover(QEvent::HoverMove,{50,60},canvas->mapToGlobal({50,60}),{40,50});QCoreApplication::sendEvent(canvas,&hover);const bool initial=canvas->brushCursorVisible();if(!initial)QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());
        auto *library=client.brushLibrary();library->setRoundness(.2);library->setAngle(90);library->setCursorSimple(true);auto *outline=findVisualItem(window->contentItem(),"brushCursorOutline");QVERIFY(outline);QCOMPARE(outline->height(),outline->width());QCOMPARE(outline->rotation(),0.);library->setCursorOutline(false);QVERIFY(!canvas->brushCursorVisible());QCOMPARE(canvas->cursor().shape(),Qt::CrossCursor);library->setCursorCenter(true);QVERIFY(canvas->brushCursorVisible());auto *cross=findVisualItem(window->contentItem(),"brushCursorCenterCross");QVERIFY(cross);QVERIFY(cross->isVisible());
        library->setCursorHoverOnly(true);InputSample sample;sample.position={10,10};QVERIFY(client.beginStroke(sample));QVERIFY(!canvas->brushCursorVisible());client.endStroke();QVERIFY(canvas->brushCursorVisible());
        QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(!canvas->brushCursorVisible());QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());if(!initial)QTest::keyClick(window,Qt::Key_CapsLock);
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);const auto id=documents->newDocument(32,32,true);QVERIFY(!id.isEmpty());auto *other=documents->client(id);QVERIFY(other);QTRY_VERIFY(other->ready());QVERIFY(other->brushLibrary()->cursorCenter());QVERIFY(!other->brushLibrary()->cursorOutline());other->brushLibrary()->setCursorWidth(2);QCOMPARE(library->cursorWidth(),2);library->setHardness(.25);QCOMPARE(other->brushLibrary()->hardness(),.25);
        const auto firstId=documents->documents().first().toMap().value("id").toString();QSignalSpy firstStopped(&client,&PaintCoreClient::stopped);QVERIFY(documents->closeDocument(firstId,true));QTRY_COMPARE(firstStopped.size(),1);other->brushLibrary()->setCursorWidth(3);
        const auto thirdId=documents->newDocument(16,16,true);QVERIFY(!thirdId.isEmpty());auto *third=documents->client(thirdId);QVERIFY(third);QTRY_VERIFY(third->ready());QCOMPARE(third->brushLibrary()->cursorWidth(),3);QCOMPARE(third->brushLibrary()->hardness(),.25);
        QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
        PaintCoreClient reloaded(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(reloaded.ready());QVERIFY(reloaded.brushLibrary()->cursorCenter());QCOMPARE(reloaded.brushLibrary()->cursorWidth(),3);QCOMPARE(reloaded.brushLibrary()->hardness(),.25);QSignalSpy stoppedAgain(&reloaded,&PaintCoreClient::stopped);reloaded.shutdown();QTRY_COMPARE(stoppedAgain.size(),1);
    }
    void mainWindowRestoresWithoutNativeFrameOrClippedContent() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
#ifdef Q_OS_WIN
        if(QGuiApplication::platformName()=="windows") {
            const auto style=GetWindowLongPtrW(reinterpret_cast<HWND>(window->winId()),GWL_STYLE);
            QVERIFY2(style&WS_MINIMIZEBOX,"Taskbar minimization requires WS_MINIMIZEBOX on the frameless HWND");
            QVERIFY(style&WS_MAXIMIZEBOX);QVERIFY(style&WS_SYSMENU);QVERIFY(!(style&WS_CAPTION));
        }
#endif
        for(bool maximized:{false,true})for(int cycle=0;cycle<3;++cycle) {
            const auto normalBefore=window->geometry();
            if(maximized)window->showMaximized();else window->showNormal();QTest::qWait(80);
            const auto before=window->geometry();
#ifdef Q_OS_WIN
            if(QGuiApplication::platformName()=="windows" && cycle!=1)SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_MINIMIZE,0);
            else
#endif
                {auto *minimize=findVisualItem(window->contentItem(),"windowMinimize");QVERIFY(minimize);QVERIFY(QMetaObject::invokeMethod(minimize,"clicked"));}
            QTRY_COMPARE(window->visibility(),QWindow::Minimized);QTest::qWait(80);
#ifdef Q_OS_WIN
            if(QGuiApplication::platformName()=="windows" && maximized && cycle==2) {
                const auto handle=reinterpret_cast<HWND>(window->winId());
                // A second taskbar action can arrive before the queued Qt
                // maximized-state restoration; it must stay minimized.
                SendMessageW(handle,WM_SYSCOMMAND,SC_RESTORE,0);
                SendMessageW(handle,WM_SYSCOMMAND,SC_MINIMIZE,0);
                QTest::qWait(100);QTRY_COMPARE(window->visibility(),QWindow::Minimized);QVERIFY(IsIconic(handle));
            }
            if(QGuiApplication::platformName()=="windows") {
                const auto handle=reinterpret_cast<HWND>(window->winId());QVERIFY(IsIconic(handle));
                RECT bounds{};QVERIFY(GetWindowRect(handle,&bounds));
                QVERIFY2(!QRect(bounds.left,bounds.top,bounds.right-bounds.left,bounds.bottom-bounds.top).intersects(before),"Minimization must remove the window, including its native iconic caption, from the workspace");
                SendMessageW(handle,WM_SYSCOMMAND,SC_RESTORE,0);
            }
            else
#endif
                {if(maximized)window->showMaximized();else window->showNormal();}
            QTRY_COMPARE(window->visibility(),maximized?QWindow::Maximized:QWindow::Windowed);QTRY_COMPARE(window->geometry(),before);QTest::qWait(100);
#ifdef Q_OS_WIN
            if(QGuiApplication::platformName()=="windows") {
                const auto handle=reinterpret_cast<HWND>(window->winId());RECT outer{},inner{};QVERIFY(GetWindowRect(handle,&outer));QVERIFY(GetClientRect(handle,&inner));
                QVERIFY(!(GetWindowLongPtrW(handle,GWL_STYLE)&WS_CAPTION));QVERIFY(!IsIconic(handle));
                QCOMPARE(inner.right,outer.right-outer.left);QCOMPARE(inner.bottom,outer.bottom-outer.top);
                const auto region=CreateRectRgn(0,0,0,0);const int kind=GetWindowRgn(handle,region);
                const bool centerVisible=kind==ERROR || PtInRegion(region,inner.right/2,inner.bottom/2);
                const bool rounded=kind==COMPLEXREGION && !PtInRegion(region,0,0) && !PtInRegion(region,inner.right-1,0);
                DeleteObject(region);QVERIFY(centerVisible);QCOMPARE(rounded,!maximized);
            }
#endif
            const auto image=window->grabWindow();QVERIFY(!image.isNull());QVERIFY(std::abs(image.width()-qRound(window->width()*window->devicePixelRatio()))<=1);QVERIFY(std::abs(image.height()-qRound(window->height()*window->devicePixelRatio()))<=1);QVERIFY(image.pixelColor(image.width()/2,image.height()/2).alpha()>240);
            auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);QVERIFY(canvas->width()>0);QVERIFY(canvas->height()>0);
            if(maximized) {window->showNormal();QTRY_COMPARE(window->geometry(),normalBefore);}
        }
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void dockSeparatorsUseThinDarkGrayAcrossMainAndFloatingColumns() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        const auto check=[&](QQuickWindow *host,const QString &id) {
            int count=0;
            for(const auto &value:workspace.layoutItems(id,host->width(),host->height())) {
                const auto data=value.toMap();if(data.value("kind")!="split")continue;
                auto *divider=findVisualItem(host->contentItem(),"dockDivider:"+data.value("id").toString());
                if(!divider || divider->property("color").value<QColor>()!=QColor("#1b1c1f"))return false;
                if(data.value("axis")=="horizontal"?divider->width()!=1:divider->height()!=1)return false;
                ++count;
            }
            return count>0;
        };
        QTest::mouseMove(window,{500,300});QTRY_VERIFY(check(window,"main"));
        const auto first=workspace.leftGroups().first().toMap().value("id").toString(),second=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachGroup(first);
        const auto payload=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",second},{"whole",true}}).toJson());
        QVERIFY(workspace.dockPayload(payload,"floating",first,"after"));
        QQuickWindow *floating=nullptr;
        QTRY_VERIFY((floating=window->findChild<QQuickWindow*>("floatingDock:"+first)));
        QTest::mouseMove(floating,{100,100});QTRY_VERIFY(check(floating,first));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void flexibleDockTreeSupportsFourEdgesAndValidatesPersistence() {
        QTemporaryDir temp;const auto path=temp.filePath("docks.ini");WorkspaceManager workspace(path);
        const auto data=[](const QString &id){return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",id},{"whole",true}}).toJson());};
        const auto rect=[&](const QString &host,const QString &id){for(const auto &v:workspace.layoutItems(host,1400,800))if(v.toMap().value("id")==id)return v.toMap().value("rect").toRectF();return QRectF();};
        for(const auto &v:workspace.leftGroups()+workspace.rightGroups())for(const auto &edge:QStringList{"left","right"}) {
            const auto target=v.toMap().value("id").toString(),location=v.toMap().value("location").toString();
            QVERIFY(workspace.dockToolStrip("drawverse-tools-v1",location,target,edge));const auto a=rect("main","__toolstrip"),b=rect("main",target);QCOMPARE(a.width(),qreal(36));
            if(edge=="left")QVERIFY(a.right()<=b.left()-1);else QVERIFY(b.right()<=a.left()-1);
        }
        QVERIFY(!workspace.dockToolStrip("drawverse-tools-v1","center","__canvas","right"));
        QVERIFY(!workspace.dockPayload(data("__canvas"),"floating"));
        for(const auto &edge:QStringList{"left","right","top","bottom"}) {
            workspace.resetLayout();const auto source=workspace.leftGroups().first().toMap().value("id").toString(),target=workspace.rightGroups().first().toMap().value("id").toString();
            QVERIFY(workspace.dockPayload(data(source),"right",target,edge));const auto a=rect("main",source),b=rect("main",target);QVERIFY(!a.intersects(b));
            if(edge=="left")QVERIFY(a.right()<=b.left()-1);if(edge=="right")QVERIFY(b.right()<=a.left()-1);
            if(edge=="top")QVERIFY(a.bottom()<=b.top()-1);if(edge=="bottom")QVERIFY(b.bottom()<=a.top()-1);
        }
        workspace.resetLayout();const auto left=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(left);
        QCOMPARE(rect("main","__canvas").left(),rect("main","__toolstrip").right()+1);
        const auto color=workspace.rightGroups().first().toMap().value("id").toString();
        QVERIFY(workspace.dockPayload(data(color),"floating",left,"after"));QCOMPARE(workspace.columnGroups(color),QStringList({left,color}));
        workspace.setColumnCollapsed(left,true);workspace.setColumnCollapsed(color,true);
        const auto a=rect(left,left),b=rect(left,color);QCOMPARE(b.top(),a.bottom()+8);QVERIFY(b.bottom()<200);
        workspace.setColumnCollapsed(color,true);workspace.saveLayout();const auto layout=workspace.layoutItems(left,1400,800);WorkspaceManager restored(path);QCOMPARE(restored.layoutItems(left,1400,800),layout);
        QSettings settings(path,QSettings::IniFormat);const auto valid=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();
        QCOMPARE(valid.value("version").toInt(),6);
        for(const auto &invalid:QStringList{"duplicate","missing","ratio","window","geometry","icons"}) {
            auto root=valid;auto docks=root.value("docks").toObject();
            if(invalid=="duplicate")docks.insert(left,DockTree::leaf("__canvas"));
            if(invalid=="missing")docks.insert(left,DockTree::leaf(color));
            if(invalid=="ratio"){auto node=docks.value(left).toObject();node.insert("ratio",1.5);docks.insert(left,node);}
            if(invalid=="window")docks.insert("orphan",DockTree::leaf("unknown"));
            if(invalid=="geometry"){auto windows=root.value("windows").toObject();windows.insert(left,QJsonObject{{"width",-1}});root.insert("windows",windows);}
            if(invalid=="icons")root.insert("iconGroups",QJsonArray{"unknown"});
            root.insert("docks",docks);settings.setValue("workspace",QJsonDocument(root).toJson());settings.sync();
            QVERIFY(!restored.restoreLayout());QCOMPARE(restored.layoutItems(left,1400,800),layout);
        }
        auto oldTree=valid;oldTree.insert("version",5);oldTree.remove("iconGroups");auto mixed=oldTree.value("docks").toObject();auto mainTree=mixed.value("main").toObject();QVERIFY(DockTree::remove(mainTree,"__canvas"));mixed.insert("main",mainTree);mixed.insert(left,DockTree::split(mixed.value(left).toObject(),DockTree::leaf("__canvas"),"horizontal",.5));oldTree.insert("docks",mixed);settings.setValue("workspace",QJsonDocument(oldTree).toJson());settings.sync();WorkspaceManager separated(path);QCOMPARE(separated.groupDefinition("__canvas").value("host").toString(),QString("main"));QCOMPARE(separated.columnGroups(left),QStringList({left,color}));QCOMPARE(separated.floatingWindows().size(),1);
        auto legacy=valid;legacy.insert("version",4);legacy.remove("docks");legacy.remove("windows");settings.setValue("workspace",QJsonDocument(legacy).toJson());settings.sync();WorkspaceManager migrated(path);QCOMPARE(migrated.visiblePanels().size(),6);
        settings.sync();QCOMPARE(QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object().value("version").toInt(),6);
    }
    void floatingCanvasAndToolsDockAcrossNativeWindows() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));
        QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);QPointer<CanvasItem> retained=canvas;
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.frame().isNull());
        auto *grip=findVisualItem(main->contentItem(),"canvasGrip");QVERIFY(grip);
        QTest::mouseDClick(main,Qt::LeftButton,Qt::NoModifier,grip->mapToScene({grip->width()/2,grip->height()/2}).toPoint());
        QTRY_COMPARE(applicationWindows().size(),2);QTRY_VERIFY(canvas->window()!=main);
        auto *floating=canvas->window();QVERIFY(floating);QPointer<QQuickWindow> retainedWindow=floating;
        floating->requestActivate();QTRY_VERIFY(floating->isActive());QTRY_VERIFY(canvas->height()>200);canvas->actualSize();
        client.setBrushRadius(4);client.setBrushColor(Qt::red);
        const auto paintAt=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(32,32)).toPoint();
        const auto centerColor=[&] {
            const auto region=client.frameRegion();const auto frame=client.frame();
            if(frame.isNull() || region.isEmpty())return QColor();
            return frame.pixelColor(qRound((32-region.x())/region.width()*frame.width()),qRound((32-region.y())/region.height()*frame.height()));
        };
        QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,paintAt);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(centerColor().red()>200);
        QTest::keyClick(floating,Qt::Key_Z,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),0);
        QTest::keyClick(floating,Qt::Key_Y,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),1);
        const auto at=canvas->mapToScene({canvas->width()/2,canvas->height()/2});QWheelEvent wheel(at,floating->mapToGlobal(at.toPoint()),{},QPoint(0,120),Qt::NoButton,Qt::NoModifier,Qt::NoScrollPhase,false);
        QCoreApplication::sendEvent(floating,&wheel);QVERIFY(canvas->zoom()>1);const auto zoom=canvas->zoom();
        const auto panAt=canvas->mapToScene({canvas->width()/2,canvas->height()/2}).toPoint();
        QTest::mousePress(floating,Qt::MiddleButton,Qt::NoModifier,panAt);QTest::mouseMove(floating,panAt+QPoint(12,8));QTest::mouseRelease(floating,Qt::MiddleButton,Qt::NoModifier,panAt+QPoint(12,8));
        const auto pan=canvas->documentRect().center()-QPointF(canvas->width()/2,canvas->height()/2);QCOMPARE(pan,QPointF(12,8));
        auto *documents=main->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QCOMPARE(documents->windows().size(),1);
        const auto first=documents->activeId(),host=documents->windows().first().toMap().value("id").toString();
        auto *extraTitle=findVisualItem(floating->contentItem(),"documentWindowTitle:"+host);QVERIFY(extraTitle);QCOMPARE(extraTitle->height(),qreal(0));
        auto *documentArea=findVisualItem(floating->contentItem(),"documentArea:"+host);QVERIFY(documentArea);QCOMPARE(documentArea->y(),qreal(2));
        auto *singleTitle=findVisualItem(floating->contentItem(),"documentTab:"+first);QVERIFY(singleTitle);QCOMPARE(singleTitle->property("color").value<QColor>(),floating->color());QCOMPARE(floating->color(),QColor("#202226"));
        for(auto *item:documentArea->findChildren<QObject*>())QVERIFY(item->property("tooltip").toString()!=QStringLiteral("新建画布"));
        const auto singlePreview=qEnvironmentVariable("DRAWVERSE_DOCK_PREVIEW");if(!singlePreview.isEmpty()){QTest::qWait(80);QVERIFY(floating->grabWindow().save(singlePreview+".single.png"));}
        const auto second=documents->newDocument(96,48,true);QVERIFY(!second.isEmpty());auto *other=documents->client(second);QVERIFY(other && other!=&client);QTRY_VERIFY(other->ready() && other->documentWidth()==96);
        other->setBrushColor(Qt::blue);other->setBrushRadius(3);InputSample sample;sample.position={20,20};QVERIFY(other->beginStroke(sample));other->endStroke();QTRY_COMPARE(other->undoDepth(),1);QCOMPARE(client.undoDepth(),1);
        const auto url=QUrl::fromLocalFile(temp.filePath("second.ora"));QSignalSpy files(other,&PaintCoreClient::fileFinished);QVERIFY(other->saveDocument(url));QTRY_COMPARE_WITH_TIMEOUT(files.size(),1,10000);QVERIFY(files.first().first().toBool());
        QVERIFY(documents->moveDocument(second,host));QTRY_COMPARE(documents->windows().size(),1);QTRY_COMPARE(applicationWindows().size(),2);
        QTRY_COMPARE(extraTitle->height(),qreal(22));QCOMPARE(floating->title(),QString("second.ora"));
        QVERIFY(documents->activate(first));QTRY_VERIFY(floating->title().startsWith(QStringLiteral("未命名")));QTRY_COMPARE(canvas->window(),floating);QCOMPARE(retained.data(),canvas);QCOMPARE(canvas->zoom(),zoom);QCOMPARE(canvas->documentRect().center()-QPointF(canvas->width()/2,canvas->height()/2),pan);
        // Each document keeps its own frame, history and layer tree in a tab group.
        QVERIFY(documents->activate(second));QTRY_COMPARE(floating->title(),QString("second.ora"));other->undo();QTRY_COMPARE(other->undoDepth(),0);QCOMPARE(client.undoDepth(),1);QVERIFY(documents->activate(first));
        auto *empty=findVisualItem(main->contentItem(),"documentArea:main");QVERIFY(empty);QCOMPARE(empty->property("color").value<QColor>(),QColor("#17191C"));
        const auto tile=workspace.groupDefinition(workspace.leftGroups().first().toMap().value("id").toString());QVERIFY(tile.value("host")=="main");
        QPointingDevice pen("Floating tablet",2001,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,2);
        const auto tabletAt=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(48,40)*canvas->zoom());
        QTabletEvent press(QEvent::TabletPress,&pen,tabletAt,floating->mapToGlobal(tabletAt.toPoint()),.75,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(floating,&press);QVERIFY(press.isAccepted());
        QTabletEvent release(QEvent::TabletRelease,&pen,tabletAt,floating->mapToGlobal(tabletAt.toPoint()),0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(floating,&release);QTRY_COMPARE(client.undoDepth(),2);
        QVERIFY(!documents->closeDocument(first));QCOMPARE(documents->documents().size(),2);
        const auto preview=qEnvironmentVariable("DRAWVERSE_DOCK_PREVIEW");if(!preview.isEmpty()){QTest::qWait(100);QVERIFY(floating->grabWindow().save(preview));}
        const auto original=floating->position();const auto gripAt=QPoint(24,10);QTest::mousePress(floating,Qt::LeftButton,Qt::NoModifier,gripAt);documents->beginDrag(first,true);QTRY_VERIFY(documents->dragging());
        QTest::mouseMove(floating,gripAt+QPoint(16,12));QTRY_VERIFY(floating->position()!=original);QVERIFY(documents->dragTarget().isEmpty());QTest::keyClick(floating,Qt::Key_Escape);QTRY_VERIFY(!documents->dragging());QTRY_COMPARE(floating->position(),original);QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,gripAt);
        QTest::mousePress(floating,Qt::LeftButton,Qt::NoModifier,gripAt);documents->beginDrag(first,true);QTRY_VERIFY(documents->dragging());
        const auto body=empty->mapToGlobal({empty->width()/2,100}).toPoint();QTest::mouseMove(floating,floating->mapFromGlobal(body));QVERIFY(documents->dragTarget().isEmpty());
        const auto edge=empty->mapToGlobal({empty->width()/2,10}).toPoint();QTest::mouseMove(floating,floating->mapFromGlobal(edge));QTRY_COMPARE(documents->dragTarget(),QString("main"));QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,floating->mapFromGlobal(edge));
        QTRY_COMPARE(applicationWindows().size(),1);QTRY_COMPARE(canvas->window(),main);QVERIFY(retainedWindow.isNull());QCOMPARE(canvas->zoom(),zoom);
        // A multi-document float returns to one title row after removing its other tab.
        documents->floatDocument(first);QTRY_COMPARE(documents->windows().size(),1);const auto oneHost=documents->windows().first().toMap().value("id").toString();
        QQuickWindow *oneWindow=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="documentWindow:"+oneHost)oneWindow=qobject_cast<QQuickWindow*>(w);return oneWindow!=nullptr;}());
        auto *oneTitle=findVisualItem(oneWindow->contentItem(),"documentWindowTitle:"+oneHost);QVERIFY(oneTitle);QCOMPARE(oneTitle->height(),qreal(0));
        QVERIFY(documents->moveDocument(second,oneHost));QTRY_COMPARE(oneTitle->height(),qreal(22));QVERIFY(documents->closeDocument(second,true));QTRY_COMPARE(oneTitle->height(),qreal(0));
        documents->returnWindow(oneHost);QTRY_COMPARE(applicationWindows().size(),1);
        const auto replacement=documents->newDocument(96,48,true);QVERIFY(!replacement.isEmpty());QTRY_VERIFY(documents->client(replacement)->ready() && documents->client(replacement)->documentWidth()==96);
        const auto opened=documents->openDocument(url);QVERIFY(!opened.isEmpty());auto *reopened=documents->client(opened);QVERIFY(reopened);QTRY_VERIFY(reopened->ready() && !reopened->fileBusy() && reopened->documentUrl()==url);QCOMPARE(reopened->documentWidth(),96);QCOMPARE(reopened->undoDepth(),0);
        // A later save failure must not reuse the initial-open failure handler.
        QSignalSpy reopenedFiles(reopened,&PaintCoreClient::fileFinished);
        QVERIFY(main->setProperty("savingBeforeAction",true));QVERIFY(main->setProperty("exiting",true));QVERIFY(main->setProperty("pendingAction","closeDocument"));
        QVERIFY(reopened->saveDocument(QUrl::fromLocalFile(temp.filePath("second.ora/blocked.ora"))));
        QTRY_COMPARE_WITH_TIMEOUT(reopenedFiles.size(),1,10000);QVERIFY(!reopenedFiles.first().first().toBool());
        QCOMPARE(documents->documents().size(),3);QCOMPARE(documents->client(opened),reopened);QVERIFY(reopened->ready());
        QVERIFY(!main->property("exiting").toBool());QVERIFY(!main->property("savingBeforeAction").toBool());QVERIFY(main->property("pendingAction").toString().isEmpty());
        QSignalSpy openFailures(documents,&DocumentManager::fileFailed);
        const auto missing=documents->openDocument(QUrl::fromLocalFile(temp.filePath("missing.ora")));QVERIFY(!missing.isEmpty());
        QTRY_COMPARE_WITH_TIMEOUT(openFailures.size(),1,10000);QTRY_COMPARE(documents->documents().size(),3);QVERIFY(!documents->client(missing));
        documents->requestClose(first);auto *unsaved=main->findChild<QObject*>("unsavedDocumentDialog");QVERIFY(unsaved);QTRY_VERIFY(unsaved->property("opened").toBool());QVERIFY(!documents->activate(replacement));QVERIFY(QMetaObject::invokeMethod(unsaved,"reject"));QTRY_VERIFY(!unsaved->property("opened").toBool());QCOMPARE(documents->documents().size(),3);
        // Closing the application first resolves the correct document, even with a file task running.
        QSignalSpy exitFiles(&client,&PaintCoreClient::fileFinished);
        QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath("first.ora"))));QVERIFY(client.fileBusy());
        QVERIFY(QMetaObject::invokeMethod(main,"close"));auto *busy=main->findChild<QObject*>("busyDocumentDialog");QVERIFY(busy);
        QTRY_VERIFY(busy->property("opened").toBool());QCOMPARE(main->property("pendingDocument").toString(),first);QCOMPARE(main->property("pendingAction").toString(),QString("closeDocument"));
        QVERIFY(QMetaObject::invokeMethod(busy,"reject"));QTRY_VERIFY(!busy->property("opened").toBool());QVERIFY(!main->property("exiting").toBool());QCOMPARE(documents->documents().size(),3);
        QTRY_COMPARE_WITH_TIMEOUT(exitFiles.size(),1,10000);QVERIFY(exitFiles.first().first().toBool());
        QVERIFY(documents->closeDocument(first,true));QVERIFY(documents->closeDocument(replacement,true));QVERIFY(documents->closeDocument(opened));QCOMPARE(documents->documents().size(),0);
        const auto fresh=documents->newDocument(24,24);QVERIFY(!fresh.isEmpty());QTRY_VERIFY(documents->client(fresh)->ready() && documents->client(fresh)->documentWidth()==24);QVERIFY(documents->closeDocument(fresh,true));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void panelColumnsUseOneHeaderAndDragTheActualWindow() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("workspace.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto first=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(first);workspace.detachPanel(first,"brush-settings");
        QString second;for(const auto &v:workspace.floatingGroups())if(v.toMap().value("active")=="brush-settings")second=v.toMap().value("id").toString();QVERIFY(!second.isEmpty());
        const auto payload=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",second},{"whole",true}}).toJson());QVERIFY(workspace.dockPayload(payload,"floating",first,"after"));
        QTRY_COMPARE(applicationWindows().size(),2);QQuickWindow *floating=nullptr;for(auto *w:applicationWindows())if(w!=main)floating=qobject_cast<QQuickWindow*>(w);QVERIFY(floating);
        auto *top=findVisualItem(floating->contentItem(),"columnHeader:"+first),*bottom=findVisualItem(floating->contentItem(),"columnHeader:"+second);QVERIFY(top && bottom);QVERIFY(top->isVisible());QVERIFY(!bottom->isVisible());
        auto *lowerBar=findVisualItem(floating->contentItem(),"panelTabBar:"+second),*backing=findVisualItem(floating->contentItem(),"floatingColumnBody");QVERIFY(lowerBar && backing);
        QVERIFY(std::abs(top->property("color").value<QColor>().alphaF()-.75)<.005);QCOMPARE(lowerBar->property("color").value<QColor>(),QColor("#05080a"));QCOMPARE(backing->property("color").value<QColor>(),QColor("#1c1e21"));
        const auto rendered=[&](QQuickItem *item,QPointF point){const auto image=floating->grabWindow();const auto at=item->mapToScene(point)*floating->devicePixelRatio();return image.pixelColor(at.toPoint());};
        QTRY_COMPARE(rendered(lowerBar,{lowerBar->width()/2,1}),QColor("#05080a"));QTRY_COMPARE(rendered(backing,{2,backing->height()-2}),QColor("#1c1e21"));
        const auto expanded=floating->size();workspace.setColumnCollapsed(first,true);
        QTRY_COMPARE(floating->width(),30);QTRY_COMPARE(floating->height(),expanded.height());
        auto *a=findVisualItem(floating->contentItem(),"dockTile:"+first),*b=findVisualItem(floating->contentItem(),"dockTile:"+second);QVERIFY(a && b);QTRY_COMPARE(b->mapToScene({0,0}).y(),a->mapToScene({0,0}).y()+a->height()+8);
        QTRY_VERIFY(findVisualItem(floating->contentItem(),"railPanel:brush-settings"));
        workspace.setColumnCollapsed(first,false);QTRY_COMPARE(floating->size(),expanded);
        const auto available=floating->screen()->availableGeometry();floating->setPosition({available.left()+(available.width()-floating->width())/2,available.top()});
        const auto before=floating->position();floating->requestActivate();QTRY_VERIFY(floating->isActive());
        const auto at=QPoint(30,4);QTest::mousePress(floating,Qt::LeftButton,Qt::NoModifier,at);workspace.beginDrag(first,"",true);QTRY_VERIFY(workspace.dragging());
        QTest::mouseMove(floating,at+QPoint(12,16));QTRY_VERIFY(floating->position()!=before);QVERIFY(workspace.dragTarget().isEmpty());
        const auto live=workspace.floatingWindows().first().toMap();QCOMPARE(live.value("x").toInt(),floating->x());QCOMPARE(live.value("y").toInt(),floating->y());
        QTest::keyClick(floating,Qt::Key_Escape);QTRY_VERIFY(!workspace.dragging());QTRY_COMPARE(floating->position(),before);QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,at);
        // A release near a different panel dynamically docks the whole column.
        const auto target=workspace.rightGroups().first().toMap().value("id").toString();auto *targetItem=findVisualItem(main->contentItem(),"dockGroup:"+target);QVERIFY(targetItem);
        QTest::mousePress(floating,Qt::LeftButton,Qt::NoModifier,at);workspace.beginDrag(first,"",true);QTRY_VERIFY(workspace.dragging());
        const auto global=targetItem->mapToGlobal({4,60}).toPoint();QTest::mouseMove(floating,floating->mapFromGlobal(global));QTRY_COMPARE(workspace.dragTarget(),target);QCOMPARE(workspace.dragPlacement(),QString("column-left"));
        QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,floating->mapFromGlobal(global));QTRY_VERIFY(!workspace.dragging());QTRY_COMPARE(applicationWindows().size(),1);QCOMPARE(workspace.groupDefinition(first).value("host").toString(),QString("main"));QCOMPARE(workspace.columnGroups(first),QStringList({first,second}));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void iconRailsSwitchSlideDismissAndDragPanels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.rightGroups().first().toMap().value("id").toString(),layers=workspace.rightGroups().last().toMap().value("id").toString(),brush=workspace.leftGroups().first().toMap().value("id").toString();
        workspace.detachPanel(layers,"history");const auto historyClass=workspace.groupForPanel("history");QVERIFY(workspace.dockPayload(QString::fromUtf8(QJsonDocument(QJsonObject{{"group",historyClass},{"whole",true}}).toJson()),"right",layers,"after"));
        workspace.setColumnCollapsed(color,true);workspace.detachPanel(brush,"brush-settings");QString incoming;
        for(const auto &v:workspace.floatingGroups())if(v.toMap().value("active")=="brush-settings")incoming=v.toMap().value("id").toString();QVERIFY(!incoming.isEmpty());
        const auto data=[](const QString &id){return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",id},{"whole",true}}).toJson());};
        QVERIFY(workspace.dockPayload(data(incoming),"right",layers,"after"));QTRY_COMPARE(applicationWindows().size(),1);
        const auto custom=workspace.addCustomPanel("Rail colors","palette");QVERIFY(!custom.isEmpty());
        const auto split=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",color},{"panel",custom},{"whole",false}}).toJson());
        QVERIFY(workspace.dockPayload(split,"right",color,"after"));
        for(const auto &id:workspace.columnGroups(color)) {QVERIFY(workspace.groupDefinition(id).value("icons").toBool());}
        for(const auto &v:workspace.layoutItems("main",main->width(),600)){const auto item=v.toMap();if(workspace.columnGroups(color).contains(item.value("id").toString())){QVERIFY(item.value("rail").toBool());QCOMPARE(item.value("rect").toRectF().width(),qreal(28));}}
        const auto click=[&](const QString &id){auto *icon=findVisualItem(main->contentItem(),"railPanel:"+id);if(!icon)return false;QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());return true;};
        const auto flyout=[&](const QString &panel)->QQuickWindow*{for(auto *w:applicationWindows())if(w->objectName().endsWith(":"+panel) && w->objectName().startsWith("panelFlyout:") && w->isVisible())return qobject_cast<QQuickWindow*>(w);return nullptr;};
        QVERIFY(click("history"));QTRY_VERIFY(flyout("history"));QPointer<QQuickWindow> history=flyout("history");QTRY_VERIFY(findVisualItem(history->contentItem(),"historyEntry:0"));
        QVERIFY(click("layers"));QTRY_VERIFY(flyout("layers"));QPointer<QQuickWindow> layer=flyout("layers");QVERIFY(history);QTRY_VERIFY(findVisualItem(layer->contentItem(),"layerList"));QVERIFY(findVisualItem(history->contentItem(),"historyEntry:0"));
        QCOMPARE(history->property("selectedPanel").toString(),QString("history"));QCOMPARE(layer->property("selectedPanel").toString(),QString("layers"));
        QVERIFY(click("color"));QTRY_VERIFY(flyout("color"));QPointer<QQuickWindow> palette=flyout("color");QTRY_COMPARE(applicationWindows().size(),4);
        QVERIFY(click("history"));QTRY_VERIFY(history.isNull());QVERIFY(layer && palette);QTRY_COMPARE(applicationWindows().size(),3);
        QVERIFY(click("history"));QTRY_VERIFY(flyout("history"));history=flyout("history");QTRY_VERIFY(history->isActive());QVERIFY(layer && palette);QTRY_COMPARE(applicationWindows().size(),4);
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            HWND front=nullptr;for(auto hwnd=GetTopWindow(nullptr);hwnd;hwnd=GetWindow(hwnd,GW_HWNDNEXT))if(hwnd==reinterpret_cast<HWND>(history->winId()) || hwnd==reinterpret_cast<HWND>(layer->winId()) || hwnd==reinterpret_cast<HWND>(palette->winId())){front=hwnd;break;}
            QCOMPARE(front,reinterpret_cast<HWND>(history->winId()));
        }
#endif
        auto *background=findVisualItem(main->contentItem(),"__railBackground:"+color);QVERIFY(background);QCOMPARE(background->property("color").value<QColor>(),QColor("#1c1e21"));
        const auto blank=background->mapToScene({background->width()/2,background->height()-12}).toPoint();QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,blank);
        QEvent deactivate(QEvent::WindowDeactivate);QCoreApplication::sendEvent(history,&deactivate);QTest::qWait(30);QVERIFY(history && layer && palette);QCOMPARE(applicationWindows().size(),4);
        const auto image=main->grabWindow();const auto dpr=main->devicePixelRatio();QCOMPARE(image.pixelColor(qRound(blank.x()*dpr),qRound(blank.y()*dpr)),QColor("#1c1e21"));
        palette->resize(std::min(260,palette->width()),palette->minimumHeight());QTest::qWait(30);
        const auto before=palette->position();auto *grip=findVisualItem(palette->contentItem(),"groupGrip:"+color);QVERIFY(grip);
        auto *area=findVisualItem(main->contentItem(),"mainDockWorkspace");QVERIFY(area);const auto visibleBottom=std::min(qreal(workspace.availableScreenGeometry(palette).bottom()+1),area->mapToGlobal({0,0}).y()+area->height());
        const int delta=palette->y()+palette->height()>=visibleBottom-1?-30:30;
        const auto at=grip->mapToScene({10,4}).toPoint();QTest::mousePress(palette,Qt::LeftButton,Qt::NoModifier,at);QTest::mouseMove(palette,at+QPoint(0,delta));QTest::mouseRelease(palette,Qt::LeftButton,Qt::NoModifier,at+QPoint(0,delta));QTRY_VERIFY(palette->y()!=before.y());QCOMPARE(palette->x(),before.x());
        auto *retract=findVisualItem(layer->contentItem(),"flyoutRetract:layers");QVERIFY(retract);QTest::mouseClick(layer,Qt::LeftButton,Qt::NoModifier,retract->mapToScene({retract->width()/2,retract->height()/2}).toPoint());QTRY_VERIFY(layer.isNull());QVERIFY(history && palette);
        QTest::keyClick(history,Qt::Key_Escape);QTRY_VERIFY(history.isNull());QVERIFY(palette);
        auto *icon=findVisualItem(main->contentItem(),"railGrip:history");QVERIFY(icon);const auto iconAt=icon->mapToScene({14,14}).toPoint();QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,iconAt);QTest::mouseMove(main,iconAt+QPoint(-50,12));QTRY_VERIFY(workspace.dragging());
        QQuickWindow *detached=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName().startsWith("floatingDock:"))detached=qobject_cast<QQuickWindow*>(w);return detached!=nullptr;}());QTest::mouseRelease(detached,Qt::LeftButton,Qt::NoModifier,{24,4});QTRY_VERIFY(!workspace.dragging());QVERIFY(palette);
        QString historyGroup;for(const auto &v:workspace.floatingGroups())if(v.toMap().value("panels").toStringList().contains("history"))historyGroup=v.toMap().value("id").toString();QVERIFY(!historyGroup.isEmpty());QVERIFY(!workspace.groupDefinition(historyGroup).value("icons").toBool());
        workspace.returnGroup(historyGroup);QTRY_VERIFY(workspace.floatingWindows().isEmpty());QVERIFY(!workspace.groupDefinition(historyGroup).value("icons").toBool());QVERIFY(palette);
        workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));for(const auto &id:restored.columnGroups(color))QVERIFY(restored.groupDefinition(id).value("icons").toBool());
        const auto preview=qEnvironmentVariable("DRAWVERSE_RAIL_PREVIEW");if(!preview.isEmpty()){QTest::qWait(50);QVERIFY(main->grabWindow().save(preview+".main.png"));QVERIFY(palette->grabWindow().save(preview+".color.png"));}
        QSettings settings(temp.filePath("layout.ini"),QSettings::IniFormat);auto saved=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();auto groups=saved.value("groups").toArray();
        for(int i=0;i<groups.size();++i){auto g=groups[i].toObject();if(g.value("id")==brush){g.insert("collapsed",true);groups[i]=g;}}
        saved.insert("groups",groups);settings.setValue("workspace",QJsonDocument(saved).toJson());settings.sync();WorkspaceManager migrated(temp.filePath("layout.ini"));QVERIFY(migrated.groupDefinition(brush).value("icons").toBool());QVERIFY(!migrated.groupDefinition(brush).value("collapsed").toBool());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void floatingIconRailsKeepBackgroundSidePanelsAndReturnState() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.rightGroups().first().toMap().value("id").toString(),group=workspace.rightGroups().last().toMap().value("id").toString();workspace.detachPanel(group,"history");const auto historyClass=workspace.groupForPanel("history");const auto detachColumn=[&]{workspace.detachGroup(group);QVERIFY(workspace.dockPayload(QString::fromUtf8(QJsonDocument(QJsonObject{{"group",historyClass},{"whole",true}}).toJson()),"floating",group,"after"));};
        workspace.setColumnCollapsed(group,true);detachColumn();
        const auto floatingWindow=[&]()->QQuickWindow*{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+group && w->isVisible())return qobject_cast<QQuickWindow*>(w);return nullptr;};
        QTRY_VERIFY(floatingWindow());QQuickWindow *rail=floatingWindow();const auto available=workspace.availableScreenGeometry(rail);
        rail->setPosition(available.left(),available.top()+20);rail->resize(30,130);QTRY_COMPARE(rail->height(),130);
        const auto click=[&](const QString &panel){auto *icon=findVisualItem(rail->contentItem(),"railPanel:"+panel);if(!icon)return false;QTest::mouseClick(rail,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());return true;};
        const auto flyout=[&](const QString &panel)->QQuickWindow*{for(auto *w:applicationWindows())if(w->objectName().startsWith("panelFlyout:") && w->objectName().endsWith(":"+panel) && w->isVisible())return qobject_cast<QQuickWindow*>(w);return nullptr;};
        QVERIFY(click("layers"));QTRY_VERIFY(flyout("layers"));QPointer<QQuickWindow> layers=flyout("layers");
        QVERIFY(click("history"));QTRY_VERIFY(flyout("history"));QPointer<QQuickWindow> history=flyout("history");QVERIFY(layers);
        QTRY_VERIFY(findVisualItem(layers->contentItem(),"layerList"));QTRY_VERIFY(findVisualItem(history->contentItem(),"historyEntry:0"));
        QVERIFY(layers->height()>rail->height()+80);QVERIFY(layers->y()+layers->height()>rail->y()+rail->height());
        auto *historyIcon=findVisualItem(rail->contentItem(),"railPanel:history");QVERIFY(historyIcon);
        QVERIFY(std::abs(history->y()-historyIcon->mapToGlobal({0,0}).y())<=1);QCOMPARE(history->x(),rail->x()+rail->width()-1);
        auto *resizeFrame=findVisualItem(layers->contentItem(),"panelFlyoutResizeFrame");QVERIFY(resizeFrame && resizeFrame->isVisible());
        layers->resize(260,layers->minimumHeight());QTRY_COMPARE(layers->width(),260);QTRY_COMPARE(layers->height(),layers->minimumHeight());
        layers->resize(300,layers->minimumHeight()+12);QTRY_COMPARE(layers->width(),300);QTRY_COMPARE(layers->height(),layers->minimumHeight()+12);QCOMPARE(rail->height(),130);
        const auto historyY=history->y();rail->setY(rail->y()-8);QTRY_COMPARE(history->y(),historyY-8);
        QTest::mouseClick(rail,Qt::LeftButton,Qt::NoModifier,{14,rail->height()-10});QEvent deactivate(QEvent::WindowDeactivate);QCoreApplication::sendEvent(history,&deactivate);QTest::qWait(30);QVERIFY(layers && history);
        const auto lastSize=layers->size();const auto lastPosition=layers->position();
        QVERIFY(click("layers"));QTRY_VERIFY(layers.isNull());QVERIFY(history);
        QVERIFY(click("layers"));QTRY_VERIFY(flyout("layers"));layers=flyout("layers");QTRY_VERIFY(layers->isActive());QCOMPARE(layers->size(),lastSize);QCOMPARE(layers->position(),lastPosition);QVERIFY(history);
        resizeFrame=findVisualItem(layers->contentItem(),"panelFlyoutResizeFrame");QVERIFY(resizeFrame);
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            HWND front=nullptr;for(auto hwnd=GetTopWindow(nullptr);hwnd;hwnd=GetWindow(hwnd,GW_HWNDNEXT))if(hwnd==reinterpret_cast<HWND>(layers->winId()) || hwnd==reinterpret_cast<HWND>(history->winId())){front=hwnd;break;}
            QCOMPARE(front,reinterpret_cast<HWND>(layers->winId()));
            const auto original=layers->size();RECT rect{};QVERIFY(GetWindowRect(reinterpret_cast<HWND>(layers->winId()),&rect));
            QTRY_COMPARE(resizeFrame->width(),qreal(layers->width()));QTRY_COMPARE(resizeFrame->height(),qreal(layers->height()));
            const auto inset=static_cast<LONG>(std::lround(3*layers->devicePixelRatio()));
            const POINT corner{rect.right-inset,rect.bottom-inset};QVERIFY(SetCursorPos(corner.x,corner.y));QTest::qWait(30);
            INPUT down{};down.type=INPUT_MOUSE;down.mi.dwFlags=MOUSEEVENTF_LEFTDOWN;
            const auto release=qScopeGuard([]{INPUT up{};up.type=INPUT_MOUSE;up.mi.dwFlags=MOUSEEVENTF_LEFTUP;SendInput(1,&up,sizeof(INPUT));});
            QCOMPARE(SendInput(1,&down,sizeof(INPUT)),UINT(1));
            // Release from outside the UI thread so a native sizing loop cannot block it.
            std::jthread resize([corner]{Sleep(400);SetCursorPos(corner.x+30,corner.y+20);Sleep(100);INPUT up{};up.type=INPUT_MOUSE;up.mi.dwFlags=MOUSEEVENTF_LEFTUP;SendInput(1,&up,sizeof(INPUT));});
            QTRY_VERIFY(layers->width()>original.width() && layers->height()>original.height());QCOMPARE(rail->height(),130);
        }
#endif
        const auto image=rail->grabWindow(),mainImage=main->grabWindow();const auto dpr=rail->devicePixelRatio();QCOMPARE(image.pixelColor(qRound(14*dpr),qRound((rail->height()-10)*dpr)),QColor("#1c1e21"));
        auto *background=findVisualItem(main->contentItem(),"__railBackground:"+color);QVERIFY(background);const auto blank=background->mapToScene({14,background->height()-12});
        QCOMPARE(mainImage.pixelColor(qRound(blank.x()*main->devicePixelRatio()),qRound(blank.y()*main->devicePixelRatio())),QColor("#1c1e21"));
        const auto preview=qEnvironmentVariable("DRAWVERSE_FLOATING_RAIL_PREVIEW");if(!preview.isEmpty()){QTest::qWait(50);QVERIFY(rail->grabWindow().save(preview+".rail.png"));QVERIFY(layers->grabWindow().save(preview+".layers.png"));QVERIFY(history->grabWindow().save(preview+".history.png"));}
        workspace.setColumnCollapsed(group,false);QTRY_VERIFY(layers.isNull() && history.isNull());workspace.returnWindow(group);QTRY_VERIFY(workspace.floatingWindows().isEmpty());
        QVERIFY(!workspace.groupDefinition(group).value("icons").toBool());QVERIFY(workspace.groupDefinition(color).value("icons").toBool());QCOMPARE(workspace.columnGroups(group),QStringList({group,historyClass}));
        detachColumn();workspace.setColumnCollapsed(group,true);workspace.returnWindow(group);QTRY_VERIFY(workspace.floatingWindows().isEmpty());QVERIFY(workspace.groupDefinition(group).value("icons").toBool());QCOMPARE(workspace.columnGroups(group),QStringList({group,historyClass}));
        for(const bool collapsed:{false,true}) {
            detachColumn();workspace.setColumnCollapsed(group,collapsed);QTRY_VERIFY(floatingWindow());rail=floatingWindow();QTRY_VERIFY(rail->isExposed());QTest::qWait(30);auto *area=findVisualItem(main->contentItem(),"mainDockWorkspace");QVERIFY(area);
            const QPoint at(10,7);QTest::mousePress(rail,Qt::LeftButton,Qt::NoModifier,at);workspace.beginDrag(group,"",true);QTRY_VERIFY(workspace.dragging());
            const auto global=area->mapToGlobal({area->width()+4,100}).toPoint();QTest::mouseMove(rail,rail->mapFromGlobal(global));QTRY_COMPARE(workspace.dragTarget(),QString("__workspace_right"));
            QTest::mouseRelease(rail,Qt::LeftButton,Qt::NoModifier,rail->mapFromGlobal(global));QTRY_VERIFY(!workspace.dragging());QTRY_VERIFY(workspace.floatingWindows().isEmpty());
            QCOMPARE(workspace.groupDefinition(group).value("icons").toBool(),collapsed);QVERIFY(workspace.groupDefinition(color).value("icons").toBool());QCOMPARE(workspace.columnGroups(group),QStringList({group,historyClass}));
        }
        workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));QVERIFY(restored.groupDefinition(group).value("icons").toBool());QCOMPARE(restored.columnGroups(group),QStringList({group,historyClass}));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void columnEdgesAndInnerRowsPreserveCategoryTrees() {
        QTemporaryDir temp;WorkspaceManager workspace(temp.filePath("layout.ini"));
        const auto color=workspace.groupForPanel("color"),layers=workspace.groupForPanel("layers");
        const auto data=[](const QString &id){return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",id},{"whole",true}}).toJson());};
        workspace.detachPanel(layers,"navigator");const auto nav=workspace.groupForPanel("navigator");
        QVERIFY(workspace.dockPayload(data(nav),"right",color,"left"));
        QCOMPARE(workspace.categoryGroups(nav),QStringList({nav,color}));QCOMPARE(workspace.columnGroups(nav),QStringList({nav,color,layers}));
        workspace.detachPanel(layers,"history");const auto history=workspace.groupForPanel("history");QVERIFY(workspace.dockPayload(data(history),"right",color,"merge"));
        QCOMPARE(workspace.categoryDefinition(nav).value("panels").toStringList(),QStringList({"navigator","color","history"}));
        const auto brush=workspace.groupForPanel("brush");workspace.detachGroup(brush);QVERIFY(workspace.dockPayload(data(brush),"right",color,"column-left"));
        QCOMPARE(workspace.columnGroups(brush),QStringList({brush}));QCOMPARE(workspace.columnGroups(nav),QStringList({nav,color,layers}));
        workspace.setColumnCollapsed(color,true);
        const auto items=workspace.layoutItems("main",1600,800);int separators=0,rows=0;
        for(const auto &v:items){const auto item=v.toMap();if(item.value("kind")=="railSeparator"){++separators;QCOMPARE(item.value("rect").toRectF().height(),qreal(8));}
            if(item.value("id")==nav){++rows;QCOMPARE(item.value("panels").toStringList(),QStringList({"navigator","color","history"}));QCOMPARE(item.value("rect").toRectF().width(),qreal(28));}}
        QCOMPARE(separators,1);QCOMPARE(rows,1);
        int expanded=0;for(const auto &v:workspace.layoutItems("__category:"+nav,600,400)){const auto item=v.toMap();if(item.value("kind")=="leaf"){++expanded;QVERIFY(!item.value("rail").toBool());}}
        QCOMPARE(expanded,2);workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));QCOMPARE(restored.columnGroups(nav),workspace.columnGroups(nav));QCOMPARE(restored.categoryGroups(nav),workspace.categoryGroups(nav));QVERIFY(restored.groupDefinition(color).value("icons").toBool());
        QSet<QString> seen,splits;auto bad=DockTree::split(DockTree::leaf(nav),DockTree::leaf(color),"vertical");bad.insert("role","row");QVERIFY(!DockTree::validate(bad,QSet<QString>{nav,color},seen,splits));
        // A legacy horizontal split without the new role stays two independent columns.
        restored.resetLayout();const auto left=restored.groupForPanel("brush"),right=restored.groupForPanel("color");restored.detachGroup(left);QVERIFY(restored.dockPayload(data(left),"right",right,"column-right"));QCOMPARE(restored.categoryGroups(left),QStringList({left}));
    }
    void categoryFlyoutsShareRowsTabsAndRetractWithoutExpanding() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.groupForPanel("color"),layers=workspace.groupForPanel("layers");const auto data=[](const QString &id){return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",id},{"whole",true}}).toJson());};
        workspace.detachPanel(layers,"navigator");const auto nav=workspace.groupForPanel("navigator");QVERIFY(workspace.dockPayload(data(nav),"right",color,"left"));workspace.detachPanel(layers,"history");QVERIFY(workspace.dockPayload(data(workspace.groupForPanel("history")),"right",color,"merge"));QTRY_COMPARE(applicationWindows().size(),1);
        QCOMPARE(workspace.sidePlacement(nav,"left"),QString("column-left"));QCOMPARE(workspace.sidePlacement(nav,"right"),QString("right"));QCOMPARE(workspace.sidePlacement(color,"left"),QString("left"));QCOMPARE(workspace.sidePlacement(color,"right"),QString("column-right"));
        workspace.setColumnCollapsed(nav,true);auto *line=findVisualItem(main->contentItem(),"railSeparatorLine");QVERIFY(line);QCOMPARE(line->width(),qreal(18));QVERIFY(std::abs(line->x()+line->width()/2-line->parentItem()->width()/2)<.01);QCOMPARE(line->property("color").value<QColor>(),QColor("#3b3d42"));
        const auto click=[&](const QString &panel){auto *icon=findVisualItem(main->contentItem(),"railPanel:"+panel);if(!icon)return false;QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());return true;};
        const auto flyout=[&](const QString &panel)->QQuickWindow*{for(auto *w:applicationWindows())if(w->objectName().startsWith("panelFlyout:") && w->objectName().endsWith(":"+panel) && w->isVisible())return qobject_cast<QQuickWindow*>(w);return nullptr;};
        QVERIFY(click("navigator"));QTRY_VERIFY(flyout("navigator"));QPointer<QQuickWindow> row=flyout("navigator");QTRY_VERIFY(findVisualItem(row->contentItem(),"navigatorCanvas"));QTRY_VERIFY(findVisualItem(row->contentItem(),"historyList"));QVERIFY(findVisualItem(row->contentItem(),"panelTab:history"));QVERIFY(findVisualItem(row->contentItem(),"panelTab:color"));
        auto *navTile=findVisualItem(row->contentItem(),"dockTile:"+nav),*colorTile=findVisualItem(row->contentItem(),"dockTile:"+color);QVERIFY(navTile && colorTile);QVERIFY(navTile->x()+navTile->width()<colorTile->x()+.1);QCOMPARE(navTile->y(),colorTile->y());
        auto *historyList=findVisualItem(row->contentItem(),"historyList");QVERIFY(historyList);for(auto *control:historyList->parentItem()->findChildren<QObject*>())QVERIFY(control->property("glyph").toString()!="undo" && control->property("glyph").toString()!="redo");
        QVERIFY(click("color"));QTRY_VERIFY(findVisualItem(row->contentItem(),"colorWheel"));QCOMPARE(row->property("selectedPanel").toString(),QString("color"));QTRY_COMPARE(applicationWindows().size(),2);
        row->resize(row->minimumWidth()+20,row->minimumHeight()+35);QTest::qWait(30);const auto lastSize=row->size();QVERIFY(click("layers"));QTRY_VERIFY(flyout("layers"));QPointer<QQuickWindow> other=flyout("layers");QCOMPARE(applicationWindows().size(),3);
        auto *grip=findVisualItem(row->contentItem(),"groupGrip:"+nav);QVERIFY(grip);QTest::mouseDClick(row,Qt::LeftButton,Qt::NoModifier,grip->mapToScene({10,4}).toPoint());QTRY_VERIFY(row.isNull());QVERIFY(other);for(const auto &id:workspace.columnGroups(nav))QVERIFY(workspace.groupDefinition(id).value("icons").toBool());
        QVERIFY(click("history"));QTRY_VERIFY(flyout("history"));row=flyout("history");QCOMPARE(row->size(),lastSize);QTRY_VERIFY(findVisualItem(row->contentItem(),"navigatorCanvas"));QTRY_VERIFY(findVisualItem(row->contentItem(),"historyList"));QVERIFY(click("history"));QTRY_VERIFY(row.isNull());QVERIFY(other);
        QVERIFY(click("navigator"));QTRY_VERIFY(flyout("navigator"));row=flyout("navigator");const auto preview=qEnvironmentVariable("DRAWVERSE_CATEGORY_PREVIEW");if(!preview.isEmpty()){QTest::qWait(80);QVERIFY(main->grabWindow().save(preview+".main.png"));QVERIFY(row->grabWindow().save(preview+".row.png"));}
        QCOMPARE(QQmlProperty::read(row,"palette.button").value<QColor>(),QColor("#25262b"));QCOMPARE(QQmlProperty::read(row,"palette.buttonText").value<QColor>(),QColor("#bababa"));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void navigatorTracksVisibleDocumentAndSharesContentAcrossWindows() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto group=workspace.groupForPanel("navigator");workspace.setActive(group,"navigator");
        const auto check=[&](QQuickWindow *window,CanvasItem *view) {
            auto *mini=qobject_cast<CanvasItem*>(findVisualItem(window->contentItem(),"navigatorCanvas"));auto *frame=findVisualItem(window->contentItem(),"navigatorViewFrame");auto *preview=findVisualItem(window->contentItem(),"navigatorPreview");if(!mini || !frame || !preview || !frame->isVisible())return false;
            const auto rect=view->visibleDocumentRect();const auto expected=QRectF(mini->documentRect().topLeft()+rect.topLeft()*mini->zoom(),rect.size()*mini->zoom());
            for(auto *object:preview->parentItem()->findChildren<QObject*>())if(object->property("text").toString()==QStringLiteral("适合窗口  F") || object->property("text").toString()==QStringLiteral("实际像素  100%"))return false;
            return QLineF(frame->position(),expected.topLeft()).length()<.01 && std::abs(frame->width()-expected.width())<.01 && std::abs(frame->height()-expected.height())<.01 && QQmlProperty::read(frame,"border.color").value<QColor>()==QColor("#ff3434") && mini->size()==preview->size();
        };
        auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->fitToView();QTRY_VERIFY(check(main,canvas));QCOMPARE(canvas->visibleDocumentRect(),QRectF(0,0,960,640));canvas->zoomBy(3);QTRY_VERIFY(check(main,canvas));QVERIFY(canvas->visibleDocumentRect().width()<960);
        const auto before=canvas->visibleDocumentRect();const auto center=canvas->mapToScene({canvas->width()/2,canvas->height()/2}).toPoint();QTest::mousePress(main,Qt::MiddleButton,Qt::NoModifier,center);QTest::mouseMove(main,center+QPoint(25,15));QTest::mouseRelease(main,Qt::MiddleButton,Qt::NoModifier,center+QPoint(25,15));QTRY_VERIFY(canvas->visibleDocumentRect()!=before);QTRY_VERIFY(check(main,canvas));QCOMPARE(client.undoDepth(),0);
        workspace.setColumnCollapsed(group,true);auto *icon=findVisualItem(main->contentItem(),"railPanel:navigator");QVERIFY(icon);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName().endsWith(":navigator"))floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());QTRY_VERIFY(check(floating,canvas));
        const auto retained=canvas->visibleDocumentRect();auto *documents=main->findChild<DocumentManager*>("documentManager");QVERIFY(documents);const auto first=documents->activeId(),second=documents->newDocument(128,64,true);QVERIFY(!second.isEmpty());QTRY_VERIFY(documents->activeClient()->ready());auto current=[&]{return qobject_cast<CanvasItem*>(main->property("canvas").value<QObject*>());};QTRY_VERIFY(current() && current()!=canvas);QTRY_VERIFY(check(floating,current()));
        documents->floatDocument(second);QTRY_VERIFY(current()->window()!=main);current()->zoomBy(3);QTRY_VERIFY(check(floating,current()));QVERIFY(documents->activate(first));QTRY_VERIFY(check(floating,canvas));QCOMPARE(canvas->visibleDocumentRect(),retained);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void movingWholeCategoryColumnsPreservesRowsAndOuterEdges() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.groupForPanel("color"),layers=workspace.groupForPanel("layers"),brush=workspace.groupForPanel("brush");workspace.detachPanel(layers,"navigator");const auto nav=workspace.groupForPanel("navigator");const auto data=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",nav},{"whole",true}}).toJson());QVERIFY(workspace.dockPayload(data,"right",color,"left"));
        const auto members=workspace.columnGroups(nav),row=workspace.categoryGroups(nav);QCOMPARE(row,QStringList({nav,color}));QCOMPARE(members,QStringList({nav,color,layers}));
        auto *grip=findVisualItem(main->contentItem(),"groupGrip:"+nav);QVERIFY(grip);const auto at=grip->mapToScene({10,4}).toPoint();QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,at);workspace.beginDrag(nav,"",true);QTRY_VERIFY(workspace.dragging());
        QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+nav && w->isVisible())floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());QCOMPARE(workspace.categoryGroups(nav),row);QCOMPARE(workspace.columnGroups(nav),members);
        QTest::keyClick(floating,Qt::Key_Escape);QTRY_VERIFY(!workspace.dragging());QTRY_VERIFY(workspace.floatingWindows().isEmpty());QCOMPARE(workspace.categoryGroups(nav),row);QTest::mouseRelease(main,Qt::LeftButton,Qt::NoModifier,at);
        grip=findVisualItem(main->contentItem(),"groupGrip:"+nav);QVERIFY(grip);QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,grip->mapToScene({10,4}).toPoint());workspace.beginDrag(nav,"",true);QTRY_VERIFY(workspace.dragging());floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+nav && w->isVisible())floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());
        workspace.setColumnCollapsed(brush,true);QTest::qWait(30);
        auto *target=findVisualItem(main->contentItem(),"dockTile:"+brush),*railBackground=findVisualItem(main->contentItem(),"__railBackground:"+brush);QVERIFY(target && railBackground);QVERIFY(railBackground->height()>target->height()+100);
        const auto global=railBackground->mapToGlobal({railBackground->width()-4,railBackground->height()-60}).toPoint();QTest::mouseMove(floating,floating->mapFromGlobal(global));QTRY_COMPARE(workspace.dragTarget(),brush);QCOMPARE(workspace.dragPlacement(),QString("column-right"));
        auto *glow=findVisualItem(main->contentItem(),"dockColumnPreview:main");QVERIFY(glow && glow->isVisible());QVERIFY(glow->height()>target->height()-1);
        QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,floating->mapFromGlobal(global));QTRY_VERIFY(!workspace.dragging());QTRY_VERIFY(workspace.floatingWindows().isEmpty());QCOMPARE(workspace.categoryGroups(nav),row);QCOMPARE(workspace.columnGroups(nav),members);QCOMPARE(workspace.columnGroups(brush),QStringList({brush}));
        workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));QCOMPARE(restored.categoryGroups(nav),row);QCOMPARE(restored.columnGroups(nav),members);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void panelViewsPersistIndependentlyAndIgnoreInvalidOptionalState() {
        QTemporaryDir temp;const auto path=temp.filePath("layout.ini");WorkspaceManager workspace(path);
        const auto group=workspace.rightGroups().last().toMap().value("id").toString();workspace.setColumnCollapsed(group,true);
        workspace.updatePanelView("layers",260,340,48);workspace.updatePanelView("color",230,210,-20);workspace.saveLayout();
        WorkspaceManager restored(path);QCOMPARE(restored.panelView("layers"),workspace.panelView("layers"));QCOMPARE(restored.panelView("color"),workspace.panelView("color"));QVERIFY(restored.groupDefinition(group).value("icons").toBool());
        restored.updatePanelView("layers",0,200,0);restored.updatePanelView("unknown",200,200,0);restored.updatePanelView("color",200,200,std::numeric_limits<int>::min());
        QCOMPARE(restored.panelView("layers"),workspace.panelView("layers"));QVERIFY(restored.panelView("unknown").isEmpty());QCOMPARE(restored.panelView("color"),workspace.panelView("color"));
        QSettings settings(path,QSettings::IniFormat);auto saved=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();auto views=saved.value("panelViews").toObject();
        views.insert("layers",QJsonObject{{"width",0},{"height",200},{"offset",0}});views.insert("color",QJsonObject{{"width",200},{"height",200},{"offset",std::numeric_limits<int>::min()}});saved.insert("panelViews",views);settings.setValue("workspace",QJsonDocument(saved).toJson());settings.sync();
        WorkspaceManager invalid(path);QVERIFY(invalid.panelView("layers").isEmpty());QVERIFY(invalid.panelView("color").isEmpty());QVERIFY(invalid.groupDefinition(group).value("icons").toBool());
        saved.remove("panelViews");settings.setValue("workspace",QJsonDocument(saved).toJson());settings.sync();WorkspaceManager legacy(path);QVERIFY(legacy.panelView("layers").isEmpty());QVERIFY(legacy.groupDefinition(group).value("icons").toBool());
        restored.resetLayout();QVERIFY(restored.panelView("layers").isEmpty());
    }
    void panelPresentationRemembersFlyoutsAndPaintsActualHeaderAlpha() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto brush=workspace.leftGroups().first().toMap().value("id").toString(),color=workspace.rightGroups().first().toMap().value("id").toString();workspace.setActive(brush,"brush");
        auto *nav=findVisualItem(main->contentItem(),"columnHeader:"+brush),*tabs=findVisualItem(main->contentItem(),"panelTabBar:"+brush),*tab=findVisualItem(main->contentItem(),"panelTab:brush");QVERIFY(nav && tabs && tab);
        QCOMPARE(nav->property("color").value<QColor>(),QColor("#05080a"));QCOMPARE(tabs->property("color").value<QColor>(),QColor("#05080a"));QCOMPARE(tab->property("radius").toReal(),qreal(3));
        auto *preset=findVisualItem(main->contentItem(),"brushPreset:round-pressure");QTRY_VERIFY(preset);QCOMPARE(QQmlProperty::read(preset,"border.color").value<QColor>(),QColor("#1b1c1f"));
        workspace.setColumnCollapsed(color,true);const auto click=[&](const QString &panel){auto *icon=findVisualItem(main->contentItem(),"railPanel:"+panel);if(!icon)return false;QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());return true;};
        const auto flyout=[&](const QString &panel)->QQuickWindow*{for(auto *w:applicationWindows())if(w->objectName().endsWith(":"+panel) && w->objectName().startsWith("panelFlyout:") && w->isVisible())return qobject_cast<QQuickWindow*>(w);return nullptr;};
        QVERIFY(click("color"));QTRY_VERIFY(flyout("color"));QPointer<QQuickWindow> panel=flyout("color");panel->resize(250,panel->minimumHeight());QTest::qWait(30);
        auto *grip=findVisualItem(panel->contentItem(),"groupGrip:"+color);QVERIFY(grip);const auto from=grip->mapToScene({10,4}).toPoint();
        const auto initial=panel->position();auto *area=findVisualItem(main->contentItem(),"mainDockWorkspace");QVERIFY(area);const int delta=panel->y()+panel->height()>=std::min(workspace.availableScreenGeometry(panel).bottom()+1,qRound(area->mapToGlobal({0,0}).y()+area->height()))-1?-20:20;
        QTest::mousePress(panel,Qt::LeftButton,Qt::NoModifier,from);QTest::mouseMove(panel,from+QPoint(0,delta));QTest::mouseRelease(panel,Qt::LeftButton,Qt::NoModifier,from+QPoint(0,delta));QTRY_VERIFY(panel->position()!=initial);
        const auto size=panel->size();const auto position=panel->position();const auto view=workspace.panelView("color");QVERIFY(!view.isEmpty());
        QVERIFY(click("history"));QTRY_VERIFY(flyout("history"));QPointer<QQuickWindow> history=flyout("history");
        QVERIFY(click("color"));QTRY_VERIFY(panel.isNull());QVERIFY(history);QCOMPARE(workspace.panelView("color"),view);
        QVERIFY(click("color"));QTRY_VERIFY(flyout("color"));panel=flyout("color");QCOMPARE(panel->size(),size);QCOMPARE(panel->position(),position);QVERIFY(history);
        workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));QCOMPARE(restored.panelView("color"),workspace.panelView("color"));
        const auto checkHeader=[&](QQuickWindow *window,const QString &id) {
            auto *header=findVisualItem(window->contentItem(),"columnHeader:"+id),*bar=findVisualItem(window->contentItem(),"panelTabBar:"+id);if(!header || !bar)return false;
            const auto c=header->property("color").value<QColor>();if(c.red()!=5 || c.green()!=8 || c.blue()!=10 || std::abs(c.alphaF()-.75)>.005)return false;
            const auto image=window->grabWindow();if(image.isNull())return false;const auto at=header->mapToScene({header->width()/2,4});const auto pixel=image.pixelColor(qRound(at.x()*window->devicePixelRatio()),qRound(at.y()*window->devicePixelRatio()));
            const auto body=image.pixelColor(qRound(3*window->devicePixelRatio()),qRound((window->height()-40)*window->devicePixelRatio()));
            // The software renderer returns RGB32 with premultiplied channels; other backends retain alpha.
            const auto expected=image.hasAlphaChannel()?QColor(5,8,10,191):QColor(4,6,7);
            return window->color()==Qt::transparent && std::abs(pixel.alpha()-expected.alpha())<=1 && std::abs(pixel.red()-expected.red())<=1 && std::abs(pixel.green()-expected.green())<=1 && std::abs(pixel.blue()-expected.blue())<=1 && body==QColor("#1c1e21");
        };
        QTRY_VERIFY(checkHeader(panel,color));workspace.detachGroup(brush);QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+brush)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());QTRY_VERIFY(checkHeader(floating,brush));
        const auto preview=qEnvironmentVariable("DRAWVERSE_PANEL_PRESENTATION_PREVIEW");if(!preview.isEmpty()){QTest::qWait(80);QVERIFY(main->grabWindow().save(preview+".main.png"));QVERIFY(panel->grabWindow().save(preview+".color.png"));QVERIFY(floating->grabWindow().save(preview+".brush.png"));}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void compactStatusReflectsDocumentAndZoomControlsPreserveCenter() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);
        const auto label=[&](const QString &name){auto *item=findVisualItem(main->contentItem(),name);return item?item->property("text").toString():QString();};
        QTRY_COMPARE(label("statusTool"),QStringLiteral("工具  画笔工具"));client.setEraser(true);QTRY_COMPARE(label("statusTool"),QStringLiteral("工具  橡皮擦工具"));client.brushLibrary()->select("round-fine");QTRY_COMPARE(label("statusBrush"),QStringLiteral("画笔  细线圆笔"));QVERIFY(client.eraser());
        client.setMoveTool(true);QTRY_COMPARE(label("statusTool"),QStringLiteral("工具  移动工具"));client.setSelectionTool(2);QTRY_COMPARE(label("statusTool"),QStringLiteral("工具  椭圆选框"));client.setSelectionTool(0);client.setMoveTool(false);
        QTRY_COMPARE(label("statusCanvas"),QStringLiteral("画布  960 × 640"));client.addDefaultLayer();QTRY_COMPARE(label("statusLayers"),QStringLiteral("图层  2"));QTRY_VERIFY(!client.layerEditBusy());
        auto *strip=findVisualItem(main->contentItem(),"toolStrip");QVERIFY(strip);for(auto *object:strip->findChildren<QObject*>())QVERIFY(object->property("tooltip").toString()!=QStringLiteral("适合窗口 F"));
        const auto click=[&](const QString &name){auto *item=findVisualItem(main->contentItem(),name);if(!item)return false;QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,item->mapToScene({item->width()/2,item->height()/2}).toPoint());return true;};
        canvas->actualSize();const QPointF center(canvas->width()/2,canvas->height()/2);const auto documentPoint=canvas->documentPoint(center);const int undo=client.undoDepth();
        QVERIFY(click("statusZoomIn"));QTRY_VERIFY(std::abs(canvas->zoom()-1.15)<.0001);QVERIFY(QLineF(documentPoint,canvas->documentPoint(center)).length()<.001);QTRY_COMPARE(label("statusZoom"),QString("115.0%"));
        QVERIFY(click("statusZoomOut"));QTRY_VERIFY(std::abs(canvas->zoom()-1)<.0001);QVERIFY(QLineF(documentPoint,canvas->documentPoint(center)).length()<.001);QCOMPARE(client.undoDepth(),undo);
        canvas->zoomBy(std::numeric_limits<double>::quiet_NaN());canvas->zoomBy(-1);QCOMPARE(canvas->zoom(),qreal(1));canvas->zoomBy(100);QCOMPARE(canvas->zoom(),qreal(8));QVERIFY(click("statusFit"));QTRY_VERIFY(canvas->zoom()<8);QCOMPARE(client.undoDepth(),undo);
        auto *documents=main->findChild<DocumentManager*>("documentManager");QVERIFY(documents);const auto first=documents->activeId();const auto second=documents->newDocument(128,256,true);QVERIFY(!second.isEmpty());QTRY_VERIFY(documents->activeClient()->ready());QTRY_COMPARE(label("statusCanvas"),QStringLiteral("画布  128 × 256"));QTRY_COMPARE(label("statusLayers"),QStringLiteral("图层  1"));
        QVERIFY(documents->activate(first));QTRY_COMPARE(label("statusCanvas"),QStringLiteral("画布  960 × 640"));QTRY_COMPARE(label("statusLayers"),QStringLiteral("图层  2"));QVERIFY(documents->closeDocument(first,true));QVERIFY(documents->closeDocument(second,true));
        QTRY_COMPARE(label("statusCanvas"),QStringLiteral("画布  —"));QTRY_VERIFY(!findVisualItem(main->contentItem(),"statusFit")->isEnabled());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void panelsReturnToCanvasOnlyAndEmptyWorkspaceEdges() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());auto *documents=main->findChild<DocumentManager*>("documentManager");QVERIFY(documents);
        const auto source=workspace.leftGroups().first().toMap().value("id").toString();const auto groups=workspace.leftGroups()+workspace.rightGroups();for(const auto &v:groups)workspace.detachGroup(v.toMap().value("id").toString());workspace.floatToolStrip(main->x()+20,main->y()+90);
        auto *area=findVisualItem(main->contentItem(),"mainDockWorkspace");QVERIFY(area);QTRY_VERIFY(area->width()>900);
        for(const auto &side:QStringList{"left","right"}) {
            if(side=="right") {QVERIFY(documents->closeDocument(documents->activeId(),true));QCOMPARE(documents->documents().size(),0);workspace.detachGroup(source);}
            QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+source)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());
            const auto grip=QPoint(24,4);QTest::mousePress(floating,Qt::LeftButton,Qt::NoModifier,grip);workspace.beginDrag(source,"",true);QTRY_VERIFY(workspace.dragging());
            // Accept the window boundary and a small outside tolerance, not the canvas body.
            const auto global=area->mapToGlobal({side=="left"?-4:area->width()+4,100}).toPoint();QTest::mouseMove(floating,floating->mapFromGlobal(global));QTRY_COMPARE(workspace.dragTarget(),"__workspace_"+side);
            QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,floating->mapFromGlobal(global));QTRY_COMPARE(workspace.groupDefinition(source).value("host").toString(),QString("main"));
            bool canvasRoom=false;for(const auto &v:workspace.layoutItems("main",main->width(),600))if(v.toMap().value("id")=="__canvas")canvasRoom=v.toMap().value("rect").toRectF().width()>320;QVERIFY(canvasRoom);
        }
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void draggingPanelsSelectsNearestExposedEdgeAndShowsEdgeGlow() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        for(const auto &edge:QStringList{"left","right","before","after"}) {
            workspace.resetLayout();const auto source=workspace.leftGroups().first().toMap().value("id").toString(),target=workspace.rightGroups().first().toMap().value("id").toString();
            workspace.detachGroup(target);workspace.detachGroup(source);QTRY_COMPARE(applicationWindows().size(),3);
            QQuickWindow *receiver=nullptr,*moving=nullptr;for(auto *w:applicationWindows()) {
                if(w->objectName()=="floatingDock:"+source)moving=qobject_cast<QQuickWindow*>(w);
                if(w->objectName()=="floatingDock:"+target)receiver=qobject_cast<QQuickWindow*>(w);
            }
            QVERIFY(receiver && moving);receiver->setPosition(main->position()+QPoint(380,160));receiver->raise();
            auto *tile=findVisualItem(receiver->contentItem(),"dockTile:"+target);QVERIFY(tile);
            const QRectF area(tile->mapToGlobal({0,0}),QSizeF(tile->width(),tile->height()));
            const QPoint grip(24,4);QTest::mousePress(moving,Qt::LeftButton,Qt::NoModifier,grip);workspace.beginDrag(source,"",true);QTRY_VERIFY(workspace.dragging());
            const auto body=area.center().toPoint();QTest::mouseMove(moving,moving->mapFromGlobal(body));QVERIFY(workspace.dragTarget().isEmpty());
            if(edge=="left") {
                const auto offset=body-moving->position();
                const QPoint nearWindowEdge(qRound(area.left())-moving->width()+offset.x(),qRound(area.center().y())+offset.y());
                QTest::mouseMove(moving,moving->mapFromGlobal(nearWindowEdge));
                QVERIFY(std::abs(moving->geometry().right()+1-area.left())<=1);
                QVERIFY(std::abs(nearWindowEdge.x()-area.left())>24);
                QVERIFY(workspace.dragTarget()!=target);
            }
            QPointF at=area.center();if(edge=="left")at.setX(area.left()-4);else if(edge=="right")at.setX(area.right()+4);else if(edge=="before")at.setY(area.top()-4);else at.setY(area.bottom()+4);
            QTest::mouseMove(moving,moving->mapFromGlobal(at.toPoint()));QTRY_COMPARE(workspace.dragTarget(),target);QCOMPARE(workspace.dragPlacement(),edge=="left" || edge=="right"?"column-"+edge:edge);
            auto *glow=findVisualItem(receiver->contentItem(),edge=="left" || edge=="right"?"dockColumnPreview:"+target:"dockPreview:"+target);QVERIFY(glow);QVERIFY(glow->isVisible());QCOMPARE(glow->property("mode").toString(),edge);
            QQuickWindow *outsideHint=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="dockingHintWindow")outsideHint=qobject_cast<QQuickWindow*>(w);return outsideHint && outsideHint->isVisible();}());
            const auto bounds=outsideHint->geometry();if(edge=="left")QVERIFY(bounds.right()+1<=std::ceil(area.left()));else if(edge=="right")QVERIFY(bounds.left()>=std::floor(area.right()));else if(edge=="before")QVERIFY(bounds.bottom()+1<=std::ceil(area.top()));else QVERIFY(bounds.top()>=std::floor(area.bottom()));QVERIFY(outsideHint->flags().testFlag(Qt::WindowTransparentForInput));
            const auto preview=qEnvironmentVariable("DRAWVERSE_EDGE_PREVIEW");if(!preview.isEmpty() && edge=="right"){QTest::qWait(80);QVERIFY(receiver->grabWindow().save(preview));}
            QTest::mouseRelease(moving,Qt::LeftButton,Qt::NoModifier,moving->mapFromGlobal(at.toPoint()));QTRY_VERIFY(!workspace.dragging());QTRY_COMPARE(workspace.groupDefinition(source).value("host").toString(),target);
            QTRY_COMPARE(applicationWindows().size(),2);
        }
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void panelTabsSwapWithinTheirBarBeforeDetaching() {
        QTemporaryDir temp;const auto path=temp.filePath("layout.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());const auto group=workspace.rightGroups().last().toMap().value("id").toString();
        const auto swap=[&](QQuickWindow *window,const QString &first,const QString &second) {
            auto *source=findVisualItem(window->contentItem(),"panelTabGrip:"+first),*target=findVisualItem(window->contentItem(),"panelTabGrip:"+second);if(!source || !target)return false;
            const auto from=source->mapToScene({source->width()/2,source->height()/2}).toPoint(),to=target->mapToScene({target->width()/2,target->height()/2}).toPoint();
            const auto original=workspace.groupDefinition(group).value("panels").toStringList();
            QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,from);QTest::mouseMove(window,to);
            if(workspace.dragging() || workspace.groupDefinition(group).value("panels").toStringList()!=original)return false;
            QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,to);return true;
        };
        QVERIFY(swap(main,"layers","navigator"));QTRY_COMPARE(workspace.groupDefinition(group).value("panels").toStringList(),(QStringList{"navigator","history","layers"}));QCOMPARE(workspace.groupDefinition(group).value("active").toString(),QString("layers"));QVERIFY(workspace.floatingWindows().isEmpty());
        workspace.detachGroup(group);QQuickWindow *floating=nullptr;
        QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+group)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());
        QVERIFY(swap(floating,"history","layers"));QTRY_COMPARE(workspace.groupDefinition(group).value("panels").toStringList(),(QStringList{"navigator","layers","history"}));
        workspace.returnGroup(group);QTRY_VERIFY(workspace.floatingWindows().isEmpty());workspace.setColumnCollapsed(group,true);
        auto *icon=findVisualItem(main->contentItem(),"railPanel:history");QVERIFY(icon);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());
        QQuickWindow *flyout=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="panelFlyout:"+group+":history")flyout=qobject_cast<QQuickWindow*>(w);return flyout!=nullptr;}());
        QPointer<QQuickWindow> retained=flyout;QTRY_VERIFY(findVisualItem(flyout->contentItem(),"historyEntry:0"));QVERIFY(findVisualItem(flyout->contentItem(),"panelTabGrip:layers"));
        QVERIFY(swap(flyout,"layers","navigator"));QTRY_COMPARE(workspace.groupDefinition(group).value("panels").toStringList(),(QStringList{"layers","navigator","history"}));
        QVERIFY(swap(flyout,"navigator","layers"));workspace.setActive(group,"history");
        QCOMPARE(workspace.groupDefinition(group).value("active").toString(),QString("history"));QVERIFY(!workspace.dragging());
        workspace.saveLayout();WorkspaceManager restored(path);QCOMPARE(restored.groupDefinition(group).value("panels"),workspace.groupDefinition(group).value("panels"));
        auto *grip=findVisualItem(flyout->contentItem(),"panelTabGrip:history");QVERIFY(grip);const auto at=grip->mapToScene({grip->width()/2,grip->height()/2}).toPoint();
        QTest::mousePress(flyout,Qt::LeftButton,Qt::NoModifier,at);QTest::mouseMove(flyout,at+QPoint(0,60));QTRY_VERIFY(workspace.dragging());QTRY_VERIFY(retained.isNull());
        QQuickWindow *detached=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName().startsWith("floatingDock:"))detached=qobject_cast<QQuickWindow*>(w);return detached!=nullptr;}());
        QTest::keyClick(detached,Qt::Key_Escape);QTRY_VERIFY(!workspace.dragging());QTRY_COMPARE(workspace.groupDefinition(group).value("panels").toStringList(),(QStringList{"navigator","layers","history"}));QTest::mouseRelease(main,Qt::LeftButton,Qt::NoModifier,{500,200});
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void closingOlderPopupDoesNotUntrackNewPopup() {
        QTemporaryDir temp; WorkspaceManager workspace(temp.filePath("layout.ini"));
        QWindow older,newer; older.setGeometry(10,10,60,60); newer.setGeometry(200,10,60,60);
        older.show(); newer.show(); QTRY_VERIFY(older.isVisible()); QTRY_VERIFY(newer.isVisible());
        workspace.watchMenuWindow(&older,true);
        workspace.watchMenuWindow(&newer,true);
        workspace.watchMenuWindow(&older,false);
        QTest::mouseClick(&older,Qt::LeftButton,Qt::NoModifier,QPoint(5,5));
        QTRY_VERIFY(!newer.isVisible()); QVERIFY(older.isVisible());
        workspace.watchMenuWindow(&newer,false); older.close();
    }
    void menuSurfaceKeepsOnlyTopCornersRoundAndFlatTintAt50Percent() {
        MenuSurface surface; surface.setWidth(60); surface.setHeight(28);
        QColor tint("#202226"); tint.setAlphaF(.5); surface.setTint(tint); surface.setTopCornersOnly(true);
        QImage image(60,28,QImage::Format_ARGB32_Premultiplied); image.fill(Qt::transparent);
        QPainter painter(&image); surface.paint(&painter); painter.end();
        QCOMPARE(image.pixelColor(0,0).alpha(),0); QCOMPARE(image.pixelColor(59,0).alpha(),0);
        QVERIFY(std::abs(image.pixelColor(0,27).alphaF()-.5)<.005);
        QVERIFY(std::abs(image.pixelColor(59,27).alphaF()-.5)<.005);
        QCOMPARE(image.pixelColor(30,2),image.pixelColor(30,25));
        surface.setTopCornersOnly(false); image.fill(Qt::transparent); painter.begin(&image); surface.paint(&painter); painter.end();
        QCOMPARE(image.pixelColor(0,27).alpha(),0);
        surface.setRadius(0); painter.begin(&image); surface.paint(&painter); painter.end();
        QVERIFY(image.pixelColor(0,0).alpha()>0);
        surface.setRadius(10); painter.begin(&image); surface.paint(&painter); painter.end();
        QCOMPARE(image.pixelColor(0,0).alpha(),0); // Repaint clears former square corners.
        surface.setTopRightCornerOnly(true); surface.setTint(QColor("#d8323e"));
        painter.begin(&image); surface.paint(&painter); painter.end();
        QCOMPARE(image.pixelColor(0,0),QColor("#d8323e"));
        QCOMPARE(image.pixelColor(59,0).alpha(),0);
        QCOMPARE(image.pixelColor(0,27),QColor("#d8323e"));
        QCOMPARE(image.pixelColor(59,27),QColor("#d8323e"));
    }
    void menusReopenWithoutGhostsAndFloatBesidePanels() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini")); QQmlApplicationEngine engine;
        QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &e:errors) warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/Main.qml")); QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window); QTRY_VERIFY(client.ready());
        // Native mouse moves use the desktop cursor: keep the title buttons on
        // the available screen even when the default window is wider at high DPI.
        if(QGuiApplication::platformName()!=QStringLiteral("offscreen")) {
            const auto available=window->screen()->availableGeometry();
            window->resize(std::min(window->width(),available.width()),std::min(window->height(),available.height()));
            window->setPosition(available.topLeft());
            QTest::qWait(100);
        }
        auto *bar=qobject_cast<MenuSurface*>(findVisualItem(window->contentItem(),"menuBarGlassBackground"));
        QVERIFY(bar); QVERIFY(bar->tint().alpha()>0 && bar->tint().alpha()<255); QCOMPARE(bar->radius(),qreal(10));
        QCOMPARE(bar->tint().name(),QString("#05080a")); QVERIFY(std::abs(bar->tint().alphaF()-.75)<.005); QVERIFY(bar->topCornersOnly());
        const auto barPixel=[&] {
            const auto image=window->grabWindow();
            const auto point=bar->mapToScene({bar->width()/2,bar->height()/2})*window->devicePixelRatio();
            return image.pixelColor(point.toPoint());
        };
        auto *body=findVisualItem(window->contentItem(),"opaqueWorkspaceBackground"); QVERIFY(body);
        QCOMPARE(body->mapToScene({0,0}).y(),bar->height());
        if(window->grabWindow().hasAlphaChannel()) QTRY_COMPARE(barPixel().alpha(),191);
        else QTRY_COMPARE(barPixel(),QColor(4,6,7)); // Software grab is RGB32 on a black clear surface.
        QCOMPARE(window->grabWindow().pixelColor(qRound(window->width()*window->devicePixelRatio()/2),
                 qRound(60*window->devicePixelRatio())).alpha(),255);
        auto *closeButton=findVisualItem(window->contentItem(),"windowClose"); QVERIFY(closeButton);
        auto *closeBackground=qobject_cast<MenuSurface*>(closeButton->property("background").value<QObject*>()); QVERIFY(closeBackground);
        QCOMPARE(closeBackground->radius(),qreal(10)); QVERIFY(closeBackground->topRightCornerOnly());
        QTest::mouseMove(window,QPoint(500,100));
        QTRY_COMPARE(closeBackground->tint(),QColor(Qt::transparent));
        QCOMPARE(closeButton->property("contentItem").value<QObject*>()->property("color").value<QColor>(),QColor(Qt::white));
        const auto closeCenter=closeButton->mapToScene({closeButton->width()/2,closeButton->height()/2}).toPoint();
        window->raise();window->requestActivate();QTRY_VERIFY(window->isActive());
        QTest::mouseMove(window,closeCenter);
        QTRY_VERIFY(closeButton->property("hovered").toBool()); QTRY_COMPARE(closeBackground->tint(),QColor("#f04450"));
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,closeCenter);
        QTRY_COMPARE(closeBackground->tint(),QColor("#b5222c"));
        QTest::mouseMove(window,QPoint(500,50));
        QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,QPoint(500,50));
        QTRY_COMPARE(closeBackground->tint(),QColor(Qt::transparent)); QVERIFY(window->isVisible());
        auto *entry=findVisualItem(window->contentItem(),QStringLiteral("menuEntry:窗口")); QVERIFY(entry);
        auto *focus=qobject_cast<QQuickItem*>(entry->property("background").value<QObject*>()); QVERIFY(focus);
        QCOMPARE(focus->property("radius").toReal(),qreal(0));
        auto *menu=window->findChild<QObject*>("windowMenu"); QVERIFY(menu);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,entry->mapToScene({entry->width()/2,entry->height()/2}).toPoint());
        QTRY_VERIFY(menu->property("opened").toBool());
        QCOMPARE(visibleWindowsCount(),2);
        auto *glass=qobject_cast<MenuSurface*>(menu->property("background").value<QObject*>()); QVERIFY(glass);
        QCOMPARE(glass->radius(),qreal(0)); QCOMPARE(glass->tint().alpha(),255);
        QCOMPARE(glass->tint().name(),QString("#1c1e21")); QVERIFY(!glass->topCornersOnly());
        auto *list=qobject_cast<QQuickItem*>(menu->property("contentItem").value<QObject*>()); QVERIFY(list);
        auto *row=findVisualItem(list,QStringLiteral("glassMenuItem:自定义面板…")); QVERIFY(row);
        auto *label=qobject_cast<QQuickItem*>(row->property("contentItem").value<QObject*>()); QVERIFY(label);
        QCOMPARE(row->property("topPadding").toReal(),qreal(3)); QCOMPARE(row->property("bottomPadding").toReal(),qreal(3));
        QCOMPARE(row->implicitHeight()-label->implicitHeight(),qreal(6));
        QCOMPARE(row->property("background").value<QObject*>()->property("radius").toReal(),qreal(0));
        const auto preview=qEnvironmentVariable("DRAWVERSE_MENU_PREVIEW");
        if(!preview.isEmpty()) { QTest::qWait(100); QVERIFY(window->grabWindow().save(preview)); }
        QTest::keyClick(list->window(),Qt::Key_Escape); QTRY_VERIFY(!menu->property("visible").toBool());
        const auto menuImage=[&] {
            const auto image=list->window()->grabWindow();
            const auto position=glass->mapToScene({0,0}); const auto dpr=list->window()->devicePixelRatio();
            // Exclude the fractional outer pixel coverage at 1.1x scale. All
            // interior text/background pixels must stay identical over changing content.
            const int fringe=qEnvironmentVariable("QT_QUICK_BACKEND")==QStringLiteral("software") && std::fmod(dpr,1)!=0?static_cast<int>(std::ceil(dpr)):1;
            return image.copy(QRect(qRound(position.x()*dpr),qRound(position.y()*dpr),qRound(glass->width()*dpr),qRound(glass->height()*dpr)).adjusted(1,1,-fringe,-fringe));
        };
        QTest::mouseMove(window,{20,window->height()-20});
        QVERIFY(QMetaObject::invokeMethod(menu,"open")); QTRY_VERIFY(menu->property("opened").toBool());
        QTest::qWait(30); const auto firstMenu=menuImage(); QVERIFY(!firstMenu.isNull());
        QCOMPARE(firstMenu.pixelColor(0,0).alpha(),255);
        QCOMPARE(firstMenu.pixelColor(firstMenu.width()-1,firstMenu.height()-1).alpha(),255);
        auto *options=findVisualItem(window->contentItem(),"brushOptionsBar"); QVERIFY(options);
        options->setVisible(false);QTRY_COMPARE(menuImage(),firstMenu);
        options->setVisible(true);QTRY_COMPARE(menuImage(),firstMenu);
        for(int cycle=0;cycle<6;++cycle) {
            QVERIFY(QMetaObject::invokeMethod(menu,"close")); QTRY_VERIFY(!menu->property("visible").toBool());
            QVERIFY(QMetaObject::invokeMethod(menu,"open")); QTRY_VERIFY(menu->property("opened").toBool());
            QTRY_COMPARE(menuImage(),firstMenu); QCOMPARE(bar->radius(),qreal(10));
        }
        QVERIFY(QMetaObject::invokeMethod(menu,"close")); QTRY_VERIFY(!menu->property("visible").toBool());
        const auto normalGeometry=window->geometry();
        window->showMaximized(); QTRY_COMPARE(bar->radius(),qreal(0));
        QCOMPARE(closeBackground->radius(),qreal(0));
        QTRY_COMPARE(window->geometry(),window->screen()->availableGeometry());
        window->showNormal(); QTRY_COMPARE(bar->radius(),qreal(10));
        QCOMPARE(closeBackground->radius(),qreal(10));
        QTRY_COMPARE(window->geometry(),normalGeometry);
        window->raise();window->requestActivate();QTRY_VERIFY(window->isActive());
        auto *windowEntry=findVisualItem(window->contentItem(),QStringLiteral("menuEntry:窗口")); QVERIFY(windowEntry);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,windowEntry->mapToScene({windowEntry->width()/2,windowEntry->height()/2}).toPoint());
        auto *windowMenu=window->findChild<QObject*>("windowMenu"); QVERIFY(windowMenu); QTRY_VERIFY(windowMenu->property("opened").toBool());
        auto *panelItem=windowMenu->findChild<QObject*>("windowPanel:color"); QVERIFY(panelItem); QVERIFY(panelItem->property("checked").toBool());
        const auto menuPreview=qEnvironmentVariable("DRAWVERSE_WINDOW_MENU_PREVIEW");
        if(!menuPreview.isEmpty()) {
            QTest::qWait(120);
            auto *menuGhost=qobject_cast<QQuickItem*>(windowMenu->property("contentItem").value<QObject*>());
            QVERIFY(menuGhost && menuGhost->window());
            QVERIFY(menuGhost->window()->grabWindow().save(menuPreview));
        }
        // The offscreen backend cannot grab the keyboard, so keyboard Escape is unreliable here;
        // close the top-level menu explicitly and verify it really goes away.
        QVERIFY(QMetaObject::invokeMethod(windowMenu,"close"));QTRY_VERIFY(!windowMenu->property("visible").toBool());
        const auto group=workspace.rightGroups().first().toMap().value("id").toString(); workspace.detachPanel(group,"color");
        QTRY_COMPARE(visibleWindowsCount(),2);
        QQuickWindow *floating=nullptr; for(auto *candidate:applicationWindows()) if(candidate->isVisible() && candidate->objectName().startsWith("floatingDock:")) floating=qobject_cast<QQuickWindow*>(candidate);
        QVERIFY(floating); const auto floatingId=workspace.floatingGroups().first().toMap().value("id").toString();
        floating->resize(150,300); floating->setPosition(floating->screen()->availableGeometry().topLeft()+QPoint(10,10));
        auto *button=findVisualItem(floating->contentItem(),"panelMenu:"+floatingId); QVERIFY(button);
        // Synthetic clicks do not activate a native Qt Tool window like real clicks do.
        floating->requestActivate(); QTRY_VERIFY(floating->isActive());
        QTRY_VERIFY(button->mapToScene({button->width()/2,button->height()/2}).x()<floating->width());
        QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,button->mapToScene({button->width()/2,button->height()/2}).toPoint());
        auto *panelMenu=floating->findChild<QObject*>("panelOperationsMenu:"+floatingId); QVERIFY(panelMenu); QTRY_VERIFY(panelMenu->property("opened").toBool());
        QTRY_COMPARE(visibleWindowsCount(),3);
        auto *host=qobject_cast<QQuickWindow*>(panelMenu->property("sideWindow").value<QObject*>()); QVERIFY(host);
        QTRY_VERIFY(host->isVisible());
        QVERIFY(host->x()>=floating->x()+floating->width() || host->x()+host->width()<=floating->x());
        QTRY_VERIFY(host->isActive());
        QTRY_VERIFY(panelMenu->property("activeFocus").toBool());
        auto *floatingGlass=qobject_cast<MenuSurface*>(panelMenu->property("background").value<QObject*>()); QVERIFY(floatingGlass);
        QCOMPARE(floatingGlass->radius(),qreal(0)); QCOMPARE(floatingGlass->tint().alpha(),255);
        QVERIFY(panelMenu->setProperty("currentIndex",-1));
        const auto sideFrame=[](QQuickWindow *popup) {
            auto frame=popup->grabWindow();
            // Qt's software readback at fractional DPR has an unpainted
            // fringe of up to one logical pixel, also with native RGBA windows.
            if(qEnvironmentVariable("QT_QUICK_BACKEND")==QStringLiteral("software") && std::fmod(popup->devicePixelRatio(),1)!=0) {
                const int fringe=static_cast<int>(std::ceil(popup->devicePixelRatio()));
                frame=frame.copy(0,0,frame.width()-fringe,frame.height()-fringe);
            }
            return frame;
        };
        QTest::qWait(100); const auto firstSide=sideFrame(host); QVERIFY(!firstSide.isNull());
        QCOMPARE(firstSide.pixelColor(0,0).alpha(),255);
        QCOMPARE(firstSide.pixelColor(firstSide.width()-1,firstSide.height()-1).alpha(),255);
        if(!preview.isEmpty()) { QTest::qWait(30); QVERIFY(host->grabWindow().save(preview+".side.png")); }
        QTest::keyClick(host,Qt::Key_Down); QTest::keyClick(host,Qt::Key_Escape);
        QTRY_VERIFY(!panelMenu->property("visible").toBool());
        QTRY_COMPARE(visibleWindowsCount(),2);
        for(int cycle=0;cycle<3;++cycle) {
            QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,button->mapToScene({button->width()/2,button->height()/2}).toPoint());
            QTRY_VERIFY(panelMenu->property("opened").toBool());
            host=qobject_cast<QQuickWindow*>(panelMenu->property("sideWindow").value<QObject*>()); QVERIFY(host);
            QVERIFY(panelMenu->setProperty("currentIndex",-1));
            if(!preview.isEmpty()) {
                QVERIFY(firstSide.save(preview+".side-expected.png"));
                QVERIFY(sideFrame(host).save(preview+".side-reopened.png"));
            }
            const bool sameFrame=QTest::qWaitFor([&]{return sideFrame(host)==firstSide;},5000);
            if(!sameFrame) {
                firstSide.save("menu-side-expected.png"); sideFrame(host).save("menu-side-reopened.png");
                qInfo()<<"Side dimensions"<<firstSide.size()<<sideFrame(host).size();
            }
            QVERIFY(sameFrame);
            // Both owner-panel space and the main canvas dismiss the popup.
            auto *outside=cycle%2==0?floating:window;
            const QPoint blank=outside==floating?QPoint(5,250):QPoint(500,500);
            QTest::mouseClick(outside,Qt::LeftButton,Qt::NoModifier,blank);
            QTRY_VERIFY(!panelMenu->property("visible").toBool());
            QTRY_COMPARE(visibleWindowsCount(),2);
            QCOMPARE(client.modified(),false);
        }
        floating->setX(floating->screen()->availableGeometry().right()-floating->width());
        QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,button->mapToScene({button->width()/2,button->height()/2}).toPoint());
        QTRY_VERIFY(panelMenu->property("opened").toBool());
        host=qobject_cast<QQuickWindow*>(panelMenu->property("sideWindow").value<QObject*>()); QVERIFY(host);
        QVERIFY(host->x()+host->width()<=floating->x());
        auto *returnItem=findVisualItem(host->contentItem(),QStringLiteral("glassMenuItem:返回工作区")); QVERIFY(returnItem);
        QTest::mouseClick(host,Qt::LeftButton,Qt::NoModifier,returnItem->mapToScene({returnItem->width()/2,returnItem->height()/2}).toPoint());
        QTRY_COMPARE(workspace.floatingGroups().size(),0); QTRY_COMPARE(visibleWindowsCount(),1);
        QCOMPARE(warnings,QStringList()); QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void nativeMenuBarBlursLiveBackdropWithoutMenuSnapshots() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini")); QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/Main.qml")); QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window); QTRY_VERIFY(client.ready());
        if(QGuiApplication::platformName()!=QStringLiteral("windows")) {
            QVERIFY(!window->property("menuBarBlurActive").toBool());
            QVERIFY(!workspace.setMenuBarBlur(window,true));
        } else {
            QVERIFY(window->property("menuBarBlurActive").toBool());
            const auto available=window->screen()->availableGeometry();
            window->resize(std::min(window->width(),available.width()),std::min(window->height(),available.height()));
            window->setPosition(available.topLeft());QTest::qWait(100);
#ifdef Q_OS_WIN
            const auto hasRoundedNativeCorners=[&] {
                const auto handle=reinterpret_cast<HWND>(window->winId());
                RECT bounds{}; if(!GetWindowRect(handle,&bounds)) return false;
                const auto region=CreateRectRgn(0,0,0,0);
                const int radius=qRound(10*window->devicePixelRatio());
                const bool rounded=GetWindowRgn(handle,region)==COMPLEXREGION
                    && !PtInRegion(region,0,0) && !PtInRegion(region,bounds.right-bounds.left-1,0)
                    && !PtInRegion(region,radius/2,0) && PtInRegion(region,radius,0)
                    && PtInRegion(region,(bounds.right-bounds.left)/2,0)
                    && PtInRegion(region,(bounds.right-bounds.left)/2,bounds.bottom-bounds.top-1);
                DeleteObject(region); return rounded;
            };
            QTRY_VERIFY(hasRoundedNativeCorners());
            const auto originalGeometry=window->geometry();
            window->resize(1000,650); QTRY_VERIFY(hasRoundedNativeCorners());
            QTRY_COMPARE(window->property("normalGeometry").toRect(),window->geometry());
            window->showMaximized(); QTRY_COMPARE(window->geometry(),window->screen()->availableGeometry());
            QTRY_VERIFY(!hasRoundedNativeCorners());
            window->showNormal(); QTRY_VERIFY(hasRoundedNativeCorners());
            window->setGeometry(originalGeometry); QTRY_VERIFY(hasRoundedNativeCorners());
            QTRY_COMPARE(window->property("normalGeometry").toRect(),originalGeometry);
#endif
            QQuickWindow backdrop;
            backdrop.setFlags(Qt::Window|Qt::FramelessWindowHint); backdrop.setColor(Qt::black);
            backdrop.setGeometry(window->geometry());
            BlurTestBackdrop stripes(backdrop.contentItem()); stripes.setSize(backdrop.size());
            backdrop.show(); backdrop.raise(); window->raise(); window->requestActivate(); QTRY_VERIFY(window->isActive());
#ifdef Q_OS_WIN
            // Desktop capture needs both test windows above other applications,
            // independently of Windows' foreground activation restrictions.
            SetWindowPos(reinterpret_cast<HWND>(backdrop.winId()),HWND_TOPMOST,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE);
            SetWindowPos(reinterpret_cast<HWND>(window->winId()),HWND_TOPMOST,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE);
#endif
            const auto capture=[&] {
                QTest::qWait(300);
                return window->screen()->grabWindow(0,window->x()+500,window->y()+8,320,12).toImage();
            };
            const auto sharpestEdge=[](const QImage &image) {
                int edge=0; const int row=image.height()/2;
                for(int x=1;x<image.width();++x) {
                    const auto a=image.pixelColor(x-1,row),b=image.pixelColor(x,row);
                    edge=std::max(edge,std::abs(a.red()-b.red())+std::abs(a.green()-b.green())+std::abs(a.blue()-b.blue()));
                }
                return edge;
            };
            QVERIFY(workspace.setMenuBarBlur(window,false)); const auto sharp=capture(); QVERIFY(!sharp.isNull());
            const auto sharpTitle=window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).toImage();
            QVERIFY(!sharpTitle.isNull());
            QVERIFY(workspace.setMenuBarBlur(window,true)); const auto blurred=capture(); QVERIFY(!blurred.isNull());
            const auto preview=qEnvironmentVariable("DRAWVERSE_MENU_PREVIEW");
            if(!preview.isEmpty()) {
                QVERIFY(sharp.save(preview+".sharp.png")); QVERIFY(blurred.save(preview+".blur.png"));
                QVERIFY(window->grabWindow().save(preview+".render.png"));
                QVERIFY(window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).save(preview+".title.png"));
            }
            const auto evidence=QString("Sharp edge=%1, blurred edge=%2").arg(sharpestEdge(sharp)).arg(sharpestEdge(blurred));
            QVERIFY2(sharpestEdge(sharp)>100,qPrintable(evidence));
            QVERIFY2(sharpestEdge(blurred)<sharpestEdge(sharp)/2,qPrintable(evidence));
#ifdef Q_OS_WIN
            // Verify actual compositor pixels after restore, not just the QML
            // surface: a leaked Windows iconic caption or square blur patch
            // can be invisible in QQuickWindow::grabWindow().
            const auto originalTitle=window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).toImage();
            if(!preview.isEmpty())QVERIFY(originalTitle.save(preview+".original.png"));
            const QRect captionArea(qRound(12*originalTitle.devicePixelRatio()),qRound(3*originalTitle.devicePixelRatio()),qRound(230*originalTitle.devicePixelRatio()),qRound(22*originalTitle.devicePixelRatio()));
            for(bool maximized:{false,true})for(int cycle=0;cycle<3;++cycle) {
                if(maximized)window->showMaximized();else window->showNormal();
                QTest::qWait(300);
                const auto before=window->geometry();
                const auto beforeTitle=window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).toImage();
                const auto expectedVisibility=maximized?QWindow::Maximized:QWindow::Windowed;
                SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_MINIMIZE,0);
                QTRY_COMPARE(window->visibility(),QWindow::Minimized);
                SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_RESTORE,0);
                QTRY_COMPARE(window->visibility(),expectedVisibility);QTRY_COMPARE(window->geometry(),before);
                QTRY_COMPARE(hasRoundedNativeCorners(),!maximized);window->raise();window->requestActivate();
                const auto restored=capture();QVERIFY(sharpestEdge(restored)<sharpestEdge(sharp)/2);
                const auto restoredTitle=window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).toImage();
                const auto corner=restoredTitle.pixelColor(0,0);if(!maximized)QVERIFY(corner.green()>190 && corner.red()<40);
                // The native caption occupies the left side. Its restored
                // pixels must match the original custom title/menu rendering.
                if(!preview.isEmpty())QVERIFY(restoredTitle.save(preview+".restored.png"));
                QVERIFY(restoredTitle.rect().contains(captionArea));
                for(int y=captionArea.top();y<=captionArea.bottom();++y)for(int x=captionArea.left();x<=captionArea.right();++x) {
                    const auto a=beforeTitle.pixelColor(x,y),b=restoredTitle.pixelColor(x,y);
                    // DWM may round a text blend by one channel level on restore.
                    // A system caption changes hundreds of pixels by much more.
                    QVERIFY2(std::abs(a.red()-b.red())<=2 && std::abs(a.green()-b.green())<=2 && std::abs(a.blue()-b.blue())<=2,qPrintable(QString("Restored caption pixel differs at %1,%2: %3 -> %4").arg(x).arg(y).arg(a.name()).arg(b.name())));
                }
            }
            window->showNormal();QTRY_COMPARE(window->geometry(),originalGeometry);QTRY_VERIFY(hasRoundedNativeCorners());QTest::qWait(300);
#endif
            auto *close=findVisualItem(window->contentItem(),"windowClose");QVERIFY(close);
            const auto closeCenter=close->mapToScene({close->width()/2,close->height()/2}).toPoint();
            QCursor::setPos(window->mapToGlobal(closeCenter));
            QTest::mouseMove(window,closeCenter);QTRY_VERIFY(close->property("hovered").toBool());
            QTest::qWait(100); // Let the compositor present the new hover color.
            const auto title=window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).toImage();
            QVERIFY(!title.isNull());
            // The excluded corner keeps the backdrop (including the system shadow).
            const auto corner=title.pixelColor(0,0),sourceCorner=sharpTitle.pixelColor(0,0);
            QVERIFY(std::abs(corner.hueF()-sourceCorner.hueF())<.005);
            QVERIFY(corner.valueF()>=sourceCorner.valueF()*.8 && corner.valueF()<=sourceCorner.valueF());
            QVERIFY(corner.green()>190 && corner.red()<40);
            const auto dpr=title.devicePixelRatio();
            const auto red=title.pixelColor(title.width()-qRound(5*dpr),qRound(20*dpr));
            if(!preview.isEmpty()) QVERIFY(title.save(preview+".title.png"));
            QVERIFY2(red.red()>180 && red.green()<90 && red.blue()<100,qPrintable(QString("Hover=%1, color=%2").arg(close->property("hovered").toBool()).arg(red.name())));
            // A pure-color corner cannot reveal a rectangular blur beneath a
            // transparent QML corner. Sharp checker edges outside the native
            // backdrop region must survive every state transition.
            stripes.showCornerPattern();window->hide();QTest::qWait(200);
            const auto cornerCapture=[&]{return window->screen()->grabWindow(0,window->x(),window->y(),5,2).toImage();};
            const auto sharpCorner=cornerCapture();QVERIFY(sharpestEdge(sharpCorner)>250);
            if(!preview.isEmpty())QVERIFY(sharpCorner.save(preview+".corner-source.png"));
            window->showNormal();QTest::qWait(200);
#ifdef Q_OS_WIN
            for(bool maximized:{false,true})for(int cycle=0;cycle<3;++cycle) {
                if(maximized)window->showMaximized();
                SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_MINIMIZE,0);
                QVERIFY(!IsWindowVisible(reinterpret_cast<HWND>(workspace.menuBlurWindowHandle())));
                SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_RESTORE,0);
                QTRY_COMPARE(window->visibility(),maximized?QWindow::Maximized:QWindow::Windowed);
                window->showNormal();QTRY_VERIFY(hasRoundedNativeCorners());QTest::qWait(200);
                const auto cornerImage=cornerCapture();
                if(!preview.isEmpty())QVERIFY(cornerImage.save(preview+".sharp-corner.png"));
                QVERIFY2(sharpestEdge(cornerImage)>sharpestEdge(sharpCorner)*.6,qPrintable(QString("Excluded-corner edge %1, source %2; maximized=%3, cycle=%4, geometry=%5,%6").arg(sharpestEdge(cornerImage)).arg(sharpestEdge(sharpCorner)).arg(maximized).arg(cycle).arg(window->x()).arg(window->y())));
            }
#endif
        }
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void uiScaleAppliesToMainPanelAndToolWindowsWithoutChangingLayout() {
        QTemporaryDir temp;
        PaintCoreClient client(nullptr,temp.filePath("storage.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini"));
        QQmlApplicationEngine engine;
        QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors) {
            for(const auto &error:errors) warnings.append(error.toString());
        });
        engine.rootContext()->setContextProperty("PaintClient",&client);
        engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/Main.qml"));
        QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());
        QVERIFY(window); QTRY_VERIFY(client.ready());
        auto *options=findVisualItem(window->contentItem(),"brushOptionsBar");
        QVERIFY(options); QCOMPARE(options->height(),qreal(36));
        const auto group=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachPanel(group,"color");
        workspace.floatToolStrip(window->x()+60,window->y()+80);
        QTRY_COMPARE(applicationWindows().size(),3);
        const qreal expected=qEnvironmentVariable("QT_SCALE_FACTOR").toDouble();
        QVERIFY(expected>=1.1);
        for(auto *nativeWindow:applicationWindows()) {
            auto *quick=qobject_cast<QQuickWindow*>(nativeWindow); QVERIFY(quick);
            if(qEnvironmentVariable("QT_QPA_PLATFORM")=="offscreen")
                QVERIFY(std::abs(quick->devicePixelRatio()-expected)<.000001);
            QCOMPARE(quick->devicePixelRatio(),window->devicePixelRatio());
            QTRY_VERIFY(!quick->grabWindow().isNull());
            const auto image=quick->grabWindow();
            QVERIFY(std::abs(image.width()-quick->width()*quick->devicePixelRatio())<=1);
            QVERIFY(std::abs(image.height()-quick->height()*quick->devicePixelRatio())<=1);
        }
        QCOMPARE(workspace.floatingGroups().first().toMap().value("panels").toStringList(),QStringList{"color"});
        workspace.saveLayout();
        WorkspaceManager restored(temp.filePath("layout.ini"));
        QCOMPARE(restored.floatingGroups(),workspace.floatingGroups());
        QVERIFY(restored.toolsFloating());
        QCOMPARE(warnings,QStringList());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);
        client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void selectionOutlineCombinesInDocumentCoordinatesWithoutRedrawOnIdenticalState() {
        SelectionOverlay item;item.setDocumentRect({20,30,200,200});item.setZoom(2);
        const auto step=[](int op,int shape,QRectF r){return QVariantMap{{"operation",op},{"shape",shape},{"x",r.x()},{"y",r.y()},{"width",r.width()},{"height",r.height()}};};
        const QVariantList steps{step(0,0,{10,10,50,50}),step(2,1,{20,20,20,20})};
        item.setSteps(steps);item.setEnabledSelection(true);QVERIFY(item.documentPath().contains({15,15}));QVERIFY(!item.documentPath().contains({30,30}));QVERIFY(!item.documentPath().contains({90,90}));
        QSignalSpy changed(&item,&SelectionOverlay::geometryChanged);item.setSteps(steps);item.setDocumentRect({20,30,200,200});item.setZoom(2);QCOMPARE(changed.size(),0);
        item.setSteps(steps+QVariantList{step(4,0,{})});QVERIFY(!item.documentPath().contains({15,15}));QVERIFY(item.documentPath().contains({30,30}));QVERIFY(item.documentPath().contains({90,90}));
        item.setDocumentRect({40,50,200,200});QVERIFY(item.documentPath().contains({90,90}));
        QImage image(260,260,QImage::Format_ARGB32_Premultiplied);image.fill(Qt::transparent);QPainter painter(&image);item.paint(&painter);painter.end();QVERIFY(image.pixelColor(40,100).alpha()>0);QCOMPARE(image.pixelColor(0,0).alpha(),0);
    }
    void shiftDragConstrainsSelectionToSquareAndCircle() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());
        auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();main->requestActivate();
        const auto at=[&](QPointF point){return canvas->mapToScene(canvas->documentRect().topLeft()+point*canvas->zoom()).toPoint();};
        // Unconstrained drag keeps the dragged proportions.
        client.setSelectionTool(1);QTRY_COMPARE(client.selectionTool(),1);
        QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,at({10,10}));QTest::mouseMove(main,at({70,40}));QTest::mouseRelease(main,Qt::LeftButton,Qt::NoModifier,at({70,40}));
        QTRY_COMPARE(client.selectionSteps().size(),1);
        auto step=client.selectionSteps().first().toMap();
        QCOMPARE(step.value("width").toDouble(),60.);QCOMPARE(step.value("height").toDouble(),30.);
        // Shift while dragging constrains to a square that keeps the anchor corner.
        const auto square=CanvasItem::constrainedRect({20,20},{80,50});
        QCOMPARE(square,QRectF(20,20,60,60));
        // Dragging up-left keeps the pressed corner instead of flipping the box.
        QCOMPARE(CanvasItem::constrainedRect({80,50},{20,20}),QRectF(20,-10,60,60));
        QCOMPARE(CanvasItem::constrainedRect({10,10},{40,90}),QRectF(10,10,80,80));
        QCOMPARE(CanvasItem::constrainedRect({-30,-30},{10,-60}),QRectF(-30,-70,40,40));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void layerSearchRefocusAfterToolsAndCanvasBlank() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("search.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);window->requestActivate();QTRY_VERIFY(window->isActive());QVERIFY(QTest::qWaitForWindowExposed(window));QTRY_VERIFY(client.ready());
        QQuickItem *search=nullptr;QTRY_VERIFY((search=findVisualItem(window->contentItem(),"layerSearch")));QTRY_VERIFY(search->width()>40);QTest::qWait(50);
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);auto *tool=findVisualItem(window->contentItem(),"eraserTool");QVERIFY(tool);
        const auto click=[&](QQuickItem *i){QTest::mouseClick(i->window(),Qt::LeftButton,Qt::NoModifier,i->mapToScene({i->width()/2,i->height()/2}).toPoint());};
        click(search);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(window,Qt::Key_A);QCOMPARE(search->property("text").toString(),QStringLiteral("a"));
        for(int i=0;i<3;++i){
            click(tool);QTRY_VERIFY(canvas->hasActiveFocus());QVERIFY(client.eraser());click(search);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(window,Qt::Key_B);QCOMPARE(search->property("text").toString(),QStringLiteral("a")+QString(2*i+1,QLatin1Char('b')));QVERIFY(client.eraser());
            QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,canvas->mapToScene({2,2}).toPoint());QTRY_VERIFY(canvas->hasActiveFocus());click(search);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(window,Qt::Key_B);QCOMPARE(search->property("text").toString(),QStringLiteral("a")+QString(2*i+2,QLatin1Char('b')));QVERIFY(client.eraser());
        }
        QCOMPARE(findVisualItem(window->contentItem(),"layerSearch"),search);
        // Icon-expanded panels are separate native windows: send keys to the actual focus window.
        workspace.setColumnCollapsed(workspace.groupForPanel("layers"),true);
        QQuickItem *rail=nullptr;QTRY_VERIFY((rail=findVisualItem(window->contentItem(),"railPanel:layers")));click(rail);
        QQuickWindow *flyout=nullptr;const auto findFlyout=[&](){for(auto *w:applicationWindows())if(w->objectName().startsWith("panelFlyout:") && w->objectName().endsWith(":layers") && w->isVisible())return qobject_cast<QQuickWindow*>(w);return static_cast<QQuickWindow*>(nullptr);};QTRY_VERIFY((flyout=findFlyout()));
        QTRY_VERIFY((search=findVisualItem(flyout->contentItem(),"layerSearch")));QTRY_VERIFY(search->width()>40);QVERIFY(QTest::qWaitForWindowExposed(flyout));QTest::qWait(50);click(search);QTRY_COMPARE(QGuiApplication::focusWindow(),flyout);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(QGuiApplication::focusWindow(),Qt::Key_A);QCOMPARE(search->property("text").toString(),QStringLiteral("a"));
        for(int i=0;i<3;++i){
            click(tool);QTRY_COMPARE(QGuiApplication::focusWindow(),window);QTRY_VERIFY(canvas->hasActiveFocus());click(search);QTRY_COMPARE(QGuiApplication::focusWindow(),flyout);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(QGuiApplication::focusWindow(),Qt::Key_B);QCOMPARE(search->property("text").toString(),QStringLiteral("a")+QString(2*i+1,QLatin1Char('b')));QVERIFY(client.eraser());
            window->requestActivate();QTRY_VERIFY(window->isActive());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,canvas->mapToScene({2,2}).toPoint());QTRY_VERIFY(canvas->hasActiveFocus());click(search);QTRY_COMPARE(QGuiApplication::focusWindow(),flyout);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(QGuiApplication::focusWindow(),Qt::Key_B);QCOMPARE(search->property("text").toString(),QStringLiteral("a")+QString(2*i+2,QLatin1Char('b')));QVERIFY(client.eraser());QVERIFY(flyout->isVisible());
        }
        QCOMPARE(findVisualItem(flyout->contentItem(),"layerSearch"),search);QCOMPARE(warnings,QStringList());auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void firstLayerSearchClickAndCtrlCanvasPickMove() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("pick.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);window->requestActivate();QTRY_VERIFY(window->isActive());QTRY_VERIFY(client.ready());
        QQuickItem *search=nullptr;QTRY_VERIFY((search=findVisualItem(window->contentItem(),"layerSearch")));QTRY_VERIFY(search->width()>40 && search->height()>0);QVERIFY(QTest::qWaitForWindowExposed(window));QTest::qWait(50);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,search->mapToScene({search->width()/2,search->height()/2}).toPoint());QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(window,Qt::Key_A);QCOMPARE(search->property("text").toString(),QStringLiteral("a"));QTest::keyClick(window,Qt::Key_Escape);QCOMPARE(search->property("text").toString(),QString());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());const auto bottom=client.activeLayer();client.setBrushRadius(4);QVERIFY(client.beginStroke({{16,16},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());client.addLayer("top");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto top=client.activeLayer();QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto screen=[&](QPointF pt){return canvas->mapToScene(canvas->documentRect().topLeft()+pt*canvas->zoom()).toPoint();};
        QTRY_COMPARE(client.undoDepth(),3);const auto depth=client.undoDepth();client.activateEraser();QTest::mouseClick(window,Qt::LeftButton,Qt::ControlModifier,screen({16,16}));QTRY_COMPARE(client.activeLayer(),bottom);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth);QVERIFY(client.eraser());QVERIFY(!client.moveTool());
        QTest::mousePress(window,Qt::LeftButton,Qt::ControlModifier,screen({32,32}));QTest::mouseMove(window,screen({38,35}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,screen({38,35}));QTRY_COMPARE(client.activeLayer(),top);QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.eraser());
        const auto offset=[&](quint64 id){for(const auto &v:client.layers()){const auto l=v.toMap();if(l.value("id").toULongLong()==id)return QPoint(l.value("offsetX").toInt(),l.value("offsetY").toInt());}return QPoint(-999,-999);};
        QCOMPARE(offset(top),QPoint(6,3));QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier);QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(offset(top),QPoint(0,0));QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(offset(top),QPoint(6,3));QTRY_VERIFY(!client.layerEditBusy());
        QTest::mousePress(window,Qt::LeftButton,Qt::ControlModifier,screen({60,60}));QTest::mouseRelease(window,Qt::LeftButton,Qt::ControlModifier,screen({55,55}));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);QCOMPARE(offset(top),QPoint(6,3));
        client.activateMarquee();QTest::mouseClick(window,Qt::LeftButton,Qt::ControlModifier,screen({16,16}));QTRY_COMPARE(client.activeLayer(),bottom);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionTool(),1);QVERIFY(!client.selectionEnabled());
        QCOMPARE(warnings,QStringList());auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void placedImageFreeTransformRasterizeAndShortcuts() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("transform.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QStringList warnings;QQmlApplicationEngine engine;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());QImage image(16,16,QImage::Format_RGBA8888);image.fill(Qt::red);image.setPixelColor(0,0,Qt::blue);const auto path=temp.filePath("placed.png");QVERIFY(image.save(path));QVERIFY(client.placeImage(QUrl::fromLocalFile(path)));QTRY_VERIFY(!client.fileBusy() && client.transformActive());QTRY_VERIFY(!client.transformBusy());QVERIFY(client.activeImage());QCOMPARE(client.undoDepth(),0);auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();window->requestActivate();QTRY_VERIFY(window->isActive());canvas->forceActiveFocus();QTRY_VERIFY(client.frameRevision()>=client.revision());
        auto *bar=findVisualItem(window->contentItem(),"transformOptionsBar");QVERIFY(bar);QVERIFY(bar->isVisible());auto *overlay=findVisualItem(window->contentItem(),"transformOverlay");QVERIFY(overlay && overlay->isVisible());const auto p=client.transformPoints();QCOMPARE(p.size(),16);const auto screen=[&](QPointF pt){return canvas->mapToScene(canvas->documentRect().topLeft()+pt*canvas->zoom()).toPoint();};
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,screen(p[15].toPointF()));QTest::mouseMove(window,screen(p[15].toPointF()+QPointF(8,8)));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,screen(p[15].toPointF()+QPointF(8,8)));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.transformBusy());QVERIFY(client.transformPoints()!=p);canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.transformBusy());QCOMPARE(client.transformPoints(),p);QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.transformBusy());
        auto *warp=findVisualItem(window->contentItem(),"transformWarp");QVERIFY(warp);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,warp->mapToScene({12,12}).toPoint());QTRY_VERIFY(client.transformWarp());QTRY_VERIFY(!client.transformBusy());
        const auto beforeGrid=client.transformPoints();QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,screen(beforeGrid[5].toPointF()));QTest::mouseMove(window,screen(beforeGrid[5].toPointF()+QPointF(3,-2)));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,screen(beforeGrid[5].toPointF()+QPointF(3,-2)));QTRY_VERIFY(!client.transformBusy());QTRY_VERIFY(client.transformPoints()!=beforeGrid);client.undo();QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformPoints(),beforeGrid);
        if(const auto preview=qEnvironmentVariable("DRAWVERSE_PASS_PREVIEW");!preview.isEmpty()){canvas->zoomBy(8);QTest::qWait(120);QVERIFY(window->grabWindow().save(preview));canvas->actualSize();}
        canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Right,Qt::ShiftModifier);QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformPoints()[0].toPointF().x(),beforeGrid[0].toPointF().x()+10.);client.undo();QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformPoints(),beforeGrid);
        canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Return);QTRY_VERIFY(!client.transformActive());QTRY_COMPARE(client.undoDepth(),1);QVERIFY(client.activeImage());QVERIFY(client.modified());const auto placed=client.activeLayer();QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath("placed.ora"))));QTRY_VERIFY(!client.fileBusy());
        canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_T,Qt::ControlModifier);QTRY_VERIFY(client.transformActive() && !client.transformBusy());QTest::keyClick(window,Qt::Key_Escape);QTRY_VERIFY(!client.transformActive());QCOMPARE(client.undoDepth(),1);
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());const auto beforeNumeric=client.transformPoints();auto *numeric=findVisualItem(window->contentItem(),"transformX");QVERIFY(numeric);QCOMPARE(numeric->height(),qreal(20));QCOMPARE(numeric->width(),qreal(52));QCOMPARE(numeric->property("font").value<QFont>().pixelSize(),9);numeric->setProperty("text",QString::number((beforeNumeric[0].toPointF().x()+beforeNumeric[15].toPointF().x())*.5+2.));QVERIFY(QMetaObject::invokeMethod(numeric,"editingFinished"));QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformPoints()[0].toPointF().x(),beforeNumeric[0].toPointF().x()+2.);auto *skew=findVisualItem(window->contentItem(),"transformSH");QVERIFY(skew);skew->setProperty("text","10");QVERIFY(QMetaObject::invokeMethod(skew,"editingFinished"));QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformSkewH(),10.);client.undo();QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformSkewH(),0.);client.redo();QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformSkewH(),10.);client.undo();QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformSkewH(),0.);
        client.activateMoveTool();QTRY_VERIFY(!client.transformActive());QTRY_VERIFY(client.moveTool());QTRY_COMPARE(client.undoDepth(),2);client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(client.undoDepth(),1);
        QVERIFY(client.rasterizeImage());QTRY_VERIFY(!client.transformBusy());QTRY_VERIFY(!client.activeImage());QTRY_COMPARE(client.undoDepth(),2);client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.activeImage());QCOMPARE(client.activeLayer(),placed);
        QVERIFY(client.openDocument(QUrl::fromLocalFile(temp.filePath("placed.ora"))));QTRY_VERIFY(!client.fileBusy() && client.ready());QVERIFY(client.activeImage());QCOMPARE(client.undoDepth(),0);documents->floatDocument(documents->activeId());QTRY_VERIFY(canvas->window()!=window);auto *floating=canvas->window();floating->requestActivate();QTest::qWait(80);canvas->forceActiveFocus();QTest::keyClick(floating,Qt::Key_T,Qt::ControlModifier);QTRY_VERIFY(client.transformActive() && !client.transformBusy());QVERIFY(findVisualItem(floating->contentItem(),"transformOptionsBar"));QTest::keyClick(floating,Qt::Key_Escape);QTRY_VERIFY(!client.transformActive());QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void liquifyCanvasTransactionOptionsAndDocumentLock() {
        QTemporaryDir temp;const auto settings=temp.filePath("liquify.ini");PaintCoreClient client(nullptr,settings);WorkspaceManager workspace(temp.filePath("layout.ini"));QStringList warnings;QQmlApplicationEngine engine;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);const auto first=documents->activeId();const auto other=documents->newDocument(32,32,true);QVERIFY(!other.isEmpty());QTRY_VERIFY(documents->client(other)->ready());documents->floatDocument(other);QVERIFY(documents->activate(first));
        QQuickWindow *foreign=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName().startsWith("documentWindow:"))foreign=qobject_cast<QQuickWindow*>(w);return foreign!=nullptr;}());QVERIFY(foreign->isVisible());
        workspace.floatToolStrip(window->x()+20,window->y()+90);QQuickWindow *tools=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingToolStrip")tools=qobject_cast<QQuickWindow*>(w);return tools!=nullptr;}());QVERIFY(tools->isVisible());
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());client.setBrushRadius(12);client.setBrushColor(Qt::red);QVERIFY(client.beginStroke({{45,45},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());const auto target=client.activeLayer();client.addLayer("other");QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(Qt::blue);QVERIFY(client.beginStroke({{85,85},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(target);QTRY_COMPARE(client.activeLayer(),target);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath("clean.ora"))));QTRY_VERIFY(!client.fileBusy() && !client.modified());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();window->requestActivate();QTRY_VERIFY(window->isActive());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(client.frame().size(),QSize(qCeil(128*window->devicePixelRatio()),qCeil(128*window->devicePixelRatio())));const auto before=client.frame();const auto depth=client.undoDepth();workspace.setActive(workspace.groupForPanel("history"),"history");QQuickItem *historyList=nullptr;QTRY_VERIFY((historyList=findVisualItem(window->contentItem(),"historyList")));historyList->setHeight(48);QTest::qWait(80);historyList->setProperty("contentY",24);QTRY_COMPARE(client.documentHistoryScrollPosition(),qreal(24));QCOMPARE(client.liquifyStrength(),50);QCOMPARE(client.liquifyDensity(),100);QCOMPARE(client.liquifySpeed(),100);QVERIFY(!client.liquifyPressure());client.setLiquifySize(48);
        QTest::qWait(100);window->requestActivate();QTest::qWait(100);QVERIFY(documents->activate(first));auto *action=window->findChild<QObject*>("liquifyAction");QVERIFY(action);QCOMPARE(documents->activeId(),first);QVERIFY(action->property("enabled").toBool());QVERIFY(QMetaObject::invokeMethod(action,"trigger"));QTRY_VERIFY2(client.liquifyActive() && !client.liquifyBusy(),qPrintable(client.lastError()+" active="+QString::number(client.liquifyActive())+" busy="+QString::number(client.liquifyBusy())));auto *panel=findVisualItem(window->contentItem(),"liquifyOptionsBar");QVERIFY(panel);QTRY_VERIFY(panel->isVisible());auto *focus=findVisualItem(window->contentItem(),"liquifyFocus");QVERIFY(focus);auto *indicator=focus->property("indicator").value<QQuickItem*>();QVERIFY(indicator);QCOMPARE(indicator->width(),qreal(13));QCOMPARE(indicator->height(),qreal(13));auto *separator=findVisualItem(window->contentItem(),"liquifyTitleSeparator");QVERIFY(separator);QCOMPARE(separator->width(),qreal(1));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,focus->mapToScene({8,focus->height()/2}).toPoint());QTRY_VERIFY(client.liquifyFocus() && !client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.frame()!=before);QVERIFY(!client.modified());client.setLiquifyFocus(false);QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),before);QTRY_VERIFY(!foreign->isVisible());QTRY_VERIFY(!tools->isVisible());QVERIFY(!documents->activate(other));QCOMPARE(documents->activeId(),first);QVERIFY(documents->newDocument(8,8,true).isEmpty());const auto count=client.layers().size();client.addDefaultLayer();QCOMPARE(client.layers().size(),count);
        canvas->forceActiveFocus();const auto brushSize=client.brushRadius();QTest::keyClick(window,Qt::Key_BracketRight);QCOMPARE(client.liquifySize(),55);QTest::keyClick(window,Qt::Key_BracketLeft);QCOMPARE(client.liquifySize(),48);QCOMPARE(client.brushRadius(),brushSize);QCOMPARE(client.undoDepth(),0);QTRY_COMPARE(historyList->property("count").toInt(),1);QCOMPARE(client.historyEntries().first().toMap().value("title").toString(),QStringLiteral("液化起始状态"));
        auto *pressure=findVisualItem(window->contentItem(),"liquifyPressure");QVERIFY(pressure);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,pressure->mapToScene({16,14}).toPoint());QVERIFY(client.liquifyPressure());auto *speed=findVisualItem(window->contentItem(),"liquifySpeedInput");QVERIFY(speed);QVERIFY(!speed->isEnabled());const auto fixed=panel->position();canvas->zoomBy(2.);canvas->actualSize();QCOMPARE(panel->position(),fixed);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({45,45}));QVERIFY(!client.modified());QTest::mouseMove(window,at({55,45}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({55,45}));QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.frame()!=before);QTRY_COMPARE(client.undoDepth(),1);QVERIFY(!client.modified());const auto preview=client.frame();canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),before);QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),preview);
        QVERIFY(client.resetLiquify());QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),before);client.setLiquifyTool(1);QVERIFY(speed->isEnabled());QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({45,45}));QTest::qWait(120);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({45,45}));QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.frame()!=before);canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Return);QTRY_VERIFY(!client.liquifyActive());QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_COMPARE(historyList->property("contentY").toReal(),qreal(24));QVERIFY(client.modified());QTRY_VERIFY(foreign->isVisible());QTRY_VERIFY(tools->isVisible());QVERIFY(!panel->isVisible());QTest::qWait(50);QCOMPARE(documents->activeId(),first);client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),before);
        const auto documentHistory=client.historyEntries();const auto savedHistoryScroll=client.documentHistoryScrollPosition();QVERIFY(!findVisualItem(tools->contentItem(),"liquifyTool"));QVERIFY(QMetaObject::invokeMethod(action,"trigger"));QTRY_VERIFY(client.liquifyActive() && !client.liquifyBusy());window->requestActivate();QTest::qWait(80);
        QPointingDevice pen("liquify pen",712,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);auto sendPen=[&](QEvent::Type type){const auto p=at({45,45});QTabletEvent e(type,&pen,p,window->mapToGlobal(p),.5,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&e);return e.isAccepted();};QVERIFY(sendPen(QEvent::TabletPress));QVERIFY(client.drawing());QTest::qWait(100);QVERIFY(sendPen(QEvent::TabletRelease));QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.frame()!=before);QTRY_COMPARE(client.undoDepth(),1);canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.liquifyBusy());QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.liquifyBusy());canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Escape);QTRY_VERIFY(!client.liquifyActive());QCOMPARE(client.undoDepth(),depth);QCOMPARE(client.historyEntries(),documentHistory);QCOMPARE(client.documentHistoryScrollPosition(),savedHistoryScroll);QTRY_COMPARE(historyList->property("contentY").toReal(),savedHistoryScroll);documents->floatDocument(first);QTRY_VERIFY(canvas->window()!=window);auto *floating=canvas->window();floating->requestActivate();QTest::qWait(100);QVERIFY(documents->activate(first));QVERIFY(client.beginLiquify());QTRY_VERIFY(client.liquifyActive() && !client.liquifyBusy());auto *floatingOptions=findVisualItem(floating->contentItem(),"liquifyOptionsBar");QVERIFY(floatingOptions);QVERIFY(floatingOptions->isVisible());canvas->forceActiveFocus();client.setLiquifySize(1);QTest::keyClick(floating,Qt::Key_BracketRight);QCOMPARE(client.liquifySize(),2);QTest::keyClick(floating,Qt::Key_BracketLeft);QCOMPARE(client.liquifySize(),1);client.setLiquifySize(48);QVERIFY(client.beginStroke({{45,45},1}));client.endStroke();QTRY_VERIFY(!client.liquifyBusy());QTRY_COMPARE(client.undoDepth(),1);QTest::keyClick(floating,Qt::Key_Z,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.liquifyBusy());QTest::keyClick(floating,Qt::Key_Z,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.liquifyBusy());QVERIFY(!documents->activate(other));QVERIFY(client.finishLiquify(false));QTRY_VERIFY(!client.liquifyActive());QTRY_VERIFY(!findVisualItem(floating->contentItem(),"liquifyOptionsBar") || !findVisualItem(floating->contentItem(),"liquifyOptionsBar")->isVisible());client.setLiquifyStrength(62);client.setLiquifyDensity(81);client.setLiquifySpeed(73);QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
        PaintCoreClient restored(nullptr,settings);QTRY_VERIFY(restored.ready());QCOMPARE(restored.liquifySize(),48);QCOMPARE(restored.liquifyTool(),1);QCOMPARE(restored.liquifyStrength(),62);QCOMPARE(restored.liquifyDensity(),81);QCOMPARE(restored.liquifySpeed(),73);QVERIFY(restored.liquifyPressure());QSignalSpy stoppedAgain(&restored,&PaintCoreClient::stopped);restored.shutdown();QTRY_COMPARE(stoppedAgain.size(),1);
    }
    void layerContextClipboardFocusShortcutsAndLowerEdgeDrop() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("actions.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);window->requestActivate();QTRY_VERIFY(window->isActive());
        QTRY_VERIFY(client.ready());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        client.addLayer("top");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto top=client.activeLayer();client.setBrushRadius(8);client.setBrushColor(Qt::blue);QVERIFY(client.beginStroke({{16,16},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.addMask(top));QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());const auto mask=client.activeLayer();
        const auto item=[&](const QString &name){return findVisualItem(window->contentItem(),name);};
        auto *maskFrame=item("thumbnailFocus:"+QString::number(mask));QVERIFY(maskFrame);QVERIFY(maskFrame->isVisible());QCOMPARE(QQmlProperty::read(maskFrame,"border.color").value<QColor>(),QColor(Qt::white));
        auto *contentFrame=item("thumbnailFocus:"+QString::number(top));QVERIFY(contentFrame);QVERIFY(!contentFrame->isVisible());
        auto *panel=item("layersPanel");QVERIFY(panel);auto *row=item("layerRow:"+QString::number(top));QVERIFY(row);QCOMPARE(QQmlProperty::read(row,"border.width").toInt(),0);
        const auto rightClick=[&](quint64 id){auto *r=item("layerRow:"+QString::number(id));if(!r)return false;QTest::mouseClick(window,Qt::RightButton,Qt::NoModifier,r->mapToScene({r->width()*.7,r->height()/2}).toPoint());return true;};
        QVERIFY(rightClick(top));QTRY_COMPARE(client.activeLayer(),top);auto *menu=window->findChild<QObject*>("layerContextMenu");QVERIFY(menu);QTRY_VERIFY(menu->property("visible").toBool());auto *merge=window->findChild<QObject*>("contextMergeLayers");QVERIFY(merge);QCOMPARE(merge->property("text").toString(),QStringLiteral("向下合并"));QVERIFY(merge->property("enabled").toBool());QVERIFY(QMetaObject::invokeMethod(menu,"close"));
        panel->forceActiveFocus();QTRY_VERIFY(client.layerShortcutsAllowed());QTest::keyClick(window,Qt::Key_C,Qt::ControlModifier);QTRY_VERIFY(client.canPasteLayers());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.layers().size(),3);
        QTest::keyClick(window,Qt::Key_V,Qt::ControlModifier);QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());const auto pasted=client.activeLayer();QVERIFY(pasted!=top);QTest::keyClick(window,Qt::Key_Delete);QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());
        client.selectLayers(top,0);QTRY_COMPARE(client.activeLayer(),top);QTRY_VERIFY(!client.layerEditBusy());panel->forceActiveFocus();QTest::keyClick(window,Qt::Key_J,Qt::ControlModifier);QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());const auto duplicate=client.activeLayer();quint64 duplicateMask=0;for(const auto &v:client.layers()){const auto l=v.toMap();if(l.value("mask").toBool() && l.value("parent").toULongLong()==duplicate)duplicateMask=l.value("id").toULongLong();}QVERIFY(duplicateMask);
        client.selectLayer(duplicateMask);QTRY_COMPARE(client.activeLayer(),duplicateMask);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(item("thumbnailFocus:"+QString::number(duplicateMask))->isVisible());QVERIFY(!item("thumbnailFocus:"+QString::number(duplicate))->isVisible());panel->forceActiveFocus();QTest::keyClick(window,Qt::Key_Delete);QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());bool duplicateExists=false;for(const auto &v:client.layers())if(v.toMap().value("id").toULongLong()==duplicate)duplicateExists=true;QVERIFY(duplicateExists);
        auto *search=item("layerSearch");QVERIFY(search);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,search->mapToScene({search->width()/2,search->height()/2}).toPoint());QTRY_VERIFY(search->hasActiveFocus());QTRY_VERIFY(!client.layerShortcutsAllowed());QVERIFY(search->setProperty("text","abc"));QVERIFY(search->setProperty("cursorPosition",0));const auto count=client.layers().size();QTest::keyClick(window,Qt::Key_Delete);QCOMPARE(search->property("text").toString(),QStringLiteral("bc"));QCOMPARE(client.layers().size(),count);
        auto *list=item("layerList");QVERIFY(list);QTRY_COMPARE(list->property("count").toInt(),0);QVERIFY(item("layerSearchEmpty")->isVisible());
        search->setProperty("text"," TOP ");QTRY_COMPARE(list->property("count").toInt(),2);QVERIFY(!item("layerSearchEmpty")->isVisible());
        client.activateEraser();search->setProperty("text","");QTest::keyClick(window,Qt::Key_B);QCOMPARE(search->property("text").toString(),QStringLiteral("b"));QVERIFY(client.eraser());QTest::keyClick(window,Qt::Key_Escape);QCOMPARE(search->property("text").toString(),QString());QTRY_VERIFY(client.layerShortcutsAllowed());QTest::keyClick(window,Qt::Key_B);QVERIFY(!client.eraser());
        search->forceActiveFocus();search->setProperty("text","top");QTest::keyClick(window,Qt::Key_Return);QTRY_VERIFY(client.layerShortcutsAllowed());QCOMPARE(search->property("text").toString(),QStringLiteral("top"));
        search->forceActiveFocus();auto *eraser=item("eraserTool");QVERIFY(eraser);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,eraser->mapToScene({13,13}).toPoint());QVERIFY(client.eraser());QTRY_VERIFY(client.layerShortcutsAllowed());QTest::keyClick(window,Qt::Key_B);QVERIFY(!client.eraser());
        // Clipboard shortcuts must work after a tool click has released search focus.
        QTest::keyClick(window,Qt::Key_C,Qt::ControlModifier);QTRY_VERIFY(!client.layerEditBusy());QTest::keyClick(window,Qt::Key_V,Qt::ControlModifier);QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());client.undo();QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());search->setProperty("text","");
        QVERIFY(!window->findChild<QObject*>("contextCopyLayers"));QVERIFY(!window->findChild<QObject*>("contextPasteLayers"));QVERIFY(!window->findChild<QObject*>("copyLayersAction"));QVERIFY(!window->findChild<QObject*>("pasteLayersAction"));QVERIFY(!item("liquifyTool"));
        client.selectLayers(top,0);QTRY_COMPARE(client.activeLayer(),top);QTRY_VERIFY(!client.layerEditBusy());panel->forceActiveFocus();QTest::keyClick(window,Qt::Key_G,Qt::ControlModifier);QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();client.toggleGroupExpanded(group);QTRY_COMPARE(list->property("count").toInt(),3);search->setProperty("text"," TOP ");QTRY_COMPARE(list->property("count").toInt(),3);QVERIFY(item("layerRow:"+QString::number(top)));search->setProperty("text","组");QTRY_COMPARE(list->property("count").toInt(),2);search->setProperty("text","   ");QTRY_COMPARE(list->property("count").toInt(),3);client.toggleGroupExpanded(group);search->setProperty("text","");QTRY_COMPARE(list->property("count").toInt(),4);QVERIFY(rightClick(group));QTRY_VERIFY(menu->property("visible").toBool());QCOMPARE(merge->property("text").toString(),QStringLiteral("合并组"));QVERIFY(QMetaObject::invokeMethod(menu,"close"));
        client.selectLayers(group,0);client.selectLayers(duplicate,int(Qt::ControlModifier));QTRY_COMPARE(client.activeLayer(),duplicate);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(rightClick(group));QTRY_VERIFY(menu->property("visible").toBool());QCOMPARE(client.selectedLayers().size(),2);QCOMPARE(merge->property("text").toString(),QStringLiteral("合并图层"));QVERIFY(QMetaObject::invokeMethod(menu,"close"));
        // Real drop events exercise the lower blue line and destination-parent inheritance.
        client.selectLayers(duplicate,0);QTRY_COMPARE(client.activeLayer(),duplicate);QTRY_VERIFY(!client.layerEditBusy());auto *target=item("layerRow:"+QString::number(top));QVERIFY(target);const auto at=target->mapToScene({target->width()*.72,target->height()-3}).toPoint();QMimeData mime;mime.setData("application/x-drawverse-layer",client.layerDragPayload(duplicate).toUtf8());QDragEnterEvent enter(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&enter);QDragMoveEvent move(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&move);QDropEvent drop(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&drop);QVERIFY(drop.isAccepted());QTRY_VERIFY(!client.layerEditBusy());
        QVariantMap moved;for(const auto &v:client.layers())if(v.toMap().value("id").toULongLong()==duplicate)moved=v.toMap();QCOMPARE(moved.value("parent").toULongLong(),group);QCOMPARE(client.lastError(),QString());
        client.selectLayers(group,0);QTRY_COMPARE(client.activeLayer(),group);QTRY_VERIFY(!client.layerEditBusy());const auto depth=client.undoDepth();panel->forceActiveFocus();QTest::keyClick(window,Qt::Key_E,Qt::ControlModifier);QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);client.undo();QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());
        auto *mainDuplicate=window->findChild<QObject*>("duplicateLayersAction");QVERIFY(mainDuplicate);QVERIFY(mainDuplicate->property("enabled").toBool());QVERIFY(QMetaObject::invokeMethod(mainDuplicate,"trigger"));QTRY_COMPARE(client.layers().size(),9);QTRY_VERIFY(!client.layerEditBusy());
        workspace.setActive(workspace.groupForPanel("brush"),"brush");QQuickItem *brushSearch=nullptr;QTRY_VERIFY((brushSearch=item("brushSearch")));auto *presets=item("brushPresetList");QVERIFY(presets);brushSearch->forceActiveFocus();QVERIFY(client.textInputFocused());brushSearch->setProperty("text"," 细线 ");QTRY_COMPARE(presets->property("count").toInt(),1);QTest::keyClick(window,Qt::Key_Return);QTRY_VERIFY(client.layerShortcutsAllowed());brushSearch->forceActiveFocus();QTest::keyClick(window,Qt::Key_Escape);QTRY_COMPARE(presets->property("count").toInt(),client.brushLibrary()->presets().size());QTRY_VERIFY(client.layerShortcutsAllowed());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void groupPassThroughIsGroupOnlyAndChangesBackdropWithUndo() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("pass.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();client.setBrushRadius(64);client.setBrushColor(QColor("#404040"));QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.addLayer("dodge");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto top=client.activeLayer();client.setBrushRadius(12);client.setBrushColor(QColor("#808080"));QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.setLayerBlend(top,9));QTRY_COMPARE(client.undoDepth(),4);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto pixel=[&](){const auto r=client.frameRegion();const auto &f=client.frame();return f.pixelColor(qFloor((32.5-r.x())*f.width()/r.width()),qFloor((32.5-r.y())*f.height()/r.height()));};const auto flat=pixel();QVERIFY(client.groupLayer(top,"组 1"));QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();QVERIFY(group!=top);QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.setLayerBlend(group,0));QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());const auto isolated=pixel();QVERIFY(flat!=isolated);auto *blend=findVisualItem(window->contentItem(),"layerBlendMode");QVERIFY(blend);QTRY_COMPARE(blend->property("count").toInt(),28);QCOMPARE(blend->property("currentIndex").toInt(),1);const auto depth=client.undoDepth();const auto revision=client.revision();
        client.previewLayerBlend(group,27);QTRY_COMPARE(pixel(),flat);QCOMPARE(client.undoDepth(),depth);QCOMPARE(client.revision(),revision);client.clearLayerBlendPreview();QTRY_COMPARE(pixel(),isolated);QVERIFY(blend->setProperty("currentIndex",0));QVERIFY(QMetaObject::invokeMethod(blend,"activated",Q_ARG(int,0)));QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(),flat);QCOMPARE(blend->property("currentText").toString(),QStringLiteral("穿透"));client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(),isolated);client.redo();QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(),flat);
        const auto preview=qEnvironmentVariable("DRAWVERSE_PASS_PREVIEW");if(!preview.isEmpty())QVERIFY(window->grabWindow().save(preview));client.selectLayer(top);QTRY_COMPARE(client.activeLayer(),top);QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(blend->property("count").toInt(),27);QVERIFY(!client.setLayerBlend(top,27));QCOMPARE(client.undoDepth(),depth+1);QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void eyedropperRingComparesColorsDuringMouseAndTabletDrag() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("ring.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushRadius(12);client.setBrushColor(Qt::red);QVERIFY(client.beginStroke({{32,64},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(Qt::green);QVERIFY(client.beginStroke({{96,64},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);QTRY_VERIFY(client.frameRevision()>=client.revision());client.activateEyedropper();client.setBrushColor(Qt::blue);const auto revision=client.revision();const int depth=client.undoDepth();
        auto *ring=findVisualItem(window->contentItem(),"eyedropperSamplingRing");QVERIFY(ring);auto *show=findVisualItem(window->contentItem(),"showSamplingRing");QVERIFY(show);QVERIFY(client.samplingRingEnabled());QCOMPARE(QQmlProperty::read(show,"ToolTip.delay",qmlContext(show)).toInt(),1000);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};const auto red=at({32,64}),green=at({96,64});QTest::mouseMove(window,red);QVERIFY(!ring->isVisible());QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,red);QTRY_VERIFY(ring->isVisible());QCOMPARE(canvas->previousSampleColor(),QColor(Qt::blue));QCOMPARE(canvas->sampledColor(),QColor(Qt::red));QCOMPARE(client.brushColor(),QColor(Qt::red));QCOMPARE(ring->width(),120.);
        // Inspect the actual native composition: top=new, bottom=old, neutral outer ring, hollow center.
        QTest::qWait(80);const auto image=window->grabWindow();QVERIFY(!image.isNull());const auto center=canvas->mapToScene(canvas->cursorPosition());const qreal scale=qreal(image.width())/window->width();const auto screenPixel=[&](QPointF offset){const auto p=(center+offset)*scale;return image.pixelColor(qRound(p.x()),qRound(p.y()));};QCOMPARE(screenPixel({0,-36}),QColor(Qt::red));QCOMPARE(screenPixel({0,36}),QColor(Qt::blue));QCOMPARE(screenPixel({0,-51}),QColor("#808080"));QCOMPARE(screenPixel({4,4}),QColor(Qt::red));const auto preview=qEnvironmentVariable("DRAWVERSE_RING_PREVIEW");if(!preview.isEmpty())QVERIFY(image.save(preview));
        QTest::mouseMove(window,green);QTRY_COMPARE(canvas->sampledColor(),QColor(Qt::green));QCOMPARE(canvas->previousSampleColor(),QColor(Qt::blue));QCOMPARE(client.brushColor(),QColor(Qt::green));QTest::mouseMove(window,at({-4,64}));QVERIFY(!ring->isVisible());QCOMPARE(client.brushColor(),QColor(Qt::green));QTest::mouseMove(window,red);QTRY_VERIFY(ring->isVisible());QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,red);QVERIFY(!ring->isVisible());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,show->mapToScene({7,show->height()/2}).toPoint());QVERIFY(!client.samplingRingEnabled());QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,green);QVERIFY(!ring->isVisible());QCOMPARE(client.brushColor(),QColor(Qt::green));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,green);client.setSamplingRingEnabled(true);
        client.activateBrush();client.setBrushColor(Qt::blue);canvas->forceActiveFocus();QTest::keyPress(window,Qt::Key_Alt);QTest::mousePress(window,Qt::LeftButton,Qt::AltModifier,red);QTRY_VERIFY(ring->isVisible());QCOMPARE(canvas->previousSampleColor(),QColor(Qt::blue));QTest::mouseMove(window,green);QTRY_COMPARE(canvas->sampledColor(),QColor(Qt::green));QTest::mouseRelease(window,Qt::LeftButton,Qt::AltModifier,green);QTest::keyRelease(window,Qt::Key_Alt);QVERIFY(!ring->isVisible());QVERIFY(!client.drawing());
        client.activateEyedropper();client.setBrushColor(Qt::blue);QPointingDevice pen("ring pen",126,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);auto send=[&](QEvent::Type type,QPoint p){QTabletEvent e(type,&pen,p,window->mapToGlobal(p),.5,0,0,0,0,0,Qt::NoModifier,type==QEvent::TabletMove?Qt::NoButton:Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&e);return e.isAccepted();};QVERIFY(send(QEvent::TabletPress,red));QVERIFY(ring->isVisible());QCOMPARE(canvas->previousSampleColor(),QColor(Qt::blue));QVERIFY(send(QEvent::TabletMove,green));QCOMPARE(canvas->sampledColor(),QColor(Qt::green));QVERIFY(send(QEvent::TabletRelease,green));QVERIFY(!ring->isVisible());
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,red);QVERIFY(ring->isVisible());QEvent deactivate(QEvent::WindowDeactivate);QCoreApplication::sendEvent(window,&deactivate);QVERIFY(!ring->isVisible());QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,red);QCOMPARE(client.undoDepth(),depth);QCOMPARE(client.revision(),revision);QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void altEyedropperSamplesWithoutHistoryAndBracketKeysResize() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();main->requestActivate();QTest::qWait(60);
        client.setSelectionTool(0);QTRY_COMPARE(client.selectionTool(),0);
        // Paint a known colour, then sample it back through the eyedropper path.
        client.setBrushRadius(6);client.setBrushColor(QColor("#c81e3a"));
        InputSample stroke;stroke.position={32,32};stroke.pressure=1;
        QVERIFY(client.beginStroke(stroke));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),10000);
        const auto sampled=client.sampleDocumentPixel(32,32);
        QVERIFY2(sampled.isValid(),"eyedropper returned no colour");
        QVERIFY2(sampled.red()>150 && sampled.green()<90,qPrintable(sampled.name()));
        // Alt is the eyedropper: it adopts the colour and writes nothing to the document.
        client.setBrushColor(QColor("#1188ee"));QTRY_COMPARE(client.brushColor(),QColor("#1188ee"));
        const auto historyBefore=client.undoDepth();const auto revisionBefore=client.revision();
        const auto center=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(32,32)*canvas->zoom()).toPoint();
        QTest::mousePress(main,Qt::LeftButton,Qt::AltModifier,center);
        QTest::mouseRelease(main,Qt::LeftButton,Qt::AltModifier,center);
        QTRY_VERIFY(client.brushColor().red()>150 && client.brushColor().green()<90);
        QCOMPARE(client.undoDepth(),historyBefore);QCOMPARE(client.revision(),revisionBefore);
        // Switching away from the marquee and back keeps the chosen shape instead of resetting it.
        client.setSelectionTool(2);QTRY_COMPARE(client.selectionTool(),2);
        client.activateBrush();QTRY_VERIFY(!client.selectionTool());
        auto *selectionSlot=findVisualItem(main->contentItem(),"selectionTool");QVERIFY(selectionSlot);QCOMPARE(selectionSlot->property("glyph").toString(),QString("ellipseSelection"));
        auto *move=findVisualItem(main->contentItem(),"moveLayerTool");QVERIFY(move);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,move->mapToScene({13,13}).toPoint());QVERIFY(client.moveTool());QCOMPARE(selectionSlot->property("glyph").toString(),QString("ellipseSelection"));
        client.activateMarquee();QTRY_COMPARE(client.selectionTool(),2);
        QVERIFY(!client.moveTool() && !client.eraser());
        client.setSelectionTool(1);QTRY_COMPARE(client.selectionTool(),1);
        client.activateEraser();QTRY_VERIFY(!client.selectionTool());
        client.activateMarquee();QTRY_COMPARE(client.selectionTool(),1);
        client.setSelectionTool(0);QTRY_VERIFY(!client.selectionTool());        // Alt is the eyedropper: the brush ring goes away and the pointer becomes a crosshair.
        canvas->forceActiveFocus();QTest::mouseMove(main,center);
        QTest::keyPress(main,Qt::Key_Alt);QTest::qWait(40);
        QVERIFY2(!canvas->brushCursorVisible(),"brush ring still shown while Alt is held");
        auto *cross=findVisualItem(main->contentItem(),"eyedropperCursor");QVERIFY(cross);QVERIFY(cross->isVisible());QCOMPARE(canvas->cursor().shape(),Qt::BlankCursor);
        QTest::mouseMove(main,center+QPoint(5,3));QTRY_VERIFY(QLineF(cross->mapToScene({8,8}),center+QPoint(5,3)).length()<1);
        QPointingDevice pen("Picker pen",1901,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,2);
        const QPointF penPoint=center+QPoint(11,8);QTabletEvent hover(QEvent::TabletMove,&pen,penPoint,main->mapToGlobal(penPoint.toPoint()),0,0,0,0,0,0,Qt::AltModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(main,&hover);QVERIFY(hover.isAccepted());QVERIFY(canvas->eyedropperCursorVisible());QVERIFY(QLineF(cross->mapToScene({8,8}),penPoint).length()<1);
        const auto pickerImage=main->grabWindow();const auto blackPixel=cross->mapToScene({0,7});const auto dpi=pickerImage.width()/qreal(main->width());QCOMPARE(pickerImage.pixelColor(qRound(blackPixel.x()*dpi),qRound(blackPixel.y()*dpi)),QColor(Qt::black));
        client.setBrushColor(QColor("#1188ee"));QTabletEvent pickPress(QEvent::TabletPress,&pen,center,main->mapToGlobal(center),.6,0,0,0,0,0,Qt::AltModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(main,&pickPress);QTabletEvent pickRelease(QEvent::TabletRelease,&pen,center,main->mapToGlobal(center),0,0,0,0,0,0,Qt::AltModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(main,&pickRelease);QVERIFY(pickPress.isAccepted() && pickRelease.isAccepted());QVERIFY(client.brushColor().red()>150 && client.brushColor().green()<90);QCOMPARE(client.undoDepth(),historyBefore);
        client.setSelectionTool(2);QVERIFY(!cross->isVisible());client.activateBrush();
        QTest::keyRelease(main,Qt::Key_Alt);QTest::qWait(40);QVERIFY(!cross->isVisible());
        const auto beforePan=canvas->documentRect().topLeft();QTest::keyPress(main,Qt::Key_Space);
        QTabletEvent panPress(QEvent::TabletPress,&pen,center,main->mapToGlobal(center),.6,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(main,&panPress);const auto panEnd=center+QPoint(12,9);QTabletEvent panMove(QEvent::TabletMove,&pen,panEnd,main->mapToGlobal(panEnd),.6,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::LeftButton);QCoreApplication::sendEvent(main,&panMove);QTabletEvent panRelease(QEvent::TabletRelease,&pen,panEnd,main->mapToGlobal(panEnd),0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(main,&panRelease);QTest::keyRelease(main,Qt::Key_Space);QCOMPARE(canvas->documentRect().topLeft(),beforePan+QPointF(12,9));QCOMPARE(client.undoDepth(),historyBefore);QVERIFY(!client.drawing());
        // Bracket keys resize the active painting tool.
        client.setBrushRadius(20);const auto wide=client.brushRadius();
        QTest::keyClick(main,Qt::Key_BracketLeft);QTRY_VERIFY(client.brushRadius()<wide);
        const auto narrow=client.brushRadius();
        QTest::keyClick(main,Qt::Key_BracketRight);QTRY_VERIFY(client.brushRadius()>narrow);
        auto *picker=findVisualItem(main->contentItem(),"eyedropperTool");QVERIFY(picker);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,picker->mapToScene({13,13}).toPoint());QVERIFY(client.eyedropperTool());QVERIFY(picker->property("checked").toBool());QVERIFY(!client.selectionTool() && !client.moveTool() && !client.eraser());
        client.setBrushColor(QColor("#1188ee"));const auto pickedAt=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(32,32)*canvas->zoom()).toPoint();QTest::mouseMove(main,pickedAt);QVERIFY(canvas->eyedropperCursorVisible());QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,pickedAt);QVERIFY(client.brushColor().red()>150 && client.brushColor().green()<90);QCOMPARE(client.undoDepth(),historyBefore);QCOMPARE(client.revision(),revisionBefore);
        auto *toolStatus=findVisualItem(main->contentItem(),"statusTool");QVERIFY(toolStatus);QVERIFY(toolStatus->property("text").toString().contains(QStringLiteral("吸色")));
        canvas->forceActiveFocus();QTest::keyClick(main,Qt::Key_B);QVERIFY(!client.eyedropperTool());QTest::keyClick(main,Qt::Key_I);QVERIFY(client.eyedropperTool());
        workspace.floatToolStrip(250,150);QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingToolStrip")floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());auto *floatingPicker=findVisualItem(floating->contentItem(),"eyedropperTool");QVERIFY(floatingPicker);QVERIFY(floatingPicker->property("checked").toBool());client.activateBrush();QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,floatingPicker->mapToScene({13,13}).toPoint());QVERIFY(client.eyedropperTool());QVERIFY(floatingPicker->y()+floatingPicker->height()<floatingPicker->parentItem()->height());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }    void selectionOutlineMarchesWhileVisibleAndStopsOtherwise() {
        QQuickWindow window;window.setObjectName("antsHost");window.setColor(Qt::transparent);
        auto *item=new SelectionOverlay();item->setObjectName("antsOverlay");item->setParentItem(window.contentItem());
        item->setDocumentRect({0,0,64,64});item->setZoom(1);
        const auto step=[](int op,int shape,QRectF r){return QVariantMap{{"operation",op},{"shape",shape},{"x",r.x()},{"y",r.y()},{"width",r.width()},{"height",r.height()}};};
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));
        QSignalSpy phaseChanged(item,&SelectionOverlay::dashPhaseChanged);QSignalSpy geometryChanged(item,&SelectionOverlay::geometryChanged);
        // With no selection there is nothing to confirm, so no animation runs.
        QTest::qWait(200);QCOMPARE(item->dashPhase(),0);QCOMPARE(phaseChanged.size(),0);
        item->setSteps(QVariantList{step(0,0,{8,8,40,40})});item->setEnabledSelection(true);
        // The dash phase advances so the boundary visibly marches; geometry stays untouched.
        QTRY_VERIFY_WITH_TIMEOUT(item->dashPhase()!=0,1500);
        const auto settled=geometryChanged.size();
        QTRY_VERIFY_WITH_TIMEOUT(phaseChanged.size()>1,1500);
        QCOMPARE(geometryChanged.size(),settled);
        // Closing the window stops the timer instead of repainting forever.
        window.close();QTest::qWait(30);
        const auto stopped=phaseChanged.size();QTest::qWait(250);QCOMPARE(phaseChanged.size(),stopped);
    }

    void denseLassoPaintingUpdatesFramesAndRestoresSelectionAfterUndo() {
        QTemporaryDir temp;
        PaintCoreClient client(nullptr,temp.filePath("selection.ini"));
        QTRY_VERIFY(client.ready());
        client.newTransparentDocument(1024,1024);
        QTRY_VERIFY(client.ready() && client.documentWidth()==1024);
        client.requestViewport(0,{0,0,1024,1024},{1024,1024});
        QVector<QPointF> points;
        for(int i=0;i<4096;++i) {
            const double angle=i*6.283185307179586/4096;
            points.append({512+410*std::cos(angle),512+410*std::sin(angle)});
        }
        client.activateBrush();client.setBrushRadius(32);client.setBrushColor(Qt::black);
        for(const double feather:{0.,8.}) {
            client.setSelectionFeather(feather);
            QVERIFY(client.editSelectionPath(points,0,false));
            QTRY_VERIFY(!client.layerEditBusy());
            const auto depth=client.undoDepth();
            QCOMPARE(client.selectionSteps().last().toMap().value("points").toList().size(),4096);
            int ticks=0;
            QTimer heartbeat;heartbeat.setInterval(1);
            connect(&heartbeat,&QTimer::timeout,this,[&]{++ticks;});heartbeat.start();
            InputSample sample;sample.position={180,512};
            QVERIFY(client.beginStroke(sample));
            for(int i=1;i<=120;++i) {
                sample.position={180+i*5.5,512+std::sin(i*.07)*160};
                client.strokeTo(sample);
            }
            client.endStroke();
            QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),depth+1,10000);
            QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),10000);
            heartbeat.stop();QVERIFY(ticks>0);
            QCOMPARE(client.frame().size(),QSize(1024,1024));
            QVERIFY(client.frame().pixelColor(180,512).alpha()>0);
            QCOMPARE(client.frame().pixelColor(10,10).alpha(),0);
            client.activateEraser();client.setBrushRadius(32);
            const auto paintedAlpha=client.frame().pixelColor(180,512).alpha();
            sample.position={180,512};QVERIFY(client.beginStroke(sample));client.endStroke();
            QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),depth+2,10000);
            QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),10000);
            QVERIFY(client.frame().pixelColor(180,512).alpha()<paintedAlpha);
            QCOMPARE(client.frame().pixelColor(10,10).alpha(),0);
            client.undo();QTRY_COMPARE(client.undoDepth(),depth+1);
            QTRY_VERIFY(client.frameRevision()>=client.revision());
            QCOMPARE(client.frame().pixelColor(180,512).alpha(),paintedAlpha);
            client.activateBrush();
            client.undo();QTRY_COMPARE(client.undoDepth(),depth);
            QTRY_VERIFY(client.frameRevision()>=client.revision());
            QCOMPARE(client.frame().pixelColor(180,512).alpha(),0);
            // Clearing and undoing the selection must invalidate the coverage cache.
            QVERIFY(client.clearSelection());QTRY_VERIFY(!client.layerEditBusy());
            QVERIFY(!client.selectionEnabled());
            client.undo();QTRY_COMPARE(client.undoDepth(),depth);
            QTRY_VERIFY(client.selectionEnabled() && !client.layerEditBusy());
            sample.position={100,512};QVERIFY(client.beginStroke(sample));client.endStroke();
            QTRY_COMPARE(client.undoDepth(),depth+1);
            QTRY_VERIFY(client.frameRevision()>=client.revision());
            QCOMPARE(client.frame().pixelColor(80,512).alpha(),0);
            QVERIFY(client.frame().pixelColor(110,512).alpha()>0);
            client.undo();QTRY_COMPARE(client.undoDepth(),depth);
            QTRY_VERIFY(!client.layerEditBusy());
        }
        QVERIFY(client.lastError().isEmpty());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);
        client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void smudgeSharedBrushParametersInputAndDelayedHelp() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("smudge.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);QTest::keyClick(window,Qt::Key_R);QTRY_VERIFY(client.smudgeTool());
        QCOMPARE(client.smudgeStrength(),.5);QVERIFY(client.smudgePressure() && !client.smudgeAllLayers() && !client.smudgeFingerPainting());auto *bar=findVisualItem(window->contentItem(),"smudgeOptionsBar");QVERIFY(bar && bar->isVisible());
        auto *preset=findVisualItem(window->contentItem(),"smudgeBrushPreset");QVERIFY(preset);QVERIFY(QMetaObject::invokeMethod(preset,"activated",Q_ARG(int,1)));QVERIFY(client.smudgeTool());QCOMPARE(preset->property("count").toInt(),client.brushLibrary()->presets().size());
        client.brushLibrary()->setSpacing(.02);QCOMPARE(client.brushSpacing(),.02);BrushLibrary restored;QVERIFY(restored.restore(client.brushLibrary()->snapshot()));QCOMPARE(restored.spacing(),.02);
        auto *strength=findVisualItem(window->contentItem(),"smudgeStrengthInput");QVERIFY(strength);strength->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_8);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QCOMPARE(client.smudgeStrength(),.8);
        auto click=[&](const QString &name){auto *item=findVisualItem(window->contentItem(),name);if(!item)return false;QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,item->mapToScene({7,item->height()/2}).toPoint());return true;};
        QVERIFY(click("smudgePressure"));QVERIFY(!client.smudgePressure());QVERIFY(click("smudgeAllLayers"));QVERIFY(client.smudgeAllLayers());QVERIFY(click("smudgeAllLayers"));QVERIFY(click("smudgeFingerPainting"));QVERIFY(client.smudgeFingerPainting());
        for(const auto name:{"smudgePressure","smudgeAllLayers","smudgeFingerPainting","wandAntialias","wandContiguous","wandAllLayers","bucketAntialias","bucketContiguous","bucketAllLayers","selectionAntialias"}) {
            auto *item=findVisualItem(window->contentItem(),name);QVERIFY(item);QCOMPARE(QQmlProperty::read(item,"ToolTip.delay",qmlContext(item)).toInt(),1000);QVERIFY(!QQmlProperty::read(item,"ToolTip.text",qmlContext(item)).toString().isEmpty());
        }
        auto *helpItem=findVisualItem(window->contentItem(),"smudgeFingerPainting");auto *tip=QQmlProperty::read(helpItem,"ToolTip.toolTip",qmlContext(helpItem)).value<QObject*>();QVERIFY(tip);QTest::mouseMove(window,{2,100});QTest::mouseMove(window,helpItem->mapToScene({7,helpItem->height()/2}).toPoint());QTRY_VERIFY(helpItem->property("hovered").toBool());QTest::qWait(400);QVERIFY(!tip->property("visible").toBool());QTRY_VERIFY_WITH_TIMEOUT(tip->property("visible").toBool(),1500);QTest::mouseMove(window,{2,100});QTRY_VERIFY(!tip->property("visible").toBool());
        client.setBrushRadius(4);client.setBrushOpacity(1);client.setBrushColor(Qt::blue);const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({20,32}));QTest::mouseMove(window,at({32,32}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({32,32}));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());
        QCOMPARE(client.historyEntries().last().toMap().value("kind").toInt(),26);QCOMPARE(client.historyEntries().last().toMap().value("title").toString(),QStringLiteral("涂抹"));
        const auto pixel=[&](int x,int y){const auto r=client.frameRegion();const auto &f=client.frame();return f.pixelColor(qFloor((x+.5-r.x())*f.width()/r.width()),qFloor((y+.5-r.y())*f.height()/r.height()));};QVERIFY(pixel(20,32).blue()>0);client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(20,32).alpha(),0);
        client.setSmudgePressure(true);QPointingDevice pen("smudge pen",125,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        for(const auto type:{QEvent::TabletPress,QEvent::TabletMove,QEvent::TabletRelease}) {const auto local=at(type==QEvent::TabletPress?QPointF(20,32):QPointF(32,32));QTabletEvent event(type,&pen,local,window->mapToGlobal(local),.5,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&event);QVERIFY(event.isAccepted());}QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        const auto preview=qEnvironmentVariable("DRAWVERSE_SMUDGE_PREVIEW");if(!preview.isEmpty())QVERIFY(window->grabWindow().save(preview));client.activateBrush();QVERIFY(!client.smudgeTool());QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void bucketOptionsMouseTabletAndFloatingCanvasFillOnce() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("bucket.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);
        QTest::keyClick(window,Qt::Key_G);QTRY_VERIFY(client.bucketTool());QVERIFY(!client.eraser() && !client.selectionTool() && !client.moveTool() && !client.eyedropperTool());
        QCOMPARE(client.bucketTolerance(),0);QCOMPARE(client.bucketOpacity(),1.);QVERIFY(client.bucketAntialias() && client.bucketContiguous() && !client.bucketAllLayers());
        auto *bar=findVisualItem(window->contentItem(),"bucketOptionsBar");QVERIFY(bar && bar->isVisible());
        auto *bucketIcon=findVisualItem(window->contentItem(),"bucketOptionsIcon");QVERIFY(bucketIcon && bucketIcon->isVisible());QCOMPARE(bucketIcon->property("name").toString(),QStringLiteral("bucket"));QCOMPARE(bucketIcon->width(),17.);QCOMPARE(bucketIcon->height(),17.);
        auto *statusTool=findVisualItem(window->contentItem(),"statusTool");QVERIFY(statusTool);QCOMPARE(statusTool->property("text").toString(),QStringLiteral("工具  油漆桶工具"));
        const auto preview=qEnvironmentVariable("DRAWVERSE_BUCKET_PREVIEW");if(!preview.isEmpty()){QTest::qWait(100);QVERIFY(window->grabWindow().save(preview));}
        auto toggle=[&](const QString &name){auto *item=findVisualItem(window->contentItem(),name);if(!item)return false;QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,item->mapToScene({7,item->height()/2}).toPoint());return true;};
        QVERIFY(toggle("bucketAntialias"));QVERIFY(!client.bucketAntialias());QVERIFY(toggle("bucketContiguous"));QVERIFY(!client.bucketContiguous());QVERIFY(toggle("bucketAllLayers"));QVERIFY(client.bucketAllLayers());
        QVERIFY(toggle("bucketAntialias"));QVERIFY(toggle("bucketContiguous"));QVERIFY(toggle("bucketAllLayers"));
        auto *opacity=findVisualItem(window->contentItem(),"bucketOpacityInput");auto *tolerance=findVisualItem(window->contentItem(),"bucketTolerance");QVERIFY(opacity && tolerance);
        opacity->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_4);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QCOMPARE(client.bucketOpacity(),.4);QCOMPARE(client.brushOpacity(),1.);
        QVERIFY(tolerance->setProperty("value",32));QVERIFY(QMetaObject::invokeMethod(tolerance,"valueModified"));QCOMPARE(client.bucketTolerance(),32);
        client.setBucketTolerance(-1);QCOMPARE(client.bucketTolerance(),32);client.setBucketTolerance(256);QCOMPARE(client.bucketTolerance(),32);client.setBucketOpacity(2);QCOMPARE(client.bucketOpacity(),.4);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        client.setBrushColor(Qt::red);QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({8,8}));QTest::mouseMove(window,at({40,40}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({40,40}));
        QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),1,10000);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(!client.drawing());
        QCOMPARE(client.historyEntries().last().toMap().value("kind").toInt(),25);QCOMPARE(client.historyEntries().last().toMap().value("title").toString(),QStringLiteral("油漆桶"));QCOMPARE(client.historyEntries().last().toMap().value("icon").toString(),QStringLiteral("bucket"));
        const auto pixel=[&](int x,int y){const auto r=client.frameRegion();const auto &f=client.frame();if(f.isNull() || r.isEmpty())return QColor();const int px=qFloor((x+.5-r.x())*f.width()/r.width()),py=qFloor((y+.5-r.y())*f.height()/r.height());return px>=0 && py>=0 && px<f.width() && py<f.height()?f.pixelColor(px,py):QColor();};
        QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(pixel(63,63).alpha(),102);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(8,8).alpha(),0);
        client.activateEraser();QVERIFY(!client.bucketTool());client.activateBucket();QCOMPARE(client.bucketOpacity(),.4);QCOMPARE(client.bucketTolerance(),32);client.setBucketOpacity(1);client.setBucketTolerance(0);client.setBrushColor(Qt::blue);
        QPointingDevice pen("bucket pen",123,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto send=[&](QEvent::Type type,QPointF p){const auto local=at(p);QTabletEvent event(type,&pen,local,window->mapToGlobal(local),.7,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&event);return event.isAccepted();};
        QVERIFY(send(QEvent::TabletPress,{8,8}));QVERIFY(send(QEvent::TabletMove,{40,40}));QVERIFY(send(QEvent::TabletRelease,{40,40}));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(63,63),QColor(Qt::blue));
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());
        QQuickWindow floating;floating.resize(200,180);auto *floatingCanvas=new CanvasItem(floating.contentItem());floatingCanvas->setSize({200,180});floatingCanvas->setClient(&client);floating.show();floatingCanvas->actualSize();QTest::qWait(80);
        auto point=floatingCanvas->mapToScene(floatingCanvas->documentRect().topLeft()+QPointF(8,8)*floatingCanvas->zoom()).toPoint();QTest::mouseClick(&floating,Qt::LeftButton,Qt::NoModifier,point);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        floatingCanvas->setClient(nullptr);floating.hide();QCOMPARE(warnings,QStringList());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void wandOptionsSelectRealMaskOnceAcrossMouseTabletAndFloatingCanvas() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("wand.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        // Paint a round island: its true mask outline must exclude the bounding-box corner.
        client.setBrushRadius(12);client.setBrushColor(Qt::red);QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);QTest::keyClick(window,Qt::Key_W);QTRY_COMPARE(client.selectionTool(),5);
        auto *bar=findVisualItem(window->contentItem(),"wandOptionsBar");QVERIFY(bar && bar->isVisible());QVERIFY(!findVisualItem(window->contentItem(),"selectionOptionsBar")->isVisible());QCOMPARE(client.wandSampleSize(),1);QCOMPARE(client.wandTolerance(),10);QCOMPARE(client.wandOperation(),1);QVERIFY(client.wandAntialias() && client.wandContiguous() && !client.wandAllLayers());
        auto *sample=findVisualItem(window->contentItem(),"wandSampleSize");QVERIFY(sample);QCOMPARE(sample->property("count").toInt(),7);QVERIFY(QMetaObject::invokeMethod(sample,"activated",Q_ARG(int,6)));QCOMPARE(client.wandSampleSize(),101);client.setWandSampleSize(1);client.setWandSampleSize(2);QCOMPARE(client.wandSampleSize(),1);
        auto *tolerance=findVisualItem(window->contentItem(),"wandTolerance");QVERIFY(tolerance);QVERIFY(tolerance->setProperty("value",0));QVERIFY(QMetaObject::invokeMethod(tolerance,"valueModified"));QCOMPARE(client.wandTolerance(),0);client.setWandTolerance(256);QCOMPARE(client.wandTolerance(),0);
        const auto click=[&](const QString &name){auto *item=findVisualItem(window->contentItem(),name);if(!item)return false;QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,item->mapToScene({7,item->height()/2}).toPoint());return true;};
        QVERIFY(click("wandAntialias"));QVERIFY(!client.wandAntialias());QVERIFY(click("wandAntialias"));QVERIFY(click("wandContiguous"));QVERIFY(!client.wandContiguous());QVERIFY(click("wandContiguous"));QVERIFY(click("wandAllLayers"));QVERIFY(client.wandAllLayers());QVERIFY(click("wandAllLayers"));
        QVERIFY(click("wandMode:1"));QCOMPARE(client.wandOperation(),1);QVERIFY(click("wandMode:0"));QCOMPARE(client.wandOperation(),0);QCOMPARE(client.selectionOperation(),0);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({32,32}));QTest::mouseMove(window,at({2,2}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({2,2}));QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.selectionEnabled());QCOMPARE(client.historyEntries().last().toMap().value("title").toString(),QStringLiteral("魔棒"));QVERIFY(!client.drawing());
        auto *outline=window->findChild<SelectionOverlay*>("selectionOutline");QVERIFY(outline);QTRY_VERIFY(outline->documentPath().contains({32.5,32.5}));const auto maskStep=client.selectionSteps().first().toMap();QVERIFY(!outline->documentPath().contains({maskStep.value("x").toDouble()+.5,maskStep.value("y").toDouble()+.5}));QVERIFY(maskStep.value("mask").toByteArray().size()>0);
        const auto preview=qEnvironmentVariable("DRAWVERSE_WAND_PREVIEW");if(!preview.isEmpty()){QTest::qWait(80);QVERIFY(window->grabWindow().save(preview));}
        client.undo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(!client.selectionEnabled());
        QPointingDevice pen("wand pen",124,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        for(const auto type:{QEvent::TabletPress,QEvent::TabletMove,QEvent::TabletRelease}) {const auto local=at({32,32});QTabletEvent event(type,&pen,local,window->mapToGlobal(local),.2,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&event);QVERIFY(event.isAccepted());}
        QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());client.undo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        QQuickWindow floating;floating.resize(200,180);auto *floatingCanvas=new CanvasItem(floating.contentItem());floatingCanvas->setSize({200,180});floatingCanvas->setClient(&client);floating.show();floatingCanvas->actualSize();QTest::qWait(80);const auto point=floatingCanvas->mapToScene(floatingCanvas->documentRect().topLeft()+QPointF(32,32)*floatingCanvas->zoom()).toPoint();QTest::mouseClick(&floating,Qt::LeftButton,Qt::NoModifier,point);QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());
        floatingCanvas->setClient(nullptr);floating.hide();client.activateLasso();QCOMPARE(client.selectionTool(),3);client.activateWand();QCOMPARE(client.wandTolerance(),0);QCOMPARE(client.wandOperation(),0);QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void lassoPolygonAndMarqueeShareOptionsHistoryAndPersistence() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("selection.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        client.setSelectionTool(3);auto *options=findVisualItem(window->contentItem(),"selectionOptionsBar");QVERIFY(options);QTRY_VERIFY(options->isVisible());
        for(int i=0;i<4;++i){auto *button=findVisualItem(options,QString("selectionMode:%1").arg(i));QVERIFY(button);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,button->mapToScene({13,13}).toPoint());QCOMPARE(client.selectionOperation(),i);}
        auto *featherText=findVisualItem(options,"selectionFeatherText");QVERIFY(featherText);featherText->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_4);QTest::keyClick(window,Qt::Key_2);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(client.selectionFeather(),42.);
        auto *antialias=findVisualItem(options,"selectionAntialias");QVERIFY(antialias);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,antialias->mapToScene({6,6}).toPoint());QTRY_VERIFY(!client.selectionAntialias());
        client.setSelectionOperation(0);client.setSelectionFeather(3);canvas->forceActiveFocus();
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({16,16}));QTest::mouseMove(window,at({96,16}));QTest::mouseMove(window,at({16,96}));QVERIFY(canvas->selectionPreviewPoints().size()>=3);QCOMPARE(client.undoDepth(),0);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({16,16}));
        QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());auto step=client.selectionSteps().last().toMap();QCOMPARE(step.value("shape").toInt(),2);QCOMPARE(step.value("feather").toDouble(),3.);QVERIFY(!step.value("antialias").toBool());QCOMPARE(client.historyEntries().last().toMap().value("title").toString(),QString::fromUtf8("套索"));
        auto *outline=qobject_cast<SelectionOverlay*>(findVisualItem(window->contentItem(),"selectionOutline"));QVERIFY(outline);QTRY_VERIFY(outline->documentPath().contains({30,30}));QVERIFY(!outline->documentPath().contains({90,90}));
        auto *lassoButton=findVisualItem(window->contentItem(),"lassoTool");QVERIFY(lassoButton);
        QTest::mouseClick(window,Qt::RightButton,Qt::NoModifier,lassoButton->mapToScene({13,13}).toPoint());auto *variants=window->findChild<QObject*>("lassoVariantPanel");QVERIFY(variants);QTRY_VERIFY(variants->property("visible").toBool());auto *variantContent=qobject_cast<QQuickItem*>(variants->property("contentItem").value<QObject*>());QVERIFY(variantContent);auto *polygon=findVisualItem(variantContent,"variantTrigger:polygon");QVERIFY(polygon);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,polygon->mapToScene({20,15}).toPoint());QTRY_COMPARE(client.selectionTool(),4);QTRY_VERIFY(!variants->property("visible").toBool());
        canvas->forceActiveFocus();QCOMPARE(client.selectionFeather(),3.);QCOMPARE(client.selectionOperation(),0);QVERIFY(!client.selectionAntialias());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({32,32}));QVERIFY(!canvas->selectionPreviewPoints().isEmpty());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({100,32}));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({100,100}));QCOMPARE(client.undoDepth(),1);
        QTest::keyClick(window,Qt::Key_Backspace);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({100,100}));QTest::keyClick(window,Qt::Key_Return);
        QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(canvas->selectionPreviewPoints().isEmpty());QCOMPARE(client.historyEntries().last().toMap().value("kind").toInt(),24);QCOMPARE(client.selectionSteps().last().toMap().value("points").toList().size(),3);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({16,16}));QTest::keyClick(window,Qt::Key_Escape);QVERIFY(canvas->selectionPreviewPoints().isEmpty());QCOMPARE(client.undoDepth(),2);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({16,16}));client.activateBrush();QVERIFY(canvas->selectionPreviewPoints().isEmpty());client.activateLasso();QCOMPARE(client.selectionTool(),4);
        client.setSelectionTool(1);QCOMPARE(client.selectionFeather(),3.);client.setSelectionOperation(1);QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({4,4}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({16,16}));QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());step=client.selectionSteps().last().toMap();QCOMPARE(step.value("operation").toInt(),1);QCOMPARE(step.value("feather").toDouble(),3.);
        const auto saved=client.selectionSteps();QSignalSpy files(&client,&PaintCoreClient::fileFinished);const auto path=QUrl::fromLocalFile(temp.filePath("lasso.ora"));QVERIFY(client.saveDocument(path));QTRY_COMPARE(files.size(),1);QVERIFY(files.last()[0].toBool());QVERIFY(client.clearSelection());QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.openDocument(path));QTRY_COMPARE(files.size(),2);QVERIFY(files.last()[0].toBool());QTRY_VERIFY(client.ready());QCOMPARE(client.selectionSteps(),saved);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void polygonDoubleClickClosureAndTabletFreehandUseSameInputPath() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("selection.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        client.setSelectionTool(4);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({20,20}));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({100,20}));QTest::mouseDClick(window,Qt::LeftButton,Qt::NoModifier,at({20,100}));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().last().toMap().value("points").toList().size(),3);
        // Space changes only the viewport, never completes a partially collected polygon.
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({40,40}));const auto points=canvas->selectionPreviewPoints();QTest::keyPress(window,Qt::Key_Space);QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({60,60}));QTest::mouseMove(window,at({65,65}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({65,65}));QTest::keyRelease(window,Qt::Key_Space);QCOMPARE(client.undoDepth(),1);QCOMPARE(canvas->selectionPreviewPoints().size(),points.size());QTest::keyClick(window,Qt::Key_Escape);canvas->actualSize();
        client.setSelectionTool(3);QPointingDevice pen("test pen",321,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto send=[&](QEvent::Type type,QPointF p){const QPointF local=at(p);QTabletEvent event(type,&pen,local,window->mapToGlobal(local.toPoint()),.7,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&event);return event.isAccepted();};
        QVERIFY(send(QEvent::TabletPress,{24,24}));QVERIFY(send(QEvent::TabletMove,{104,24}));QVERIFY(send(QEvent::TabletMove,{24,104}));QVERIFY(send(QEvent::TabletRelease,{24,24}));QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.historyEntries().last().toMap().value("kind").toInt(),21);
        client.setSelectionTool(4);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({20,20}));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({100,20}));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({20,100}));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({20,20}));QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(canvas->selectionPreviewPoints().isEmpty());
        client.setSelectionTool(4);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({20,20}));QTest::keyClick(window,Qt::Key_Return);QCOMPARE(client.undoDepth(),3);QVERIFY(canvas->selectionPreviewPoints().isEmpty());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void selectionsDragCombineUndoConstrainAndPersist() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("selection.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());QVERIFY(!findVisualItem(window->contentItem(),"selectionOutline"));auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(100);
        const auto position=[&](QPointF point){return canvas->mapToScene(canvas->documentRect().topLeft()+point*canvas->zoom()).toPoint();};
        QTest::keyClick(window,Qt::Key_M);QTRY_COMPARE(client.selectionTool(),1);
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,position({16,16}));QTest::mouseMove(window,position({80,80}));QVERIFY(!canvas->selectionPreview().isEmpty());QCOMPARE(client.undoDepth(),0);QVERIFY(!client.selectionEnabled());QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,position({80,80}));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.selectionEnabled());QCOMPARE(client.selectionSteps().size(),1);
        QTest::keyClick(window,Qt::Key_M,Qt::ShiftModifier);QTRY_COMPARE(client.selectionTool(),2);
        QTest::mousePress(window,Qt::LeftButton,Qt::AltModifier,position({32,32}));QTest::mouseMove(window,position({64,64}));QTest::mouseRelease(window,Qt::LeftButton,Qt::AltModifier,position({64,64}));QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().last().toMap().value("operation").toInt(),2);QCOMPARE(client.selectionSteps().last().toMap().value("shape").toInt(),1);
        const auto outline=[&](){return qobject_cast<SelectionOverlay*>(findVisualItem(window->contentItem(),"selectionOutline"));};QVERIFY(outline());QTRY_VERIFY(outline() && outline()->documentPath().contains({24,48}));QVERIFY(!outline()->documentPath().contains({48,48}));QVERIFY(!outline()->documentPath().contains({100,48}));
        QTest::mousePress(window,Qt::LeftButton,Qt::ShiftModifier,position({96,16}));QTest::mouseMove(window,position({120,40}));QTest::mouseRelease(window,Qt::LeftButton,Qt::ShiftModifier,position({120,40}));QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().last().toMap().value("operation").toInt(),1);QTRY_VERIFY(outline() && outline()->documentPath().contains({108,28}));
        QTest::mousePress(window,Qt::LeftButton,Qt::ShiftModifier|Qt::AltModifier,position({8,8}));QTest::mouseMove(window,position({50,100}));QTest::mouseRelease(window,Qt::LeftButton,Qt::ShiftModifier|Qt::AltModifier,position({50,100}));QTRY_COMPARE(client.undoDepth(),4);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().last().toMap().value("operation").toInt(),3);client.undo();QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,position({8,8}));QTest::mouseMove(window,position({120,120}));QTest::keyClick(window,Qt::Key_Escape);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,position({120,120}));QVERIFY(canvas->selectionPreview().isEmpty());QCOMPARE(client.undoDepth(),3);
        QTest::keyClick(window,Qt::Key_B);QTRY_COMPARE(client.selectionTool(),0);client.setBrushRadius(5);client.setBrushColor(Qt::black);InputSample sample;sample.position={8,48};QVERIFY(client.beginStroke(sample));sample.position={120,48};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),4);QTRY_VERIFY(!client.layerEditBusy());canvas->setClient(nullptr);client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frame().size()==QSize(128,128) && client.frameRevision()>=client.revision());QVERIFY(client.frame().pixelColor(24,48).alpha()>0);QCOMPARE(client.frame().pixelColor(48,48).alpha(),0);QCOMPARE(client.frame().pixelColor(100,48).alpha(),0);canvas->setClient(&client);canvas->actualSize();
        const auto savedSteps=client.selectionSteps();QSignalSpy files(&client,&PaintCoreClient::fileFinished);const auto path=QUrl::fromLocalFile(temp.filePath("selection.ora"));QVERIFY(client.saveDocument(path));QTRY_COMPARE(files.size(),1);QVERIFY(files.last()[0].toBool());QVERIFY(client.clearSelection());QTRY_VERIFY(!client.selectionEnabled());QTRY_VERIFY(!client.layerEditBusy());client.undo();QTRY_VERIFY(client.selectionEnabled());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps(),savedSteps);
        const auto generation=client.generation();QVERIFY(client.openDocument(path));QTRY_COMPARE(files.size(),2);QVERIFY(files.last()[0].toBool());QTRY_VERIFY(client.generation()>generation);QTRY_VERIFY(client.ready());QCOMPARE(client.selectionSteps(),savedSteps);QCOMPARE(client.undoDepth(),0);
        workspace.floatToolStrip(250,150);QTRY_COMPARE(applicationWindows().size(),2);QQuickWindow *floating=nullptr;for(auto *w:applicationWindows())if(w!=window)floating=qobject_cast<QQuickWindow*>(w);QVERIFY(floating);auto *tool=findVisualItem(floating->contentItem(),"selectionTool");QVERIFY(tool);QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,tool->mapToScene({tool->width()/2,tool->height()/2}).toPoint());QTRY_COMPARE(client.selectionTool(),2);
        window->requestActivate();canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().size(),1);QTest::keyClick(window,Qt::Key_I,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(outline() && outline()->documentPath().isEmpty());QVERIFY(client.selectionEnabled());
        QTest::keyClick(window,Qt::Key_D,Qt::ControlModifier);QTRY_VERIFY(!client.selectionEnabled());QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!findVisualItem(window->contentItem(),"selectionOutline"));
        client.setSelectionTool(2);QPointingDevice pen("Selection tablet",1001,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,2);
        const QPointF start=position({20,20}),end=position({100,100});QTabletEvent press(QEvent::TabletPress,&pen,start,window->mapToGlobal(start.toPoint()),.6,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(window,&press);QVERIFY(press.isAccepted());QTabletEvent release(QEvent::TabletRelease,&pen,end,window->mapToGlobal(end.toPoint()),0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(window,&release);QVERIFY(release.isAccepted());QTRY_VERIFY(client.selectionEnabled());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().size(),1);QCOMPARE(client.selectionSteps().first().toMap().value("shape").toInt(),1);
        const QPoint center=position({60,60});const auto before=canvas->documentPoint(canvas->mapFromScene(center));QWheelEvent wheel(center,window->mapToGlobal(center),{},QPoint(0,120),Qt::NoButton,Qt::NoModifier,Qt::NoScrollPhase,false);QCoreApplication::sendEvent(window,&wheel);QVERIFY(QLineF(before,canvas->documentPoint(canvas->mapFromScene(center))).length()<.001);QTRY_VERIFY(outline() && outline()->documentPath().contains({60,60}));QVERIFY(!outline()->documentPath().contains({20,20}));
        const auto preview=qEnvironmentVariable("DRAWVERSE_SELECTION_PREVIEW");if(!preview.isEmpty()){QTest::qWait(200);QVERIFY(window->grabWindow().save(preview));}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void toolsDockAlongWholeColumnAndStayResizableAfterPanelLeaves() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.groupForPanel("color"),layers=workspace.groupForPanel("layers");workspace.detachGroup(color);QVERIFY(workspace.dockToolStrip("drawverse-tools-v1","floating",color,"left"));
        QQuickWindow *mixed=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+color)mixed=qobject_cast<QQuickWindow*>(w);return mixed!=nullptr;}());mixed->resize(420,550);QTest::qWait(30);
        workspace.detachGroup(color);QTRY_COMPARE(mixed->objectName(),QString("floatingToolStrip"));QTRY_COMPARE(mixed->width(),38);QCOMPARE(mixed->maximumWidth(),74);QCOMPARE(mixed->minimumWidth(),38);
        mixed->resize(74,600);QTRY_COMPARE(mixed->width(),74);QTRY_COMPARE(workspace.toolStripWidth(),72);mixed->resize(38,450);QTRY_COMPARE(workspace.toolStripWidth(),36);QTRY_COMPARE(mixed->height(),450);
        QVERIFY(workspace.dockPayload(QString::fromUtf8(QJsonDocument(QJsonObject{{"group",color},{"whole",true}}).toJson()),"right",layers,"after"));QCOMPARE(workspace.columnGroups(layers),QStringList({layers,color}));QTest::qWait(30);
        QVERIFY(workspace.dockToolStrip("drawverse-tools-v1","right",layers,"left"));QTRY_VERIFY(!workspace.toolsFloating());QTest::qWait(30);auto *tools=findVisualItem(main->contentItem(),"dockTile:__toolstrip"),*upper=findVisualItem(main->contentItem(),"dockTile:"+layers);QVERIFY(tools && upper);QCOMPARE(tools->y(),upper->y());QVERIFY(tools->height()>upper->height()+100);
        workspace.setToolStripWidth(999);QTRY_COMPARE(tools->width(),qreal(72));workspace.setToolStripWidth(1);QTRY_COMPARE(tools->width(),qreal(36));workspace.setToolStripWidth(55);QTRY_COMPARE(tools->width(),qreal(55));const auto preview=qEnvironmentVariable("DRAWVERSE_WORKSPACE_TOOLS_PREVIEW");if(!preview.isEmpty()){QTest::qWait(80);QVERIFY(main->grabWindow().save(preview+".main.png"));}workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));QCOMPARE(restored.toolStripWidth(),55);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void collapsedExpandButtonDragsWithoutExpandingAndPixelsStayInPlace() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->zoomBy(2);
        const auto origin=canvas->mapToScene(canvas->documentRect().topLeft());const auto zoom=canvas->zoom();const auto left=workspace.groupForPanel("brush"),right=workspace.groupForPanel("color");
        for(const auto &id:QStringList{right,left})for(const bool folded:{true,false,true}) {workspace.setColumnCollapsed(id,folded);QTRY_VERIFY(QLineF(origin,canvas->mapToScene(canvas->documentRect().topLeft())).length()<.01);QCOMPARE(canvas->zoom(),zoom);}
        QQuickWindow *frameWindow=nullptr;for(auto *w:QGuiApplication::allWindows())if(w->objectName()=="applicationOutlineWindow")frameWindow=qobject_cast<QQuickWindow*>(w);QVERIFY(frameWindow);
        auto *outline=findVisualItem(frameWindow->contentItem(),"applicationWindowOutline");QVERIFY(outline);auto stroke=QQmlProperty::read(outline,"border.color").value<QColor>();QCOMPARE(stroke.rgb(),QColor("#63666b").rgb());QVERIFY(std::abs(stroke.alphaF()-.5)<.01);QCOMPARE(frameWindow->geometry(),main->geometry().adjusted(-1,-1,1,1));
        QVERIFY(frameWindow->flags().testFlag(Qt::WindowTransparentForInput));main->showMaximized();QTRY_VERIFY(!frameWindow->isVisible());main->showNormal();QTRY_VERIFY(frameWindow->isVisible());QTest::qWait(50);

        auto *bar=findVisualItem(main->contentItem(),"brushOptionsBar");QVERIFY(bar);QCOMPARE(bar->height(),qreal(36));auto *documents=main->findChild<DocumentManager*>("documentManager");QVERIFY(documents);auto *tab=findVisualItem(main->contentItem(),"documentTab:"+documents->activeId());QVERIFY(tab);QCOMPARE(tab->property("radius").toReal(),qreal(10));QCOMPARE(tab->y(),qreal(3));QCOMPARE(tab->property("color").value<QColor>(),bar->property("color").value<QColor>());QCOMPARE(tab->mapToScene({0,0}).x()-findVisualItem(main->contentItem(),"documentTitleRow:main")->mapToScene({0,0}).x(),qreal(8));
        auto *grip=findVisualItem(main->contentItem(),"railExpandGrip:"+right);QVERIFY(grip);const auto at=grip->mapToScene({20,5}).toPoint();QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,at);QVERIFY(grip->property("pressed").toBool());QTest::mouseMove(main,at+QPoint(-80,30));QTRY_VERIFY(workspace.dragging());QVERIFY(workspace.groupDefinition(right).value("icons").toBool());QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+right)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());QTest::keyClick(floating,Qt::Key_Escape);QTRY_VERIFY(!workspace.dragging());QTest::mouseRelease(main,Qt::LeftButton,Qt::NoModifier,at);QTRY_VERIFY(workspace.floatingWindows().isEmpty());QVERIFY(workspace.groupDefinition(right).value("icons").toBool());
        grip=findVisualItem(main->contentItem(),"railExpandGrip:"+right);QVERIFY(grip);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,grip->mapToScene({20,5}).toPoint());QTRY_VERIFY(!workspace.groupDefinition(right).value("icons").toBool());QTRY_VERIFY(QLineF(origin,canvas->mapToScene(canvas->documentRect().topLeft())).length()<.01);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void colorClicksAndCapsLockControlBrushOutline() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.setBrushColor(QColor("#112233"));client.setSecondaryBrushColor(QColor("#abcdef"));auto *swatch=findVisualItem(main->contentItem(),"foregroundColorSwatch");QVERIFY(swatch);const auto click=swatch->mapToScene({12,12}).toPoint();QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,click);QCOMPARE(client.brushColor(),QColor("#abcdef"));QCOMPARE(client.secondaryBrushColor(),QColor("#112233"));
        QTest::mouseDClick(main,Qt::LeftButton,Qt::NoModifier,click);QVERIFY(!findVisualItem(main->contentItem(),"brushColorHex"));

        auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->forceActiveFocus();canvas->actualSize();client.setBrushRadius(15);QHoverEvent hover(QEvent::HoverMove,{100,100},canvas->mapToGlobal({100,100}),{90,90});QCoreApplication::sendEvent(canvas,&hover);const bool originallyVisible=canvas->brushCursorVisible();if(!originallyVisible)QTest::keyClick(main,Qt::Key_CapsLock);QTRY_VERIFY(canvas->brushCursorVisible());QCOMPARE(canvas->brushCursorRect(),QRectF(85,85,30,30));QCOMPARE(canvas->cursor().shape(),Qt::BlankCursor);auto *outline=findVisualItem(main->contentItem(),"brushCursorOutline");QVERIFY(outline && outline->isVisible());
        canvas->zoomBy(2);QCOMPARE(canvas->brushCursorRect().width(),qreal(60));client.setEraser(true);client.setBrushRadius(6);QCOMPARE(canvas->brushCursorRect().width(),qreal(24));client.setEraser(false);QCOMPARE(client.brushRadius(),qreal(15));QTest::keyClick(main,Qt::Key_CapsLock);QVERIFY(!canvas->brushCursorVisible());QCOMPARE(canvas->cursor().shape(),Qt::CrossCursor);QTest::keyClick(main,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());client.setMoveTool(true);QVERIFY(!canvas->brushCursorVisible());QCOMPARE(canvas->cursor().shape(),Qt::SizeAllCursor);client.setEraser(false);QVERIFY(canvas->brushCursorVisible());if(!originallyVisible)QTest::keyClick(main,Qt::Key_CapsLock);
        const auto brushGroup=workspace.groupForPanel("brush");workspace.setActive(brushGroup,"brush");QQuickItem *preset=nullptr;QTRY_VERIFY((preset=findVisualItem(main->contentItem(),"brushPreset:round-fine")));QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,preset->mapToScene({20,20}).toPoint());preset->forceActiveFocus();QVERIFY(!canvas->hasActiveFocus());
        const auto oldOrigin=canvas->mapToScene(canvas->documentRect().topLeft());const auto center=canvas->mapToScene({canvas->width()/2,canvas->height()/2}).toPoint();QTest::keyPress(main,Qt::Key_Space);QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,center);QTest::mouseMove(main,center+QPoint(30,20));QTest::mouseRelease(main,Qt::LeftButton,Qt::NoModifier,center+QPoint(30,20));QTest::keyRelease(main,Qt::Key_Space);QTRY_VERIFY(QLineF(canvas->mapToScene(canvas->documentRect().topLeft()),oldOrigin+QPointF(30,20)).length()<.01);QCOMPARE(client.undoDepth(),0);
        canvas->fitToView();const auto outside=canvas->mapToScene(canvas->documentRect().bottomRight()+QPointF(4,6))*main->devicePixelRatio();QTRY_COMPARE(main->grabWindow().pixelColor(outside.toPoint()),QColor("#17191c"));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void topMenusUseNativeWindowsAboveFloatingPanels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());const auto group=workspace.groupForPanel("color");workspace.detachGroup(group);QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+group)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());
        auto *menu=main->findChild<QObject*>("fileMenuPopup");QVERIFY(menu);QVERIFY(QMetaObject::invokeMethod(menu,"open"));QTRY_VERIFY(menu->property("opened").toBool());auto *content=qobject_cast<QQuickItem*>(menu->property("contentItem").value<QObject*>());QVERIFY(content);QQuickWindow *popup=nullptr;QTRY_VERIFY((popup=content->window()) && popup!=main);floating->raise();
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {QTRY_VERIFY((GetWindowLongPtr(reinterpret_cast<HWND>(popup->winId()),GWL_EXSTYLE)&WS_EX_TOPMOST)!=0);HWND front=nullptr;for(auto w=GetTopWindow(nullptr);w;w=GetWindow(w,GW_HWNDNEXT))if(w==reinterpret_cast<HWND>(popup->winId()) || w==reinterpret_cast<HWND>(floating->winId())){front=w;break;}QCOMPARE(front,reinterpret_cast<HWND>(popup->winId()));}
#endif
        QVERIFY(QMetaObject::invokeMethod(menu,"close"));QTRY_VERIFY(!menu->property("opened").toBool());QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void brushPresetsKeepIndependentSettingsAndRejectStalePreviews() {
        BrushLibrary library;QCOMPARE(library.radius(),qreal(12));library.setRadius(22);library.setOpacity(.4);library.setSpacing(.3);QVERIFY(library.select("round-fine"));QCOMPARE(library.radius(),qreal(22));QCOMPARE(library.opacity(),qreal(1));QVERIFY(library.select("round-pressure"));QCOMPARE(library.radius(),qreal(22));QCOMPARE(library.spacing(),qreal(.3));
        QSignalSpy requests(&library,&BrushLibrary::previewRequested);library.requestPreview(library.selectedId());QCOMPARE(requests.size(),1);QCOMPARE(requests.first()[2].toReal(),qreal(24));const auto oldToken=requests.first()[1].toULongLong();library.setRadius(24);QCOMPARE(requests.size(),1);library.setSpacing(.4);library.acceptPreview(library.selectedId(),oldToken,"stale");QVERIFY(library.selectedPreview().isEmpty());library.requestPreview(library.selectedId());QCOMPARE(requests.size(),2);library.acceptPreview(library.selectedId(),requests.last()[1].toULongLong(),"fresh");QCOMPARE(library.selectedPreview(),QString("fresh"));
        const auto copy=library.saveCopy(QStringLiteral("自定义圆笔"));QVERIFY(!copy.isEmpty());QCOMPARE(library.selectedId(),copy);QVERIFY(library.rename(copy,QStringLiteral("自定义圆笔二")));QVERIFY(!library.rename(copy," "));QVERIFY(!library.remove("round-pressure"));
        BrushLibrary restored;QVERIFY(restored.restore(library.snapshot()));QCOMPARE(restored.selectedId(),copy);QCOMPARE(restored.radius(),qreal(24));QCOMPARE(restored.opacity(),qreal(.4));QCOMPARE(restored.presets().size(),5);
        auto invalid=QJsonDocument::fromJson(library.snapshot()).object();auto rows=invalid.value("presets").toArray();auto row=rows.first().toObject();row["radius"]=-1;rows[0]=row;invalid["presets"]=rows;QVERIFY(!restored.restore(QJsonDocument(invalid).toJson()));QCOMPARE(restored.radius(),qreal(24));QVERIFY(!restored.restore("not json"));library.setRadius(std::numeric_limits<double>::quiet_NaN());QCOMPARE(library.radius(),qreal(24));
        library.setEraser(true);QCOMPARE(library.radius(),qreal(12));library.setRadius(40);QVERIFY(library.select("round-wide"));QCOMPARE(library.radius(),qreal(40));library.setEraser(false);QCOMPARE(library.radius(),qreal(24));QVERIFY(library.select(copy));
        BrushLibrary both;QVERIFY(both.restore(library.snapshot()));QCOMPARE(both.radius(),qreal(24));both.setEraser(true);QCOMPARE(both.radius(),qreal(40));
        auto legacy=QJsonDocument::fromJson(library.snapshot()).object();legacy.remove("toolRadii");BrushLibrary old;QVERIFY(old.restore(QJsonDocument(legacy).toJson()));
        QVERIFY(restored.remove(copy));QCOMPARE(restored.selectedId(),QString("round-pressure"));QCOMPARE(restored.presets().size(),4);QVERIFY(!restored.select("missing"));
        library.retryPreview();const auto failedToken=requests.last()[1].toULongLong();library.acceptPreview(library.selectedId(),failedToken,{});QCOMPARE(library.previewMessage(),QStringLiteral("预览暂不可用"));const auto count=requests.size();library.requestPreview(library.selectedId());QCOMPARE(requests.size(),count);library.retryPreview();QCOMPARE(requests.size(),count+1);
    }
    void brushLibrarySettingsAndEnginePreviewsShareSelectionAcrossWindows() {
        QTemporaryDir temp;const auto preferences=temp.filePath("storage.ini");PaintCoreClient client(nullptr,preferences);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();QVERIFY(library);const auto revision=client.revision();const int undo=client.undoDepth();const bool modified=client.modified();
        const auto group=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachPanel(group,"brush");QTRY_COMPARE(applicationWindows().size(),2);QQuickWindow *floating=nullptr;for(auto *w:applicationWindows())if(w!=window)floating=qobject_cast<QQuickWindow*>(w);QVERIFY(floating);floating->resize(190,500);
        QTRY_VERIFY2(findVisualItem(floating->contentItem(),"brushPreset:round-fine"),qPrintable(warnings.join('\n')));auto *fine=findVisualItem(floating->contentItem(),"brushPreset:round-fine");QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,fine->mapToScene({fine->width()/2,fine->height()/2}).toPoint());QTRY_COMPARE(library->selectedId(),QString("round-fine"));auto *label=findVisualItem(window->contentItem(),"selectedBrushName");QVERIFY(label);QTRY_COMPARE(label->property("text").toString(),QStringLiteral("细线圆笔"));
        auto *size=findVisualItem(window->contentItem(),"settingsBrushSizeText");QVERIFY(size);window->requestActivate();size->forceActiveFocus();QTRY_VERIFY(size->hasActiveFocus());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,size->mapToScene({size->width()/2,size->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_4);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(client.brushRadius(),qreal(20));
        QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto image=[](const QString &url){QImage result;result.loadFromData(QByteArray::fromBase64(url.mid(url.indexOf(',')+1).toLatin1()));return result;};const auto opaque=image(library->selectedPreview());QVERIFY(!opaque.isNull());QCOMPARE(opaque.size(),QSize(240,60));QVERIFY(opaque.pixelColor(0,0).alpha()==0);int alpha=0;for(int y=0;y<60;++y)for(int x=0;x<240;++x)alpha+=opaque.pixelColor(x,y).alpha();QVERIFY(alpha>1000);
        const auto fixedPreview=library->selectedPreview();library->setRadius(100);QCOMPARE(library->selectedPreview(),fixedPreview);library->setRadius(20);QCOMPARE(library->selectedPreview(),fixedPreview);
        library->setOpacity(.15);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto transparent=image(library->selectedPreview());QVERIFY(!transparent.isNull());int fadedAlpha=0;for(int y=0;y<60;++y)for(int x=0;x<240;++x)fadedAlpha+=transparent.pixelColor(x,y).alpha();QVERIFY(fadedAlpha<alpha);QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),undo);QCOMPARE(client.modified(),modified);
        library->setSpacing(.6);QCOMPARE(client.brushSpacing(),qreal(.6));QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(image(library->selectedPreview())!=transparent);QVERIFY(library->select("round-pressure"));QCOMPARE(client.brushRadius(),qreal(20));QVERIFY(library->select("round-fine"));QCOMPARE(client.brushRadius(),qreal(20));QCOMPARE(client.brushOpacity(),qreal(.15));
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());client.setBrushColor(Qt::black);InputSample sample;sample.position={20,64};sample.pressure=.6f;sample.tool=1;QVERIFY(client.beginStroke(sample));sample.position={100,64};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());auto *previewCanvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(previewCanvas);previewCanvas->setClient(nullptr);client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,128));const int paintedAlpha=client.frame().pixelColor(60,64).alpha();QVERIFY(paintedAlpha>0 && paintedAlpha<250);previewCanvas->setClient(&client);
        const auto copy=library->saveCopy(QStringLiteral("保留的画笔"));QVERIFY(!copy.isEmpty());QTRY_COMPARE(label->property("text").toString(),QStringLiteral("保留的画笔"));
        const auto preview=qEnvironmentVariable("DRAWVERSE_BRUSH_PREVIEW");if(!preview.isEmpty()){QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);floating->resize(250,560);QTest::qWait(250);QVERIFY(floating->grabWindow().save(preview+".library.png"));auto settingsGroup=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(settingsGroup);QTRY_COMPARE(applicationWindows().size(),3);for(auto *w:applicationWindows())if(auto *q=qobject_cast<QQuickWindow*>(w))if(findVisualItem(q->contentItem(),"selectedBrushName")){q->resize(280,440);QTest::qWait(200);QVERIFY(q->grabWindow().save(preview+".settings.png"));}}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reloaded(nullptr,preferences);QTRY_VERIFY(reloaded.ready());QTRY_COMPARE(reloaded.brushLibrary()->selectedId(),copy);QCOMPARE(reloaded.brushRadius(),qreal(20));QCOMPARE(reloaded.brushSpacing(),qreal(.6));QSignalSpy stoppedAgain(&reloaded,&PaintCoreClient::stopped);reloaded.shutdown();QTRY_COMPARE(stoppedAgain.size(),1);
    }
    void toolStripFloatsDocksPersistsAndTabsJoinContent() {
        QTemporaryDir temp;const auto path=temp.filePath("tools.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());QCOMPARE(applicationWindows().size(),1);
        const auto group=workspace.rightGroups().last().toMap().value("id").toString();auto *panel=findVisualItem(window->contentItem(),"dockGroup:"+group);QVERIFY(panel);
        auto *selected=findVisualItem(panel,"panelTab:layers");auto *inactive=findVisualItem(panel,"panelTab:history");QVERIFY(selected && inactive);
        QCOMPARE(selected->property("color"),panel->property("color"));QVERIFY(inactive->property("color").value<QColor>().lightness()<selected->property("color").value<QColor>().lightness());
        workspace.setActive(group,"history");QTRY_COMPARE(inactive->property("color"),panel->property("color"));QVERIFY(selected->property("color").value<QColor>().lightness()<inactive->property("color").value<QColor>().lightness());workspace.setActive(group,"layers");
        QVERIFY(!workspace.dockToolStrip("invalid"));QVERIFY(!workspace.toolsFloating());
        auto *grip=findVisualItem(window->contentItem(),"toolStripGrip");QVERIFY(grip);QTest::mouseDClick(window,Qt::LeftButton,Qt::NoModifier,grip->mapToScene({grip->width()/2,grip->height()/2}).toPoint());QTRY_VERIFY(workspace.toolsFloating());QTRY_COMPARE(applicationWindows().size(),2);
        QQuickWindow *tools=nullptr;for(auto *w:applicationWindows())if(w->objectName()=="floatingToolStrip")tools=qobject_cast<QQuickWindow*>(w);QVERIFY(tools);QCOMPARE(tools->width(),38);
        // Floating recreates the strip; the docked grip can already be destroyed.
        auto *floatingGrip=findVisualItem(tools->contentItem(),"toolStripGrip");QVERIFY(floatingGrip);
        for(auto *object:floatingGrip->findChildren<QObject*>())QVERIFY(!object->property("text").toString().contains(QStringLiteral("拖动工具条")));
        auto *eraser=findVisualItem(tools->contentItem(),"eraserTool");QVERIFY(eraser);QTest::mouseClick(tools,Qt::LeftButton,Qt::NoModifier,eraser->mapToScene({13,13}).toPoint());QTRY_VERIFY(client.eraser());
        auto *brush=findVisualItem(tools->contentItem(),"brushTool");QVERIFY(brush);QTest::mouseClick(tools,Qt::LeftButton,Qt::NoModifier,brush->mapToScene({13,13}).toPoint());QTRY_VERIFY(!client.eraser());
        tools->setPosition(window->x()+70,window->y()+130);QTRY_COMPARE(workspace.toolStripY(),tools->y());workspace.saveLayout();WorkspaceManager restored(path);QVERIFY(restored.toolsFloating());QCOMPARE(restored.toolStripX(),tools->x());QCOMPARE(restored.toolStripY(),tools->y());
        auto drop=[&](Qt::KeyboardModifiers modifiers,const QByteArray &data){QMimeData mime;mime.setData("application/x-drawverse-tool-strip",data);const QPoint point(12,100);QDragEnterEvent enter(point,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(window,&enter);QDragMoveEvent move(point,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(window,&move);QDropEvent end(point,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(window,&end);return end.isAccepted();};
#ifdef Q_OS_MACOS
        const auto suppress=Qt::MetaModifier;
#else
        const auto suppress=Qt::ControlModifier;
#endif
        QVERIFY(!drop(suppress,"drawverse-tools-v1"));QVERIFY(workspace.toolsFloating());QVERIFY(!drop(Qt::NoModifier,"bad"));QVERIFY(workspace.toolsFloating());QVERIFY(drop(Qt::NoModifier,"drawverse-tools-v1"));QTRY_VERIFY(!workspace.toolsFloating());QTRY_COMPARE(applicationWindows().size(),1);
        workspace.floatToolStrip(window->x()+90,window->y()+150);QTRY_COMPARE(applicationWindows().size(),2);for(auto *w:applicationWindows())if(w->objectName()=="floatingToolStrip")w->close();QTRY_VERIFY(!workspace.toolsFloating());QTRY_COMPARE(applicationWindows().size(),1);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void defaultLayoutRestoresTwoDockedColumnsAndMenuEntry() {
        QTemporaryDir temp;const auto path=temp.filePath("default.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        QCOMPARE(workspace.leftGroups().size(),1);QCOMPARE(workspace.rightGroups().size(),2);QVERIFY(workspace.floatingWindows().isEmpty());
        workspace.detachGroup(workspace.leftGroups().first().toMap().value("id").toString());QTRY_COMPARE(applicationWindows().size(),2);
        QObject *action=nullptr;for(auto *object:window->findChildren<QObject*>()) {
            const auto text=object->property("text").toString();QVERIFY(text!=QStringLiteral("参考图布局"));QVERIFY(text!=QStringLiteral("双列停靠布局"));
            if(object->objectName()=="defaultLayoutAction")action=object;
        }
        QVERIFY(action);QVERIFY(QMetaObject::invokeMethod(action,"triggered"));QTRY_COMPARE(applicationWindows().size(),1);
        QCOMPARE(workspace.leftGroups().size(),1);QCOMPARE(workspace.rightGroups().size(),2);QVERIFY(!workspace.leftCollapsed());QVERIFY(!workspace.rightCollapsed());
        workspace.saveLayout();WorkspaceManager restored(path);QCOMPARE(restored.leftGroups().size(),1);QCOMPARE(restored.rightGroups().size(),2);QVERIFY(restored.floatingWindows().isEmpty());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void workspacePlacementPersistenceAndHiddenPanels() {
        QTemporaryDir temp;const auto path=temp.filePath("workspace-v3.ini");WorkspaceManager workspace(path);
        const auto first=workspace.rightGroups().first().toMap().value("id").toString();
        const auto second=workspace.rightGroups().last().toMap().value("id").toString();
        auto data=[](const QString &group,const QString &panel,bool whole=false){return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",group},{"panel",panel},{"whole",whole}}).toJson());};
        auto tabs=[&](const QString &group){return workspace.groupDefinition(group).value("panels").toStringList();};
        const auto custom=workspace.addCustomPanel("Custom colors","palette");QVERIFY(!custom.isEmpty());
        QVERIFY(workspace.dockPayload(data(first,custom),"right",first,"merge","color"));QCOMPARE(tabs(first).first(),custom);
        QVERIFY(!workspace.dockPayload(data(first,custom),"left",first));QCOMPARE(tabs(first).first(),custom);
        QVERIFY(!workspace.dockPayload(data(first,custom),"right",first,"merge","unknown"));
        QVERIFY(workspace.dockPayload(data(first,custom),"right",second,"before"));
        QCOMPARE(workspace.rightGroups().size(),3);const auto inserted=workspace.rightGroups()[1].toMap().value("id").toString();QCOMPARE(tabs(inserted),QStringList{custom});
        QVERIFY(workspace.dockPayload(data(inserted,{},true),"right",second,"after"));QCOMPARE(workspace.rightGroups().last().toMap().value("id").toString(),inserted);
        QVERIFY(workspace.dockPayload(data(inserted,custom),"right",second,"merge","history"));QVERIFY(tabs(second).indexOf(custom)<tabs(second).indexOf("history"));
        QSignalSpy structural(&workspace,&WorkspaceManager::groupsChanged);
        workspace.setActive(second,"history");QCOMPARE(structural.size(),0);workspace.setColumnCollapsed(second,true);
        workspace.setLeftDockWidth(274);workspace.setRightDockWidth(354);workspace.setRightCollapsed(true);workspace.updateDockHeight(first,279);
        workspace.hidePanel(second,custom);QVERIFY(!workspace.visiblePanels().contains(custom));QVERIFY(workspace.allPanels().contains(custom));
        workspace.saveLayout();WorkspaceManager reopened(path);QCOMPARE(reopened.leftDockWidth(),274);QCOMPARE(reopened.rightDockWidth(),354);QVERIFY(reopened.rightCollapsed());
        QVERIFY(reopened.groupDefinition(second).value("icons").toBool());QCOMPARE(reopened.groupDefinition(second).value("active").toString(),QString("history"));
        QCOMPARE(reopened.groupDefinition(first).value("dockHeight").toInt(),279);QVERIFY(!reopened.visiblePanels().contains(custom));
        reopened.showPanel(custom);QVERIFY(reopened.visiblePanels().contains(custom));QVERIFY(reopened.rightCollapsed());
        const auto left=reopened.leftGroups().first().toMap().value("id").toString();reopened.detachGroup(left);QCOMPARE(reopened.floatingGroups().size(),1);reopened.returnGroup(left);QCOMPARE(reopened.leftGroups().size(),1);
        QStringList visible;for(const auto &v:reopened.leftGroups()+reopened.rightGroups()+reopened.floatingGroups())visible.append(v.toMap().value("panels").toStringList());
        QSet<QString> seen;for(const auto &id:visible){QVERIFY(!seen.contains(id));seen.insert(id);}QCOMPARE(seen.size(),7);
    }
    void referenceWorkspaceDockingAndColorPicker() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("ui-layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *logo=findVisualItem(window->contentItem(),"applicationLogo");QVERIFY(logo);QTRY_COMPARE(logo->property("status").toInt(),1);
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);QTRY_VERIFY(canvas->width()>650);
        auto *radiusInput=findVisualItem(window->contentItem(),"brushRadiusText");QVERIFY(radiusInput);QVERIFY(radiusInput->width()>=28);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,radiusInput->mapToScene({radiusInput->width()/2,radiusInput->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_3);QTest::keyClick(window,Qt::Key_7);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(client.brushRadius(),qreal(18.5));
        auto *wheel=qobject_cast<ColorWheelItem*>(findVisualItem(window->contentItem(),"colorWheel"));QVERIFY(wheel);QTRY_VERIFY(wheel->height()>80);
        client.setBrushColor(Qt::red);QTRY_COMPARE(wheel->color(),QColor(Qt::red));
        const qreal radius=(qMin(wheel->width(),wheel->height())-6)/2;const QPointF center(wheel->width()/2,wheel->height()/2);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,wheel->mapToScene(center+QPointF(-.4625*radius,-.801*radius)).toPoint());QVERIFY(client.brushColor().greenF()>.9 && client.brushColor().redF()<.1);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,wheel->mapToScene(center+QPointF(-.39*radius,-.67*radius)).toPoint());QVERIFY(client.brushColor().valueF()>.95 && client.brushColor().hsvSaturationF()<.1);
        const auto preview=qEnvironmentVariable("DRAWVERSE_WORKSPACE_PREVIEW");
        auto capture=[&](const QString &suffix){QTest::mouseMove(window,{20,window->height()-20});QTest::qWait(120);return preview.isEmpty() || window->grabWindow().save(preview+suffix+".png");};
        QVERIFY(capture(".main"));
        window->resize(980,700);QTest::qWait(100);auto *compactList=findVisualItem(window->contentItem(),"layerList");QVERIFY(compactList);QVERIFY(compactList->height()>=60);QVERIFY(canvas->width()>=400);QVERIFY(capture(".compact"));window->resize(1480,940);QTest::qWait(100);
        auto first=workspace.rightGroups().first().toMap().value("id").toString();auto second=workspace.rightGroups().last().toMap().value("id").toString();
        auto payload=[](const QString &group,const QString &panel){return QJsonDocument(QJsonObject{{"group",group},{"panel",panel},{"whole",false}}).toJson();};
        auto drop=[&](const QString &source,const QString &panel,const QString &target,qreal x,qreal y,Qt::KeyboardModifiers modifiers=Qt::NoModifier,QQuickWindow *host=nullptr){
            if(!host)host=window;
            auto *item=findVisualItem(host->contentItem(),"dockGroup:"+target);if(!item)return false;
            const auto at=item->mapToScene({x<0?item->width()/2:x,y<0?item->height()-4:y}).toPoint();QMimeData mime;mime.setData("application/x-drawverse-panel",payload(source,panel));
            QDragEnterEvent enter(at,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(host,&enter);if(!enter.isAccepted()){QDragLeaveEvent leave;QCoreApplication::sendEvent(host,&leave);return false;}
            QDragMoveEvent move(at,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(host,&move);
            auto *hint=findVisualItem(host->contentItem(),"dockPreview:"+target);if(!hint || !hint->isVisible())return false;
            if(!preview.isEmpty()){QTest::qWait(100);if(!host->grabWindow().save(preview+".drop.png"))return false;}
            QDropEvent event(at,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(host,&event);return event.isAccepted();
        };
        QVERIFY(drop(second,"history",first,-1,18));QTRY_COMPARE(workspace.groupDefinition(first).value("panels").toStringList(),(QStringList{"color","history"}));
        QTest::qWait(50);QVERIFY(drop(first,"history",first,8,20));QTRY_COMPARE(workspace.groupDefinition(first).value("panels").toStringList().first(),QString("history"));
        QTest::qWait(50);QVERIFY(drop(first,"history",second,-1,4));QTRY_COMPARE(workspace.rightGroups().size(),3);const auto middle=workspace.rightGroups()[1].toMap().value("id").toString();
        QTest::qWait(50);QVERIFY(drop(middle,"history",second,-1,-1));QTRY_COMPARE(workspace.rightGroups().last().toMap().value("panels").toStringList(),QStringList{"history"});
        QTest::qWait(50);QVERIFY(!drop(first,"color",second,-1,80,Qt::ControlModifier));QCOMPARE(workspace.rightGroups().size(),3);
        workspace.resetLayout();QTest::qWait(80);first=workspace.rightGroups().first().toMap().value("id").toString();
        auto click=[&](const QString &name){auto *item=findVisualItem(window->contentItem(),name);if(!item)return false;QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,item->mapToScene({item->width()/2,item->height()/2}).toPoint());return true;};
        QVERIFY(!findVisualItem(window->contentItem(),"groupCollapse:"+first));
        QVERIFY(click("dockCollapse:right"));QTRY_VERIFY(findVisualItem(window->contentItem(),"railPanel:layers"));QTRY_VERIFY(findVisualItem(window->contentItem(),"railPanel:layers")->isVisible());QVERIFY(capture(".collapsed"));
        QVERIFY(click("railPanel:layers"));QQuickWindow *peek=nullptr;
        QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName().startsWith("panelFlyout:"))peek=qobject_cast<QQuickWindow*>(w);return peek!=nullptr;}());
        QTRY_VERIFY(findVisualItem(peek->contentItem(),"layerList"));
        QVERIFY(click("dockCollapse:right"));QTRY_VERIFY(!findVisualItem(window->contentItem(),"railPanel:layers"));QTRY_COMPARE(applicationWindows().size(),1);
        workspace.hidePanel(first,"color");QTRY_VERIFY(!workspace.visiblePanels().contains("color"));workspace.showPanel("color");QTRY_VERIFY(workspace.visiblePanels().contains("color"));
        const auto brush=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(brush);QTRY_COMPARE(applicationWindows().size(),2);
        QQuickWindow *floating=nullptr;for(auto *w:applicationWindows())if(w!=window)floating=qobject_cast<QQuickWindow*>(w);QVERIFY(floating);
        if(!preview.isEmpty()){QTest::qWait(150);QVERIFY(floating->grabWindow().save(preview+".floating.png"));}
        QPointer<QQuickWindow> retained=floating;
        const auto custom=workspace.addCustomPanel("Saved colors","palette");QVERIFY(!custom.isEmpty());QTest::qWait(50);QVERIFY(retained);QCOMPARE(applicationWindows().size(),2);
        second=workspace.rightGroups().last().toMap().value("id").toString();QVERIFY(drop(second,"navigator",brush,-1,18,Qt::NoModifier,floating));QTRY_VERIFY(workspace.groupDefinition(brush).value("panels").toStringList().contains("navigator"));QVERIFY(retained);QCOMPARE(applicationWindows().size(),2);
        const auto railHeight=floating->height();workspace.setColumnCollapsed(brush,true);QTRY_COMPARE(floating->height(),railHeight);workspace.setColumnCollapsed(brush,false);QTRY_VERIFY(floating->height()>=200);
        floating->close();QTRY_COMPARE(workspace.leftGroups().size(),1);QTRY_COMPARE(applicationWindows().size(),1);
        QTRY_COMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void inlineMasksClippingAltClicksAndOpaquePreviewsWorkInFloatingPanels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        auto layer=[&](quint64 id){for(const auto &v:client.layers())if(v.toMap().value("id").toULongLong()==id)return v.toMap();return QVariantMap();};
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);
        client.newDocument(641,479);QTRY_COMPARE(client.documentWidth(),641);QTRY_VERIFY(client.ready());
        QTRY_VERIFY_WITH_TIMEOUT(!layer(1).value("thumbnail").toString().isEmpty(),10000);
        const auto encoded=layer(1).value("thumbnail").toString().section(',',1).toLatin1();const auto image=QImage::fromData(QByteArray::fromBase64(encoded));QVERIFY(!image.isNull());
        for(int y=0;y<image.height();++y)for(int x=0;x<image.width();++x)QCOMPARE(image.pixelColor(x,y),QColor(Qt::white));
        auto *thumb=findVisualItem(window->contentItem(),"layerThumbnail:1");auto *checker=findVisualItem(window->contentItem(),"thumbnailTransparency:1");QVERIFY(thumb && checker);QTRY_VERIFY(thumb->property("paintedWidth").toReal()>0);QCOMPARE(checker->width(),thumb->property("paintedWidth").toReal());QCOMPARE(checker->height(),thumb->property("paintedHeight").toReal());
        const auto opaquePreview=qEnvironmentVariable("DRAWVERSE_CLIPPING_PREVIEW");if(!opaquePreview.isEmpty()){QTRY_VERIFY(client.frameRevision()>=client.revision());QTest::qWait(100);QVERIFY(window->grabWindow().save(opaquePreview+".opaque.png"));}
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());canvas->actualSize();
        InputSample point;point.position={64.5,64.5};client.setBrushColor(Qt::white);client.setBrushRadius(25);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        client.addDefaultLayer();QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto top=client.activeLayer();client.setBrushColor(QColor("#d67065"));client.setBrushRadius(60);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());
        auto sample=[&](qreal x,qreal y){const auto r=client.frameRegion();const auto &f=client.frame();if(f.isNull() || r.isEmpty())return QColor();const int px=qFloor((x-r.x())*f.width()/r.width()),py=qFloor((y-r.y())*f.height()/r.height());return px>=0 && py>=0 && px<f.width() && py<f.height()?f.pixelColor(px,py):QColor();};
        QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(sample(100.5,64.5).alpha(),255);
        auto altClick=[&](QQuickWindow *w,quint64 id){auto *b=findVisualItem(w->contentItem(),"clippingBoundary:"+QString::number(id));if(!b || !b->isEnabled())return false;QTest::mouseClick(w,Qt::LeftButton,Qt::AltModifier,b->mapToScene({b->width()*0.6,b->height()/2}).toPoint());return true;};
        QVERIFY(altClick(window,top));QTRY_VERIFY(layer(top).value("clipped").toBool());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(top).value("clipBase").toULongLong(),quint64(1));QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(sample(100.5,64.5).alpha(),0);QCOMPARE(sample(64.5,64.5).alpha(),255);
        auto *arrow=findVisualItem(window->contentItem(),"clippingArrow:"+QString::number(top));QVERIFY(arrow);QVERIFY(arrow->isVisible());QVERIFY(findVisualItem(window->contentItem(),"layerName:1")->property("font").value<QFont>().underline());
        auto clickLowerEdge=[&](){auto *r=findVisualItem(window->contentItem(),"layerRow:1");if(!r)return false;QTest::mouseClick(window,Qt::LeftButton,Qt::AltModifier,r->mapToScene({r->width()*0.6,2}).toPoint());return true;};
        QVERIFY(clickLowerEdge());QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!layer(top).value("clipped").toBool());QVERIFY(clickLowerEdge());QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(layer(top).value("clipped").toBool());
        client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!layer(top).value("clipped").toBool());client.redo();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(layer(top).value("clipped").toBool());
        QVERIFY(client.addMask(1));QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());const auto mask=client.activeLayer();
        auto *list=findVisualItem(window->contentItem(),"layerList");QVERIFY(list);QTRY_COMPARE(list->property("count").toInt(),2);QVERIFY(!findVisualItem(window->contentItem(),"layerRow:"+QString::number(mask)));
        auto *raw=findVisualItem(window->contentItem(),"layerThumbnail:1");auto *maskThumb=findVisualItem(window->contentItem(),"layerThumbnail:"+QString::number(mask));QVERIFY(raw && maskThumb);
        // Delegate creation precedes the asynchronous Row layout, especially at high DPI.
        QTRY_VERIFY(maskThumb->mapToScene({0,0}).x()>raw->mapToScene({raw->width(),0}).x());QTRY_COMPARE(maskThumb->mapToScene({0,0}).y(),raw->mapToScene({0,0}).y());
        auto hit=[&](QQuickWindow *w,quint64 id,Qt::KeyboardModifiers modifiers=Qt::NoModifier){QTest::qWait(50);auto *i=findVisualItem(w->contentItem(),"layerThumbnailHit:"+QString::number(id));if(!i || !i->isVisible())return false;QTest::mouseClick(w,Qt::LeftButton,modifiers,i->mapToScene({i->width()/2,i->height()/2}).toPoint());return true;};
        QTRY_VERIFY(list->height()>=46);
        QVERIFY(hit(window,1));QTRY_COMPARE(client.activeLayer(),quint64(1));QTRY_VERIFY(!client.layerEditBusy());QVERIFY(hit(window,mask));QTRY_COMPARE(client.activeLayer(),mask);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushColor(Qt::black);client.setBrushRadius(9);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(sample(64.5,64.5).alpha(),0);
        QVERIFY(hit(window,mask,Qt::ShiftModifier));QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!layer(mask).value("visible").toBool());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(sample(64.5,64.5).alpha(),255);QVERIFY(hit(window,mask,Qt::ShiftModifier));QTRY_VERIFY(!client.layerEditBusy());
        const auto dock=workspace.rightGroups().last().toMap().value("id").toString();workspace.detachPanel(dock,"layers");QTRY_COMPARE(workspace.floatingGroups().size(),1);QQuickWindow *floating=nullptr;QTRY_VERIFY(([&](){for(auto *w:applicationWindows())if(w!=window){floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}return false;})());QTRY_VERIFY(findVisualItem(floating->contentItem(),"clippingBoundary:"+QString::number(top)));
        QVERIFY(altClick(floating,top));QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!layer(top).value("clipped").toBool());QVERIFY(altClick(floating,top));QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(layer(top).value("clipped").toBool());QVERIFY(hit(floating,mask));QTRY_COMPARE(client.activeLayer(),mask);QTRY_VERIFY(!client.layerEditBusy());
        const auto preview=qEnvironmentVariable("DRAWVERSE_CLIPPING_PREVIEW");if(!preview.isEmpty()){floating->resize(360,700);canvas->fitToView();QTest::mouseMove(floating,{24,floating->height()-100});QTest::qWait(500);QVERIFY(window->grabWindow().save(preview));QVERIFY(floating->grabWindow().save(preview+".layers.png"));}
        const auto path=QUrl::fromLocalFile(temp.filePath("clipping.ora"));QSignalSpy files(&client,&PaintCoreClient::fileFinished);QVERIFY(client.saveDocument(path));QTRY_COMPARE_WITH_TIMEOUT(files.size(),1,10000);QVERIFY(files.first().first().toBool());client.newDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());QVERIFY(client.openDocument(path));QTRY_COMPARE_WITH_TIMEOUT(files.size(),2,10000);QVERIFY(files.last().first().toBool());QTRY_VERIFY(client.ready());int clips=0;for(const auto &v:client.layers())clips+=v.toMap().value("clipped").toBool()?1:0;QCOMPARE(clips,1);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void masksDefaultsDepthAndDragDropWorkAcrossFloatingPanels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors)warnings.append(error.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        auto layer=[&](quint64 id){for(const auto &v:client.layers())if(v.toMap().value("id").toULongLong()==id)return v.toMap();return QVariantMap();};
        QCOMPARE(layer(1).value("name").toString(),QStringLiteral("背景"));QCOMPARE(client.undoDepth(),0);
        client.newDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());QTRY_VERIFY(client.frameRevision()>=client.revision() && !client.frame().isNull());QTRY_COMPARE(client.frame().pixelColor(client.frame().width()/2,client.frame().height()/2),QColor(Qt::white));
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();
        auto centerColor=[&](){const auto region=client.frameRegion();const auto &frame=client.frame();if(frame.isNull() || region.isEmpty())return QColor();const int x=qFloor((64.5-region.x())*frame.width()/region.width());const int y=qFloor((64.5-region.y())*frame.height()/region.height());if(x<0 || y<0 || x>=frame.width() || y>=frame.height())return QColor();return frame.pixelColor(x,y);};
        auto *maskButton=findVisualItem(window->contentItem(),"addLayerMask");QVERIFY(maskButton);QVERIFY(QMetaObject::invokeMethod(maskButton,"clicked"));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto mask=client.activeLayer();QVERIFY(layer(mask).value("mask").toBool());QCOMPARE(layer(mask).value("depth").toInt(),1);
        auto *inlineMask=findVisualItem(window->contentItem(),"inlineMask:1");QVERIFY(inlineMask);QVERIFY(inlineMask->isVisible());QVERIFY(!findVisualItem(window->contentItem(),"layerRow:"+QString::number(mask)));
        client.setBrushRadius(12);client.setBrushColor(Qt::black);InputSample point;point.position={64.5,64.5};const auto depth=client.undoDepth();QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(centerColor().alpha(),0);
        client.setBrushColor(Qt::white);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+2);QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(centerColor(),QColor(Qt::white));client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(centerColor().alpha(),0);
        client.setBrushColor(QColor("#808080"));QVERIFY(client.beginStroke(point));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(centerColor().alpha(),128);client.undo();QTRY_VERIFY(!client.layerEditBusy());
        client.setLayerProperties(mask,false,1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(centerColor(),QColor(Qt::white));client.setLayerProperties(mask,true,1);QTRY_VERIFY(!client.layerEditBusy());
        QQuickItem *opacity=nullptr;QTRY_VERIFY((opacity=findVisualItem(window->contentItem(),"layerOpacity")));QVERIFY(QMetaObject::invokeMethod(opacity,"committed",Q_ARG(double,.5)));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(1).value("opacity").toFloat(),.5f);QCOMPARE(layer(mask).value("opacity").toFloat(),1.f);client.undo();QTRY_VERIFY(!client.layerEditBusy());
        auto *density=findVisualItem(window->contentItem(),"maskDensity");QVERIFY(density);QVERIFY(QMetaObject::invokeMethod(density,"committed",Q_ARG(double,.5)));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(mask).value("opacity").toFloat(),.5f);QCOMPARE(layer(1).value("opacity").toFloat(),1.f);client.undo();QTRY_VERIFY(!client.layerEditBusy());
        client.selectLayer(1);QTRY_VERIFY(!client.layerEditBusy());client.groupLayer(1,"绘画组");QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();QCOMPARE(layer(mask).value("depth").toInt(),2);
        auto *thumbnail=findVisualItem(window->contentItem(),"layerThumbnail:"+QString::number(group));QVERIFY(thumbnail);QVERIFY(!thumbnail->isVisible());
        client.selectLayer(group);QTRY_VERIFY(!client.layerEditBusy());client.addDefaultLayer();QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());const auto first=client.activeLayer();QCOMPARE(layer(first).value("name").toString(),QStringLiteral("图层一"));
        client.addDefaultLayer();QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());const auto second=client.activeLayer();QCOMPARE(layer(second).value("name").toString(),QStringLiteral("图层二"));
        QVERIFY(client.dropLayer(first,0,0));QTRY_COMPARE(layer(first).value("parent").toULongLong(),quint64(0));QTRY_VERIFY(!client.layerEditBusy());
        const auto dockGroup=workspace.rightGroups().last().toMap().value("id").toString();workspace.detachPanel(dockGroup,"layers");QTRY_COMPARE(workspace.floatingGroups().size(),1);QQuickWindow *floating=nullptr;QTRY_VERIFY(([&](){for(auto *w:applicationWindows())if(w!=window){floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}return false;})());
        QTRY_VERIFY(findVisualItem(floating->contentItem(),"layerRow:"+QString::number(group)));
        auto drop=[&](quint64 source,quint64 target,int placement){auto *row=findVisualItem(floating->contentItem(),target ? "layerRow:"+QString::number(target) : "layerRootDrop");if(!row)return false;const auto at=row->mapToScene({row->width()*0.72,placement==0?row->height()/2:placement==1?3.:row->height()-3.}).toPoint();QMimeData mime;mime.setData("application/x-drawverse-layer",client.layerDragPayload(source).toUtf8());QDragEnterEvent enter(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(floating,&enter);QDragMoveEvent move(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(floating,&move);QDropEvent event(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(floating,&event);return event.isAccepted();};
        QVERIFY(drop(first,group,0));QTRY_COMPARE(layer(first).value("parent").toULongLong(),group);QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(drop(first,second,2));QTRY_VERIFY(!client.layerEditBusy());auto ids=[&](){QList<quint64> ids;for(const auto &v:client.layers())ids.append(v.toMap().value("id").toULongLong());return ids;};QVERIFY(ids().indexOf(first)>ids().indexOf(second));client.undo();QTRY_VERIFY(!client.layerEditBusy());QVERIFY(ids().indexOf(first)<ids().indexOf(second));
        QVERIFY(drop(first,0,0));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(first).value("parent").toULongLong(),quint64(0));QCOMPARE(ids().last(),first);client.undo();QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(first).value("parent").toULongLong(),group);
        const auto preview=qEnvironmentVariable("DRAWVERSE_MASKS_PREVIEW");
        if(!preview.isEmpty()){floating->resize(360,760);client.selectLayer(mask);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());canvas->fitToView();QTest::qWait(250);QVERIFY(window->grabWindow().save(preview));QVERIFY(floating->grabWindow().save(preview+".layers.png"));}
        const auto path=QUrl::fromLocalFile(temp.filePath("masks.ora"));QSignalSpy files(&client,&PaintCoreClient::fileFinished);QVERIFY(client.saveDocument(path));QTRY_COMPARE_WITH_TIMEOUT(files.size(),1,10000);QVERIFY(files.first().first().toBool());
        const auto bad=client.layerDragPayload(first);client.newDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());QVERIFY(!client.acceptLayerDrop(bad,0,0));
        QVERIFY(client.openDocument(path));QTRY_COMPARE_WITH_TIMEOUT(files.size(),2,10000);QVERIFY(files.last().first().toBool());QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(client.ready());int masks=0;for(const auto &v:client.layers())if(v.toMap().value("mask").toBool())++masks;QCOMPARE(masks,1);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void groupsCollapseMoveUndoAndSaveWorkAcrossFloatingPanels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors) warnings.append(error.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        auto layer=[&](quint64 id) {for(const auto &value:client.layers()) if(value.toMap().value("id").toULongLong()==id) return value.toMap();return QVariantMap();};
        client.newTransparentDocument(128,96);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());
        InputSample point;point.position={32.5,48.5};client.setBrushColor(QColor("#e17662"));client.setBrushRadius(12);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);
        auto *groupButton=findVisualItem(window->contentItem(),"groupLayer");QVERIFY(groupButton);QVERIFY(QMetaObject::invokeMethod(groupButton,"clicked"));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());
        const auto group=client.activeLayer();QVERIFY(layer(group).value("group").toBool());QCOMPARE(layer(1).value("parent").toULongLong(),group);QCOMPARE(layer(1).value("depth").toInt(),1);
        QVERIFY(!client.beginStroke(point));client.clearError();QVERIFY(!findVisualItem(window->contentItem(),"lockTransparency")->isEnabled());
        client.addLayer("组内颜色");QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());const auto child=client.activeLayer();QCOMPARE(layer(child).value("parent").toULongLong(),group);
        client.setBrushColor(QColor("#2ea99d"));point.position={96.5,48.5};QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),4);
        auto *list=findVisualItem(window->contentItem(),"layerList");QVERIFY(list);QTRY_COMPARE(list->property("count").toInt(),3);
        auto *expand=findVisualItem(window->contentItem(),"groupExpand:"+QString::number(group));QVERIFY(expand);QVERIFY(QMetaObject::invokeMethod(expand,"clicked"));QTRY_COMPARE(list->property("count").toInt(),1);QCOMPARE(client.undoDepth(),4);
        const auto dock=workspace.rightGroups().last().toMap().value("id").toString();workspace.detachPanel(dock,"layers");QTRY_COMPARE(applicationWindows().size(),2);
        QQuickWindow *floating=nullptr;for(auto *candidate:applicationWindows()) if(candidate!=window) floating=qobject_cast<QQuickWindow*>(candidate);QVERIFY(floating);
        QTRY_VERIFY((list=findVisualItem(floating->contentItem(),"layerList")));QTRY_COMPARE(list->property("count").toInt(),1);QTRY_VERIFY((expand=findVisualItem(floating->contentItem(),"groupExpand:"+QString::number(group))));QVERIFY(QMetaObject::invokeMethod(expand,"clicked"));QTRY_COMPARE(list->property("count").toInt(),3);
        client.selectLayer(group);QTRY_COMPARE(client.activeLayer(),group);QTRY_VERIFY(!client.layerEditBusy());client.setLayerProperties(group,true,.5);QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(client.moveLayer(group,4,2));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(1).value("offsetX").toInt(),4);QCOMPARE(layer(child).value("offsetY").toInt(),2);
        QVERIFY(client.setLayerLocks(group,4));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(child).value("effectiveLocks").toInt(),4);
        client.selectLayer(child);QTRY_COMPARE(client.activeLayer(),child);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(!client.beginStroke(point));client.clearError();QVERIFY(!findVisualItem(floating->contentItem(),"layerOpacity")->isEnabled());
        QVERIFY(client.setLayerLocks(group,0));QTRY_VERIFY(!client.layerEditBusy());
        auto *menuButton=findVisualItem(floating->contentItem(),"layerGroupMenu");QVERIFY(menuButton);QVERIFY(QMetaObject::invokeMethod(menuButton,"clicked"));
        auto *menu=floating->findChild<QObject*>("groupOperationsMenu");QVERIFY(menu);QTRY_VERIFY(menu->property("visible").toBool());
        QObject *moveInto=nullptr;QTRY_VERIFY((moveInto=floating->findChild<QObject*>("moveLayerInto:"+QString::number(group))));QVERIFY(!moveInto->property("enabled").toBool());
        auto *moveOut=floating->findChild<QObject*>("moveLayerOut");QVERIFY(moveOut);QVERIFY(moveOut->property("enabled").toBool());QVERIFY(QMetaObject::invokeMethod(moveOut,"triggered"));QVERIFY(QMetaObject::invokeMethod(menu,"close"));
        QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(child).value("parent").toULongLong(),quint64(0));client.undo();QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(child).value("parent").toULongLong(),group);
        QVERIFY(client.ungroupLayer(group));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());client.undo();QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());
        client.selectLayer(group);QTRY_COMPARE(client.activeLayer(),group);QTRY_VERIFY(!client.layerEditBusy());
        const auto url=QUrl::fromLocalFile(temp.filePath("groups.ora"));QSignalSpy files(&client,&PaintCoreClient::fileFinished);QVERIFY(client.saveDocument(url,4));QTRY_COMPARE_WITH_TIMEOUT(files.size(),1,10000);QVERIFY(files.first().first().toBool());
        client.newTransparentDocument(16,16);QTRY_COMPARE(client.documentWidth(),16);QVERIFY(client.openDocument(url));QTRY_COMPARE_WITH_TIMEOUT(files.size(),2,10000);QVERIFY(files.last().first().toBool());QTRY_COMPARE(client.layers().size(),3);
        const auto loadedGroup=client.activeLayer();QVERIFY(layer(loadedGroup).value("group").toBool());QCOMPARE(layer(loadedGroup).value("opacity").toFloat(),.5f);
        workspace.returnGroup(workspace.floatingGroups().first().toMap().value("id").toString());QTRY_COMPARE(applicationWindows().size(),1);
        const auto preview=qEnvironmentVariable("DRAWVERSE_GROUPS_PREVIEW");if(!preview.isEmpty()) {
            window->findChild<CanvasItem*>("mainCanvas")->fitToView();QTRY_VERIFY(client.frameRevision()>=client.revision());QTest::qWait(300);QVERIFY(window->grabWindow().save(preview));
        }
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void transparentCanvasAndNavigatorUseCheckerboardWithoutChangingPixels() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini")); WorkspaceManager workspace(temp.filePath("layout.ini"));
        QQmlApplicationEngine engine; QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors) warnings.append(error.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/Main.qml")); QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas"); QVERIFY(canvas); QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(128,96); QTRY_COMPARE(client.documentWidth(),128); QTRY_VERIFY(!client.layerEditBusy()); canvas->actualSize();
        auto *grid=findVisualItem(window->contentItem(),"canvasTransparency"); QVERIFY(grid);
        auto sample=[&](QQuickItem *item,QPointF local) {
            const auto shot=window->grabWindow(); if(shot.isNull()) return QColor();
            const auto point=item->mapToScene(local); const auto scale=static_cast<qreal>(shot.width())/window->width();
            return shot.pixelColor(qFloor(point.x()*scale),qFloor(point.y()*scale));
        };
        auto checkGrid=[&](QQuickItem *item) {
            return sample(item,{6,6})==QColor(Qt::white) && sample(item,{18,6})==QColor(204,204,204)
                && sample(item,{6,18})==QColor(204,204,204) && sample(item,{18,18})==QColor(Qt::white);
        };
        QTRY_VERIFY_WITH_TIMEOUT(checkGrid(grid),10000);
        const auto center=canvas->mapToScene(canvas->documentRect().center());
        QWheelEvent zoom(center,window->mapToGlobal(center.toPoint()),{},QPoint(0,120),Qt::NoButton,Qt::NoModifier,Qt::NoScrollPhase,false);
        QCoreApplication::sendEvent(window,&zoom);QVERIFY(canvas->zoom()>1);QTRY_VERIFY(checkGrid(grid));canvas->actualSize();QTRY_VERIFY(checkGrid(grid));
        client.setBrushColor(Qt::black); client.setBrushRadius(10); InputSample point; point.position={64.5,48.5}; QVERIFY(client.beginStroke(point));client.endStroke();
        QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(client.frameRevision()>=client.revision());
        QTRY_COMPARE(sample(canvas,canvas->documentRect().topLeft()+QPointF(64.5,48.5)),QColor(Qt::black));
        client.addLayer("second");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());
        client.setLayerProperties(2,false,1);QTRY_VERIFY(!client.layerEditBusy());
        client.setLayerProperties(1,false,1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());
        QTRY_VERIFY(checkGrid(grid));
        QCOMPARE(sample(canvas,canvas->documentRect().topLeft()+QPointF(66,54)),QColor(204,204,204));
        QCOMPARE(qAlpha(client.frame().pixel(client.frame().width()/2,client.frame().height()/2)),0);
        const auto path=temp.filePath("transparent.png");QSignalSpy saved(&client,&PaintCoreClient::fileFinished);
        QVERIFY(client.saveDocument(QUrl::fromLocalFile(path),1));QTRY_COMPARE_WITH_TIMEOUT(saved.size(),1,10000);QVERIFY(saved.first().first().toBool());
        const QImage exported(path);QVERIFY(!exported.isNull());QCOMPARE(exported.pixelColor(64,48).alpha(),0);QCOMPARE(exported.pixelColor(6,6).alpha(),0);
        const auto group=workspace.rightGroups().last().toMap().value("id").toString();
        auto *dock=findVisualItem(window->contentItem(),"dockGroup:"+group);QVERIFY(dock);QVERIFY(dock->setProperty("selected","navigator"));workspace.setActive(group,"navigator");
        QQuickItem *miniGrid=nullptr;QTRY_VERIFY((miniGrid=findVisualItem(window->contentItem(),"navigatorTransparency")));
        QTRY_VERIFY_WITH_TIMEOUT(checkGrid(miniGrid),10000);
        client.setLayerProperties(1,true,1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision(1)>=client.revision());
        QCOMPARE(qAlpha(client.frame(1).pixel(client.frame(1).width()/2,client.frame(1).height()/2)),255);
        QTRY_VERIFY(checkGrid(miniGrid));
        const auto preview=qEnvironmentVariable("DRAWVERSE_TRANSPARENCY_PREVIEW");
        if(!preview.isEmpty()) {client.setLayerProperties(1,false,1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTest::qWait(100);QVERIFY(window->grabWindow().save(preview));}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void layerControlsCommitOnceAndWorkInFloatingPanels() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini")); WorkspaceManager workspace(temp.filePath("layout.ini"));
        QQmlApplicationEngine engine; QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){ for(const auto &error:errors) warnings.append(error.toString()); });
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/Main.qml")); QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        auto layer=[&](){ for(const auto &value:client.layers()) if(value.toMap().value("id").toULongLong()==client.activeLayer()) return value.toMap(); return QVariantMap(); };
        QQuickItem *opacity=nullptr;QTRY_VERIFY((opacity=findVisualItem(window->contentItem(),"layerOpacity")));
        auto *edit=opacity->findChild<QObject*>("layerOpacityInput"); QVERIFY(edit);
        QVERIFY(edit->setProperty("text","0%")); QVERIFY(QMetaObject::invokeMethod(edit,"commit"));
        QTRY_COMPARE(layer().value("opacity").toFloat(),0.f); QTRY_VERIFY(!client.layerEditBusy()); QCOMPARE(client.undoDepth(),1);
        QVERIFY(edit->setProperty("text","100%")); QVERIFY(QMetaObject::invokeMethod(edit,"commit"));
        QTRY_COMPARE(layer().value("opacity").toFloat(),1.f); QTRY_VERIFY(!client.layerEditBusy()); QCOMPARE(client.undoDepth(),2);
        QVERIFY(QMetaObject::invokeMethod(opacity,"openSlider"));
        auto *slider=qobject_cast<QQuickItem*>(opacity->findChild<QObject*>("layerOpacitySlider")); QVERIFY(slider);
        QTRY_VERIFY(slider->isVisible() && slider->width()>100);
        const auto start=slider->mapToScene({slider->width()*.8,slider->height()/2}).toPoint();
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,start);
        for(int i=0;i<8;++i) QTest::mouseMove(window,slider->mapToScene({slider->width()*(.7-i*.04),slider->height()/2}).toPoint());
        QCOMPARE(client.undoDepth(),2);
        QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,slider->mapToScene({slider->width()*.42,slider->height()/2}).toPoint());
        QTRY_COMPARE(client.undoDepth(),3); QTRY_VERIFY(!client.layerEditBusy()); QVERIFY(layer().value("opacity").toFloat()<.6f);
        QTest::keyClick(window,Qt::Key_Escape);
        auto *blend=findVisualItem(window->contentItem(),"layerBlendMode"); QVERIFY(blend); QCOMPARE(blend->property("count").toInt(),27);
        QVERIFY(blend->setProperty("currentIndex",3)); QVERIFY(QMetaObject::invokeMethod(blend,"activated",Q_ARG(int,3)));
        QTRY_COMPARE(layer().value("blendMode").toInt(),3); QTRY_VERIFY(!client.layerEditBusy());
        auto *fill=findVisualItem(window->contentItem(),"layerFill"); QVERIFY(fill);
        QVERIFY(QMetaObject::invokeMethod(fill,"committed",Q_ARG(double,.35))); QTRY_COMPARE(layer().value("fill").toFloat(),.35f); QTRY_VERIFY(!client.layerEditBusy());
        for(const auto &name:QStringList{"lockTransparency","lockPosition","lockAll"}) {
            auto *button=findVisualItem(window->contentItem(),name); QVERIFY(button); QVERIFY(QMetaObject::invokeMethod(button,"clicked")); QTRY_VERIFY(button->property("selected").toBool()); QTRY_VERIFY(!client.layerEditBusy());
        }
        QCOMPARE(layer().value("locks").toInt(),7); QVERIFY(!opacity->isEnabled()); QVERIFY(!blend->isEnabled()); QVERIFY(!fill->isEnabled());
        InputSample sample; sample.position={32,32}; QVERIFY(!client.beginStroke(sample)); client.clearError();
        auto *eye=findVisualItem(window->contentItem(),"layerVisibility:1"); QVERIFY(eye); QVERIFY(QMetaObject::invokeMethod(eye,"clicked"));
        QTRY_VERIFY(!layer().value("visible").toBool()); QTRY_VERIFY(!client.layerEditBusy());
        QTRY_VERIFY_WITH_TIMEOUT(!layer().value("thumbnail").toString().isEmpty(),10000);
        const auto group=workspace.rightGroups().last().toMap().value("id").toString(); workspace.detachPanel(group,"layers");
        QTRY_COMPARE(applicationWindows().size(),2);
        QQuickWindow *floating=nullptr; for(auto *candidate:applicationWindows()) if(candidate!=window) floating=qobject_cast<QQuickWindow*>(candidate);
        QVERIFY(floating); QQuickItem *unlock=nullptr; QTRY_VERIFY((unlock=findVisualItem(floating->contentItem(),"lockAll")));
        QVERIFY(QMetaObject::invokeMethod(unlock,"clicked")); QTRY_COMPARE(layer().value("locks").toInt(),3); QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(findVisualItem(floating->contentItem(),"layerOpacity")->isEnabled());
        workspace.returnGroup(workspace.floatingGroups().first().toMap().value("id").toString()); QTRY_COMPARE(applicationWindows().size(),1);
        const auto preview=qEnvironmentVariable("DRAWVERSE_LAYERS_PREVIEW");
        if(!preview.isEmpty()) {
            workspace.resetLayout();
            client.addLayer(QStringLiteral("上色 · 柔光")); QTRY_COMPARE(client.layers().size(),2); QTRY_VERIFY(!client.layerEditBusy());
            client.setBrushColor(QColor("#2ea99d")); client.setBrushRadius(28);
            for(int row=0;row<3;++row) {
                InputSample point; point.capabilities=1; point.tool=1; point.pressure=.3f; point.position={160.,170.+row*120}; QVERIFY(client.beginStroke(point));
                for(int i=1;i<=80;++i) { point.position={160.+i*8.,170.+row*120+std::sin(i*.07)*60}; point.pressure=static_cast<float>(.2+.7*std::sin(i*.02)); client.strokeTo(point); }
                client.endStroke();
            }
            QTRY_VERIFY(client.frameRevision()>=client.revision());
            QVERIFY(client.setLayerBlend(client.activeLayer(),13)); QTRY_VERIFY(!client.layerEditBusy());
            QVERIFY(client.setLayerFill(client.activeLayer(),.8)); QTRY_VERIFY(!client.layerEditBusy());
            client.setLayerProperties(client.activeLayer(),true,.85); QTRY_VERIFY(!client.layerEditBusy());
            QVERIFY(client.setLayerLocks(client.activeLayer(),2)); QTRY_VERIFY(!client.layerEditBusy());
            QTest::qWait(350); QVERIFY(window->grabWindow().save(preview));
        }
        QCOMPARE(warnings,QStringList()); QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void moveToolMouseKeyboardAndPositionLock() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini")); WorkspaceManager workspace(temp.filePath("layout.ini")); QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace); engine.load(QUrl("qrc:/qml/Main.qml")); QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); auto *canvas=window->findChild<CanvasItem*>("mainCanvas"); QVERIFY(canvas); QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(128,64); QTRY_COMPARE(client.documentWidth(),128); QTRY_VERIFY(client.ready()); canvas->actualSize();
        InputSample sample; sample.position={16.5,16.5}; QVERIFY(client.beginStroke(sample)); client.endStroke(); QTRY_COMPARE(client.undoDepth(),1);
        client.setMoveTool(true); const auto pos=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(16,16)).toPoint();
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,pos); QTest::mouseMove(window,pos+QPoint(20,3)); QCOMPARE(client.undoDepth(),1);
        QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,pos+QPoint(20,3)); QTRY_COMPARE(client.undoDepth(),2); QTRY_VERIFY(!client.layerEditBusy());
        auto layer=[&](){ return client.layers().first().toMap(); }; QCOMPARE(layer().value("offsetX").toInt(),20); QCOMPARE(layer().value("offsetY").toInt(),3);
        QTest::keyClick(window,Qt::Key_Left,Qt::ShiftModifier); QTRY_COMPARE(layer().value("offsetX").toInt(),10); QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(client.setLayerLocks(1,2)); QTRY_VERIFY(!client.layerEditBusy()); const auto revision=client.revision();
        QTest::keyClick(window,Qt::Key_Right); QTRY_VERIFY(!client.lastError().isEmpty()); QTRY_VERIFY(!client.layerEditBusy()); QCOMPARE(client.revision(),revision); QCOMPARE(layer().value("offsetX").toInt(),10);
        client.clearError(); client.setEraser(false); sample.position={2.5,16.5}; const auto paintDepth=client.undoDepth(); QVERIFY(client.beginStroke(sample)); client.endStroke(); QTRY_COMPARE(client.undoDepth(),paintDepth+1); QTRY_VERIFY(client.revision()>revision);
        const auto depth=client.undoDepth(); client.setMoveTool(true); QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,pos); QTest::keyClick(window,Qt::Key_Escape); QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,pos+QPoint(10,0)); QTest::qWait(50); QCOMPARE(client.undoDepth(),depth);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void layerMetadataAndThumbnailsSurviveAsyncSaveOpen() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini")); QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        InputSample sample; sample.position={40.5,40.5}; QVERIFY(client.beginStroke(sample)); client.endStroke(); QTRY_COMPARE(client.undoDepth(),1);
        QVERIFY(client.moveLayer(1,-17,4)); QTRY_VERIFY(!client.layerEditBusy()); QVERIFY(client.setLayerFill(1,.42)); QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(client.setLayerBlend(1,8)); QTRY_VERIFY(!client.layerEditBusy()); QVERIFY(client.setLayerLocks(1,7)); QTRY_VERIFY(!client.layerEditBusy());
        const auto before=client.layers().first().toMap(); const auto path=QUrl::fromLocalFile(temp.filePath("layer-settings.ora")); QSignalSpy saved(&client,&PaintCoreClient::fileFinished);
        QVERIFY(client.saveDocument(path)); QTRY_COMPARE(saved.size(),1); QVERIFY(saved.last().first().toBool()); QVERIFY(!client.modified());
        client.newTransparentDocument(64,64); QTRY_COMPARE(client.documentWidth(),64); QTRY_VERIFY(client.ready()); QVERIFY(client.openDocument(path)); QTRY_COMPARE(saved.size(),2); QVERIFY(saved.last().first().toBool());
        QTRY_VERIFY(client.ready()); const auto after=client.layers().first().toMap();
        for(const auto &key:QStringList{"fill","blendMode","locks","offsetX","offsetY","dissolveSeed"}) QCOMPARE(after.value(key),before.value(key));
        client.requestLayerPreview(client.activeLayer());
        QTRY_VERIFY_WITH_TIMEOUT(!client.layers().first().toMap().value("thumbnail").toString().isEmpty(),10000);
        QImage thumbnail; const auto base64=client.layers().first().toMap().value("thumbnail").toString().section(',',1).toLatin1(); QVERIFY(thumbnail.loadFromData(QByteArray::fromBase64(base64),"PNG")); QVERIFY(thumbnail.width()<=64 && thumbnail.height()<=64);
        QVERIFY(client.lastError().isEmpty()); QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void storageSettingsDialogShowsAppliedAndPendingValues() {
        QTemporaryDir temp; QVERIFY(temp.isValid()); PaintCoreClient client(nullptr,temp.filePath("storage.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini")); QQmlApplicationEngine engine; QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){ for(const auto &error:errors) warnings.append(error.toString()); });
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/Main.qml")); QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        QTRY_VERIFY_WITH_TIMEOUT(client.ready() && !client.storageBusy(),10000);
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        auto *options=findVisualItem(window->contentItem(),"brushOptionsBar");QVERIFY(options);
        for(auto *item:options->findChildren<QObject*>()) {
            QVERIFY(item->property("tooltip").toString()!=QStringLiteral("性能与暂存盘"));
            QVERIFY(item->property("tooltip").toString()!=QStringLiteral("创建自定义面板"));
        }
        auto *customAction=window->findChild<QObject*>("customPanelAction");QVERIFY(customAction);
        auto *customDialog=window->findChild<QObject*>("customPanelDialog");QVERIFY(customDialog);
        QVERIFY(QMetaObject::invokeMethod(customAction,"triggered"));QTRY_VERIFY(customDialog->property("opened").toBool());
        QVERIFY(QMetaObject::invokeMethod(customDialog,"reject"));QTRY_VERIFY(!customDialog->property("opened").toBool());
        auto *storageAction=window->findChild<QObject*>("storagePreferencesAction");QVERIFY(storageAction);QVERIFY(storageAction->property("enabled").toBool());
        auto *dialog=window->findChild<QObject*>("preferencesDialog"); QVERIFY(dialog);QVERIFY(QMetaObject::invokeMethod(storageAction,"triggered"));QTRY_VERIFY(dialog->property("opened").toBool());QTRY_VERIFY(!client.storageBusy());
        auto *memory=dialog->findChild<QObject*>("storageMemory"); QVERIFY(memory); QCOMPARE(memory->property("currentIndex").toInt(),2);
        auto *scratch=dialog->findChild<QObject*>("storageScratch"); QVERIFY(scratch); QCOMPARE(scratch->property("value").toInt(),8);
        auto *reserve=dialog->findChild<QObject*>("storageReserve"); QVERIFY(reserve); QCOMPARE(reserve->property("value").toInt(),512);
        auto *directory=dialog->findChild<QObject*>("storageDirectory"); QVERIFY(directory); QVERIFY(directory->property("text").toString().isEmpty());
        const auto preview=qEnvironmentVariable("DRAWVERSE_STORAGE_PREVIEW");
        if(!preview.isEmpty()) { QTest::qWait(200); QVERIFY(window->grabWindow().save(preview)); }
        QVERIFY(memory->setProperty("currentIndex",0)); QVERIFY(scratch->setProperty("value",2));
        QVERIFY(reserve->setProperty("value",0)); QVERIFY(directory->setProperty("text",temp.path()));
        QSignalSpy saved(&client,&PaintCoreClient::storageFinished);
        auto *button=dialog->findChild<QObject*>("saveStorageSettings"); QVERIFY(button); QVERIFY(QMetaObject::invokeMethod(button,"clicked"));
        QTRY_COMPARE(saved.size(),1); QVERIFY(saved.first().first().toBool());
        QCOMPARE(client.storageSettings().value("memoryMiB").toInt(),64);
        QCOMPARE(client.activeStorageSettings().value("memoryMiB").toInt(),256);
        QVERIFY(QMetaObject::invokeMethod(dialog,"close")); QVERIFY(QMetaObject::invokeMethod(dialog,"open")); QTRY_VERIFY(!client.storageBusy());
        QCOMPARE(memory->property("currentIndex").toInt(),0); QCOMPARE(reserve->property("value").toInt(),0);
        QCOMPARE(warnings,QStringList()); QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void storagePreferencesPersistOnlyAfterValidationAndApplyAfterRestart() {
        QTemporaryDir temp; QVERIFY(temp.isValid()); const auto settings=temp.filePath("storage.ini");
        QVERIFY(QDir().mkpath(temp.filePath(QStringLiteral("暂存盘"))));
        const auto chosen=temp.filePath(QStringLiteral("暂存盘"));
        {
            PaintCoreClient client(nullptr,settings); QTRY_VERIFY_WITH_TIMEOUT(client.ready() && !client.storageBusy(),10000);
            QSignalSpy result(&client,&PaintCoreClient::storageFinished);
            client.addLayer("retained"); QTRY_COMPARE(client.undoDepth(),1);
            QVERIFY(client.saveStorageSettings(chosen,64,2,0));
            QVERIFY(!client.saveStorageSettings(chosen,128,4,0));
            QTRY_COMPARE(result.size(),1); QVERIFY(result.last().first().toBool());
            QCOMPARE(client.storageSettings().value("directory").toString(),chosen);
            QCOMPARE(client.activeStorageSettings().value("memoryMiB").toInt(),256);
            QCOMPARE(client.undoDepth(),1); QCOMPARE(client.layers().size(),2);
            QVERIFY(client.storageMessage().contains(QStringLiteral("重启")));
            QVERIFY(client.saveStorageSettings(temp.filePath("missing"),128,4,0));
            QTRY_COMPARE(result.size(),2); QVERIFY(!result.last().first().toBool());
            QCOMPARE(client.storageSettings().value("directory").toString(),chosen);
            QSettings persisted(settings,QSettings::IniFormat); QCOMPARE(persisted.value("storage/memoryMiB").toInt(),64);
            QCOMPARE(persisted.value("storage/directory").toString(),chosen);
            client.undo(); QTRY_COMPARE(client.redoDepth(),1);
            QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
        }
        PaintCoreClient restarted(nullptr,settings); QTRY_VERIFY_WITH_TIMEOUT(restarted.ready() && !restarted.storageBusy(),10000);
        QCOMPARE(restarted.activeStorageSettings().value("directory").toString(),chosen);
        QCOMPARE(restarted.activeStorageSettings().value("memoryMiB").toInt(),64);
        QCOMPARE(restarted.activeStorageSettings().value("scratchGiB").toInt(),2);
        QVERIFY(restarted.storageInfo().value("availableGiB").toDouble()>0);
        QSignalSpy stopped(&restarted,&PaintCoreClient::stopped); restarted.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void invalidStartupStorageCanBeCorrectedFromQml() {
        QTemporaryDir temp; QVERIFY(temp.isValid()); const auto settings=temp.filePath("bad-storage.ini");
        { QSettings bad(settings,QSettings::IniFormat); bad.setValue("storage/directory",temp.filePath("missing")); bad.sync(); }
        PaintCoreClient client(nullptr,settings); WorkspaceManager workspace(temp.filePath("layout.ini"));
        QQmlApplicationEngine engine; QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){ for(const auto &error:errors) warnings.append(error.toString()); });
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/Main.qml")); QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        QTRY_VERIFY_WITH_TIMEOUT(!client.storageBusy() && !client.lastError().isEmpty(),10000); QVERIFY(!client.ready());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        auto *dialog=window->findChild<QObject*>("preferencesDialog"); QVERIFY(dialog);
        QVERIFY(QMetaObject::invokeMethod(dialog,"open")); QTRY_VERIFY(!client.storageBusy());
        auto *directory=dialog->findChild<QObject*>("storageDirectory"); QVERIFY(directory);
        QVERIFY(directory->setProperty("text",temp.path()));
        auto *save=dialog->findChild<QObject*>("saveStorageSettings"); QVERIFY(save);
        QSignalSpy result(&client,&PaintCoreClient::storageFinished); QVERIFY(QMetaObject::invokeMethod(save,"clicked"));
        QTRY_VERIFY(!result.isEmpty()); QVERIFY(result.last().first().toBool()); QVERIFY(!client.ready());
        QCOMPARE(client.storageSettings().value("directory").toString(),temp.path());
        QCOMPARE(warnings,QStringList());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void largePagedDocumentPaintUndoRedoAndSaveRemainAsynchronous() {
        QTemporaryDir temp; QVERIFY(temp.isValid());
        QImage source(4096,4160,QImage::Format_RGBA8888); source.fill(QColor(30,80,120));
        const auto path=temp.filePath("large.png"); QVERIFY(source.save(path)); source=QImage();
        PaintCoreClient client; QSignalSpy files(&client,&PaintCoreClient::fileFinished);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        QVERIFY(client.openDocument(QUrl::fromLocalFile(path)));
        QTRY_COMPARE_WITH_TIMEOUT(client.documentHeight(),4160,30000);
        QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy(),30000);
        QTRY_VERIFY_WITH_TIMEOUT(!client.frame().isNull() && client.frameRevision()>=client.revision(),30000);
        const auto center=[&client] {return client.frame().pixel(client.frame().width()/2,client.frame().height()/2);};
        QCOMPARE(qRed(center()),30);
        client.setBrushRadius(256); client.setBrushColor(Qt::red);
        InputSample sample; sample.position={2048,2080};
        QVERIFY(client.beginStroke(sample));
        QElapsedTimer elapsed; elapsed.start(); client.endStroke(); QVERIFY(elapsed.elapsed()<1000);
        QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),1,30000);
        QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),30000);
        QVERIFY(qRed(center())>240 && qGreen(center())<10);
        client.undo(); QTRY_COMPARE_WITH_TIMEOUT(client.redoDepth(),1,30000);
        QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),30000);
        QCOMPARE(qRed(center()),30);
        client.redo(); QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),1,30000);
        QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),30000);
        QVERIFY(qRed(center())>240 && qGreen(center())<10);
        QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath("painted.png")),1));
        QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy(),30000);
        QVERIFY(QFileInfo::exists(temp.filePath("painted.png")));
        QVERIFY2(client.lastError().isEmpty(),qPrintable(client.lastError()));
        QVERIFY(files.size()>=2); QVERIFY(files.last().first().toBool());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void fullCanvasStrokeExceedsOldHistoryLimitAndRemainsUndoable() {
        PaintCoreClient client; QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(1536,1536); QTRY_COMPARE(client.documentWidth(),1536);
        client.setBrushRadius(64); InputSample sample; sample.position={32,32};
        QVERIFY(client.beginStroke(sample));
        for(int row=0;row<24;++row) for(int column=0;column<24;++column) {
            const int x=row%2==0?column:23-column;
            sample.position={x*64.+32.,row*64.+32.}; client.strokeTo(sample);
        }
        QElapsedTimer elapsed; elapsed.start(); client.endStroke();
        QVERIFY(elapsed.elapsed()<1000); QVERIFY(client.modified());
        QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),1,30000);
        QTRY_VERIFY_WITH_TIMEOUT(!client.frame().isNull() && client.frameRevision()>=client.revision(),30000);
        QCOMPARE(qAlpha(client.frame().pixel(client.frame().width()/2,client.frame().height()/2)),255);
        QVERIFY2(client.lastError().isEmpty(),qPrintable(client.lastError()));
        client.undo(); QTRY_COMPARE_WITH_TIMEOUT(client.redoDepth(),1,30000);
        QTRY_VERIFY(client.frameRevision()>=client.revision());
        QCOMPARE(qAlpha(client.frame().pixel(client.frame().width()/2,client.frame().height()/2)),0);
        client.redo(); QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),1,30000);
        QTRY_VERIFY(client.frameRevision()>=client.revision());
        QCOMPARE(qAlpha(client.frame().pixel(client.frame().width()/2,client.frame().height()/2)),255);
        QVERIFY(client.lastError().isEmpty());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void drawingKeepsQmlLayerDelegatesAlive() {
        QTemporaryDir temp;
        PaintCoreClient client; WorkspaceManager workspace(temp.filePath("performance.ini"));
        QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);
        engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/Main.qml"));
        QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready() && !client.frame().isNull(),10000);
        QPointer<QQuickItem> row;
        const auto rowName="layerRow:"+QString::number(client.activeLayer());
        QTRY_VERIFY((row=findVisualItem(window->contentItem(),rowName)));
        QSignalSpy layers(&client,&PaintCoreClient::layersChanged);
        QSignalSpy frames(&client,&PaintCoreClient::frameChanged);
        const auto initialRevision=client.frameRevision();
        client.setBrushRadius(8); InputSample sample; sample.position={40,40};
        QVERIFY(client.beginStroke(sample));
        for(int i=1;i<=60;++i) {
            sample.position={40.+i*3,40.+(i%3)}; sample.pressure=.3f+(i%5)*.1f;
            client.strokeTo(sample); QTest::qWait(8);
        }
        client.endStroke(); QTRY_COMPARE(client.undoDepth(),1);
        QTRY_VERIFY(client.frameRevision()>initialRevision && client.frameRevision()>=client.revision());
        QVERIFY(frames.size()>1); QCOMPARE(layers.size(),0);
        QVERIFY(row); QCOMPARE(findVisualItem(window->contentItem(),rowName),row.data());
        QVERIFY(client.modified());
        client.setLayerProperties(client.activeLayer(),false,.3);
        QTRY_VERIFY(!client.layers().first().toMap().value("visible").toBool());
        QVERIFY(!layers.isEmpty());
        QTRY_VERIFY((row=findVisualItem(window->contentItem(),rowName)));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
        delete window;
    }
    void fileCancellationAndShutdownStayAsynchronous() {
        QTemporaryDir temp; PaintCoreClient client; QSignalSpy files(&client,&PaintCoreClient::fileFinished);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(2048,2048); QTRY_COMPARE(client.documentWidth(),2048);
        const auto path=temp.filePath("cancelled.png");
        QVERIFY(client.saveDocument(QUrl::fromLocalFile(path),1)); client.cancelFile();
        QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy(),10000); QCOMPARE(files.size(),1);
        if(!files.last().first().toBool()) QVERIFY(!QFileInfo::exists(path));
        client.clearError();
        QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath("shutdown.png")),1));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); QElapsedTimer elapsed; elapsed.start();
        client.shutdown(); QVERIFY(elapsed.elapsed()<1000); QTRY_COMPARE_WITH_TIMEOUT(stopped.size(),1,10000);
    }
    void asynchronousFileRoundTripAndFailure() {
        QTemporaryDir temp; QVERIFY(temp.isValid());
        PaintCoreClient client; QSignalSpy files(&client,&PaintCoreClient::fileFinished);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(96,96); QTRY_COMPARE(client.documentWidth(),96);
        client.setBrushColor(Qt::red); InputSample s; s.position={32,32};
        QVERIFY(client.beginStroke(s)); QVERIFY(client.modified()); client.endStroke();
        QVERIFY(client.modified()); // Protects immediate close before the worker's metadata arrives.
        QTRY_COMPARE(client.undoDepth(),1);
        QVERIFY(client.modified());
        for(const auto &entry:QList<QPair<QString,int>>{{"image.png",1},{"image.jpg",2},{"image.webp",3}}) {
            QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath(entry.first)),entry.second));
            QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy(),10000); QVERIFY(files.last().first().toBool());
            QVERIFY(QFileInfo::exists(temp.filePath(entry.first))); QVERIFY(client.modified());
        }
        const auto url=QUrl::fromLocalFile(temp.filePath(QStringLiteral("绘画.ora")));
        QVERIFY(client.saveDocument(url)); QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy(),10000);
        QVERIFY(files.last().first().toBool()); QVERIFY(!client.modified()); QCOMPARE(client.documentUrl(),url);
        client.newTransparentDocument(32,32); QTRY_COMPARE(client.documentWidth(),32);
        QVERIFY(client.documentUrl().isEmpty());
        QVERIFY(client.openDocument(url)); QVERIFY(!client.ready());
        QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy() && client.ready(),10000);
        QVERIFY(files.last().first().toBool()); QCOMPARE(client.documentWidth(),96); QCOMPARE(client.undoDepth(),0);
        QTRY_VERIFY(!client.frame().isNull() && client.frame().width()==96);
        QTRY_VERIFY(qAlpha(client.frame().pixel(32,32))>0);
        const auto generation=client.generation();
        QVERIFY(client.openDocument(QUrl::fromLocalFile(temp.filePath("missing.png"))));
        QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy() && client.ready(),10000);
        QVERIFY(!files.last().first().toBool()); QVERIFY(!client.lastError().isEmpty());
        QCOMPARE(client.generation(),generation); QCOMPARE(client.documentWidth(),96); QCOMPARE(client.documentUrl(),url);
        const auto misleading=QUrl::fromLocalFile(temp.filePath("image-content.ora"));
        QVERIFY(client.saveDocument(misleading,1)); QTRY_VERIFY(!client.fileBusy());
        QVERIFY(client.openDocument(misleading)); QTRY_VERIFY(!client.fileBusy() && client.ready());
        QVERIFY(files.last().first().toBool()); QVERIFY(client.documentUrl().isEmpty());
        QCOMPARE(client.documentName(),QString("image-content.ora"));
        QVERIFY(!client.saveDocument(QUrl("https://example.invalid/file"))); QVERIFY(!client.fileBusy());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void asynchronousHistoryAndLayers() {
        PaintCoreClient client; QSignalSpy completed(&client,&PaintCoreClient::commandCompleted);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready() && !client.frame().isNull(),10000);
        client.newTransparentDocument(128,128);
        QTRY_COMPARE(client.documentWidth(),128);
        QTRY_COMPARE(client.frame().width(),128);
        client.setBrushColor(Qt::red); client.setBrushRadius(12);
        InputSample s; s.position={32,32}; s.pressure=.8f; s.capabilities=1; s.tool=1;
        QVERIFY(client.beginStroke(s)); s.position={80,32}; client.strokeTo(s); client.endStroke();
        QTRY_COMPARE(client.undoDepth(),1);
        QTRY_VERIFY(qAlpha(client.frame().pixel(50,32))>0);
        client.undo(); QTRY_COMPARE(client.redoDepth(),1);
        QTRY_COMPARE(qAlpha(client.frame().pixel(50,32)),0);
        client.redo(); QTRY_COMPARE(client.undoDepth(),1);
        QTRY_VERIFY(qAlpha(client.frame().pixel(50,32))>0);
        client.addLayer(QStringLiteral("测试 🖌")); QTRY_COMPARE(client.layers().size(),2);
        const auto id=client.activeLayer();
        QCOMPARE(client.layers().first().toMap().value("name").toString(),QStringLiteral("测试 🖌"));
        client.setLayerProperties(id,false,.3); QTRY_VERIFY(!client.layers().first().toMap().value("visible").toBool());
        client.removeLayer(id); QTRY_COMPARE(client.layers().size(),1);
        QVERIFY(!completed.isEmpty()); for(const auto &event:completed) QVERIFY(event.first().toBool());
        QVERIFY(client.lastError().isEmpty());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void pressureCancelAndValidation() {
        PaintCoreClient client; QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(128,128); QTRY_COMPARE(client.documentWidth(),128);
        QTRY_COMPARE(client.frame().width(),128);
        client.setBrushRadius(20);
        InputSample low; low.position={30,64}; low.pressure=.2f; low.capabilities=1; low.tool=1;
        QVERIFY(client.beginStroke(low)); client.endStroke();
        QTRY_COMPARE(client.undoDepth(),1);
        InputSample high=low; high.position={90,64}; high.pressure=1;
        QVERIFY(client.beginStroke(high)); client.endStroke(); QTRY_COMPARE(client.undoDepth(),2);
        QTRY_VERIFY(qAlpha(client.frame().pixel(90,76))>0);
        QCOMPARE(qAlpha(client.frame().pixel(30,76)),0);
        InputSample cancelled=high; cancelled.position={64,20};
        QVERIFY(client.beginStroke(cancelled)); client.cancelStroke();
        QTest::qWait(100); QCOMPARE(client.undoDepth(),2);
        QTRY_COMPARE(qAlpha(client.frame().pixel(64,20)),0);
        client.newTransparentDocument(1000001,1); QVERIFY(!client.lastError().isEmpty()); QCOMPARE(client.documentWidth(),128);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void workspaceRoundTripAndCorruption() {
        QTemporaryDir temp; QVERIFY(temp.isValid()); const QString path=temp.filePath("workspace.ini");
        WorkspaceManager workspace(path);
        QVERIFY(workspace.addCustomPanel("Note","notes").isEmpty());
        const auto id=workspace.addCustomPanel(QStringLiteral("自定义色板"),"palette"); QVERIFY(!id.isEmpty());
        const auto group=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachPanel(group,id); QCOMPARE(workspace.floatingGroups().size(),1);
        const auto floating=workspace.floatingGroups().first().toMap().value("id").toString();
        workspace.updateGeometry(floating,-90000,-90000,400,300); workspace.saveLayout();
        WorkspaceManager restored(path); QCOMPARE(restored.floatingGroups().size(),1);
        QCOMPARE(restored.panelDefinition(id).value("title").toString(),QStringLiteral("自定义色板"));
        QVERIFY(restored.floatingGroups().first().toMap().value("x").toInt()>-90000);
        restored.returnGroup(floating); QCOMPARE(restored.floatingGroups().size(),0);
        const auto target=restored.rightGroups().first().toMap().value("id").toString();
        const auto payload=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",floating},{"panel",id},{"whole",true}}).toJson());
        QVERIFY(restored.dockPayload(payload,"right",target));
        QCOMPARE(restored.rightGroups().size(),2);
        QVERIFY(!restored.dockPayload("{}","right",target));
        QSettings settings(path,QSettings::IniFormat); settings.setValue("workspace",QByteArray("{bad json}")); settings.sync();
        WorkspaceManager fallback(path); QCOMPARE(fallback.rightGroups().size(),2); QCOMPARE(fallback.panelDefinition("layers").value("kind").toString(),QString("layers"));
    }
    void sparseLargeCanvasAndViewportReplacement() {
        PaintCoreClient client; QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        const quint64 previousGeneration=client.generation();
        QElapsedTimer elapsed; elapsed.start(); client.newTransparentDocument(1000000,1000000);
        QVERIFY(elapsed.elapsed()<1000); QVERIFY(!client.ready());
        QTRY_VERIFY(client.ready() && client.generation()>previousGeneration);
        QCOMPARE(client.documentWidth(),1000000);
        QTRY_VERIFY_WITH_TIMEOUT(!client.frame().isNull(),10000);
        QVERIFY(client.frame().width()<=1024 && client.frame().height()<=1024);
        client.setBrushColor(Qt::red); client.setBrushRadius(8);
        InputSample sample; sample.position={900032,900032};
        QVERIFY(client.beginStroke(sample)); client.endStroke(); QTRY_COMPARE(client.undoDepth(),1);
        client.requestViewport(0,QRectF(0,0,64,64),QSize(64,64));
        const QRectF target(900000,900000,64,64);
        client.requestViewport(0,target,QSize(128,128));
        QTRY_COMPARE_WITH_TIMEOUT(client.frameRegion(),target,10000);
        QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),10000);
        QCOMPARE(client.frame().size(),QSize(128,128));
        QVERIFY(qAlpha(client.frame().pixel(64,64))>0);
        QCOMPARE(qAlpha(client.frame().pixel(0,0)),0);
        // Navigation is a separate frame channel with a hard display-memory bound.
        client.requestViewport(1,QRectF(0,0,1000000,1000000),QSize(8192,8192));
        QTRY_VERIFY_WITH_TIMEOUT(!client.frame(1).isNull(),10000);
        QVERIFY(qint64(client.frame(1).width())*client.frame(1).height()<=4194304);
        QVERIFY(client.frame(1).sizeInBytes()<=16777216);
        client.undo(); QTRY_COMPARE(client.redoDepth(),1);
        QTRY_VERIFY(client.frameRevision()>=client.revision());
        QCOMPARE(qAlpha(client.frame().pixel(64,64)),0);
        client.requestViewport(1,{}, {},false); QVERIFY(client.frame(1).isNull());
        client.newTransparentDocument(64,64); QTRY_VERIFY(client.ready() && client.documentWidth()==64);
        QTRY_COMPARE(client.frame().size(),QSize(64,64));
        QCOMPARE(client.frameRegion(),QRectF(0,0,64,64));
        QVERIFY(client.lastError().isEmpty());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void retiredNotesMigrateWithoutResettingWorkspace() {
        QTemporaryDir temp; const auto path=temp.filePath("legacy.ini");
        WorkspaceManager original(path); const auto palette=original.addCustomPanel("Colors","palette");
        const auto source=original.rightGroups().first().toMap().value("id").toString();
        original.detachPanel(source,palette); original.saveLayout();
        QSettings settings(path,QSettings::IniFormat);
        auto root=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();
        auto panels=root.value("panels").toObject();
        panels.insert("old-note",QJsonObject{{"kind","notes"},{"title",QStringLiteral("灵感随笔")},{"content","old text"},{"custom",true}});
        auto groups=root.value("groups").toArray();
        auto mixed=groups.first().toObject(); auto tabs=mixed.value("panels").toArray();
        tabs.append("old-note"); mixed.insert("panels",tabs); mixed.insert("active","old-note"); groups[0]=mixed;
        panels.insert("floating-note",QJsonObject{{"kind","notes"},{"title","Note"}});
        groups.append(QJsonObject{{"id","legacy-note-window"},{"location","floating"},{"active","floating-note"},{"panels",QJsonArray{"floating-note"}}});
        root.insert("version",1); root.insert("groups",groups); root.insert("panels",panels);
        settings.setValue("workspace",QJsonDocument(root).toJson()); settings.sync();
        WorkspaceManager migrated(path);
        QCOMPARE(migrated.floatingGroups().size(),1); // The floating palette survives.
        QCOMPARE(migrated.panelDefinition(palette).value("kind").toString(),QString("palette"));
        QVERIFY(migrated.panelDefinition("old-note").isEmpty());
        QVERIFY(migrated.panelDefinition("floating-note").isEmpty());
        QCOMPARE(migrated.rightGroups().first().toMap().value("active").toString(),QString("color"));
        settings.sync(); const auto saved=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();
        QCOMPARE(saved.value("version").toInt(),6); QVERIFY(!saved.value("panels").toObject().contains("old-note"));
        WorkspaceManager reopened(path); QCOMPARE(reopened.floatingGroups().size(),1);
    }
    void queueBackpressureRollsBack() {
        PaintCoreClient client; QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(128,128); QTRY_COMPARE(client.frame().width(),128);
        client.setBrushRadius(8);
        InputSample s; s.position={32,64}; QVERIFY(client.beginStroke(s));
        // A burst deliberately outruns the worker. The GUI loop must not wait for it.
        QElapsedTimer elapsed; elapsed.start();
        for(int i=0;i<30000;++i) { s.position={32.+(i%2)*32,64}; client.strokeTo(s); }
        client.endStroke(); QVERIFY(elapsed.elapsed()<3000);
        QVERIFY(!client.lastError().isEmpty());
        QTest::qWait(100);
        QTRY_COMPARE_WITH_TIMEOUT(qAlpha(client.frame().pixel(32,64)),0,10000);
        QCOMPARE(client.undoDepth(),0);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void qmlMouseTabletAndFloatingWindows() {
        QTemporaryDir temp;
        PaintCoreClient client; WorkspaceManager workspace(temp.filePath("qml.ini"));
        QQmlApplicationEngine engine; QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors) { for(const auto &error:errors) warnings.append(error.toString()); });
        engine.rootContext()->setContextProperty("PaintClient",&client);
        engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/Main.qml"));
        QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready() && !client.frame().isNull(),10000);
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas"); QVERIFY(canvas);
        auto *busy=findVisualItem(window->contentItem(),"canvasBusy"); QVERIFY(busy);
        QTRY_VERIFY(!busy->isVisible());
        QCOMPARE(applicationWindows().size(),1);
        QCOMPARE(workspace.floatingGroups().size(),0);
        QTRY_VERIFY(canvas->width()>300); canvas->fitToView();
        const QString navigatorGroup=workspace.rightGroups().last().toMap().value("id").toString();
        auto *navigatorDock=findVisualItem(window->contentItem(),"dockGroup:"+navigatorGroup); QVERIFY(navigatorDock);
        QVERIFY(navigatorDock->setProperty("selected","navigator"));
        workspace.setActive(navigatorGroup,"navigator");
        QTRY_VERIFY(!client.frame(1).isNull());
        QVERIFY(navigatorDock->setProperty("selected","layers"));
        workspace.setActive(navigatorGroup,"layers");
        QTRY_COMPARE(client.frame().width(),qCeil(canvas->documentRect().width()*window->devicePixelRatio()));
        QVERIFY(qint64(client.frame().width())*client.frame().height()<=4194304);
        const auto rect=canvas->documentRect();
        QCOMPARE(canvas->documentPoint(rect.topLeft()),QPointF(0,0));
        const QPoint center=canvas->mapToScene(rect.center()).toPoint();
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,center);
        QTRY_COMPARE(client.undoDepth(),1);
        QPointingDevice pen("Test tablet",999,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,
                            QInputDevice::Capability::Position|QInputDevice::Capability::Pressure|QInputDevice::Capability::XTilt|QInputDevice::Capability::Rotation,1,2);
        const QPointF pos=center+QPoint(60,30);
        QTabletEvent press(QEvent::TabletPress,&pen,pos,window->mapToGlobal(pos.toPoint()),.75f,20,10,.1f,30,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);
        QCoreApplication::sendEvent(window,&press); QVERIFY(press.isAccepted());
        QTabletEvent release(QEvent::TabletRelease,&pen,pos,window->mapToGlobal(pos.toPoint()),0,20,10,.1f,30,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);
        QCoreApplication::sendEvent(window,&release); QTRY_COMPARE(client.undoDepth(),2);
        const QPointF local=canvas->mapFromScene(center);
        const auto beforeZoom=canvas->documentPoint(local);
        QWheelEvent wheel(center,window->mapToGlobal(center),{},QPoint(0,120),Qt::NoButton,Qt::NoModifier,Qt::NoScrollPhase,false);
        QCoreApplication::sendEvent(window,&wheel);
        QVERIFY(QLineF(beforeZoom,canvas->documentPoint(local)).length()<.001);
        const auto beforePan=canvas->documentPoint(local);
        QTest::mousePress(window,Qt::MiddleButton,Qt::NoModifier,center);
        QTest::mouseMove(window,center+QPoint(40,20));
        QTest::mouseRelease(window,Qt::MiddleButton,Qt::NoModifier,center+QPoint(40,20));
        QVERIFY(QLineF(beforePan,canvas->documentPoint(local+QPointF(40,20))).length()<.001);
        QCOMPARE(client.undoDepth(),2);
        const auto group=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachGroup(group); QTRY_COMPARE(workspace.floatingGroups().size(),1);
        QTRY_COMPARE(applicationWindows().size(),2);
        workspace.returnGroup(workspace.floatingGroups().first().toMap().value("id").toString());
        QTRY_COMPARE(applicationWindows().size(),1);
        const QString notes=workspace.addCustomPanel("Palette","palette");
        auto source=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachPanel(source,notes); QTRY_COMPARE(applicationWindows().size(),2);
        auto floating=workspace.floatingGroups().first().toMap().value("id").toString();
        QQuickItem *target=nullptr;
        QTRY_VERIFY((target=findVisualItem(window->contentItem(),"dockGroup:"+source)));
        QTRY_VERIFY(target->width()>=180 && target->height()>=180);
        const auto dropPosition=target->mapToScene(QPointF(target->width()/2,18)).toPoint();
        QMimeData mime;
        mime.setData("application/x-drawverse-panel",QJsonDocument(QJsonObject{{"group",floating},{"panel",notes},{"whole",false}}).toJson());
        QDragEnterEvent enter(dropPosition,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);
        QCoreApplication::sendEvent(window,&enter); QVERIFY(enter.isAccepted());
        QDropEvent drop(dropPosition,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);
        QCoreApplication::sendEvent(window,&drop); QVERIFY(drop.isAccepted());
        QTRY_COMPARE(workspace.floatingGroups().size(),0); QTRY_COMPARE(applicationWindows().size(),1);
        QCOMPARE(workspace.rightGroups().size(),2);
        QVERIFY(workspace.rightGroups().first().toMap().value("panels").toStringList().contains(notes));
        workspace.detachPanel(source,notes); QTRY_COMPARE(applicationWindows().size(),2);
        QQuickWindow *floatingWindow=nullptr;
        for(auto *candidate:applicationWindows()) if(candidate!=window) floatingWindow=qobject_cast<QQuickWindow*>(candidate);
        QVERIFY(floatingWindow); floatingWindow->close();
        QTRY_COMPARE(workspace.floatingGroups().size(),0); QTRY_COMPARE(applicationWindows().size(),1);
        QCOMPARE(warnings,QStringList());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
        QVERIFY(window->close()); QVERIFY(!window->isVisible());
    }
    void panelTabsRetractWithoutExpandingRailsAndKeepSharedPresentation() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        // The default right column groups navigator/layers/history with the shared colour panel.
        const auto navigator=workspace.groupForPanel("navigator");
        QVERIFY(!navigator.isEmpty());QVERIFY(!workspace.groupForPanel("color").isEmpty());
        for(const auto &id:workspace.columnGroups(navigator))workspace.setColumnCollapsed(id,true);
        for(const auto &id:workspace.columnGroups(navigator))QVERIFY(workspace.groupDefinition(id).value("icons").toBool());
        auto *icon=findVisualItem(main->contentItem(),"railPanel:navigator");QVERIFY(icon);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());
        auto flyout=[&]()->QQuickWindow*{for(auto *window:applicationWindows())if(window->isVisible() && window->objectName().startsWith("panelFlyout:") && window->objectName().endsWith(":navigator"))return qobject_cast<QQuickWindow*>(window);return nullptr;};QTRY_VERIFY(flyout());
        QPointer<QQuickWindow> row=flyout();
        // Other already-open category windows stay untouched.
        auto *colorIcon=findVisualItem(main->contentItem(),"railPanel:color");QVERIFY(colorIcon);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,colorIcon->mapToScene({14,14}).toPoint());
        auto other=[&]()->QQuickWindow*{for(auto *window:applicationWindows())if(window->isVisible() && window->objectName().startsWith("panelFlyout:") && window->objectName().endsWith(":color") && window!=row)return qobject_cast<QQuickWindow*>(window);return nullptr;};QTRY_VERIFY(other());QPointer<QQuickWindow> otherRow=other();
        // Request 11: the shared top drag/tab bar of a floating column is translucent 75%; the docked bar stays opaque.
        auto *flyoutBar=findVisualItem(row->contentItem(),"panelTabBar:"+navigator);QVERIFY(flyoutBar);QVERIFY(std::abs(flyoutBar->property("color").value<QColor>().alphaF()-.75)<.01);
        // Request 6: double clicking the header retracts this presentation without re-expanding the source rail.
        auto *header=findVisualItem(row->contentItem(),"groupGrip:"+navigator);QVERIFY(header);QTest::mouseDClick(row,Qt::LeftButton,Qt::NoModifier,header->mapToScene({10,4}).toPoint());QTRY_VERIFY(row.isNull());
        for(const auto &id:workspace.columnGroups(navigator))QVERIFY(workspace.groupDefinition(id).value("icons").toBool());
        QVERIFY(icon->isVisible());QCOMPARE(workspace.groupDefinition(navigator).value("icons").toBool(),true);QVERIFY(!otherRow.isNull());
        // Request 6: a panel tab double click retracts the same way and must not expand the collapsed rail.
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());QTRY_VERIFY(flyout());row=flyout();
        auto *tab=findVisualItem(row->contentItem(),"panelTab:layers");QVERIFY(tab);QTest::mouseDClick(row,Qt::LeftButton,Qt::NoModifier,tab->mapToScene({8,10}).toPoint());QTRY_VERIFY(row.isNull());
        for(const auto &id:workspace.columnGroups(navigator))QVERIFY(workspace.groupDefinition(id).value("icons").toBool());
        QVERIFY(!otherRow.isNull());QVERIFY(workspace.groupDefinition(workspace.groupForPanel("color")).value("icons").toBool());
        otherRow->close();QTRY_VERIFY(otherRow.isNull());
        // Docked columns keep their semantics: a tab double click still collapses the column into its icon rail.
        workspace.setColumnCollapsed(navigator,false);
        auto *dockedBar=findVisualItem(main->contentItem(),"panelTabBar:"+navigator);QVERIFY(dockedBar);QTRY_VERIFY(std::abs(dockedBar->property("color").value<QColor>().alphaF()-1.)<.01);
        auto *dockedTab=findVisualItem(main->contentItem(),"panelTab:layers");QVERIFY(dockedTab);QTest::mouseDClick(main,Qt::LeftButton,Qt::NoModifier,dockedTab->mapToScene({8,10}).toPoint());
        QTRY_VERIFY(workspace.groupDefinition(navigator).value("icons").toBool());for(const auto &id:workspace.columnGroups(navigator))QVERIFY(workspace.groupDefinition(id).value("icons").toBool());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void textFieldsKeepSpaceAndHistoryReplayStaysStepped() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(48,48);QTRY_COMPARE(client.documentWidth(),48);QTRY_VERIFY(client.ready());client.setBrushRadius(3);client.setBrushColor(Qt::black);
        // Request 25: a focused text input keeps space while the canvas stays out of the shared pan state.
        auto *view=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(view);
        QQmlComponent component(&engine);
        component.setData("import QtQuick.Controls\nTextField { objectName:\"spacePanField\" }",QUrl());
        auto *field=qobject_cast<QQuickItem*>(component.create(engine.rootContext()));
        QVERIFY(field);field->setParentItem(main->contentItem());field->setWidth(120);field->setHeight(24);field->setProperty("text",QString("78"));field->forceActiveFocus();QTRY_COMPARE(main->activeFocusItem(),field);
        const auto undoBefore=client.undoDepth();QTest::keyClick(main,Qt::Key_Space);QVERIFY(!view->spacePanning());QTRY_COMPARE(field->property("text").toString(),QString("78 "));QCOMPARE(client.undoDepth(),undoBefore);
        QTest::keyClick(main,Qt::Key_Backspace);QTRY_COMPARE(field->property("text").toString(),QString("78"));
        // Request 27: four replay steps must be observable one at a time, not an instant jump.
        const auto group=workspace.groupForPanel("history");workspace.setActive(group,"history");
        auto *historyTab=findVisualItem(main->contentItem(),"panelTab:history");QVERIFY(historyTab);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,historyTab->mapToScene({8,10}).toPoint());QTRY_VERIFY(findVisualItem(main->contentItem(),"historyList"));
        client.setBrushRadius(3);InputSample sample;sample.position={24,24};sample.pressure=1;
        for(int i=0;i<4;++i) {const auto stroke=i+1;client.setBrushColor(i%2?Qt::black:Qt::white);QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),stroke);QTRY_VERIFY(!client.layerEditBusy());}
        QQuickItem *list=findVisualItem(main->contentItem(),"historyList");QVERIFY(list);QTRY_VERIFY(list->property("count").toInt()>0);
        auto *timer=main->findChild<QObject*>("historyReplayTimer");QVERIFY(timer);QCOMPARE(timer->property("interval").toInt(),20);QVERIFY(!timer->property("running").toBool());
        auto *previous=findVisualItem(main->contentItem(),"historyEntry:0");QVERIFY(previous);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,previous->mapToScene({20,12}).toPoint());
        QTRY_VERIFY(timer->property("running").toBool());
        QList<int> seen;QStringList trace;
        for(int i=0;i<40;++i) {const int depth=client.undoDepth();if(seen.isEmpty() || seen.last()!=depth){seen.append(depth);trace.append(QString::number(depth));}if(depth==0)break;QTest::qWait(5);}
        QTRY_COMPARE(client.undoDepth(),0);QCOMPARE(client.redoDepth(),4);
        QVERIFY2(seen.size()>=3 && seen.first()==4 && seen.last()==0,qPrintable("single-step replay did not walk through every depth: "+trace.join(",")));
        bool stoppedRunning=false;
        for(int i=0;i<100 && !stoppedRunning;++i) {stoppedRunning=!timer->property("running").toBool();if(!stoppedRunning)QTest::qWait(5);}
        QVERIFY2(stoppedRunning,qPrintable(QString("replay timer kept running: undo=%1 redo=%2 target=%3").arg(client.undoDepth()).arg(client.redoDepth()).arg(list->parentItem()?list->parentItem()->property("targetDepth").toInt():-99)));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void editMenuOpensKeyboardShortcutList() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        auto *action=main->findChild<QObject*>("keyboardShortcutsAction");QVERIFY(action);QVERIFY(action->property("enabled").toBool());
        auto *dialog=main->findChild<QObject*>("keyboardShortcuts");QVERIFY(dialog);QVERIFY(QMetaObject::invokeMethod(action,"triggered"));
        QTRY_VERIFY(dialog->property("opened").toBool());
        // The list is real: one row per bound shortcut. Assert on the rendered pixels so a dialog
        // that only reports a model without drawing rows would still fail.
        QObject *list=nullptr;QTRY_VERIFY((list=dialog->findChild<QObject*>("shortcutList")));QTRY_COMPARE(list->property("count").toInt(),26);
        auto *dialogContent=qobject_cast<QQuickItem*>(dialog->property("contentItem").value<QObject*>());QVERIFY(dialogContent);
        QQuickItem *listItem=qobject_cast<QQuickItem*>(list);QVERIFY(listItem);
        QImage shot;bool grabbed=false;
        QSharedPointer<QQuickItemGrabResult> grab=dialogContent->grabToImage();
        QVERIFY(grab);connect(grab.data(),&QQuickItemGrabResult::ready,this,[&]{shot=grab->image();grabbed=true;});
        QTRY_VERIFY(grabbed);QVERIFY(!shot.isNull());
        const auto origin=listItem->mapToScene({0,0})-dialogContent->mapToScene({0,0});int painted=0;
        for(int y=0;y<qMin(60,shot.height()-int(origin.y()));++y)for(int x=0;x<qMin(200,shot.width());++x)
            if(shot.pixelColor(int(origin.x())+x,int(origin.y())+y).value()>90)++painted;
        QVERIFY2(painted>40,qPrintable(QString("shortcut rows did not render: %1 lit pixels").arg(painted)));
        QVERIFY(QMetaObject::invokeMethod(dialog,"close"));QTRY_VERIFY(!dialog->property("opened").toBool());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void selectionToolVariantsUseCornerMarkAndRightClickPanel() {        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        auto *selection=findVisualItem(main->contentItem(),"selectionTool");QVERIFY(selection);
        // The corner triangle is the affordance that tells the user the slot has more shapes.
        QObject *mark=main->findChild<QObject*>("variantMark:selectionTool");QVERIFY(mark);QVERIFY(mark->property("visible").toBool());
        auto *panel=main->findChild<QObject*>("toolVariantPanel");QVERIFY(panel);QTRY_VERIFY(!panel->property("visible").toBool());
        // Left click only activates the tool, it never switches the shape.
        client.setSelectionTool(1);QTRY_COMPARE(client.selectionTool(),1);
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,selection->mapToScene({13,13}).toPoint());QTRY_COMPARE(client.selectionTool(),1);
        // Right click opens the floating list of shapes.
        QTest::mouseClick(main,Qt::RightButton,Qt::NoModifier,selection->mapToScene({13,13}).toPoint());QTRY_VERIFY(panel->property("visible").toBool());
        // Popup content belongs to the popup's own window, so search it from the panel content.
        auto *panelContent=qobject_cast<QQuickItem*>(panel->property("contentItem").value<QObject*>());QVERIFY(panelContent);
        auto *rectangle=findVisualItem(panelContent,"variantEntry:rectangle"),*ellipse=findVisualItem(panelContent,"variantEntry:ellipse");
        QVERIFY(rectangle && ellipse);
        QVERIFY(rectangle->isEnabled());QVERIFY(ellipse->isEnabled());
        // The wand remains unsupported; the lasso is now a separate usable family.
        QVERIFY(findVisualItem(main->contentItem(),"wandTool"));
        auto *lassoSlot=qobject_cast<QQuickItem*>(findVisualItem(main->contentItem(),"lassoTool"));
        QVERIFY(lassoSlot);QVERIFY(lassoSlot->isEnabled());
        QVERIFY(!findVisualItem(panelContent,"variantEntry:lasso"));
        QVERIFY(!findVisualItem(panelContent,"variantEntry:wand"));
        // Picking a shape from the panel is what changes the active shape.
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,findVisualItem(panelContent,"variantTrigger:ellipse")->mapToScene({20,15}).toPoint());QTRY_COMPARE(client.selectionTool(),2);QTRY_VERIFY(!panel->property("visible").toBool());
        QTest::mouseClick(main,Qt::RightButton,Qt::NoModifier,selection->mapToScene({13,13}).toPoint());QTRY_VERIFY(panel->property("visible").toBool());
        auto *selectedState=findVisualItem(panelContent,"toolVariantState:ellipse");QVERIFY(selectedState);QTRY_COMPARE(selectedState->property("text").toString(),QString("✓"));
        QVERIFY(QMetaObject::invokeMethod(panel,"close"));QTRY_VERIFY(!panel->property("visible").toBool());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
};
int main(int argc,char **argv) {
    configureUiScale();
    QQuickWindow::setDefaultAlphaBuffer(true);
#ifdef Q_OS_WIN
    // Offscreen does not use the Windows font database. Use installed system fonts.
    if (qEnvironmentVariable("QT_QPA_PLATFORM") == "offscreen")
        qputenv("QT_QPA_FONTDIR",QDir(qEnvironmentVariable("WINDIR")).filePath("Fonts").toUtf8());
#endif
    QGuiApplication app(argc,argv); Q_INIT_RESOURCE(ui_resources);
    QQuickStyle::setStyle("Basic"); qmlRegisterType<CanvasItem>("DrawVerse",1,0,"PaintCanvas");
    qmlRegisterType<ColorWheelItem>("DrawVerse",1,0,"ColorWheel");
    qmlRegisterType<SelectionOverlay>("DrawVerse",1,0,"SelectionOutline");
    qmlRegisterType<MenuSurface>("DrawVerse",1,0,"MenuSurface");
    qmlRegisterType<DocumentManager>("DrawVerse",1,0,"DocumentManager");
    UiTests tests; return QTest::qExec(&tests,argc,argv);
}
#include "ui_tests.moc"
