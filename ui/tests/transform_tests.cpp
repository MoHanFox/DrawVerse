#include "canvas/CanvasItem.h"
#include "canvas/SelectionOverlay.h"
#include "color/ColorWheelItem.h"
#include "documents/DocumentManager.h"
#include "workspace/WorkspaceManager.h"
#include "platform/MenuSurface.h"
#include <QtTest>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QQuickItemGrabResult>
#include <QTemporaryDir>
#include <QTimer>
#include <QSettings>
static QQuickItem *findVisualItem(QQuickItem *item,const QString &name){if(item->objectName()==name)return item;for(auto *child:item->childItems())if(auto *found=findVisualItem(child,name))return found;return nullptr;}
class TransformTests : public QObject {
    Q_OBJECT
private slots:
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
        QTest::addColumn<int>("kind");QTest::newRow("linked-owner-mask")<<0;QTest::newRow("masked-group")<<1;QTest::newRow("continuous-multiple")<<2;QTest::newRow("nested-normal-child")<<3;QTest::newRow("nested-pass-through-child")<<4;QTest::newRow("fixed-mask-fallback")<<5;QTest::newRow("ancestor-mask-fallback")<<6;
    }
    void compoundLayerMovesKeepResidentSources() {
        QFETCH(int,kind);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("compound-move.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(96,96);QTRY_COMPARE(client.documentWidth(),96);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushRadius(6);client.setBrushColor(Qt::red);client.brushLibrary()->setPressureSize(false);QVERIFY(client.beginStroke({{32.5,32.5},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);const auto owner=client.activeLayer();
        QVERIFY(client.addMask(owner));QTRY_VERIFY(!client.layerEditBusy());const auto ownerMask=client.activeLayer();client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());
        if(kind==1){QVERIFY(client.groupSelectedLayers("moving group"));QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();QVERIFY(client.addMask(group));QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(group);QTRY_COMPARE(client.activeLayer(),group);QTRY_VERIFY(!client.layerEditBusy());}
        if(kind==2){client.addLayer("partner");QTRY_VERIFY(!client.layerEditBusy());const auto partner=client.activeLayer();client.setBrushColor(Qt::blue);QVERIFY(client.beginStroke({{32.5,60.5},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);client.selectLayers(partner,Qt::ControlModifier);QTRY_COMPARE(client.selectedLayers().size(),2);QTRY_VERIFY(!client.layerEditBusy());}
        if(kind==3 || kind==4){quint64 target=owner;for(int i=0;i<2;++i){QVERIFY(client.groupLayer(target,"ancestor"));QTRY_VERIFY(!client.layerEditBusy());target=client.activeLayer();QVERIFY(client.setLayerBlend(target,kind==3?0:27));QTRY_VERIFY(!client.layerEditBusy());}client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());}
        if(kind==5){QVERIFY(client.setMaskLinked(ownerMask,false));QTRY_VERIFY(!client.layerEditBusy());}
        if(kind==6){QVERIFY(client.groupLayer(owner,"masked ancestor"));QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.addMask(client.activeLayer()));QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());}
        QQuickWindow window;window.resize(640,480);window.setColor(Qt::black);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({640,480});canvas->setClient(&client);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));window.requestActivate();QTRY_VERIFY(window.isActive());canvas->actualSize();canvas->forceActiveFocus();QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto screen=[&](QPointF p){return(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        const auto capture=[&]{auto grab=canvas->grabToImage();if(!grab || !QTest::qWaitFor([&]{return !grab->image().isNull();},5000))return QImage{};return grab->image();};
        const auto redAt=[&](const QImage &image,QPoint p){if(image.isNull())return false;const auto color=image.pixelColor(p*window.devicePixelRatio());return color.red()>180 && color.green()<70 && color.blue()<70;};
        QTRY_VERIFY(redAt(capture(),screen({32,32})));client.setMoveTool(true);const auto depth=client.undoDepth();
        QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,screen({32,32}));QTRY_VERIFY(client.layerMoveActive());
        if(qEnvironmentVariable("QT_QUICK_BACKEND")=="rhi")QVERIFY(canvas->gpuDisplay());
        if(canvas->gpuDisplay() && kind<5)QTRY_VERIFY(client.residentPreview());
        if(kind>=5)QVERIFY(!client.residentPreview());
        const auto source=client.residentPreview();
        for(int i=1;i<=32;++i)QTest::mouseMove(&window,screen({32+i*.5,32}));
        QImage moved;QTRY_VERIFY(!(moved=capture()).isNull() && redAt(moved,screen({48,32})));QVERIFY(!redAt(moved,screen({32,32})));QCOMPARE(client.undoDepth(),depth);
        if(source){QTRY_COMPARE(client.residentPreview()->revision,client.revision());QCOMPARE(client.residentPreview()->sourceId,source->sourceId);QCOMPARE(client.residentPreview()->images[0].cacheKey(),source->images[0].cacheKey());QCOMPARE(client.residentPreview()->vertices.size(),4);}
        QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,screen({48,32}));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(client.frameRevision()>=client.revision() && !client.residentPreview());QTRY_VERIFY(redAt(capture(),screen({48,32})));
        client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_VERIFY(redAt(capture(),screen({32,32})));QVERIFY(client.lastError().isEmpty());canvas->setClient(nullptr);client.shutdown();
    }
    void finishingEditRestoresCanvasShortcutsWithoutAClick() {
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
            QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,position);
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
        QTest::addColumn<bool>("subtrees");QTest::addColumn<int>("compound");QTest::newRow("flat")<<false<<0;QTest::newRow("unrelated-groups-and-masks")<<true<<0;QTest::newRow("linked-owner-mask")<<true<<1;QTest::newRow("complete-masked-group")<<true<<2;QTest::newRow("continuous-multiple")<<true<<3;
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
        if(compound==1 || compound==2){QVERIFY(client.addMask(moving));QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(moving);QTRY_COMPARE(client.activeLayer(),moving);QTRY_VERIFY(!client.layerEditBusy());}
        if(compound==2){QVERIFY(client.groupSelectedLayers("target group"));QTRY_VERIFY(!client.layerEditBusy());moving=client.activeLayer();QVERIFY(client.addMask(moving));QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(moving);QTRY_COMPARE(client.activeLayer(),moving);QTRY_VERIFY(!client.layerEditBusy());}
        quint64 partner=0;
        if(compound==3){client.addLayer("partner");QTRY_VERIFY(!client.layerEditBusy());partner=client.activeLayer();client.activateBrush();client.setBrushRadius(5);client.setBrushColor(Qt::cyan);QVERIFY(client.beginStroke({{65,65},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(moving);QTRY_COMPARE(client.activeLayer(),moving);client.selectLayers(partner,Qt::ControlModifier);QTRY_COMPARE(client.selectedLayers().size(),2);QTRY_VERIFY(!client.layerEditBusy());}
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();QVERIFY(QTest::qWaitForWindowExposed(window));QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto capture=[&]{if(auto *overlay=findVisualItem(window->contentItem(),"transformOverlay"))overlay->setVisible(false);auto grab=window->contentItem()->grabToImage();if(!grab)return QImage();QSignalSpy ready(grab.data(),&QQuickItemGrabResult::ready);if(grab->image().isNull() && !ready.wait(2000))return QImage();auto image=grab->image();if(image.isNull())return image;const auto dpr=double(image.width())/window->width();const auto origin=canvas->mapToScene({0,0})*dpr;auto result=image.copy(QRect(origin.toPoint(),QSize(qRound(canvas->width()*dpr),qRound(canvas->height()*dpr))));result.setDevicePixelRatio(dpr);return result;};
        const auto at=[&](const QImage &image,QPointF p){const auto v=(canvas->documentRect().topLeft()+p*canvas->zoom())*image.devicePixelRatio();return image.pixelColor(v.toPoint());};
        const auto close=[](QColor a,QColor b){return std::abs(a.red()-b.red())<8 && std::abs(a.green()-b.green())<8 && std::abs(a.blue()-b.blue())<8 && std::abs(a.alpha()-b.alpha())<8;};
        QImage before;QColor red;const auto initialReport=[&]{before.save("artifacts/transform-initial-window.png");client.frame().save("artifacts/transform-initial-frame.png");return QString("red=%1 frame=%2 frameSize=%3x%4 zoom=%5 doc=%6,%7 origin=%8,%9").arg(red.name()).arg(client.frame().pixelColor(32,32).name()).arg(client.frame().width()).arg(client.frame().height()).arg(canvas->zoom()).arg(canvas->documentRect().x()).arg(canvas->documentRect().y()).arg(canvas->mapToScene({0,0}).x()).arg(canvas->mapToScene({0,0}).y());};QTRY_VERIFY2(!(before=capture()).isNull() && (red=at(before,{32,32})).red()>150,qPrintable(initialReport()));const auto background=at(before,{8,8});QVERIFY(background.green()>60 && background.blue()>60);
        if(qgetenv("QT_QUICK_BACKEND")=="rhi")QTRY_VERIFY(canvas->gpuDisplay());
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());const auto initial=client.transformPoints();const auto depth=client.undoDepth();
        if(canvas->gpuDisplay()){QTRY_VERIFY(canvas->forwardPreview());QTRY_VERIFY(client.residentPreview());}
        const auto source=client.residentPreview();
        for(int i=1;i<=100;++i){auto points=initial;for(auto &value:points){auto p=value.toPointF();p.rx()+=16.*i/100.;value=p;}QVERIFY(client.updateTransform(points,false,2));}
        QImage moved;const auto report=[&]{moved.save("artifacts/transform-preview-moved.png");before.save("artifacts/transform-preview-before.png");return QString("forward=%1 expected=%2 actual=%3 alpha=%4 background=%5 / %6 revision=%7 / %8").arg(canvas->forwardPreview()).arg(red.name()).arg(at(moved,{48,32}).name()).arg(at(moved,{48,32}).alpha()).arg(background.name()).arg(at(moved,{8,8}).name()).arg(client.revision()).arg(client.residentPreview()?client.residentPreview()->revision:0);};QTRY_VERIFY2(!(moved=capture()).isNull() && close(at(moved,{48,32}),red),qPrintable(report()));
        QVERIFY(close(at(moved,{32,32}),background));QVERIFY(close(at(moved,{8,8}),background));QCOMPARE(client.undoDepth(),depth);
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
    Q_INIT_RESOURCE(ui_resources);QQuickStyle::setStyle("Basic");
    qmlRegisterType<CanvasItem>("DrawVerse",1,0,"PaintCanvas");qmlRegisterType<ColorWheelItem>("DrawVerse",1,0,"ColorWheel");
    qmlRegisterType<SelectionOverlay>("DrawVerse",1,0,"SelectionOutline");qmlRegisterType<MenuSurface>("DrawVerse",1,0,"MenuSurface");qmlRegisterType<DocumentManager>("DrawVerse",1,0,"DocumentManager");
    TransformTests test;return QTest::qExec(&test,argc,argv);
}
#include "transform_tests.moc"
