#include "platform/UiResources.h"
#include "canvas/CanvasItem.h"
#include "canvas/SelectionOverlay.h"
#include "color/ColorWheelItem.h"
#include "documents/DocumentManager.h"
#include "workspace/WorkspaceManager.h"
#include "platform/MenuSurface.h"
#include "platform/TabletInput.h"
#include "platform/InteractionTrace.h"
#include "PendingMoveEvidence.h"
#include <QtTest>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQmlComponent>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QQuickItemGrabResult>
#include <QTemporaryDir>
#include <QTimer>
#include <QSettings>
#include <QJsonDocument>
#include <QTabletEvent>
#include <QPointingDevice>
#include <QClipboard>
#include <QMimeData>
#include <QScopeGuard>
#include <QScreen>
#include <qpa/qwindowsysteminterface.h>
#include <private/qhighdpiscaling_p.h>
#ifdef Q_OS_WIN
#include <QtCore/qt_windows.h>
#undef near
#endif
static QQuickItem *findVisualItem(QQuickItem *item,const QString &name){if(item->objectName()==name)return item;for(auto *child:item->childItems())if(auto *found=findVisualItem(child,name))return found;return nullptr;}
class TransformTests : public QObject {
    Q_OBJECT
private slots:
    void initTestCase() {
        configureTabletInput("auto");
        if(auto *screen=QGuiApplication::primaryScreen())qInfo()<<"B1 native test screen:"<<screen->name()<<"logical DPI:"<<screen->logicalDotsPerInch()<<"DPR:"<<screen->devicePixelRatio();
        for(const auto *device:QInputDevice::devices())qInfo()<<"Initial Qt device:"<<device->name()<<device->systemId()<<device->type();
    }
    void cursorDirectionTurnsRebasesAndWraps() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("direction-diagnosis.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newTransparentDocument(256,256);QTRY_VERIFY(client.ready() && client.documentWidth()==256 && !client.layerEditBusy());
        QQuickWindow window;window.resize(500,500);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({500,500});canvas->setClient(&client);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        auto *library=client.brushLibrary();library->setRadius(12);library->setAngle(0);library->setRoundness(.3);library->setAngleControl("direction");
        QPointingDevice pen("B2 direction diagnosis pen",9304,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        QJsonArray states;QPointF current(200,200);InteractionTrace::start(true);
        const auto evidence=qScopeGuard([&]{const auto trace=InteractionTrace::finish();const auto dir=qEnvironmentVariable("DRAWVERSE_B2_EVIDENCE_DIR");if(!dir.isEmpty()){QFile file(QDir(dir).filePath("direction-diagnosis.json"));if(file.open(QIODevice::WriteOnly))file.write(QJsonDocument(QJsonObject{{"synthetic_input",true},{"states",states},{"trace",trace}}).toJson());}});
        const auto hover=[&](QPointF at){QTabletEvent event(QEvent::TabletMove,&pen,at,canvas->mapToGlobal(at),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(canvas,&event);current=at;};
        const auto record=[&](const char *stage,QPointF delta){states.append(QJsonObject{{"stage",stage},{"time_ns",double(InteractionTrace::now())},{"device",pen.name()},{"canvas",QString::number(quintptr(canvas),16)},{"x",current.x()},{"y",current.y()},{"dx",delta.x()},{"dy",delta.y()},{"document_x",canvas->documentPoint(current).x()},{"document_y",canvas->documentPoint(current).y()},{"zoom",canvas->zoom()},{"angle",canvas->brushCursorAngle()},{"ellipse_angle_mod180",std::remainder(canvas->brushCursorAngle(),180.)},{"radius",canvas->brushCursorRect().width()/canvas->zoom()/2},{"roundness",canvas->brushCursorRoundness()},{"pending",library->persistencePending()},{"visible",canvas->brushCursorVisible()}});};
        hover(current);hover(current+QPointF(10,10));QTRY_COMPARE(canvas->brushCursorAngle(),45.);record("stable-diagonal",{10,10});
        for(int i=0;i<5;++i){hover(current);QCOMPARE(canvas->brushCursorAngle(),45.);record("stationary",{});}
        for(int i=0;i<3;++i){hover(current+QPointF(.1,.05));QVERIFY(qAbs(canvas->brushCursorAngle()-26.565051)<.001);record("slow-stable",{.1,.05});}
        hover(current+QPointF(0,10));QCOMPARE(canvas->brushCursorAngle(),90.);record("stop-and-turn",{0,10});
        hover(current+QPointF(-10,.01));const auto upper=canvas->brushCursorAngle();record("wrap-upper",{-10,.01});
        hover(current+QPointF(-10,-.01));const auto lower=canvas->brushCursorAngle();record("wrap-lower",{-10,-.01});
        QVERIFY(upper>179 && lower< -179);QVERIFY(qAbs(std::remainder(upper-lower,180.))<1.);
        // This amplitude is a synthetic probe, not a measured tablet noise distribution.
        for(const auto &delta:{QPointF(.0001,0),QPointF(0,.0001),QPointF(-.0001,0),QPointF(0,-.0001)}){hover(current+delta);const auto expected=std::atan2(delta.y(),delta.x())*180/3.141592653589793;QVERIFY(qAbs(canvas->brushCursorAngle()-expected)<.001);record("synthetic-near-still",delta);}
        canvas->zoomBy(1.5);QTRY_VERIFY(qAbs(canvas->zoom()-1.5)<1e-9);hover(current+QPointF(0,10));QCOMPARE(canvas->brushCursorAngle(),90.);record("after-zoom",{0,10});
        const auto beforePan=canvas->documentPoint({0,0});
        const auto mouseStart=QPoint(180,180);QTest::mousePress(&window,Qt::MiddleButton,Qt::NoModifier,mouseStart);QTest::mouseMove(&window,mouseStart+QPoint(25,10));QTest::mouseRelease(&window,Qt::MiddleButton,Qt::NoModifier,mouseStart+QPoint(25,10));canvas->zoomBy(1.);
        QVERIFY(QLineF(beforePan,canvas->documentPoint({0,0})).length()>1);
        hover(current);hover(current+QPointF(0,10));QCOMPARE(canvas->brushCursorAngle(),90.);record("after-pan",{0,10});
        library->setRadius(24);QCOMPARE(canvas->brushCursorRect().width()/canvas->zoom(),48.); // Invalidate incompatible old size immediately.
        library->setAngleControl("fixed");library->setAngle(27);QTRY_VERIFY(!library->persistencePending());QTRY_COMPARE(canvas->brushCursorAngle(),27.);record("new-settings",{});
        QVERIFY(library->select("round-fine"));QCOMPARE(canvas->brushCursorAngle(),library->angle());QCOMPARE(canvas->brushCursorRoundness(),library->roundness());
        client.activateEraser();QCOMPARE(canvas->brushCursorRect().width()/canvas->zoom(),library->radius()*2);client.activateBrush();
        canvas->setVisible(false);QVERIFY(!canvas->brushCursorVisible());canvas->setVisible(true);canvas->setClient(nullptr);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void quickHoverDoesNotReuseAnotherPensGeometry() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("geometry.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        QQuickWindow window;window.resize(400,400);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({400,400});canvas->setClient(&client);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));
        auto *library=client.brushLibrary();library->setRadius(12);library->setRoundness(.5);library->setAngleControl("pen-rotation");library->setTiltRoundness(true);library->setMinimumRoundness(.2);
        const auto capabilities=QInputDevice::Capability::Position|QInputDevice::Capability::Pressure|QInputDevice::Capability::XTilt|QInputDevice::Capability::Rotation;
        QPointingDevice first("B2 geometry first",9305,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,capabilities,1,1),second("B2 geometry second",9306,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,capabilities,1,1);
        const QPointF at(200,200);QTabletEvent raw(QEvent::TabletMove,&first,at,canvas->mapToGlobal(at),0,60,0,0,80,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(canvas,&raw);QTRY_COMPARE(canvas->brushCursorAngle(),80.);QCOMPARE(canvas->brushCursorRoundness(),.25);
        QHoverEvent retained(QEvent::HoverMove,at,canvas->mapToGlobal(at),at,Qt::NoModifier,&first);QCoreApplication::sendEvent(canvas,&retained);QCOMPARE(canvas->brushCursorAngle(),80.);QCOMPARE(canvas->brushCursorRoundness(),.25);
        QHoverEvent switched(QEvent::HoverMove,at,canvas->mapToGlobal(at),at,Qt::NoModifier,&second);QCoreApplication::sendEvent(canvas,&switched);QCOMPARE(canvas->brushCursorAngle(),0.);QCOMPARE(canvas->brushCursorRoundness(),.5);
        QTabletEvent press(QEvent::TabletPress,&second,at,canvas->mapToGlobal(at),.5,60,0,0,45,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(canvas,&press);QVERIFY(client.drawing());
        QTRY_VERIFY(qAbs(canvas->brushCursorRect().width()/canvas->zoom()-12)<.001);
        library->setCursorSimple(true);QCOMPARE(canvas->brushCursorRect().width()/canvas->zoom(),24.);
        library->setCursorSimple(false);QVERIFY(library->persistencePending());QVERIFY(qAbs(canvas->brushCursorRect().width()/canvas->zoom()-12)<.001);QCOMPARE(canvas->brushCursorAngle(),45.);
        QTabletEvent release(QEvent::TabletRelease,&second,at,canvas->mapToGlobal(at),0,60,0,0,45,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(canvas,&release);QTRY_VERIFY(!client.drawing() && !client.layerEditBusy());
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void tabletContactMoveBelongsToPressedCanvas() {
        QTemporaryDir temp;PaintCoreClient first(nullptr,temp.filePath("preferences.ini"));
        QTRY_VERIFY2(first.ready() && first.brushLibraryReady(),qPrintable(first.lastError()));
        PaintCoreClient second(nullptr,temp.filePath("preferences.ini"),first.brushCatalogConnection());
        QTRY_VERIFY2(second.ready() && second.brushLibraryReady(),qPrintable(second.lastError()));
        first.newTransparentDocument(800,300);second.newTransparentDocument(800,300);
        QTRY_VERIFY(first.documentWidth()==800 && second.documentWidth()==800 && !first.layerEditBusy() && !second.layerEditBusy());
        QQuickWindow window;window.resize(640,300);
        auto *a=new CanvasItem(window.contentItem());a->setSize({300,300});a->setClient(&first);
        auto *b=new CanvasItem(window.contentItem());b->setPosition({320,0});b->setSize({300,300});b->setClient(&second);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));a->actualSize();b->actualSize();
        first.setBrushColor(Qt::red);first.setBrushRadius(6);first.brushLibrary()->setPressureSize(false);first.brushLibrary()->setPressureFlow(false);
        const auto pixel=a->documentPoint({210,150});
        QPointingDevice pen("B1 contact pen",9201,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto send=[&](QEvent::Type type,QPointF scene){QTabletEvent e(type,&pen,scene,window.mapToGlobal(scene),type==QEvent::TabletRelease?0:.7,0,0,0,0,0,Qt::NoModifier,type==QEvent::TabletMove?Qt::NoButton:Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(&window,&e);};
        QTabletEvent hover(QEvent::TabletMove,&pen,{150,150},window.mapToGlobal(QPointF(150,150)),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(&window,&hover);
        const auto releaseSpace=qScopeGuard([&]{QTest::keyRelease(&window,Qt::Key_Space);});
        QTest::keyPress(&window,Qt::Key_Space);QCOMPARE(window.cursor().shape(),Qt::OpenHandCursor);QVERIFY(!a->brushCursorVisible());
        QTest::keyRelease(&window,Qt::Key_Space);QVERIFY(!a->spacePanning());QCOMPARE(window.cursor().shape(),Qt::BlankCursor);
        send(QEvent::TabletPress,{150,150});QVERIFY(first.drawing());QVERIFY(!second.drawing());
        // Native contact remains addressed to its original window while crossing
        // the adjacent canvas. B's later-installed application filter sees it first.
        send(QEvent::TabletMove,{430,150});
        QCOMPARE(a->cursorPosition(),QPointF(430,150));QVERIFY(!b->brushCursorVisible());QVERIFY(first.drawing());
        send(QEvent::TabletRelease,{430,150});QTRY_VERIFY(!first.drawing() && !first.layerEditBusy());
        QTRY_COMPARE(first.undoDepth(),1);QCOMPARE(second.undoDepth(),0);
        QTRY_VERIFY(first.frameRevision()>=first.revision());QTRY_VERIFY(first.sampleDocumentPixel(pixel.x(),pixel.y()).alpha()>240);
        const auto painted=first.sampleDocumentPixel(pixel.x(),pixel.y());QVERIFY(painted.red()>240 && painted.green()<10);
        first.undo();QTRY_COMPARE(first.undoDepth(),0);QTRY_VERIFY(first.frameRevision()>=first.revision());QTRY_COMPARE(first.sampleDocumentPixel(pixel.x(),pixel.y()).alpha(),0);
        first.redo();QTRY_COMPARE(first.undoDepth(),1);QTRY_VERIFY(first.frameRevision()>=first.revision());QTRY_COMPARE(first.sampleDocumentPixel(pixel.x(),pixel.y()),painted);
        send(QEvent::TabletPress,{120,110});send(QEvent::TabletRelease,{120,110});QTRY_COMPARE(first.undoDepth(),2);QTRY_VERIFY(!first.layerEditBusy());
        a->setClient(nullptr);b->setClient(nullptr);
        QSignalSpy stoppedA(&first,&PaintCoreClient::stopped),stoppedB(&second,&PaintCoreClient::stopped);
        first.shutdown();second.shutdown();QTRY_COMPARE(stoppedA.size(),1);QTRY_COMPARE(stoppedB.size(),1);
    }
    void proximityLossDoesNotEndMouseOrOtherPenStroke() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("proximity-owner.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(128,128);
        QTRY_VERIFY(client.documentWidth()==128 && !client.layerEditBusy());
        QQuickWindow window;window.resize(400,400);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({400,400});canvas->setClient(&client);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->fitToView();
        QPointingDevice pen("B1 owned pen",9202,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        QPointingDevice other("B1 other pen",9203,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto at=canvas->documentRect().center();
        const auto leave=[&](const QPointingDevice *device){QTabletEvent e(QEvent::TabletLeaveProximity,device,at,canvas->mapToGlobal(at),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(qApp,&e);};
        QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,at.toPoint());QVERIFY(client.drawing());
        leave(&other);QVERIFY2(client.drawing(),"Unowned proximity loss ended a mouse stroke");
        QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,at.toPoint());QTRY_VERIFY(!client.layerEditBusy());
        QTabletEvent press(QEvent::TabletPress,&pen,at,canvas->mapToGlobal(at),.7,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(&window,&press);QVERIFY(client.drawing());
        leave(&other);QVERIFY2(client.drawing(),"Another device ended the accepted pen stroke");
        leave(&pen);QTRY_VERIFY(!client.drawing() && !client.layerEditBusy());QTRY_COMPARE(client.undoDepth(),2);
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void tabletWindowHandoffClearsPreviousOutline() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("handoff.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        QQuickWindow first,second;first.resize(300,300);second.resize(300,300);second.setPosition({750,100});
        auto *a=new CanvasItem(first.contentItem());a->setSize({300,300});a->setClient(&client);
        auto *b=new CanvasItem(second.contentItem());b->setSize({300,300});b->setClient(&client);
        first.show();second.show();QVERIFY(QTest::qWaitForWindowExposed(&first));QVERIFY(QTest::qWaitForWindowExposed(&second));
        QPointingDevice pen("B1 handoff pen",9204,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        QQmlEngine engine;engine.rootContext()->setContextProperty("a",a);engine.rootContext()->setContextProperty("b",b);
        QQmlComponent component(&engine);component.setData("import QtQml\nQtObject { property bool firstOutline: a.brushCursorVisible; property bool secondOutline: b.brushCursorVisible }",QUrl());
        QScopedPointer<QObject> state(component.create());QVERIFY2(state,qPrintable(component.errorString()));
        const auto hover=[&](QQuickWindow *window){QTabletEvent e(QEvent::TabletMove,&pen,{150,150},window->mapToGlobal(QPointF(150,150)),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(window,&e);};
        hover(&first);QVERIFY(state->property("firstOutline").toBool());QCOMPARE(first.cursor().shape(),Qt::BlankCursor);
        // A driver may change target without an intervening Quick HoverLeave.
        hover(&second);QVERIFY(state->property("secondOutline").toBool());QCOMPARE(second.cursor().shape(),Qt::BlankCursor);
        QVERIFY2(!state->property("firstOutline").toBool(),"Previous window retained a stale pen outline");QVERIFY(first.cursor().shape()!=Qt::BlankCursor);
        QHoverEvent cached(QEvent::HoverMove,{130,130},first.mapToGlobal(QPointF(130,130)),{150,150});QCoreApplication::sendEvent(a,&cached);
        QVERIFY(!state->property("firstOutline").toBool());
        QMouseEvent mouse(QEvent::MouseMove,{140,140},first.mapToGlobal(QPointF(140,140)),Qt::NoButton,Qt::NoButton,Qt::NoModifier);QCoreApplication::sendEvent(&first,&mouse);
        QVERIFY(state->property("firstOutline").toBool());QVERIFY(!state->property("secondOutline").toBool());QVERIFY(second.cursor().shape()!=Qt::BlankCursor);
        hover(&second);b->setVisible(false);QVERIFY(!state->property("secondOutline").toBool());QVERIFY(second.cursor().shape()!=Qt::BlankCursor);b->setVisible(true);
        a->setClient(nullptr);b->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void selectedBackendAlsoFiltersNativeTabletMenu() {
#ifdef Q_OS_WIN
        configureTabletInput("wintab");
        if(activeTabletInputBackend()!=QStringLiteral("wintab"))QSKIP("WinTab is unavailable; dual-backend menu rejection requires WinTab selection");
        const auto restore=qScopeGuard([]{configureTabletInput("auto");});
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("menu.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.setInitialProperties({{"createInitialDocument",true}});
        engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.setBrushWriteBlend(0);
        auto *combo=findVisualItem(main->contentItem(),"brushWriteBlend");QVERIFY(combo);auto *popup=combo->property("popup").value<QObject*>();QVERIFY(popup);
        QSignalSpy actions(combo,SIGNAL(activated(int)));QVERIFY(actions.isValid());
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,combo->mapToScene({combo->width()/2,combo->height()/2}).toPoint());
        QQuickItem *option=nullptr;
        QTRY_VERIFY([&]{for(auto *window:QGuiApplication::allWindows())if(auto *quick=qobject_cast<QQuickWindow*>(window))if((option=findVisualItem(quick->contentItem(),"brushWriteBlendOption:3")))return true;return false;}());
        QTRY_VERIFY(popup->property("visible").toBool());QVERIFY(option->window()!=main);
        QPointingDevice rejected("wmpointer",9205,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        QPointingDevice accepted("wintab",9206,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto click=[&](const QPointingDevice *device){const auto global=option->mapToGlobal({option->width()/2,option->height()/2});auto *target=option->window();for(auto type:{QEvent::TabletPress,QEvent::TabletRelease}){QTabletEvent e(type,device,target->mapFromGlobal(global),global,type==QEvent::TabletPress?.5:0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletPress?Qt::LeftButton:Qt::NoButton);QCoreApplication::sendEvent(target,&e);}};
        // Qt's excluded wmpointer stream can name the native popup directly;
        // canvas-window filtering cannot protect this receiver.
        click(&rejected);QCOMPARE(actions.size(),0);QCOMPARE(client.brushWriteBlend(),0);QVERIFY(popup->property("visible").toBool());
        click(&accepted);QTRY_COMPARE(actions.size(),1);QCOMPARE(client.brushWriteBlend(),3);QTRY_VERIFY(!popup->property("visible").toBool());QCOMPARE(client.undoDepth(),0);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
#else
        QSKIP("Windows backend routing regression");
#endif
    }
    void ordinaryTabletControlsAndTextFocusRemainNormal() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("controls.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());QQmlEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
        QQmlComponent component(&engine);component.setData(R"(
import QtQuick
import QtQuick.Window
import "qrc:/qml/controls"
Window {
    visible: true; width: 650; height: 300
    CompactCheckBox { objectName: "toggle"; x: 350; y: 40; width: 180; height: 30; text: "pen toggle" }
    CompactTextField { objectName: "text"; x: 350; y: 100; width: 180; height: 30 }
    CompactSlider { objectName: "slider"; x: 350; y: 160; width: 180; from: 0; to: 100; value: 20 }
})",QUrl());
        QScopedPointer<QObject> root(component.create());QVERIFY2(root,qPrintable(component.errorString()));auto *window=qobject_cast<QQuickWindow*>(root.data());QVERIFY(window);
        auto *canvas=new CanvasItem(window->contentItem());canvas->setSize({300,300});canvas->setClient(&client);
        window->requestActivate();QVERIFY(QTest::qWaitForWindowActive(window));
        auto *toggle=findVisualItem(window->contentItem(),"toggle"),*text=findVisualItem(window->contentItem(),"text"),*slider=findVisualItem(window->contentItem(),"slider");QVERIFY(toggle && text && slider);
        QSignalSpy clicks(toggle,SIGNAL(clicked()));QVERIFY(clicks.isValid());
        auto *pen=new QPointingDevice("B1 controls pen",9207,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure|QInputDevice::Capability::Hover,1,3);pen->setParent(qApp);QWindowSystemInterface::registerInputDevice(pen);
        const auto input=[&](QPointF scene,Qt::MouseButtons buttons,qreal pressure){QWindowSystemInterface::handleTabletEvent(window,pen,QHighDpi::toNativeLocalPosition(scene,window),QHighDpi::toNativeGlobalPosition(window->mapToGlobal(scene),window),buttons,pressure,0,0,0,0,0);QWindowSystemInterface::flushWindowSystemEvents();};
        const auto point=toggle->mapToScene({40,15});input(point,Qt::NoButton,0);input(point,Qt::LeftButton,.5);
        for(int i=0;i<8;++i)input(point+QPointF(i,0),Qt::LeftButton,.5);
        QCOMPARE(clicks.size(),0);input(point,Qt::NoButton,0);QCOMPARE(clicks.size(),1);QVERIFY(toggle->property("checked").toBool());
        // Two further valid fast clicks must not be suppressed by a time debounce.
        for(int i=0;i<2;++i){input(point,Qt::LeftButton,.5);input(point,Qt::NoButton,0);}QCOMPARE(clicks.size(),3);QVERIFY(toggle->property("checked").toBool());
        input(point,Qt::RightButton,0);input(point,Qt::NoButton,0);QCOMPARE(clicks.size(),3);
        const auto start=slider->mapToScene({40,11}),end=slider->mapToScene({140,11});input(start,Qt::LeftButton,.5);input(end,Qt::LeftButton,.5);input(end,Qt::NoButton,0);QVERIFY(slider->property("value").toDouble()>60);
        text->forceActiveFocus();QVERIFY(text->hasActiveFocus());for(auto key:{Qt::Key_A,Qt::Key_Space,Qt::Key_B})QTest::keyClick(window,key);QCOMPARE(text->property("text").toString(),QString("a b"));QVERIFY(!canvas->spacePanning());QVERIFY(!client.drawing());QCOMPARE(client.undoDepth(),0);
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void missingSpaceReleaseRecoversWithoutPointerMovement() {
#ifdef Q_OS_WIN
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("space.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        QQuickWindow window;window.resize(400,400);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({400,400});canvas->setClient(&client);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->forceActiveFocus();
        QPointingDevice pen("B1 space pen",9209,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto hover=[&]{QTabletEvent e(QEvent::TabletMove,&pen,{200,200},window.mapToGlobal(QPointF(200,200)),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(&window,&e);};hover();
        QQmlEngine engine;engine.rootContext()->setContextProperty("canvasView",canvas);QQmlComponent component(&engine);
        component.setData("import QtQml\nQtObject { property bool pan: canvasView.spacePanning; property bool brush: canvasView.brushCursorVisible }",QUrl());QScopedPointer<QObject> state(component.create());QVERIFY(state);
        class ReleaseBlocker final:public QObject {bool eventFilter(QObject *,QEvent *event) override{return event->type()==QEvent::KeyRelease && static_cast<QKeyEvent*>(event)->key()==Qt::Key_Space;}} blocker;
        qApp->installEventFilter(&blocker);
        const auto key=[](bool down){INPUT input{};input.type=INPUT_KEYBOARD;input.ki.wVk=VK_SPACE;input.ki.dwFlags=down?0:KEYEVENTF_KEYUP;return SendInput(1,&input,sizeof(INPUT));};
        const auto release=qScopeGuard([&]{qApp->removeEventFilter(&blocker);if(GetAsyncKeyState(VK_SPACE)&0x8000)key(false);QCoreApplication::processEvents();QKeyEvent up(QEvent::KeyRelease,Qt::Key_Space,Qt::NoModifier);QCoreApplication::sendEvent(&window,&up);});
        QCOMPARE(key(true),UINT(1));QTRY_VERIFY(state->property("pan").toBool());QCOMPARE(window.cursor().shape(),Qt::OpenHandCursor);
        hover();QVERIFY(state->property("pan").toBool());QCOMPARE(key(false),UINT(1));
        QTRY_VERIFY_WITH_TIMEOUT(!state->property("pan").toBool(),600);QVERIFY(state->property("brush").toBool());QCOMPARE(canvas->cursor().shape(),Qt::BlankCursor);QCOMPARE(window.cursor().shape(),Qt::BlankCursor);
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
#else
        QSKIP("Windows physical Space release recovery");
#endif
    }
    void tabletFocusAndMouseUngrabCancelOnlyAcceptedStroke() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("cancel.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newTransparentDocument(128,128);QTRY_VERIFY(client.documentWidth()==128 && !client.layerEditBusy());
        QQuickWindow window;window.resize(400,400);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({400,400});canvas->setClient(&client);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->fitToView();
        QPointingDevice pen("B1 cancellation pen",9208,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto at=canvas->documentRect().center();const auto input=[&](QEvent::Type type){QTabletEvent e(type,&pen,at,canvas->mapToGlobal(at),type==QEvent::TabletRelease?0:.6,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(&window,&e);};
        QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,at.toPoint());QVERIFY(client.drawing());QCOMPARE(window.mouseGrabberItem(),canvas);
        canvas->ungrabMouse();QVERIFY(window.mouseGrabberItem()!=canvas);
        QTRY_VERIFY(!client.drawing() && !client.layerEditBusy());QCOMPARE(client.undoDepth(),0);QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,at.toPoint());
        input(QEvent::TabletPress);QVERIFY(client.drawing());QEvent deactivate(QEvent::WindowDeactivate);QCoreApplication::sendEvent(&window,&deactivate);QTRY_VERIFY(!client.drawing() && !client.layerEditBusy());QCOMPARE(client.undoDepth(),0);QVERIFY(window.cursor().shape()!=Qt::BlankCursor);
        QEvent activate(QEvent::WindowActivate);QCoreApplication::sendEvent(&window,&activate);input(QEvent::TabletPress);QVERIFY(client.drawing());input(QEvent::TabletRelease);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        input(QEvent::TabletPress);QVERIFY(client.drawing());canvas->setVisible(false);QTRY_VERIFY(!client.drawing() && !client.layerEditBusy());QCOMPARE(client.undoDepth(),1);
        auto *replacement=new CanvasItem(window.contentItem());replacement->setSize({400,400});replacement->setClient(&client);replacement->fitToView();
        input(QEvent::TabletPress);QVERIFY(client.drawing());input(QEvent::TabletRelease);QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());
        input(QEvent::TabletPress);QVERIFY(client.drawing());
        QSignalSpy detached(replacement,&QQuickItem::windowChanged);
        delete replacement;qInfo()<<"Window changes while deleting replacement canvas:"<<detached.size();
        QTRY_VERIFY(!client.drawing() && !client.layerEditBusy());QCOMPARE(client.undoDepth(),2);canvas->setVisible(true);
        input(QEvent::TabletPress);QVERIFY(client.drawing());input(QEvent::TabletRelease);QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void shortcutKeysUpdateLikeSpaceWithoutPointerMovement() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("shortcut-events.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(128,128);
        QTRY_VERIFY(!client.layerEditBusy());
        QQuickWindow window;window.resize(512,512);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({512,512});canvas->setClient(&client);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->fitToView();
        const auto at=canvas->documentRect().center();
        // Window activation does not imply a pointer entry after the preceding
        // native window was destroyed. Seed that entry before stationary-key checks.
        QEnterEvent enter(at,at,window.mapToGlobal(at));QCoreApplication::sendEvent(&window,&enter);QTest::mouseMove(&window,at.toPoint());
        QVERIFY(canvas->brushCursorVisible());
        const auto key=[&](int code,bool down){QKeyEvent event(down?QEvent::KeyPress:QEvent::KeyRelease,code,Qt::NoModifier);QCoreApplication::sendEvent(&window,&event);};
        QPointingDevice pen("shortcut state pen",9196,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto hover=[&](Qt::KeyboardModifiers modifiers){QTabletEvent event(QEvent::TabletMove,&pen,at,canvas->mapToGlobal(at),0,0,0,0,0,0,modifiers,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(&window,&event);};
        for(int tool=0;tool<4;++tool) {
            if(tool==0)client.activateBrush();else if(tool==1)client.activateEraser();else if(tool==2)client.activateSmudge();else client.activateLasso();
            const auto geometry=canvas->brushCursorRect();const auto angle=canvas->brushCursorAngle();
            for(int i=0;i<8;++i) {
                key(Qt::Key_Space,true);QCOMPARE(canvas->cursor().shape(),Qt::OpenHandCursor);QVERIFY(!canvas->brushCursorVisible());
                key(Qt::Key_Space,false);QCOMPARE(canvas->brushCursorVisible(),tool<3);
                if(tool==3)continue;
                key(Qt::Key_Alt,true);QVERIFY(canvas->eyedropperCursorVisible());QVERIFY(!canvas->brushCursorVisible());
                hover(Qt::NoModifier);QVERIFY(canvas->eyedropperCursorVisible());
                key(Qt::Key_Alt,false);QVERIFY(canvas->brushCursorVisible());
                hover(Qt::AltModifier);QVERIFY(!canvas->eyedropperCursorVisible());
                key(Qt::Key_Control,true);QCOMPARE(canvas->cursor().shape(),Qt::SizeAllCursor);QVERIFY(!canvas->brushCursorVisible());
                hover(Qt::NoModifier);QCOMPARE(canvas->cursor().shape(),Qt::SizeAllCursor);
                key(Qt::Key_Control,false);QVERIFY(canvas->brushCursorVisible());
                hover(Qt::ControlModifier);QVERIFY(canvas->brushCursorVisible());
            }
            // Tablet hover intentionally updates the pointer position; keyboard-only
            // transitions above must still leave the brush geometry unchanged.
            QCOMPARE(canvas->brushCursorRect().size(),geometry.size());QCOMPARE(canvas->brushCursorAngle(),angle);
        }
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void physicalShortcutsUpdateQmlWithoutPointerOrKeyEvents() {
#ifdef Q_OS_WIN
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("physical-shortcuts.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        QQuickWindow window;window.resize(512,512);
        auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({512,512});canvas->setClient(&client);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->fitToView();
        const auto at=canvas->documentRect().center();QEnterEvent enter(at,at,window.mapToGlobal(at));QCoreApplication::sendEvent(&window,&enter);QTest::mouseMove(&window,at.toPoint());
        QQmlEngine engine;engine.rootContext()->setContextProperty("canvasView",canvas);
        QQmlComponent component(&engine);
        component.setData("import QtQml\nQtObject { property bool brush: canvasView.brushCursorVisible; property bool eyedropper: canvasView.eyedropperCursorVisible }",QUrl());
        QScopedPointer<QObject> bindings(component.create());QVERIFY2(bindings,qPrintable(component.errorString()));
        QTRY_VERIFY(bindings->property("brush").toBool());
        class KeyBlocker final : public QObject {
            bool eventFilter(QObject *,QEvent *event) override {
                return event->type()==QEvent::KeyPress || event->type()==QEvent::KeyRelease || event->type()==QEvent::ShortcutOverride;
            }
        } blocker;
        qApp->installEventFilter(&blocker);
        const auto key=[](WORD code,bool down){INPUT input{};input.type=INPUT_KEYBOARD;input.ki.wVk=code;input.ki.dwFlags=down?0:KEYEVENTF_KEYUP;return SendInput(1,&input,sizeof(INPUT));};
        const auto release=qScopeGuard([&]{
            for(WORD code:{WORD(VK_MENU),WORD(VK_CONTROL)})if(GetAsyncKeyState(code)&0x8000)key(code,false);
            QCoreApplication::processEvents();qApp->removeEventFilter(&blocker);
        });
        QPointingDevice pen("physical shortcut pen",9195,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        for(bool tablet:{false,true}) {
            if(tablet){QTabletEvent hover(QEvent::TabletMove,&pen,at,canvas->mapToGlobal(at),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(&window,&hover);}
            const auto position=canvas->cursorPosition();
            QCOMPARE(key(VK_MENU,true),UINT(1));
            QTRY_VERIFY_WITH_TIMEOUT(bindings->property("eyedropper").toBool(),600);
            QVERIFY(!bindings->property("brush").toBool());QCOMPARE(window.cursor().shape(),Qt::BlankCursor);
            QTest::qWait(80);QVERIFY(bindings->property("eyedropper").toBool());
            QCOMPARE(key(VK_MENU,false),UINT(1));
            QTRY_VERIFY_WITH_TIMEOUT(bindings->property("brush").toBool(),600);
            QVERIFY(!bindings->property("eyedropper").toBool());
            QCOMPARE(key(VK_CONTROL,true),UINT(1));
            QTRY_COMPARE_WITH_TIMEOUT(window.cursor().shape(),Qt::SizeAllCursor,600);
            QVERIFY(!bindings->property("brush").toBool());
            QCOMPARE(key(VK_CONTROL,false),UINT(1));
            QTRY_VERIFY_WITH_TIMEOUT(bindings->property("brush").toBool(),600);
            QTRY_COMPARE(window.cursor().shape(),Qt::BlankCursor);
        }
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
#else
        QSKIP("Physical keyboard polling is Windows specific");
#endif
    }
    void continuousStrokesKeepPublishingFramesAndAllowLaterEdits() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("publication-retry.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(128,128);
        QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushRadius(3);client.setBrushColor(Qt::red);client.brushLibrary()->setPressureSize(false);
        QQuickWindow window;window.resize(256,256);window.setColor(Qt::white);
        auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({256,256});canvas->setClient(&client);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        QSignalSpy frames(&client,&PaintCoreClient::frameChanged);
        for(int stroke=0;stroke<24;++stroke) {
            const auto y=8.+stroke*4;
            QVERIFY(client.beginStroke({{8,y},1}));
            for(int x=9;x<=112;++x) {
                client.strokeTo({{double(x),y},1});
                if(x%8==0)QTest::qWait(1);
            }
            client.endStroke();
            QTRY_COMPARE(client.undoDepth(),stroke+1);QTRY_VERIFY(!client.layerEditBusy());
        }
        QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(frames.size()>24);
        const auto redAt=[&](QPointF p){
            auto grab=canvas->grabToImage();
            if(!grab || !QTest::qWaitFor([&]{return !grab->image().isNull();},5000))return false;
            const auto pixel=(canvas->documentRect().topLeft()+p*canvas->zoom())*window.devicePixelRatio();
            const auto color=grab->image().pixelColor(pixel.toPoint());
            return color.red()>180 && color.green()<70 && color.blue()<70;
        };
        QTRY_VERIFY(redAt({60,100}));
        QVERIFY(client.beginLiquify());QTRY_VERIFY(client.liquifyActive() && !client.liquifyBusy());
        QVERIFY(client.finishLiquify(false));QTRY_VERIFY(!client.liquifyActive() && !client.liquifyBusy());
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());
        QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive() && !client.transformBusy());
        client.setBrushColor(Qt::red);QVERIFY(client.beginStroke({{64,116},1}));client.endStroke();
        QTRY_COMPARE(client.undoDepth(),25);QTRY_VERIFY(!client.layerEditBusy());
        QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_VERIFY(redAt({64,116}));
        QVERIFY(client.lastError().isEmpty());canvas->setClient(nullptr);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void directLiquifyHistoryJumpRestoresTheDisplayedResult() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("liquify-jump.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newTransparentDocument(128,96);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushRadius(10);client.setBrushColor(Qt::red);QVERIFY(client.beginStroke({{42,48},1}));client.strokeTo({{65,48},1});client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        QQuickWindow window;window.resize(256,192);window.setColor(Qt::white);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({256,192});canvas->setClient(&client);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        QVERIFY(client.beginLiquify());QTRY_VERIFY(client.liquifyActive() && !client.liquifyBusy());
        const bool gpu=canvas->gpuDisplay();
        const auto settled=[&]{return gpu ? client.residentPreview() && client.residentPreview()->revision>=client.revision() : client.frameRevision()>=client.revision();};
        const auto capture=[&]{auto result=canvas->grabToImage();if(!result || !QTest::qWaitFor([&]{return !result->image().isNull();},5000))return QImage{};return result->image();};
        QTRY_VERIFY(settled());const auto identity=capture();QVERIFY(!identity.isNull());client.setLiquifySize(48);client.setLiquifyStrength(100);client.setLiquifyDensity(60);
        for(int i=0;i<3;++i){QVERIFY(client.beginStroke({{50.+i*3,48},1}));client.strokeTo({{58.+i*3,50},1});client.endStroke();QTRY_COMPARE(client.undoDepth(),i+1);QTRY_VERIFY(!client.liquifyBusy());}
        QTRY_VERIFY(settled());const auto warped=capture();QVERIFY(!warped.isNull());QVERIFY(warped!=identity);
        for(int target:{0,3,1,2,0,3}) {
            const auto revision=client.revision();QVERIFY(client.selectHistory(target));QTRY_COMPARE(client.undoDepth(),target);QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(settled());QCOMPARE(client.revision(),revision+1);QCOMPARE(client.redoDepth(),3-target);
            if(target==0 || target==3)QCOMPARE(capture(),target==0 ? identity : warped);
        }
        QVERIFY(client.finishLiquify(false));QTRY_VERIFY(!client.liquifyActive() && !client.liquifyBusy());QCOMPARE(client.undoDepth(),1);QVERIFY(client.lastError().isEmpty());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void selectedLiquifyKeepsTheMeshPathAndProtectsUnselectedPixels_data() {
        QTest::addColumn<int>("feather");QTest::addColumn<int>("context");QTest::newRow("hard-lasso")<<0<<0;QTest::newRow("feathered-lasso")<<8<<0;QTest::newRow("nested-normal-lasso")<<8<<1;QTest::newRow("nested-pass-lasso")<<8<<2;QTest::newRow("ancestor-mask-fallback")<<8<<3;QTest::newRow("empty-ancestor-mask")<<8<<4;QTest::newRow("zero-opacity-ancestor-mask")<<8<<5;QTest::newRow("empty-owner-mask")<<8<<6;
    }
    void selectedLiquifyKeepsTheMeshPathAndProtectsUnselectedPixels() {
        QFETCH(int,feather);QFETCH(int,context);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("selected-liquify.ini"));QTRY_VERIFY(client.ready()&&client.brushLibraryReady());
        client.newDocument(2048,1536);QTRY_COMPARE(client.documentWidth(),2048);QTRY_VERIFY(!client.layerEditBusy());client.addLayer("selected liquify");QTRY_VERIFY(!client.layerEditBusy());
        const auto paint=[&](QPointF p,QColor c){client.setBrushColor(c);client.setBrushRadius(24);return client.beginStroke({p,1});};
        QVERIFY(paint({980.5,768.5},Qt::red));client.endStroke();QTRY_COMPARE(client.undoDepth(),2);QVERIFY(paint({1080.5,768.5},Qt::blue));client.endStroke();QTRY_COMPARE(client.undoDepth(),3);
        const auto owner=client.activeLayer();
        if(context && context!=6){quint64 target=owner;for(int i=0;i<2;++i){QVERIFY(client.groupLayer(target,"liquify ancestor"));QTRY_VERIFY(!client.layerEditBusy());target=client.activeLayer();QVERIFY(client.setLayerBlend(target,context==2?27:0));QTRY_VERIFY(!client.layerEditBusy());}if(context>=3){QVERIFY(client.addMask(target));QTRY_VERIFY(!client.layerEditBusy());if(context==3 || context==5){QVERIFY(paint({400,300},Qt::black));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());}if(context==5){client.setLayerProperties(client.activeLayer(),true,0);QTRY_VERIFY(!client.layerEditBusy());}}client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());}
        if(context==6){QVERIFY(client.addMask(owner));QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());}
        client.setSelectionFeather(feather);QVERIFY(client.editSelectionPath({{700,500},{1040,500},{1040,1100},{700,1100}},0,false));QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.selectionEnabled());const auto selection=client.selectionSteps();const auto depth=client.undoDepth();
        QQuickWindow window;window.resize(1024,832);window.setColor(Qt::black);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({1024,832});canvas->setClient(&client);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto capture=[&]{auto result=canvas->grabToImage();if(!result || !QTest::qWaitFor([&]{return !result->image().isNull();},5000))return QImage{};return result->image();};
        const auto at=[&](const QImage &image,QPointF p){const auto local=canvas->documentRect().topLeft()+p*canvas->zoom();return image.pixelColor(qRound(local.x()*image.width()/canvas->width()),qRound(local.y()*image.height()/canvas->height()));};
        const auto close=[](QColor a,QColor b){return qAbs(a.red()-b.red())<=3 && qAbs(a.green()-b.green())<=3 && qAbs(a.blue()-b.blue())<=3;};
        const auto original=capture();QVERIFY(!original.isNull());const bool gpu=canvas->gpuDisplay() && context!=3;
        for(bool commit:{false,true}){
            client.setLiquifySize(2000);client.setLiquifyStrength(100);client.setLiquifyDensity(100);QVERIFY(client.beginLiquify());QTRY_VERIFY(client.liquifyActive()&&!client.liquifyBusy());
            if(context==3)QVERIFY(!client.residentPreview());
            quint64 source=0,mask=0;if(gpu){QTRY_VERIFY(client.residentPreview()&&!client.residentPreview()->selection.isNull());source=client.residentPreview()->sourceId;mask=client.residentPreview()->selection.cacheKey();QTRY_VERIFY(client.residentPreview()->revision>=client.revision());}
            const auto identity=capture();QVERIFY(!identity.isNull());QVERIFY(close(at(identity,{980,768}),at(original,{980,768})));const auto cpuRevision=client.frameRevision();
            QElapsedTimer timer;timer.start();QVERIFY(client.beginStroke({{980,768},1}));for(int i=1;i<=16;++i)client.strokeTo({{980+40.*i/16.,768},1});client.endStroke();QTRY_VERIFY(!client.liquifyBusy());
            if(gpu){QTRY_VERIFY(client.residentPreview()->revision>=client.revision());QCOMPARE(client.frameRevision(),cpuRevision);QCOMPARE(client.residentPreview()->sourceId,source);QCOMPARE(client.residentPreview()->selection.cacheKey(),mask);}else{QTRY_VERIFY(client.frameRevision()>=client.revision());}
            const auto warped=capture();QVERIFY(!warped.isNull());QVERIFY(warped!=identity);QVERIFY(close(at(warped,{1080,768}),at(original,{1080,768})));QVERIFY(close(at(warped,{1150,768}),at(original,{1150,768})));QVERIFY(at(warped,{1015,768}).red()>220 && at(warped,{1015,768}).green()<80);
            qInfo()<<"selected liquify feather"<<feather<<"gpu"<<gpu<<"input through captured frame ms"<<timer.elapsed();
            client.undo();QTRY_VERIFY(!client.liquifyBusy());if(gpu){QTRY_VERIFY(client.residentPreview()->revision>=client.revision());}else{QTRY_VERIFY(client.frameRevision()>=client.revision());}QTRY_VERIFY(close(at(capture(),{980,768}),at(identity,{980,768})));
            client.redo();QTRY_VERIFY(!client.liquifyBusy());QVERIFY(client.finishLiquify(commit));QTRY_VERIFY(!client.liquifyActive()&&!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.selectionSteps(),selection);
            if(commit){QCOMPARE(client.undoDepth(),depth+1);client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(close(at(capture(),{980,768}),at(original,{980,768})));client.redo();QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(at(capture(),{1015,768}).green()<80);}else{QCOMPARE(client.undoDepth(),depth);QVERIFY(close(at(capture(),{980,768}),at(original,{980,768})));}
        }
        QVERIFY(client.lastError().isEmpty());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void reflectionHistoryHasSpecificLabelsAndRestoresPixels_data() {
        QTest::addColumn<bool>("horizontal");QTest::addColumn<bool>("transform");
        for(bool horizontal:{false,true})for(bool transform:{false,true})QTest::newRow(qPrintable(QString("%1-%2").arg(transform?"transform":"image",horizontal?"horizontal":"vertical")))<<horizontal<<transform;
    }
    void reflectionHistoryHasSpecificLabelsAndRestoresPixels() {
        QFETCH(bool,horizontal);QFETCH(bool,transform);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("flip.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(128,96);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(Qt::red);client.setBrushRadius(12);QVERIFY(client.beginStroke({{30,30},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(Qt::blue);QVERIFY(client.beginStroke({{70,50},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);
        const auto ready=[&]{client.requestViewport(0,{0,0,128,96},{128,96});return client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,96);};QTRY_VERIFY(ready());const auto before=client.frame();const auto depth=client.undoDepth();
        const auto label=horizontal?QStringLiteral("水平翻转"):QStringLiteral("垂直翻转");
        if(transform){
            QVERIFY(client.beginTransform());QTRY_VERIFY(!client.transformBusy());auto points=client.transformPoints();const auto center=(points[0].toPointF()+points[15].toPointF())*.5;
            for(auto &point:points){auto p=point.toPointF();if(horizontal)p.setX(2*center.x()-p.x());else p.setY(2*center.y()-p.y());point=p;}
            QVERIFY(client.updateTransform(points,false,client.transformInterpolation()));QVERIFY(client.checkpointTransform());QTRY_COMPARE(client.undoDepth(),1);QTRY_COMPARE(client.historyEntries().last().toMap().value("title").toString(),label);
            QVERIFY(client.selectHistory(0));QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.transformBusy());QVERIFY(client.selectHistory(1));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.transformBusy());QVERIFY(client.finishTransform(true));QTRY_VERIFY(!client.transformActive()&&!client.transformBusy());
        }else{auto *action=window->findChild<QObject*>(horizontal?"canvasFlipHorizontalAction":"canvasFlipVerticalAction");QVERIFY(action);QVERIFY(QMetaObject::invokeMethod(action,"trigger"));}
        QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_COMPARE(client.historyEntries().last().toMap().value("title").toString(),label);QTRY_VERIFY(ready());const auto after=client.frame();QVERIFY(after!=before);
        for(int i=0;i<3;++i){QVERIFY(client.selectHistory(depth));QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(ready());QCOMPARE(client.frame(),before);QVERIFY(client.selectHistory(depth+1));QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(ready());QCOMPARE(client.frame(),after);}
        QVERIFY(client.lastError().isEmpty());auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void selected4kTransformKeepsResidentTexturesAndRestoresHistory_data() {
        QTest::addColumn<bool>("warp");QTest::addColumn<int>("context");QTest::newRow("affine")<<false<<0;QTest::newRow("warp")<<true<<0;QTest::newRow("nested-normal-affine")<<false<<1;QTest::newRow("nested-normal-warp")<<true<<1;QTest::newRow("nested-pass-warp")<<true<<2;QTest::newRow("empty-fixed-owner-warp")<<true<<3;QTest::newRow("empty-ancestor-warp")<<true<<4;QTest::newRow("zero-opacity-ancestor-warp")<<true<<5;QTest::newRow("fixed-mask-fallback-warp")<<true<<6;QTest::newRow("ancestor-mask-fallback-warp")<<true<<7;
    }
    void selected4kTransformKeepsResidentTexturesAndRestoresHistory() {
        QFETCH(bool,warp);QFETCH(int,context);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("selected4k.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newDocument(4096,2160);QTRY_COMPARE(client.documentWidth(),4096);QTRY_VERIFY(!client.layerEditBusy());client.addLayer("partial selection");QTRY_VERIFY(!client.layerEditBusy());client.activateBrush();client.setBrushRadius(64);client.setBrushOpacity(.6);client.brushLibrary()->setPressureSize(false);client.brushLibrary()->setPressureFlow(false);
        for(const auto &p:QList<QPair<QPointF,QColor>>{{{1400.5,900.5},Qt::red},{{2200.5,900.5},Qt::blue}}){client.setBrushColor(p.second);QVERIFY(client.beginStroke({p.first,1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy() && !client.drawing());}
        const auto selectedOwner=client.activeLayer();
        if(context && context!=3 && context!=6){const auto owner=client.activeLayer();for(int i=0;i<2;++i){QVERIFY(client.groupSelectedLayers("selection ancestor"));QTRY_VERIFY(!client.layerEditBusy());if(context==1){QVERIFY(client.setLayerBlend(client.activeLayer(),0));QTRY_VERIFY(!client.layerEditBusy());}}client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());}
        if(context>=3){quint64 target=selectedOwner;if(context!=3 && context!=6){for(const auto &v:client.layers()){const auto row=v.toMap();if(row.value("id").toULongLong()==selectedOwner)target=row.value("parent").toULongLong();}}QVERIFY(client.addMask(target));QTRY_VERIFY(!client.layerEditBusy());const auto mask=client.activeLayer();QVERIFY(client.setMaskLinked(mask,false));QTRY_VERIFY(!client.layerEditBusy());if(context>=5){client.setBrushColor(Qt::black);client.setBrushRadius(8);QVERIFY(client.beginStroke({{1440,900},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());if(context==5){client.setLayerProperties(mask,true,0);QTRY_VERIFY(!client.layerEditBusy());}}client.selectLayer(selectedOwner);QTRY_COMPARE(client.activeLayer(),selectedOwner);QTRY_VERIFY(!client.layerEditBusy());}
        client.setSelectionFeather(2);QVERIFY(client.editSelectionPath({{1200,700},{1600,710},{1590,1100},{1210,1090}},0,false));QTRY_VERIFY(!client.layerEditBusy() && client.selectionEnabled());const auto selection=client.selectionSteps();const auto depth=client.undoDepth();
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);QVERIFY(QTest::qWaitForWindowExposed(window));canvas->fitToView();QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto capture=[&]{if(auto *overlay=findVisualItem(window->contentItem(),"transformOverlay"))overlay->setVisible(false);auto grab=window->contentItem()->grabToImage();if(!grab)return QImage{};QSignalSpy ready(grab.data(),&QQuickItemGrabResult::ready);if(!ready.wait(5000))return QImage{};return grab->image();};
        const auto at=[&](const QImage &image,QPointF p){const double dpr=double(image.width())/window->width();const auto v=canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom())*dpr;return image.pixelColor(v.toPoint());};
        const auto close=[](QColor a,QColor b){return qAbs(a.red()-b.red())<10 && qAbs(a.green()-b.green())<10 && qAbs(a.blue()-b.blue())<10;};
        QImage before;QTRY_VERIFY(!(before=capture()).isNull() && at(before,{1400,900}).red()>at(before,{1400,900}).green()+60);const auto red=at(before,{1400,900}),blue=at(before,{2200,900}),background=at(before,{1400,800});
        if(qEnvironmentVariable("QT_QUICK_BACKEND")=="rhi")QVERIFY(canvas->gpuDisplay());
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());const auto localDepth=client.undoDepth();const auto initial=client.transformPoints();
        if(canvas->gpuDisplay() && context<6)QTRY_VERIFY(client.residentPreview() && canvas->forwardPreview());
        if(context>=6)QVERIFY(!client.residentPreview());
        const auto identity=client.residentPreview();
        quint64 initialUploads=0,movingUploads=0;
        if(identity){QVERIFY(identity->prepared);QTRY_VERIFY(identity->uploads->load()>0 && identity->prepared->uploads->load()>0);initialUploads=identity->uploads->load();movingUploads=identity->prepared->uploads->load();}
        auto points=initial;for(auto &v:points)v=v.toPointF()+QPointF(512,0);if(warp){points[5]=points[5].toPointF()+QPointF(12,-8);points[10]=points[10].toPointF()+QPointF(-8,6);}
        QVERIFY(client.updateTransform(points,warp,2));QTRY_VERIFY(!client.transformBusy());
        ResidentPreviewPtr moving;
        if(canvas->gpuDisplay() && context<6){QTRY_VERIFY(client.residentPreview() && client.residentPreview()->sourceId!=identity->sourceId);moving=client.residentPreview();QCOMPARE(moving->sourceId,identity->prepared->sourceId);for(int i=0;i<3;++i)QCOMPARE(moving->images[i].cacheKey(),identity->prepared->images[i].cacheKey());QCOMPARE(moving->uploads,identity->prepared->uploads);}
        if(moving)for(int i=0;i<2;++i){
            QVERIFY(client.updateTransform(initial,false,2));QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.residentPreview()->sourceId,identity->sourceId);
            QImage restored;QTRY_VERIFY(!(restored=capture()).isNull() && close(at(restored,{1400,900}),red));
            QVERIFY(client.updateTransform(points,warp,2));QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.residentPreview()->sourceId,moving->sourceId);
        }
        const auto frameRevision=client.frameRevision();
        for(int i=1;i<=12;++i){auto next=points;for(auto &v:next)v=v.toPointF()+QPointF(i,0);QVERIFY(client.updateTransform(next,warp,2));}
        QTRY_VERIFY(!client.transformBusy());QCOMPARE(client.undoDepth(),localDepth);
        if(moving){QTRY_COMPARE(client.residentPreview()->revision,client.revision());QCOMPARE(client.residentPreview()->sourceId,moving->sourceId);for(int i=0;i<3;++i)QCOMPARE(client.residentPreview()->images[i].cacheKey(),moving->images[i].cacheKey());QCOMPARE(client.frameRevision(),frameRevision);}
        QImage preview;QTRY_VERIFY(!(preview=capture()).isNull() && close(at(preview,{1924,900}),red));QVERIFY(close(at(preview,{1400,900}),background));QVERIFY(close(at(preview,{2200,900}),blue));
        if(moving){QCOMPARE(moving->uploads->load(),movingUploads);QCOMPARE(identity->uploads->load(),initialUploads);qInfo()<<"prepared source batches before first drag"<<initialUploads<<movingUploads<<"after drag"<<identity->uploads->load()<<moving->uploads->load();}
        QVERIFY(client.finishTransform(true));QTRY_VERIFY_WITH_TIMEOUT(!client.transformActive() && !client.transformBusy(),30000);QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(client.frameRevision()>=client.revision() && !client.residentPreview());
        QImage final;QTRY_VERIFY(!(final=capture()).isNull() && close(at(final,{1924,900}),red));QVERIFY(close(at(final,{2200,900}),blue));QVERIFY(client.selectionSteps()!=selection);
        client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.selectionSteps(),selection);QImage undone;QTRY_VERIFY(!(undone=capture()).isNull() && close(at(undone,{1400,900}),red));
        client.redo();QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(client.frameRevision()>=client.revision());const auto finalSelection=client.selectionSteps();
        QVERIFY(client.beginTransform());QTRY_VERIFY(!client.transformBusy());auto cancelled=client.transformPoints();for(auto &v:cancelled)v=v.toPointF()+QPointF(-100,0);QVERIFY(client.updateTransform(cancelled,warp,2));QTRY_VERIFY(!client.transformBusy());QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive() && !client.transformBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.undoDepth(),depth+1);QCOMPARE(client.selectionSteps(),finalSelection);
        QVERIFY2(warnings.isEmpty(),qPrintable(warnings.join('\n')));QVERIFY(client.lastError().isEmpty());auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void attachedMaskDoesNotChangeTransformHandles() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("mask-handles.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(128,96);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushRadius(8);client.brushLibrary()->setPressureSize(false);client.setBrushColor(Qt::red);QVERIFY(client.beginStroke({{60,48},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);
        const auto owner=client.activeLayer();QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());const auto expected=client.transformPoints();QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive());
        QVERIFY(client.addMask(owner));QTRY_VERIFY(!client.layerEditBusy());client.setBrushRadius(64);client.setBrushColor(Qt::white);QVERIFY(client.beginStroke({{60,48},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());QCOMPARE(client.transformPoints(),expected);QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive());QVERIFY(client.lastError().isEmpty());client.shutdown();
    }
    void longTaskProgressUsesActualWorkAndConfiguredDelay() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("task-progress.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &error:errors)warnings.append(error.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);window->resize(1800,1000);QVERIFY(QTest::qWaitForWindowExposed(window));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        QCOMPARE(client.longTaskDelay(),1000);client.setLongTaskDelay(10000);client.newTransparentDocument(4096,2160);QTRY_COMPARE(client.documentWidth(),4096);QTRY_VERIFY(!client.layerEditBusy());client.activateBucket();client.setBrushColor(QColor("#bf3311"));QVERIFY(client.bucketFill({0,0}));QTRY_VERIFY_WITH_TIMEOUT(!client.layerEditBusy(),30000);
        auto *overlay=findVisualItem(window->contentItem(),"longTaskProgress");QVERIFY(overlay);QVERIFY(!overlay->isVisible());
        bool sawVisible=false,sawActualCount=false;int heartbeats=0;QTimer heartbeat;heartbeat.setInterval(1);connect(&heartbeat,&QTimer::timeout,this,[&]{++heartbeats;});
        connect(&client,&PaintCoreClient::computationProgressChanged,this,[&]{sawVisible|=client.longTaskVisible();const auto progress=client.computationProgress();sawActualCount|=progress.value("state").toInt()==1 && progress.value("completed").toULongLong()>0 && progress.value("completed").toULongLong()<progress.value("total").toULongLong();});
        const auto depth=client.undoDepth();QVERIFY(client.flipImage(true));QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),depth+1,30000);QTRY_COMPARE(client.computationProgress().value("state").toInt(),2);QVERIFY(!sawVisible);QVERIFY(!overlay->isVisible());
        client.setLongTaskDelay(0);sawVisible=false;sawActualCount=false;heartbeat.start();QVERIFY(client.flipImage(true));QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),depth+2,30000);heartbeat.stop();QTRY_COMPARE(client.computationProgress().value("state").toInt(),2);QCOMPARE(client.computationProgress().value("kind").toInt(),3);QVERIFY(sawVisible);QVERIFY(sawActualCount);QVERIFY(heartbeats>1);QVERIFY(!client.longTaskVisible());QVERIFY(!overlay->isVisible());
        QVERIFY(!window->findChild<QObject*>("liquifyCommitProgress"));QVERIFY(client.lastError().isEmpty());QCOMPARE(warnings,QStringList());
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void taskProgressPreferencesAndEditingScopesStayWithTheirDocument() {
        QTemporaryDir temp;const auto settings=temp.filePath("task-settings.ini");PaintCoreClient client(nullptr,settings);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QVERIFY(QTest::qWaitForWindowExposed(window));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);const auto initial=documents->activeId();const auto second=documents->newDocument(64,64,true);QVERIFY(!second.isEmpty());auto *other=documents->client(second);QVERIFY(other);QTRY_VERIFY(other->ready() && other->brushLibraryReady());
        client.setLongTaskDelay(240);QCOMPARE(other->longTaskDelay(),240);other->setLongTaskDelay(470);QCOMPARE(client.longTaskDelay(),470);QTRY_COMPARE(QSettings(settings,QSettings::IniFormat).value("performance/taskProgressDelayMs").toInt(),470);
        QVERIFY(documents->activate(initial));auto *dialog=window->findChild<QObject*>("preferencesDialog");QVERIFY(dialog);QVERIFY(QMetaObject::invokeMethod(dialog,"open"));QTRY_VERIFY(dialog->property("opened").toBool());auto *delay=dialog->findChild<QObject*>("taskProgressDelay");QVERIFY(delay);QCOMPARE(delay->property("value").toInt(),470);delay->setProperty("value",350);auto *save=dialog->findChild<QObject*>("saveStorageSettings");QVERIFY(save);QVERIFY(QMetaObject::invokeMethod(save,"clicked"));QTRY_COMPARE(client.longTaskDelay(),350);QCOMPARE(other->longTaskDelay(),350);QVERIFY(QMetaObject::invokeMethod(dialog,"close"));QTRY_VERIFY(!client.storageBusy());
        other->setBrushRadius(5);QVERIFY(other->beginStroke({{32,32},1}));other->endStroke();QTRY_VERIFY(!other->layerEditBusy());QVERIFY(documents->activate(second));documents->floatDocument(second);QTRY_VERIFY(!documents->windows().isEmpty());
        QVERIFY(other->beginTransform());QTRY_VERIFY(other->transformActive() && !other->transformBusy());auto points=other->transformPoints();for(auto &point:points)point=point.toPointF()+QPointF(4,2);QVERIFY(other->updateTransform(points,false,3));QTRY_VERIFY(!other->transformBusy());QVERIFY(other->finishTransform(true));QTRY_VERIFY(!other->transformActive());QTRY_COMPARE(other->computationProgress().value("state").toInt(),2);QCOMPARE(other->computationProgress().value("kind").toInt(),1);QVERIFY(!client.longTaskVisible());
        QVERIFY(other->beginLiquify());QTRY_VERIFY(other->liquifyActive() && !other->liquifyBusy());other->setLiquifySize(24);QVERIFY(other->beginStroke({{36,34},1}));other->strokeTo({{41,34},1});other->endStroke();QTRY_VERIFY(!other->liquifyBusy());QVERIFY(other->finishLiquify(true));QTRY_VERIFY(!other->liquifyActive());QTRY_COMPARE(other->computationProgress().value("state").toInt(),2);QCOMPARE(other->computationProgress().value("kind").toInt(),2);QVERIFY(!other->longTaskVisible());QVERIFY(!client.longTaskVisible());QVERIFY(other->lastError().isEmpty());
        QTRY_COMPARE(QSettings(settings,QSettings::IniFormat).value("performance/taskProgressDelayMs").toInt(),350);PaintCoreClient reopened(nullptr,settings);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.longTaskDelay(),350);reopened.shutdown();
        QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void compoundLayerMovesKeepResidentSources_data() {
        QTest::addColumn<int>("kind");QTest::newRow("linked-owner-mask")<<0;QTest::newRow("masked-group")<<1;QTest::newRow("continuous-multiple")<<2;QTest::newRow("nested-normal-child")<<3;QTest::newRow("nested-pass-through-child")<<4;QTest::newRow("fixed-mask-fallback")<<5;QTest::newRow("ancestor-mask-fallback")<<6;QTest::newRow("empty-fixed-mask")<<7;QTest::newRow("empty-ancestor-mask")<<8;QTest::newRow("zero-opacity-ancestor-mask")<<9;
    }
    void compoundLayerMovesKeepResidentSources() {
        QFETCH(int,kind);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("compound-move.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(96,96);QTRY_COMPARE(client.documentWidth(),96);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushRadius(6);client.setBrushColor(Qt::red);client.brushLibrary()->setPressureSize(false);QVERIFY(client.beginStroke({{32.5,32.5},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);const auto owner=client.activeLayer();
        QVERIFY(client.addMask(owner));QTRY_VERIFY(!client.layerEditBusy());const auto ownerMask=client.activeLayer();client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());
        if(kind==1){QVERIFY(client.groupSelectedLayers("moving group"));QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();QVERIFY(client.addMask(group));QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(group);QTRY_COMPARE(client.activeLayer(),group);QTRY_VERIFY(!client.layerEditBusy());}
        if(kind==2){client.addLayer("partner");QTRY_VERIFY(!client.layerEditBusy());const auto partner=client.activeLayer();client.setBrushColor(Qt::blue);QVERIFY(client.beginStroke({{32.5,60.5},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);client.selectLayers(partner,Qt::ControlModifier);QTRY_COMPARE(client.selectedLayers().size(),2);QTRY_VERIFY(!client.layerEditBusy());}
        if(kind==3 || kind==4){quint64 target=owner;for(int i=0;i<2;++i){QVERIFY(client.groupLayer(target,"ancestor"));QTRY_VERIFY(!client.layerEditBusy());target=client.activeLayer();QVERIFY(client.setLayerBlend(target,kind==3?0:27));QTRY_VERIFY(!client.layerEditBusy());}client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());}
        if(kind==5 || kind==7){QVERIFY(client.setMaskLinked(ownerMask,false));QTRY_VERIFY(!client.layerEditBusy());if(kind==5){client.selectLayer(ownerMask);QTRY_COMPARE(client.activeLayer(),ownerMask);QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(Qt::black);QVERIFY(client.beginStroke({{80,80},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());}}
        if(kind==6 || kind==8 || kind==9){QVERIFY(client.groupLayer(owner,"masked ancestor"));QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.addMask(client.activeLayer()));QTRY_VERIFY(!client.layerEditBusy());if(kind==6 || kind==9){client.setBrushColor(Qt::black);QVERIFY(client.beginStroke({{80,80},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());}if(kind==9){client.setLayerProperties(client.activeLayer(),true,0);QTRY_VERIFY(!client.layerEditBusy());}client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());}
        QQuickWindow window;window.resize(640,480);window.setColor(Qt::black);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({640,480});canvas->setClient(&client);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));window.requestActivate();QTRY_VERIFY(window.isActive());canvas->actualSize();canvas->forceActiveFocus();QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto screen=[&](QPointF p){return(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        const auto capture=[&]{auto grab=canvas->grabToImage();if(!grab || !QTest::qWaitFor([&]{return !grab->image().isNull();},5000))return QImage{};return grab->image();};
        const auto redAt=[&](const QImage &image,QPoint p){if(image.isNull())return false;const auto color=image.pixelColor(p*window.devicePixelRatio());return color.red()>180 && color.green()<70 && color.blue()<70;};
        QTRY_VERIFY(redAt(capture(),screen({32,32})));client.setMoveTool(true);
        if(canvas->gpuDisplay() && kind!=5 && kind!=6)QTRY_VERIFY_WITH_TIMEOUT(client.movePreparation(),30000);
        const auto depth=client.undoDepth();
        QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,screen({32,32}));QTRY_VERIFY(client.layerMoveActive());
        if(qEnvironmentVariable("QT_QUICK_BACKEND")=="rhi")QVERIFY(canvas->gpuDisplay());
        if(canvas->gpuDisplay() && kind!=5 && kind!=6)QTRY_VERIFY(client.residentPreview());
        if(kind==5 || kind==6)QVERIFY(!client.residentPreview());
        const auto source=client.residentPreview();
        for(int i=1;i<=32;++i)QTest::mouseMove(&window,screen({32+i*.5,32}));
        QImage moved;QTRY_VERIFY(!(moved=capture()).isNull() && redAt(moved,screen({48,32})));QVERIFY(!redAt(moved,screen({32,32})));QCOMPARE(client.undoDepth(),depth);
        if(source){QTRY_COMPARE(client.residentPreview()->revision,client.revision());QCOMPARE(client.residentPreview()->sourceId,source->sourceId);QCOMPARE(client.residentPreview()->images[0].cacheKey(),source->images[0].cacheKey());QCOMPARE(client.residentPreview()->vertices.size(),4);}
        QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,screen({48,32}));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(client.frameRevision()>=client.revision() && !client.residentPreview());QTRY_VERIFY(redAt(capture(),screen({48,32})));
        client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_VERIFY(redAt(capture(),screen({32,32})));QVERIFY(client.lastError().isEmpty());canvas->setClient(nullptr);client.shutdown();
    }
    void pendingMoveObservationRejectsUnrelatedState() {
        // Two colors alone also matched recorded wrong displacements. Replay the
        // context counterexample without a window, asynchronous timing or extra input.
        QJsonObject valid{{"generation",3},{"owner",2},{"revision",7},{"sequence",14},{"undo_depth",5},{"error",""},
            {"move",QJsonObject{{"active",true},{"starting",false},{"closing",false},{"dirty",false},
                {"submittedKind",4},{"completedSequence",14},{"deltaX",384},{"deltaY",0}}},
            {"frame",QJsonObject{{"generation",3},{"view",0},{"revision",7}}},
            {"owner_metadata",QJsonObject{{"id",2},{"offsetX",384},{"offsetY",0}}}};
        const auto accepts=[](const QJsonObject &s){return PendingMoveEvidence::acceptedContext(s,3,{384,0},5);};
        QVERIFY(accepts(valid));
        for(const auto &change:QList<QPair<QString,QJsonValue>>{{"deltaX",1880},{"deltaY",56},{"starting",true},
                {"closing",true},{"dirty",true},{"active",false},{"completedSequence",13}}){
            auto wrong=valid;auto move=wrong["move"].toObject();move[change.first]=change.second;wrong["move"]=move;
            QVERIFY2(!accepts(wrong),qPrintable(change.first));
        }
        for(const auto &change:QList<QPair<QString,QJsonValue>>{{"generation",4},{"undo_depth",6},{"error","backend error"}}){
            auto wrong=valid;wrong[change.first]=change.second;QVERIFY2(!accepts(wrong),qPrintable(change.first));
        }
        auto stale=valid;auto frame=stale["frame"].toObject();frame["revision"]=5;stale["frame"]=frame;QVERIFY(!accepts(stale));
        auto wrongOwner=valid;auto owner=wrongOwner["owner_metadata"].toObject();owner["offsetX"]=1880;wrongOwner["owner_metadata"]=owner;QVERIFY(!accepts(wrongOwner));
    }
    void pendingMoveGrabAssociationRejectsStaleAndInterruptedEvidence() {
        CanvasPresentationObservation observation;CanvasPresentationIdentity first;
        first.valid=true;first.imageKey=10;observation.adopted(first);const auto epoch=observation.snapshot().epoch;
        observation.adopted(first);QCOMPARE(observation.snapshot().epoch,epoch);
        auto changed=first;changed.imageKey=11;observation.adopted(changed);observation.adopted(first);
        QCOMPARE(observation.snapshot().epoch,epoch+2); // Actual producer preserves ABA.
        observation.adopted({});QVERIFY(!observation.snapshot().identity.valid);
        // Predicate counterexamples, not a substitute for the visible-window test.
        QJsonObject context{{"generation",3},{"owner",2},{"revision",7},{"sequence",14},{"undo_depth",5},{"error",""},
            {"move",QJsonObject{{"active",true},{"starting",false},{"closing",false},{"dirty",false},
                {"submittedKind",4},{"completedSequence",14},{"deltaX",384},{"deltaY",0}}},
            {"frame",QJsonObject{{"generation",3},{"view",0},{"revision",7},{"request",9},{"frame",20}}},
            {"owner_metadata",QJsonObject{{"id",2},{"offsetX",384},{"offsetY",0}}},
            {"frame_key","1234"},{"frame_region",QJsonArray{0,0,3840,2160}},
            {"canvas","canvas1"},{"window","window1"},{"active",true},{"visible",true},{"canvas_visible",true},
            {"dpr",1},{"zoom",0.25},{"document_origin",QJsonArray{32,90}},{"size",QJsonArray{1024,720}},
            {"context_epoch",10},{"input_epoch",2},{"target_plan",2},{"preview_source",0},{"preview_revision",0},
            {"begin_sequence",13},{"target_sequence",14},
            {"scenario_invalid",false},{"client_matches",true}};
        const QJsonObject adopted{{"epoch",3},{"valid",true},{"canvas","canvas1"},{"window","window1"},{"generation",3},
            {"resident",false},{"image_key","1234"},{"region",QJsonArray{0,0,3840,2160}},
            {"document_origin",QJsonArray{32,90}},{"zoom",0.25},{"size",QJsonArray{1024,720}}};
        context["render"]=adopted;
        const auto view=PendingMoveEvidence::viewIdentity(context);
        auto notified=context;notified["revision"]=8;QCOMPARE(PendingMoveEvidence::viewIdentity(notified),view);
        notified["zoom"]=0.5;QVERIFY(PendingMoveEvidence::viewIdentity(notified)!=view);
        const auto make=[](QJsonObject s){QJsonObject sample{{"id",1},{"ready_id",1},{"completed",true},{"image_key","grab1"},{"grab_canvas","canvas1"}};
            for(const auto *key:{"request_state","ready_state","complete_state","assertion_state"})sample[key]=s;return sample;};
        const auto accepts=[](const QJsonObject &s){return PendingMoveEvidence::associated(s,3,{384,0},5);};
        const auto valid=make(context);QVERIFY(accepts(valid));
        // GUI correct at every endpoint, but the scene graph is still old.
        auto wrong=context;auto render=adopted;render["image_key"]="old";wrong["render"]=render;QVERIFY(!accepts(make(wrong)));
        wrong=context;auto frame=wrong["frame"].toObject();frame["revision"]=6;wrong["frame"]=frame;QVERIFY(!accepts(make(wrong)));
        // Same colors and target displacement, different operation sequence.
        auto other=valid;wrong=context;wrong["sequence"]=13;other["request_state"]=wrong;QVERIFY(!accepts(other));
        wrong=context;wrong["sequence"]=15;QVERIFY(!accepts(make(wrong))); // Entire grab belongs to another operation.
        // ABA: input, document/view, cancellation or content changed then returned.
        for(const auto *field:{"input_epoch","context_epoch","target_plan"}) {
            auto interrupted=valid;wrong=context;wrong[field]=context[field].toInt()+1;
            interrupted["ready_state"]=wrong;QVERIFY2(!accepts(interrupted),field);
        }
        auto interrupted=valid;wrong=context;wrong["scenario_invalid"]=true;interrupted["ready_state"]=wrong;QVERIFY(!accepts(interrupted));
        interrupted=valid;wrong=context;render=adopted;render["epoch"]=5;wrong["render"]=render;interrupted["ready_state"]=wrong;QVERIFY(!accepts(interrupted));
        for(const auto *field:{"generation","zoom"}) {interrupted=valid;wrong=context;wrong[field]=99;interrupted["ready_state"]=wrong;QVERIFY(!accepts(interrupted));}
        interrupted=valid;interrupted["grab_canvas"]="otherCanvas";QVERIFY(!accepts(interrupted));
        interrupted=valid;interrupted["ready_id"]=2;QVERIFY(!accepts(interrupted));
        // A later, genuinely adopted target is allowed, rather than permanently
        // pinning the first revision/image. Runtime validates its actual pixels.
        auto next=context;next["revision"]=8;next["sequence"]=15;next["target_sequence"]=15;next["frame_key"]="new";next["context_epoch"]=11;
        auto move=next["move"].toObject();move["completedSequence"]=15;next["move"]=move;
        frame=next["frame"].toObject();frame["revision"]=8;frame["frame"]=21;next["frame"]=frame;
        render=adopted;render["epoch"]=4;render["image_key"]="new";next["render"]=render;QVERIFY(accepts(make(next)));
    }
    void prewarmed4kResidentMoveReusesFullQualityTexturesAndRestoresFullFrame() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("first-move.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(3840,2160);
        QTRY_COMPARE(client.documentWidth(),3840);QTRY_VERIFY(!client.layerEditBusy());client.addLayer("moving");QTRY_VERIFY(!client.layerEditBusy());
        const auto owner=client.activeLayer();client.activateBucket();client.setBrushColor(Qt::red);QVERIFY(client.bucketFill({0,0}));QTRY_VERIFY_WITH_TIMEOUT(!client.layerEditBusy(),30000);
        QVERIFY(client.addMask(owner));QTRY_VERIFY(!client.layerEditBusy());client.activateBrush();client.setBrushColor(Qt::black);client.setBrushRadius(90);client.brushLibrary()->setPressureSize(false);
        QVERIFY(client.beginStroke({{2000.5,1080.5},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.groupLayer(owner,"group"));QTRY_VERIFY(!client.layerEditBusy());
        client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());const auto depth=client.undoDepth();
        QQuickWindow window;window.resize(1024,720);window.setColor(Qt::black);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({1024,720});canvas->setClient(&client);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));window.requestActivate();QTRY_VERIFY(window.isActive());canvas->fitToView();canvas->forceActiveFocus();QTRY_VERIFY(client.frameRevision()>=client.revision());
        if(qEnvironmentVariable("QT_QUICK_BACKEND")=="rhi")QVERIFY(canvas->gpuDisplay());
        PendingMoveEvidence evidence(canvas,&client,&window,owner);
        const auto screen=[&](QPointF p){return(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        const auto capture=[&]{return evidence.capture();};
        const auto red=[&](const QImage &image,QPointF p){const auto c=image.pixelColor(screen(p)*window.devicePixelRatio());return c.red()>180 && c.green()<70 && c.blue()<70;};
        const auto white=[&](const QImage &image,QPointF p){const auto c=image.pixelColor(screen(p)*window.devicePixelRatio());return c.red()>230 && c.green()>230 && c.blue()>230;};
        QImage initial;QTRY_VERIFY(!(initial=capture()).isNull() && red(initial,{100,1080}) && white(initial,{2000,1080}));
        // Activate a selection tool first, then move directly without another canvas gesture.
        client.activateMarquee();QVERIFY(client.selectionTool()>0);
        QElapsedTimer preparation;preparation.start();client.setMoveTool(true);
        QVERIFY(!client.movePreparation());
        evidence.phase("unprepared-press");
        // This fixture starts at offset (0,0). Derive the integer target from
        // the actual rounded screen coordinates, using the same inverse zoom.
        evidence.expect(client.generation(),QPoint(qRound((screen({1584,1080}).x()-screen({1200,1080}).x())/canvas->zoom()),
            qRound((screen({1584,1080}).y()-screen({1200,1080}).y())/canvas->zoom())),depth);
        QElapsedTimer pendingInput;pendingInput.start();
        evidence.send(QEvent::MouseButtonPress,screen({1200,1080}));
        evidence.send(QEvent::MouseMove,screen({1584,1080}));
        evidence.mark("after-planned-mouse-move");
        QTRY_VERIFY(client.layerMoveActive());
        const auto pendingSource=client.residentPreview();
        if(pendingSource)QVERIFY(pendingSource->images[0].width()>512);
        evidence.phase("pendingMoved");
        QImage pendingMoved;QTRY_VERIFY(evidence.pendingMatches(pendingMoved=capture()));
        evidence.passed();
        qInfo()<<"4K move accepts input before prepared assets ms"<<pendingInput.elapsed()<<"gpu"<<canvas->gpuDisplay();
        QTest::keyClick(&window,Qt::Key_Escape);QTRY_VERIFY(!client.layerMoveActive());QCOMPARE(client.undoDepth(),depth);
        QTRY_VERIFY(!client.frame().isNull());
        QTRY_VERIFY_WITH_TIMEOUT(client.movePreparation(),30000);
        if(canvas->gpuDisplay())QTRY_VERIFY_WITH_TIMEOUT(client.movePreparation() && client.movePreparation()->uploads->load()>0,30000);
        qInfo()<<"4K move preparation and texture warmup ms"<<preparation.elapsed()<<"gpu"<<canvas->gpuDisplay();
        const quint64 uploads=client.movePreparation()?client.movePreparation()->uploads->load():0;
        QElapsedTimer elapsed;elapsed.start();QElapsedTimer beat;beat.start();qint64 maxGuiGap=0;QTimer heartbeat;heartbeat.setInterval(1);connect(&heartbeat,&QTimer::timeout,this,[&]{maxGuiGap=qMax(maxGuiGap,beat.restart());});heartbeat.start();
        evidence.send(QEvent::MouseButtonPress,screen({1200,1080}));
        // The first delta only reuses the prepared source and moves four vertices.
        evidence.send(QEvent::MouseMove,screen({1584,1080}));
        QImage moved;QTRY_VERIFY(!(moved=capture()).isNull() && white(moved,{100,1080}) && red(moved,{800,1080}) && white(moved,{2384,1080}));
        heartbeat.stop();qInfo()<<"4K first move through captured changed frame ms"<<elapsed.elapsed()<<"max GUI heartbeat gap ms"<<maxGuiGap<<"gpu"<<canvas->gpuDisplay();
        const auto source=client.residentPreview();
        if(canvas->gpuDisplay()){QVERIFY(source);QCOMPARE(source->vertices.size(),4);QVERIFY(source->images[0].width()>512);QVERIFY(source->images[0].width()<=2048);QCOMPARE(source->images[2].size(),QSize(1,1));QCOMPARE(source->uploads->load(),uploads);}
        evidence.send(QEvent::MouseMove,screen({1648,1080}));
        if(source){QTRY_COMPARE(client.residentPreview()->revision,client.revision());QCOMPARE(client.residentPreview()->sourceId,source->sourceId);QCOMPARE(client.residentPreview()->images[0].cacheKey(),source->images[0].cacheKey());QCOMPARE(source->uploads->load(),uploads);}
        evidence.send(QEvent::MouseButtonRelease,screen({1648,1080}));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(client.frameRevision()>=client.revision() && !client.residentPreview());QVERIFY(client.frame().width()>512);
        client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(client.frameRevision()>=client.revision());QImage undone;QTRY_VERIFY(!(undone=capture()).isNull() && red(undone,{100,1080}));
        QVERIFY(client.lastError().isEmpty());canvas->setClient(nullptr);client.shutdown();
    }
    void complex4kLayerMoveRestoresFullFramesAndHistory_data() {
        QTest::addColumn<bool>("ancestorMask");
        QTest::newRow("fixed-owner-mask")<<false;QTest::newRow("fixed-ancestor-mask")<<true;
    }
    void complex4kLayerMoveRestoresFullFramesAndHistory() {
        QFETCH(bool,ancestorMask);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("4k-complex-move.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(3840,2160);
        QTRY_COMPARE(client.documentWidth(),3840);QTRY_VERIFY(!client.layerEditBusy());client.addLayer("moving full layer");QTRY_VERIFY(!client.layerEditBusy());
        const auto owner=client.activeLayer();client.activateBucket();client.setBrushColor(Qt::red);QVERIFY(client.bucketFill({0,0}));QTRY_VERIFY_WITH_TIMEOUT(!client.layerEditBusy(),30000);
        quint64 maskOwner=owner;if(ancestorMask){QVERIFY(client.groupLayer(owner,"masked ancestor"));QTRY_VERIFY(!client.layerEditBusy());maskOwner=client.activeLayer();}
        QVERIFY(client.addMask(maskOwner));QTRY_VERIFY(!client.layerEditBusy());const auto mask=client.activeLayer();
        client.activateBrush();client.setBrushColor(Qt::black);client.setBrushRadius(90);client.brushLibrary()->setPressureSize(false);
        QVERIFY(client.beginStroke({{2000.5,1080.5},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());
        if(!ancestorMask){QVERIFY(client.setMaskLinked(mask,false));QTRY_VERIFY(!client.layerEditBusy());}
        client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());const auto depth=client.undoDepth();
        QQuickWindow window;window.resize(1024,720);window.setColor(Qt::black);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({1024,720});canvas->setClient(&client);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));window.requestActivate();QTRY_VERIFY(window.isActive());canvas->fitToView();canvas->forceActiveFocus();QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto screen=[&](QPointF p){return(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        const auto capture=[&]{auto grab=canvas->grabToImage();if(!grab || !QTest::qWaitFor([&]{return !grab->image().isNull();},5000))return QImage{};return grab->image();};
        const auto at=[&](const QImage &image,QPointF p){const auto v=screen(p);return image.pixelColor(qRound(v.x()*image.width()/canvas->width()),qRound(v.y()*image.height()/canvas->height()));};
        const auto red=[](QColor c){return c.red()>200 && c.green()<60 && c.blue()<60;};
        const auto white=[](QColor c){return c.red()>220 && c.green()>220 && c.blue()>220;};
        QImage initial;const auto report=[&]{initial.save("artifacts/move-sparse-initial.png");client.frame().save("artifacts/move-sparse-frame.png");return QString("capture=%1x%2 zoom=%3 origin=%4,%5 first=%6 mask=%7 frame=%8/%9").arg(initial.width()).arg(initial.height()).arg(canvas->zoom()).arg(canvas->documentRect().x()).arg(canvas->documentRect().y()).arg(initial.isNull()?QString("null"):at(initial,{100,1080}).name()).arg(initial.isNull()?QString("null"):at(initial,{2000,1080}).name()).arg(client.frameRevision()).arg(client.revision());};
        QTRY_VERIFY2(!(initial=capture()).isNull() && red(at(initial,{100,1080})) && white(at(initial,{2000,1080})),qPrintable(report()));QVERIFY(client.frame().width()>512);
        client.setMoveTool(true);
        // Cancel and commit are both covered by the prewarmed first-drag case;
        // this bounded fallback row keeps one transition and its final history.
        for(bool commit:{true}) {
            QElapsedTimer timer;timer.start();QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,screen({1200,1080}));QTRY_VERIFY(client.layerMoveActive());QVERIFY(!client.residentPreview());
            for(int i=1;i<=12;++i)QTest::mouseMove(&window,screen({1200+32.*i,1080}));
            const auto clearPreview=[&]{
                const auto preview=client.residentPreview();
                return preview?preview->images[0].width()>512
                    :client.frameRevision()>=client.revision() && client.frame().width()>512;
            };
            QTRY_VERIFY_WITH_TIMEOUT(clearPreview(),30000);
            qInfo()<<"4K complex move through captured preview ms"<<timer.elapsed()<<"ancestor mask"<<ancestorMask;
            QCOMPARE(client.undoDepth(),depth);
            if(!commit)QTest::keyClick(&window,Qt::Key_Escape);
            QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,screen({1584,1080}));QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().width()>512);
            QTRY_COMPARE(client.undoDepth(),depth+(commit?1:0));QImage final;
            QTRY_VERIFY(!(final=capture()).isNull() && (commit?white(at(final,{100,1080})):red(at(final,{100,1080}))) && white(at(final,{2000,1080})));
        }
        client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy() && client.frameRevision()>=client.revision());QImage undone;QTRY_VERIFY(!(undone=capture()).isNull() && red(at(undone,{100,1080})) && white(at(undone,{2000,1080})));
        client.redo();QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy() && client.frameRevision()>=client.revision());QImage redone;QTRY_VERIFY(!(redone=capture()).isNull() && white(at(redone,{100,1080})) && white(at(redone,{2000,1080})));
        QVERIFY(client.lastError().isEmpty());canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void selectionDeleteAndCopyEditPixelsWithoutRemovingTheLayer() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("selection-edit.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);window->requestActivate();QVERIFY(QTest::qWaitForWindowActive(window));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());client.setBrushRadius(8);client.setBrushColor(Qt::red);client.brushLibrary()->setPressureSize(false);
        for(const auto &p:{QPointF(32,32),QPointF(96,96)}){QVERIFY(client.beginStroke({p,1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());}
        QVERIFY(client.editSelectionPath({{8,8},{56,8},{56,56},{8,56}},0,false));QTRY_VERIFY(!client.layerEditBusy() && client.selectionEnabled());const auto depth=client.undoDepth();const auto layers=client.layers().size();const auto owner=client.activeLayer();
        qInfo("selection phase: prepared");auto *saved=new QMimeData;const auto *original=QGuiApplication::clipboard()->mimeData();if(original)for(const auto &format:original->formats())saved->setData(format,original->data(format));const auto restore=qScopeGuard([saved]{QGuiApplication::clipboard()->setMimeData(saved);});QGuiApplication::clipboard()->clear();
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_C,Qt::ControlModifier);QTRY_VERIFY(!client.layerEditBusy() && QGuiApplication::clipboard()->mimeData()->hasFormat("application/x-drawverse-layers-v1"));QCOMPARE(client.undoDepth(),depth);
        qInfo("selection phase: copied");PaintCoreClient pasted(nullptr,temp.filePath("pasted.ini"));QTRY_VERIFY(pasted.ready() && pasted.brushLibraryReady());pasted.newTransparentDocument(128,128);QTRY_COMPARE(pasted.documentWidth(),128);QTRY_VERIFY(!pasted.layerEditBusy());QVERIFY(pasted.pasteLayers());QTRY_VERIFY(!pasted.layerEditBusy());QTRY_VERIFY(pasted.sampleDocumentPixel(32,32).alpha()>240);QCOMPARE(pasted.sampleDocumentPixel(96,96).alpha(),0);
        qInfo("selection phase: pasted");window->requestActivate();QVERIFY(QTest::qWaitForWindowActive(window));canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Delete);QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.layers().size(),layers);QCOMPARE(client.activeLayer(),owner);QVERIFY(client.selectionEnabled());QTRY_COMPARE(client.sampleDocumentPixel(32,32).alpha(),0);QVERIFY(client.sampleDocumentPixel(96,96).alpha()>240);QCOMPARE(client.historyEntries().last().toMap().value("title").toString(),QString("清除选区内容"));
        client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.sampleDocumentPixel(32,32).alpha()>240);client.redo();QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(client.sampleDocumentPixel(32,32).alpha(),0);QVERIFY(client.selectionEnabled());
        QSignalSpy targetStopped(&pasted,&PaintCoreClient::stopped);pasted.shutdown();QTRY_COMPARE(targetStopped.size(),1);auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void eachBrushToolRemembersSelectionInEveryIsolationMode_data() {
        QTest::addColumn<int>("mode");for(int mode=0;mode<3;++mode)QTest::newRow(qPrintable(QString::number(mode)))<<mode;
    }
    void eachBrushToolRemembersSelectionInEveryIsolationMode() {
        QFETCH(int,mode);QTemporaryDir temp;const auto path=temp.filePath("tool-selections.ini");
        const QStringList ids{"round-fine","round-wide","round-translucent"};
        {
            PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready() && client.brushLibraryReady());auto *library=client.brushLibrary();library->setToolIsolation(mode);
            client.activateBrush();QVERIFY(library->select(ids[0]));client.activateEraser();QVERIFY(library->select(ids[1]));client.activateSmudge();QVERIFY(library->select(ids[2]));
            for(int i=0;i<3;++i){client.activateBrush();QCOMPARE(library->selectedId(),ids[0]);client.activateEraser();QCOMPARE(library->selectedId(),ids[1]);client.activateSmudge();QCOMPARE(library->selectedId(),ids[2]);}
            const auto stored=library->snapshot();BrushLibrary restored;QVERIFY(restored.restore(stored));for(int tool=0;tool<3;++tool){restored.setToolContext(tool);QCOMPARE(restored.selectedId(),ids[tool]);}
            // Isolation governs parameters, while changing it preserves each choice.
            for(int next=0;next<3;++next){restored.setToolIsolation(next);for(int tool=0;tool<3;++tool){restored.setToolContext(tool);QCOMPARE(restored.selectedId(),ids[tool]);}}
            const auto before=library->snapshot();auto invalid=QJsonDocument::fromJson(before).object();auto scopes=invalid.value("toolIsolation").toObject();scopes["selections"]=QJsonArray{"missing","round-wide","round-translucent"};invalid["toolIsolation"]=scopes;QVERIFY(!library->restore(QJsonDocument(invalid).toJson()));QCOMPARE(library->snapshot(),before);
            auto legacy=QJsonDocument::fromJson(before).object();scopes=legacy.value("toolIsolation").toObject();scopes["version"]=1;scopes.remove("selections");legacy["toolIsolation"]=scopes;BrushLibrary migrated;QVERIFY(migrated.restore(QJsonDocument(legacy).toJson()));if(mode<2)for(int tool=0;tool<3;++tool){migrated.setToolContext(tool);QCOMPARE(migrated.selectedId(),ids[2]);}
            library->flushPersistence();QTRY_VERIFY(!library->persistencePending());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        }
        PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready() && reopened.brushLibraryReady());reopened.activateBrush();QCOMPARE(reopened.brushLibrary()->selectedId(),ids[0]);reopened.activateEraser();QCOMPARE(reopened.brushLibrary()->selectedId(),ids[1]);reopened.activateSmudge();QCOMPARE(reopened.brushLibrary()->selectedId(),ids[2]);QVERIFY(reopened.lastError().isEmpty());QSignalSpy stopped(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void isolatedToolSwitchRetainsPreviewJobsWithoutRebuildingCatalog_data() {
        QTest::addColumn<int>("mode");QTest::newRow("size-opacity-flow")<<1;QTest::newRow("all-parameters")<<2;
    }
    void isolatedToolSwitchRetainsPreviewJobsWithoutRebuildingCatalog() {
        QFETCH(int,mode);BrushLibrary library;auto root=QJsonDocument::fromJson(library.catalogSnapshot()).object();auto rows=root.value("presets").toArray();
        for(int i=4;i<512;++i){auto row=rows.first().toObject();row["id"]=QString("switch-%1").arg(i);rows.append(row);}root["presets"]=rows;QVERIFY(library.restore(QJsonDocument(root).toJson()));
        library.setToolIsolation(mode);quint64 token=0;connect(&library,&BrushLibrary::previewRequested,this,[&](const QString &,quint64 t,qreal,qreal,qreal,QVariantMap){token=t;});
        library.requestPreview(library.selectedId());QVERIFY(token);library.acceptPreview(library.selectedId(),token,"shared-preview");
        library.setToolContext(1);library.setRadius(31);if(mode==2)library.setHardness(.3);library.setToolContext(2);library.setRadius(42);library.setToolContext(0);
        QSignalSpy rebuilt(&library,&BrushLibrary::presetsChanged);QElapsedTimer timer;timer.start();
        for(int i=0;i<10;++i)for(int tool:{1,2,0})library.setToolContext(tool);
        qInfo()<<"512-preset isolated tool switches mode"<<mode<<"30 switches us"<<timer.nsecsElapsed()/1000;
        QCOMPARE(library.radius(),12.);QCOMPARE(library.selectedPreview(),QString("shared-preview"));
        // A real profile difference still restores its parameters and invalidates its thumbnail.
        library.setToolContext(1);QCOMPARE(library.radius(),31.);if(mode==2){QCOMPARE(library.hardness(),.3);QVERIFY(library.selectedPreview().isEmpty());}
        QCOMPARE(rebuilt.size(),0);
    }
    void isolatedToolSnapshotsMatchFreshEncoding_data() {QTest::addColumn<int>("mode");QTest::newRow("partial")<<1;QTest::newRow("full")<<2;}
    void isolatedToolSnapshotsMatchFreshEncoding() {
        QFETCH(int,mode);BrushLibrary library;auto root=QJsonDocument::fromJson(library.catalogSnapshot()).object();auto rows=root.value("presets").toArray();
        for(int i=4;i<512;++i){auto row=rows.first().toObject();row["id"]=QString("snapshot-%1").arg(i);rows.append(row);}root["presets"]=rows;QVERIFY(library.restore(QJsonDocument(root).toJson()));library.setToolIsolation(mode);
        for(int tool=0;tool<3;++tool){library.setToolContext(tool);QVERIFY(library.select(QStringList{"round-pressure","round-fine","round-wide"}[tool]));library.setRadius(17+tool*11);library.setHardness(.2+tool*.2);library.setWriteBlend(tool);}
        QByteArray published;connect(&library,&BrushLibrary::selectionChanged,this,[&]{published=library.snapshot();});
        library.snapshot();library.catalogSnapshot();QElapsedTimer timer;timer.start();qsizetype bytes=0;
        for(int i=0;i<10;++i)for(int tool:{0,1,2}){library.setToolContext(tool);bytes+=library.snapshot().size()+library.catalogSnapshot().size();}
        qInfo()<<"512-preset switch + preference/catalog snapshots mode"<<mode<<"30 switches ms"<<timer.nsecsElapsed()/1000000.<<"bytes"<<bytes;
        for(int tool:{0,1,2,0,2,1}) {
            library.setToolContext(tool);const auto fast=library.snapshot(),catalog=library.catalogSnapshot();QCOMPARE(published,fast);
            // Force the independent full construction path without changing final values.
            const auto width=library.cursorWidth();library.setCursorWidth(width==0?1:0);library.setCursorWidth(width);
            QCOMPARE(library.snapshot(),fast);QCOMPARE(library.catalogSnapshot(),catalog);
            BrushLibrary reopened;QVERIFY(reopened.restore(fast));QCOMPARE(reopened.toolContext(),tool);QCOMPARE(reopened.selectedId(),library.selectedId());QCOMPARE(reopened.radius(),library.radius());QCOMPARE(reopened.hardness(),library.hardness());QCOMPARE(reopened.writeBlend(),library.writeBlend());
        }
        QVERIFY(!library.createOutlinePreset(false).isEmpty());QVERIFY(library.rename("snapshot-10","changed shared name"));library.setHardness(.77);library.snapshot();library.catalogSnapshot();library.setToolContext(2);
        const auto edited=library.snapshot(),catalog=library.catalogSnapshot();library.setCursorWidth(1);library.setCursorWidth(0);QCOMPARE(library.snapshot(),edited);QCOMPARE(library.catalogSnapshot(),catalog);
    }
    void tabletBackendSelectionChecksDriverStateAndPersistsChoice() {
#ifdef Q_OS_WIN
        configureTabletInput("windows-ink");QCOMPARE(activeTabletInputBackend(),QString("windows-ink"));
        configureTabletInput("auto");QVERIFY(activeTabletInputBackend()=="windows-ink" || activeTabletInputBackend()=="wintab");
#endif
        for(const auto &mode:{QString("auto"),QString("windows-ink"),QString("wintab")})for(bool driver:{false,true}) {
            bool enabled=false;int calls=0;
            const auto result=selectTabletInputBackend(mode,[&](bool requested){++calls;enabled=requested && driver;return true;},[&]{return enabled;});
            QCOMPARE(calls,1);QCOMPARE(result,(driver && mode!="windows-ink")?QString("wintab"):QString("windows-ink"));
        }
        QPointingDevice winTab("wintab",9190,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        QPointingDevice windowsInk("wmpointer",9191,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        QVERIFY(acceptTabletInputDevice("wintab",&winTab));QVERIFY(!acceptTabletInputDevice("wintab",&windowsInk));
        QVERIFY(acceptTabletInputDevice("windows-ink",&windowsInk));QVERIFY(acceptTabletInputDevice("auto",&windowsInk));
        QTemporaryDir temp;const auto path=temp.filePath("tablet.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());
        QCOMPARE(client.tabletInputMode(),QString("auto"));QSignalSpy changes(&client,&PaintCoreClient::tabletInputSettingsChanged);
        client.setTabletInputMode("wintab");QCOMPARE(client.tabletInputMode(),QString("wintab"));QCOMPARE(changes.size(),1);
        client.setTabletInputMode("invalid");QCOMPARE(changes.size(),1);QCOMPARE(QSettings(path,QSettings::IniFormat).value("input/tabletApi").toString(),QString("wintab"));
        client.setTabletInputMode("windows-ink");QCOMPARE(client.tabletInputMode(),QString("windows-ink"));
        WorkspaceManager workspace(temp.filePath("tablet-layout.ini"));QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
        QQmlComponent component(&engine,QUrl("qrc:/qml/preferences/PreferencesPanel.qml"));QScopedPointer<QObject> dialog(component.create());QVERIFY2(dialog,qPrintable(component.errorString()));
        auto *combo=dialog->findChild<QObject*>("tabletInputApi");QVERIFY(combo);QCOMPARE(combo->property("count").toInt(),3);QCOMPARE(combo->property("currentIndex").toInt(),1);
        combo->setProperty("currentIndex",2);QVERIFY(QMetaObject::invokeMethod(combo,"activated",Q_ARG(int,2)));QCOMPARE(client.tabletInputMode(),QString("wintab"));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void tabletProximityLossEndsStrokeAndEditingRemainsUsable_data() {
        QTest::addColumn<bool>("liquify");QTest::newRow("brush")<<false;QTest::newRow("liquify")<<true;
    }
    void tabletProximityLossEndsStrokeAndEditingRemainsUsable() {
        QFETCH(bool,liquify);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("pen-proximity.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(128,128);
        QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());client.setBrushRadius(12);
        QVERIFY(client.beginStroke({{64,64},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());
        QQuickWindow window;window.resize(512,512);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({512,512});canvas->setClient(&client);
        auto *focus=new QQuickItem(window.contentItem());focus->setFlag(QQuickItem::ItemIsFocusScope);window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->fitToView();
        if(liquify){QVERIFY(client.beginLiquify());QTRY_VERIFY(!client.liquifyBusy());}
        focus->forceActiveFocus();QVERIFY(!canvas->hasActiveFocus());
        QPointingDevice pen("proximity pen",9182,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto at=canvas->documentRect().center();
        const auto send=[&](QEvent::Type type,QPointF p){QTabletEvent e(type,&pen,p,canvas->mapToGlobal(p),type==QEvent::TabletRelease?0.:.6,0,0,0,0,0,Qt::NoModifier,type==QEvent::TabletMove?Qt::NoButton:Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(&window,&e);};
        send(QEvent::TabletPress,at);QVERIFY(client.drawing());QVERIFY(canvas->hasActiveFocus());send(QEvent::TabletMove,at+QPointF(20,0));
        QTabletEvent leave(QEvent::TabletLeaveProximity,&pen,at,canvas->mapToGlobal(at),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(qApp,&leave);
        QTRY_VERIFY(!client.drawing() && !client.layerEditBusy() && !client.liquifyBusy());
        // A late release must not end the next stroke or leave the canvas pen-owned.
        send(QEvent::TabletRelease,at);send(QEvent::TabletPress,at);QVERIFY(client.drawing());send(QEvent::TabletRelease,at);
        QTRY_VERIFY(!client.drawing() && !client.layerEditBusy() && !client.liquifyBusy());
        if(liquify){QVERIFY(client.finishLiquify(false));QTRY_VERIFY(!client.liquifyActive() && !client.liquifyBusy());}
        QVERIFY(client.beginTransform());QTRY_VERIFY(!client.transformBusy());QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive() && !client.transformBusy());
        QTRY_VERIFY(client.frameRevision()>=client.revision());auto grab=canvas->grabToImage();QVERIFY(grab);QTRY_VERIFY(!grab->image().isNull());
        QVERIFY(client.lastError().isEmpty());canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void duplicateTabletPressStillEndsOnRelease() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("pen-duplicate.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(128,128);
        QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());client.setBrushRadius(12);
        QQuickWindow window;window.resize(512,512);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({512,512});canvas->setClient(&client);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->fitToView();
        QPointingDevice pen("duplicate pen",9192,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        QPointingDevice windowsInk("wmpointer",9193,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto at=canvas->documentRect().center();
        const auto send=[&](const QPointingDevice *device,QEvent::Type type,QPointF p){QTabletEvent e(type,device,p,canvas->mapToGlobal(p),type==QEvent::TabletRelease?0.:.6,0,0,0,0,0,Qt::NoModifier,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(&window,&e);};
#ifdef Q_OS_WIN
        configureTabletInput("wintab");
        if(activeTabletInputBackend()==QStringLiteral("wintab")) {
            send(&windowsInk,QEvent::TabletPress,at);QVERIFY(!client.drawing());
        }
#endif
        send(&pen,QEvent::TabletPress,at);QVERIFY(client.drawing());
#ifdef Q_OS_WIN
        if(activeTabletInputBackend()==QStringLiteral("wintab")) {
            QTabletEvent leave(QEvent::TabletLeaveProximity,&windowsInk,at,canvas->mapToGlobal(at),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);
            QCoreApplication::sendEvent(&window,&leave);QVERIFY(client.drawing());
        }
#endif
        send(&pen,QEvent::TabletPress,at+QPointF(1,0));QVERIFY(client.drawing());
        send(&pen,QEvent::TabletMove,at+QPointF(20,0));QVERIFY(client.drawing());
        send(&pen,QEvent::TabletRelease,at+QPointF(20,0));
        QTRY_VERIFY(!client.drawing() && !client.layerEditBusy());
        QTRY_COMPARE(client.undoDepth(),1);QVERIFY(client.lastError().isEmpty());
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void shortcutCursorSurvivesModifierlessTabletMoves() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("pen-shortcuts.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(128,128);
        QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());
        QQuickWindow window;window.resize(512,512);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({512,512});canvas->setClient(&client);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->fitToView();
        const auto at=canvas->documentRect().center();QTest::mouseMove(&window,QPoint(1,1));QTest::mouseMove(&window,canvas->mapToScene(at).toPoint());
        QVERIFY(canvas->brushCursorVisible());QCOMPARE(window.cursor().shape(),Qt::BlankCursor);
        QPointingDevice pen("shortcut pen",9194,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto hover=[&]{QTabletEvent e(QEvent::TabletMove,&pen,at,canvas->mapToGlobal(at),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(&window,&e);};
        const auto mouseRelease=[&]{QMouseEvent e(QEvent::MouseButtonRelease,at,canvas->mapToGlobal(at),Qt::NoButton,Qt::NoButton,Qt::NoModifier);QCoreApplication::sendEvent(canvas,&e);};
        QTest::keyPress(&window,Qt::Key_Alt);QTRY_VERIFY(canvas->eyedropperCursorVisible());
        mouseRelease();QVERIFY(canvas->eyedropperCursorVisible());QTest::keyRelease(&window,Qt::Key_Alt);QVERIFY(!canvas->eyedropperCursorVisible());
        QTest::keyPress(&window,Qt::Key_Control);QTRY_COMPARE(canvas->cursor().shape(),Qt::SizeAllCursor);QTRY_COMPARE(window.cursor().shape(),Qt::SizeAllCursor);
        mouseRelease();QCOMPARE(canvas->cursor().shape(),Qt::SizeAllCursor);QTest::keyRelease(&window,Qt::Key_Control);QTRY_COMPARE(window.cursor().shape(),Qt::BlankCursor);
        QTest::keyPress(&window,Qt::Key_Alt);QTRY_VERIFY(canvas->eyedropperCursorVisible());
        hover();QVERIFY(canvas->eyedropperCursorVisible());
        QTest::keyRelease(&window,Qt::Key_Alt);QTRY_VERIFY(!canvas->eyedropperCursorVisible());
        hover();QVERIFY(!canvas->eyedropperCursorVisible());
        QTest::keyPress(&window,Qt::Key_Control);QTRY_COMPARE(canvas->cursor().shape(),Qt::SizeAllCursor);
        hover();QCOMPARE(canvas->cursor().shape(),Qt::SizeAllCursor);
        QTest::keyRelease(&window,Qt::Key_Control);QTRY_VERIFY(canvas->cursor().shape()!=Qt::SizeAllCursor);
        hover();QVERIFY(canvas->cursor().shape()!=Qt::SizeAllCursor);
        QKeyEvent lostAlt(QEvent::KeyPress,Qt::Key_Alt,Qt::AltModifier);QCoreApplication::sendEvent(&window,&lostAlt);
        QTRY_VERIFY(canvas->eyedropperCursorVisible());QTRY_VERIFY_WITH_TIMEOUT(!canvas->eyedropperCursorVisible(),600);
        QKeyEvent lostCtrl(QEvent::KeyPress,Qt::Key_Control,Qt::ControlModifier);QCoreApplication::sendEvent(&window,&lostCtrl);
        QTRY_COMPARE(canvas->cursor().shape(),Qt::SizeAllCursor);QTRY_VERIFY_WITH_TIMEOUT(canvas->cursor().shape()!=Qt::SizeAllCursor,600);
        QVERIFY(client.lastError().isEmpty());canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void duplicateMousePressStillEndsOnRelease() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("mouse-duplicate.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(128,128);
        QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());client.setBrushRadius(12);
        QQuickWindow window;window.resize(512,512);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({512,512});canvas->setClient(&client);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->fitToView();
        const auto at=canvas->documentRect().center(),moved=at+QPointF(20,0);
        const auto send=[&](QEvent::Type type,QPointF p,Qt::MouseButton button,Qt::MouseButtons buttons){QMouseEvent e(type,p,canvas->mapToGlobal(p),button,buttons,Qt::NoModifier);QCoreApplication::sendEvent(canvas,&e);};
        send(QEvent::MouseButtonPress,at,Qt::LeftButton,Qt::LeftButton);QVERIFY(client.drawing());
        send(QEvent::MouseButtonPress,at,Qt::LeftButton,Qt::LeftButton);QVERIFY(client.drawing());
        send(QEvent::MouseMove,moved,Qt::NoButton,Qt::LeftButton);QVERIFY(client.drawing());
        send(QEvent::MouseButtonRelease,moved,Qt::LeftButton,Qt::NoButton);
        QTRY_VERIFY(!client.drawing() && !client.layerEditBusy());QTRY_COMPARE(client.undoDepth(),1);
        QVERIFY(client.lastError().isEmpty());canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void transformKeepsFloatingPanelsAndUsesCorrectFileGuard() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("panels.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        const auto group=workspace.groupForPanel("brush");workspace.detachGroup(group);QQuickWindow *panel=nullptr;
        QTRY_VERIFY([&]{for(auto *w:QGuiApplication::allWindows())if(w->objectName()=="floatingDock:"+group)panel=qobject_cast<QQuickWindow*>(w);return panel && panel->isVisible();}());
        QVERIFY(client.beginTransform());QTRY_VERIFY(!client.transformBusy());QVERIFY(panel->isVisible());auto *status=findVisualItem(window->contentItem(),"workspaceStatusBar");QVERIFY(status);QCOMPARE(status->property("toolName").toString(),QString("自由变换"));QVERIFY(!window->property("liquifying").toBool());QVERIFY(window->property("editing").toBool());
        QVERIFY(QMetaObject::invokeMethod(window,"requestAction",Q_ARG(QVariant,QVariant("new"))));QCOMPARE(window->property("fileNotice").toString(),QString("请先确定或取消自由变换"));
        QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive() && !client.transformBusy());QVERIFY(panel->isVisible());QCOMPARE(window->property("fileNotice").toString(),QString());
        QVERIFY(client.beginLiquify());QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(!panel->isVisible());QVERIFY(client.finishLiquify(false));QTRY_VERIFY(!client.liquifyActive() && !client.liquifyBusy());QTRY_VERIFY(panel->isVisible());
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void finishingEditRestoresCanvasShortcutsWithoutAClick_data() {QTest::addColumn<bool>("tablet");QTest::newRow("mouse")<<false;QTest::newRow("pen")<<true;}
    void finishingEditRestoresCanvasShortcutsWithoutAClick() {
        QFETCH(bool,tablet);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("finish-focus.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);
        window->resize(1800,900);window->requestActivate();QVERIFY(QTest::qWaitForWindowActive(window));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(64,64);
        QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushRadius(4);QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_VERIFY(!client.drawing() && !client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);
        for(bool liquify:{false,true})for(bool commit:{false,true}) {
            QVERIFY(liquify?client.beginLiquify():client.beginTransform());
            QTRY_VERIFY(liquify?!client.liquifyBusy():!client.transformBusy());
            const auto prefix=liquify?QString("liquify"):QString("transform");
            auto *bar=findVisualItem(window->contentItem(),prefix+"OptionsBar");QVERIFY(bar);
            auto *button=findVisualItem(bar,prefix+(commit?"Commit":"Cancel"));QVERIFY(button);
            button->forceActiveFocus();QTRY_VERIFY(button->hasActiveFocus());QVERIFY(!canvas->hasActiveFocus());
            // Finish via the toolbar; no canvas focus helper or canvas click follows.
            const auto position=button->mapToScene({button->width()/2,button->height()/2}).toPoint();
            QVERIFY(QRect(QPoint(0,0),window->size()).contains(position));
            if(tablet) {
                auto *pen=new QPointingDevice("edit finish pen",9183,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);pen->setParent(qApp);QWindowSystemInterface::registerInputDevice(pen);
                for(bool press:{true,false}){QWindowSystemInterface::handleTabletEvent(window,pen,QHighDpi::toNativeLocalPosition(QPointF(position),window),QHighDpi::toNativeGlobalPosition(QPointF(window->mapToGlobal(position)),window),press?Qt::LeftButton:Qt::NoButton,press?.5:0.,0,0,0,0,0);QWindowSystemInterface::flushWindowSystemEvents();}
            } else QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,position);
            QTRY_VERIFY(!client.liquifyActive() && !client.transformActive());
            QTRY_VERIFY(canvas->hasActiveFocus());
            QTest::keyClick(window,Qt::Key_E);QTRY_VERIFY(client.eraser());
            QTest::keyClick(window,Qt::Key_B);QTRY_VERIFY(!client.eraser());
            const auto radius=client.brushRadius();QTest::keyClick(window,Qt::Key_BracketRight);QTRY_VERIFY(client.brushRadius()>radius);
        }
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);
        QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void degenerateLassoIsANoopAndSelfIntersectionStillWorks() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("tiny-lasso.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(client.selectAll());QTRY_VERIFY(!client.layerEditBusy());
        const auto before=client.selectionSteps();const auto depth=client.undoDepth();const auto revision=client.revision();
        QSignalSpy errors(&client,&PaintCoreClient::errorChanged);
        for(bool polygonal:{false,true})for(const auto &path:QList<QVector<QPointF>>{
            {{4,4},{4,4}},{{4,4},{4,4},{4,4}},{{4,4},{4.01,4.01},{4.02,4.02}}}) {
            QVERIFY(!client.editSelectionPath(path,0,polygonal));
            QCOMPARE(client.selectionSteps(),before);QCOMPARE(client.undoDepth(),depth);QCOMPARE(client.revision(),revision);
        }
        QVERIFY(client.editSelectionPath({{8,8},{32,32},{8,32},{32,8}},0,false));
        QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);
        QCOMPARE(errors.size(),0);QVERIFY(client.lastError().isEmpty());client.shutdown();
    }
    void invalidIntermediateTransformKeepsLastGeometryAndRecovers() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("invalid-transform.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformPoints().size()==16 && !client.transformBusy());
        const auto initial=client.transformPoints();QSignalSpy errors(&client,&PaintCoreClient::errorChanged);
        auto invalid=initial;invalid[3]=initial[0];
        QVERIFY(!client.updateTransform(invalid,false,2));QCOMPARE(client.transformPoints(),initial);QCOMPARE(errors.size(),0);
        invalid=initial;invalid[0]=initial[15];
        QVERIFY(!client.updateTransform(invalid,false,2));QCOMPARE(client.transformPoints(),initial);QCOMPARE(errors.size(),0);
        auto moved=initial;for(auto &value:moved)value=value.toPointF()+QPointF(3,4);
        QVERIFY(client.updateTransform(moved,false,2));QTRY_COMPARE(client.transformPoints(),moved);
        QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive() && !client.transformBusy());
        QCOMPARE(errors.size(),0);QVERIFY(client.lastError().isEmpty());client.shutdown();
    }
    void emptyTransformAndCancelPublishClosedState() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("cancel.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);
        engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        auto *overlay=findVisualItem(window->contentItem(),"transformOverlay");QVERIFY(overlay);
        auto *menu=overlay->findChild<QObject*>("transformContextMenu");QVERIFY(menu);
        auto *cancel=menu->findChild<QQuickItem*>(QString::fromUtf8("glassMenuItem:取消"));QVERIFY(cancel);
        QVERIFY(cancel->implicitHeight()<=32);QCOMPARE(menu->property("padding").toInt(),6);
        QSignalSpy states(&client,&PaintCoreClient::stateChanged);
        QVERIFY(client.beginTransform());QVERIFY(overlay->isVisible());
        states.clear();QTRY_VERIFY(!client.transformActive() && !client.transformBusy());
        QVERIFY(!states.isEmpty());QVERIFY(!overlay->isVisible());
        client.activateBrush();client.setBrushRadius(4);QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();
        QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.drawing() && !client.layerEditBusy());const auto depth=client.undoDepth();
        for(bool escape:{false,true}) {
            QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformPoints().size()==16 && !client.transformBusy());
            QVERIFY(overlay->isVisible());states.clear();
            if(escape){window->requestActivate();QVERIFY(QTest::qWaitForWindowActive(window));QTest::keyClick(window,Qt::Key_Escape);}
            else QVERIFY(QMetaObject::invokeMethod(cancel,"triggered"));
            QTRY_VERIFY(!client.transformActive() && !client.transformBusy());
            QVERIFY(!states.isEmpty());QVERIFY(!overlay->isVisible());QCOMPARE(client.undoDepth(),depth);
        }
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);
        QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void transformResidentPreviewFollowsAndCompositesTransparentStack_data() {
        QTest::addColumn<bool>("subtrees");QTest::addColumn<int>("compound");QTest::newRow("flat")<<false<<0;QTest::newRow("unrelated-groups-and-masks")<<true<<0;QTest::newRow("linked-owner-mask")<<true<<1;QTest::newRow("complete-masked-group")<<true<<2;QTest::newRow("continuous-multiple")<<true<<3;QTest::newRow("nested-normal-child")<<true<<4;QTest::newRow("nested-pass-through-child")<<true<<5;
    }
    void transformResidentPreviewFollowsAndCompositesTransparentStack() {
        QFETCH(bool,subtrees);QFETCH(int,compound);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("transform-resident.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());QTRY_VERIFY(client.brushLibraryReady());
        client.newTransparentDocument(96,96);QTRY_COMPARE(client.documentWidth(),96);QTRY_VERIFY(!client.layerEditBusy());
        client.activateBucket();client.setBrushColor(Qt::blue);QVERIFY(client.bucketFill({0,0}));QTRY_VERIFY(!client.layerEditBusy());client.setLayerProperties(client.activeLayer(),true,.5);QTRY_VERIFY(!client.layerEditBusy());const auto bottom=client.activeLayer();
        client.addLayer("moving");QTRY_VERIFY(!client.layerEditBusy());auto moving=client.activeLayer();client.activateBrush();client.setBrushRadius(6);client.setBrushColor(Qt::red);QVERIFY(client.beginStroke({{32.5,32.5},1}));client.endStroke();QTRY_VERIFY(!client.drawing() && !client.layerEditBusy());
        client.addLayer("above");QTRY_VERIFY(!client.layerEditBusy());client.activateBucket();client.setBrushColor(Qt::green);QVERIFY(client.bucketFill({0,0}));QTRY_VERIFY(!client.layerEditBusy());client.setLayerProperties(client.activeLayer(),true,.25);QTRY_VERIFY(!client.layerEditBusy());const auto above=client.activeLayer();
        if(subtrees) {
            client.selectLayer(bottom);QTRY_COMPARE(client.activeLayer(),bottom);QTRY_VERIFY(!client.layerEditBusy());
            QVERIFY(client.groupSelectedLayers("below group"));QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.addMask(client.activeLayer()));QTRY_VERIFY(!client.layerEditBusy());
            client.selectLayer(above);QTRY_COMPARE(client.activeLayer(),above);QTRY_VERIFY(!client.layerEditBusy());
            QVERIFY(client.addMask(above));QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(above);QTRY_COMPARE(client.activeLayer(),above);QTRY_VERIFY(!client.layerEditBusy());
            QVERIFY(client.groupSelectedLayers("above group"));QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.addMask(client.activeLayer()));QTRY_VERIFY(!client.layerEditBusy());
        }
        client.selectLayer(moving);QTRY_COMPARE(client.activeLayer(),moving);QTRY_VERIFY(!client.layerEditBusy());
        if(compound==1 || compound==2 || compound>=4){QVERIFY(client.addMask(moving));QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(moving);QTRY_COMPARE(client.activeLayer(),moving);QTRY_VERIFY(!client.layerEditBusy());}
        if(compound==2){QVERIFY(client.groupSelectedLayers("target group"));QTRY_VERIFY(!client.layerEditBusy());moving=client.activeLayer();QVERIFY(client.addMask(moving));QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(moving);QTRY_COMPARE(client.activeLayer(),moving);QTRY_VERIFY(!client.layerEditBusy());}
        if(compound>=4){
            QVERIFY(client.groupLayer(moving,"inner context"));QTRY_VERIFY(!client.layerEditBusy());const auto inner=client.activeLayer();QVERIFY(client.setLayerBlend(inner,compound==4?0:27));QTRY_VERIFY(!client.layerEditBusy());
            client.selectLayer(moving);QTRY_COMPARE(client.activeLayer(),moving);QTRY_VERIFY(!client.layerEditBusy());client.addLayer("stationary sibling");QTRY_VERIFY(!client.layerEditBusy());client.activateBrush();client.setBrushRadius(5);client.setBrushColor(Qt::cyan);QVERIFY(client.beginStroke({{65,65},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());
            QVERIFY(client.groupLayer(inner,"outer context"));QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.setLayerBlend(client.activeLayer(),compound==4?0:27));QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(moving);QTRY_COMPARE(client.activeLayer(),moving);QTRY_VERIFY(!client.layerEditBusy());
        }
        quint64 partner=0;
        if(compound==3){client.addLayer("partner");QTRY_VERIFY(!client.layerEditBusy());partner=client.activeLayer();client.activateBrush();client.setBrushRadius(5);client.setBrushColor(Qt::cyan);QVERIFY(client.beginStroke({{65,65},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(moving);QTRY_COMPARE(client.activeLayer(),moving);client.selectLayers(partner,Qt::ControlModifier);QTRY_COMPARE(client.selectedLayers().size(),2);QTRY_VERIFY(!client.layerEditBusy());}
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();QVERIFY(QTest::qWaitForWindowExposed(window));QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto capture=[&]{if(auto *overlay=findVisualItem(window->contentItem(),"transformOverlay"))overlay->setVisible(false);auto grab=window->contentItem()->grabToImage();if(!grab)return QImage();QSignalSpy ready(grab.data(),&QQuickItemGrabResult::ready);if(!ready.wait(2000))return QImage();auto image=grab->image();if(image.isNull())return image;const auto dpr=double(image.width())/window->width();const auto origin=canvas->mapToScene({0,0})*dpr;auto result=image.copy(QRect(origin.toPoint(),QSize(qRound(canvas->width()*dpr),qRound(canvas->height()*dpr))));result.setDevicePixelRatio(dpr);return result;};
        const auto at=[&](const QImage &image,QPointF p){const auto v=(canvas->documentRect().topLeft()+p*canvas->zoom())*image.devicePixelRatio();return image.pixelColor(v.toPoint());};
        const auto close=[](QColor a,QColor b){return std::abs(a.red()-b.red())<8 && std::abs(a.green()-b.green())<8 && std::abs(a.blue()-b.blue())<8 && std::abs(a.alpha()-b.alpha())<8;};
        QImage before;QColor red;const auto initialReport=[&]{before.save("artifacts/transform-initial-window.png");client.frame().save("artifacts/transform-initial-frame.png");return QString("red=%1 frame=%2 frameSize=%3x%4 zoom=%5 doc=%6,%7 origin=%8,%9").arg(red.name()).arg(client.frame().pixelColor(32,32).name()).arg(client.frame().width()).arg(client.frame().height()).arg(canvas->zoom()).arg(canvas->documentRect().x()).arg(canvas->documentRect().y()).arg(canvas->mapToScene({0,0}).x()).arg(canvas->mapToScene({0,0}).y());};QTRY_VERIFY2(!(before=capture()).isNull() && (red=at(before,{32,32})).red()>150,qPrintable(initialReport()));const auto background=at(before,{8,8});QVERIFY(background.green()>60 && background.blue()>60);
        const auto sibling=at(before,{65,65});
        if(qgetenv("QT_QUICK_BACKEND")=="rhi")QTRY_VERIFY(canvas->gpuDisplay());
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());const auto initial=client.transformPoints();const auto depth=client.undoDepth();
        if(canvas->gpuDisplay()){QTRY_VERIFY(canvas->forwardPreview());QTRY_VERIFY(client.residentPreview());}
        const auto source=client.residentPreview();
        for(int i=1;i<=100;++i){auto points=initial;for(auto &value:points){auto p=value.toPointF();p.rx()+=16.*i/100.;value=p;}QVERIFY(client.updateTransform(points,false,2));}
        QImage moved;const auto report=[&]{moved.save("artifacts/transform-preview-moved.png");before.save("artifacts/transform-preview-before.png");return QString("forward=%1 expected=%2 actual=%3 alpha=%4 background=%5 / %6 revision=%7 / %8").arg(canvas->forwardPreview()).arg(red.name()).arg(at(moved,{48,32}).name()).arg(at(moved,{48,32}).alpha()).arg(background.name()).arg(at(moved,{8,8}).name()).arg(client.revision()).arg(client.residentPreview()?client.residentPreview()->revision:0);};QTRY_VERIFY2(!(moved=capture()).isNull() && close(at(moved,{48,32}),red),qPrintable(report()));
        QVERIFY(close(at(moved,{32,32}),background));QVERIFY(close(at(moved,{8,8}),background));QCOMPARE(client.undoDepth(),depth);
        if(compound>=4)QVERIFY(close(at(moved,{65,65}),sibling));
        if(source){QCOMPARE(client.residentPreview()->sourceId,source->sourceId);QCOMPARE(client.residentPreview()->images[0].cacheKey(),source->images[0].cacheKey());}
        QVERIFY(client.checkpointTransform());QTRY_COMPARE(client.undoDepth(),depth+1);client.undo();QTRY_VERIFY(!client.transformBusy());QImage restored;QTRY_VERIFY(!(restored=capture()).isNull() && close(at(restored,{32,32}),red));
        auto warp=initial;warp[5]=warp[5].toPointF()+QPointF(4,3);QVERIFY(client.updateTransform(warp,true,2));QVERIFY(client.checkpointTransform());QTRY_VERIFY(!client.transformBusy());
        QImage warped;QTRY_VERIFY(!(warped=capture()).isNull() && warped!=restored);
        QVERIFY(client.finishTransform(true));QTRY_VERIFY(!client.transformActive());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_VERIFY(!client.residentPreview());
        const auto finalDepth=client.undoDepth();client.undo();QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),finalDepth-1);
        QVERIFY(client.beginTransform());QTRY_VERIFY(!client.transformBusy());QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive());
        QCOMPARE(warnings,QStringList());QVERIFY(client.lastError().isEmpty());auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void foldedWarpPreviewMatchesCommittedPixels() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("fold.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini")); QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);
        engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml")); QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newTransparentDocument(96,96); QTRY_COMPARE(client.documentWidth(),96); QTRY_VERIFY(!client.layerEditBusy());
        client.activateBucket();client.setBrushColor(Qt::white);QVERIFY(client.bucketFill({0,0}));QTRY_VERIFY(!client.layerEditBusy());
        const auto depth=client.undoDepth();client.activateBrush();client.setBrushRadius(22);client.setBrushColor(Qt::black);
        QVERIFY(client.beginStroke({{48,48},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+1);
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();
        QVERIFY(QTest::qWaitForWindowExposed(window));QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto capture=[&]{
            canvas->actualSize();
            if(auto *overlay=findVisualItem(window->contentItem(),"transformOverlay"))overlay->setVisible(false);
            auto grab=window->contentItem()->grabToImage();if(!grab)return QImage();QSignalSpy ready(grab.data(),&QQuickItemGrabResult::ready);if(grab->image().isNull() && !ready.wait(2000))return QImage();auto image=grab->image();if(image.isNull())return image;
            const auto scale=double(image.width())/window->width();
            const auto origin=(canvas->mapToScene({0,0})+canvas->documentRect().topLeft())*scale;
            return image.copy(QRect(origin.toPoint(),QSize(qRound(96*canvas->zoom()*scale),qRound(96*canvas->zoom()*scale))));
        };
        for(double opacity:{1.,.5}) {
            client.setLayerProperties(client.activeLayer(),true,opacity);QTRY_VERIFY(!client.layerEditBusy());
            QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());
            if(canvas->gpuDisplay())QTRY_VERIFY(canvas->forwardPreview());
            QVariantList points;
            for(auto p:QList<QPointF>{{35,6},{78,15},{85,5},{103,9},{35,46},{54,21},{96,39},{126,35},{15,43},{59,68},{89,69},{66,39},{16,71},{39,97},{82,99},{125,68}})points.append(QPointF(p.x()*.75,p.y()));
            QVERIFY(client.updateTransform(points,true,1));QTRY_VERIFY(!client.transformBusy());
            if(canvas->gpuDisplay())QTRY_VERIFY(client.residentPreview() && client.residentPreview()->revision==client.revision());
            else QTRY_VERIFY(client.frameRevision()>=client.revision());
            QTest::qWait(60);const auto preview=capture();QVERIFY(!preview.isNull());
            QVERIFY(client.finishTransform(true));QTRY_VERIFY(!client.transformActive());
            QTRY_VERIFY(client.frameRevision()>=client.revision() && !client.residentPreview());
            QImage final;int different=0;
            const auto matches=[&]{
                final=capture();if(final.size()!=preview.size())return false;different=0;
                const auto near=[](const QImage &a,const QImage &b,int x,int y){
                    const auto c=a.pixelColor(x,y);
                    for(int yy=std::max(0,y-1);yy<=std::min(b.height()-1,y+1);++yy)
                        for(int xx=std::max(0,x-1);xx<=std::min(b.width()-1,x+1);++xx){
                            const auto d=b.pixelColor(xx,yy);
                            if(std::max({std::abs(c.red()-d.red()),std::abs(c.green()-d.green()),std::abs(c.blue()-d.blue())})<=16)return true;
                        }
                    return false;
                };
                // Display resampling at fractional DPR can shift coverage by one physical pixel.
                for(int y=0;y<final.height();++y)for(int x=0;x<final.width();++x)
                    if(!near(preview,final,x,y) || !near(final,preview,x,y))++different;
                return different<double(final.width()*final.height())*.02;
            };
            const auto report=[&]{preview.save("artifacts/transform-fold-preview.png");final.save("artifacts/transform-fold-final.png");return QString("opacity=%1 mismatched=%2 / %3").arg(opacity).arg(different).arg(final.width()*final.height());};
            QTRY_VERIFY2(matches(),qPrintable(report()));
            preview.save(QString("artifacts/transform-fold-%1-preview.png").arg(opacity));final.save(QString("artifacts/transform-fold-%1-final.png").arg(opacity));
            qInfo()<<"fold opacity"<<opacity<<"different pixels"<<different<<"/"<<final.width()*final.height();
            client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());
        }
        QVERIFY(client.lastError().isEmpty());auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);
        QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }

};
int main(int argc,char **argv){
    QGuiApplication app(argc,argv);app.setOrganizationName("DrawVerseTests");app.setApplicationName("TransformTests");
    initializeDrawVerseUiResources();QQuickStyle::setStyle("Basic");
    qmlRegisterType<CanvasItem>("DrawVerse",1,0,"PaintCanvas");qmlRegisterType<ColorWheelItem>("DrawVerse",1,0,"ColorWheel");
    qmlRegisterType<SelectionOverlay>("DrawVerse",1,0,"SelectionOutline");qmlRegisterType<MenuSurface>("DrawVerse",1,0,"MenuSurface");qmlRegisterType<DocumentManager>("DrawVerse",1,0,"DocumentManager");
    TransformTests test;return QTest::qExec(&test,argc,argv);
}
#include "transform_tests.moc"
