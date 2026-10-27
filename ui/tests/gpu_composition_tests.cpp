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
