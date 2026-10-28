#include "PendingMoveEvidence.h"
#include "platform/UiResources.h"
#include <QSGRendererInterface>
#include <QQuickGraphicsConfiguration>
#include <QCryptographicHash>
#include <QTimer>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include "documents/DocumentManager.h"
#include "workspace/WorkspaceManager.h"
#include "canvas/SelectionOverlay.h"
#include "color/ColorWheelItem.h"
#include "platform/MenuSurface.h"
#include "platform/UiScale.h"

// Manual C3 probe. It shares the existing fixture/input/grab protocol, not a new
// production scheduler. All I/O and pixel reads happen after measured intervals.
class MoveStartTracking final : public QObject {
    Q_OBJECT
private slots:
    void residentAssociationRejectsStaleCpuAndGpuContent() {
        QJsonObject context{{"generation",3},{"owner",2},{"revision",7},{"sequence",14},{"undo_depth",5},{"error",""},
            {"move",QJsonObject{{"active",true},{"starting",false},{"closing",false},{"dirty",false},
                {"submittedKind",4},{"completedSequence",14},{"deltaX",384},{"deltaY",0}}},
            {"frame",QJsonObject{{"generation",3},{"view",0},{"revision",6},{"request",9},{"frame",20}}},
            {"owner_metadata",QJsonObject{{"id",2},{"offsetX",384},{"offsetY",0}}},
            {"frame_key","old-cpu"},{"frame_region",QJsonArray{0,0,4096,4096}},
            {"canvas","canvas1"},{"window","window1"},{"active",true},{"visible",true},{"canvas_visible",true},
            {"dpr",1},{"zoom",.175},{"document_origin",QJsonArray{152,0}},{"size",QJsonArray{1024,720}},
            {"context_epoch",10},{"input_epoch",2},{"target_plan",2},{"preview_source",30},{"preview_revision",7},
            {"begin_sequence",13},{"target_sequence",14},{"scenario_invalid",false},{"client_matches",true}};
        QJsonObject render{{"epoch",3},{"valid",true},{"canvas","canvas1"},{"window","window1"},{"generation",3},
            {"resident",true},{"source",30},{"revision",7},{"image_key","0"},
            {"document_origin",QJsonArray{152,0}},{"zoom",.175},{"size",QJsonArray{1024,720}}};
        context["render"]=render;
        const auto make=[](QJsonObject s){QJsonObject grab{{"id",1},{"ready_id",1},{"completed",true},{"image_key","grab1"},{"grab_canvas","canvas1"}};
            for(const auto *key:{"request_state","ready_state","complete_state","assertion_state"})grab[key]=s;return grab;};
        const auto accepts=[](const QJsonObject &g){return PendingMoveEvidence::associated(g,3,{384,0},5,true);};
        QVERIFY(!PendingMoveEvidence::associated(make(context),3,{384,0},5)); // Original pendingMoved contract unchanged.
        QVERIFY(accepts(make(context))); // Current resident need not update the inactive CPU image.
        for(const auto &field:{"source","revision","generation"}){
            auto wrong=context;auto r=render;r[field]=99;wrong["render"]=r;QVERIFY(!accepts(make(wrong)));
        }
        auto wrong=context;auto r=render;r["resident"]=false;r["image_key"]="old-cpu";r["region"]=context["frame_region"];wrong["render"]=r;
        QVERIFY(!accepts(make(wrong))); // A correct GUI resident cannot certify the old normal node.
        wrong=context;wrong["preview_source"]=0;QVERIFY(!accepts(make(wrong)));
        wrong=context;wrong["preview_revision"]=6;QVERIFY(!accepts(make(wrong)));
        auto interrupted=make(context);wrong=context;wrong["input_epoch"]=3;interrupted["ready_state"]=wrong;QVERIFY(!accepts(interrupted));
        CanvasPresentationObservation bounded(1);CanvasPresentationIdentity id;id.valid=true;
        bounded.adopted(id);const auto first=bounded.snapshot();bounded.adopted(id);QCOMPARE(bounded.samples().size(),size_t(1));
        id.revision=1;bounded.adopted(id);QCOMPARE(bounded.overflow(),quint64(1));QVERIFY(bounded.snapshot().epoch>first.epoch);
    }
    void embeddedShaderArtifacts() {
        const auto directory=qEnvironmentVariable("DRAWVERSE_SHADER_EXPORT");
        if(directory.isEmpty())return;
        QVERIFY(QDir().mkpath(directory));QJsonObject resources;
        for(const auto *name:{"liquify.vert.qsb","liquify.frag.qsb"}){
            const auto url=QString(":/shaders/")+name;QFile resource(url);QVERIFY(resource.open(QIODevice::ReadOnly));
            const auto data=resource.readAll();QVERIFY(!data.isEmpty());
            QFile out(directory+"/"+name);QVERIFY(out.open(QIODevice::WriteOnly|QIODevice::NewOnly));QCOMPARE(out.write(data),data.size());
            resources[url]=QJsonObject{{"size",data.size()},{"sha256",QString::fromLatin1(QCryptographicHash::hash(data,QCryptographicHash::Sha256).toHex())}};
        }
        QFile out(directory+"/resources.json");QVERIFY(out.open(QIODevice::WriteOnly|QIODevice::NewOnly));out.write(QJsonDocument(resources).toJson());
    }
    void measure() {
        const auto output=qEnvironmentVariable("DRAWVERSE_MOVE_REPORT");
        const bool pair=qEnvironmentVariableIntValue("DRAWVERSE_C3_SHADER_PAIR")!=0;
        const bool mainQml=qEnvironmentVariableIntValue("DRAWVERSE_C3_MAIN_QML")!=0;
        const bool direct=qEnvironmentVariableIntValue("DRAWVERSE_C3_DIRECT_W")!=0;
        const bool tracking=qEnvironmentVariableIntValue("DRAWVERSE_C3_TRACKING")!=0;
        const bool tracing=qEnvironmentVariableIntValue("DRAWVERSE_MOVE_TRACE")!=0;
        const int size=qEnvironmentVariableIntValue("DRAWVERSE_MOVE_SCENE")==1?1024:4096;
        QJsonObject report{{"schema",1},{"baseline","9b36e2f80f414ddee062eb87fb6ed429359cae17"},
            {"direct_w",direct},{"tracking",tracking},{"shader_pair",pair},{"main_qml",mainQml},{"trace_enabled",tracing},{"passed",false}};
        QJsonArray rows,trace;
        const auto exportReport=qScopeGuard([&]{report["rows"]=rows;report["trace"]=trace;
            report["trace_dropped"]=double(InteractionTrace::dropped);
            QFile f(output);if(f.open(QIODevice::WriteOnly|QIODevice::NewOnly))f.write(QJsonDocument(report).toJson());});
        QVERIFY(!output.isEmpty());
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("preferences.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        std::unique_ptr<QQuickWindow> minimal;QQuickWindow *window=nullptr;CanvasItem *canvas=nullptr;
        if(mainQml){
            engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
            // Configure this window's cache before its first scenegraph exposure.
            engine.setInitialProperties({{"createInitialDocument",true},{"visible",false}});engine.load(QUrl("qrc:/qml/app/Main.qml"));
            QVERIFY(!engine.rootObjects().isEmpty());window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);
            const auto cacheFile=qEnvironmentVariable("DRAWVERSE_C3_PIPELINE_CACHE");
            if(!cacheFile.isEmpty()){
                QVERIFY(!QFile::exists(cacheFile));auto configuration=window->graphicsConfiguration();
                QVERIFY(configuration.isAutomaticPipelineCacheEnabled());
                configuration.setPipelineCacheLoadFile(cacheFile);configuration.setPipelineCacheSaveFile(cacheFile);
                window->setGraphicsConfiguration(configuration);report["pipeline_cache_initially_absent"]=true;
            }
            auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);
            QTRY_VERIFY(!documents->activeId().isEmpty());QCOMPARE(documents->activeClient(),&client);
            canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);
        }else{
            minimal=std::make_unique<QQuickWindow>();window=minimal.get();window->resize(1024,720);window->setColor(Qt::black);
            canvas=new CanvasItem(window->contentItem());canvas->setSize({1024,720});canvas->setClient(&client);
        }
        client.newDocument(size,size);QTRY_COMPARE(client.documentWidth(),size);QTRY_VERIFY(!client.layerEditBusy());
        client.addLayer("C3 full red raster");QTRY_VERIFY(!client.layerEditBusy());const auto owner=client.activeLayer();
        client.activateBucket();client.setBrushColor(Qt::red);QVERIFY(client.bucketFill({0,0}));QTRY_VERIFY_WITH_TIMEOUT(!client.layerEditBusy(),60000);
        window->show();window->requestActivate();QVERIFY(QTest::qWaitForWindowActive(window));canvas->fitToView();canvas->forceActiveFocus();
        QTRY_VERIFY(!client.frame().isNull() && client.frameRevision()>=client.revision());
        QCOMPARE(window->rendererInterface()->graphicsApi(),QSGRendererInterface::Direct3D11);
        const auto screen=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom());};
        const int depth=client.undoDepth();
        report["fixture"]=QJsonObject{{"size",size},{"nonempty_bounds",QJsonArray{0,0,size,size}},{"window",QJsonArray{window->width(),window->height()}},
            {"owner",double(owner)},{"layers",QJsonArray::fromVariantList(client.layers())},{"zoom",canvas->zoom()},
            {"dpr",window->devicePixelRatio()},{"dpi",window->screen()->logicalDotsPerInch()},{"screen",window->screen()->name()},
            {"api",int(window->rendererInterface()->graphicsApi())},{"rhi_disk_cache",qEnvironmentVariable("QSG_RHI_DISABLE_DISK_CACHE")}};
        const auto config=window->graphicsConfiguration();
        report["cache"]=QJsonObject{{"automatic_pipeline",config.isAutomaticPipelineCacheEnabled()},
            {"load_file",config.pipelineCacheLoadFile()},{"save_file",config.pipelineCacheSaveFile()},
            {"disable_shader_disk_cache_attribute",QCoreApplication::testAttribute(Qt::AA_DisableShaderDiskCache)},
            {"QSG_RHI_DISABLE_DISK_CACHE",qEnvironmentVariable("QSG_RHI_DISABLE_DISK_CACHE")},
            {"QML_DISABLE_DISK_CACHE",qEnvironmentVariable("QML_DISABLE_DISK_CACHE")},
            {"QML_DISK_CACHE_PATH",qEnvironmentVariable("QML_DISK_CACHE_PATH")}};
        // getResource is called on the render thread, not by the GUI observer.
        const auto rhiIdentity=std::make_shared<std::atomic<quintptr>>(0);
        const auto rhiConnection=connect(window,&QQuickWindow::afterSynchronizing,window,[window,rhiIdentity]{
            rhiIdentity->store(reinterpret_cast<quintptr>(window->rendererInterface()->getResource(window,QSGRendererInterface::RhiResource)));
        },Qt::DirectConnection);
        const auto disconnectRhi=qScopeGuard([&]{disconnect(rhiConnection);});
        InteractionTrace::start(tracing);
        const auto stopTrace=qScopeGuard([&]{trace=InteractionTrace::finish();});
        // Direct render callbacks only record the calling thread's time. No GUI
        // object/state reads, no GPU completion or frame/input correlation claim.
        connect(window,&QQuickWindow::beforeSynchronizing,window,[]{InteractionTrace::point("c3.render.beforeSync",0);},Qt::DirectConnection);
        connect(window,&QQuickWindow::afterSynchronizing,window,[]{InteractionTrace::point("c3.render.afterSync",0);},Qt::DirectConnection);
        connect(window,&QQuickWindow::beforeRendering,window,[]{InteractionTrace::point("c3.render.beforeRender",0);},Qt::DirectConnection);
        connect(window,&QQuickWindow::afterRendering,window,[]{InteractionTrace::point("c3.render.afterRender",0);},Qt::DirectConnection);
        connect(window,&QQuickWindow::frameSwapped,window,[]{InteractionTrace::point("c3.render.swapped",0);},Qt::DirectConnection);
        for(int phase=(direct && !pair)?1:0;phase<2;++phase){
            const bool prepared=pair || phase!=0, track=pair?phase==1:tracking;
            const QString label=pair?(phase?"W2-tracking":"W1-immediate"):(phase?"W":"I");
            PendingMoveEvidence evidence(canvas,&client,window,owner,4096,true);
            evidence.acceptResidentContextForProbe();
            const auto preparationStart=InteractionTrace::now();
            QJsonObject row{{"phase",label},{"prepare_start_ns",double(preparationStart)}};bool stored=false;
            const auto retainUnfinished=qScopeGuard([&]{if(!stored){row["terminal"]="assertion_or_preparation_incomplete";
                row["final_state"]=evidence.observation();row["evidence"]=evidence.evidenceData();rows.append(row);}});
            client.setMoveTool(true);
            if(prepared)QTRY_VERIFY_WITH_TIMEOUT(client.movePreparation() && client.movePreparation()->uploads->load()>0,60000);
            const auto assets=client.movePreparation();
            row=QJsonObject{{"phase",label},{"prepare_start_ns",double(preparationStart)},
                {"tracking",track},{"window_identity",QString::number(reinterpret_cast<quintptr>(window))},
                {"rhi_identity",QString::number(rhiIdentity->load())},{"prepare_request",double(client.movePreparationRequest())},{"assets_ready",bool(assets)},
                {"source",assets?double(assets->sourceId):0},{"uploads",assets?double(assets->uploads->load()):0},
                {"prepare_end_ns",double(InteractionTrace::now())},{"generation",double(client.generation())}};
            QJsonArray inputs,observations;QList<double> beats;int observationsDropped=0,beatsDropped=0;
            auto state=[&]{auto s=evidence.observation();if(observations.size()<4096)observations.append(s);else ++observationsDropped;return s;};
            const auto origin=screen({size*.3,size*.4});const QPoint target(384,0);
            evidence.expect(client.generation(),target,depth);
            qint64 press=0,planOrigin=0,holdDone=0,release=0,end=0,lastBeat=InteractionTrace::now();
            int sent=0;QString terminal="pending";const auto priorReceipt=client.moveReceipt();
            QEventLoop loop;QTimer timer,heartbeat,timeout;
            const auto send=[&](QEvent::Type type,QPoint delta,int index,qint64 planned){
                InteractionTrace::operation=phase*1000+index;
                const auto start=InteractionTrace::now();evidence.send(type,origin+QPointF(delta)*canvas->zoom());const auto finish=InteractionTrace::now();
                inputs.append(QJsonObject{{"id",index},{"plan_id",double(evidence.planId())},{"type",int(type)},{"planned_ns",double(planned)},
                    {"dispatch_ns",double(start)},{"end_ns",double(finish)},{"delta",QJsonArray{delta.x(),delta.y()}},
                    {"sequence_after",double(client.moveSequence())},{"move_after",QJsonObject::fromVariantMap(client.moveObservation())}});
            };
            const auto inspect=[&]{
                const auto s=state();const auto t=InteractionTrace::now();
                if(s["scenario_invalid"].toBool()){terminal="invalid_input_or_lifecycle";loop.quit();return;}
                if(!client.lastError().isEmpty()){terminal="product_error";loop.quit();return;}
                if(client.moveReceipt().sequence>priorReceipt.sequence && client.moveReceipt().kind==6){terminal="unexpected_cancel";loop.quit();return;}
                const auto move=client.moveObservation();const auto adopted=evidence.presentation()->snapshot().identity;
                const auto p=client.residentPreview();
                const bool sameContent=adopted.valid && adopted.generation==client.generation()
                    && (p?(adopted.resident && adopted.source==p->sourceId && adopted.revision==p->revision)
                         :(!adopted.resident && adopted.imageKey==client.frame().cacheKey() && client.frameRevision()>=client.revision()));
                if(sent==(track?120:1) && !holdDone && move["submittedKind"].toInt()==4 && !move["dirty"].toBool()
                    && move["completedSequence"].toULongLong()>=client.moveSequence() && sameContent){holdDone=t;loop.quit();}
                if(release && !client.layerEditBusy() && client.moveReceipt().kind==5 && client.undoDepth()==depth+1
                    && client.frameRevision()>=client.revision() && !p && sameContent){end=t;terminal="committed";loop.quit();}
            };
            // Twelve timer intervals of pause, followed by a turn: no input is
            // dispatched in the pause, and no missed positions are burst-caught-up.
            int ticks=0;
            connect(&timer,&QTimer::timeout,&loop,[&]{
                ++ticks;if(ticks>=49 && ticks<=60)return;
                ++sent;QPoint delta;
                if(sent<=48)delta={sent*4,0};
                else if(sent<=84)delta={192, (sent-48)*4};
                else delta={192+(sent-84)*192/36,144-(sent-84)*4};
                send(QEvent::MouseMove,delta,sent,planOrigin+qint64(ticks)*8000000);
                if(sent==120)timer.stop();
            });timer.setInterval(8);timer.setTimerType(Qt::PreciseTimer);
            heartbeat.setInterval(8);heartbeat.setTimerType(Qt::PreciseTimer);
            connect(&heartbeat,&QTimer::timeout,&loop,[&]{const auto t=InteractionTrace::now();if(beats.size()<20000)beats.append(double(t-lastBeat)/1e6);else ++beatsDropped;lastBeat=t;inspect();});
            timeout.setSingleShot(true);connect(&timeout,&QTimer::timeout,&loop,[&]{terminal="timeout";loop.quit();});
            press=InteractionTrace::now();send(QEvent::MouseButtonPress,{},0,press);planOrigin=InteractionTrace::now();
            if(track)timer.start();else{sent=1;send(QEvent::MouseMove,target,1,planOrigin);}
            heartbeat.start();timeout.start(90000);loop.exec();timer.stop();
            row["hold_end_ns"]=double(holdDone);row["hold_state"]=state();
            row["hold_terminal"]=holdDone?"adopted_target":terminal;
            bool pixels=false;
            if(holdDone){
                // Timing interval has ended. Same-image, stable render/grab
                // association and original white/red thresholds remain intact.
                QImage held;
                const bool observed=QTest::qWaitFor([&]{return (pixels=evidence.pendingMatches(held=evidence.capture())) || evidence.observation()["scenario_invalid"].toBool();},5000);
                if(pixels)QVERIFY(held.save(QFileInfo(output).absolutePath()+"/"+label+"-held.png"));
                if(!pixels)terminal=observed?"invalid_during_grab":"pixel_or_association_incomplete";
                // Keep observing ingress and lifecycle through Release/close.
            }
            if(holdDone && pixels){
                release=InteractionTrace::now();send(QEvent::MouseButtonRelease,target,121,release);loop.exec();
            }
            heartbeat.stop();timeout.stop();
            row["rhi_identity_after"]=QString::number(rhiIdentity->load());
            row["inputs"]=inputs;row["observations"]=observations;row["press_ns"]=double(press);row["release_ns"]=double(release);
            row["end_ns"]=double(end);row["terminal"]=terminal;row["held_pixels_and_association"]=pixels;
            row["observations_dropped"]=observationsDropped;row["heartbeat_dropped"]=beatsDropped;
            if(end){const auto image=client.frame();QVERIFY(image.save(QFileInfo(output).absolutePath()+"/"+label+"-final.png"));row["normal_frame_sha256"]=QString::fromLatin1(QCryptographicHash::hash(
                QByteArrayView(reinterpret_cast<const char*>(image.constBits()),image.sizeInBytes()),QCryptographicHash::Sha256).toHex());}
            row["heartbeat_ms"]=QJsonArray::fromVariantList([&]{QVariantList v;for(auto b:beats)v.append(b);return v;}());
            QJsonArray adoptions;for(const auto &r:evidence.presentation()->samples()){
                const auto &i=r.identity;adoptions.append(QJsonObject{{"ns",double(r.ns)},{"epoch",double(r.epoch)},{"thread",QString::number(r.thread)},
                    {"generation",double(i.generation)},{"source",double(i.source)},{"revision",double(i.revision)},{"image_key",QString::number(i.imageKey)},
                    {"resident",i.resident},{"valid",i.valid}});
            }row["adoptions"]=adoptions;row["adoptions_dropped"]=double(evidence.presentation()->overflow());
            row["final_state"]=state();row["evidence"]=evidence.evidenceData();rows.append(row);stored=true;
            QVERIFY2(terminal=="committed",qPrintable(terminal));QVERIFY(pixels);QCOMPARE(client.undoDepth(),depth+1);
            evidence.passed();
            client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy() && client.frameRevision()>=client.revision());
            client.setMoveTool(false);
        }
        trace=InteractionTrace::finish();report["passed"]=true;
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
};
int main(int argc,char **argv){
    if(qEnvironmentVariableIntValue("DRAWVERSE_C3_MAIN_QML")){configureUiScale();QQuickWindow::setDefaultAlphaBuffer(true);}
    QGuiApplication app(argc,argv);QQuickStyle::setStyle("Basic");
    qmlRegisterType<MenuSurface>("DrawVerse",1,0,"MenuSurface");qmlRegisterType<SelectionOverlay>("DrawVerse",1,0,"SelectionOutline");
    qmlRegisterType<DocumentManager>("DrawVerse",1,0,"DocumentManager");qmlRegisterType<CanvasItem>("DrawVerse",1,0,"PaintCanvas");qmlRegisterType<ColorWheelItem>("DrawVerse",1,0,"ColorWheel");app.setOrganizationName("DrawVerseTests");app.setApplicationName("MoveStartTracking");initializeDrawVerseUiResources();MoveStartTracking test;return QTest::qExec(&test,argc,argv);}
#include "move_start_tracking.moc"
