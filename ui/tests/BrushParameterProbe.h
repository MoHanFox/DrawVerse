#pragma once
// Opt-in current-version parameter probe; historical A3 remains unchanged.
class ParameterIngress final : public QObject {
public:
    QQuickWindow *window=nullptr;const QEvent *planned=nullptr;bool armed=false,invalid=false;
    quint64 next=0;QJsonArray inputs;int dropped=0;
    ParameterIngress(){qApp->installEventFilter(this);}
    ~ParameterIngress(){qApp->removeEventFilter(this);}
    void send(QEvent::Type type,QPointF position){
        QMouseEvent event(type,position,window->mapToGlobal(position.toPoint()),
            type==QEvent::MouseMove?Qt::NoButton:Qt::LeftButton,
            type==QEvent::MouseButtonRelease?Qt::NoButton:Qt::LeftButton,Qt::NoModifier);
        planned=&event;++next;QCoreApplication::sendEvent(window,&event);planned=nullptr;
    }
    void click(QPointF p){send(QEvent::MouseButtonPress,p);send(QEvent::MouseButtonRelease,p);}
protected:
    bool eventFilter(QObject *receiver,QEvent *event) override {
        if(!armed)return false;
        const auto type=event->type();
        const bool lifecycle=type==QEvent::ApplicationDeactivate || (receiver==window &&
            (type==QEvent::WindowDeactivate || type==QEvent::Hide || type==QEvent::Resize || type==QEvent::Move));
        const bool input=receiver==window && (type==QEvent::MouseButtonPress || type==QEvent::MouseButtonRelease ||
            type==QEvent::MouseMove || type==QEvent::Wheel || type==QEvent::TabletPress || type==QEvent::TabletMove ||
            type==QEvent::TabletRelease || type==QEvent::KeyPress || type==QEvent::KeyRelease);
        if(input || lifecycle){
            const bool expected=event==planned && receiver==window;
            if(!expected)invalid=true;
            QJsonObject row{{"ns",double(InteractionTrace::now())},{"type",int(type)},
                {"planned_id",expected?double(next):0},{"planned_exact_event",expected},{"lifecycle",lifecycle}};
            if(expected){const auto *m=static_cast<QMouseEvent*>(event);row["x"]=m->position().x();row["y"]=m->position().y();row["buttons"]=int(m->buttons());}
            if(inputs.size()<20000)inputs.append(row);else ++dropped;
        }
        return false; // Observation never suppresses input or cancellation.
    }
};

inline void runBrushParameterProbe(){
    const auto output=qEnvironmentVariable("DRAWVERSE_INTERACTION_REPORT");QVERIFY(!output.isEmpty());
    const auto directory=QFileInfo(output).absolutePath(),preferences=directory+"/preferences.ini";
    const int count=qEnvironmentVariableIntValue("DRAWVERSE_INTERACTION_COUNT");QVERIFY(count>=2);
    const bool tracing=qEnvironmentVariableIntValue("DRAWVERSE_INTERACTION_TRACE")!=0;
    const bool controls=qEnvironmentVariableIntValue("DRAWVERSE_PARAMETER_CONTROLS")!=0;
    QJsonObject report{{"schema",2},{"product_baseline","a7fb6773cce94769a07bc7a1a6bf274bc7088ca3"},
        {"trace_enabled",tracing},{"passed",false}};QJsonArray rows;
    ParameterIngress ingress;
    const auto retain=qScopeGuard([&]{report["rows"]=rows;report["input_events"]=ingress.inputs;
        report["input_dropped"]=ingress.dropped;report["invalid_input_or_lifecycle"]=ingress.invalid;
        report["trace"]=InteractionTrace::finish();report["trace_dropped"]=double(InteractionTrace::dropped);
        QFile file(output);if(file.open(QIODevice::WriteOnly|QIODevice::NewOnly))file.write(QJsonDocument(report).toJson());});
    PaintCoreClient client(nullptr,preferences);WorkspaceManager workspace(directory+"/workspace.ini");
    QTRY_VERIFY_WITH_TIMEOUT(client.ready() && client.brushLibraryReady() && client.brushCatalogPrepared(),15000);
    auto *library=client.brushLibrary();
    if(qEnvironmentVariableIntValue("DRAWVERSE_INTERACTION_PRESETS")==256){
        while(library->presets().size()<256){library->select("round-pressure");QVERIFY(!library->saveCopy(QString("issue2a-%1").arg(library->presets().size(),3,10,QChar('0'))).isEmpty());}
        library->select("round-pressure");
    }
    library->flushPersistence();
    QQmlApplicationEngine engine;const auto disarmBeforeTeardown=qScopeGuard([&]{ingress.armed=false;});
    engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
    engine.setInitialProperties({{"createInitialDocument",true},{"visible",false}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
    auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);ingress.window=window;
    window->resize(1280,800);window->showNormal();window->requestActivate();
    QVERIFY(QTest::qWaitForWindowExposed(window));QVERIFY(QTest::qWaitForWindowActive(window));
    auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);
    client.newDocument(512,384);QTRY_COMPARE(client.documentWidth(),512);QTRY_VERIFY(!client.layerEditBusy());canvas->actualSize();
    auto *panel=visual(window->contentItem(),"brushSettingsPanel");QVERIFY(panel);panel->setProperty("category",0);
    QTRY_VERIFY(client.frameRevision()>=client.revision());
    QCOMPARE(window->rendererInterface()->graphicsApi(),QSGRendererInterface::Direct3D11);
    report["environment"]=QJsonObject{{"renderer","initialized D3D11"},{"dpr",window->devicePixelRatio()},
        {"logical_dpi",window->screen()->logicalDotsPerInch()},{"window_width",window->width()},{"window_height",window->height()},
        {"zoom",canvas->zoom()},{"canvas_size",QJsonArray{512,384}},{"preset_count",library->presets().size()},{"selected",library->selectedId()}};
    int settings=0,brush=0,persist=0,colors=0;auto lastSignal=InteractionTrace::now();
    QObject::connect(library,&BrushLibrary::settingsChanged,&client,[&]{++settings;lastSignal=InteractionTrace::now();});
    QObject::connect(&client,&PaintCoreClient::brushChanged,&client,[&]{++brush;lastSignal=InteractionTrace::now();});
    QObject::connect(library,&BrushLibrary::persistRequested,&client,[&]{++persist;lastSignal=InteractionTrace::now();});
    QObject::connect(library,&BrushLibrary::activeColorsChanged,&client,[&]{++colors;lastSignal=InteractionTrace::now();});
    QList<double> heartbeat;QTimer beat;beat.setInterval(8);beat.setTimerType(Qt::PreciseTimer);auto lastBeat=InteractionTrace::now();int heartbeatDropped=0;
    QObject::connect(&beat,&QTimer::timeout,&client,[&]{const auto t=InteractionTrace::now();if(heartbeat.size()<20000)heartbeat.append(double(t-lastBeat)/1e6);else ++heartbeatDropped;lastBeat=t;});beat.start();
    const auto saved=[&](const QString &field){QSettings ini(preferences,QSettings::IniFormat);ini.sync();const auto root=QJsonDocument::fromJson(ini.value("brushes/state").toByteArray()).object();
        for(const auto &v:root["presets"].toArray()){const auto row=v.toObject();if(row["id"].toString()==library->selectedId())return row[field].toDouble(-1);}return -1.;};
    const auto quiet=[&]{return !library->persistencePending() && InteractionTrace::now()-lastSignal>80000000;};
    QVERIFY(QTest::qWaitFor(quiet,15000));ingress.armed=true;InteractionTrace::start(tracing);
    QStringList fields{"radius"};if(controls)fields<<"flow"<<"opacity";
    int operation=0;
    for(const auto &field:fields){
        const auto name=field=="radius"?QString("settingsBrushSizeSlider"):field=="flow"?QString("brushToolbarFlow"):QString("brushToolbarOpacity");
        QQuickItem *control=nullptr;QTRY_VERIFY((control=visual(window->contentItem(),name)) && control->width()>20);
        auto *track=field=="radius"?visual(window->contentItem(),name+"Track"):control;QVERIFY(track);
        auto *handle=track->property("handle").value<QQuickItem*>();QVERIFY(handle);
        const auto at=[&](double f){return track->mapToScene(QPointF(track->property("leftPadding").toDouble()+handle->width()/2+f*(track->property("availableWidth").toDouble()-handle->width()),track->height()/2));};
        const auto oracle=[&](double f){return field=="radius"?(1+std::round(std::exp(std::log(2.)+f*(std::log(2001.)-std::log(2.)))-2))/2:std::round(f*100)/100;};
        const auto model=[&]{return field=="radius"?library->radius():field=="flow"?library->flow():library->opacity();};
        const auto correct=[&](double expected){return std::abs(model()-expected)<.00001 && std::abs(control->property("value").toDouble()/(field=="radius"?2:1)-expected)<.00001;};
        for(const QString mode:{QString("same"),QString("discrete"),QString("drag")}){
            ingress.click(at(.2));QVERIFY(QTest::qWaitFor([&]{return quiet() && std::abs(saved(field)-oracle(.2))<.00001;},15000));
            if(mode=="drag"){ingress.click(at(.38));QVERIFY(correct(oracle(.38)));QVERIFY(QTest::qWaitFor(quiet,15000));}
            QJsonArray samples;QList<double> warm,gaps;int failures=0,timeouts=0,changes=0,completed=0;
            const auto start=InteractionTrace::now();auto prior=start,lastEnd=start;const int s0=settings,b0=brush,p0=persist,c0=colors,h0=heartbeat.size();
            double expected=oracle(.2),releaseUs=0;
            for(int i=0;i<count;++i){
                if(ingress.invalid)break;
                InteractionTrace::operation=++operation;const auto previous=model();const auto t=InteractionTrace::now();if(i)gaps.append(double(t-prior)/1e6);prior=t;
                // Avoid exact .005 ties in CompactSlider's .01 quantization:
                // the deterministic trajectory is slightly above interior ties,
                // with the original independent start/final values unchanged.
                double fraction=mode=="same"?.2:mode=="discrete"?(i%2?.2:.6):.38+(.83-.38)*i/(count-1);
                if(field!="radius" && mode=="drag" && i>0 && i<count-1)fraction+=.00001;
                expected=oracle(fraction);
                {InteractionTrace::Scope dispatch("input.dispatch");if(mode=="drag"){if(i==0)ingress.send(QEvent::MouseButtonPress,at(.38));ingress.send(QEvent::MouseMove,at(fraction));}else ingress.click(at(fraction));}
                lastEnd=InteractionTrace::now();const double syncUs=double(lastEnd-t)/1000;if(i)warm.append(syncUs);
                const bool valid=correct(expected);if(std::abs(previous-model())>.00001)++changes;if(!valid)++failures;
                bool done=true;
                if(mode=="drag" || mode=="same")QTest::qWait(8);
                else done=QTest::qWaitFor([&]{return ingress.invalid || (quiet() && std::abs(saved(field)-expected)<.00001);},15000);
                if(!done)++timeouts;else if(valid && !ingress.invalid)++completed;
                samples.append(QJsonObject{{"operation",operation},{"first",i==0},{"start_ns",double(t)},{"end_ns",double(lastEnd)},
                    {"sync_us",syncUs},{"expected",expected},{"model_immediate",model()},{"valid",valid},{"timeout",!done}});
                if(!valid || !done || ingress.invalid)break;
            }
            if(mode=="drag" && !ingress.invalid && !failures && !timeouts && samples.size()==count){const auto t=InteractionTrace::now();ingress.send(QEvent::MouseButtonRelease,at(.83));lastEnd=InteractionTrace::now();releaseUs=double(lastEnd-t)/1000;}
            const bool persisted=!failures && !timeouts && QTest::qWaitFor([&]{return ingress.invalid || (quiet() && std::abs(saved(field)-expected)<.00001);},15000);
            const bool finalValid=!ingress.invalid && persisted && correct(expected) && std::abs(saved(field)-expected)<.00001;
            QList<double> beats;for(auto j=h0;j<heartbeat.size();++j)beats.append(heartbeat[j]);
            rows.append(QJsonObject{{"scene",field+"-"+mode},{"start_ns",double(start)},{"end_ns",double(InteractionTrace::now())},
                {"requested",count},{"attempts",samples.size()},{"completed",completed},{"actual_changes",changes},{"failures",failures},{"timeouts",timeouts+(!persisted)},
                {"samples",samples},{"warm_sync_us",stats(warm)},{"actual_input_gap_ms",stats(gaps)},{"heartbeat_ms",stats(beats)},
                {"post_input_quiescence_observation_ms",double(InteractionTrace::now()-lastEnd)/1e6},{"release_sync_us",releaseUs},
                {"settingsChanged",settings-s0},{"brushChanged",brush-b0},{"persistRequested",persist-p0},{"activeColorsChanged",colors-c0},
                {"expected_final",expected},{"model_final",model()},{"control_final",control->property("value").toDouble()/(field=="radius"?2:1)},
                {"saved_final",saved(field)},{"final_valid",finalValid}});
            QVERIFY2(finalValid && completed==count && !failures && !timeouts,"Parameter target/input contract failed; retained raw report");
        }
    }
    // Limited pressure comparison, using the existing real toolbar checkbox.
    if(controls){
        auto *box=visual(window->contentItem(),"brushToolbarPressureOpacity");QVERIFY(box);
        const auto before=library->pressureOpacity();const auto start=InteractionTrace::now();
        ingress.click(box->mapToScene({box->width()/2,box->height()/2}));const auto end=InteractionTrace::now();
        const bool immediate=library->pressureOpacity()!=before;QVERIFY(immediate);QVERIFY(QTest::qWaitFor(quiet,15000));
        report["pressure_control"]=QJsonObject{{"field","pressureOpacity"},{"sync_us",double(end-start)/1000},{"changed",immediate}};
    }
    report["error"]=client.lastError();report["heartbeat_dropped"]=heartbeatDropped;report["passed"]=!ingress.invalid && client.lastError().isEmpty();
    beat.stop();ingress.armed=false;canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    QVERIFY(report["passed"].toBool());
}
