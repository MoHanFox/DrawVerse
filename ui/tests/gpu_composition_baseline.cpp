// G0 baseline of the existing production path, not the experimental compositor.
#include "canvas/CanvasItem.h"
#include "canvas/CanvasPresentationObservation.h"
#include "platform/UiResources.h"
#include "platform/MenuSurface.h"
#include "canvas/SelectionOverlay.h"
#include <QtTest>
#include <QQuickWindow>
#include <QQmlEngine>
#include <QQmlComponent>
#include <QQuickItemGrabResult>
#include <QSGRendererInterface>
#include <QScreen>
#include <QTemporaryDir>
#include <QFile>
#include <QJsonDocument>
#include <QScopeGuard>
#include <QTimer>
#include <QEventLoop>
#include <cmath>

namespace {
template<class Predicate> bool nativeWait(Predicate predicate,int timeout){
    if(predicate())return true;
    QEventLoop loop;QTimer poll,deadline;poll.setTimerType(Qt::PreciseTimer);poll.setInterval(1);deadline.setSingleShot(true);
    bool result=false;
    QObject::connect(&poll,&QTimer::timeout,&loop,[&]{if(predicate()){result=true;loop.quit();}});
    QObject::connect(&deadline,&QTimer::timeout,&loop,&QEventLoop::quit);
    poll.start();deadline.start(timeout);loop.exec();return result;
}
#define DRIVER_VERIFY_TIMEOUT(condition,timeout) do {if(nativeDriver){QVERIFY(nativeWait([&]{return bool(condition);},timeout));}else {QTRY_VERIFY_WITH_TIMEOUT(condition,timeout);}}while(false)
#define DRIVER_VERIFY(condition) DRIVER_VERIFY_TIMEOUT(condition,5000)
#define DRIVER_COMPARE_TIMEOUT(actual,expected,timeout) do {if(nativeDriver){QVERIFY(nativeWait([&]{return (actual)==(expected);},timeout));QCOMPARE(actual,expected);}else {QTRY_COMPARE_WITH_TIMEOUT(actual,expected,timeout);}}while(false)
#define DRIVER_COMPARE(actual,expected) DRIVER_COMPARE_TIMEOUT(actual,expected,5000)

class Ingress final : public QObject {
public:
    QObject *window=nullptr;QEvent *planned=nullptr;bool armed=false;int unexpected=0;
    QJsonArray events;
    bool eventFilter(QObject *receiver,QEvent *event) override {
        const auto t=event->type();
        const bool input=t==QEvent::MouseMove || t==QEvent::MouseButtonPress || t==QEvent::MouseButtonRelease ||
            t==QEvent::TabletMove || t==QEvent::TabletPress || t==QEvent::TabletRelease || t==QEvent::Wheel ||
            t==QEvent::WindowDeactivate || t==QEvent::Hide || t==QEvent::Resize || t==QEvent::Move ||
            t==QEvent::KeyPress || t==QEvent::KeyRelease;
        if(armed && receiver==window && input && event!=planned){++unexpected;
            if(events.size()<256)events.append(QJsonObject{{"ns",double(InteractionTrace::now())},{"type",int(t)}});
        }
        return false; // Observe only; never hide foreign input to make a run pass.
    }
};
QJsonObject identity(const CanvasPresentationIdentity &i){
    return {{"canvas",QString::number(i.canvas,16)},{"window",QString::number(i.window,16)},
        {"generation",double(i.generation)},{"source",double(i.source)},{"revision",double(i.revision)},
        {"image_key",QString::number(i.imageKey)},{"geometry",QString::number(i.geometryKey)},
        {"x0",i.firstVertex.x()},{"y0",i.firstVertex.y()},
        {"dx",i.presentationDelta.x()},{"dy",i.presentationDelta.y()},{"resident",i.resident},{"valid",i.valid},
        {"composition",i.composition},{"device_epoch",double(i.deviceEpoch)},{"request",double(i.request)},
        {"publication",double(i.publication)},{"plan",double(i.plan)}};
}
}
class CompositionBaseline final : public QObject {
    Q_OBJECT
private slots:
    void measure(){
        const auto output=qEnvironmentVariable("DRAWVERSE_G0_REPORT");
        const bool transform=qEnvironmentVariable("DRAWVERSE_G0_OPERATION")=="transform";
        const int size=qEnvironmentVariableIntValue("DRAWVERSE_G0_SIZE");
        const bool tracing=qEnvironmentVariableIntValue("DRAWVERSE_G0_TRACE")!=0;
        const bool warpMode=qEnvironmentVariableIntValue("DRAWVERSE_G0_WARP")==1;
        const bool nativeDriver=qEnvironmentVariable("DRAWVERSE_G0_DRIVER")=="native-v5";
        const bool general=qEnvironmentVariableIntValue("DRAWVERSE_GPU_COMPOSITION")==1;
        QJsonObject report{{"qt",qVersion()},{"passed",false},{"operation",transform?"transform":"move"},
            {"size",size},{"trace_enabled",tracing},{"composition_enabled",general},{"warp",warpMode},{"input","Qt mouse events to native window; not physical pen"}};
        QJsonArray rows;
        QJsonArray states;int statesDropped=0;
        const auto save=qScopeGuard([&]{report["rows"]=rows;report["trace"]=InteractionTrace::finish();report["trace_dropped"]=double(InteractionTrace::dropped);
            QFile f(output);if(f.open(QIODevice::WriteOnly))f.write(QJsonDocument(report).toJson());});
        QVERIFY(!output.isEmpty());QVERIFY(size==1024 || size==4096);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("preferences.ini"));
        DRIVER_VERIFY(client.ready() && client.brushLibraryReady());
        client.newDocument(size,size);DRIVER_COMPARE(client.documentWidth(),size);DRIVER_VERIFY(!client.layerEditBusy());
        client.addLayer("G0 patterned full raster");DRIVER_VERIFY(!client.layerEditBusy());
        client.activateBucket();client.setBrushColor(Qt::red);QVERIFY(client.bucketFill({0,0}));DRIVER_VERIFY_TIMEOUT(!client.layerEditBusy(),60000);
        client.activateBrush();client.setBrushRadius(size*.075);client.brushLibrary()->setPressureSize(false);client.brushLibrary()->setPressureFlow(false);
        client.brushLibrary()->setOpacity(.5);
        for(int y=0;y<2;++y)for(int x=0;x<3;++x){client.setBrushColor(QColor(20+x*25,80+y*60,220-x*35));
            const auto revision=client.revision();QVERIFY(client.beginStroke({{size*(.2+x*.3)+.5,size*(.2+y*.5)+.5},1}));client.endStroke();
            // A same-layer selection is an ordered actor barrier, with no
            // history edit. Setup history itself may be budget-pruned at 4K.
            client.selectLayer(client.activeLayer());DRIVER_VERIFY_TIMEOUT(!client.layerEditBusy(),60000);
            QVERIFY(client.revision()>revision);QVERIFY(client.lastError().isEmpty());}
        // Freeze the fixture through the public document round trip, before
        // timing. This starts with an empty history so normal budget pruning of
        // setup strokes cannot invalidate the strict +1 transaction assertion.
        const auto fixturePath=QUrl::fromLocalFile(temp.filePath("fixture.ora"));
        QVERIFY(client.saveDocument(fixturePath));DRIVER_VERIFY_TIMEOUT(!client.fileBusy(),60000);QVERIFY(client.lastError().isEmpty());
        const auto generation=client.generation();QVERIFY(client.openDocument(fixturePath));
        DRIVER_VERIFY_TIMEOUT(!client.fileBusy() && client.generation()!=generation,60000);QCOMPARE(client.undoDepth(),0);
        report["fixture_version"]=nativeDriver?5:4;report["event_driver"]=nativeDriver?"native QEventLoop + 1ms bounded predicate timer":"QtTest QTRY/qWait v4";report["history_setup"]="ORA round trip outside measurement; zero initial history; ordered setup barrier";
        client.brushLibrary()->setCursorOutline(false);client.brushLibrary()->setCursorImage(false);
        QQuickWindow window;window.resize(1024,720);window.setColor(Qt::black);
        QQmlEngine engine;QQmlComponent component(&engine,QUrl("qrc:/qml/canvas/CanvasPane.qml"));
        std::unique_ptr<QObject> paneObject(component.createWithInitialProperties({{"ownClient",QVariant::fromValue(&client)},{"mainWindow",QVariant::fromValue(&window)}}));
        QVERIFY2(paneObject,qPrintable(component.errorString()));auto *pane=qobject_cast<QQuickItem*>(paneObject.get());QVERIFY(pane);
        pane->setParentItem(window.contentItem());pane->setSize({1024,720});
        auto *canvas=qvariant_cast<CanvasItem*>(pane->property("canvasView"));QVERIFY(canvas);
        auto observer=std::make_shared<CanvasPresentationObservation>(4096);canvas->observePresentation(observer);
        auto *overlay=pane->findChild<QQuickItem*>("transformOverlay");QVERIFY(overlay);
        window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));canvas->fitToView();canvas->forceActiveFocus();
        DRIVER_VERIFY(client.frameRevision()>=client.revision() && !client.frame().isNull());
        // fitToView can schedule a newer viewport after the first frame. A
        // current revision alone cannot certify sampling density/request.
        DRIVER_VERIFY(client.frameRegion()==canvas->prefetchedDocumentRect() && client.frame().size()==
            QSize(qCeil(client.frameRegion().width()*canvas->zoom()*window.devicePixelRatio()),
                  qCeil(client.frameRegion().height()*canvas->zoom()*window.devicePixelRatio())));
        if(general && quint64(size)*size<=4194304 && window.rendererInterface()->graphicsApi()!=QSGRendererInterface::Software){
            DRIVER_VERIFY_TIMEOUT(client.compositionPacket() && observer->snapshot().identity.composition &&
                observer->snapshot().identity.revision==client.revision() && observer->snapshot().identity.request==client.compositionPacket()->info.key.request_id,15000);
        }
        const auto initialFrame=client.frame();
        report["initial_frame"]=QJsonObject::fromVariantMap(client.frameIdentity());
        report["initial_frame_pixels"]=QJsonArray{initialFrame.width(),initialFrame.height()};
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        if(!software)QCOMPARE(window.rendererInterface()->graphicsApi(),QSGRendererInterface::Direct3D11);
        report["api"]=int(window.rendererInterface()->graphicsApi());report["dpr"]=window.devicePixelRatio();
        report["screen_hz"]=window.screen()->refreshRate();report["zoom"]=canvas->zoom();
        const auto toScreen=[&](QPointF p){return canvas->documentRect().topLeft()+p*canvas->zoom();};
        const auto capture=[&]{const auto opacity=overlay->opacity();overlay->setOpacity(0);auto grab=pane->grabToImage();
            if(!grab || !QTest::qWaitFor([&]{return !grab->image().isNull();},10000)){overlay->setOpacity(opacity);return QImage{};}
            const auto image=grab->image();overlay->setOpacity(opacity);return image;};
        // Correctness pass is outside all measured intervals.
        auto initial=capture();QVERIFY(!initial.isNull());QVERIFY(initial.save(output+"-initial.png"));
        Ingress ingress;ingress.window=&window;window.installEventFilter(&ingress);
        QJsonArray swapped;
        const auto frameConnection=connect(&window,&QQuickWindow::frameSwapped,&window,[&]{
            const auto receipt=observer->snapshot();std::lock_guard lock(frameMutex);
            if(frameHistory.size()<4096)frameHistory.push_back({InteractionTrace::now(),receipt});else ++frameDropped;
        },Qt::DirectConnection);
        const auto disconnectFrame=qScopeGuard([&]{disconnect(frameConnection);});
        InteractionTrace::start(tracing);
        const int depth=client.undoDepth();
        for(int phase=0;phase<3;++phase){
            QJsonObject row{{"phase",phase==0?"immediate":phase==1?"ready":"continuous"}};
            QJsonArray inputs;states={};bool stored=false;
            const auto stateConnection=connect(&client,&PaintCoreClient::stateChanged,&client,[&]{
                if(states.size()>=4096){++statesDropped;return;}
                QJsonObject s{{"ns",double(InteractionTrace::now())},{"revision",double(client.revision())},
                    {"sequence",double(client.moveSequence())},
                    {"move",QJsonObject::fromVariantMap(client.moveObservation())},{"frame",QJsonObject::fromVariantMap(client.frameIdentity())}};
                const auto p=client.residentPreview();if(p && !p->vertices.isEmpty()){
                    s["resident_revision"]=double(p->revision);s["source"]=double(p->sourceId);s["x0"]=p->vertices[0][0];s["y0"]=p->vertices[0][1];
                }
                for(const auto &l:client.layers()){const auto m=l.toMap();if(m.value("id").toULongLong()==client.activeLayer()){s["offset_x"]=m.value("offsetX").toInt();s["offset_y"]=m.value("offsetY").toInt();}}
                states.append(s);
            });
            const auto stopStates=qScopeGuard([&]{disconnect(stateConnection);});
            const auto retain=qScopeGuard([&]{ingress.armed=false;report["unplanned_input"]=ingress.events;report["unplanned_input_count"]=ingress.unexpected;
                if(!stored){row["inputs"]=inputs;row["states"]=states;row["terminal"]="incomplete";rows.append(row);}});
            const qint64 enter=InteractionTrace::now();row["enter_ns"]=double(enter);
            if(transform){QVERIFY(client.beginTransform());DRIVER_VERIFY(client.transformActive() && client.transformPoints().size()==16 && !client.transformBusy());}
            else client.setMoveTool(true);
            if(transform && warpMode)QVERIFY(client.updateTransform(client.transformPoints(),true,client.transformInterpolation()));
            const bool composition=general && !software && bool(client.compositionPacket()) && (!transform || bool(client.compositionPacket()->transform));
            row["composition_expected"]=composition;
            if(transform)row["target_gui_confirmed_ns"]=double(InteractionTrace::now());
            if(phase && !software && !composition){
                if(transform)DRIVER_VERIFY_TIMEOUT(client.residentPreview() && client.residentPreview()->uploads->load()>0,60000);
                else DRIVER_VERIFY_TIMEOUT(client.movePreparation() && client.movePreparation()->uploads->load()>0,60000);
            }
            if(composition && phase)DRIVER_VERIFY_TIMEOUT(client.compositionPacket() && canvas->compositionStatistics()["nodeRequest"].toULongLong()==client.compositionPacket()->info.key.request_id && (!transform || canvas->compositionStatistics()["lastSubmitGeometry"].toULongLong()==client.compositionPacket()->geometrySequence),15000);
            const auto anchor=client.residentPreview();
            const auto assets=transform?std::static_pointer_cast<const ResidentPreviewAssets>(anchor):client.movePreparation();
            if(assets){QJsonArray dimensions;qsizetype bytes=assets->selection.sizeInBytes();
                for(const auto &image:assets->images){dimensions.append(QJsonArray{image.width(),image.height()});bytes+=image.sizeInBytes();}
                row["source_dimensions"]=dimensions;row["source_host_bytes"]=double(bytes);row["uploads_before_press"]=double(assets->uploads->load());}
            row["ready_before_press"]=transform?bool(anchor):bool(client.movePreparation());
            if(composition){row["ready_before_press"]=true;row["source_host_bytes"]=client.compositionPacket()->data.size()*65536.;row["composition_counters_before"]=QJsonObject::fromVariantMap(canvas->compositionStatistics());}
            row["ready_ns"]=double(InteractionTrace::now());row["begin_revision"]=double(client.revision());
            const auto origin=toScreen({size*.3,size*.4});
            const auto send=[&](QEvent::Type type,QPoint delta,int index,qint64 planned){
                const auto p=origin+QPointF(delta)*canvas->zoom();const qint64 start=InteractionTrace::now();
                InteractionTrace::operation=phase*1000+index;
                QMouseEvent event(type,p,window.mapToGlobal(p),type==QEvent::MouseMove?Qt::NoButton:Qt::LeftButton,
                    type==QEvent::MouseButtonRelease?Qt::NoButton:Qt::LeftButton,Qt::NoModifier);
                ingress.planned=&event;QCoreApplication::sendEvent(&window,&event);ingress.planned=nullptr;
                inputs.append(QJsonObject{{"id",index},{"t0_ns",double(start)},{"planned_ns",double(planned)},
                    {"end_ns",double(InteractionTrace::now())},{"dx",delta.x()},{"dy",delta.y()},
                    {"sequence",double(client.moveSequence())},{"revision",double(client.revision())},{"event_type",int(type)}});
            };
            ingress.armed=true;send(QEvent::MouseButtonPress,{},0,InteractionTrace::now());
            const auto plan=InteractionTrace::now();
            const int attempts=phase==2?120:1;QPoint target;int sent=0;
            QTimer timer;timer.setInterval(8);timer.setTimerType(Qt::PreciseTimer);
            const auto move=[&]{++sent;target=phase==2?QPoint(sent*2,sent):QPoint(96,48);send(QEvent::MouseMove,target,sent,phase==2?plan+qint64(sent)*8000000:plan);if(sent==attempts)timer.stop();};
            connect(&timer,&QTimer::timeout,&timer,move);
            if(phase==2)timer.start();else move();
            DRIVER_COMPARE_TIMEOUT(sent,attempts,15000);
            DRIVER_VERIFY_TIMEOUT(!client.transformBusy() && (!client.layerMoveActive() || !client.moveObservation()["dirty"].toBool()),15000);
            // Freeze the latest authoritative packet, then wait for that exact
            // geometry to be adopted. Earlier receipts remain in history and
            // are correlated after timing ends, without GUI polling timestamps.
            // An immediate unprepared move intentionally uses the production
            // CPU path. Requiring a resident here would invent a faster baseline.
            const bool resident=!composition && !software && (transform || phase!=0);
            row["resident_expected"]=resident;
            if(!transform)DRIVER_VERIFY_TIMEOUT(!client.moveObservation()["starting"].toBool() &&
                client.moveObservation()["completedSequence"].toULongLong()>=client.moveSequence(),15000);
            if(resident){DRIVER_VERIFY_TIMEOUT(client.residentPreview() && client.residentPreview()->revision==client.revision(),15000);}
            if(resident)DRIVER_VERIFY_TIMEOUT(client.residentPreview() && (transform?
                (std::abs(client.residentPreview()->vertices[0][0]-target.x())<.01 && std::abs(client.residentPreview()->vertices[0][1]-target.y())<.01):
                client.residentPreview()->presentationDelta==target),15000);
            const auto finalPreview=client.residentPreview();
            quint64 geometry=0;if(finalPreview)geometry=quint64(qHashBits(finalPreview->vertices.constData(),size_t(finalPreview->vertices.size())*sizeof(finalPreview->vertices[0])));
            if(composition){
                DRIVER_VERIFY_TIMEOUT(client.compositionPacket() && client.compositionPacket()->preview && (transform?client.compositionPacket()->transform && std::abs(client.compositionPacket()->transform->geometry.points[0]-target.x())<.01 && std::abs(client.compositionPacket()->transform->geometry.points[1]-target.y())<.01:client.compositionPacket()->presentationDelta==target),15000);
                const auto packet=client.compositionPacket();
                DRIVER_VERIFY_TIMEOUT(observer->snapshot().identity.valid && observer->snapshot().identity.composition &&
                    observer->snapshot().identity.deviceEpoch==packet->info.key.device_epoch && observer->snapshot().identity.source==packet->editSequence &&
                    observer->snapshot().identity.geometryKey==packet->geometrySequence && (transform?std::abs(observer->snapshot().identity.firstVertex.x()-target.x())<.01 && std::abs(observer->snapshot().identity.firstVertex.y()-target.y())<.01:observer->snapshot().identity.presentationDelta==target),15000);
            }else{
                if(!resident)DRIVER_VERIFY_TIMEOUT(client.frameRevision()>=client.revision(),15000);
                DRIVER_VERIFY_TIMEOUT(!resident?observer->snapshot().identity.imageKey==client.frame().cacheKey():
                    (observer->snapshot().identity.valid && observer->snapshot().identity.source==finalPreview->sourceId && observer->snapshot().identity.geometryKey==geometry),15000);
            }
            if(composition)row["target_gui_confirmed_ns"]=double(client.compositionTargetConfirmedNs());
            row["hold_ns"]=double(InteractionTrace::now());row["held_identity"]=identity(observer->snapshot().identity);
            QCOMPARE(ingress.unexpected,0);
            // Timing has stopped. Freeze the adopted content identity across
            // an independent full image grab; handles are a sibling of Canvas.
            QImage preview;
            const auto at=[&](const QImage &image,QPointF p){const auto v=toScreen(p);return image.pixelColor(qBound(0,int(v.x()*image.width()/canvas->width()),image.width()-1),qBound(0,int(v.y()*image.height()/canvas->height()),image.height()-1));};
            const auto pixelsMatch=[&]{
                const auto before=observer->snapshot().identity;preview=capture();const auto after=observer->snapshot().identity;
                if(preview.isNull() || !(before==after))return false;
                const auto white=at(preview,{target.x()*.5,size*.4}),red=at(preview,{size*.65,size*.4});
                return white.red()>230 && white.green()>230 && white.blue()>230 && red.red()>180 && red.green()<70 && red.blue()<70;
            };
            DRIVER_VERIFY(pixelsMatch());QVERIFY(preview.save(output+QString("-%1-held.png").arg(phase)));
            row["first_vertex_origin"]=QJsonArray{anchor && !anchor->vertices.isEmpty()?anchor->vertices[0][0]:0,anchor && !anchor->vertices.isEmpty()?anchor->vertices[0][1]:0};
            row["input_count"]=sent;
            row["release_ns"]=double(InteractionTrace::now());send(QEvent::MouseButtonRelease,target,attempts+1,InteractionTrace::now());
            if(transform){DRIVER_VERIFY(!client.transformBusy());QVERIFY(client.finishTransform(true));}
            DRIVER_VERIFY_TIMEOUT(!client.layerEditBusy() && !client.transformActive() && !client.layerMoveActive(),60000);
            DRIVER_VERIFY_TIMEOUT(!client.residentPreview() && client.frameRevision()>=client.revision(),60000);
            if(composition){
                DRIVER_VERIFY_TIMEOUT(client.compositionHandoffRevision()==0 && observer->snapshot().identity.composition && observer->snapshot().identity.source==0 &&
                    observer->snapshot().identity.revision==client.revision(),15000);
                row["composition_counters_final"]=QJsonObject::fromVariantMap(canvas->compositionStatistics());
            }else DRIVER_VERIFY_TIMEOUT(observer->snapshot().identity.imageKey==client.frame().cacheKey(),10000);
            row["final_adopted_ns"]=double(observer->snapshot().ns);row["final_identity"]=identity(observer->snapshot().identity);
            ingress.armed=false;QCOMPARE(ingress.unexpected,0);
            DRIVER_COMPARE(client.undoDepth(),depth+1);QVERIFY(client.lastError().isEmpty());
            row["history_delta"]=client.undoDepth()-depth;row["inputs"]=inputs;row["states"]=states;row["terminal"]="committed";rows.append(row);stored=true;
            // Independent correctness, including the changed content, not handles.
            auto held=capture();QVERIFY(!held.isNull());QVERIFY(held!=initial);QVERIFY(held.save(output+QString("-%1-final.png").arg(phase)));
            client.undo();DRIVER_COMPARE(client.undoDepth(),depth);DRIVER_VERIFY(!client.layerEditBusy() && client.frameRevision()>=client.revision());
            DRIVER_COMPARE(client.frame(),initialFrame);
            QImage undone;
            const auto saveUndo=qScopeGuard([&]{if(!undone.isNull())undone.save(output+QString("-%1-undone.png").arg(phase));
                report["image_metadata"]=QJsonObject{{"initial_format",int(initial.format())},{"undo_format",int(undone.format())},
                    {"initial_dpr",initial.devicePixelRatio()},{"undo_dpr",undone.devicePixelRatio()}};});
            DRIVER_VERIFY((undone=capture())==initial);
            if(!transform)client.setMoveTool(false);
        }
        QJsonArray adopted;
        for(const auto &r:observer->samples()){auto j=identity(r.identity);j["ns"]=double(r.ns);j["epoch"]=double(r.epoch);adopted.append(j);}
        report["adoptions"]=adopted;report["adoptions_dropped"]=double(observer->overflow());
        {std::lock_guard lock(frameMutex);for(const auto &r:frameHistory){auto j=identity(r.receipt.identity);j["ns"]=double(r.ns);j["adoption_ns"]=double(r.receipt.ns);j["epoch"]=double(r.receipt.epoch);swapped.append(j);}}
        report["submitted_frames"]=swapped;report["frames_dropped"]=frameDropped;report["states_dropped"]=statesDropped;
        QCOMPARE(observer->overflow(),quint64(0));QCOMPARE(frameDropped,0);
        report["passed"]=true;canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();DRIVER_COMPARE(stopped.size(),1);
    }
private:
    struct Frame {qint64 ns;CanvasPresentationObservation::Receipt receipt;};
    std::mutex frameMutex;std::vector<Frame> frameHistory;int frameDropped=0;
};
int main(int argc,char **argv){QGuiApplication app(argc,argv);initializeDrawVerseUiResources();
    qmlRegisterType<MenuSurface>("DrawVerse",1,0,"MenuSurface");qmlRegisterType<CanvasItem>("DrawVerse",1,0,"PaintCanvas");
    qmlRegisterType<SelectionOverlay>("DrawVerse",1,0,"SelectionOutline");app.setOrganizationName("DrawVerseTests");app.setApplicationName("G0Baseline");
    CompositionBaseline test;return QTest::qExec(&test,argc,argv);}
#include "gpu_composition_baseline.moc"
