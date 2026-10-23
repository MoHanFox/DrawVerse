#include "canvas/CanvasItem.h"
#include "platform/InteractionTrace.h"
#include "platform/UiResources.h"
#include <QtTest>
#include <QQuickWindow>
#include <QQuickItemGrabResult>
#include <QSGRendererInterface>
#include <QScreen>
#include <QTemporaryDir>
#include <QJsonDocument>
#include <QFile>
#include <QScopeGuard>
#include <QTimer>
#include <QEventLoop>
#include <algorithm>

namespace {
QJsonObject stats(QList<double> values) {
    std::sort(values.begin(),values.end());
    const auto at=[&](double p){return values.isEmpty()?0.:values[qMax(0,int(std::ceil(values.size()*p))-1)];};
    return {{"n",values.size()},{"p50",at(.5)},{"p95",at(.95)},{"max",values.isEmpty()?0.:values.last()}};
}
}
class LayerMoveBaseline : public QObject {
    Q_OBJECT
private slots:
    void measure() {
        const auto output=qEnvironmentVariable("DRAWVERSE_MOVE_REPORT");
        const auto scenario=qEnvironmentVariableIntValue("DRAWVERSE_MOVE_SCENE");
        const bool tracing=qEnvironmentVariableIntValue("DRAWVERSE_MOVE_TRACE")!=0;
        QJsonObject report{{"pid",double(QCoreApplication::applicationPid())},{"qt_version",qVersion()},{"scenario",scenario},{"trace_enabled",tracing},{"input","Qt synthetic mouse events delivered to native QQuickWindow"},{"passed",false}};
        QJsonArray rows,trace;
        const auto exportReport=qScopeGuard([&]{report["rows"]=rows;report["trace"]=trace;report["trace_dropped"]=double(InteractionTrace::dropped);QFile file(output);if(file.open(QIODevice::WriteOnly))file.write(QJsonDocument(report).toJson());});
        QVERIFY(scenario>=1 && scenario<=4);QVERIFY(!output.isEmpty());
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("preferences.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        const int size=scenario==1?1024:4096;
        client.newDocument(size,size);QTRY_COMPARE(client.documentWidth(),size);QTRY_VERIFY(!client.layerEditBusy());
        client.addLayer("C1 full red raster");QTRY_VERIFY(!client.layerEditBusy());const auto owner=client.activeLayer();
        client.activateBucket();client.setBrushColor(Qt::red);QVERIFY(client.bucketFill({0,0}));QTRY_VERIFY_WITH_TIMEOUT(!client.layerEditBusy(),60000);
        quint64 mask=0;
        if(scenario>=3){
            QVERIFY(client.addMask(owner));QTRY_VERIFY(!client.layerEditBusy());mask=client.activeLayer();
            client.activateBrush();client.setBrushColor(Qt::black);client.setBrushRadius(90);client.brushLibrary()->setPressureSize(false);client.brushLibrary()->setPressureFlow(false);
            QVERIFY(client.beginStroke({{size*.5+.5,size*.5+.5},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());
            if(scenario==4){QVERIFY(client.setMaskLinked(mask,false));QTRY_VERIFY(!client.layerEditBusy());}
            else {QVERIFY(client.groupLayer(owner,"C1 Normal group"));QTRY_VERIFY(!client.layerEditBusy());}
            client.selectLayer(owner);QTRY_COMPARE(client.activeLayer(),owner);QTRY_VERIFY(!client.layerEditBusy());
        }
        QQuickWindow window;window.resize(1024,720);window.setColor(Qt::black);
        auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({1024,720});canvas->setClient(&client);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->fitToView();canvas->forceActiveFocus();
        QTRY_VERIFY(client.frameRevision()>=client.revision() && !client.frame().isNull());
        QVERIFY(canvas->gpuDisplay());QCOMPARE(window.rendererInterface()->graphicsApi(),QSGRendererInterface::Direct3D11);
        const auto screen=[&](QPointF p){return canvas->documentRect().topLeft()+p*canvas->zoom();};
        const auto capture=[&]{auto grab=canvas->grabToImage();if(!grab || !QTest::qWaitFor([&]{return !grab->image().isNull();},10000))return QImage{};return grab->image();};
        const auto at=[&](const QImage &image,QPointF p){const auto v=screen(p);return image.pixelColor(qRound(v.x()*image.width()/canvas->width()),qRound(v.y()*image.height()/canvas->height()));};
        const auto red=[](QColor c){return c.red()>200 && c.green()<60 && c.blue()<60;};
        const auto white=[](QColor c){return c.red()>220 && c.green()>220 && c.blue()>220;};
        QImage initial;QTRY_VERIFY(!(initial=capture()).isNull() && red(at(initial,{size*.03,size*.5})));
        if(mask)QVERIFY(white(at(initial,{size*.5,size*.5})));
        report["fixture"]=QJsonObject{{"document_size",size},{"nonempty_raster_bounds",QJsonArray{0,0,size,size}},{"owner",double(owner)},{"mask",double(mask)},{"mask_linked",scenario==3},{"group",scenario==3},{"layers",QJsonArray::fromVariantList(client.layers())},{"window_width",window.width()},{"window_height",window.height()},{"zoom",canvas->zoom()},{"dpr",window.devicePixelRatio()},{"logical_dpi",window.screen()->logicalDotsPerInch()},{"screen",window.screen()->name()},{"graphics_api",int(window.rendererInterface()->graphicsApi())},{"gpu_display",canvas->gpuDisplay()},{"viewport_width",client.frame().width()},{"viewport_height",client.frame().height()}};
        const int depth=client.undoDepth();
        const auto offset=[&]{for(const auto &v:client.layers()){const auto m=v.toMap();if(m.value("id").toULongLong()==owner)return QPoint(m.value("offsetX").toInt(),m.value("offsetY").toInt());}return QPoint(INT_MIN,INT_MIN);};
        InteractionTrace::start(tracing);
        const auto stopTrace=qScopeGuard([&]{trace=InteractionTrace::finish();});
        QPoint finalDelta;
        for(int phase=0;phase<2;++phase){
            InteractionTrace::operation=phase*1000;
            client.setMoveTool(false);
            // Count only this activation's reset and completion, not the preceding clear.
            QSignalSpy preparation(&client,&PaintCoreClient::movePreparationChanged);
            const auto prepareStart=InteractionTrace::now();client.setMoveTool(true);
            if(phase){
                if(scenario==4)QTRY_VERIFY_WITH_TIMEOUT(preparation.size()>=2,60000);
                else QTRY_VERIFY_WITH_TIMEOUT(client.movePreparation() && client.movePreparation()->uploads->load()>0,60000);
            }
            const auto assets=client.movePreparation();
            QJsonObject row{{"phase",phase==0?"immediate":"subsequent"},{"prepare_start_ns",double(prepareStart)},{"press_assets_ready",bool(assets)},{"press_upload_batches",assets?double(assets->uploads->load()):0},{"press_source",assets?double(assets->sourceId):0},{"prepare_observation_ms",double(InteractionTrace::now()-prepareStart)/1e6}};
            if(assets){QJsonArray dimensions;for(const auto &image:assets->images)dimensions.append(QJsonArray{image.width(),image.height()});row["source_dimensions"]=dimensions;}
            QJsonArray inputs,observations;QList<double> sync,gaps,beats;
            QSet<quint64> sources;quint64 maxUploads=0;int observationDropped=0,heartbeatDropped=0;
            const auto start=screen({size*.3,size*.4});
            // Fractional local coordinates preserve a precisely defined 384/128 document delta.
            const auto end=start+QPointF(384,128)*canvas->zoom();
            const auto expected=canvas->documentPoint(end)-canvas->documentPoint(start);
            finalDelta={qRound(expected.x()),qRound(expected.y())};
            const auto startRevision=client.revision();
            qint64 release=0,metadataDone=0,frameDone=0,lastSend=0,lastBeat=InteractionTrace::now();int sent=0;bool timedOut=false;
            QEventLoop loop;QTimer timer,heartbeat,timeout;
            heartbeat.setInterval(8);heartbeat.setTimerType(Qt::PreciseTimer);
            connect(&heartbeat,&QTimer::timeout,&loop,[&]{const auto t=InteractionTrace::now();if(beats.size()<20000)beats.append(double(t-lastBeat)/1e6);else ++heartbeatDropped;lastBeat=t;});
            const auto observe=[&]{
                const auto t=InteractionTrace::now();const auto p=client.residentPreview();
                if(p){sources.insert(p->sourceId);maxUploads=qMax(maxUploads,quint64(p->uploads->load()));}
                if(observations.size()<20000)observations.append(QJsonObject{{"ns",double(t)},{"revision",double(client.revision())},{"frame_revision",double(client.frameRevision())},{"active",client.layerMoveActive()},{"undo_depth",client.undoDepth()},{"source",p?double(p->sourceId):0},{"resident_revision",p?double(p->revision):0},{"offset_x",offset().x()},{"offset_y",offset().y()}});else ++observationDropped;
                if(release && !client.layerEditBusy() && client.undoDepth()==depth+1 && offset()==finalDelta){
                    if(!metadataDone)metadataDone=t;
                    if(client.frameRevision()>=client.revision() && client.revision()>startRevision && !p){frameDone=t;loop.quit();}
                }
            };
            connect(&client,&PaintCoreClient::stateChanged,&loop,observe);connect(&client,&PaintCoreClient::frameChanged,&loop,observe);
            const auto send=[&](QEvent::Type type,QPointF p,int index){
                InteractionTrace::operation=phase*1000+index;
                const auto t=InteractionTrace::now();QMouseEvent e(type,p,window.mapToGlobal(p),type==QEvent::MouseMove?Qt::NoButton:Qt::LeftButton,type==QEvent::MouseButtonRelease?Qt::NoButton:Qt::LeftButton,Qt::NoModifier);
                QCoreApplication::sendEvent(&window,&e);const auto finish=InteractionTrace::now();
                inputs.append(QJsonObject{{"index",index},{"type",int(type)},{"start_ns",double(t)},{"sync_us",double(finish-t)/1000},{"x",p.x()},{"y",p.y()},{"accepted",e.isAccepted()}});
                if(type==QEvent::MouseMove){sync.append(double(finish-t)/1000);if(lastSend)gaps.append(double(t-lastSend)/1e6);lastSend=t;}
                InteractionTrace::operation=phase*1000;
            };
            connect(&timer,&QTimer::timeout,&loop,[&]{
                ++sent;send(QEvent::MouseMove,start+(end-start)*(double(sent)/120),sent);
                if(sent==120){timer.stop();release=InteractionTrace::now();send(QEvent::MouseButtonRelease,end,121);observe();}
            });
            timer.setInterval(8);timer.setTimerType(Qt::PreciseTimer);
            timeout.setSingleShot(true);connect(&timeout,&QTimer::timeout,&loop,[&]{timedOut=true;loop.quit();});
            heartbeat.start();timeout.start(90000);send(QEvent::MouseButtonPress,start,0);timer.start();loop.exec();timer.stop();heartbeat.stop();timeout.stop();
            QJsonArray sourceIds;for(auto id:sources)sourceIds.append(double(id));
            row["generation"]=double(client.generation());row["start_revision"]=double(startRevision);row["observation_dropped"]=observationDropped;row["heartbeat_dropped"]=heartbeatDropped;row["inputs"]=inputs;row["observations"]=observations;row["attempts"]=sent;row["completed"]=frameDone?1:0;row["timeouts"]=timedOut?1:0;row["move_sync_us"]=stats(sync);row["actual_send_gap_ms"]=stats(gaps);row["heartbeat_ms"]=stats(beats);row["release_ns"]=double(release);row["release_to_metadata_ms"]=metadataDone?double(metadataDone-release)/1e6:-1;row["release_to_full_frame_ms"]=frameDone?double(frameDone-release)/1e6:-1;row["expected_delta"]=QJsonArray{finalDelta.x(),finalDelta.y()};row["final_delta"]=QJsonArray{offset().x(),offset().y()};row["history_delta"]=client.undoDepth()-depth;row["resident_sources"]=sourceIds;row["max_source_upload_batches"]=double(maxUploads);rows.append(row);
            QVERIFY2(!timedOut,"timed drag/close barrier did not complete; raw evidence retained");QCOMPARE(sent,120);QCOMPARE(offset(),finalDelta);QCOMPARE(client.undoDepth(),depth+1);
            if(phase && assets){QCOMPARE(sources.size(),1);QVERIFY(sources.contains(assets->sourceId));QCOMPARE(maxUploads,quint64(assets->uploads->load()));QCOMPARE(maxUploads,quint64(row["press_upload_batches"].toDouble()));}
            if(scenario==4)QVERIFY(sources.isEmpty());
            // Outside measured intervals: reset via real history, never reset the repository or pixels.
            if(!phase){client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy() && client.frameRevision()>=client.revision());QCOMPARE(offset(),QPoint());}
        }
        trace=InteractionTrace::finish();
        // Independent correctness readback after all measured intervals.
        QImage moved;QTRY_VERIFY(!(moved=capture()).isNull() && white(at(moved,{size*.03,size*.4})) && red(at(moved,{size*.7,size*.4})));
        if(mask)QVERIFY(white(at(moved,{size*.5+(scenario==3?finalDelta.x():0),size*.5+(scenario==3?finalDelta.y():0)})));
        client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy() && client.frameRevision()>=client.revision());QCOMPARE(offset(),QPoint());
        QImage undone;QTRY_VERIFY(!(undone=capture()).isNull() && red(at(undone,{size*.03,size*.4})));
        client.redo();QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy() && client.frameRevision()>=client.revision());QCOMPARE(offset(),finalDelta);
        QImage redone;QTRY_VERIFY(!(redone=capture()).isNull() && white(at(redone,{size*.03,size*.4})));
        const auto beforeCancel=client.undoDepth();const auto from=screen({size*.3,size*.4});QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,from.toPoint());QTest::mouseMove(&window,(from+QPointF(20,10)).toPoint());QTest::keyClick(&window,Qt::Key_Escape);QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,(from+QPointF(20,10)).toPoint());QTRY_VERIFY(!client.layerEditBusy() && client.frameRevision()>=client.revision());QCOMPARE(client.undoDepth(),beforeCancel);QCOMPARE(offset(),finalDelta);
        QImage canceled;QTRY_VERIFY(!(canceled=capture()).isNull() && white(at(canceled,{size*.03,size*.4})) && red(at(canceled,{size*.7,size*.4})));
        report["correctness"]=QJsonObject{{"final_pixel_readback",true},{"undo_redo_pixels",true},{"cancel_history_offset_and_pixels",true}};
        report["passed"]=true;canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
};
int main(int argc,char **argv){QGuiApplication app(argc,argv);app.setOrganizationName("DrawVerseTests");app.setApplicationName("LayerMoveBaseline");initializeDrawVerseUiResources();LayerMoveBaseline test;return QTest::qExec(&test,argc,argv);}
#include "layer_move_baseline.moc"
