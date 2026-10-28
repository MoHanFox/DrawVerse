#include "brushes/BrushLibrary.h"
#include "engine/PaintCoreClient.h"
#include "documents/DocumentManager.h"
#include "platform/InteractionTrace.h"
#include "platform/UiResources.h"
#include "platform/MenuSurface.h"
#include "canvas/CanvasItem.h"
#include "canvas/SelectionOverlay.h"
#include "color/ColorWheelItem.h"
#include "workspace/WorkspaceManager.h"
#include <QtTest>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QJsonDocument>
#include <QSettings>
#include <QTemporaryDir>

namespace {
QJsonObject object(const QByteArray &bytes){return QJsonDocument::fromJson(bytes).object();}
QJsonObject colors(const QColor &front,const QColor &back){return {{"foreground",front.name(QColor::HexArgb)},{"background",back.name(QColor::HexArgb)}};}
QJsonObject saved(const QString &path){QSettings settings(path,QSettings::IniFormat);settings.sync();return object(settings.value("brushes/state").toByteArray());}
int count(const QJsonArray &events,const QString &name){int result=0;for(const auto &value:events)if(value.toObject().value("name").toString()==name)++result;return result;}
QQuickItem *visual(QQuickItem *item,const QString &name){if(item->objectName()==name && item->isVisible())return item;for(auto *child:item->childItems())if(auto *found=visual(child,name))return found;return nullptr;}
}
class ColorStateTests : public QObject {
    Q_OBJECT
private slots:
    void recentColorCommitRetainsValidatedCaches(){
        for(int mode=0;mode<3;++mode){
            BrushLibrary source,receiver;source.setToolIsolation(mode);source.snapshot();source.catalogSnapshot();
            QVERIFY(receiver.restore(source.snapshot()));receiver.snapshot();
            InteractionTrace::start(true);QVERIFY(source.useColor("#cc123456"));
            const auto bytes=source.snapshot();const auto catalog=source.catalogSnapshot();
            const auto trace=InteractionTrace::finish();
            qInfo()<<"recent color snapshot rebuilds"<<count(trace,"snapshotObject.rebuild");
            QCOMPARE(count(trace,"snapshotObject.rebuild"),0);
            QSignalSpy rebuilds(&receiver,&BrushLibrary::presetsChanged);QSignalSpy recent(&receiver,&BrushLibrary::recentColorsChanged);
            QVERIFY(receiver.restoreSharedSettings(source));QCOMPARE(receiver.recentColors(),source.recentColors());
            QCOMPARE(rebuilds.size(),0);QCOMPARE(recent.size(),1);
            BrushLibrary cold;QVERIFY(cold.restore(bytes));QCOMPARE(cold.snapshot(),bytes);QCOMPARE(cold.catalogSnapshot(),catalog);
            source.flushPersistence();QVERIFY(!source.useColor("#cc123456"));
            const auto prior=receiver.snapshot();auto invalid=object(prior);invalid["recentColors"]=QJsonArray{"invalid color"};
            QVERIFY(!receiver.restore(QJsonDocument(invalid).toJson()));QCOMPARE(receiver.snapshot(),prior);
        }
    }
    void scalarCacheMatchesColdConstructionAndSharedProjection() {
        for(int mode=0;mode<3;++mode)for(int sender=0;sender<3;++sender)for(int tool=0;tool<3;++tool) {
            BrushLibrary source,receiver,reference;source.setToolIsolation(mode);
            for(int t=0;t<3;++t){source.setToolContext(t);source.setRadius(11+t);source.setFlow(.3+t*.1);source.setOpacity(.6+t*.1);}
            source.setToolContext(sender);source.flushPersistence();
            QVERIFY(receiver.restore(source.snapshot()));receiver.setToolContext(tool,false);receiver.snapshot();
            QSignalSpy rebuilds(&receiver,&BrushLibrary::presetsChanged),feedback(&receiver,&BrushLibrary::persistRequested);
            source.catalogSnapshot();InteractionTrace::start(true);
            source.setRadius(36);source.setFlow(.27);source.setOpacity(.42);source.setActiveColors(Qt::red,Qt::blue);
            const auto bytes=source.snapshot();
            const auto setterTrace=InteractionTrace::finish();QCOMPARE(count(setterTrace,"snapshotObject.rebuild"),0);
            BrushLibrary cold;QVERIFY(cold.restore(bytes));QCOMPARE(cold.snapshot(),bytes);QCOMPARE(cold.catalogSnapshot(),source.catalogSnapshot());
            InteractionTrace::start(true);QVERIFY(receiver.restoreSharedSettings(source));
            QCOMPARE(count(InteractionTrace::finish(),"preferences.restore.full"),0);
            QVERIFY(reference.restore(bytes));reference.setToolContext(tool,false);
            QCOMPARE(receiver.snapshot(),reference.snapshot());QCOMPARE(receiver.catalogSnapshot(),reference.catalogSnapshot());
            QCOMPARE(receiver.toolContext(),tool);QCOMPARE(rebuilds.size(),0);QCOMPARE(feedback.size(),0);
            for(int t=0;t<3;++t){receiver.setToolContext(t,false);reference.setToolContext(t,false);QCOMPARE(receiver.radius(),reference.radius());QCOMPARE(receiver.flow(),reference.flow());QCOMPARE(receiver.opacity(),reference.opacity());}
            receiver.setToolContext(tool,false);receiver.snapshot();
            const auto before=receiver.snapshot();source.setHardness(.37);QVERIFY(!receiver.restoreSharedSettings(source));QCOMPARE(receiver.snapshot(),before);
            auto invalid=object(before);auto rows=invalid.value("presets").toArray();auto row=rows.first().toObject();row["flow"]=-1;rows[0]=row;invalid["presets"]=rows;
            QVERIFY(!receiver.restore(QJsonDocument(invalid).toJson()));QCOMPARE(receiver.snapshot(),before);
        }
    }
    void catalogReusesCurrentPreferenceRowsWithoutChangingState() {
        for(int mode=0;mode<=2;++mode){
            BrushLibrary library;library.setToolIsolation(mode);
            // A catalog-first query must retain the original cold exact path.
            InteractionTrace::start(true);library.catalogSnapshot();
            QCOMPARE(count(InteractionTrace::finish(),"catalogSnapshot.presetRows"),1);
            for(int step=0;step<7;++step){
                library.setRadius(21+step);
                if(step==1){library.setFlow(.43);library.setOpacity(.61);library.setActiveColors(Qt::red,Qt::blue);}
                if(step==2){const auto copy=library.saveCopy("parameter reuse");QVERIFY(!copy.isEmpty());const auto group=library.createGroup("reuse group");QVERIFY(library.movePresetToGroup(copy,group));}
                if(step==3){library.setPressureOpacity(true);library.setAngleControl("direction");}
                if(step==4){library.setToolContext(1);library.setRadius(48);library.setPressureOpacity(false);}
                if(step==5){library.setToolContext(0);QVERIFY(library.select("round-fine"));library.setRadius(9);}
                if(step==6){library.setToolIsolation(0);library.setToolContext(2);library.setRadius(17);}
                const auto preferences=library.snapshot();const auto root=object(preferences);
                QJsonObject projection{{"version",root["version"]},{"selected",root["selected"]},{"presets",root["presets"]}};
                if(root.contains("organization"))projection["organization"]=root["organization"];
                InteractionTrace::start(true);const auto actual=library.catalogSnapshot();
                const auto trace=InteractionTrace::finish();
                QCOMPARE(object(actual),projection);
                // Independent reference has no preference-object cache: it must
                // execute the old full row construction on the same active state.
                BrushLibrary reference;QVERIFY(reference.restore(preferences));
                InteractionTrace::start(true);const auto exact=reference.catalogSnapshot();
                QCOMPARE(count(InteractionTrace::finish(),"catalogSnapshot.presetRows"),1);
                QCOMPARE(actual,exact);
                QCOMPARE(library.snapshot(),preferences); // Query must not lose dirty preferences.
                QCOMPARE(count(trace,"catalogSnapshot.presetRows"),0);
                QCOMPARE(count(trace,"catalogSnapshot.preference_rows_reused")+count(trace,"catalogSnapshot.object_reuse"),1);
            }
        }
    }
    void qmlSwatchesUpdateImmediately() {
        QTemporaryDir directory;PaintCoreClient client(nullptr,directory.filePath("preferences.ini"));WorkspaceManager workspace(directory.filePath("workspace.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        QQmlApplicationEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);window->showNormal();window->resize(1280,800);window->requestActivate();
        QVERIFY(QTest::qWaitForWindowExposed(window));QVERIFY(QTest::qWaitForWindowActive(window));
        auto *front=visual(window->contentItem(),"foregroundColorSwatch"),*back=visual(window->contentItem(),"secondaryColorSwatch");QVERIFY(front && back);
        auto *swatchWindow=front->window();QVERIFY(swatchWindow);
        QTRY_VERIFY(QRectF(0,0,swatchWindow->width(),swatchWindow->height()).contains(front->mapToScene({12,12})));
        client.brushLibrary()->snapshot();client.brushLibrary()->catalogSnapshot();
        client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);
        // No QTRY/event-loop wait: these real QML color bindings must already
        // reflect the edits while the persistence debounce is still pending.
        QCOMPARE(front->property("color").value<QColor>(),QColor(Qt::red));QCOMPARE(back->property("color").value<QColor>(),QColor(Qt::blue));
        QVERIFY(client.brushLibrary()->persistencePending());
        QTest::mouseClick(swatchWindow,Qt::LeftButton,Qt::NoModifier,front->mapToScene({12,12}).toPoint());
        QCOMPARE(front->property("color").value<QColor>(),QColor(Qt::blue));QCOMPARE(back->property("color").value<QColor>(),QColor(Qt::red));
        if(auto *canvas=window->findChild<CanvasItem*>("mainCanvas"))canvas->setClient(nullptr);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void validCacheRetainsPresetsAndCatalog() {
        BrushLibrary library;library.flushPersistence();
        const auto initial=object(library.snapshot());const auto catalog=library.catalogSnapshot();
        QSignalSpy changed(&library,&BrushLibrary::activeColorsChanged);
        library.setActiveColors(library.foregroundColor(),library.backgroundColor());
        QCOMPARE(changed.size(),0);QVERIFY(!library.persistencePending());QCOMPARE(object(library.snapshot()),initial);
        InteractionTrace::start(true);
        for(const auto &pair:QList<QPair<QColor,QColor>>{{Qt::red,Qt::white},{Qt::red,Qt::blue},{Qt::blue,Qt::red}}){
            library.setActiveColors(pair.first,pair.second);
            auto expected=initial;expected["colorState"]=colors(pair.first,pair.second);
            QCOMPARE(object(library.snapshot()),expected);QCOMPARE(library.catalogSnapshot(),catalog);
        }
        const auto trace=InteractionTrace::finish();QCOMPARE(changed.size(),3);
        QCOMPARE(count(trace,"snapshotObject.rebuild"),0);QCOMPARE(count(trace,"snapshotObject.reuse"),3);
        QCOMPARE(count(trace,"catalogSnapshot.rebuild"),0);QCOMPARE(count(trace,"catalogSnapshot.bytes_reuse"),3);
        BrushLibrary restored;QVERIFY(restored.restore(library.snapshot()));QCOMPARE(object(restored.snapshot()),object(library.snapshot()));
    }
    void mixedDirtyStatesRoundTrip() {
        for(bool warm:{false,true})for(bool colorFirst:{false,true}){
            BrushLibrary library;library.setToolIsolation(2);
            if(warm){library.snapshot();library.catalogSnapshot();}
            if(colorFirst)library.setActiveColors(Qt::red,Qt::blue);
            library.setRadius(17);library.setFlow(.43);library.setOpacity(.61);
            const auto copy=library.saveCopy("color interleave");QVERIFY(!copy.isEmpty());
            library.setRadius(23);QVERIFY(library.select("round-fine"));library.setFlow(.27);
            library.setToolContext(1);library.setRadius(9);
            if(!colorFirst)library.setActiveColors(Qt::red,Qt::blue);
            const auto snapshot=library.snapshot();const auto catalog=library.catalogSnapshot();
            BrushLibrary restored;QVERIFY(restored.restore(snapshot));
            QCOMPARE(object(restored.snapshot()),object(snapshot));QCOMPARE(object(restored.catalogSnapshot()),object(catalog));
            QCOMPARE(object(snapshot).value("colorState").toObject(),colors(Qt::red,Qt::blue));
            restored.setToolContext(0);QCOMPARE(restored.selectedId(),QString("round-fine"));QCOMPARE(restored.flow(),.27);
            QVERIFY(restored.select(copy));QCOMPARE(restored.radius(),23.);QCOMPARE(restored.opacity(),.61);
            QVERIFY(restored.select("round-pressure"));QCOMPARE(restored.radius(),17.);QCOMPARE(restored.flow(),.43);
            restored.setToolContext(1);QCOMPARE(restored.radius(),9.);
        }
    }
    void beforeDebouncePixelsSaveAndReopen() {
        QTemporaryDir directory;QVERIFY(directory.isValid());const auto path=directory.filePath("preferences.ini");
        {
            PaintCoreClient client(nullptr,path);QTRY_VERIFY_WITH_TIMEOUT(client.ready() && client.brushLibraryReady() && client.brushCatalogPrepared(),15000);
            auto *library=client.brushLibrary();client.newDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());
            library->setPressureSize(false);library->setPressureFlow(false);library->setPressureOpacity(false);library->setRadius(8);
            library->setFlow(1);library->setOpacity(1);library->flushPersistence();
            library->snapshot();library->catalogSnapshot();
            client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);client.swapBrushColors();
            QCOMPARE(client.brushColor(),QColor(Qt::blue));QVERIFY(library->persistencePending());
            // No event-loop wait between color edit and stroke start.
            QVERIFY(client.beginStroke({{32,64},1}));client.endStroke();
            QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
            QTRY_COMPARE(client.sampleDocumentPixel(32,64),QColor(Qt::blue));
            client.setBrushColor(Qt::red);QVERIFY(library->persistencePending());
            QVERIFY(client.beginStroke({{96,64},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),2);
            QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(client.sampleDocumentPixel(96,64),QColor(Qt::red));
            library->setRadius(13);client.setBrushColor(Qt::green);const auto copy=library->saveCopy("persist interleave");QVERIFY(!copy.isEmpty());
            library->setFlow(.41);client.setSecondaryBrushColor(Qt::yellow);
            QTRY_VERIFY_WITH_TIMEOUT(saved(path).value("colorState").toObject()==colors(Qt::green,Qt::yellow) && saved(path).value("selected").toString()==copy && !library->persistencePending(),15000);
            QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        }
        PaintCoreClient reopened(nullptr,path);QTRY_VERIFY_WITH_TIMEOUT(reopened.ready() && reopened.brushLibraryReady() && reopened.brushCatalogPrepared(),15000);
        QCOMPARE(reopened.brushColor(),QColor(Qt::green));QCOMPARE(reopened.secondaryBrushColor(),QColor(Qt::yellow));
        QCOMPARE(reopened.brushLibrary()->radius(),13.);QCOMPARE(reopened.brushLibrary()->flow(),.41);
        QCOMPARE(saved(path).value("colorState").toObject(),colors(Qt::green,Qt::yellow));
        QSignalSpy stopped(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void documentBroadcastRetainsIsolatedToolsAndSingleWriter_data() {
        QTest::addColumn<bool>("adoptInitial");QTest::newRow("normal-hidden-service")<<false;QTest::newRow("adopted-initial")<<true;
    }
    void documentBroadcastRetainsIsolatedToolsAndSingleWriter() {
        QFETCH(bool,adoptInitial);
        QTemporaryDir directory;const auto path=directory.filePath("shared.ini");QObject root;
        PaintCoreClient first(nullptr,path);DocumentManager documents;documents.configure(&first,&root,adoptInitial);
        QTRY_VERIFY(first.ready() && first.brushLibraryReady() && first.brushCatalogPrepared());
        auto *library=first.brushLibrary();library->setToolIsolation(2);library->setRadius(3);
        first.activateEraser();library->setRadius(7);first.activateBrush();library->flushPersistence();
        const auto id=documents.newDocument(64,64,true);QVERIFY(!id.isEmpty());auto *second=documents.client(id);QVERIFY(second);
        QTRY_VERIFY(second->ready() && second->brushLibraryReady());second->activateEraser();
        QTRY_COMPARE(second->brushRadius(),7.);library->snapshot();library->catalogSnapshot();second->brushLibrary()->snapshot();
        QSignalSpy firstRebuilds(library,&BrushLibrary::presetsChanged),secondRebuilds(second->brushLibrary(),&BrushLibrary::presetsChanged);
        second->setBrushColor(Qt::red);second->setSecondaryBrushColor(Qt::blue);
        QTRY_COMPARE(first.brushColor(),QColor(Qt::red));QTRY_COMPARE(first.secondaryBrushColor(),QColor(Qt::blue));
        QTRY_VERIFY_WITH_TIMEOUT(saved(path).value("colorState").toObject()==colors(Qt::red,Qt::blue),15000);
        QCOMPARE(first.brushRadius(),3.);QCOMPARE(second->brushRadius(),7.);QVERIFY(second->eraser());
        first.swapBrushColors();QTRY_COMPARE(second->brushColor(),QColor(Qt::blue));QTRY_COMPARE(second->secondaryBrushColor(),QColor(Qt::red));
        QTRY_VERIFY_WITH_TIMEOUT(saved(path).value("colorState").toObject()==colors(Qt::blue,Qt::red),15000);
        QTRY_VERIFY(!library->persistencePending() && !second->brushLibrary()->persistencePending());
        QCOMPARE(first.brushRadius(),3.);QCOMPARE(second->brushRadius(),7.);
        second->brushLibrary()->setRadius(9);second->brushLibrary()->setFlow(.31);second->brushLibrary()->setOpacity(.47);second->brushLibrary()->flushPersistence();
        first.activateEraser();QCOMPARE(first.brushRadius(),9.);QCOMPARE(library->flow(),.31);QCOMPARE(library->opacity(),.47);
        first.activateBrush();QCOMPARE(first.brushRadius(),3.);
        QCOMPARE(firstRebuilds.size(),0);QCOMPARE(secondRebuilds.size(),0);
        QSignalSpy stopped(&documents,&DocumentManager::stopped);documents.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
};
int main(int argc,char **argv) {
    QGuiApplication app(argc,argv);initializeDrawVerseUiResources();QQuickStyle::setStyle("Basic");
    qmlRegisterType<MenuSurface>("DrawVerse",1,0,"MenuSurface");qmlRegisterType<SelectionOverlay>("DrawVerse",1,0,"SelectionOutline");
    qmlRegisterType<DocumentManager>("DrawVerse",1,0,"DocumentManager");qmlRegisterType<CanvasItem>("DrawVerse",1,0,"PaintCanvas");qmlRegisterType<ColorWheelItem>("DrawVerse",1,0,"ColorWheel");
    ColorStateTests test;return QTest::qExec(&test,argc,argv);
}
#include "color_state_tests.moc"
