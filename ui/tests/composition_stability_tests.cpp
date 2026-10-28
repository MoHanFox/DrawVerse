#include "platform/UiResources.h"
#include "canvas/CanvasItem.h"
#include "canvas/CompositionNode.h"
#include <QtTest>
#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QTemporaryDir>
#include <QFile>
#include <QJsonDocument>
#include <QScopeGuard>
#include <memory>
#include <vector>

namespace {
void save(const QString &name,const QJsonObject &data){
    const auto dir=qEnvironmentVariable("DRAWVERSE_G5_REPORT_DIR");if(dir.isEmpty())return;
    QFile f(dir+"/"+name+".json");if(f.open(QIODevice::WriteOnly))f.write(QJsonDocument(data).toJson());
}
int difference(const QImage &a,const QImage &b){
    static int capture=0;const auto dir=qEnvironmentVariable("DRAWVERSE_G5_REPORT_DIR");
    if(!dir.isEmpty()){
        const auto name=QString::fromLatin1(QTest::currentTestFunction())+"-"+QString::fromLatin1(QTest::currentDataTag()?QTest::currentDataTag():"")+"-"+QString::number(++capture);
        a.save(dir+"/"+name+"-a.png");b.save(dir+"/"+name+"-b.png");
    }
    if(a.isNull() || a.size()!=b.size())return 256;
    const auto x=a.convertToFormat(QImage::Format_RGBA8888),y=b.convertToFormat(QImage::Format_RGBA8888);int error=0;
    for(int j=0;j<x.height();++j)for(int i=0;i<x.width()*4;++i)error=std::max(error,std::abs(int(x.constScanLine(j)[i])-int(y.constScanLine(j)[i])));
    return error;
}
bool ready(PaintCoreClient &c){return QTest::qWaitFor([&]{return c.ready() && c.brushLibraryReady() && !c.layerEditBusy();},15000);}
bool document(PaintCoreClient &c,int side){const auto g=c.generation();c.newDocument(side,side);return QTest::qWaitFor([&]{return c.generation()!=g && c.documentWidth()==side && ready(c);},15000);}
bool dab(PaintCoreClient &c,QPointF p,QColor color=Qt::red){
    c.activateBrush();c.setBrushColor(color);c.setBrushRadius(5);const auto depth=c.undoDepth();
    if(!c.beginStroke({p,1}))return false;c.strokeTo({p+QPointF(1,1),1});c.endStroke();
    return QTest::qWaitFor([&]{return c.undoDepth()==depth+1 && !c.drawing();},15000);
}
CanvasItem *attach(QQuickWindow &w,PaintCoreClient &c){
    w.resize(384,288);w.setColor(Qt::black);auto *canvas=new CanvasItem(w.contentItem());canvas->setSize({384,288});canvas->setClient(&c);
    w.show();if(!QTest::qWaitForWindowExposed(&w))return nullptr;canvas->fitToView();canvas->actualSize();return canvas;
}
bool software(const QQuickWindow &w){return w.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;}
bool cpuReady(PaintCoreClient &c,CanvasItem &canvas,QQuickWindow &w){
    return QTest::qWaitFor([&]{const auto r=canvas.prefetchedDocumentRect();
        const QSize size(qCeil(r.width()*canvas.zoom()*w.devicePixelRatio()),qCeil(r.height()*canvas.zoom()*w.devicePixelRatio()));
        return !c.frame().isNull() && c.frameRevision()==c.revision() && c.frameRegion()==r && c.frame().size()==size;
    },15000);
}
bool gpuReady(PaintCoreClient &c,CanvasItem &canvas){
    return QTest::qWaitFor([&]{const auto stats=canvas.compositionStatistics();const auto r=canvas.prefetchedDocumentRect();
        const auto dpr=canvas.window()?canvas.window()->devicePixelRatio():1.;
        const QSize wanted(qCeil(r.width()*canvas.zoom()*dpr),qCeil(r.height()*canvas.zoom()*dpr));
        return c.compositionPacket() && c.compositionPacket()->region==r && c.compositionPacket()->pixels==wanted && !stats["failed"].toBool() &&
        stats["nodeEpoch"]==stats["currentEpoch"] && stats["nodeRevision"].toULongLong()==c.revision() &&
        stats["nodeRequest"].toULongLong()==c.compositionPacket()->info.key.request_id;
    },15000);
}
QImage frame(QQuickWindow &w){QSignalSpy swapped(&w,&QQuickWindow::frameSwapped);w.update();if(!QTest::qWaitFor([&]{return !swapped.isEmpty();},5000))return {};return w.grabWindow();}
bool matchesCpu(PaintCoreClient &c,CanvasItem &canvas,QQuickWindow &w){
    if(software(w))return cpuReady(c,canvas,w);
    if(!gpuReady(c,canvas))return false;
    const auto gpu=frame(w);const auto prior=qgetenv("DRAWVERSE_GPU_COMPOSITION");const bool wasSet=qEnvironmentVariableIsSet("DRAWVERSE_GPU_COMPOSITION");
    const auto restore=qScopeGuard([&]{if(wasSet)qputenv("DRAWVERSE_GPU_COMPOSITION",prior);else qunsetenv("DRAWVERSE_GPU_COMPOSITION");emit c.frameChanged();});
    qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit c.frameChanged();
    if(!cpuReady(c,canvas,w))return false;
    const int error=difference(gpu,frame(w));
    if(error>1)qWarning()<<"GPU/CPU window error"<<error;
    return error<=1;
}
}
class CompositionStabilityTests final : public QObject {
    Q_OBJECT
private slots:
    void initTestCase(){initializeDrawVerseUiResources();}
    void init(){qunsetenv("DRAWVERSE_GPU_COMPOSITION");qunsetenv("DRAWVERSE_COMPOSITION_FAIL");QTRY_COMPARE(CompositionNode::reservedPayloadBytes(),0);}
    void cleanup(){qunsetenv("DRAWVERSE_GPU_COMPOSITION");qunsetenv("DRAWVERSE_COMPOSITION_FAIL");}
    void readbackRejectsOldVersionAndPacket(){
        CompositionReadback readback;CompositionPacket a;a.info.key.device_epoch=4;a.info.key.document_generation=2;a.info.key.revision=7;a.info.key.request_id=3;
        a.pixels={128,128};a.region={0,0,128,128};readback.request(a);const auto old=readback.version.load();
        QVERIFY(readback.accept(old,CompositionReadback::identityOf(a),0,"old"));QCOMPARE(readback.completed.load(),1);
        auto b=a;b.info.key.request_id=4;readback.request(b);const auto current=readback.version.load();QCOMPARE(readback.completed.load(),0);
        QVERIFY(!readback.accept(old,CompositionReadback::identityOf(a),1,"late"));
        QVERIFY(!readback.accept(current,CompositionReadback::identityOf(a),1,"wrong packet"));
        auto wrong=b;wrong.geometrySequence=9;QVERIFY(!readback.accept(current,CompositionReadback::identityOf(wrong),0,"wrong geometry"));
        wrong=b;wrong.info.key.device_epoch=5;QVERIFY(!readback.accept(current,CompositionReadback::identityOf(wrong),0,"wrong epoch"));
        QVERIFY(readback.accept(current,CompositionReadback::identityOf(b),0,"linear"));
        QVERIFY(!readback.accept(current,CompositionReadback::identityOf(b),0,"duplicate"));QCOMPARE(readback.completed.load(),1);
        QVERIFY(readback.accept(current,CompositionReadback::identityOf(b),1,"display"));QCOMPARE(readback.completed.load(),2);
        QCOMPARE(readback.linear,QByteArray("linear"));QCOMPARE(readback.display,QByteArray("display"));QVERIFY(!readback.pending(b,0));
        readback.request(b);QVERIFY(!readback.accept(current,CompositionReadback::identityOf(b),1,"same packet old version"));QCOMPARE(readback.completed.load(),0);
        QCOMPARE(readback.pending(b,current).value(),readback.version.load());
    }
    void defaultPolicy_data(){
        QTest::addColumn<QByteArray>("overrideValue");QTest::addColumn<bool>("cpuCompute");
        QTest::newRow("default")<<QByteArray()<<false;QTest::newRow("off")<<QByteArray("0")<<false;
        QTest::newRow("invalid")<<QByteArray("invalid")<<false;QTest::newRow("explicit-on")<<QByteArray("1")<<false;
        QTest::newRow("cpu-compute-independent")<<QByteArray()<<true;
    }
    void defaultPolicy(){
        QFETCH(QByteArray,overrideValue);QFETCH(bool,cpuCompute);
        if(!overrideValue.isNull())qputenv("DRAWVERSE_GPU_COMPOSITION",overrideValue);
        QCOMPARE(compositionDisplayRequested(),overrideValue.isNull() || overrideValue=="1");
        QTemporaryDir temp;PaintCoreClient c(nullptr,temp.filePath("prefs.ini"));QVERIFY(ready(c));QVERIFY(document(c,128));
        if(cpuCompute){QTRY_VERIFY(!c.computeBusy());QVERIFY(c.saveComputeDevice("cpu"));QTRY_VERIFY(!c.computeBusy() && c.computeDevice()=="cpu");}
        QVERIFY(dab(c,{43,38}));const auto revision=c.revision();const auto depth=c.undoDepth();
        QQuickWindow w;auto *canvas=attach(w,c);QVERIFY(canvas);QVERIFY(cpuReady(c,*canvas,w));
        if(qEnvironmentVariableIsSet("DRAWVERSE_G5_EXPECTED_DPR"))QCOMPARE(w.devicePixelRatio(),qreal(qEnvironmentVariableIntValue("DRAWVERSE_G5_EXPECTED_DPR")));
        const bool enabled=!software(w) && compositionDisplayRequested();
        if(enabled)QVERIFY2(gpuReady(c,*canvas),qPrintable(canvas->compositionStatistics()["fallback"].toString()));
        else {QTest::qWait(60);QCOMPARE(canvas->compositionStatistics()["adoptions"].toULongLong(),0);QVERIFY(!c.compositionPacket());}
        QCOMPARE(c.revision(),revision);QCOMPARE(c.undoDepth(),depth);QCOMPARE(canvas->compositionStatistics()["testReadbacks"].toInt(),0);
        auto report=QJsonObject::fromVariantMap(canvas->compositionStatistics());report["passed"]=true;report["gpu"]=enabled;report["dpr"]=w.devicePixelRatio();report["compute"]=c.computeDevice();
        save("policy-"+QString::fromLatin1(QTest::currentDataTag()),report);
    }
    void gpuDoesNotWaitForCpuDisplayReferences(){
        QTemporaryDir temp;PaintCoreClient c(nullptr,temp.filePath("prefs.ini"));QVERIFY(ready(c));QVERIFY(document(c,128));QVERIFY(dab(c,{43,38}));
        QQuickWindow w;w.resize(384,288);auto *canvas=new CanvasItem(w.contentItem());canvas->setSize({384,288});canvas->setClient(&c);
        // Quick synchronization blocks the GUI here. Delay only its display
        // references, preserving the authoritative core frame independently.
        const auto hold=connect(&w,&QQuickWindow::beforeSynchronizing,&w,[canvas]{canvas->m_image={};canvas->m_overview={};},Qt::DirectConnection);
        w.show();QVERIFY(QTest::qWaitForWindowExposed(&w));canvas->actualSize();
        if(software(w)){disconnect(hold);emit c.frameChanged();QVERIFY(cpuReady(c,*canvas,w));return;}
        QVERIFY(gpuReady(c,*canvas));const auto gpu=frame(w);disconnect(hold);
        qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit c.frameChanged();QVERIFY(cpuReady(c,*canvas,w));const int error=difference(gpu,frame(w));QVERIFY2(error<=1,qPrintable(QString::number(error)));
        save("no-cpu-display-wait",{{"passed",true},{"window_error",error},{"normal_readbacks",0}});
    }
    void blendHoverUsesCpuAndRestores(){
        QTemporaryDir temp;PaintCoreClient c(nullptr,temp.filePath("prefs.ini"));QVERIFY(ready(c));QVERIFY(document(c,128));
        QVERIFY(dab(c,{43,38},QColor("#d080c0")));c.addLayer("hover");QTRY_VERIFY(c.layers().size()==2 && !c.layerEditBusy());const auto target=c.activeLayer();QVERIFY(dab(c,{43,38},QColor("#35be72")));
        QQuickWindow w;auto *canvas=attach(w,c);QVERIFY(canvas);QVERIFY(cpuReady(c,*canvas,w));if(!software(w))QVERIFY(gpuReady(c,*canvas));
        const auto original=c.frame();const auto depth=c.undoDepth();const auto revision=c.revision();const auto before=software(w)?QImage():frame(w);
        c.previewLayerBlend(target,3);QTRY_VERIFY(c.frame()!=original && cpuReady(c,*canvas,w));QVERIFY(!c.compositionPacket());QVERIFY(c.compositionFallback().contains("hover"));
        const auto preview=c.frame();if(!software(w)){const auto screen=frame(w);QVERIFY(difference(screen,before)>1);qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit c.frameChanged();QVERIFY(difference(screen,frame(w))<=1);qunsetenv("DRAWVERSE_GPU_COMPOSITION");}
        c.clearLayerBlendPreview();QTRY_VERIFY(c.frame()==original && cpuReady(c,*canvas,w));if(!software(w))QVERIFY(gpuReady(c,*canvas));
        QCOMPARE(c.revision(),revision);QCOMPARE(c.undoDepth(),depth);QVERIFY(preview!=original);
        QVERIFY(c.setLayerBlend(target,3));QTRY_VERIFY(c.revision()>revision && !c.layerEditBusy());QVERIFY(cpuReady(c,*canvas,w));if(!software(w))QVERIFY(gpuReady(c,*canvas));
        QCOMPARE(c.frame(),preview);QCOMPARE(c.undoDepth(),depth+1);save("blend-hover",{{"passed",true},{"software",software(w)}});
    }
    void windowsAndDocumentsRejectOldCallbacks(){
        QTemporaryDir temp;std::vector<std::unique_ptr<PaintCoreClient>> clients;std::vector<std::unique_ptr<QQuickWindow>> windows;std::vector<CanvasItem*> canvases;
        for(int i=0;i<3;++i){
            clients.push_back(std::make_unique<PaintCoreClient>(nullptr,temp.filePath(QString::number(i)+".ini"),i?clients.front()->brushCatalogConnection():nullptr));
            auto &c=*clients.back();QVERIFY(ready(c));QVERIFY(document(c,128));QVERIFY(dab(c,{30.+i*20.,38},QColor::fromHsv(i*110,240,210)));
            windows.push_back(std::make_unique<QQuickWindow>());canvases.push_back(attach(*windows.back(),c));QVERIFY(canvases.back());QVERIFY(cpuReady(c,*canvases.back(),*windows.back()));
            QVERIFY(matchesCpu(c,*canvases.back(),*windows.back()));
        }
        for(int i=0;i<3;++i)for(int j=0;j<i;++j)QVERIFY(clients[i]->frame()!=clients[j]->frame());
        const bool sw=software(*windows[0]);QVERIFY(software(*windows[0]) || gpuReady(*clients[0],*canvases[0]));const auto old=clients[0]->compositionPacket();const auto uploads=canvases[0]->compositionStatistics()["uploads"].toULongLong();
        const auto epoch=canvases[0]->m_compositionEpoch;
        for(int i=0;i<3;++i)for(int j=0;j<i;++j)QVERIFY(canvases[i]->m_compositionEpoch!=canvases[j]->m_compositionEpoch);
        canvases[1]->setClient(nullptr);canvases[1]->setVisible(false);
        canvases[0]->setParentItem(windows[1]->contentItem());QVERIFY(canvases[0]->m_compositionEpoch!=epoch);
        QVERIFY(cpuReady(*clients[0],*canvases[0],*windows[1]));QVERIFY(matchesCpu(*clients[0],*canvases[0],*windows[1]));if(!sw)QVERIFY(gpuReady(*clients[0],*canvases[0]));
        if(!sw){
            QVERIFY(canvases[0]->compositionStatistics()["uploads"].toULongLong()>uploads);
            canvases[0]->compositionFallback("CPU: stale old window failure",old);canvases[0]->compositionFallback({},old);QVERIFY(!canvases[0]->m_compositionFailed);
        }
        canvases[2]->setClient(nullptr);canvases[2]->setVisible(false);const auto prior=clients[0]->compositionPacket();canvases[0]->setClient(clients[2].get());
        QVERIFY(cpuReady(*clients[2],*canvases[0],*windows[1]));QVERIFY(matchesCpu(*clients[2],*canvases[0],*windows[1]));if(!sw){QVERIFY(gpuReady(*clients[2],*canvases[0]));canvases[0]->compositionFallback("CPU: stale other client failure",prior);QVERIFY(!canvases[0]->m_compositionFailed);}
        for(auto &c:clients){QVERIFY(c->lastError().isEmpty());QCOMPARE(c->undoDepth(),1);}
        save("windows-documents",{{"passed",true},{"software",sw},{"dpr",windows[1]->devicePixelRatio()},{"normal_readbacks",0}});
        windows.clear();QTRY_COMPARE(CompositionNode::reservedPayloadBytes(),0);
    }
    void budgetRecovery_data(){QTest::addColumn<bool>("clipping");QTest::newRow("group")<<false;QTest::newRow("clipping")<<true;}
    void budgetRecovery(){
        QFETCH(bool,clipping);QTemporaryDir temp;PaintCoreClient c(nullptr,temp.filePath("prefs.ini"));QVERIFY(ready(c));QVERIFY(document(c,2048));const auto raster=c.activeLayer();quint64 extra=0;
        if(clipping){c.addLayer("clip");QTRY_VERIFY(c.layers().size()==2 && !c.layerEditBusy());extra=c.activeLayer();QVERIFY(c.setLayerClipping(extra,true));QTRY_VERIFY(!c.layerEditBusy() && c.layers().first().toMap()["clipped"].toBool());}
        else {QVERIFY(c.groupLayer(raster,"budget"));QTRY_VERIFY(c.layers().size()==2 && !c.layerEditBusy());extra=c.activeLayer();}
        QQuickWindow w;auto *canvas=attach(w,c);QVERIFY(canvas);canvas->fitToView();QVERIFY(cpuReady(c,*canvas,w));const bool sw=software(w);
        if(!sw){QTRY_VERIFY(canvas->compositionStatistics()["failed"].toBool());QVERIFY(canvas->compositionStatistics()["fallback"].toString().contains("budget"));}
        const auto failures=canvas->compositionStatistics()["failures"].toInt();c.selectLayer(raster);QTRY_VERIFY(c.activeLayer()==raster && !c.layerEditBusy());QVERIFY(dab(c,{1024,1024}));
        if(!sw){QTest::qWait(80);QCOMPARE(canvas->compositionStatistics()["failures"].toInt(),failures);}
        const auto generation=c.generation();const auto revision=c.revision();
        if(clipping){c.selectLayer(extra);QTRY_VERIFY(c.activeLayer()==extra && !c.layerEditBusy());c.removeLayer(extra);}
        else QVERIFY(c.ungroupLayer(extra));
        QTRY_VERIFY(c.revision()>revision && c.layers().size()==1 && !c.layerEditBusy());QCOMPARE(c.generation(),generation);
        if(!sw)QVERIFY2(gpuReady(c,*canvas),qPrintable(canvas->compositionStatistics()["fallback"].toString()));
        QVERIFY(cpuReady(c,*canvas,w));QVERIFY(matchesCpu(c,*canvas,w));
        // A new document must also clear a failed epoch, even with the same canvas.
        QVERIFY(document(c,2048));const auto id=c.activeLayer();QVERIFY(c.groupLayer(id,"again"));QTRY_VERIFY(!c.layerEditBusy() && c.layers().size()==2);
        if(!sw)QTRY_VERIFY(canvas->compositionStatistics()["failed"].toBool());
        QVERIFY(document(c,128));QVERIFY(cpuReady(c,*canvas,w));if(!sw)QVERIFY(gpuReady(c,*canvas));
        save("budget-"+QString::fromLatin1(QTest::currentDataTag()),QJsonObject::fromVariantMap(canvas->compositionStatistics()));
    }
    void processPressureRecoversAfterWindowCloses(){
        QTemporaryDir temp;std::vector<std::unique_ptr<PaintCoreClient>> clients;std::vector<std::unique_ptr<QQuickWindow>> windows;std::vector<CanvasItem*> canvases;int victim=-1;quint64 peak=0;
        for(int i=0;i<12;++i){
            clients.push_back(std::make_unique<PaintCoreClient>(nullptr,temp.filePath(QString::number(i)+".ini"),i?clients.front()->brushCatalogConnection():nullptr));
            auto &c=*clients.back();QVERIFY(ready(c));QVERIFY(document(c,128));QVERIFY(dab(c,{40,40},QColor::fromHsv(i*23,220,210)));
            windows.push_back(std::make_unique<QQuickWindow>());auto *canvas=attach(*windows.back(),c);QVERIFY(canvas);canvases.push_back(canvas);QVERIFY(cpuReady(c,*canvas,*windows.back()));
            if(software(*windows.back())){QCOMPARE(CompositionNode::reservedPayloadBytes(),0);save("process-pressure",{{"passed",true},{"software_cpu",true}});return;}
            QTRY_VERIFY(canvas->compositionStatistics()["adoptions"].toULongLong()>0 || canvas->compositionStatistics()["failed"].toBool());
            peak=std::max(peak,CompositionNode::reservedPayloadBytes());QVERIFY(peak<=2ull*1024*1024*1024);
            if(canvas->compositionStatistics()["failed"].toBool()){QVERIFY(canvas->compositionStatistics()["fallback"].toString().contains("process/in-flight"));victim=i;break;}
        }
        QVERIFY2(victim>0,"real process quota was not reached within the frozen 12-window bound");
        auto &c=*clients[size_t(victim)];auto *canvas=canvases[size_t(victim)];const auto original=c.frame();const auto revision=c.revision();const auto depth=c.undoDepth();const auto serial=CompositionNode::releasedPayloadSerial();
        QTest::qWait(80);QCOMPARE(canvas->compositionStatistics()["failures"].toInt(),1);
        windows[0].reset();QTRY_VERIFY(CompositionNode::releasedPayloadSerial()>serial);
        QVERIFY2(gpuReady(c,*canvas),qPrintable(canvas->compositionStatistics()["fallback"].toString()));
        QCOMPARE(c.revision(),revision);QCOMPARE(c.undoDepth(),depth);QCOMPARE(c.frame(),original);QCOMPARE(canvas->compositionStatistics()["testReadbacks"].toInt(),0);
        for(const auto &client:clients){QCOMPARE(client->undoDepth(),1);QVERIFY(client->lastError().isEmpty());}
        save("process-pressure",{{"passed",true},{"windows",victim+1},{"peak_reserved",double(peak)},{"retries",double(canvas->m_compositionRetries)},{"normal_readbacks",0}});
        windows.clear();QTRY_COMPARE(CompositionNode::reservedPayloadBytes(),0);
    }
    void faultRecovery_data(){QTest::addColumn<QByteArray>("fault");for(const auto *v:{"init","upload","device"})QTest::newRow(v)<<QByteArray(v);}
    void faultRecovery(){
        QFETCH(QByteArray,fault);QTemporaryDir temp;PaintCoreClient c(nullptr,temp.filePath("prefs.ini"));QVERIFY(ready(c));QVERIFY(document(c,128));
        qputenv("DRAWVERSE_COMPOSITION_FAIL",fault);QQuickWindow w;w.setPersistentGraphics(false);w.setPersistentSceneGraph(false);auto *canvas=attach(w,c);QVERIFY(canvas);QVERIFY(cpuReady(c,*canvas,w));
        const bool sw=software(w);if(!sw)QTRY_VERIFY(canvas->m_compositionFailed);QVERIFY(cpuReady(c,*canvas,w));QCOMPARE(canvas->zoom(),qreal(1));QCOMPARE(c.frame().size(),QSize(qCeil(128*w.devicePixelRatio()),qCeil(128*w.devicePixelRatio())));const auto epoch=canvas->m_compositionEpoch;const auto original=c.frame();const auto depth=c.undoDepth();
        w.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&w));canvas->forceActiveFocus();c.setBrushRadius(2);
        for(int i=0;i<25;++i){
            c.setBrushColor(QColor::fromHsv(i*47%360,240,200));const auto p=(canvas->documentRect().topLeft()+QPointF(10+i%5*22,10+i/5*22)*canvas->zoom()).toPoint();
            const auto before=c.undoDepth();QTest::mousePress(&w,Qt::LeftButton,Qt::NoModifier,p);QVERIFY(c.drawing());QTest::mouseMove(&w,p+QPoint(1,1));QTest::mouseRelease(&w,Qt::LeftButton,Qt::NoModifier,p+QPoint(1,1));QTRY_COMPARE(c.undoDepth(),before+1);
        }
        QVERIFY(cpuReady(c,*canvas,w));QVERIFY(c.frame()!=original);const auto final=c.frame();const auto revision=c.revision();QCOMPARE(c.undoDepth(),depth+25);
        if(!sw){QCOMPARE(canvas->compositionStatistics()["failures"].toInt(),1);QCOMPARE(canvas->m_compositionRetries,0);}
        qunsetenv("DRAWVERSE_COMPOSITION_FAIL");QSignalSpy invalidated(&w,&QQuickWindow::sceneGraphInvalidated);w.hide();w.releaseResources();if(!sw)QTRY_VERIFY(!invalidated.isEmpty());
        w.show();QVERIFY(QTest::qWaitForWindowExposed(&w));if(!sw)QTRY_VERIFY(canvas->m_compositionEpoch!=epoch);QVERIFY(cpuReady(c,*canvas,w));
        if(!sw){QVERIFY(gpuReady(c,*canvas));const auto gpu=frame(w);qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit c.frameChanged();QVERIFY(difference(gpu,frame(w))<=1);qunsetenv("DRAWVERSE_GPU_COMPOSITION");emit c.frameChanged();}
        QCOMPARE(c.revision(),revision);QCOMPARE(c.undoDepth(),depth+25);QCOMPARE(c.frame(),final);QCOMPARE(canvas->compositionStatistics()["testReadbacks"].toInt(),0);
        for(int i=0;i<25;++i){c.undo();QTRY_COMPARE(c.undoDepth(),depth+24-i);}QVERIFY(cpuReady(c,*canvas,w));QCOMPARE(c.frame(),original);QVERIFY(c.lastError().isEmpty());
        save("fault-"+QString::fromLatin1(QTest::currentDataTag()),{{"passed",true},{"synthetic",true},{"strokes",25},{"software",sw},{"new_epoch",canvas->m_compositionEpoch!=epoch},{"normal_readbacks",0}});
    }
    void faultDuringMove_data(){QTest::addColumn<bool>("commit");QTest::newRow("commit")<<true;QTest::newRow("cancel")<<false;}
    void faultDuringMove(){
        QFETCH(bool,commit);QTemporaryDir temp;PaintCoreClient c(nullptr,temp.filePath("prefs.ini"));QVERIFY(ready(c));QVERIFY(document(c,128));c.addLayer("move");QTRY_VERIFY(c.layers().size()==2 && !c.layerEditBusy());QVERIFY(dab(c,{43,38}));
        QQuickWindow w;auto *canvas=attach(w,c);QVERIFY(canvas);QVERIFY(cpuReady(c,*canvas,w));const bool sw=software(w);if(!sw)QVERIFY(gpuReady(c,*canvas));const auto original=c.frame();const auto depth=c.undoDepth();
        c.setMoveTool(true);QVERIFY(c.beginLayerMove({43,38},false));c.updateLayerMove({9,5});QTRY_VERIFY(!c.moveObservation()["starting"].toBool());
        if(!sw){QTRY_VERIFY(c.compositionPacket() && c.compositionPacket()->preview);QTRY_COMPARE(canvas->compositionStatistics()["nodeGeometry"].toULongLong(),c.compositionPacket()->geometrySequence);}
        qputenv("DRAWVERSE_COMPOSITION_FAIL","device");c.updateLayerMove({18,9});if(!sw){QVERIFY(c.compositionPacket());QCOMPARE(c.compositionPacket()->presentationDelta,QPoint(18,9));QTRY_VERIFY(canvas->m_compositionFailed);}
        c.finishLayerMove({18,9},commit);QTRY_VERIFY(!c.layerMoveActive() && !c.layerEditBusy());QTRY_COMPARE(c.compositionHandoffRevision(),quint64(0));QVERIFY(cpuReady(c,*canvas,w));QCOMPARE(c.undoDepth(),depth+int(commit));QCOMPARE(c.layers().first().toMap()["offsetX"].toInt(),commit?18:0);QCOMPARE(c.layers().first().toMap()["offsetY"].toInt(),commit?9:0);
        if(commit){QVERIFY(c.frame()!=original);c.undo();QTRY_COMPARE(c.undoDepth(),depth);QVERIFY(cpuReady(c,*canvas,w));}QCOMPARE(c.frame(),original);QVERIFY(c.lastError().isEmpty());
        save("move-fault-"+QString::fromLatin1(QTest::currentDataTag()),{{"passed",true},{"synthetic",true},{"commit",commit},{"handoff",double(c.compositionHandoffRevision())}});
    }
};
QTEST_MAIN(CompositionStabilityTests)
#include "composition_stability_tests.moc"
