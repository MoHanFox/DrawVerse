#include "platform/UiResources.h"
#include "platform/MemoryBudget.h"
#include "engine/PaintCoreClient.h"
#include "canvas/CanvasItem.h"
#include <QScopeGuard>
#include <cmath>
#include <QtTest>
#include <QQmlEngine>
#include <QQmlContext>
#include <QQmlComponent>
#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QQuickItem>
#include <QQuickStyle>
#include <QTemporaryDir>
#include <QDir>
#include <QSettings>
#include <QProcess>
#include <QProcessEnvironment>

class AdjustmentTests : public QObject {
    Q_OBJECT
private slots:
    void initTestCase(){initializeDrawVerseUiResources();QQuickStyle::setStyle("Basic");}
    void hiddenLayerEditsAreSilent_data(){QTest::addColumn<bool>("ancestor");QTest::newRow("hidden-layer")<<false;QTest::newRow("hidden-parent")<<true;}
    void hiddenLayerEditsAreSilent(){
        QFETCH(bool,ancestor);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newDocument(64,64);QTRY_VERIFY(client.documentWidth()==64 && !client.layerEditBusy());
        const auto target=client.activeLayer();quint64 hidden=target;
        if(ancestor){QVERIFY(client.groupLayer(target,"hidden"));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());hidden=client.activeLayer();client.selectLayer(target);QTRY_COMPARE(client.activeLayer(),target);QTRY_VERIFY(!client.layerEditBusy());}
        auto revision=client.revision();client.setLayerProperties(hidden,false,1);QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
        revision=client.revision();const auto undo=client.undoDepth();QSignalSpy errors(&client,&PaintCoreClient::errorChanged);
        InputSample sample;sample.position={32,32};
        QVERIFY(!client.beginStroke(sample));client.activateEraser();QVERIFY(!client.beginStroke(sample));client.activateSmudge();QVERIFY(!client.beginStroke(sample));client.activateBrush();
        client.activateBucket();QVERIFY(!client.bucketFill({32,32}));client.activateBrush();QVERIFY(!client.beginLayerMove({32,32},false));QVERIFY(!client.beginLayerMove({32,32},true));QVERIFY(!client.beginTransform());QVERIFY(!client.beginLiquify());QVERIFY(!client.rasterizeImage());
        for(int kind=0;kind<3;++kind)QVERIFY(!client.beginAdjustment(kind));
        QVERIFY(client.selectAll());QTRY_VERIFY(client.selectionEnabled() && !client.layerEditBusy());revision=client.revision();const auto selectedUndo=client.undoDepth();
        client.deleteSelectionOrLayers();QVERIFY(!client.cutSelectedLayers());QTest::qWait(80);
        QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),selectedUndo);QVERIFY(!client.drawing() && !client.transformActive() && !client.liquifyActive() && !client.adjustmentActive());QCOMPARE(errors.size(),0);QVERIFY(client.lastError().isEmpty());
        client.clearSelection();QTRY_VERIFY(!client.selectionEnabled() && !client.layerEditBusy());
        client.setLayerProperties(hidden,true,1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.revision()>revision);
        QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_VERIFY(client.undoDepth()>undo);QTRY_VERIFY(!client.layerEditBusy());
    }
    void savedQualityReachesOtherDocuments(){
        QTemporaryDir temp;const auto path=temp.filePath("prefs.ini");PaintCoreClient first(nullptr,path);QTRY_VERIFY(first.ready());
        PaintCoreClient second(nullptr,path,first.brushCatalogConnection());QTRY_VERIFY(second.ready());
        QVERIFY(first.highestCanvasQuality() && second.highestCanvasQuality());
        const auto values=first.renderSettings();QVERIFY(first.saveRenderSettings(values["memoryMiB"].toInt(),values["gpuMiB"].toInt(),false));QVERIFY(!first.highestCanvasQuality());QTRY_VERIFY(!second.highestCanvasQuality());
        QVERIFY(first.saveRenderSettings(values["memoryMiB"].toInt(),values["gpuMiB"].toInt(),true));QVERIFY(first.highestCanvasQuality());QTRY_VERIFY(second.highestCanvasQuality());
    }
    void renderQualitySurvivesProcessRestart_data(){
        QTest::addColumn<QString>("saved");QTest::newRow("disabled")<<QString("false");QTest::newRow("enabled")<<QString("true");QTest::newRow("unset-default")<<QString("unset");
    }
    void renderQualitySurvivesProcessRestart(){
        QFETCH(QString,saved);QTemporaryDir temp;const auto path=temp.filePath("prefs.ini");
        for(const auto &mode:{QString("write"),QString("read")}){
            QProcess process;auto environment=QProcessEnvironment::systemEnvironment();environment.insert("DRAWVERSE_QUALITY_RESTART_PATH",path);environment.insert("DRAWVERSE_QUALITY_RESTART_MODE",mode);environment.insert("DRAWVERSE_QUALITY_RESTART_VALUE",saved);process.setProcessEnvironment(environment);
            const auto logPath=temp.filePath(mode+".txt");process.setProcessChannelMode(QProcess::MergedChannels);process.start(QCoreApplication::applicationFilePath(),{"renderQualityRestartProbe","-o",logPath+",txt"});QVERIFY(process.waitForStarted());QVERIFY(process.waitForFinished(20000));
            QFile log(logPath);QVERIFY(log.open(QIODevice::ReadOnly));const auto output=log.readAll()+process.readAll();QVERIFY2(process.exitStatus()==QProcess::NormalExit && process.exitCode()==0,output.constData());
            if(mode=="read")for(const auto &line:output.split('\n'))if(line.contains("Restarted preference"))qInfo().noquote()<<line.trimmed();
        }
    }
    void renderQualityRestartProbe(){
        const auto path=qEnvironmentVariable("DRAWVERSE_QUALITY_RESTART_PATH");if(path.isEmpty())QSKIP("Invoked only by cross-process restart regression");
        const auto saved=qEnvironmentVariable("DRAWVERSE_QUALITY_RESTART_VALUE");const bool expected=saved!="false";
        PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready() && !client.storageBusy() && !client.computeBusy());
        if(qEnvironmentVariable("DRAWVERSE_QUALITY_RESTART_MODE")=="write"){
            if(saved!="unset"){const auto values=client.renderSettings();QVERIFY(client.saveRenderSettings(values["memoryMiB"].toInt(),values["gpuMiB"].toInt(),expected));}
            return;
        }
        const auto restored=client.renderSettings()["highestQuality"];qInfo()<<"Restarted preference"<<restored<<restored.typeName();QCOMPARE(client.highestCanvasQuality(),expected);
        QQuickWindow owner;owner.resize(800,600);owner.show();QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);
        QQmlComponent component(&engine,QUrl("qrc:/qml/preferences/PreferencesPanel.qml"));QScopedPointer<QObject> dialog(component.createWithInitialProperties({{"parent",QVariant::fromValue(owner.contentItem())}}));QVERIFY2(dialog,qPrintable(component.errorString()));
        QVERIFY(QMetaObject::invokeMethod(dialog.data(),"open"));QTRY_VERIFY(dialog->property("opened").toBool());auto *quality=dialog->findChild<QObject*>("highestCanvasQuality");QVERIFY(quality);QCOMPARE(quality->property("checked").toBool(),expected);QCOMPARE(restored.metaType(),QMetaType::fromType<bool>());
        QTRY_VERIFY(!client.storageBusy() && !client.computeBusy());auto *save=dialog->findChild<QObject*>("saveStorageSettings");QVERIFY(save);QSignalSpy finished(&client,&PaintCoreClient::storageFinished);QVERIFY(QMetaObject::invokeMethod(save,"clicked"));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.first().first().toBool());
        QSettings settings(path,QSettings::IniFormat);QCOMPARE(settings.value("performance/highestQuality").toBool(),expected);QCOMPARE(client.highestCanvasQuality(),expected);QVERIFY(QMetaObject::invokeMethod(dialog.data(),"close"));
    }
    void renderBudgetSettings(){
        QTemporaryDir temp;
        const auto oldMemory=CompositionBudget::memoryBytes.load(),oldCache=CompositionBudget::cacheBytes.load(),oldGpu=CompositionBudget::gpuBytes.load();
        auto restore=qScopeGuard([&]{CompositionBudget::memoryBytes=oldMemory;CompositionBudget::cacheBytes=oldCache;CompositionBudget::gpuBytes=oldGpu;});
        {
            PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready());
            QCOMPARE(client.renderSettings()["memoryMiB"].toInt(),defaultMemoryMiB());
            QCOMPARE(client.storageSettings()["memoryMiB"].toInt(),defaultMemoryMiB());
            QVERIFY(client.highestCanvasQuality());
            QVERIFY(!client.saveRenderSettings(0,128));QVERIFY(!client.saveRenderSettings(512,-1));
            client.newDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);
            client.requestViewport(0,QRectF(0,0,64,64),QSize(3840,2160));
            QTRY_COMPARE_WITH_TIMEOUT(client.frame().size(),QSize(3840,2160),15000);
            QVERIFY(client.saveRenderSettings(512,128,false));
            QVERIFY(!client.highestCanvasQuality());
            QCOMPARE(client.renderSettings()["memoryMiB"].toInt(),512);
        }
        PaintCoreClient reopened(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(reopened.ready());
        QVERIFY(!reopened.highestCanvasQuality());
        QCOMPARE(CompositionBudget::memoryBytes.load(),512ull*1024*1024);
        QCOMPARE(CompositionBudget::gpuBytes.load(),128ull*1024*1024);
        QCOMPARE(CompositionBudget::viewPixels(),16777216ull);
        reopened.newDocument(64,64);QTRY_COMPARE(reopened.documentWidth(),64);
        reopened.requestViewport(0,QRectF(0,0,64,64),QSize(64,64));QTRY_VERIFY(!reopened.frame().isNull());
    }
    void denseCurveDrag_data(){QTest::addColumn<QSize>("extent");QTest::newRow("2048x2048")<<QSize(2048,2048);QTest::newRow("4096x2160")<<QSize(4096,2160);}
    void denseCurveDrag(){
        QFETCH(QSize,extent);
        if(qEnvironmentVariable("QT_QUICK_BACKEND")!="rhi")QSKIP("Actual D3D11 acceptance only");
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        QImage image(extent,QImage::Format_RGBA8888);
        for(int y=0;y<image.height();y++)for(int x=0;x<image.width();x++)image.setPixelColor(x,y,QColor(x%256,y%256,(x+y)%256));
        const auto file=temp.filePath("dense.png");QVERIFY(image.save(file));QVERIFY(client.openDocument(QUrl::fromLocalFile(file)));QTRY_VERIFY_WITH_TIMEOUT(client.documentWidth()==extent.width() && !client.fileBusy(),15000);
        QQuickWindow owner;owner.resize(700,700);auto *canvas=new CanvasItem(owner.contentItem());canvas->setSize({700,700});canvas->setAcceptedMouseButtons(Qt::NoButton);canvas->setClient(&client);
        owner.show();QVERIFY(QTest::qWaitForWindowExposed(&owner));canvas->fitToView();
        QTRY_VERIFY_WITH_TIMEOUT(client.compositionPacket() && canvas->compositionStatistics()["passes"].toULongLong()>0,15000);
        QTRY_COMPARE(canvas->compositionStatistics()["nodeRequest"].toULongLong(),client.compositionPacket()->info.key.request_id);
        QCOMPARE(owner.rendererInterface()->graphicsApi(),QSGRendererInterface::Direct3D11);
        const auto originalDisplay=owner.grabWindow();QVERIFY(!originalDisplay.isNull());
        const auto revision=client.revision();const auto undo=client.undoDepth();
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);QQmlComponent component(&engine,QUrl("qrc:/qml/tools/AdjustmentWindow.qml"));
        QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));auto *window=qobject_cast<QQuickWindow*>(object.data());QVERIFY(window);window->setTransientParent(&owner);
        QVERIFY(QMetaObject::invokeMethod(object.data(),"openFor",Q_ARG(QVariant,0)));QTRY_VERIFY(window->isVisible());QVERIFY(QTest::qWaitForWindowExposed(window));window->requestActivate();QTRY_VERIFY(window->isActive());QTRY_VERIFY(!client.adjustmentBusy() && !object->property("dirty").toBool());
        QVERIFY2(client.adjustmentGpuPreview(),qPrintable(client.compositionFallback()));
        QTRY_COMPARE(canvas->compositionStatistics()["lastSubmitGeometry"].toULongLong(),client.compositionPacket()->geometrySequence);
        const auto resources=canvas->compositionStatistics();
        auto *drag=object->findChild<QQuickItem*>("adjustmentCurveDrag");QVERIFY(drag);
        QTRY_VERIFY(drag->width()>250 && drag->height()>220);
        auto a=drag->mapToScene({drag->width()-1,1}).toPoint();QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,a);QVERIFY(drag->property("pressed").toBool());qInfo()<<"pressed"<<drag->size()<<a<<drag->property("moving")<<drag->parentItem()->parentItem()->property("selected")<<object->property("curves");
        for(int iteration=0;iteration<12;iteration++){
            const auto previousSequence=client.compositionPacket()->geometrySequence;
            const auto b=drag->mapToScene({drag->width()-1,30.+iteration*12}).toPoint();QElapsedTimer timer;timer.start();
            QTest::mouseMove(window,b);
            while(object->property("dirty").toBool() && timer.elapsed()<2000)QTest::qWait(1);
            QVERIFY(!object->property("dirty").toBool());
            const auto cpuNs=timer.nsecsElapsed();const auto packet=client.compositionPacket();QVERIFY(packet && packet->adjustment);
            const auto sequence=packet->geometrySequence;QVERIFY(sequence>previousSequence);
            while(canvas->compositionStatistics()["lastSubmitGeometry"].toULongLong()!=sequence && timer.elapsed()<2000)QTest::qWait(1);
            QCOMPARE(canvas->compositionStatistics()["lastSubmitGeometry"].toULongLong(),sequence);
            qInfo()<<"dense curve move"<<iteration<<"UI ms"<<double(cpuNs)/1e6<<"submit ms"<<double(timer.nsecsElapsed())/1e6;
            QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),undo);QVERIFY(!client.adjustmentBusy());
        }
        QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,drag->mapToScene({drag->width()-1,162}).toPoint());
        const auto after=canvas->compositionStatistics();
        for(const auto *key:{"uploads","pageTableCreates","axisCreates","parameterBufferCreates","bindingCreates","pipelineCreates"})QCOMPARE(after[key],resources[key]);
        QVERIFY(after["textureBytes"].toULongLong()<=(extent.width()>2048?512ull:256ull)*1024*1024);
        qInfo()<<"all curves"<<object->property("curves");qInfo()<<"dense endpoint"<<object->property("curves").toList()[0].toList().last()<<"lut last"<<reinterpret_cast<const float*>(client.compositionPacket()->adjustment->lut.constData())[4096*4];
        QVERIFY(object->property("curves").toList()[0].toList().last().toList()[1].toDouble()<.8);
        const auto preview=owner.grabWindow();QVERIFY(!preview.isNull());
        originalDisplay.save(QString("artifacts/adjustment-v3-original-%1.png").arg(extent.width()));preview.save(QString("artifacts/adjustment-v3-preview-%1.png").arg(extent.width()));
        QVERIFY(preview!=originalDisplay);
        object->setProperty("ending",2);object->setProperty("dirty",true);QVERIFY(QMetaObject::invokeMethod(object.data(),"process"));
        QTRY_VERIFY_WITH_TIMEOUT(!window->isVisible() && !client.adjustmentActive(),20000);QTRY_COMPARE(client.undoDepth(),undo+1);
        QTRY_VERIFY(client.compositionPacket() && client.compositionPacket()->info.key.revision==client.revision());
        QTRY_COMPARE(canvas->compositionStatistics()["revision"].toULongLong(),client.revision());
        QTRY_VERIFY(!client.adjustmentGpuPreview());
        const auto committed=owner.grabWindow();QCOMPARE(committed.size(),preview.size());int maximum=0;
        for(int y=0;y<preview.height();y++)for(int x=0;x<preview.width();x++){
            const auto a=preview.pixelColor(x,y),b=committed.pixelColor(x,y);
            maximum=std::max({maximum,std::abs(a.red()-b.red()),std::abs(a.green()-b.green()),std::abs(a.blue()-b.blue()),std::abs(a.alpha()-b.alpha())});
        }
        QVERIFY2(maximum<=1,qPrintable(QString("dense preview/commit difference %1").arg(maximum)));
        client.undo();QTRY_COMPARE(client.undoDepth(),undo);QTRY_VERIFY(client.compositionPacket() && client.compositionPacket()->info.key.revision==client.revision());
        QTRY_COMPARE(canvas->compositionStatistics()["revision"].toULongLong(),client.revision());QCOMPARE(owner.grabWindow(),originalDisplay);
    }
    void largeAtlasGrowth(){
        if(qEnvironmentVariable("QT_QUICK_BACKEND")!="rhi")QSKIP("Actual D3D11 acceptance only");
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newDocument(4096,2160);QTRY_VERIFY(client.documentWidth()==4096 && !client.layerEditBusy());
        QQuickWindow owner;owner.resize(600,600);auto *canvas=new CanvasItem(owner.contentItem());canvas->setSize({600,600});canvas->setClient(&client);
        owner.show();QVERIFY(QTest::qWaitForWindowExposed(&owner));canvas->fitToView();
        QTRY_VERIFY_WITH_TIMEOUT(client.compositionPacket() && canvas->compositionStatistics()["passes"].toULongLong()>0,15000);
        QVERIFY(client.compositionPacket()->tiles.isEmpty());const auto before=canvas->compositionStatistics();const auto undo=client.undoDepth();
        client.activateBucket();client.setBrushColor(QColor("#386da2"));QVERIFY(client.bucketFill({0,0}));QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),undo+1,15000);
        QTRY_VERIFY_WITH_TIMEOUT(client.compositionPacket() && client.compositionPacket()->info.key.revision==client.revision(),15000);
        QVERIFY(client.compositionPacket()->tiles.size()>1024);QTRY_COMPARE(canvas->compositionStatistics()["revision"].toULongLong(),client.revision());
        const auto after=canvas->compositionStatistics();QVERIFY(after["textureBytes"].toULongLong()>before["textureBytes"].toULongLong());QCOMPARE(after["failures"].toInt(),0);
        const auto capture=owner.grabWindow();const auto center=capture.pixelColor(capture.width()/2,capture.height()/2);
        QVERIFY(std::abs(center.red()-0x38)<=1 && std::abs(center.green()-0x6d)<=1 && std::abs(center.blue()-0xa2)<=1);
        const auto uploads=canvas->compositionStatistics()["uploads"];owner.update();QTest::qWait(40);QCOMPARE(canvas->compositionStatistics()["uploads"],uploads);
        client.undo();QTRY_COMPARE(client.undoDepth(),undo);QTRY_VERIFY(client.compositionPacket() && client.compositionPacket()->info.key.revision==client.revision());QTRY_COMPARE(canvas->compositionStatistics()["revision"].toULongLong(),client.revision());
        const auto restored=owner.grabWindow();QCOMPARE(restored.pixelColor(restored.width()/2,restored.height()/2),QColor(Qt::white));
        client.addLayer("large unsupported second layer");QTRY_VERIFY(client.layers().size()==2 && !client.layerEditBusy());
        QTRY_VERIFY_WITH_TIMEOUT(!client.compositionPacket() && !client.compositionFallback().isEmpty(),15000);
    }
    void controlsMatchLegacy_data(){QTest::addColumn<int>("kind");for(int k=0;k<3;k++)QTest::newRow(qPrintable(QString("controls-%1").arg(k)))<<k;}
    void controlsMatchLegacy(){
        QFETCH(int,kind);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        QImage image(96,80,QImage::Format_RGBA8888);for(int y=0;y<80;y++)for(int x=0;x<96;x++)image.setPixelColor(x,y,QColor(x*255/95,y*255/79,(x+y)*255/174));
        const auto file=temp.filePath("colors.png");QVERIFY(image.save(file));QVERIFY(client.openDocument(QUrl::fromLocalFile(file)));QTRY_VERIFY(client.documentWidth()==96 && !client.fileBusy());
        client.requestViewport(0,{0,0,96,80},{96,80});QTRY_VERIFY(client.frameRevision()==client.revision() && !client.frame().isNull());const auto original=client.frame();const auto undo=client.undoDepth();
        QVERIFY(client.beginAdjustment(kind));QTRY_VERIFY(!client.adjustmentBusy());
        QVariantList lut;const auto master=[](double x){return x<=.5?x*.5:x*1.5-.5;};
        for(int i=0;i<=4096;i++){const double x=double(i)/4096.;double r=x,g=x,b=x;
            if(kind==0){r=master(x*.75);g=master(x);b=master(.25+x*.75);}
            else if(kind==1)r=g=b=(17+197*std::pow(std::clamp((x*255-32)/191,0.,1.),1/1.7))/255;
            lut.append(QVariant(QVariantList{r,g,b}));
        }
        QVERIFY(client.updateAdjustment(kind,lut,42,30,-10,kind==2));QTRY_VERIFY(!client.adjustmentBusy());QTRY_COMPARE(client.frameRevision(),client.revision());const auto legacy=client.frame();QVERIFY(legacy!=original);
        const QVariantList curves{QVariantList{QVariantList{0.,0.},QVariantList{.5,.25},QVariantList{1.,1.}},QVariantList{QVariantList{0.,0.},QVariantList{1.,.75}},QVariantList{QVariantList{0.,0.},QVariantList{1.,1.}},QVariantList{QVariantList{0.,.25},QVariantList{1.,1.}}};
        const QVariantList modes{QVariantList{false,false,false},QVariantList{false,false},QVariantList{},QVariantList{false,false}};
        QVariantMap values{{"curves",curves},{"modes",modes},{"levels",QVariantList{32.,223.,1.7,17.,214.}},{"hue",42},{"saturation",30},{"lightness",-10},{"colorize",true}};
        QVERIFY(client.updateAdjustmentControls(kind,values));QTRY_VERIFY(!client.adjustmentBusy());QTRY_COMPARE(client.frameRevision(),client.revision());
        QCOMPARE(client.frame(),legacy);QCOMPARE(client.undoDepth(),undo);
        values["neutral"]=true;QVERIFY(client.updateAdjustmentControls(kind,values));QTRY_VERIFY(!client.adjustmentBusy());QTRY_COMPARE(client.frameRevision(),client.revision());QCOMPARE(client.frame(),original);
        QVERIFY(client.finishAdjustment(false));QTRY_VERIFY(!client.adjustmentActive());QCOMPARE(client.undoDepth(),undo);
    }
    void modalAdjustments_data(){QTest::addColumn<int>("kind");QTest::addColumn<bool>("cpu");for(int i=0;i<3;i++){QTest::newRow(qPrintable(QString("adjustment-%1-auto").arg(i)))<<i<<false;QTest::newRow(qPrintable(QString("adjustment-%1-cpu").arg(i)))<<i<<true;}}
    void modalAdjustments(){
        QFETCH(int,kind);QFETCH(bool,cpu);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(96,80);QTRY_VERIFY(client.documentWidth()==96 && !client.layerEditBusy());
        if(cpu){QTRY_VERIFY(!client.computeBusy());QVERIFY(client.saveComputeDevice("cpu"));QTRY_VERIFY(client.computeDevice()=="cpu" && !client.computeBusy());}
        client.setBrushColor(QColor("#707070"));client.setBrushRadius(43);client.setBrushOpacity(.8);
        const auto paintDepth=client.undoDepth();QVERIFY(client.beginStroke({{43,35},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),paintDepth+1);
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);
        QQmlComponent component(&engine,QUrl("qrc:/qml/tools/AdjustmentWindow.qml"));
        QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));
        auto *window=qobject_cast<QQuickWindow*>(object.data());QVERIFY(window);QQuickWindow owner;owner.resize(600,600);owner.show();window->setTransientParent(&owner);
        client.requestViewport(0,QRectF(0,0,96,80),QSize(96,80));QTRY_VERIFY(!client.frame().isNull() && client.frameRevision()==client.revision());const auto original=client.frame();
        const auto undo=client.undoDepth();
        QVERIFY(QMetaObject::invokeMethod(object.data(),"openFor",Q_ARG(QVariant,kind)));
        QTRY_VERIFY(window->isVisible());QCOMPARE(window->modality(),Qt::NonModal);
        QVERIFY(window->flags()&Qt::FramelessWindowHint);
        QVERIFY(QTest::qWaitForWindowExposed(window));
        if(qEnvironmentVariable("QT_QUICK_BACKEND")=="rhi")QCOMPARE(window->rendererInterface()->graphicsApi(),QSGRendererInterface::Direct3D11);
        QTRY_VERIFY(!client.adjustmentBusy());QTRY_VERIFY(!object->property("dirty").toBool());
        QCOMPARE(client.adjustmentHistogram().size(),4);
        auto *close=object->findChild<QQuickItem*>("adjustmentClose");QVERIFY(close);
        auto *closeBackground=object->findChild<QQuickItem*>("adjustmentCloseBackground");QVERIFY(closeBackground);
        QTest::mouseMove(window,QPoint(10,60));QTRY_COMPARE(closeBackground->property("color").value<QColor>().alpha(),0);
        QTest::mouseMove(window,close->mapToScene({16,16}).toPoint());QTRY_COMPARE(closeBackground->property("color").value<QColor>(),QColor("#e81123"));
        QTest::mouseMove(window,QPoint(10,60));QTRY_COMPARE(closeBackground->property("color").value<QColor>().alpha(),0);
        auto *title=object->findChild<QQuickItem*>("adjustmentTitleDrag");QVERIFY(title);
        const auto oldPosition=window->position();const auto titlePoint=title->mapToScene({50,15}).toPoint();
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,titlePoint);
        QTest::mouseMove(window,titlePoint+QPoint(35,22));
        QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,titlePoint);
        QTRY_VERIFY(window->position()!=oldPosition);QCOMPARE(window->position(),oldPosition+QPoint(35,22));
        if(kind==2){auto *drag=object->findChild<QQuickItem*>("adjustLightnessDrag");QVERIFY(drag);
            auto a=drag->mapToScene({drag->width()/2,10}).toPoint(),b=drag->mapToScene({drag->width()*.3,10}).toPoint();
            QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,a);QTest::mouseMove(window,b);
            QTRY_VERIFY(object->property("lightness").toDouble()<-20); // change occurs before release
            QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,b);
        }else if(kind==1){auto *drag=object->findChild<QQuickItem*>("levelsOutputDrag");QVERIFY(drag);
            auto a=drag->mapToScene({drag->width()-7,8}).toPoint(),b=drag->mapToScene({drag->width()*.6,8}).toPoint();
            QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,a);QTest::mouseMove(window,b);
            QTRY_VERIFY(object->property("outputWhite").toDouble()<200);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,b);
        }else{auto *drag=object->findChild<QQuickItem*>("adjustmentCurveDrag");QVERIFY(drag);
            auto a=drag->mapToScene({drag->width()-1,1}).toPoint(),b=drag->mapToScene({drag->width()-1,drag->height()*.4}).toPoint();
            QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,a);QTest::mouseMove(window,b);
            QTRY_VERIFY(object->property("curves").toList()[0].toList().last().toList()[1].toDouble()<.8);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,b);
        }
        QVERIFY(QMetaObject::invokeMethod(object.data(),"reset"));QTRY_VERIFY(!object->property("dirty").toBool() && !client.adjustmentBusy());
        if(kind==0)QVERIFY(object->setProperty("curves",QVariantList{QVariantList{QVariantList{0.,0.},QVariantList{1.,0.}},QVariantList{QVariantList{0.,0.},QVariantList{1.,1.}},QVariantList{QVariantList{0.,0.},QVariantList{1.,1.}},QVariantList{QVariantList{0.,0.},QVariantList{1.,1.}}}));
        else if(kind==1)object->setProperty("outputWhite",100);
        else object->setProperty("lightness",-50);
        object->setProperty("dirty",true);
        QTRY_VERIFY(!object->property("dirty").toBool() && !client.adjustmentBusy());
        QVERIFY2(!client.adjustmentFailed(),qPrintable(client.lastError()));QCOMPARE(client.undoDepth(),undo);
        QTRY_VERIFY(client.frameRevision()==client.revision() && client.frame()!=original);const auto adjusted=client.frame();
        QVERIFY(!client.beginTransform());
        const auto image=window->grabWindow();QVERIFY(!image.isNull());const auto artifacts=qEnvironmentVariable("DRAWVERSE_ADJUSTMENT_ARTIFACTS");if(!artifacts.isEmpty()){QDir().mkpath(artifacts);QVERIFY(image.save(QDir(artifacts).filePath(QString("adjustment-%1-%2.png").arg(qEnvironmentVariable("QT_QUICK_BACKEND"),QString::number(kind)))));}
        auto *apply=object->findChild<QQuickItem*>("adjustmentApply");QVERIFY(apply);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,apply->mapToScene(QPointF(apply->width()/2,apply->height()/2)).toPoint());QTRY_VERIFY(!window->isVisible());QTRY_COMPARE(client.undoDepth(),undo+1);
        const QStringList titles{QStringLiteral("曲线"),QStringLiteral("色阶"),QStringLiteral("色相/饱和度/明度")};QCOMPARE(client.historyEntries().last().toMap()["title"].toString(),titles[kind]);
        client.undo();QTRY_COMPARE(client.undoDepth(),undo);QTRY_VERIFY(client.frameRevision()==client.revision());QCOMPARE(client.frame(),original);
        client.redo();QTRY_COMPARE(client.undoDepth(),undo+1);QTRY_VERIFY(client.frameRevision()==client.revision());QCOMPARE(client.frame(),adjusted);
        QVERIFY(QMetaObject::invokeMethod(object.data(),"openFor",Q_ARG(QVariant,kind)));QTRY_VERIFY(window->isVisible());
        window->requestActivate();QTRY_VERIFY(window->isActive());QVERIFY(QGuiApplication::modalWindow()!=window);
        QTest::keyClick(window,Qt::Key_Escape);QTRY_VERIFY(!window->isVisible());QCOMPARE(client.undoDepth(),undo+1);
    }
    void residentCanvas_data(){
        QTest::addColumn<int>("kind");QTest::addColumn<int>("topology");
        for(int k=0;k<3;k++)for(int t=0;t<4;t++)QTest::newRow(qPrintable(QString("resident-%1-topology-%2").arg(k).arg(t)))<<k<<t;
    }
    void residentCanvas(){
        if(qEnvironmentVariable("QT_QUICK_BACKEND")!="rhi")QSKIP("Actual D3D11 acceptance only");
        QFETCH(int,kind);QFETCH(int,topology);
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");qputenv("DRAWVERSE_CPU_DISPLAY_REFERENCE","1");
        const auto env=qScopeGuard([]{qunsetenv("DRAWVERSE_CPU_DISPLAY_REFERENCE");});
        QTemporaryDir temp;{QSettings settings(temp.filePath("prefs.ini"),QSettings::IniFormat);settings.setValue("performance/highestQuality",false);settings.sync();}PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newDocument(96,80);QTRY_VERIFY(client.documentWidth()==96 && !client.layerEditBusy());
        client.addLayer("color");QTRY_VERIFY(client.layers().size()==2 && !client.layerEditBusy());const auto target=client.activeLayer();
        client.setBrushColor(QColor("#de3081"));client.setBrushOpacity(.7);client.setBrushRadius(31);
        auto undo=client.undoDepth();QVERIFY(client.beginStroke({{45,35},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),undo+1);
        const auto mutate=[&](auto action){const auto revision=client.revision();if(!action())return false;return QTest::qWaitFor([&]{return client.revision()>revision && !client.layerEditBusy();},5000);};
        if(topology==1){QVERIFY(mutate([&]{return client.groupLayer(target,"adjust group");}));QVERIFY(mutate([&]{return client.setLayerFill(client.activeLayer(),.63);}));}
        if(topology==2){QVERIFY(mutate([&]{return client.setLayerClipping(target,true);}));QVERIFY(mutate([&]{return client.setLayerBlend(target,3);}));}
        if(topology==3){QVERIFY(mutate([&]{return client.addMask(target);}));client.setBrushColor(Qt::black);client.setBrushRadius(13);undo=client.undoDepth();QVERIFY(client.beginStroke({{49,34},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),undo+1);QVERIFY(mutate([&]{return client.setLayerBlend(target,3);}));}
        client.selectLayer(target);QTRY_VERIFY(client.activeLayer()==target && !client.layerEditBusy());
        QQuickWindow owner;owner.resize(400,300);auto *canvas=new CanvasItem(owner.contentItem());canvas->setSize({400,300});canvas->setAcceptedMouseButtons(Qt::NoButton);
        auto readback=std::make_shared<CompositionReadback>();canvas->observeCompositionReadback(readback);canvas->setClient(&client);
        owner.show();QVERIFY(QTest::qWaitForWindowExposed(&owner));canvas->actualSize();
        QTRY_VERIFY(canvas->gpuDisplay() && client.compositionPacket() && canvas->compositionStatistics()["passes"].toULongLong()>0);
        QTRY_VERIFY(client.compositionPacket()->region==canvas->prefetchedDocumentRect());
        QTRY_VERIFY(client.compositionPacket()->pixels==QSize(qCeil(canvas->prefetchedDocumentRect().width()*canvas->zoom()*owner.devicePixelRatio()),qCeil(canvas->prefetchedDocumentRect().height()*canvas->zoom()*owner.devicePixelRatio())));
        QTRY_COMPARE(canvas->compositionStatistics()["nodeRequest"].toULongLong(),client.compositionPacket()->info.key.request_id);
        QTRY_VERIFY(client.frameRevision()==client.revision() && client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==client.compositionPacket()->pixels);
        const auto original=client.frame();const auto revision=client.revision();undo=client.undoDepth();
        QVERIFY(client.beginAdjustment(kind));QTRY_VERIFY(!client.adjustmentBusy());QVERIFY(client.adjustmentGpuPreview());
        QVariantList lut;for(int i=0;i<=4096;i++){const double x=double(i)/4096;lut.append(QVariant(QVariantList{kind==0?x*x:x*.7,kind==0?std::sqrt(x):x*.7,x*.7}));}
        QTRY_COMPARE(canvas->compositionStatistics()["lastSubmitGeometry"].toULongLong(),client.compositionPacket()->geometrySequence);
        const auto before=canvas->compositionStatistics();
        for(int i=0;i<12;i++){QVERIFY(client.updateAdjustment(kind,lut,40+i*3,35,-10,kind==2 && topology==3));owner.update();QTest::qWait(18);}
        QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),undo);QVERIFY(!client.adjustmentBusy());
        const auto preview=client.compositionPacket();QVERIFY(preview->adjustment);
        readback->request(*preview);owner.update();QTRY_VERIFY_WITH_TIMEOUT((owner.update(),readback->completed.load()==2),10000);
        const auto rendered=readback->display;QVERIFY(!rendered.isEmpty());
        const auto after=canvas->compositionStatistics();QCOMPARE(after["uploads"],before["uploads"]);
        QCOMPARE(after["pageTableCreates"],before["pageTableCreates"]);QCOMPARE(after["parameterBufferCreates"],before["parameterBufferCreates"]);
        QVERIFY(after["adjustmentUpdates"].toULongLong()>before["adjustmentUpdates"].toULongLong());
        QVERIFY(client.finishAdjustment(true));QTRY_VERIFY(!client.adjustmentActive() && !client.adjustmentBusy());QTRY_COMPARE(client.undoDepth(),undo+1);
        QTRY_VERIFY(client.frameRevision()==client.revision());QVERIFY(client.frame()!=original);
        QTRY_VERIFY(client.compositionPacket() && client.compositionPacket()->info.key.revision==client.revision());
        const auto committed=client.compositionPacket();readback->request(*committed);owner.update();QTRY_VERIFY_WITH_TIMEOUT((owner.update(),readback->completed.load()==2),10000);
        QCOMPARE(rendered.size(),readback->display.size());int maximum=0;
        for(qsizetype i=0;i<rendered.size();i++)maximum=std::max(maximum,std::abs(int(uchar(rendered[i]))-int(uchar(readback->display[i]))));
        QVERIFY2(maximum<=1,qPrintable(QString("preview/commit maximum channel difference %1").arg(maximum)));
        QTRY_VERIFY(!client.adjustmentGpuPreview());
        client.undo();QTRY_COMPARE(client.undoDepth(),undo);QTRY_VERIFY(client.frameRevision()==client.revision());QCOMPARE(client.frame(),original);
        QTRY_VERIFY(client.compositionPacket() && client.compositionPacket()->info.key.revision==client.revision());
        QVERIFY(client.beginAdjustment(kind));QTRY_VERIFY(!client.adjustmentBusy());QVERIFY(client.adjustmentGpuPreview());
        QVERIFY(client.updateAdjustment(kind,lut,60,30,-20));QVERIFY(client.finishAdjustment(false));QTRY_VERIFY(!client.adjustmentActive());
        QCOMPARE(client.undoDepth(),undo);QTRY_VERIFY(client.frameRevision()==client.revision());QCOMPARE(client.frame(),original);
    }

    void modalResidentAndFallback_data(){QTest::addColumn<int>("mode");QTest::newRow("modal-resident")<<0;QTest::newRow("selection-fallback")<<1;QTest::newRow("device-fallback")<<2;}
    void modalResidentAndFallback(){
        if(qEnvironmentVariable("QT_QUICK_BACKEND")!="rhi")QSKIP("Actual D3D11 acceptance only");
        QFETCH(int,mode);qputenv("DRAWVERSE_CPU_DISPLAY_REFERENCE","1");
        const auto env=qScopeGuard([]{qunsetenv("DRAWVERSE_CPU_DISPLAY_REFERENCE");qunsetenv("DRAWVERSE_COMPOSITION_FAIL");});
        QTemporaryDir temp;{QSettings settings(temp.filePath("prefs.ini"),QSettings::IniFormat);settings.setValue("performance/highestQuality",false);settings.sync();}PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newDocument(96,80);QTRY_VERIFY(client.documentWidth()==96 && !client.layerEditBusy());
        if(mode==1){QVERIFY(client.editSelection({5,5,60,60},0,0));QTRY_VERIFY(client.selectionEnabled() && !client.layerEditBusy());}
        QQuickWindow owner;owner.resize(600,600);auto *canvas=new CanvasItem(owner.contentItem());canvas->setSize({600,600});canvas->setAcceptedMouseButtons(Qt::NoButton);canvas->setClient(&client);
        owner.show();QVERIFY(QTest::qWaitForWindowExposed(&owner));canvas->actualSize();
        QTRY_VERIFY(client.compositionPacket() && canvas->compositionStatistics()["passes"].toULongLong()>0);QTRY_VERIFY(client.compositionPacket()->region==canvas->prefetchedDocumentRect());QTRY_VERIFY(client.compositionPacket()->pixels==QSize(qCeil(canvas->prefetchedDocumentRect().width()*canvas->zoom()*owner.devicePixelRatio()),qCeil(canvas->prefetchedDocumentRect().height()*canvas->zoom()*owner.devicePixelRatio())));QTRY_VERIFY(client.frameRevision()==client.revision() && client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==client.compositionPacket()->pixels);
        const auto original=client.frame();const auto revision=client.revision();const auto undo=client.undoDepth();
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);QQmlComponent component(&engine,QUrl("qrc:/qml/tools/AdjustmentWindow.qml"));
        QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));auto *window=qobject_cast<QQuickWindow*>(object.data());QVERIFY(window);window->setTransientParent(&owner);
        QVERIFY(QMetaObject::invokeMethod(object.data(),"openFor",Q_ARG(QVariant,2)));QTRY_VERIFY(window->isVisible());QTRY_VERIFY(!client.adjustmentBusy() && !object->property("dirty").toBool());
        QCOMPARE(client.adjustmentGpuPreview(),mode!=1);
        if(mode==2)qputenv("DRAWVERSE_COMPOSITION_FAIL","upload");
        auto *drag=object->findChild<QQuickItem*>("adjustLightnessDrag");QVERIFY(drag);
        const auto a=drag->mapToScene({drag->width()/2,10}).toPoint(),b=drag->mapToScene({drag->width()*.25,10}).toPoint();
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,a);QTest::mouseMove(window,b);
        QTRY_VERIFY(object->property("lightness").toDouble()<-30);
        QTRY_VERIFY(!object->property("dirty").toBool() && !client.adjustmentBusy());
        if(mode==0){QCOMPARE(client.revision(),revision);QTRY_VERIFY(canvas->compositionStatistics()["adjustmentUpdates"].toULongLong()>0);QCOMPARE(client.frame(),original);}
        else{QTRY_VERIFY(!client.adjustmentGpuPreview());QTRY_VERIFY(client.frameRevision()==client.revision() && client.frame()!=original);}
        QCOMPARE(client.undoDepth(),undo);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,b);
        qunsetenv("DRAWVERSE_COMPOSITION_FAIL");
        QVERIFY(QMetaObject::invokeMethod(object.data(),"reset"));QTRY_VERIFY(!object->property("dirty").toBool() && !client.adjustmentBusy());
        if(mode==0){auto *colorize=object->findChild<QQuickItem*>("adjustmentColorize");QVERIFY(colorize);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,colorize->mapToScene({10,10}).toPoint());QTRY_VERIFY(object->property("colorize").toBool());QCOMPARE(object->property("saturation").toDouble(),25.);}
        object->setProperty("ending",3);QVERIFY(QMetaObject::invokeMethod(object.data(),"process"));QTRY_VERIFY(!window->isVisible());QCOMPARE(client.undoDepth(),undo);
        QTRY_VERIFY(client.frameRevision()==client.revision());QCOMPARE(client.frame(),original);
    }

};
QTEST_MAIN(AdjustmentTests)
#include "adjustment_tests.moc"
