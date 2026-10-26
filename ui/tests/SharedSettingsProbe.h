// Manual investigation; included by the existing opt-in native probe only.
inline void runSharedSettingsProbe() {
    const auto output=qEnvironmentVariable("DRAWVERSE_SHARED_REPORT");
    if(output.isEmpty())QSKIP("Set DRAWVERSE_SHARED_REPORT and use isolated settings");
    const auto directory=QFileInfo(output).absolutePath();
    const auto preferences=directory+"/preferences.ini";
    const int count=8;
    const bool normalStartup=qEnvironmentVariableIntValue("DRAWVERSE_SHARED_NORMAL_STARTUP")!=0;
    PaintCoreClient services(nullptr,preferences);WorkspaceManager workspace(directory+"/workspace.ini");
    QTRY_VERIFY_WITH_TIMEOUT(services.ready() && services.brushLibraryReady() && services.brushCatalogPrepared(),30000);
    QVERIFY(services.brushLibrary()->select("round-pressure"));services.brushLibrary()->flushPersistence();
    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty("PaintClient",&services);engine.rootContext()->setContextProperty("Workspace",&workspace);
    engine.setInitialProperties({{"createInitialDocument",!normalStartup},{"visible",false}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
    auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);
    window->resize(1600,1000);window->showNormal();window->requestActivate();
    QVERIFY(QTest::qWaitForWindowExposed(window));QVERIFY(QTest::qWaitForWindowActive(window));
    auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);
    if(normalStartup)QVERIFY(!documents->newDocument(512,384,true).isEmpty());
    PaintCoreClient &client=*documents->activeClient();auto *library=client.brushLibrary();
    QTRY_VERIFY_WITH_TIMEOUT(client.ready() && client.brushLibraryReady() && !client.layerEditBusy(),30000);
    const auto original=documents->activeId();QVERIFY(!original.isEmpty());
    auto *panel=visual(window->contentItem(),"brushSettingsPanel");QVERIFY(panel);panel->setProperty("category",0);
    auto *wheel=qobject_cast<ColorWheelItem*>(visual(window->contentItem(),"colorWheel"));QVERIFY(wheel);
    QJsonArray rows,beatEvents;QList<double> beats;QTimer heartbeat;heartbeat.setInterval(8);heartbeat.setTimerType(Qt::PreciseTimer);
    auto previous=InteractionTrace::now();
    QObject::connect(&heartbeat,&QTimer::timeout,&client,[&]{const auto now=InteractionTrace::now();const auto ms=double(now-previous)/1e6;beats.append(ms);beatEvents.append(QJsonObject{{"ns",double(now)},{"gap_ms",ms}});previous=now;});
    QSignalSpy persisted(library,&BrushLibrary::persistRequested);
    QList<PaintCoreClient*> receivers;int receiverRebuilds=0;
    if(normalStartup){receivers.append(&services);QObject::connect(services.brushLibrary(),&BrushLibrary::presetsChanged,&client,[&]{++receiverRebuilds;});}
    QJsonObject report{{"passed",false},{"preset_count",library->presets().size()},{"isolation",library->toolIsolation()},
        {"renderer",int(window->rendererInterface()->graphicsApi())},{"dpr",window->devicePixelRatio()},{"normal_startup",normalStartup}};
    const auto retain=qScopeGuard([&]{heartbeat.stop();report["rows"]=rows;report["trace"]=InteractionTrace::finish();report["trace_dropped"]=double(InteractionTrace::dropped);
        QFile file(output);if(file.open(QIODevice::WriteOnly|QIODevice::NewOnly))file.write(QJsonDocument(report).toJson());});
    ParameterIngress ingress;ingress.window=window;
    QTest::qWait(600);InteractionTrace::start(true);
    const int maxDocuments=qEnvironmentVariableIntValue("DRAWVERSE_SHARED_MAX_DOCUMENTS")>0?qEnvironmentVariableIntValue("DRAWVERSE_SHARED_MAX_DOCUMENTS"):3;
    for(int documentCount=1;documentCount<=maxDocuments;++documentCount) {
        if(documentCount>1) {
            const auto id=documents->newDocument(64,64,true);QVERIFY(!id.isEmpty());auto *receiver=documents->client(id);QVERIFY(receiver);
            QTRY_VERIFY_WITH_TIMEOUT(receiver->ready() && receiver->brushLibraryReady() && !receiver->layerEditBusy(),30000);
            receivers.append(receiver);
            QObject::connect(receiver->brushLibrary(),&BrushLibrary::presetsChanged,&client,[&]{++receiverRebuilds;});
            QVERIFY(documents->activate(original));QTest::qWait(600);
        }
        for(const QString field:{QString("color"),QString("radius"),QString("flow"),QString("opacity")}) {
            QQuickItem *control=nullptr,*track=nullptr;
            if(field!="color") {
                const auto name=field=="radius"?QString("settingsBrushSizeSlider"):field=="flow"?QString("brushToolbarFlow"):QString("brushToolbarOpacity");
                QTRY_VERIFY((control=visual(window->contentItem(),name)) && control->width()>20);
                track=field=="radius"?visual(window->contentItem(),name+"Track"):control;QVERIFY(track);
            }
            const auto at=[&](double fraction) {
                if(field=="color") {
                    const double radius=(std::min(wheel->width(),wheel->height())-6)*.4625;
                    const double angle=fraction*6.283185307179586;
                    return wheel->mapToScene(QPointF(wheel->width()/2+radius*std::cos(angle),wheel->height()/2+radius*std::sin(angle)));
                }
                auto *handle=track->property("handle").value<QQuickItem*>();
                return track->mapToScene(QPointF(track->property("leftPadding").toDouble()+handle->width()/2+fraction*(track->property("availableWidth").toDouble()-handle->width()),track->height()/2));
            };
            const auto model=[&]{return field=="radius"?library->radius():field=="flow"?library->flow():library->opacity();};
            for(int interval:{8,50}) {
                ingress.armed=true;ingress.click(at(.23));ingress.armed=false;QTest::qWait(500);
                const auto firstColor=client.brushColor();const auto firstValue=model();
                const int p0=persisted.size(),r0=receiverRebuilds;beats.clear();beatEvents={};previous=InteractionTrace::now();heartbeat.start();
                QList<double> dispatch;QJsonArray inputs;const auto start=InteractionTrace::now();ingress.armed=true;
                ingress.send(QEvent::MouseButtonPress,at(.23));
                for(int i=0;i<count;++i) {
                    const auto t=InteractionTrace::now();
                    {InteractionTrace::Scope scope("shared.input.dispatch");ingress.send(QEvent::MouseMove,at(.28+.48*i/(count-1)));}
                    const auto end=InteractionTrace::now();dispatch.append(double(end-t)/1000);
                    inputs.append(QJsonObject{{"start_ns",double(t)},{"end_ns",double(end)}});
                    QTest::qWait(interval);
                }
                ingress.send(QEvent::MouseButtonRelease,at(.76));ingress.armed=false;
                const auto released=InteractionTrace::now();
                const auto wantedColor=client.brushColor();const auto wantedValue=model();
                // Never read/sync the INI while collecting heartbeats: QSettings
                // can block against the writer and would measure our observer.
                QTRY_VERIFY_WITH_TIMEOUT(!library->persistencePending(),30000);
                QTest::qWait(250);heartbeat.stop();const auto end=InteractionTrace::now();
                bool saved=false;
                QTRY_VERIFY_WITH_TIMEOUT(([&]{QSettings ini(preferences,QSettings::IniFormat);ini.sync();const auto root=QJsonDocument::fromJson(ini.value("brushes/state").toByteArray()).object();
                    if(field=="color")saved=root.value("colorState").toObject().value("foreground").toString()==wantedColor.name(QColor::HexArgb);
                    else {saved=false;for(const auto &v:root.value("presets").toArray()){const auto row=v.toObject();if(row.value("id").toString()==library->selectedId())saved=std::abs(row.value(field).toDouble()-wantedValue)<.00001;}}
                    return saved && !library->persistencePending();})(),30000);
                const bool changed=field=="color"?wantedColor!=firstColor:wantedValue!=firstValue;
                bool receiversCorrect=true;
                for(auto *receiver:receivers) {
                    receiversCorrect &= field=="color"?receiver->brushColor().rgba()==wantedColor.rgba():std::abs((field=="radius"?receiver->brushLibrary()->radius():field=="flow"?receiver->brushLibrary()->flow():receiver->brushLibrary()->opacity())-wantedValue)<.00001;
                }
                rows.append(QJsonObject{{"documents",documentCount},{"field",field},{"interval_ms",interval},{"attempts",count},{"inputs",inputs},
                    {"start_ns",double(start)},{"release_ns",double(released)},{"end_ns",double(end)},{"dispatch_us",stats(dispatch)},{"heartbeat_ms",stats(beats)},{"beats",beatEvents},
                    {"persist_requests",persisted.size()-p0},{"receiver_rebuilds",receiverRebuilds-r0},{"changed",changed},{"saved",saved},{"receivers_correct",receiversCorrect},{"input_invalid",ingress.invalid}});
                report["input_events"]=ingress.inputs;
                QVERIFY(changed && saved && receiversCorrect && !ingress.invalid);QVERIFY(client.lastError().isEmpty());
            }
        }
    }
    report["passed"]=true;
    QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE_WITH_TIMEOUT(stopped.size(),1,30000);
}
