#include "platform/UiResources.h"
#include "canvas/CanvasItem.h"
#include "canvas/CanvasPresentationObservation.h"
#include "canvas/CompositionNode.h"
#include <QtTest>
#include <QtEndian>
#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QTemporaryDir>
#include <QFile>
#include <QDir>
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
bool writeLinearCapture(const CompositionPacket &packet,const QByteArray &linear,const QString &name){
    if(linear.size()!=96*64*16)return false;
    QByteArray capture("G4L1",4);const auto u32=[&](quint32 v){v=qToLittleEndian(v);capture.append(reinterpret_cast<const char*>(&v),4);};
    const auto f32=[&](float v){quint32 bits;std::memcpy(&bits,&v,4);u32(bits);};
    u32(96);u32(64);quint32 count=0;for(const auto &n:packet.nodes)if(n.kind==0 && n.visible)++count;u32(count);
    for(const auto &node:packet.nodes)if(node.kind==0 && node.visible){
        const auto *m=node.mask_index<quint32(packet.nodes.size())?&packet.nodes[int(node.mask_index)]:nullptr;
        u32(quint32(node.reserved));f32(node.opacity);f32(node.fill);f32(m && m->visible?m->opacity:0);
        for(int y=0;y<64;++y)for(int x=0;x<96;++x){const auto s=sample(packet,node,x,y);for(float c:s)f32(c);f32(m && m->visible?sample(packet,*m,x,y)[3]:0);}
    }
    capture.append(linear);
    QFile file(qEnvironmentVariable("DRAWVERSE_G2_REPORT_DIR")+"/"+name+".g4linear");
    return file.open(QIODevice::WriteOnly) && file.write(capture)==capture.size();
}
bool writeGroupCapture(const CompositionPacket &packet,const QByteArray &linear,const QString &name){
    if(linear.size()!=96*64*16 || packet.operations.isEmpty())return false;
    QByteArray capture("G4G1",4);const auto u32=[&](quint32 v){v=qToLittleEndian(v);capture.append(reinterpret_cast<const char*>(&v),4);};
    const auto f32=[&](float v){quint32 bits;std::memcpy(&bits,&v,4);u32(bits);};
    u32(96);u32(64);u32(quint32(packet.nodes.size()));u32(quint32(packet.operations.size()));u32(quint32(packet.surfaceCount));u32(quint32(packet.outputSurface));
    for(const auto &node:packet.nodes){
        u32(node.kind);u32(quint32(node.reserved));u32(node.mask_index);u32(node.visible);f32(node.opacity);f32(node.fill);
        for(int y=0;y<64;++y)for(int x=0;x<96;++x)for(float c:sample(packet,node,x,y))f32(c);
    }
    for(const auto &op:packet.operations){u32(op.kind);u32(op.node_index);u32(op.output);u32(op.source);u32(op.backdrop);}
    capture.append(linear);QFile file(qEnvironmentVariable("DRAWVERSE_G2_REPORT_DIR")+"/group-"+name+".g4group");
    return file.open(QIODevice::WriteOnly) && file.write(capture)==capture.size();
}
QSize requestedPixels(const CanvasItem &canvas,const QQuickWindow &window){
    const auto region=canvas.prefetchedDocumentRect();
    return {qCeil(region.width()*canvas.zoom()*window.devicePixelRatio()),qCeil(region.height()*canvas.zoom()*window.devicePixelRatio())};
}
}
class GpuCompositionTests final : public QObject {
    Q_OBJECT
private slots:
    void initTestCase(){initializeDrawVerseUiResources();qputenv("DRAWVERSE_GPU_COMPOSITION","1");qputenv("DRAWVERSE_CPU_DISPLAY_REFERENCE","1");}
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
        canvas->setClient(&client);window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->actualSize();
        const auto targetZoom=zoom/window.devicePixelRatio();
        const auto retainZoom=qScopeGuard([&]{save("zoom-"+QString::fromLatin1(QTest::currentDataTag()),{{"target",targetZoom},{"actual",canvas->zoom()},{"active",window.isActive()}});});
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
        readback->request(*packet);window.update();
        QTRY_VERIFY_WITH_TIMEOUT((window.update(),readback->completed.load()==2),10000);
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
    void busyCompositionKeepsGpuAndRetriesSameRevision_data(){
        QTest::addColumn<QByteArray>("failure");
        QTest::newRow("request")<<QByteArray("busy");
        QTest::newRow("read")<<QByteArray("busy-read");
    }
    void busyCompositionKeepsGpuAndRetriesSameRevision(){
        QFETCH(QByteArray,failure);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY(client.ready());client.newDocument(256,192);
        QTRY_VERIFY(client.ready() && client.documentWidth()==256);
        QQuickWindow window;window.resize(512,384);auto *canvas=new CanvasItem(window.contentItem());
        canvas->setSize({512,384});canvas->setClient(&client);window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        if(window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software)QSKIP("D3D11 required");
        QTRY_VERIFY(client.compositionPacket());QTRY_VERIFY(canvas->compositionStatistics()["nodeRequest"].toULongLong()>0);
        const auto revision=client.revision();const auto original=client.compositionPacket();
        const auto restore=qScopeGuard([]{qunsetenv("DRAWVERSE_COMPOSITION_FAIL");});
        qputenv("DRAWVERSE_COMPOSITION_FAIL",failure);canvas->zoomBy(1.2);
        QTest::qWait(350);
        QCOMPARE(client.compositionPacket(),original);
        QVERIFY2(client.compositionFallback().isEmpty(),qPrintable(client.compositionFallback()));
        qunsetenv("DRAWVERSE_COMPOSITION_FAIL");
        QTRY_VERIFY(client.compositionPacket() && client.compositionPacket()!=original);
        QCOMPARE(client.revision(),revision);
        QTRY_VERIFY(client.compositionPacket()->pixels==requestedPixels(*canvas,window));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void paintedPixelsRemainVisibleDuringNavigation(){
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");
        qputenv("DRAWVERSE_CPU_DISPLAY_REFERENCE","0");
        const auto restore=qScopeGuard([]{qputenv("DRAWVERSE_CPU_DISPLAY_REFERENCE","1");});
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(256,192);
        QTRY_VERIFY(client.ready() && client.documentWidth()==256 && !client.layerEditBusy());
        client.setBrushColor(Qt::red);client.setBrushRadius(35);client.brushLibrary()->setPressureSize(false);client.brushLibrary()->setPressureFlow(false);
        InputSample input;input.position={128,96};const auto depth=client.undoDepth();
        QVERIFY(client.beginStroke(input));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+1);
        QQuickWindow window;window.resize(512,384);auto *canvas=new CanvasItem(window.contentItem());
        canvas->setSize({512,384});canvas->setClient(&client);window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        if(window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software)QSKIP("D3D11 required");
        QTRY_COMPARE(canvas->compositionStatistics()["nodeRevision"].toULongLong(),client.revision());
        QTRY_VERIFY(canvas->compositionStatistics()["adoptions"].toULongLong()>0);
        for(int i=0;i<6;++i){
            canvas->zoomBy(i%2?1/1.15:1.15);
            if(i%3==0){const auto before=client.undoDepth();input.position={30.,30.+i};QVERIFY(client.beginStroke(input));client.endStroke();QTRY_COMPARE(client.undoDepth(),before+1);}
            for(int frame=0;frame<3;++frame){
                const auto shot=window.grabWindow();QVERIFY(!shot.isNull());
                const auto color=shot.pixelColor(shot.width()/2,shot.height()/2);
                QVERIFY2(color.red()>240 && color.green()<20 && color.blue()<20,qPrintable(QString("frame %1/%2 center %3").arg(i).arg(frame).arg(color.name())));
                QTest::qWait(16);
            }
        }
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void zoomKeepsCompositionPresentation(){
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY(client.ready());client.newDocument(256,192);
        QTRY_VERIFY(client.ready() && client.documentWidth()==256);
        QQuickWindow window;window.resize(512,384);
        auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({512,384});canvas->setClient(&client);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        if(window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software)QSKIP("D3D11 required");
        QTRY_COMPARE(canvas->compositionStatistics()["nodeRevision"].toULongLong(),client.revision());
        QTRY_VERIFY(canvas->compositionStatistics()["adoptions"].toULongLong()>0);
        QTRY_VERIFY(canvas->compositionStatistics()["nodeRequest"].toULongLong()>0);
        auto observed=std::make_shared<CanvasPresentationObservation>(512);canvas->observePresentation(observed);
        for(int i=0;i<6;++i){
            canvas->zoomBy(i%2?1/1.1:1.1);
            QTest::qWait(60);
            QTRY_VERIFY(client.compositionPacket() && client.compositionPacket()->pixels==requestedPixels(*canvas,window));
            window.update();QTest::qWait(30);
        }
        const auto samples=observed->samples();QVERIFY(samples.size()>2);
        for(const auto &sample:samples)QVERIFY2(sample.identity.valid && sample.identity.composition,"zoom exposed a CPU/empty presentation");
        QCOMPARE(observed->overflow(),quint64(0));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void continuousPaintingAndIncrementalUploads(){
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(256,192);
        QTRY_VERIFY(client.ready() && client.documentWidth()==256 && !client.layerEditBusy());
        QQuickWindow window;window.resize(512,384);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({512,384});canvas->setClient(&client);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->actualSize();canvas->forceActiveFocus();client.setBrushRadius(3);
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        for(int i=0;i<24;++i){
            QVERIFY(window.isActive());
            const auto undo=client.undoDepth();client.setBrushColor(QColor::fromHsv((i*53)%360,220,220));
            const auto position=(canvas->documentRect().topLeft()+QPointF(14+(i%6)*36,14+(i/6)*42)).toPoint();
            QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,position);QVERIFY(client.drawing());QTest::mouseMove(&window,position+QPoint(2,1));QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,position+QPoint(2,1));
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
    void nonNormalTransformUsesCpuFrame_data(){
        QTest::addColumn<bool>("grouped");QTest::newRow("root-blend")<<false;QTest::newRow("group-closure")<<true;
    }
    void nonNormalTransformUsesCpuFrame(){
        QFETCH(bool,grouped);
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());const auto generation=client.generation();client.newDocument(128,96);
        QTRY_VERIFY(client.generation()!=generation && !client.layerEditBusy());client.addLayer("blend transform");QTRY_VERIFY(client.layers().size()==2 && !client.layerEditBusy());
        client.setBrushColor(Qt::red);client.setBrushRadius(12);auto depth=client.undoDepth();QVERIFY(client.beginStroke({{38,34},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+1);
        auto revision=client.revision();QVERIFY(client.setLayerBlend(client.activeLayer(),3));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
        if(grouped){revision=client.revision();QVERIFY(client.groupLayer(client.activeLayer(),"CPU transform closure"));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());}
        QQuickWindow window;window.resize(384,288);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,288});canvas->setClient(&client);canvas->setAcceptedMouseButtons(Qt::NoButton);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        QTRY_VERIFY(client.frameRevision()==client.revision() && client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==requestedPixels(*canvas,window));const auto original=client.frame();depth=client.undoDepth();
        if(!software)QTRY_VERIFY(client.compositionPacket() && canvas->compositionStatistics()["nodeRevision"].toULongLong()==client.revision());
        QVERIFY(client.beginTransform());QVERIFY(!client.compositionPacket());QTRY_VERIFY(client.transformActive() && !client.transformBusy());
        QVERIFY(!client.compositionPacket());QVERIFY(!client.residentPreview());
        auto points=client.transformPoints();for(int i=0;i<16;++i)points[i]=points[i].toPointF()+QPointF(18,9);
        revision=client.revision();QVERIFY(client.updateTransform(points,false,2));QTRY_VERIFY(client.revision()>revision && client.frameRevision()==client.revision());
        QVERIFY(client.frame()!=original);QVERIFY(!client.compositionPacket());
        QSignalSpy swapped(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!swapped.isEmpty());const auto shown=window.grabWindow();
        qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit client.frameChanged();swapped.clear();window.update();QTRY_VERIFY(!swapped.isEmpty());QVERIFY(difference(shown,window.grabWindow())<=1);
        QVERIFY(client.finishTransform(true));QTRY_VERIFY(!client.transformActive() && !client.transformBusy());QCOMPARE(client.undoDepth(),depth+1);
        client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_COMPARE(client.frameRevision(),client.revision());QCOMPARE(client.frame(),original);
        save("non-normal-transform-"+QString::fromLatin1(QTest::currentDataTag()),{{"passed",true},{"cpu_fallback",true}});
    }
    void rootBlendAndFixedMask_data(){
        QTest::addColumn<int>("mode");QTest::addColumn<bool>("fixed");
        for(int mode=0;mode<27;++mode)if(mode!=1)for(bool fixed:{false,true})
            QTest::newRow(qPrintable(QString("mode-%1-%2").arg(mode).arg(fixed?"fixed-white-lod1":"linked-transparent")))<<mode<<fixed;
    }
    void rootBlendAndFixedMask(){
        QFETCH(int,mode);QFETCH(bool,fixed);qputenv("DRAWVERSE_GPU_COMPOSITION","1");
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        const auto generation=client.generation();if(fixed)client.newDocument(96,64);else client.newTransparentDocument(96,64);
        QTRY_VERIFY(client.generation()!=generation && client.documentWidth()==96 && !client.layerEditBusy());
        client.setBrushColor(QColor("#32b366"));client.setBrushRadius(23);client.setBrushOpacity(.73);
        auto depth=client.undoDepth();QVERIFY(client.beginStroke({{39,30},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+1);
        client.addLayer("G4 blend");QTRY_VERIFY(client.layers().size()==2 && !client.layerEditBusy());const auto target=client.activeLayer();
        client.setBrushColor(QColor("#e61999"));client.setBrushRadius(26);client.setBrushOpacity(.81);
        depth=client.undoDepth();QVERIFY(client.beginStroke({{48,34},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+1);
        QVERIFY(client.addMask(target));QTRY_VERIFY(client.layers().size()==3 && !client.layerEditBusy());const auto mask=client.activeLayer();
        client.setBrushColor(Qt::black);client.setBrushRadius(17);client.setBrushOpacity(.8);
        depth=client.undoDepth();QVERIFY(client.beginStroke({{43,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+1);
        auto revision=client.revision();client.setLayerProperties(mask,true,.63);QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
        if(fixed){revision=client.revision();QVERIFY(client.setMaskLinked(mask,false));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());}
        revision=client.revision();QVERIFY(client.moveLayer(target,-7,3));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
        revision=client.revision();QVERIFY(client.setLayerFill(target,.71));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
        revision=client.revision();client.setLayerProperties(target,true,.61);QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
        if(mode){revision=client.revision();QVERIFY(client.setLayerBlend(target,mode));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());}
        QQuickWindow window;window.resize(384,288);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,288});
        auto readback=std::make_shared<CompositionReadback>();canvas->observeCompositionReadback(readback);canvas->setClient(&client);canvas->setAcceptedMouseButtons(Qt::NoButton);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->actualSize();
        const auto state=qScopeGuard([&]{save("blend-state-"+QString::fromLatin1(QTest::currentDataTag()),{{"active",window.isActive()},{"drawing",client.drawing()},{"busy",client.layerEditBusy()},{"revision",double(client.revision())},{"error",client.lastError()},{"zoom",canvas->zoom()}});});
        if(fixed){const auto zoom=.5/window.devicePixelRatio();canvas->setSize({64+96*zoom,64+64*zoom});canvas->fitToView();QTRY_VERIFY(qAbs(canvas->zoom()-zoom)<1e-9);}
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        QTRY_VERIFY(client.frameRevision()==client.revision() && client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==requestedPixels(*canvas,window));
        if(software){save("blend-"+QString::fromLatin1(QTest::currentDataTag()),{{"passed",true},{"cpu_fallback",true}});return;}
        QTRY_VERIFY_WITH_TIMEOUT(client.compositionPacket() && canvas->compositionStatistics()["nodeRevision"].toULongLong()==client.revision(),15000);
        QTRY_VERIFY(client.compositionPacket()->region==canvas->prefetchedDocumentRect() && client.compositionPacket()->pixels==requestedPixels(*canvas,window));
        QTRY_COMPARE(canvas->compositionStatistics()["nodeRequest"].toULongLong(),client.compositionPacket()->info.key.request_id);
        readback->request(*client.compositionPacket());window.update();QTRY_VERIFY((window.update(),readback->completed.load()==2));
        const auto packet=client.compositionPacket();QCOMPARE(packet->lod,fixed?1:0);
        const auto directory=qEnvironmentVariable("DRAWVERSE_G2_REPORT_DIR");QVERIFY(!directory.isEmpty());
        {std::lock_guard lock(readback->mutex);QVERIFY(writeLinearCapture(*packet,readback->linear,QString::fromLatin1(QTest::currentDataTag())));}
        // Parameter-only edits invalidate composition but keep immutable source IDs.
        const auto uploads=canvas->compositionStatistics()["uploads"].toULongLong();
        revision=client.revision();client.setLayerProperties(target,true,.47);QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
        QTRY_COMPARE(canvas->compositionStatistics()["nodeRevision"].toULongLong(),client.revision());
        QTRY_COMPARE(client.frameRevision(),client.revision());
        QCOMPARE(canvas->compositionStatistics()["uploads"].toULongLong(),uploads);
        if(mode==3 && fixed){
            const auto captureDependency=[&](const QString &name){
                if(!QTest::qWaitFor([&]{return client.compositionPacket() && canvas->compositionStatistics()["nodeRevision"].toULongLong()==client.revision() && client.frameRevision()==client.revision();},10000))return false;
                readback->request(*client.compositionPacket());window.update();
                if(!QTest::qWaitFor([&]{window.update();return readback->completed.load()==2;},10000))return false;
                std::lock_guard lock(readback->mutex);return writeLinearCapture(*client.compositionPacket(),readback->linear,name);
            };
            QVERIFY(captureDependency("dependency-opacity"));
            const auto paintDependency=[&](quint64 layer,QColor color){
                client.selectLayer(layer);
                if(!QTest::qWaitFor([&]{return client.activeLayer()==layer && !client.layerEditBusy();},5000))return false;
                client.setBrushColor(color);client.setBrushRadius(3);client.setBrushOpacity(.8);
                const auto undo=client.undoDepth();if(!client.beginStroke({{42,31},1}))return false;client.endStroke();
                return QTest::qWaitFor([&]{return client.undoDepth()==undo+1;},5000);
            };
            auto priorUploads=canvas->compositionStatistics()["uploads"].toULongLong();
            QVERIFY(paintDependency(packet->nodes.front().layer_id,Qt::blue));QVERIFY(captureDependency("dependency-backdrop"));
            auto afterUploads=canvas->compositionStatistics()["uploads"].toULongLong();QVERIFY(afterUploads>priorUploads && afterUploads-priorUploads<=4);
            QVERIFY(canvas->compositionStatistics()["lastDirtyArea"].toULongLong()<=4096);
            priorUploads=afterUploads;
            QVERIFY(paintDependency(mask,Qt::white));QVERIFY(captureDependency("dependency-mask"));
            afterUploads=canvas->compositionStatistics()["uploads"].toULongLong();QVERIFY(afterUploads>priorUploads && afterUploads-priorUploads<=4);
            QVERIFY(canvas->compositionStatistics()["lastDirtyArea"].toULongLong()<=4096);
            revision=client.revision();QVERIFY(client.setLayerBlend(target,13));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
            QVERIFY(captureDependency("dependency-blend"));QCOMPARE(canvas->compositionStatistics()["uploads"].toULongLong(),afterUploads);
        }
        QSignalSpy swapped(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!swapped.isEmpty());const auto gpu=window.grabWindow();
        qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit client.frameChanged();swapped.clear();window.update();QTRY_VERIFY(!swapped.isEmpty());const auto cpu=window.grabWindow();
        const int error=difference(gpu,cpu);QVERIFY2(error<=1,qPrintable(QString("mode %1 error %2").arg(mode).arg(error)));
        QVERIFY(client.lastError().isEmpty());
        save("blend-"+QString::fromLatin1(QTest::currentDataTag()),{{"passed",true},{"window_error",error},{"parameter_upload_delta",0},{"dependencies_checked",mode==3 && fixed},{"linear_capture",true}});
    }
    void explicitFallback_data(){
        QTest::addColumn<QString>("failure");
        for(const auto *name:{"init","upload","extent","placed-image","group-budget","clipping-budget","operation-budget"})QTest::newRow(name)<<QString::fromLatin1(name);
    }
    void closureFixtures_data(){
        QTest::addColumn<QString>("fixture");
        const QDir dir(qEnvironmentVariable("DRAWVERSE_G4_FIXTURES"));
        const auto files=dir.entryList({"*.ora"},QDir::Files,QDir::Name);
        QVERIFY2(files.size()==85,"Generate the 85 authoritative ORA fixtures before this test");
        for(const auto &file:files)QTest::newRow(qPrintable(file))<<dir.absoluteFilePath(file);
    }
    void closureFixtures(){
        QFETCH(QString,fixture);qputenv("DRAWVERSE_GPU_COMPOSITION","1");
        const auto restore=qScopeGuard([]{qputenv("DRAWVERSE_GPU_COMPOSITION","1");});
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        const auto generation=client.generation();QVERIFY(client.openDocument(QUrl::fromLocalFile(fixture)));
        QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy() && client.generation()!=generation,10000);QCOMPARE(client.documentWidth(),96);QCOMPARE(client.documentHeight(),64);
        QFile truth(fixture.chopped(4)+".linear");QVERIFY(truth.open(QIODevice::ReadOnly));const auto expected=truth.readAll();QCOMPARE(expected.size(),96*64*16);
        QQuickWindow window;window.resize(384,288);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,288});canvas->setAcceptedMouseButtons(Qt::NoButton);
        auto readback=std::make_shared<CompositionReadback>();canvas->observeCompositionReadback(readback);canvas->setClient(&client);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->actualSize();
        const bool lod=fixture.contains("nested-") || fixture.contains("dissolve-");
        if(lod){const auto zoom=.5/window.devicePixelRatio();canvas->setSize({64+96*zoom,64+64*zoom});canvas->fitToView();QTRY_VERIFY(qAbs(canvas->zoom()-zoom)<1e-9);}
        const auto revision=client.revision();const auto depth=client.undoDepth();
        QTRY_VERIFY(client.frameRevision()==revision && client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==requestedPixels(*canvas,window));
        const auto name=QFileInfo(fixture).completeBaseName();const int actualLod=client.compositionPacket()?client.compositionPacket()->lod:-1;
        if(window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software){save("closure-"+name,{{"passed",true},{"cpu_fallback",true}});return;}
        QTRY_COMPARE_WITH_TIMEOUT(canvas->compositionStatistics()["nodeRevision"].toULongLong(),revision,15000);
        QVERIFY2(client.compositionPacket(),qPrintable(client.compositionFallback()));QVERIFY(!client.compositionPacket()->operations.isEmpty());
        readback->request(*client.compositionPacket());window.update();
        QTRY_VERIFY_WITH_TIMEOUT((window.update(),readback->completed.load()==2),10000);
        double error=0;QByteArray actual;
        {std::lock_guard lock(readback->mutex);actual=readback->linear;}
        QCOMPARE(actual.size(),expected.size());
        for(qsizetype i=0;i<actual.size();i+=4){float a,b;std::memcpy(&a,actual.constData()+i,4);std::memcpy(&b,expected.constData()+i,4);QVERIFY(std::isfinite(a));error=std::max(error,double(std::abs(a-b)));}
        QFile capture(qEnvironmentVariable("DRAWVERSE_G2_REPORT_DIR")+"/"+name+".g4truth");QVERIFY(capture.open(QIODevice::WriteOnly));QCOMPARE(capture.write(actual),actual.size());capture.close();
        QVERIFY2(error<=2e-5,qPrintable(QString("%1: %2").arg(name).arg(error,0,'g',12)));
        const auto uploads=canvas->compositionStatistics()["uploads"].toULongLong();window.update();QTest::qWait(60);QCOMPARE(canvas->compositionStatistics()["uploads"].toULongLong(),uploads);
        QSignalSpy swapped(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!swapped.isEmpty());const auto gpu=window.grabWindow();
        qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit client.frameChanged();swapped.clear();window.update();QTRY_VERIFY(!swapped.isEmpty());const auto cpu=window.grabWindow();
        const auto windowError=difference(gpu,cpu);QVERIFY2(windowError<=1,qPrintable(QString::number(windowError)));
        QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),depth);QVERIFY(client.lastError().isEmpty());
        save("closure-"+name,{{"passed",true},{"linear_error",error},{"window_error",windowError},{"lod",actualLod},{"stationary_upload_delta",0}});
    }
    void clippingDependencies(){
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        const auto generation=client.generation();QVERIFY(client.openDocument(QUrl::fromLocalFile(qEnvironmentVariable("DRAWVERSE_G4_FIXTURES")+"/nested-base27-clip27-outer27.ora")));
        QTRY_VERIFY(!client.fileBusy() && client.generation()!=generation);
        quint64 outer=0,mask=0,clip=0,background=0;
        for(const auto &entry:client.layers()){const auto row=entry.toMap();const auto id=row["id"].toULongLong(),parent=row["parent"].toULongLong();
            if(row["name"].toString()=="ancestor")outer=id;if(row["name"].toString()=="clip")clip=id;
            if(!parent && !row["group"].toBool() && !row["mask"].toBool())background=id;
        }
        for(const auto &entry:client.layers()){const auto row=entry.toMap();if(row["parent"].toULongLong()==outer && row["mask"].toBool())mask=row["id"].toULongLong();}
        QVERIFY(outer && mask && clip && background);
        QQuickWindow window;window.resize(384,288);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,288});canvas->setAcceptedMouseButtons(Qt::NoButton);
        auto readback=std::make_shared<CompositionReadback>();canvas->observeCompositionReadback(readback);canvas->setClient(&client);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        int maximum=0,comparison=0;
        const auto compare=[&]{
            ++comparison;QByteArray actual;QImage expected;
            const auto failed=[&](const char *reason){
                auto report=QJsonObject::fromVariantMap(canvas->compositionStatistics());report["reason"]=QString::fromLatin1(reason);report["comparison"]=comparison;report["actualBytes"]=actual.size();report["drawing"]=client.drawing();report["editBusy"]=client.layerEditBusy();
                report["cpu"]=QJsonObject::fromVariantMap(client.frameIdentity());report["cpuWidth"]=client.frame().width();report["cpuHeight"]=client.frame().height();report["maximum"]=maximum;
                if(const auto packet=client.compositionPacket()){
                    report["packetRequest"]=double(packet->info.key.request_id);report["packetRevision"]=double(packet->info.key.revision);report["packetGeometry"]=double(packet->geometrySequence);
                    report["packetWidth"]=packet->pixels.width();report["packetHeight"]=packet->pixels.height();report["preview"]=packet->preview;
                }
                const auto name="clipping-failure-"+QString::number(comparison);save(name,report);
                const auto dir=qEnvironmentVariable("DRAWVERSE_G2_REPORT_DIR");
                if(!dir.isEmpty()){
                    QFile pixels(dir+"/"+name+".rgba8");if(pixels.open(QIODevice::WriteOnly))pixels.write(actual);
                    if(!expected.isNull())expected.save(dir+"/"+name+"-cpu.png");
                }
                qWarning()<<"clipping comparison failed"<<comparison<<reason<<maximum;
                return false;
            };
            if(!QTest::qWaitFor([&]{return client.frameRevision()==client.revision() && client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==requestedPixels(*canvas,window);},10000))return failed("CPU frame not ready");
            if(software)return true;
            if(!QTest::qWaitFor([&]{return client.compositionPacket() && client.compositionPacket()->region==canvas->prefetchedDocumentRect() && client.compositionPacket()->pixels==requestedPixels(*canvas,window) && canvas->compositionStatistics()["nodeRevision"].toULongLong()==client.revision() && canvas->compositionStatistics()["nodeRequest"].toULongLong()==client.compositionPacket()->info.key.request_id;},10000))return failed("GPU packet not adopted");
            readback->request(*client.compositionPacket());window.update();
            if(!QTest::qWaitFor([&]{window.update();return readback->completed.load()==2;},10000))return failed("GPU readback not ready");
            {std::lock_guard lock(readback->mutex);actual=readback->display;}
            expected=client.frame().convertToFormat(QImage::Format_RGBA8888_Premultiplied);
            if(actual.size()!=expected.width()*expected.height()*4)return failed("different display dimensions");
            for(int y=0;y<expected.height();++y)for(int x=0;x<expected.width()*4;++x)maximum=std::max(maximum,std::abs(int(uchar(actual[y*expected.width()*4+x]))-int(expected.constScanLine(y)[x])));
            return maximum<=1 || failed("display pixel mismatch");
        };
        QVERIFY(compare());auto uploads=canvas->compositionStatistics()["uploads"].toULongLong();auto revision=client.revision();
        client.setLayerProperties(outer,true,.43);QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());QVERIFY(compare());QCOMPARE(canvas->compositionStatistics()["uploads"].toULongLong(),uploads);
        const auto depth=client.undoDepth();
        for(auto id:{background,clip,mask}){
            client.selectLayer(id);QTRY_VERIFY(client.activeLayer()==id && !client.layerEditBusy());client.setBrushColor(id==mask?Qt::black:Qt::blue);client.setBrushRadius(3);
            const auto undo=client.undoDepth();QVERIFY(client.beginStroke({{48,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),undo+1);QVERIFY(compare());
            if(!software){const auto next=canvas->compositionStatistics()["uploads"].toULongLong();QVERIFY(next>uploads && next-uploads<=4);uploads=next;QVERIFY(canvas->compositionStatistics()["lastDirtyArea"].toULongLong()<=4096);}
        }
        for(int i=0;i<3;++i){client.undo();QTRY_COMPARE(client.undoDepth(),depth+2-i);QVERIFY(compare());}
        QVERIFY(client.lastError().isEmpty());save("clipping-dependencies",{{"passed",true},{"maximum_display_error",maximum},{"parameter_upload_delta",0},{"cpu_fallback",software}});
    }
    void groupBoundaries_data(){
        QTest::addColumn<int>("innerMode");QTest::addColumn<int>("outerMode");QTest::addColumn<bool>("fixed");
        for(int inner:{0,27})for(int outer:{0,3,27})for(bool fixed:{false,true})
            QTest::newRow(qPrintable(QString("inner-%1-outer-%2-fixed-%3").arg(inner).arg(outer).arg(fixed)))<<inner<<outer<<fixed;
    }
    void groupBoundaries(){
        QFETCH(int,innerMode);QFETCH(int,outerMode);QFETCH(bool,fixed);qputenv("DRAWVERSE_GPU_COMPOSITION","1");
        const auto restore=qScopeGuard([]{qputenv("DRAWVERSE_GPU_COMPOSITION","1");});
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("prefs.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        auto generation=client.generation();if(fixed)client.newDocument(96,64);else client.newTransparentDocument(96,64);
        QTRY_VERIFY(client.generation()!=generation && client.documentWidth()==96 && !client.layerEditBusy());const auto background=client.activeLayer();
        const auto paint=[&](quint64 id,QColor color,QPointF point,int radius){
            client.selectLayer(id);if(!QTest::qWaitFor([&]{return client.activeLayer()==id && !client.layerEditBusy();},5000))return false;
            client.setBrushColor(color);client.setBrushRadius(radius);client.setBrushOpacity(.73);
            const auto depth=client.undoDepth();if(!client.beginStroke({point,1}))return false;client.endStroke();
            return QTest::qWaitFor([&]{return client.undoDepth()==depth+1 && !client.layerEditBusy();},5000);
        };
        const auto mutate=[&](auto action){const auto revision=client.revision();if(!action())return false;
            return QTest::qWaitFor([&]{return client.revision()>revision && !client.layerEditBusy();},5000);};
        QVERIFY(paint(background,QColor("#32b366"),{39,30},23));
        client.addLayer("inside");QTRY_VERIFY(client.layers().size()==2 && !client.layerEditBusy());const auto child=client.activeLayer();
        QVERIFY(paint(child,QColor("#e61999"),{48,34},26));
        QVERIFY(mutate([&]{return client.groupLayer(child,"inner");}));const auto inner=client.activeLayer();
        client.addLayer("top");QTRY_VERIFY(client.layers().size()==4 && !client.layerEditBusy());const auto top=client.activeLayer();
        QVERIFY(paint(top,QColor("#2244ee"),{41,28},19));QVERIFY(mutate([&]{return client.setLayerBlend(top,8);}));
        QVERIFY(mutate([&]{return client.groupLayer(inner,"outer");}));const auto outer=client.activeLayer();
        if(innerMode!=27)QVERIFY(mutate([&]{return client.setLayerBlend(inner,innerMode);}));
        if(outerMode!=27)QVERIFY(mutate([&]{return client.setLayerBlend(outer,outerMode);}));
        for(const auto id:{inner,outer}){
            QVERIFY(mutate([&]{return client.setLayerFill(id,.71);}));
            QVERIFY(mutate([&]{client.setLayerProperties(id,true,.63);return true;}));
        }
        QVERIFY(mutate([&]{return client.addMask(outer);}));const auto mask=client.activeLayer();
        QVERIFY(paint(mask,Qt::black,{43,32},17));
        QVERIFY(mutate([&]{client.setLayerProperties(mask,true,.53);return true;}));
        if(fixed)QVERIFY(mutate([&]{return client.setMaskLinked(mask,false);}));
        QVERIFY(mutate([&]{return client.addMask(child);}));const auto childMask=client.activeLayer();
        QVERIFY(paint(childMask,Qt::black,{55,38},12));
        QQuickWindow window;window.resize(384,288);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,288});
        auto readback=std::make_shared<CompositionReadback>();canvas->observeCompositionReadback(readback);canvas->setClient(&client);canvas->setAcceptedMouseButtons(Qt::NoButton);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->actualSize();
        if(fixed){const double zoom=.5/window.devicePixelRatio();canvas->setSize({64+96*zoom,64+64*zoom});canvas->fitToView();QTRY_VERIFY(qAbs(canvas->zoom()-zoom)<1e-9);}
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        QTRY_VERIFY(client.frameRevision()==client.revision() && client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==requestedPixels(*canvas,window));
        if(software){save("group-"+QString::fromLatin1(QTest::currentDataTag()),{{"passed",true},{"cpu_fallback",true}});return;}
        const auto capture=[&](const QString &suffix){
            if(!QTest::qWaitFor([&]{return client.compositionPacket() && canvas->compositionStatistics()["nodeRevision"].toULongLong()==client.revision() && client.frameRevision()==client.revision();},10000))return false;
            readback->request(*client.compositionPacket());window.update();
            if(!QTest::qWaitFor([&]{window.update();return readback->completed.load()==2;},10000))return false;
            std::lock_guard lock(readback->mutex);return writeGroupCapture(*client.compositionPacket(),readback->linear,QString::fromLatin1(QTest::currentDataTag())+suffix);
        };
        QVERIFY(capture("-initial"));QCOMPARE(client.compositionPacket()->surfaceCount,6);
        auto uploads=canvas->compositionStatistics()["uploads"].toULongLong();
        QVERIFY(mutate([&]{client.setLayerProperties(outer,true,.47);return true;}));QVERIFY(capture("-opacity"));
        QCOMPARE(canvas->compositionStatistics()["uploads"].toULongLong(),uploads);
        if(innerMode==27 && outerMode==27 && fixed){
            QVERIFY(paint(background,Qt::blue,{42,31},3));QVERIFY(capture("-backdrop"));
            auto next=canvas->compositionStatistics()["uploads"].toULongLong();QVERIFY(next>uploads && next-uploads<=4);
            QVERIFY(canvas->compositionStatistics()["lastDirtyArea"].toULongLong()<=4096);uploads=next;
            QVERIFY(paint(mask,Qt::black,{47,29},3));QVERIFY(capture("-mask"));
            next=canvas->compositionStatistics()["uploads"].toULongLong();QVERIFY(next>uploads && next-uploads<=4);
            QVERIFY(canvas->compositionStatistics()["lastDirtyArea"].toULongLong()<=4096);
            QVERIFY(mutate([&]{client.setLayerProperties(outer,false,.47);return true;}));QVERIFY(capture("-hidden"));
            for(const auto &n:client.compositionPacket()->nodes)if(n.layer_id!=background){QVERIFY(!n.visible);QCOMPARE(n.tile_count,0u);}
            QVERIFY(mutate([&]{client.setLayerProperties(outer,true,.47);return true;}));QVERIFY(capture("-reshown"));
        }
        QSignalSpy swapped(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!swapped.isEmpty());const auto gpu=window.grabWindow();
        qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit client.frameChanged();swapped.clear();window.update();QTRY_VERIFY(!swapped.isEmpty());const auto cpu=window.grabWindow();
        const int error=difference(gpu,cpu);QVERIFY2(error<=1,qPrintable(QString::number(error)));QVERIFY(client.lastError().isEmpty());
        save("group-"+QString::fromLatin1(QTest::currentDataTag()),{{"passed",true},{"window_error",error},{"linear_capture",true},{"parameter_upload_delta",0}});
    }
    void compositionMoveReusesSource_data(){
        QTest::addColumn<bool>("mask");QTest::addColumn<bool>("pick");QTest::addColumn<bool>("commit");QTest::addColumn<bool>("quick");
        QTest::newRow("normal")<<false<<false<<true<<false;
        QTest::newRow("linked-mask")<<true<<false<<true<<false;
        QTest::newRow("fixed-mask-multiply")<<true<<false<<true<<false;
        QTest::newRow("fixed-mask-cancel")<<true<<false<<false<<false;
        QTest::newRow("group-linked-mask")<<true<<false<<true<<false;
        QTest::newRow("group-fixed-mask")<<true<<false<<true<<false;
        QTest::newRow("group-fixed-cancel")<<true<<false<<false<<false;
        QTest::newRow("group-child-fixed-ancestor")<<true<<false<<true<<false;
        QTest::newRow("clip-base-fixed")<<true<<false<<true<<false;
        QTest::newRow("clip-child")<<true<<false<<true<<false;
        QTest::newRow("clip-chain-cancel")<<true<<false<<false<<false;
        QTest::newRow("clip-chain-quick")<<true<<false<<true<<true;
        QTest::newRow("dissolve-fixed")<<true<<false<<true<<false;
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
        client.addLayer("moving");QTRY_VERIFY(!client.layerEditBusy() && client.activeLayer()!=background);auto target=client.activeLayer();
        client.setBrushColor(Qt::red);client.setBrushRadius(9);InputSample sample;sample.position={36,28};
        const auto depth=client.undoDepth();QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+1);
        if(mask){
            QVERIFY(client.addMask(target));QTRY_VERIFY(!client.layerEditBusy() && client.activeLayer()!=target);
            client.setBrushColor(Qt::black);client.setBrushRadius(4);sample.position={33,26};const auto undo=client.undoDepth();
            QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),undo+1);
        }
        if(QByteArray(QTest::currentDataTag()).startsWith("fixed-mask")){
            const auto revision=client.revision();QVERIFY(client.setMaskLinked(client.activeLayer(),false));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
            const auto beforeBlend=client.revision();QVERIFY(client.setLayerBlend(target,3));QTRY_VERIFY(client.revision()>beforeBlend && !client.layerEditBusy());
        }
        if(QByteArray(QTest::currentDataTag()).startsWith("group-")){
            auto revision=client.revision();QVERIFY(client.groupLayer(target,"moving group"));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());const auto group=client.activeLayer();
            revision=client.revision();QVERIFY(client.addMask(group));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());const auto ancestorMask=client.activeLayer();
            client.setBrushColor(Qt::black);client.setBrushRadius(5);const auto undo=client.undoDepth();QVERIFY(client.beginStroke({{39,27},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),undo+1);
            if(QByteArray(QTest::currentDataTag())!="group-linked-mask"){
                revision=client.revision();QVERIFY(client.setMaskLinked(ancestorMask,false));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
            }
            revision=client.revision();client.setLayerProperties(group,true,.67);QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
            if(QByteArray(QTest::currentDataTag())!="group-child-fixed-ancestor")target=group;
        }
        const auto tag=QByteArray(QTest::currentDataTag());quint64 clip=0;
        if(tag.startsWith("clip-") || tag.startsWith("dissolve-")){
            auto revision=client.revision();QVERIFY(client.setMaskLinked(client.activeLayer(),false));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
            if(tag.startsWith("dissolve-")){revision=client.revision();QVERIFY(client.setLayerBlend(target,1));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());}
            else {
                client.addLayer("clipped moving");QTRY_VERIFY(!client.layerEditBusy() && client.activeLayer()!=target);clip=client.activeLayer();
                client.setBrushColor(Qt::blue);client.setBrushRadius(18);const auto undo=client.undoDepth();QVERIFY(client.beginStroke({{38,30},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),undo+1);
                revision=client.revision();QVERIFY(client.setLayerClipping(clip,true));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
                revision=client.revision();QVERIFY(client.setLayerBlend(clip,3));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
                if(tag=="clip-child")target=clip;
            }
        }
        client.selectLayer(pick?background:target);QTRY_VERIFY(client.activeLayer()==(pick?background:target) && !client.layerEditBusy());
        if(tag.startsWith("clip-chain")){
            client.selectLayers(clip,int(Qt::ControlModifier));QTRY_COMPARE(client.selectedLayers().size(),2);
            QTRY_VERIFY(client.activeLayer()==clip && !client.layerEditBusy());
            QVERIFY(client.selectedLayers().contains(QVariant::fromValue<qulonglong>(target)));
            QVERIFY(client.selectedLayers().contains(QVariant::fromValue<qulonglong>(clip)));
        }

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
                if(disableDuringEdit || QByteArray(QTest::currentDataTag()).startsWith("fixed-mask") || QByteArray(QTest::currentDataTag()).startsWith("group-") || tag.startsWith("clip-") || tag.startsWith("dissolve-")){
                    QTRY_VERIFY(!client.moveObservation()["dirty"].toBool() && client.moveObservation()["completedSequence"].toULongLong()>=client.moveSequence());
                    QTRY_COMPARE(client.frameRevision(),client.revision());
                    QSignalSpy swapped(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!swapped.isEmpty());const auto gpu=window.grabWindow();
                    qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit client.frameChanged();swapped.clear();window.update();QTRY_VERIFY(!swapped.isEmpty());
                    const auto cpu=window.grabWindow();QVERIFY2(difference(gpu,cpu)<=1,qPrintable(QString::number(difference(gpu,cpu))));
                    if(!disableDuringEdit){qputenv("DRAWVERSE_GPU_COMPOSITION","1");emit client.frameChanged();
                        QTRY_COMPARE(canvas->compositionStatistics()["nodeGeometry"].toULongLong(),client.compositionPacket()->geometrySequence);}

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
            QTRY_VERIFY(client.compositionPacket()->region==canvas->prefetchedDocumentRect() && client.compositionPacket()->pixels==requestedPixels(*canvas,window));
            QTRY_COMPARE(canvas->compositionStatistics()["nodeRequest"].toULongLong(),client.compositionPacket()->info.key.request_id);
            readback->request(*client.compositionPacket());window.update();QTRY_VERIFY((window.update(),readback->completed.load()==2));
        }
        const auto uploads=canvas->compositionStatistics()["uploads"].toULongLong();
        const auto pipelines=canvas->compositionStatistics()["pipelineCreates"].toULongLong();
        const bool precise=!software && canvas->compositionStatistics()["preciseTransform"].toInt()>0;
        if(shape==11 && !software)QVERIFY(!precise);
        QVERIFY(client.beginTransform());QVERIFY(client.transformPoints().isEmpty());QTRY_VERIFY(client.transformActive() && !client.transformBusy() && client.transformPoints().size()==16);
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
                // A second, different absolute geometry must change real pixels
                // while retaining this window's source/pass dependencies.
                const auto prepared=canvas->compositionStatistics();const auto prior=client.revision();
                for(int i=0;i<16;++i)points[i]=points[i].toPointF()+QPointF(.625,-.375);
                QVERIFY(client.updateTransform(points,warp,interpolation));
                QTRY_VERIFY(client.revision()>prior && client.frameRevision()==client.revision());
                QTRY_COMPARE(canvas->compositionStatistics()["nodeGeometry"].toULongLong(),client.compositionPacket()->geometrySequence);
                const auto reused=canvas->compositionStatistics();
                QVERIFY(reused["geometryReuses"].toULongLong()>prepared["geometryReuses"].toULongLong());
                for(const auto *key:{"uploads","pipelineCreates","pageTableCreates","axisCreates","parameterBufferCreates","bindingCreates"})QCOMPARE(reused[key],prepared[key]);
                QTRY_VERIFY((window.update(),readback->completed.load()==2));readback->request(*client.compositionPacket());window.update();
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
    void transformResourcesFollowViewAndTopology(){
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("reuse.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());const auto generation=client.generation();client.newDocument(1024,1024);
        QTRY_VERIFY(client.generation()!=generation && client.documentWidth()==1024 && client.documentHeight()==1024 && !client.layerEditBusy());
        client.addLayer("view-topology");QTRY_VERIFY(client.layers().size()==2 && !client.layerEditBusy());
        client.activateBrush();client.setBrushColor(Qt::red);client.setBrushRadius(24);
        const auto beforeStroke=client.undoDepth();QVERIFY(client.beginStroke({{512,512},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),beforeStroke+1);const auto depth=client.undoDepth();
        QQuickWindow window;window.resize(384,288);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,288});canvas->setClient(&client);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));canvas->actualSize();
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        if(!software)QTRY_VERIFY(canvas->compositionStatistics()["nodeRevision"].toULongLong()==client.revision());
        const bool precise=!software && canvas->compositionStatistics()["preciseTransform"].toInt()>0;
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());
        auto points=client.transformPoints();
        const auto update=[&](bool warp){
            for(int i=0;i<16;++i)points[i]=points[i].toPointF()+QPointF(2.25,1.5);
            const auto revision=client.revision();
            if(!client.updateTransform(points,warp,2))return false;
            return QTest::qWaitFor([&]{return client.revision()>revision && client.frameRevision()==client.revision() &&
                (!precise || (client.compositionPacket() && canvas->compositionStatistics()["nodeGeometry"].toULongLong()==client.compositionPacket()->geometrySequence));},5000);
        };
        QVERIFY(update(false));const auto first=canvas->compositionStatistics();
        window.resize(400,300);canvas->setSize({400,300});
        QTRY_VERIFY(client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==requestedPixels(*canvas,window));
        if(precise){
            QTRY_VERIFY(client.compositionPacket() && client.compositionPacket()->region==canvas->prefetchedDocumentRect() && client.compositionPacket()->pixels==requestedPixels(*canvas,window));
            QTRY_VERIFY(client.compositionPacket() && canvas->compositionStatistics()["nodeRequest"].toULongLong()==client.compositionPacket()->info.key.request_id && canvas->compositionStatistics()["nodeGeometry"].toULongLong()==client.compositionPacket()->geometrySequence);
            QVERIFY(canvas->compositionStatistics()["axisCreates"].toULongLong()>first["axisCreates"].toULongLong());
        }
        QVERIFY(update(false));const auto resized=canvas->compositionStatistics();
        points[5]=points[5].toPointF()+QPointF(7,-4);QVERIFY(update(true));
        if(precise)QVERIFY(canvas->compositionStatistics()["pageTableCreates"].toULongLong()>resized["pageTableCreates"].toULongLong());
        QVERIFY(update(true));QVERIFY(update(false));QVERIFY(update(false));
        if(precise){
            QSignalSpy swapped(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!swapped.isEmpty());const auto gpu=window.grabWindow();
            qputenv("DRAWVERSE_GPU_COMPOSITION","0");emit client.frameChanged();swapped.clear();window.update();QTRY_VERIFY(!swapped.isEmpty());
            QVERIFY(difference(gpu,window.grabWindow())<=1);qputenv("DRAWVERSE_GPU_COMPOSITION","1");emit client.frameChanged();
        }
        QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive() && !client.transformBusy());
        QTRY_COMPARE(client.compositionHandoffRevision(),quint64(0));QCOMPARE(client.undoDepth(),depth);QVERIFY(client.lastError().isEmpty());
        auto report=QJsonObject::fromVariantMap(canvas->compositionStatistics());report["passed"]=true;save("transform-view-topology",report);
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
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
        const int side=failure=="extent"?4096:failure.endsWith("budget")?2048:128;client.newDocument(side,side);QTRY_VERIFY(client.ready() && client.documentWidth()==side && !client.layerEditBusy());
        if(failure=="placed-image"){
            const auto raster=client.activeLayer();QImage image(16,16,QImage::Format_RGBA8888);image.fill(Qt::magenta);const auto path=temp.filePath("placed.png");QVERIFY(image.save(path));
            const auto revision=client.revision();QVERIFY(client.placeImage(QUrl::fromLocalFile(path)));QTRY_VERIFY(!client.fileBusy() && client.revision()>revision && !client.layerEditBusy());
            client.selectLayer(raster);QTRY_COMPARE(client.activeLayer(),raster);
        }
        if(failure=="group-budget"){
            const auto raster=client.activeLayer();const auto revision=client.revision();QVERIFY(client.groupLayer(raster,"CPU closure"));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
            client.selectLayer(raster);QTRY_COMPARE(client.activeLayer(),raster);
        }
        if(failure=="clipping-budget"){
            client.addLayer("clipped");QTRY_VERIFY(client.layers().size()==2 && !client.layerEditBusy());
            const auto revision=client.revision();QVERIFY(client.setLayerClipping(client.activeLayer(),true));QTRY_VERIFY(client.revision()>revision && !client.layerEditBusy());
            QTRY_VERIFY(client.layers().first().toMap()["clipped"].toBool());
        }
        if(failure=="operation-budget"){
            const auto generation=client.generation();QVERIFY(client.openDocument(QUrl::fromLocalFile(qEnvironmentVariable("DRAWVERSE_G4_FIXTURES")+"/budgets/operations.ora")));
            QTRY_VERIFY(!client.fileBusy() && client.generation()!=generation && !client.layerEditBusy());
            for(const auto &entry:client.layers()){const auto row=entry.toMap();if(!row["group"].toBool() && row["name"].toString()!="empty clip"){client.selectLayer(row["id"].toULongLong());break;}}
            QTRY_VERIFY(!client.layerEditBusy());
        }
        QQuickWindow window;window.resize(384,384);auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({384,384});canvas->setClient(&client);canvas->setAcceptedMouseButtons(Qt::NoButton);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));
        QTRY_VERIFY(!client.frame().isNull() && client.frameRevision()==client.revision() && client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==requestedPixels(*canvas,window));
        const auto revision=client.revision();const auto undo=client.undoDepth();const auto reference=client.frame();
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        if(failure=="init" || failure=="upload")qputenv("DRAWVERSE_COMPOSITION_FAIL",failure.toUtf8());
        qputenv("DRAWVERSE_GPU_COMPOSITION","1");emit client.frameChanged();
        if(!software){
            if(failure=="init" || failure=="upload")QTRY_COMPARE_WITH_TIMEOUT(canvas->compositionStatistics()["failures"].toInt(),1,15000);
            else if(failure.endsWith("budget") && failure!="operation-budget"){
                QTRY_COMPARE_WITH_TIMEOUT(canvas->compositionStatistics()["failures"].toInt(),1,15000);
                QVERIFY(canvas->compositionStatistics()["fallback"].toString().contains("budget"));
            }else QTRY_VERIFY_WITH_TIMEOUT(!client.compositionFallback().isEmpty() && client.compositionFallback().contains(failure=="extent"?"extent":"transport"),15000);
        }
        QTest::qWait(80);QCOMPARE(canvas->compositionStatistics()["adoptions"].toULongLong(),0);
        save("fallback-state-"+failure,{{"revision_before",double(revision)},{"revision_after",double(client.revision())},{"undo_before",undo},{"undo_after",client.undoDepth()},{"active",window.isActive()},{"drawing",client.drawing()},{"error",client.lastError()}});
        QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),undo);QCOMPARE(client.frame(),reference);
        client.setBrushRadius(4);client.setBrushColor(Qt::black);
        InputSample input;input.position=failure=="placed-image"?QPointF(8,8):QPointF(double(client.documentWidth())/2,double(client.documentHeight())/2);
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
