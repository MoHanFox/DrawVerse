#include "CanvasItem.h"
#include "ColorWheelItem.h"
#include "SelectionOverlay.h"
#include "WorkspaceManager.h"
#include "UiScale.h"
#include "MenuSurface.h"
#include <QtTest>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickWindow>
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
#include <QPainter>
#include <QScreen>
#include <cmath>
#include <limits>

namespace {
QQuickItem *findVisualItem(QQuickItem *root,const QString &name) {
    if(root->objectName()==name) return root;
    for(auto *item:root->childItems()) if(auto *found=findVisualItem(item,name)) return found;
    return nullptr;
}
}

class UiTests final : public QObject {
    Q_OBJECT
private slots:
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
    }
    void menusReopenWithoutGhostsAndFloatBesidePanels() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini")); QQmlApplicationEngine engine;
        QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &e:errors) warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/Main.qml")); QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window); QTRY_VERIFY(client.ready());
        auto *bar=qobject_cast<MenuSurface*>(findVisualItem(window->contentItem(),"menuBarGlassBackground"));
        QVERIFY(bar); QVERIFY(bar->tint().alpha()>0 && bar->tint().alpha()<255); QCOMPARE(bar->radius(),qreal(10));
        QCOMPARE(bar->tint().name(),QString("#202226")); QVERIFY(std::abs(bar->tint().alphaF()-.5)<.001); QVERIFY(bar->topCornersOnly());
        auto *entry=findVisualItem(window->contentItem(),QStringLiteral("menuEntry:工作区")); QVERIFY(entry);
        auto *focus=qobject_cast<QQuickItem*>(entry->property("background").value<QObject*>()); QVERIFY(focus);
        QCOMPARE(focus->property("radius").toReal(),qreal(0));
        auto *menu=window->findChild<QObject*>("workspaceMenu"); QVERIFY(menu);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,entry->mapToScene({entry->width()/2,entry->height()/2}).toPoint());
        QTRY_VERIFY(menu->property("opened").toBool());
        QCOMPARE(QGuiApplication::allWindows().size(),1);
        auto *glass=qobject_cast<MenuSurface*>(menu->property("background").value<QObject*>()); QVERIFY(glass);
        QCOMPARE(glass->radius(),qreal(10)); QVERIFY(glass->tint().alpha()<255);
        QCOMPARE(glass->tint().name(),QString("#1c1e21")); QVERIFY(std::abs(glass->tint().alphaF()-.75)<.001); QVERIFY(!glass->topCornersOnly());
        auto *list=qobject_cast<QQuickItem*>(menu->property("contentItem").value<QObject*>()); QVERIFY(list);
        auto *row=findVisualItem(list,QStringLiteral("glassMenuItem:自定义面板…")); QVERIFY(row);
        auto *label=qobject_cast<QQuickItem*>(row->property("contentItem").value<QObject*>()); QVERIFY(label);
        QCOMPARE(row->property("topPadding").toReal(),qreal(3)); QCOMPARE(row->property("bottomPadding").toReal(),qreal(3));
        QCOMPARE(row->implicitHeight()-label->implicitHeight(),qreal(6));
        const auto preview=qEnvironmentVariable("DRAWVERSE_MENU_PREVIEW");
        if(!preview.isEmpty()) { QTest::qWait(100); QVERIFY(window->grabWindow().save(preview)); }
        QTest::keyClick(window,Qt::Key_Escape); QTRY_VERIFY(!menu->property("visible").toBool());
        const auto menuImage=[&] {
            const auto image=window->grabWindow();
            const auto position=glass->mapToScene({0,0}); const auto dpr=window->devicePixelRatio();
            return image.copy(QRect(qRound(position.x()*dpr),qRound(position.y()*dpr),qRound(glass->width()*dpr),qRound(glass->height()*dpr)));
        };
        QVERIFY(QMetaObject::invokeMethod(menu,"open")); QTRY_VERIFY(menu->property("opened").toBool());
        QTest::qWait(30); const auto firstMenu=menuImage(); QVERIFY(!firstMenu.isNull());
        for(int cycle=0;cycle<6;++cycle) {
            QVERIFY(QMetaObject::invokeMethod(menu,"close")); QTRY_VERIFY(!menu->property("visible").toBool());
            QVERIFY(QMetaObject::invokeMethod(menu,"open")); QTRY_VERIFY(menu->property("opened").toBool());
            QTRY_COMPARE(menuImage(),firstMenu); QCOMPARE(bar->radius(),qreal(10));
        }
        QVERIFY(QMetaObject::invokeMethod(menu,"close")); QTRY_VERIFY(!menu->property("visible").toBool());
        window->showMaximized(); QTRY_COMPARE(bar->radius(),qreal(0));
        window->showNormal(); QTRY_COMPARE(bar->radius(),qreal(10));
        auto *windowEntry=findVisualItem(window->contentItem(),QStringLiteral("menuEntry:窗口")); QVERIFY(windowEntry);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,windowEntry->mapToScene({windowEntry->width()/2,windowEntry->height()/2}).toPoint());
        auto *windowMenu=window->findChild<QObject*>("windowMenu"); QVERIFY(windowMenu); QTRY_VERIFY(windowMenu->property("opened").toBool());
        auto *panelItem=windowMenu->findChild<QObject*>("windowPanel:color"); QVERIFY(panelItem); QVERIFY(panelItem->property("checked").toBool());
        QTest::keyClick(window,Qt::Key_Escape); QTRY_VERIFY(!windowMenu->property("visible").toBool());
        const auto group=workspace.rightGroups().first().toMap().value("id").toString(); workspace.detachPanel(group,"color");
        QTRY_COMPARE(QGuiApplication::allWindows().size(),2);
        QQuickWindow *floating=nullptr; for(auto *candidate:QGuiApplication::allWindows()) if(candidate!=window) floating=qobject_cast<QQuickWindow*>(candidate);
        QVERIFY(floating); const auto floatingId=workspace.floatingGroups().first().toMap().value("id").toString();
        floating->resize(150,300); floating->setPosition(floating->screen()->availableGeometry().topLeft()+QPoint(10,10));
        auto *button=findVisualItem(floating->contentItem(),"panelMenu:"+floatingId); QVERIFY(button);
        // Synthetic clicks do not activate a native Qt Tool window like real clicks do.
        floating->requestActivate(); QTRY_VERIFY(floating->isActive());
        QTRY_VERIFY(button->mapToScene({button->width()/2,button->height()/2}).x()<floating->width());
        QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,button->mapToScene({button->width()/2,button->height()/2}).toPoint());
        auto *panelMenu=floating->findChild<QObject*>("panelOperationsMenu:"+floatingId); QVERIFY(panelMenu); QTRY_VERIFY(panelMenu->property("opened").toBool());
        QTRY_COMPARE(QGuiApplication::allWindows().size(),3);
        auto *host=qobject_cast<QQuickWindow*>(panelMenu->property("sideWindow").value<QObject*>()); QVERIFY(host);
        QTRY_VERIFY(host->isVisible());
        QVERIFY(host->x()>=floating->x()+floating->width() || host->x()+host->width()<=floating->x());
        QTRY_VERIFY(host->isActive());
        QTRY_VERIFY(panelMenu->property("activeFocus").toBool());
        auto *floatingGlass=qobject_cast<MenuSurface*>(panelMenu->property("background").value<QObject*>()); QVERIFY(floatingGlass);
        if(!preview.isEmpty()) { QTest::qWait(30); QVERIFY(host->grabWindow().save(preview+".side.png")); }
        QTest::keyClick(host,Qt::Key_Down); QTest::keyClick(host,Qt::Key_Escape);
        QTRY_VERIFY(!panelMenu->property("visible").toBool());
        QTRY_COMPARE(QGuiApplication::allWindows().size(),2);
        floating->setX(floating->screen()->availableGeometry().right()-floating->width());
        QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,button->mapToScene({button->width()/2,button->height()/2}).toPoint());
        QTRY_VERIFY(panelMenu->property("opened").toBool());
        host=qobject_cast<QQuickWindow*>(panelMenu->property("sideWindow").value<QObject*>()); QVERIFY(host);
        QVERIFY(host->x()+host->width()<=floating->x());
        auto *returnItem=findVisualItem(host->contentItem(),QStringLiteral("glassMenuItem:返回工作区")); QVERIFY(returnItem);
        QTest::mouseClick(host,Qt::LeftButton,Qt::NoModifier,returnItem->mapToScene({returnItem->width()/2,returnItem->height()/2}).toPoint());
        QTRY_COMPARE(workspace.floatingGroups().size(),0); QTRY_COMPARE(QGuiApplication::allWindows().size(),1);
        QCOMPARE(warnings,QStringList()); QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
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
        QVERIFY(options); QCOMPARE(options->height(),qreal(28));
        const auto group=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachPanel(group,"color");
        workspace.floatToolStrip(window->x()+60,window->y()+80);
        QTRY_COMPARE(QGuiApplication::allWindows().size(),3);
        const qreal expected=qEnvironmentVariable("QT_SCALE_FACTOR").toDouble();
        QVERIFY(expected>=1.1);
        for(auto *nativeWindow:QGuiApplication::allWindows()) {
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
        workspace.floatToolStrip(250,150);QTRY_COMPARE(QGuiApplication::allWindows().size(),2);QQuickWindow *floating=nullptr;for(auto *w:QGuiApplication::allWindows())if(w!=window)floating=qobject_cast<QQuickWindow*>(w);QVERIFY(floating);auto *tool=findVisualItem(floating->contentItem(),"selectionTool");QVERIFY(tool);QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,tool->mapToScene({tool->width()/2,tool->height()/2}).toPoint());QTRY_COMPARE(client.selectionTool(),1);
        window->requestActivate();canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().size(),1);QTest::keyClick(window,Qt::Key_I,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(outline() && outline()->documentPath().isEmpty());QVERIFY(client.selectionEnabled());
        QTest::keyClick(window,Qt::Key_D,Qt::ControlModifier);QTRY_VERIFY(!client.selectionEnabled());QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!findVisualItem(window->contentItem(),"selectionOutline"));
        client.setSelectionTool(2);QPointingDevice pen("Selection tablet",1001,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,2);
        const QPointF start=position({20,20}),end=position({100,100});QTabletEvent press(QEvent::TabletPress,&pen,start,window->mapToGlobal(start.toPoint()),.6,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(window,&press);QVERIFY(press.isAccepted());QTabletEvent release(QEvent::TabletRelease,&pen,end,window->mapToGlobal(end.toPoint()),0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(window,&release);QVERIFY(release.isAccepted());QTRY_VERIFY(client.selectionEnabled());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().size(),1);QCOMPARE(client.selectionSteps().first().toMap().value("shape").toInt(),1);
        const QPoint center=position({60,60});const auto before=canvas->documentPoint(canvas->mapFromScene(center));QWheelEvent wheel(center,window->mapToGlobal(center),{},QPoint(0,120),Qt::NoButton,Qt::NoModifier,Qt::NoScrollPhase,false);QCoreApplication::sendEvent(window,&wheel);QVERIFY(QLineF(before,canvas->documentPoint(canvas->mapFromScene(center))).length()<.001);QTRY_VERIFY(outline() && outline()->documentPath().contains({60,60}));QVERIFY(!outline()->documentPath().contains({20,20}));
        const auto preview=qEnvironmentVariable("DRAWVERSE_SELECTION_PREVIEW");if(!preview.isEmpty()){QTest::qWait(200);QVERIFY(window->grabWindow().save(preview));}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void brushPresetsKeepIndependentSettingsAndRejectStalePreviews() {
        BrushLibrary library;QCOMPARE(library.radius(),qreal(12));library.setRadius(22);library.setOpacity(.4);library.setSpacing(.3);QVERIFY(library.select("round-fine"));QCOMPARE(library.radius(),qreal(3));QCOMPARE(library.opacity(),qreal(1));QVERIFY(library.select("round-pressure"));QCOMPARE(library.radius(),qreal(22));QCOMPARE(library.spacing(),qreal(.3));
        QSignalSpy requests(&library,&BrushLibrary::previewRequested);library.requestPreview(library.selectedId());QCOMPARE(requests.size(),1);const auto oldToken=requests.first()[1].toULongLong();library.setRadius(24);library.acceptPreview(library.selectedId(),oldToken,"stale");QVERIFY(library.selectedPreview().isEmpty());library.requestPreview(library.selectedId());QCOMPARE(requests.size(),2);library.acceptPreview(library.selectedId(),requests.last()[1].toULongLong(),"fresh");QCOMPARE(library.selectedPreview(),QString("fresh"));
        const auto copy=library.saveCopy(QStringLiteral("自定义圆笔"));QVERIFY(!copy.isEmpty());QCOMPARE(library.selectedId(),copy);QVERIFY(library.rename(copy,QStringLiteral("自定义圆笔二")));QVERIFY(!library.rename(copy," "));QVERIFY(!library.remove("round-pressure"));
        BrushLibrary restored;QVERIFY(restored.restore(library.snapshot()));QCOMPARE(restored.selectedId(),copy);QCOMPARE(restored.radius(),qreal(24));QCOMPARE(restored.opacity(),qreal(.4));QCOMPARE(restored.presets().size(),5);
        auto invalid=QJsonDocument::fromJson(library.snapshot()).object();auto rows=invalid.value("presets").toArray();auto row=rows.first().toObject();row["radius"]=-1;rows[0]=row;invalid["presets"]=rows;QVERIFY(!restored.restore(QJsonDocument(invalid).toJson()));QCOMPARE(restored.radius(),qreal(24));QVERIFY(!restored.restore("not json"));library.setRadius(std::numeric_limits<double>::quiet_NaN());QCOMPARE(library.radius(),qreal(24));
        QVERIFY(restored.remove(copy));QCOMPARE(restored.selectedId(),QString("round-pressure"));QCOMPARE(restored.presets().size(),4);QVERIFY(!restored.select("missing"));
        library.retryPreview();const auto failedToken=requests.last()[1].toULongLong();library.acceptPreview(library.selectedId(),failedToken,{});QCOMPARE(library.previewMessage(),QStringLiteral("预览暂不可用"));const auto count=requests.size();library.requestPreview(library.selectedId());QCOMPARE(requests.size(),count);library.retryPreview();QCOMPARE(requests.size(),count+1);
    }
    void brushLibrarySettingsAndEnginePreviewsShareSelectionAcrossWindows() {
        QTemporaryDir temp;const auto preferences=temp.filePath("storage.ini");PaintCoreClient client(nullptr,preferences);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();QVERIFY(library);const auto revision=client.revision();const int undo=client.undoDepth();const bool modified=client.modified();
        const auto group=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachPanel(group,"brush");QTRY_COMPARE(QGuiApplication::allWindows().size(),2);QQuickWindow *floating=nullptr;for(auto *w:QGuiApplication::allWindows())if(w!=window)floating=qobject_cast<QQuickWindow*>(w);QVERIFY(floating);floating->resize(190,500);
        QTRY_VERIFY2(findVisualItem(floating->contentItem(),"brushPreset:round-fine"),qPrintable(warnings.join('\n')));auto *fine=findVisualItem(floating->contentItem(),"brushPreset:round-fine");QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,fine->mapToScene({fine->width()/2,fine->height()/2}).toPoint());QTRY_COMPARE(library->selectedId(),QString("round-fine"));auto *label=findVisualItem(window->contentItem(),"selectedBrushName");QVERIFY(label);QTRY_COMPARE(label->property("text").toString(),QStringLiteral("细线圆笔"));
        auto *size=findVisualItem(window->contentItem(),"settingsBrushSizeText");QVERIFY(size);window->requestActivate();size->forceActiveFocus();QTRY_VERIFY(size->hasActiveFocus());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,size->mapToScene({size->width()/2,size->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_4);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(client.brushRadius(),qreal(20));
        QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto image=[](const QString &url){QImage result;result.loadFromData(QByteArray::fromBase64(url.mid(url.indexOf(',')+1).toLatin1()));return result;};const auto opaque=image(library->selectedPreview());QVERIFY(!opaque.isNull());QCOMPARE(opaque.size(),QSize(240,60));QVERIFY(opaque.pixelColor(0,0).alpha()==0);int alpha=0;for(int y=0;y<60;++y)for(int x=0;x<240;++x)alpha+=opaque.pixelColor(x,y).alpha();QVERIFY(alpha>1000);
        library->setOpacity(.15);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto transparent=image(library->selectedPreview());QVERIFY(!transparent.isNull());int fadedAlpha=0;for(int y=0;y<60;++y)for(int x=0;x<240;++x)fadedAlpha+=transparent.pixelColor(x,y).alpha();QVERIFY(fadedAlpha<alpha);QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),undo);QCOMPARE(client.modified(),modified);
        library->setSpacing(.6);QCOMPARE(client.brushSpacing(),qreal(.6));QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(image(library->selectedPreview())!=transparent);QVERIFY(library->select("round-pressure"));QCOMPARE(client.brushRadius(),qreal(12));QVERIFY(library->select("round-fine"));QCOMPARE(client.brushRadius(),qreal(20));QCOMPARE(client.brushOpacity(),qreal(.15));
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());client.setBrushColor(Qt::black);InputSample sample;sample.position={20,64};sample.pressure=.6f;sample.tool=1;QVERIFY(client.beginStroke(sample));sample.position={100,64};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());auto *previewCanvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(previewCanvas);previewCanvas->setClient(nullptr);client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,128));const int paintedAlpha=client.frame().pixelColor(60,64).alpha();QVERIFY(paintedAlpha>0 && paintedAlpha<250);previewCanvas->setClient(&client);
        const auto copy=library->saveCopy(QStringLiteral("保留的画笔"));QVERIFY(!copy.isEmpty());QTRY_COMPARE(label->property("text").toString(),QStringLiteral("保留的画笔"));
        const auto preview=qEnvironmentVariable("DRAWVERSE_BRUSH_PREVIEW");if(!preview.isEmpty()){QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);floating->resize(250,560);QTest::qWait(250);QVERIFY(floating->grabWindow().save(preview+".library.png"));auto settingsGroup=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(settingsGroup);QTRY_COMPARE(QGuiApplication::allWindows().size(),3);for(auto *w:QGuiApplication::allWindows())if(auto *q=qobject_cast<QQuickWindow*>(w))if(findVisualItem(q->contentItem(),"selectedBrushName")){q->resize(280,440);QTest::qWait(200);QVERIFY(q->grabWindow().save(preview+".settings.png"));}}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reloaded(nullptr,preferences);QTRY_VERIFY(reloaded.ready());QTRY_COMPARE(reloaded.brushLibrary()->selectedId(),copy);QCOMPARE(reloaded.brushRadius(),qreal(20));QCOMPARE(reloaded.brushSpacing(),qreal(.6));QSignalSpy stoppedAgain(&reloaded,&PaintCoreClient::stopped);reloaded.shutdown();QTRY_COMPARE(stoppedAgain.size(),1);
    }
    void toolStripFloatsDocksPersistsAndTabsJoinContent() {
        QTemporaryDir temp;const auto path=temp.filePath("tools.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());QCOMPARE(QGuiApplication::allWindows().size(),1);
        const auto group=workspace.rightGroups().last().toMap().value("id").toString();auto *panel=findVisualItem(window->contentItem(),"dockGroup:"+group);QVERIFY(panel);
        auto *selected=findVisualItem(panel,"panelTab:layers");auto *inactive=findVisualItem(panel,"panelTab:history");QVERIFY(selected && inactive);
        QCOMPARE(selected->property("color"),panel->property("color"));QVERIFY(inactive->property("color").value<QColor>().lightness()<selected->property("color").value<QColor>().lightness());
        workspace.setActive(group,"history");QTRY_COMPARE(inactive->property("color"),panel->property("color"));QVERIFY(selected->property("color").value<QColor>().lightness()<inactive->property("color").value<QColor>().lightness());workspace.setActive(group,"layers");
        QVERIFY(!workspace.dockToolStrip("invalid"));QVERIFY(!workspace.toolsFloating());
        auto *grip=findVisualItem(window->contentItem(),"toolStripGrip");QVERIFY(grip);QTest::mouseDClick(window,Qt::LeftButton,Qt::NoModifier,grip->mapToScene({grip->width()/2,grip->height()/2}).toPoint());QTRY_VERIFY(workspace.toolsFloating());QTRY_COMPARE(QGuiApplication::allWindows().size(),2);
        QQuickWindow *tools=nullptr;for(auto *w:QGuiApplication::allWindows())if(w->objectName()=="floatingToolStrip")tools=qobject_cast<QQuickWindow*>(w);QVERIFY(tools);QCOMPARE(tools->width(),38);
        auto *eraser=findVisualItem(tools->contentItem(),"eraserTool");QVERIFY(eraser);QTest::mouseClick(tools,Qt::LeftButton,Qt::NoModifier,eraser->mapToScene({13,13}).toPoint());QTRY_VERIFY(client.eraser());
        auto *brush=findVisualItem(tools->contentItem(),"brushTool");QVERIFY(brush);QTest::mouseClick(tools,Qt::LeftButton,Qt::NoModifier,brush->mapToScene({13,13}).toPoint());QTRY_VERIFY(!client.eraser());
        tools->setPosition(window->x()+70,window->y()+130);QTRY_COMPARE(workspace.toolStripY(),tools->y());workspace.saveLayout();WorkspaceManager restored(path);QVERIFY(restored.toolsFloating());QCOMPARE(restored.toolStripX(),tools->x());QCOMPARE(restored.toolStripY(),tools->y());
        auto drop=[&](Qt::KeyboardModifiers modifiers,const QByteArray &data){QMimeData mime;mime.setData("application/x-drawverse-tool-strip",data);const QPoint point(12,100);QDragEnterEvent enter(point,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(window,&enter);QDragMoveEvent move(point,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(window,&move);QDropEvent end(point,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(window,&end);return end.isAccepted();};
#ifdef Q_OS_MACOS
        const auto suppress=Qt::MetaModifier;
#else
        const auto suppress=Qt::ControlModifier;
#endif
        QVERIFY(!drop(suppress,"drawverse-tools-v1"));QVERIFY(workspace.toolsFloating());QVERIFY(!drop(Qt::NoModifier,"bad"));QVERIFY(workspace.toolsFloating());QVERIFY(drop(Qt::NoModifier,"drawverse-tools-v1"));QTRY_VERIFY(!workspace.toolsFloating());QTRY_COMPARE(QGuiApplication::allWindows().size(),1);
        workspace.floatToolStrip(window->x()+90,window->y()+150);QTRY_COMPARE(QGuiApplication::allWindows().size(),2);for(auto *w:QGuiApplication::allWindows())if(w->objectName()=="floatingToolStrip")w->close();QTRY_VERIFY(!workspace.toolsFloating());QTRY_COMPARE(QGuiApplication::allWindows().size(),1);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void referencePresetUsesCompactNativeFloatingWindows() {
        QTemporaryDir temp;const auto path=temp.filePath("reference.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());QVERIFY(window->flags().testFlag(Qt::FramelessWindowHint));
        client.newDocument(1998,1133);QTRY_COMPARE(client.documentWidth(),1998);QTRY_VERIFY(client.ready());client.setBrushColor(QColor("#f5e3ce"));
        workspace.applyReferenceLayout(window->x(),window->y(),window->width(),window->height());QTRY_COMPARE(workspace.rightGroups().size(),2);QTRY_COMPARE(workspace.leftGroups().size(),0);QTRY_COMPARE(QGuiApplication::allWindows().size(),3);
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);QTRY_VERIFY(canvas->width()>window->width()*.8);
        auto *options=findVisualItem(window->contentItem(),"brushOptionsBar");QVERIFY(options);QCOMPARE(options->height(),qreal(28));
        QQuickWindow *colorWindow=nullptr,*brushWindow=nullptr;
        for(const auto &v:workspace.floatingGroups()) {
            const auto g=v.toMap();QQuickItem *panel=nullptr;
            for(auto *w:QGuiApplication::allWindows())if(auto *quick=qobject_cast<QQuickWindow*>(w))if((panel=findVisualItem(quick->contentItem(),"dockGroup:"+g.value("id").toString()))){if(g.value("active")=="color")colorWindow=quick;else brushWindow=quick;break;}
        }
        QVERIFY(colorWindow && brushWindow);QCOMPARE(colorWindow->width(),155);QTRY_COMPARE(brushWindow->height(),56);QVERIFY(colorWindow->flags().testFlag(Qt::FramelessWindowHint));QVERIFY(brushWindow->flags().testFlag(Qt::FramelessWindowHint));
        auto *wheel=qobject_cast<ColorWheelItem*>(findVisualItem(colorWindow->contentItem(),"colorWheel"));QVERIFY(wheel);QTRY_VERIFY(wheel->height()>=100);QVERIFY(findVisualItem(brushWindow->contentItem(),"collapsedPanel:brush-settings"));
        QPointer<QQuickWindow> retained=colorWindow;workspace.saveLayout();WorkspaceManager restored(path);QCOMPARE(restored.floatingGroups().size(),2);QVERIFY(!restored.needsReferenceLayout());
        const auto preview=qEnvironmentVariable("DRAWVERSE_REFERENCE_PREVIEW");
        if(!preview.isEmpty()) {
            QTRY_VERIFY(client.frameRevision()>=client.revision());QTest::mouseMove(window,{20,window->height()-20});QTest::qWait(250);
            QImage scene=window->grabWindow();QVERIFY(!scene.isNull());scene.setDevicePixelRatio(1);
            QPainter composite(&scene);
            for(auto *floating:QList<QQuickWindow*>{brushWindow,colorWindow}) {
                const auto shot=floating->grabWindow();QVERIFY(!shot.isNull());QVERIFY(shot.save(preview+(floating==colorWindow?".color.png":".brush.png")));
                const qreal dpr=window->devicePixelRatio();composite.drawImage(QRectF((floating->x()-window->x())*dpr,(floating->y()-window->y())*dpr,floating->width()*dpr,floating->height()*dpr),shot);
            }
            composite.end();QVERIFY(scene.save(preview+".composite.png"));QVERIFY(window->grabWindow().save(preview+".main.png"));
        }
        InputSample point;point.position={900,500};client.setBrushRadius(20);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());point.position={1100,500};QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());
        auto *initial=findVisualItem(window->contentItem(),"historyEntry:0");QVERIFY(initial);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,initial->mapToScene({initial->width()/2,initial->height()/2}).toPoint());QTRY_COMPARE(client.undoDepth(),0);QTRY_COMPARE(client.redoDepth(),2);
        auto *last=findVisualItem(window->contentItem(),"historyEntry:2");QVERIFY(last);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,last->mapToScene({last->width()/2,last->height()/2}).toPoint());QTRY_COMPARE(client.undoDepth(),2);QVERIFY(retained);
        auto *maximize=findVisualItem(window->contentItem(),"windowMaximize");QVERIFY(maximize);QVERIFY(QMetaObject::invokeMethod(maximize,"clicked"));QTRY_COMPARE(window->visibility(),QWindow::Maximized);QVERIFY(QMetaObject::invokeMethod(maximize,"clicked"));QTRY_COMPARE(window->visibility(),QWindow::Windowed);
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
        workspace.setActive(second,"history");workspace.setGroupCollapsed(second,true);QCOMPARE(structural.size(),0);
        workspace.setLeftDockWidth(274);workspace.setRightDockWidth(354);workspace.setRightCollapsed(true);workspace.updateDockHeight(first,279);
        workspace.hidePanel(second,custom);QVERIFY(!workspace.visiblePanels().contains(custom));QVERIFY(workspace.allPanels().contains(custom));
        workspace.saveLayout();WorkspaceManager reopened(path);QCOMPARE(reopened.leftDockWidth(),274);QCOMPARE(reopened.rightDockWidth(),354);QVERIFY(reopened.rightCollapsed());
        QVERIFY(reopened.groupDefinition(second).value("collapsed").toBool());QCOMPARE(reopened.groupDefinition(second).value("active").toString(),QString("history"));
        QCOMPARE(reopened.groupDefinition(first).value("dockHeight").toInt(),279);QVERIFY(!reopened.visiblePanels().contains(custom));
        reopened.showPanel(custom);QVERIFY(reopened.visiblePanels().contains(custom));QVERIFY(!reopened.rightCollapsed());
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
            QDragEnterEvent enter(at,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(host,&enter);if(!enter.isAccepted())return false;
            QDragMoveEvent move(at,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(host,&move);
            auto *hint=findVisualItem(host->contentItem(),"dockPreview:"+target);if(!hint || !hint->isVisible())return false;
            if(!preview.isEmpty()){QTest::qWait(100);if(!host->grabWindow().save(preview+".drop.png"))return false;}
            QDropEvent event(at,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(host,&event);return event.isAccepted();
        };
        QVERIFY(drop(second,"history",first,-1,80));QTRY_COMPARE(workspace.groupDefinition(first).value("panels").toStringList(),(QStringList{"color","history"}));
        QTest::qWait(50);QVERIFY(drop(first,"history",first,8,20));QTRY_COMPARE(workspace.groupDefinition(first).value("panels").toStringList().first(),QString("history"));
        QTest::qWait(50);QVERIFY(drop(first,"history",second,-1,4));QTRY_COMPARE(workspace.rightGroups().size(),3);const auto middle=workspace.rightGroups()[1].toMap().value("id").toString();
        QTest::qWait(50);QVERIFY(drop(middle,"history",second,-1,-1));QTRY_COMPARE(workspace.rightGroups().last().toMap().value("panels").toStringList(),QStringList{"history"});
        QTest::qWait(50);QVERIFY(!drop(first,"color",second,-1,80,Qt::ControlModifier));QCOMPARE(workspace.rightGroups().size(),3);
        workspace.resetLayout();QTest::qWait(80);first=workspace.rightGroups().first().toMap().value("id").toString();
        auto click=[&](const QString &name){auto *item=findVisualItem(window->contentItem(),name);if(!item)return false;QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,item->mapToScene({item->width()/2,item->height()/2}).toPoint());return true;};
        QVERIFY(click("groupCollapse:"+first));QTRY_VERIFY(workspace.groupDefinition(first).value("collapsed").toBool());QTRY_VERIFY(findVisualItem(window->contentItem(),"dockGroup:"+first)->height()<=45);QVERIFY(click("collapsedPanel:color"));QTRY_VERIFY(!workspace.groupDefinition(first).value("collapsed").toBool());
        QVERIFY(click("dockCollapse:right"));QTRY_VERIFY(workspace.rightCollapsed());QTRY_VERIFY(findVisualItem(window->contentItem(),"railPanel:layers")->isVisible());QVERIFY(capture(".collapsed"));
        QVERIFY(click("railPanel:layers"));QVERIFY(workspace.rightCollapsed());QTRY_VERIFY(findVisualItem(window->contentItem(),"layerList"));QTRY_VERIFY(findVisualItem(window->contentItem(),"layerList")->isVisible());QVERIFY(capture(".peek"));
        QVERIFY(click("dockCollapse:right"));QTRY_VERIFY(!workspace.rightCollapsed());
        workspace.hidePanel(first,"color");QTRY_VERIFY(!workspace.visiblePanels().contains("color"));workspace.showPanel("color");QTRY_VERIFY(workspace.visiblePanels().contains("color"));
        const auto brush=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(brush);QTRY_COMPARE(QGuiApplication::allWindows().size(),2);
        QQuickWindow *floating=nullptr;for(auto *w:QGuiApplication::allWindows())if(w!=window)floating=qobject_cast<QQuickWindow*>(w);QVERIFY(floating);
        if(!preview.isEmpty()){QTest::qWait(150);QVERIFY(floating->grabWindow().save(preview+".floating.png"));}
        QPointer<QQuickWindow> retained=floating;
        const auto custom=workspace.addCustomPanel("Saved colors","palette");QVERIFY(!custom.isEmpty());QTest::qWait(50);QVERIFY(retained);QCOMPARE(QGuiApplication::allWindows().size(),2);
        second=workspace.rightGroups().last().toMap().value("id").toString();QVERIFY(drop(second,"navigator",brush,-1,80,Qt::NoModifier,floating));QTRY_VERIFY(workspace.groupDefinition(brush).value("panels").toStringList().contains("navigator"));QVERIFY(retained);QCOMPARE(QGuiApplication::allWindows().size(),2);
        workspace.setGroupCollapsed(brush,true);QTRY_VERIFY(floating->height()<=8+22*workspace.groupDefinition(brush).value("panels").toStringList().size()+4);workspace.setGroupCollapsed(brush,false);QTRY_VERIFY(floating->height()>=200);
        floating->close();QTRY_COMPARE(workspace.leftGroups().size(),1);QTRY_COMPARE(QGuiApplication::allWindows().size(),1);
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
        auto *raw=findVisualItem(window->contentItem(),"layerThumbnail:1");auto *maskThumb=findVisualItem(window->contentItem(),"layerThumbnail:"+QString::number(mask));QVERIFY(raw && maskThumb);QVERIFY(maskThumb->mapToScene({0,0}).x()>raw->mapToScene({raw->width(),0}).x());QCOMPARE(maskThumb->mapToScene({0,0}).y(),raw->mapToScene({0,0}).y());
        auto hit=[&](QQuickWindow *w,quint64 id,Qt::KeyboardModifiers modifiers=Qt::NoModifier){QTest::qWait(50);auto *i=findVisualItem(w->contentItem(),"layerThumbnailHit:"+QString::number(id));if(!i || !i->isVisible())return false;QTest::mouseClick(w,Qt::LeftButton,modifiers,i->mapToScene({i->width()/2,i->height()/2}).toPoint());return true;};
        QTRY_VERIFY(list->height()>=46);
        QVERIFY(hit(window,1));QTRY_COMPARE(client.activeLayer(),quint64(1));QTRY_VERIFY(!client.layerEditBusy());QVERIFY(hit(window,mask));QTRY_COMPARE(client.activeLayer(),mask);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushColor(Qt::black);client.setBrushRadius(9);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(sample(64.5,64.5).alpha(),0);
        QVERIFY(hit(window,mask,Qt::ShiftModifier));QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!layer(mask).value("visible").toBool());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(sample(64.5,64.5).alpha(),255);QVERIFY(hit(window,mask,Qt::ShiftModifier));QTRY_VERIFY(!client.layerEditBusy());
        const auto dock=workspace.rightGroups().last().toMap().value("id").toString();workspace.detachPanel(dock,"layers");QTRY_COMPARE(workspace.floatingGroups().size(),1);QQuickWindow *floating=nullptr;QTRY_VERIFY(([&](){for(auto *w:QGuiApplication::allWindows())if(w!=window){floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}return false;})());QTRY_VERIFY(findVisualItem(floating->contentItem(),"clippingBoundary:"+QString::number(top)));
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
        auto *opacity=findVisualItem(window->contentItem(),"layerOpacity");QVERIFY(opacity);QVERIFY(QMetaObject::invokeMethod(opacity,"committed",Q_ARG(double,.5)));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(1).value("opacity").toFloat(),.5f);QCOMPARE(layer(mask).value("opacity").toFloat(),1.f);client.undo();QTRY_VERIFY(!client.layerEditBusy());
        auto *density=findVisualItem(window->contentItem(),"maskDensity");QVERIFY(density);QVERIFY(QMetaObject::invokeMethod(density,"committed",Q_ARG(double,.5)));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(mask).value("opacity").toFloat(),.5f);QCOMPARE(layer(1).value("opacity").toFloat(),1.f);client.undo();QTRY_VERIFY(!client.layerEditBusy());
        client.selectLayer(1);QTRY_VERIFY(!client.layerEditBusy());client.groupLayer(1,"绘画组");QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();QCOMPARE(layer(mask).value("depth").toInt(),2);
        auto *thumbnail=findVisualItem(window->contentItem(),"layerThumbnail:"+QString::number(group));QVERIFY(thumbnail);QVERIFY(!thumbnail->isVisible());
        client.selectLayer(group);QTRY_VERIFY(!client.layerEditBusy());client.addDefaultLayer();QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());const auto first=client.activeLayer();QCOMPARE(layer(first).value("name").toString(),QStringLiteral("图层一"));
        client.addDefaultLayer();QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());const auto second=client.activeLayer();QCOMPARE(layer(second).value("name").toString(),QStringLiteral("图层二"));
        QVERIFY(client.dropLayer(first,0,0));QTRY_COMPARE(layer(first).value("parent").toULongLong(),quint64(0));QTRY_VERIFY(!client.layerEditBusy());
        const auto dockGroup=workspace.rightGroups().last().toMap().value("id").toString();workspace.detachPanel(dockGroup,"layers");QTRY_COMPARE(workspace.floatingGroups().size(),1);QQuickWindow *floating=nullptr;QTRY_VERIFY(([&](){for(auto *w:QGuiApplication::allWindows())if(w!=window){floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}return false;})());
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
        const auto dock=workspace.rightGroups().last().toMap().value("id").toString();workspace.detachPanel(dock,"layers");QTRY_COMPARE(QGuiApplication::allWindows().size(),2);
        QQuickWindow *floating=nullptr;for(auto *candidate:QGuiApplication::allWindows()) if(candidate!=window) floating=qobject_cast<QQuickWindow*>(candidate);QVERIFY(floating);
        list=findVisualItem(floating->contentItem(),"layerList");QVERIFY(list);QTRY_COMPARE(list->property("count").toInt(),1);QTRY_VERIFY((expand=findVisualItem(floating->contentItem(),"groupExpand:"+QString::number(group))));QVERIFY(QMetaObject::invokeMethod(expand,"clicked"));QTRY_COMPARE(list->property("count").toInt(),3);
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
        workspace.returnGroup(workspace.floatingGroups().first().toMap().value("id").toString());QTRY_COMPARE(QGuiApplication::allWindows().size(),1);
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
        auto *opacity=findVisualItem(window->contentItem(),"layerOpacity"); QVERIFY(opacity);
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
        QTRY_COMPARE(QGuiApplication::allWindows().size(),2);
        QQuickWindow *floating=nullptr; for(auto *candidate:QGuiApplication::allWindows()) if(candidate!=window) floating=qobject_cast<QQuickWindow*>(candidate);
        QVERIFY(floating); auto *unlock=findVisualItem(floating->contentItem(),"lockAll"); QVERIFY(unlock);
        QVERIFY(QMetaObject::invokeMethod(unlock,"clicked")); QTRY_COMPARE(layer().value("locks").toInt(),3); QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(findVisualItem(floating->contentItem(),"layerOpacity")->isEnabled());
        workspace.returnGroup(workspace.floatingGroups().first().toMap().value("id").toString()); QTRY_COMPARE(QGuiApplication::allWindows().size(),1);
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
        client.clearError(); client.setEraser(false); sample.position={2.5,16.5}; QVERIFY(client.beginStroke(sample)); client.endStroke(); QTRY_VERIFY(client.revision()>revision);
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
        auto *dialog=window->findChild<QObject*>("storagePreferences"); QVERIFY(dialog);
        QVERIFY(QMetaObject::invokeMethod(dialog,"open")); QTRY_VERIFY(!client.storageBusy());
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
        auto *dialog=window->findChild<QObject*>("storagePreferences"); QVERIFY(dialog);
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
        QCOMPARE(saved.value("version").toInt(),4); QVERIFY(!saved.value("panels").toObject().contains("old-note"));
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
        QCOMPARE(QGuiApplication::allWindows().size(),1);
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
        QTRY_COMPARE(QGuiApplication::allWindows().size(),2);
        workspace.returnGroup(workspace.floatingGroups().first().toMap().value("id").toString());
        QTRY_COMPARE(QGuiApplication::allWindows().size(),1);
        const QString notes=workspace.addCustomPanel("Palette","palette");
        auto source=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachPanel(source,notes); QTRY_COMPARE(QGuiApplication::allWindows().size(),2);
        auto floating=workspace.floatingGroups().first().toMap().value("id").toString();
        QQuickItem *target=nullptr;
        QTRY_VERIFY((target=findVisualItem(window->contentItem(),"dockGroup:"+source)));
        QTRY_VERIFY(target->width()>=180 && target->height()>=180);
        const auto dropPosition=target->mapToScene(QPointF(target->width()/2,80)).toPoint();
        QMimeData mime;
        mime.setData("application/x-drawverse-panel",QJsonDocument(QJsonObject{{"group",floating},{"panel",notes},{"whole",false}}).toJson());
        QDragEnterEvent enter(dropPosition,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);
        QCoreApplication::sendEvent(window,&enter); QVERIFY(enter.isAccepted());
        QDropEvent drop(dropPosition,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);
        QCoreApplication::sendEvent(window,&drop); QVERIFY(drop.isAccepted());
        QTRY_COMPARE(workspace.floatingGroups().size(),0); QTRY_COMPARE(QGuiApplication::allWindows().size(),1);
        QCOMPARE(workspace.rightGroups().size(),2);
        QVERIFY(workspace.rightGroups().first().toMap().value("panels").toStringList().contains(notes));
        workspace.detachPanel(source,notes); QTRY_COMPARE(QGuiApplication::allWindows().size(),2);
        QQuickWindow *floatingWindow=nullptr;
        for(auto *candidate:QGuiApplication::allWindows()) if(candidate!=window) floatingWindow=qobject_cast<QQuickWindow*>(candidate);
        QVERIFY(floatingWindow); floatingWindow->close();
        QTRY_COMPARE(workspace.floatingGroups().size(),0); QTRY_COMPARE(QGuiApplication::allWindows().size(),1);
        QCOMPARE(warnings,QStringList());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
        QVERIFY(window->close()); QVERIFY(!window->isVisible());
    }
};
int main(int argc,char **argv) {
    configureUiScale();
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
    UiTests tests; return QTest::qExec(&tests,argc,argv);
}
#include "ui_tests.moc"
