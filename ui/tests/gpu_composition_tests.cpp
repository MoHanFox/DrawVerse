#include "platform/UiResources.h"
#include "canvas/CanvasItem.h"
#include "canvas/CompositionNode.h"
#include <QtTest>
#include <QtEndian>
#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QTemporaryDir>
#include <QFile>
#include <QJsonDocument>
#include <QScopeGuard>
#include <array>
#include <cmath>
#include <cstring>

namespace {
using Pixel=std::array<float,4>;
Pixel sample(const CompositionPacket &p,const PaintCompositionNode &node,int x,int y){
    const qint64 lx=qint64(x)-node.offset_x,ly=qint64(y)-node.offset_y;
    const int tx=int(std::floor(double(lx)/64)),ty=int(std::floor(double(ly)/64));
    for(quint32 i=node.first_tile;i<node.first_tile+node.tile_count;++i){
        if(p.tiles[int(i)].tile_x!=tx || p.tiles[int(i)].tile_y!=ty)continue;
        Pixel value{};const auto *at=p.data[int(i)]->bytes.constData()+((ly-qint64(ty)*64)*64+lx-qint64(tx)*64)*16;
        std::memcpy(value.data(),at,16);return value;
    }
    return lx>=0 && ly>=0 && lx<node.white_width && ly<node.white_height?Pixel{1,1,1,1}:Pixel{};
}
Pixel reference(const CompositionPacket &p,int x,int y){
    Pixel result{};
    for(const auto &node:p.nodes){
        if(node.kind!=0 || !node.visible)continue;
        auto value=sample(p,node,x,y);
        if(node.mask_index<quint32(p.nodes.size())){
            const auto &mask=p.nodes[int(node.mask_index)];
            if(mask.visible){const float coverage=1-sample(p,mask,x,y)[3]*mask.opacity;for(auto &c:value)c*=coverage;}
        }
        for(auto &c:value)c*=node.opacity*node.fill;
        for(int c=0;c<4;++c)result[c]=value[c]+result[c]*(1-value[3]);
    }
    return result;
}
int difference(const QImage &a,const QImage &b){
    if(a.size()!=b.size())return 256;
    int maximum=0;const auto x=a.convertToFormat(QImage::Format_RGBA8888),y=b.convertToFormat(QImage::Format_RGBA8888);
    for(int row=0;row<x.height();++row)for(int col=0;col<x.width()*4;++col)
        maximum=std::max(maximum,std::abs(int(x.constScanLine(row)[col])-int(y.constScanLine(row)[col])));
    return maximum;
}
void save(const QString &name,const QJsonObject &data){
    const auto directory=qEnvironmentVariable("DRAWVERSE_G2_REPORT_DIR");if(directory.isEmpty())return;
    QFile file(directory+"/"+name+".json");if(file.open(QIODevice::WriteOnly))file.write(QJsonDocument(data).toJson());
}
QSize requestedPixels(const CanvasItem &canvas,const QQuickWindow &window){
    const auto region=canvas.prefetchedDocumentRect();
    return {qCeil(region.width()*canvas.zoom()*window.devicePixelRatio()),qCeil(region.height()*canvas.zoom()*window.devicePixelRatio())};
}
}
class GpuCompositionTests final : public QObject {
    Q_OBJECT
private slots:
    void initTestCase(){initializeDrawVerseUiResources();qputenv("DRAWVERSE_GPU_COMPOSITION","1");}
    void documentPixels_data(){
        QTest::addColumn<bool>("white");QTest::addColumn<bool>("mask");QTest::addColumn<double>("zoom");QTest::addColumn<int>("lod");QTest::addColumn<int>("side");
        QTest::newRow("transparent-normal")<<false<<false<<1.<<0<<130;
        QTest::newRow("white-mask-offset")<<true<<true<<1.<<0<<130;
        QTest::newRow("transparent-mask-lod1")<<false<<true<<.5<<1<<130;
        QTest::newRow("white-mask-lod2")<<true<<true<<.25<<2<<130;
        QTest::newRow("partial-edge-lod6")<<true<<true<<.02<<6<<130;
        QTest::newRow("dense-1024-mask-lod2")<<true<<true<<.25<<2<<1024;
    }
    void documentPixels(){
        QFETCH(bool,white);QFETCH(bool,mask);QFETCH(double,zoom);QFETCH(int,lod);QFETCH(int,side);
        const int height=side==130?90:side;
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        if(white)client.newDocument(side,height);else client.newTransparentDocument(side,height);
        QTRY_VERIFY(client.ready() && client.documentWidth()==side && !client.layerEditBusy());
        if(white){
            client.activateEraser();client.setBrushRadius(6);InputSample erase;erase.position={105,75};
            const auto undo=client.undoDepth();QVERIFY(client.beginStroke(erase));client.endStroke();QTRY_COMPARE(client.undoDepth(),undo+1);client.activateBrush();
        }
        client.addLayer("G2 raster");QTRY_VERIFY(!client.layerEditBusy() && client.layers().size()==2);
        const auto raster=client.activeLayer();client.setBrushRadius(side==130?21:400);client.setBrushOpacity(.6);client.setBrushColor(QColor("#dd217e"));
        const auto beforeStroke=client.undoDepth();InputSample point;point.position=side==130?QPointF(36,28):QPointF(512,512);QVERIFY(client.beginStroke(point));client.endStroke();
        QTRY_COMPARE(client.undoDepth(),beforeStroke+1);
        const auto first=client.revision();QVERIFY(client.setLayerFill(raster,.7));QTRY_VERIFY(client.revision()>first && !client.layerEditBusy());
        const auto beforeOffset=client.revision();QVERIFY(client.moveLayer(raster,-7,3));QTRY_VERIFY(client.revision()>beforeOffset && !client.layerEditBusy());
        if(mask){
            QVERIFY(client.addMask(raster));QTRY_VERIFY(!client.layerEditBusy() && client.activeLayer()!=raster);
            client.setBrushColor(Qt::black);client.setBrushOpacity(.7);client.setBrushRadius(side==130?21:200);
            const auto undo=client.undoDepth();QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),undo+1);
            const auto revision=client.revision();client.setLayerProperties(client.activeLayer(),true,.45);QTRY_VERIFY(client.revision()>revision);
        }
        const auto revision=client.revision();const auto undo=client.undoDepth();
        QQuickWindow window;window.resize(512,384);window.setColor(Qt::black);
        auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({512,384});
        const auto readback=std::make_shared<CompositionReadback>();canvas->observeCompositionReadback(readback);
        canvas->setClient(&client);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        const auto targetZoom=zoom/window.devicePixelRatio();
        if(targetZoom!=1){canvas->zoomBy(targetZoom);QTRY_VERIFY(qAbs(canvas->zoom()-targetZoom)<1e-9);}
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        if(software){
            QTRY_VERIFY(client.frameRevision()==revision);QCOMPARE(canvas->compositionStatistics()["uploads"].toULongLong(),0);
            QCOMPARE(client.undoDepth(),undo);save(QString::fromLatin1(QTest::currentDataTag()),{{"passed",true},{"cpu_fallback",true}});return;
        }
        QCOMPARE(window.rendererInterface()->graphicsApi(),QSGRendererInterface::Direct3D11);
        QTRY_VERIFY_WITH_TIMEOUT(canvas->compositionStatistics()["nodeRevision"].toULongLong()==revision,15000);
        QVERIFY2(client.compositionPacket(),qPrintable(client.compositionFallback()));
        QTRY_COMPARE_WITH_TIMEOUT(client.compositionPacket()?client.compositionPacket()->lod:-1,lod,15000);
        QTRY_VERIFY_WITH_TIMEOUT(client.compositionPacket() && client.compositionPacket()->region==canvas->prefetchedDocumentRect() && client.compositionPacket()->pixels==requestedPixels(*canvas,window),15000);
        QTRY_COMPARE_WITH_TIMEOUT(canvas->compositionStatistics()["nodeRequest"].toULongLong(),client.compositionPacket()->info.key.request_id,15000);
        const auto packet=client.compositionPacket();QCOMPARE(packet->lod,lod);
        // Linear readback is independent of view/LOD: this immutable document
        // stays unchanged while navigation settles.
        if(readback->completed.load()!=2){window.update();}
        QTRY_COMPARE_WITH_TIMEOUT(readback->completed.load(),2,10000);
        double linearError=0;
        {std::lock_guard lock(readback->mutex);
            QCOMPARE(readback->linear.size(),side*height*16);
            for(int y=0;y<height;++y)for(int x=0;x<side;++x){
                Pixel actual{};std::memcpy(actual.data(),readback->linear.constData()+(y*side+x)*16,16);
                const auto expected=reference(*packet,x,y);
                for(int c=0;c<4;++c){QVERIFY(std::isfinite(actual[c]));linearError=std::max(linearError,double(std::abs(actual[c]-expected[c])));}
            }
        }
        QVERIFY2(linearError<=2e-5,qPrintable(QString::number(linearError,'g',12)));
        QTRY_VERIFY(client.frameRevision()==revision && client.frameRegion()==packet->region && client.frame().size()==packet->pixels);
        // Readback is test-only. A normal renderer never creates a QImage from it.
        // The settled display is independently checked against the real CPU window.
        QSignalSpy swapped(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!swapped.isEmpty());
        const auto gpu=window.grabWindow();const auto report=canvas->compositionStatistics();
        qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit client.frameChanged();
        swapped.clear();window.update();QTRY_VERIFY(!swapped.isEmpty());const auto cpu=window.grabWindow();
        const int windowError=difference(gpu,cpu);QVERIFY2(windowError<=1,qPrintable(QString::number(windowError)));
        QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),undo);
        QVERIFY(report["textureBytes"].toULongLong()<=256ull*1024*1024);
        QJsonObject data=QJsonObject::fromVariantMap(report);data["passed"]=true;data["linear_error"]=linearError;data["window_error"]=windowError;
        data["lod"]=lod;data["dpr"]=window.devicePixelRatio();data["api"]=int(window.rendererInterface()->graphicsApi());data["test_readbacks"]=2;data["normal_readbacks"]=0;
        save(QString::fromLatin1(QTest::currentDataTag()),data);
        const auto dir=qEnvironmentVariable("DRAWVERSE_G2_REPORT_DIR");if(!dir.isEmpty()){gpu.save(dir+"/"+QTest::currentDataTag()+"-gpu.png");cpu.save(dir+"/"+QTest::currentDataTag()+"-cpu.png");}
    }
    void continuousPaintingAndIncrementalUploads(){
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(256,192);
        QTRY_VERIFY(client.ready() && client.documentWidth()==256 && !client.layerEditBusy());
        QQuickWindow window;window.resize(512,384);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({512,384});canvas->setClient(&client);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();client.setBrushRadius(3);
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        for(int i=0;i<24;++i){
            const auto undo=client.undoDepth();client.setBrushColor(QColor::fromHsv((i*53)%360,220,220));
            const auto position=(canvas->documentRect().topLeft()+QPointF(14+(i%6)*36,14+(i/6)*42)).toPoint();
            QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,position);QTest::mouseMove(&window,position+QPoint(2,1));QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,position+QPoint(2,1));
            save("stroke-last-input",{{"index",i},{"undo_before",undo},{"revision",double(client.revision())},{"drawing",client.drawing()},{"error",client.lastError()},{"zoom",canvas->zoom()},{"x",position.x()},{"y",position.y()},{"active",window.isActive()},{"history_limit",client.historyLimit()}});
            QTRY_COMPARE(client.undoDepth(),undo+1);
        }
        if(!software)QTRY_COMPARE_WITH_TIMEOUT(canvas->compositionStatistics()["nodeRevision"].toULongLong(),client.revision(),15000);
        const auto before=canvas->compositionStatistics();
        for(int i=0;i<8;++i){QSignalSpy swapped(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!swapped.isEmpty());}
        QCOMPARE(canvas->compositionStatistics()["uploads"],before["uploads"]);QCOMPARE(canvas->compositionStatistics()["passes"],before["passes"]);
        const auto undo=client.undoDepth();client.setBrushColor(Qt::red);
        const auto at=(canvas->documentRect().topLeft()+QPointF(20,20)).toPoint();
        QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,at);QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,at);QTRY_COMPARE(client.undoDepth(),undo+1);
        if(!software){
            QTRY_COMPARE_WITH_TIMEOUT(canvas->compositionStatistics()["nodeRevision"].toULongLong(),client.revision(),15000);
            QCOMPARE(canvas->compositionStatistics()["uploads"].toULongLong()-before["uploads"].toULongLong(),1);
            QVERIFY(canvas->compositionStatistics()["lastDirtyArea"].toULongLong()>0);
            QVERIFY(canvas->compositionStatistics()["lastDirtyArea"].toULongLong()<=4096);
        }
        QCOMPARE(client.undoDepth(),25);QTRY_COMPARE(client.frameRevision(),client.revision());
        QSignalSpy swapped(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!swapped.isEmpty());const auto gpu=window.grabWindow();
        qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit client.frameChanged();swapped.clear();window.update();QTRY_VERIFY(!swapped.isEmpty());const auto cpu=window.grabWindow();
        QVERIFY(difference(gpu,cpu)<=1);QCOMPARE(canvas->compositionStatistics()["testReadbacks"].toInt(),0);
        auto data=QJsonObject::fromVariantMap(canvas->compositionStatistics());data["passed"]=true;data["strokes"]=25;data["window_error"]=difference(gpu,cpu);save("continuous",data);
    }
    void explicitFallback_data(){
        QTest::addColumn<QString>("failure");
        for(const auto *name:{"init","upload","extent","blend","fixed-mask"})QTest::newRow(name)<<QString::fromLatin1(name);
    }
    void compositionMoveReusesSource_data(){
        QTest::addColumn<bool>("mask");QTest::addColumn<bool>("pick");QTest::addColumn<bool>("commit");QTest::addColumn<bool>("quick");
        QTest::newRow("normal")<<false<<false<<true<<false;
        QTest::newRow("linked-mask")<<true<<false<<true<<false;
        QTest::newRow("pick-confirmed-target")<<false<<true<<true<<false;
        QTest::newRow("cancel")<<true<<false<<false<<false;
        QTest::newRow("quick-release")<<false<<true<<true<<true;
        QTest::newRow("pixels-and-disable-fallback")<<true<<false<<true<<false;
        QTest::newRow("final-transport-fallback")<<false<<false<<true<<false;
    }
    void compositionMoveReusesSource(){
        QFETCH(bool,mask);QFETCH(bool,pick);QFETCH(bool,commit);QFETCH(bool,quick);
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(128,96);
        QTRY_VERIFY(client.documentWidth()==128 && !client.layerEditBusy());const auto background=client.activeLayer();
        client.addLayer("moving");QTRY_VERIFY(!client.layerEditBusy() && client.activeLayer()!=background);const auto target=client.activeLayer();
        client.setBrushColor(Qt::red);client.setBrushRadius(9);InputSample sample;sample.position={36,28};
        const auto depth=client.undoDepth();QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+1);
        if(mask){
            QVERIFY(client.addMask(target));QTRY_VERIFY(!client.layerEditBusy() && client.activeLayer()!=target);
            client.setBrushColor(Qt::black);client.setBrushRadius(4);sample.position={33,26};const auto undo=client.undoDepth();
            QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),undo+1);
        }
        client.selectLayer(pick?background:target);QTRY_VERIFY(client.activeLayer()==(pick?background:target) && !client.layerEditBusy());
        QQuickWindow window;window.resize(384,288);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,288});canvas->setClient(&client);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();client.setMoveTool(true);
        QTRY_VERIFY(!client.frame().isNull() && client.frameRevision()==client.revision() && client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==requestedPixels(*canvas,window));
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        const bool disableDuringEdit=QString::fromLatin1(QTest::currentDataTag())=="pixels-and-disable-fallback";
        const bool finalTransport=QString::fromLatin1(QTest::currentDataTag())=="final-transport-fallback";
        const auto restore=qScopeGuard([]{qunsetenv("DRAWVERSE_COMPOSITION_FAIL");});
        if(!software)QTRY_COMPARE_WITH_TIMEOUT(canvas->compositionStatistics()["nodeRevision"].toULongLong(),client.revision(),15000);
        const auto original=client.frame();const auto undo=client.undoDepth();const auto uploads=canvas->compositionStatistics()["uploads"].toULongLong();
        const auto pipelines=canvas->compositionStatistics()["pipelineCreates"].toULongLong();
        QVERIFY(client.beginLayerMove({36,28},pick));client.updateLayerMove({7,4});client.updateLayerMove({19,11});
        if(quick)client.finishLayerMove({23,12},commit);
        if(!quick){
            QTRY_VERIFY(!client.moveObservation()["starting"].toBool());
            if(!software){
                QTRY_VERIFY(client.compositionPacket() && client.compositionPacket()->preview);
                QCOMPARE(client.compositionPacket()->presentationDelta,QPoint(19,11));
                const auto sequence=client.moveSequence();const auto source=client.compositionPacket()->info.key;
                for(const QPoint delta:{QPoint(-8,4),QPoint(),QPoint(23,12)}){
                    client.updateLayerMove(delta);QCOMPARE(client.compositionPacket()->presentationDelta,delta);
                    QCOMPARE(client.moveSequence(),sequence);QCOMPARE(client.compositionPacket()->info.key.revision,source.revision);
                }
                QTRY_COMPARE(canvas->compositionStatistics()["nodeGeometry"].toULongLong(),client.compositionPacket()->geometrySequence);
                QCOMPARE(canvas->compositionStatistics()["uploads"].toULongLong(),uploads);
                QCOMPARE(canvas->compositionStatistics()["pipelineCreates"].toULongLong(),pipelines);
                if(disableDuringEdit){
                    QTRY_VERIFY(!client.moveObservation()["dirty"].toBool() && client.moveObservation()["completedSequence"].toULongLong()>=client.moveSequence());
                    QTRY_COMPARE(client.frameRevision(),client.revision());
                    QSignalSpy swapped(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!swapped.isEmpty());const auto gpu=window.grabWindow();
                    qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit client.frameChanged();swapped.clear();window.update();QTRY_VERIFY(!swapped.isEmpty());
                    const auto cpu=window.grabWindow();QVERIFY2(difference(gpu,cpu)<=1,qPrintable(QString::number(difference(gpu,cpu))));
                }
                // Suppress only the primary request. Overview and close metadata
                // must not retire the source, even if windows keep presenting.
                if(!disableDuringEdit && !finalTransport){canvas->setVisible(false);client.requestViewport(0,{}, {},false);client.requestViewport(1,{0,0,128,96},{128,96});}
            }
            if(finalTransport)qputenv("DRAWVERSE_COMPOSITION_FAIL","transport");
            client.finishLayerMove({23,12},commit);
        }
        QTRY_VERIFY(!client.layerMoveActive() && !client.layerEditBusy());QCOMPARE(client.undoDepth(),undo+(commit?1:0));
        if(!software && !quick && !disableDuringEdit && !finalTransport){
            QTRY_COMPARE(client.frameRevision(1),client.revision());QVERIFY(client.compositionHandoffRevision()>0);
            QVERIFY(client.compositionPacket() && client.compositionPacket()->preview);
            const auto key=client.compositionPacket()->info.key;const auto final=client.compositionHandoffRevision();
            client.compositionPrimaryAdopted(key.document_generation,key.device_epoch,final-1);
            client.compositionPrimaryAdopted(key.document_generation,key.device_epoch+1,final);
            client.compositionPrimaryAdopted(key.document_generation+1,key.device_epoch,final);
            QCOMPARE(client.compositionHandoffRevision(),final);
            canvas->setVisible(true);
        }
        QTRY_VERIFY_WITH_TIMEOUT(client.compositionHandoffRevision()==0,15000);
        QTRY_COMPARE(client.frameRevision(),client.revision());
        if(commit){client.undo();QTRY_COMPARE(client.undoDepth(),undo);QTRY_COMPARE(client.frameRevision(),client.revision());}
        QCOMPARE(client.frame(),original);QVERIFY(client.lastError().isEmpty());
        auto report=QJsonObject::fromVariantMap(canvas->compositionStatistics());report["passed"]=true;report["pick"]=pick;report["quick"]=quick;report["commit"]=commit;
        save("move-"+QString::fromLatin1(QTest::currentDataTag()),report);
    }
    void compositionTransformReusesSource_data(){
        QTest::addColumn<int>("interpolation");QTest::addColumn<int>("shape");QTest::addColumn<bool>("commit");
        for(int k=0;k<6;++k){QTest::newRow(qPrintable(QString("affine-%1").arg(k)))<<k<<0<<true;QTest::newRow(qPrintable(QString("warp-%1").arg(k)))<<k<<1<<true;}
        QTest::newRow("perspective")<<2<<2<<true;
        QTest::newRow("local-undo-redo")<<2<<4<<true;
        QTest::newRow("fold")<<2<<5<<true;
        QTest::newRow("hard-edge-warp")<<2<<7<<true;
        QTest::newRow("dense-affine-1024")<<2<<8<<true;
        QTest::newRow("dense-warp-nearest-1024")<<0<<9<<true;
        QTest::newRow("dense-perspective-auto-1024")<<5<<10<<true;
        QTest::newRow("precision-capability-fallback")<<2<<11<<true;
        QTest::newRow("cancel")<<2<<1<<false;
        QTest::newRow("fast-release")<<2<<3<<true;
    }
    void compositionTransformReusesSource(){
        QFETCH(int,interpolation);QFETCH(int,shape);QFETCH(bool,commit);
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");
        if(shape==11)qputenv("DRAWVERSE_COMPOSITION_FAIL","precision");
        const auto restore=qScopeGuard([]{qunsetenv("DRAWVERSE_COMPOSITION_FAIL");});
        const int side=shape>=8 && shape<=10?1024:128,height=side==128?96:1024;
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(side,height);
        QTRY_VERIFY(client.documentWidth()==side && !client.layerEditBusy());client.addLayer("transforming");
        QTRY_VERIFY(!client.layerEditBusy() && client.layers().size()==2);
        if(shape==7 || side==1024){client.activateBucket();client.setBrushColor(Qt::blue);QVERIFY(client.bucketFill({0,0}));QTRY_VERIFY(!client.layerEditBusy());client.activateBrush();}
        client.setBrushColor(QColor("#ea2351"));client.setBrushRadius(13);client.setBrushOpacity(.7);InputSample sample;sample.position={42,38};
        auto depth=client.undoDepth();QVERIFY(client.beginStroke(sample));sample.position={66,51};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+1);
        if(side==1024){
            const auto path=QUrl::fromLocalFile(temp.filePath("dense-fixture.ora"));QVERIFY(client.saveDocument(path));QTRY_VERIFY(!client.fileBusy());
            const auto generation=client.generation();QVERIFY(client.openDocument(path));QTRY_VERIFY(!client.fileBusy() && client.generation()!=generation);QCOMPARE(client.undoDepth(),0);
            for(const auto &entry:client.layers()){const auto row=entry.toMap();if(row.value("name").toString()=="transforming")client.selectLayer(row.value("id").toULongLong());}
            QTRY_VERIFY(!client.layerEditBusy());
        }
        QQuickWindow window;window.resize(384,288);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,288});canvas->setClient(&client);
        const auto readback=std::make_shared<CompositionReadback>();canvas->observeCompositionReadback(readback);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        QTRY_VERIFY(client.frameRevision()==client.revision() && !client.frame().isNull() && client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==requestedPixels(*canvas,window));const auto original=client.frame();depth=client.undoDepth();
        if(!software){
            QTRY_VERIFY_WITH_TIMEOUT(canvas->compositionStatistics()["adoptions"].toULongLong()>0 && client.compositionPacket() && canvas->compositionStatistics()["nodeEpoch"].toULongLong()==client.compositionPacket()->info.key.device_epoch,15000);
            QTRY_COMPARE_WITH_TIMEOUT(canvas->compositionStatistics()["nodeRevision"].toULongLong(),client.revision(),15000);
            emit client.frameChanged();
        }
        const auto uploads=canvas->compositionStatistics()["uploads"].toULongLong();
        const auto pipelines=canvas->compositionStatistics()["pipelineCreates"].toULongLong();
        const bool precise=!software && canvas->compositionStatistics()["preciseTransform"].toInt()>0;
        if(shape==11 && !software)QVERIFY(!precise);
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy() && client.transformPoints().size()==16);
        if(precise){QVERIFY(client.compositionPacket());QVERIFY(client.compositionPacket()->preview);QVERIFY(client.compositionPacket()->transform);QVERIFY(!client.residentPreview());}
        auto points=client.transformPoints();const auto initial=points;
        for(int i=0;i<16;++i){const auto p=initial[i].toPointF();points[i]=QPointF(p.x()*1.12-p.y()*.17+12.3,p.x()*.13+p.y()*.91-3.1);}
        const bool warp=shape==1 || shape==5 || shape==7 || shape==9;const bool supported=precise;
        if(warp){points[5]=points[5].toPointF()+QPointF(9,-7);points[10]=points[10].toPointF()+QPointF(-8,6);}
        if(shape==5){points[5]=points[5].toPointF()+QPointF(65,35);points[10]=points[10].toPointF()+QPointF(-65,-35);}
        if(shape==2 || shape==10){points[3]=points[3].toPointF()+QPointF(-10,4);points[15]=points[15].toPointF()+QPointF(4,2);}
        const auto sourceRevision=precise?client.compositionPacket()->info.key.revision:0;
        const auto revision=client.revision();QVERIFY(client.updateTransform(points,warp,interpolation));
        if(supported){
            QVERIFY(client.compositionPacket()->transform);QCOMPARE(client.compositionPacket()->transform->geometry.interpolation,quint32(interpolation));
            QCOMPARE(client.compositionPacket()->info.key.revision,sourceRevision);
        }
        QByteArray previewLinear;
        if(shape!=3){
            QTRY_VERIFY(client.revision()>revision && client.frameRevision()==client.revision());
            if(supported){
                QTRY_COMPARE(canvas->compositionStatistics()["nodeGeometry"].toULongLong(),client.compositionPacket()->geometrySequence);
                QCOMPARE(canvas->compositionStatistics()["uploads"].toULongLong(),uploads);
                QCOMPARE(canvas->compositionStatistics()["pipelineCreates"].toULongLong(),pipelines);
                QTRY_COMPARE(readback->completed.load(),2);readback->completed=0;readback->geometry=client.compositionPacket()->geometrySequence;readback->version=1;window.update();
                const bool captured=QTest::qWaitFor([&]{window.update();return readback->completed.load()==2;},2000);
                if(!captured){auto diagnostic=QJsonObject::fromVariantMap(canvas->compositionStatistics());diagnostic["requestedGeometry"]=double(readback->geometry.load());diagnostic["clientGeometry"]=double(client.compositionPacket()->geometrySequence);save("readback-failure-"+QString::fromLatin1(QTest::currentDataTag()),diagnostic);}
                QVERIFY(captured);{std::lock_guard lock(readback->mutex);previewLinear=readback->linear;}
                QSignalSpy swapped(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!swapped.isEmpty());const auto gpu=window.grabWindow();
                qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit client.frameChanged();swapped.clear();window.update();QTRY_VERIFY(!swapped.isEmpty());const auto cpu=window.grabWindow();
                const int error=difference(gpu,cpu);
                const auto dir=qEnvironmentVariable("DRAWVERSE_G2_REPORT_DIR");if(!dir.isEmpty()){gpu.save(dir+"/transform-"+QTest::currentDataTag()+"-gpu.png");cpu.save(dir+"/transform-"+QTest::currentDataTag()+"-cpu.png");}
                QVERIFY2(error<=1,qPrintable(QString("window error %1").arg(error)));
                qputenv("DRAWVERSE_GPU_COMPOSITION","1");emit client.frameChanged();
                save("transform-pixels-"+QString::fromLatin1(QTest::currentDataTag()),{{"passed",true},{"window_error",error},{"source_upload_delta",0}});
            }
        }
        if(shape==4){
            QVERIFY(client.checkpointTransform());QTRY_VERIFY(!client.transformBusy());client.undo();QTRY_VERIFY(!client.transformBusy() && client.transformPoints()==initial);
            client.redo();QTRY_VERIFY(!client.transformBusy() && client.transformPoints()==points);
            if(!software)QCOMPARE(client.compositionPacket()->transform->geometry.points[0],points[0].toPointF().x());
        }
        QVERIFY(client.finishTransform(commit));QTRY_VERIFY_WITH_TIMEOUT(!client.transformActive() && !client.transformBusy(),15000);
        QTRY_VERIFY_WITH_TIMEOUT(client.compositionHandoffRevision()==0,15000);QTRY_COMPARE(client.frameRevision(),client.revision());
        QCOMPARE(client.undoDepth(),depth+(commit?1:0));
        if(supported && commit && !previewLinear.isEmpty()){
            QTRY_VERIFY(client.compositionPacket() && !client.compositionPacket()->preview && client.compositionPacket()->info.key.revision==client.revision());
            const auto final=client.compositionPacket();QCOMPARE(previewLinear.size(),side*height*16);double error=0;
            for(int y=0;y<height;++y)for(int x=0;x<side;++x){Pixel actual{};std::memcpy(actual.data(),previewLinear.constData()+(y*side+x)*16,16);const auto expected=reference(*final,x,y);for(int c=0;c<4;++c){QVERIFY(std::isfinite(actual[c]));error=std::max(error,double(std::abs(actual[c]-expected[c])));}}
            save("transform-linear-"+QString::fromLatin1(QTest::currentDataTag()),{{"linear_error",error},{"passed",error<=2e-5}});
            QVERIFY2(error<=2e-5,qPrintable(QString("linear error %1").arg(error,0,'g',12)));
        }
        if(commit){client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_COMPARE(client.frameRevision(),client.revision());}
        QCOMPARE(client.frame(),original);QVERIFY(client.lastError().isEmpty());
        save("transform-"+QString::fromLatin1(QTest::currentDataTag()),{{"passed",true},{"software_fallback",software},{"commit",commit},{"precise_capability",precise},{"side",side}});
    }
    void transformCloseBeforeTarget_data(){QTest::addColumn<bool>("commit");QTest::newRow("commit")<<true;QTest::newRow("cancel")<<false;}
    void transformCloseBeforeTarget(){
        QFETCH(bool,commit);qputenv("DRAWVERSE_GPU_COMPOSITION","1");QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(128,96);QTRY_VERIFY(!client.layerEditBusy());
        QQuickWindow window;window.resize(384,288);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,288});canvas->setClient(&client);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        QTRY_VERIFY(client.frameRevision()==client.revision() && client.frame().size()==requestedPixels(*canvas,window));
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        if(!software){QTRY_VERIFY(client.compositionPacket() && canvas->compositionStatistics()["nodeEpoch"].toULongLong()==client.compositionPacket()->info.key.device_epoch);emit client.frameChanged();}
        const auto original=client.frame();const auto depth=client.undoDepth();
        QVERIFY(client.beginTransform());QVERIFY(client.finishTransform(commit));
        QTRY_VERIFY(!client.transformActive() && !client.transformBusy());QTRY_VERIFY_WITH_TIMEOUT(client.compositionHandoffRevision()==0,15000);
        QTRY_COMPARE(client.frameRevision(),client.revision());QCOMPARE(client.frame(),original);QCOMPARE(client.undoDepth(),depth);QVERIFY(client.lastError().isEmpty());
    }
    void documentClientEpochRejectsOldHandoff(){
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");QTemporaryDir temp;PaintCoreClient a(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY2(a.ready() && a.brushLibraryReady(),qPrintable(a.lastError()));
        PaintCoreClient b(nullptr,temp.filePath("prefs.ini"),a.brushCatalogConnection());
        for(auto *client:{&a,&b}){QTRY_VERIFY2(client->ready() && client->brushLibraryReady(),qPrintable(client->lastError()));client->newDocument(128,96);QTRY_VERIFY(client->documentWidth()==128 && client->documentHeight()==96 && !client->layerEditBusy());}
        QCOMPARE(a.generation(),b.generation());QQuickWindow window;window.resize(384,288);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,288});
        canvas->setClient(&a);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        if(window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software)return;
        QTRY_VERIFY(a.compositionPacket() && canvas->compositionStatistics()["nodeEpoch"].toULongLong()==a.compositionPacket()->info.key.device_epoch);
        const auto old=a.compositionPacket()->info.key;canvas->setClient(&b);
        QTRY_VERIFY(b.compositionPacket() && canvas->compositionStatistics()["nodeEpoch"].toULongLong()==b.compositionPacket()->info.key.device_epoch);
        QVERIFY(old.device_epoch!=b.compositionPacket()->info.key.device_epoch);
        b.setMoveTool(true);QVERIFY(b.beginLayerMove({32,32},false));b.updateLayerMove({8,4});QTRY_VERIFY(!b.moveObservation()["starting"].toBool());
        canvas->setVisible(false);b.finishLayerMove({8,4},true);QTRY_VERIFY(!b.layerMoveActive() && b.compositionHandoffRevision()>0);
        const auto final=b.compositionHandoffRevision();b.compositionPrimaryAdopted(old.document_generation,old.device_epoch,final+100);
        QCOMPARE(b.compositionHandoffRevision(),final);canvas->setVisible(true);
        QTRY_VERIFY_WITH_TIMEOUT(b.compositionHandoffRevision()==0,15000);QVERIFY(b.lastError().isEmpty());
    }
    void windowResourceRelease(){
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");qunsetenv("DRAWVERSE_COMPOSITION_FAIL");
        QTRY_COMPARE(CompositionNode::reservedPayloadBytes(),0);
        for(int i=0;i<3;++i){
            {
                QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
                QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(128,128);
                QTRY_VERIFY(client.documentWidth()==128 && !client.layerEditBusy());
                QQuickWindow window;window.resize(256,256);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({256,256});canvas->setClient(&client);
                window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));
                if(window.rendererInterface()->graphicsApi()!=QSGRendererInterface::Software){
                    QTRY_VERIFY_WITH_TIMEOUT(canvas->compositionStatistics()["adoptions"].toULongLong()>0,15000);
                    QVERIFY(CompositionNode::reservedPayloadBytes()>0);
                    QVERIFY(CompositionNode::reservedPayloadBytes()<=2ull*1024*1024*1024);
                }
            }
            QTRY_COMPARE(CompositionNode::reservedPayloadBytes(),0);
        }
    }
    void explicitFallback(){
        QFETCH(QString,failure);qputenv("DRAWVERSE_GPU_COMPOSITION","0");qunsetenv("DRAWVERSE_COMPOSITION_FAIL");
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        const int side=failure=="extent"?4096:128;client.newDocument(side,side);QTRY_VERIFY(client.ready() && client.documentWidth()==side && !client.layerEditBusy());
        if(failure=="blend"){
            const auto revision=client.revision();QVERIFY(client.setLayerBlend(client.activeLayer(),1));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
        }
        if(failure=="fixed-mask"){
            const auto raster=client.activeLayer();QVERIFY(client.addMask(raster));QTRY_VERIFY(client.activeLayer()!=raster && !client.layerEditBusy());
            const auto revision=client.revision();QVERIFY(client.setMaskLinked(client.activeLayer(),false));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
        }
        QQuickWindow window;window.resize(384,384);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,384});canvas->setClient(&client);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));
        QTRY_VERIFY(!client.frame().isNull() && client.frameRevision()==client.revision() && client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==requestedPixels(*canvas,window));
        const auto revision=client.revision();const auto undo=client.undoDepth();const auto reference=client.frame();
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        if(failure=="init" || failure=="upload")qputenv("DRAWVERSE_COMPOSITION_FAIL",failure.toUtf8());
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");emit client.frameChanged();
        if(!software){
            if(failure=="init" || failure=="upload")QTRY_COMPARE_WITH_TIMEOUT(canvas->compositionStatistics()["failures"].toInt(),1,15000);
            else QTRY_VERIFY_WITH_TIMEOUT(!client.compositionFallback().isEmpty() && client.compositionFallback().contains(failure=="extent"?"extent":"transport"),15000);
        }
        QTest::qWait(80);QCOMPARE(canvas->compositionStatistics()["adoptions"].toULongLong(),0);
        QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),undo);QCOMPARE(client.frame(),reference);
        client.setBrushRadius(4);client.setBrushColor(Qt::black);
        InputSample input;input.position={double(side)/2,double(side)/2};
        QVERIFY(client.beginStroke(input));client.strokeTo(input);client.endStroke();
        QTRY_COMPARE(client.undoDepth(),undo+1);
        QTRY_VERIFY(client.frameRevision()==client.revision() && client.frame()!=reference);
        client.undo();QTRY_COMPARE(client.undoDepth(),undo);
        QTRY_VERIFY(client.frameRevision()==client.revision());QCOMPARE(client.frame(),reference);
        auto report=QJsonObject::fromVariantMap(canvas->compositionStatistics());report["passed"]=true;report["injected"]=failure;save("fallback-"+failure,report);
        qunsetenv("DRAWVERSE_COMPOSITION_FAIL");
    }
};
QTEST_MAIN(GpuCompositionTests)
#include "gpu_composition_tests.moc"
