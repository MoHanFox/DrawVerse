// Opt-in native interaction investigation; no automatic CTest registration.
#include "platform/UiResources.h"
#include "platform/InteractionTrace.h"
#include "platform/MenuSurface.h"
#include "canvas/CanvasItem.h"
#include "canvas/SelectionOverlay.h"
#include "color/ColorWheelItem.h"
#include "documents/DocumentManager.h"
#include "workspace/WorkspaceManager.h"
#include <QtTest>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QScreen>
#include <QJsonDocument>
#include <QJsonArray>
#include <QFile>
#include <QDir>
#include <QSettings>
#include <functional>

namespace {
QQuickItem *visual(QQuickItem *item,const QString &name) {
    if(item->objectName()==name && item->isVisible())return item;
    for(auto *child:item->childItems())if(auto *found=visual(child,name))return found;
    return nullptr;
}
void mouse(QQuickWindow *window,QEvent::Type type,QPointF p,Qt::MouseButton button,Qt::MouseButtons buttons) {
    QMouseEvent event(type,p,window->mapToGlobal(p.toPoint()),button,buttons,Qt::NoModifier);
    QCoreApplication::sendEvent(window,&event);
}
void click(QQuickWindow *w,QPointF p) {
    mouse(w,QEvent::MouseMove,p,Qt::NoButton,Qt::NoButton);
    mouse(w,QEvent::MouseButtonPress,p,Qt::LeftButton,Qt::LeftButton);
    mouse(w,QEvent::MouseButtonRelease,p,Qt::LeftButton,Qt::NoButton);
}
double percentile(QList<double> values,double fraction) {
    if(values.isEmpty())return 0;std::sort(values.begin(),values.end());return values[qsizetype(std::ceil(fraction*values.size()))-1];
}
QJsonObject stats(const QList<double> &values) {
    return {{"n",values.size()},{"p50",percentile(values,.5)},{"p95",percentile(values,.95)},{"max",values.isEmpty()?0:*std::max_element(values.begin(),values.end())}};
}
}
class BrushInteractionBaseline : public QObject {
    Q_OBJECT
private slots:
    void dragFinalValues() {
        const auto output=qEnvironmentVariable("DRAWVERSE_INTERACTION_FINAL_REPORT");
        if(output.isEmpty())QSKIP("Independent closure regression; set DRAWVERSE_INTERACTION_FINAL_REPORT");
        const auto directory=QFileInfo(output).absolutePath();QVERIFY(QDir().mkpath(directory));
        const auto preferences=directory+"/preferences.ini";
        PaintCoreClient client(nullptr,preferences);WorkspaceManager workspace(directory+"/workspace.ini");
        QTRY_VERIFY_WITH_TIMEOUT(client.ready() && client.brushLibraryReady() && client.brushCatalogPrepared(),15000);
        auto *library=client.brushLibrary();
        QQmlApplicationEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);
        window->showNormal();window->resize(1280,800);window->requestActivate();
        QVERIFY(QTest::qWaitForWindowExposed(window));QVERIFY(QTest::qWaitForWindowActive(window));
        auto *panel=visual(window->contentItem(),"brushSettingsPanel");QVERIFY(panel);panel->setProperty("category",0);
        QJsonArray results;bool passed=true;
        for(const QString field:{QString("radius"),QString("flow"),QString("opacity")}) {
            const auto name=field=="radius"?QString("settingsBrushSizeSlider"):field=="flow"?QString("brushToolbarFlow"):QString("brushToolbarOpacity");
            QQuickItem *control=nullptr,*track=nullptr;
            QTRY_VERIFY((control=visual(window->contentItem(),name)) && control->width()>20);
            track=field=="radius"?visual(window->contentItem(),name+"Track"):control;QVERIFY(track);
            auto *handle=track->property("handle").value<QQuickItem*>();QVERIFY(handle);
            // Qt 6.8 Slider::positionAt: handle centre over availableWidth-handleWidth.
            const auto at=[&](double fraction){return track->mapToScene(QPointF(track->property("leftPadding").toDouble()+handle->width()/2+fraction*(track->property("availableWidth").toDouble()-handle->width()),track->height()/2));};
            // Independent oracle: MappedSlider log1p/expm1 then unit diameter
            // rounding (from=1, step=1), or CompactSlider's .01 quantization.
            const auto expected=[&](double fraction){return field=="radius"?(1+std::round(std::exp(std::log(2.)+fraction*(std::log(2001.)-std::log(2.)))-2))/2:std::round(fraction*100)/100;};
            const auto model=[&]{return field=="radius"?library->radius():field=="flow"?library->flow():library->opacity();};
            const auto saved=[&]{QSettings settings(preferences,QSettings::IniFormat);settings.sync();const auto root=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &value:root.value("presets").toArray()){const auto row=value.toObject();if(row.value("id").toString()==library->selectedId())return row.value(field).toDouble(-1);}return -1.;};
            const double previous=expected(.2),start=expected(.38),target=expected(.83);
            QVERIFY(target!=previous && target!=start && previous!=start);
            click(window,at(.2));QTRY_VERIFY_WITH_TIMEOUT(std::abs(saved()-previous)<.00001 && !library->persistencePending(),15000);
            click(window,at(.38));QCOMPARE(model(),start);
            mouse(window,QEvent::MouseButtonPress,at(.38),Qt::LeftButton,Qt::LeftButton);
            for(int i=1;i<=16;++i){mouse(window,QEvent::MouseMove,at(.38+(.83-.38)*i/16),Qt::NoButton,Qt::LeftButton);QTest::qWait(8);}
            mouse(window,QEvent::MouseButtonRelease,at(.83),Qt::LeftButton,Qt::NoButton);
            // Completion is the actual isolated INI write of the independent
            // target, not a sleep, signal-quiet guess or model/control equality.
            const bool persisted=QTest::qWaitFor([&]{return !library->persistencePending() && std::abs(saved()-target)<.00001;},15000);
            const auto controlValue=control->property("value").toDouble()/(field=="radius"?2:1);
            const bool valid=persisted && std::abs(model()-target)<.00001 && std::abs(controlValue-target)<.00001;
            passed=passed && valid;
            results.append(QJsonObject{{"field",field},{"previous_saved",previous},{"start",start},{"target",target},{"model",model()},{"control",controlValue},{"saved",saved()},{"persisted_target",persisted},{"passed",valid}});
        }
        const QJsonObject report{{"input","synthetic real QML drag + Release, 16 moves, requested 8ms gaps"},{"renderer",int(window->rendererInterface()->graphicsApi())},{"dpr",window->devicePixelRatio()},{"results",results},{"passed",passed}};
        QFile file(output);QVERIFY(file.open(QIODevice::WriteOnly));file.write(QJsonDocument(report).toJson());file.close();
        if(auto *canvas=window->findChild<CanvasItem*>("mainCanvas"))canvas->setClient(nullptr);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        QVERIFY2(passed,"A closure blocked: model, QML control and persisted state must each equal the independent target; see final-value report");
    }
    void matrix() {
        const QString output=qEnvironmentVariable("DRAWVERSE_INTERACTION_REPORT");
        QVERIFY2(!output.isEmpty(),"Set DRAWVERSE_INTERACTION_REPORT to an isolated report path");
        const int sampleCount=qEnvironmentVariableIntValue("DRAWVERSE_INTERACTION_COUNT")>0?qEnvironmentVariableIntValue("DRAWVERSE_INTERACTION_COUNT"):51;
        const auto directory=QFileInfo(output).absolutePath();QVERIFY(QDir().mkpath(directory));
        PaintCoreClient client(nullptr,directory+"/preferences.ini");WorkspaceManager workspace(directory+"/workspace.ini");
        QTRY_VERIFY_WITH_TIMEOUT(client.ready() && client.brushLibraryReady() && client.brushCatalogPrepared(),15000);
        auto *library=client.brushLibrary();const bool large=qEnvironmentVariableIntValue("DRAWVERSE_INTERACTION_PRESETS")==256;
        if(large) {
            while(library->presets().size()<256) {library->select("round-pressure");QVERIFY(!library->saveCopy(QString("issue2a-%1").arg(library->presets().size(),3,10,QChar('0'))).isEmpty());}
            library->select("round-pressure");
        }
        library->flushPersistence();QTest::qWait(500);
        QQmlApplicationEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);window->showNormal();window->resize(1280,800);window->requestActivate();
        QVERIFY(QTest::qWaitForWindowExposed(window));QVERIFY(QTest::qWaitForWindowActive(window));
        QTRY_VERIFY(window->findChild<CanvasItem*>("mainCanvas"));auto *canvas=window->findChild<CanvasItem*>("mainCanvas");
        client.newDocument(512,384);QTRY_COMPARE(client.documentWidth(),512);QTRY_VERIFY(!client.layerEditBusy());
        library->setPressureSize(false);library->setPressureFlow(false);library->setPressureOpacity(false);client.setBrushRadius(48);client.setBrushOpacity(1);library->setFlow(1);
        int patchDepth=client.undoDepth();
        for(const auto &patch:QList<QPair<QPointF,QColor>>{{{128,192},Qt::red},{{384,192},Qt::blue}}) {
            client.setBrushColor(patch.second);QVERIFY(client.beginStroke({patch.first,1}));client.endStroke();++patchDepth;QTRY_COMPARE(client.undoDepth(),patchDepth);QTRY_VERIFY(!client.layerEditBusy() && !client.drawing());
        }
        QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(client.sampleDocumentPixel(128,192),QColor(Qt::red));QTRY_COMPARE(client.sampleDocumentPixel(384,192),QColor(Qt::blue));
        // Untimed diagnostic-off drawing/undo/redo smoke before every measurement.
        client.undo();QTRY_COMPARE(client.undoDepth(),patchDepth-1);QTRY_VERIFY(!client.layerEditBusy());
        QTRY_COMPARE(client.sampleDocumentPixel(384,192),QColor(Qt::white));
        client.redo();QTRY_COMPARE(client.undoDepth(),patchDepth);QTRY_VERIFY(!client.layerEditBusy());
        QTRY_COMPARE(client.sampleDocumentPixel(384,192),QColor(Qt::blue));
        canvas->actualSize();QTest::qWait(500);
        const auto api=window->rendererInterface()->graphicsApi();QVERIFY2(api==QSGRendererInterface::Direct3D11,"Main measurement requires initialized D3D11, not software/offscreen");
        auto *panel=visual(window->contentItem(),"brushSettingsPanel");QVERIFY(panel);
        QJsonObject report{{"schema",1},{"pid",double(QCoreApplication::applicationPid())},{"baseline",QStringLiteral(DRAWVERSE_INTERACTION_BASELINE)},{"tool_revision",QStringLiteral(DRAWVERSE_INTERACTION_REVISION)},
            {"renderer","initialized QSGRendererInterface::Direct3D11"},{"platform",QGuiApplication::platformName()},{"dpr",window->devicePixelRatio()},
            {"logical_dpi",window->screen()->logicalDotsPerInch()},{"physical_dpi",window->screen()->physicalDotsPerInch()},
            {"window_width",window->width()},{"window_height",window->height()},{"canvas_width",512},{"canvas_height",384},{"zoom",canvas->zoom()},
            {"preset_count",library->presets().size()},{"selected",library->selectedId()},{"input","synthetic Qt window mouse/key events"}};
        int colors=0,settings=0,brush=0,persist=0; qint64 lastSignal=InteractionTrace::now();
        const auto signal=[&]{lastSignal=InteractionTrace::now();};
        connect(library,&BrushLibrary::activeColorsChanged,this,[&]{++colors;signal();});
        connect(library,&BrushLibrary::settingsChanged,this,[&]{++settings;signal();});
        connect(&client,&PaintCoreClient::brushChanged,this,[&]{++brush;signal();});
        connect(library,&BrushLibrary::persistRequested,this,[&]{++persist;signal();});
        QList<double> heartbeat;QTimer heartbeatTimer;heartbeatTimer.setInterval(8);heartbeatTimer.setTimerType(Qt::PreciseTimer);qint64 lastBeat=InteractionTrace::now();
        connect(&heartbeatTimer,&QTimer::timeout,this,[&]{const auto t=InteractionTrace::now();if(heartbeat.size()<200000)heartbeat.append(double(t-lastBeat)/1e6);lastBeat=t;});
        heartbeatTimer.start();
        QJsonArray rows;int rowId=0;
        const auto values=[&]{return QVariantList{client.brushColor(),library->radius(),library->flow(),library->opacity(),library->pressureSize(),library->pressureFlow(),library->pressureOpacity()};};
        const auto settled=[&]{return !library->persistencePending() && InteractionTrace::now()-lastSignal>80000000;};
        const auto scene=[&](QString name,int count,int interval,bool continuous,const std::function<bool(int)> &action,const std::function<void()> &finish={}) {
            QTest::qWait(200);QJsonArray samples;QList<double> sync,gaps;int failures=0,timeouts=0,completed=0,changes=0;const int c0=colors,s0=settings,b0=brush,p0=persist;
            const int hb0=heartbeat.size();qint64 previous=0,lastEnd=0;const auto begin=InteractionTrace::now();
            InteractionTrace::operation=++rowId*1000;
            for(int i=0;i<count;++i) {
                InteractionTrace::operation=rowId*1000+i;
                const auto beforeState=values();
                const auto t=InteractionTrace::now();if(previous)gaps.append(double(t-previous)/1e6);previous=t;
                const int cs=colors,ss=settings,bs=brush;
                bool valid=false;{InteractionTrace::Scope scope("input.dispatch");valid=action(i);}lastEnd=InteractionTrace::now();const auto elapsed=double(lastEnd-t)/1000;
                if(i)sync.append(elapsed);
                if(!continuous)QTest::qWait(qMax(0,125-int((InteractionTrace::now()-t)/1000000)));
                bool timeout=false;if(!continuous && !QTest::qWaitFor(settled,5000)){++timeouts;timeout=true;}
                const bool changed=beforeState!=values();if(changed)++changes;
                if(!valid)++failures;else if(!timeout)++completed;
                samples.append(QJsonObject{{"operation",InteractionTrace::operation},{"first",i==0},{"start_ns",double(t)},{"sync_us",elapsed},{"valid",valid},{"actual_changed",changed},
                    {"color_changes",colors-cs},{"settings_notifications",settings-ss},{"brush_notifications",brush-bs},{"timeout",timeout}});
                if(continuous)QTest::qWait(interval);
            }
            double releaseUs=0;if(finish){const auto t=InteractionTrace::now();finish();lastEnd=InteractionTrace::now();releaseUs=double(lastEnd-t)/1000;}
            const bool drained=QTest::qWaitFor(settled,5000);if(!drained)++timeouts;
            QList<double> beats;for(qsizetype i=hb0;i<heartbeat.size();++i)beats.append(heartbeat[i]);
            rows.append(QJsonObject{{"scene",name},{"attempts",count},{"completed",completed},{"actual_changes",changes},{"failures",failures},{"timeouts",timeouts},{"samples",samples},
                {"release_sync_us",releaseUs},{"warm_sync_us",stats(sync)},{"actual_input_gap_ms",stats(gaps)},{"heartbeat_ms",stats(beats)},
                {"post_input_quiescence_observation_ms",double(InteractionTrace::now()-lastEnd)/1e6},{"total_observation_ms",double(InteractionTrace::now()-begin)/1e6},
                {"activeColorsChanged",colors-c0},{"settingsChanged",settings-s0},{"brushChanged",brush-b0},{"persistRequested",persist-p0}});
            InteractionTrace::operation=-1;
        };
        const bool diagnostic=qEnvironmentVariableIntValue("DRAWVERSE_INTERACTION_TRACE")!=0;
        report["trace_enabled"]=diagnostic;InteractionTrace::start(diagnostic);
        const bool onlyA2=qEnvironmentVariableIntValue("DRAWVERSE_INTERACTION_A2_ONLY")!=0;
        {
            scene("A0-idle",sampleCount,16,true,[&](int){return true;});
            client.activateEyedropper();client.setBrushColor(Qt::red);QTest::qWait(300);
            scene("A1-same-color",sampleCount,16,false,[&](int){const auto p=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(128,192)*canvas->zoom());click(window,p);return client.brushColor()==QColor(Qt::red);});
        }
        client.activateEyedropper();client.setBrushColor(Qt::blue);QTest::qWait(300);
        scene("A2-alternate-color",sampleCount,16,false,[&](int i){const bool red=i%2==0;const auto p=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(red?128:384,192)*canvas->zoom());click(window,p);return client.brushColor()==QColor(red?Qt::red:Qt::blue);});
        if(!onlyA2) {
            client.activateBrush();
            for(const QString field:{QString("radius"),QString("flow"),QString("opacity")}) {
                panel->setProperty("category",0);QTest::qWait(300);
                const QString name=field=="radius"?"settingsBrushSizeSlider":field=="opacity"?"brushToolbarOpacity":"brushToolbarFlow";
                auto *slider=visual(window->contentItem(),field=="radius"?name+"Track":name);QVERIFY2(slider,qPrintable(name));
                const auto read=[&]{return field=="radius"?library->radius():field=="flow"?library->flow():library->opacity();};
                const auto at=[&](double f){return slider->mapToScene(QPointF(5+f*(slider->width()-10),slider->height()/2));};
                // Real slider clicks. Expected state is compared to the QML control's edited value below.
                const auto check=[&]{auto *root=visual(window->contentItem(),name);return root && qAbs(read()-(field=="radius"?root->property("value").toDouble()/2:root->property("value").toDouble()))<.011;};
                click(window,at(.3));QTest::qWait(300);
                scene("A3-"+field+"-same",sampleCount,16,false,[&](int){const auto before=read();click(window,at(.3));return qAbs(read()-before)<.001 && check();});
                scene("A3-"+field+"-discrete",sampleCount,16,false,[&](int i){const auto before=read();click(window,at(i%2?.3:.7));return qAbs(read()-before)>.001 && check();});
                scene("A3-"+field+"-drag",sampleCount,16,true,[&](int i){if(i==0)mouse(window,QEvent::MouseButtonPress,at(.3),Qt::LeftButton,Qt::LeftButton);mouse(window,QEvent::MouseMove,at(.3+.4*i/qMax(1,sampleCount-1)),Qt::NoButton,Qt::LeftButton);return check();},[&]{mouse(window,QEvent::MouseButtonRelease,at(.7),Qt::LeftButton,Qt::NoButton);});
            }
            for(const QString target:{QString("Size"),QString("Flow"),QString("Opacity")}) {
                panel->setProperty("category",target=="Size"?1:0);QTest::qWait(300);
                auto *box=visual(window->contentItem(),target=="Size"?"settingsPressureSize":"brushToolbarPressure"+target);QVERIFY(box);
                const auto read=[&]{return target=="Size"?library->pressureSize():target=="Flow"?library->pressureFlow():library->pressureOpacity();};
                scene("A4-pressure"+target,sampleCount,16,false,[&](int){const bool before=read();click(window,box->mapToScene(QPointF(box->width()/2,box->height()/2)));return read()!=before;});
                scene("A4-pressure"+target+"-same-setter-control",sampleCount,16,false,[&](int){const bool before=read();if(target=="Size")library->setPressureSize(before);else if(target=="Flow")library->setPressureFlow(before);else library->setPressureOpacity(before);return read()==before;});
            }
            // Limited auxiliary synthetic hover observation. No tablet compatibility claims.
            library->setAngleControl("direction");library->setRoundness(.4);panel->setProperty("category",0);client.activateBrush();QTest::qWait(400);
            QJsonArray poses;
            for(const QString pattern:{QString("fixed-direction"),QString("slow"),QString("near-still")}) {
                scene("A6-"+pattern,sampleCount,16,true,[&](int i){const double step=pattern=="fixed-direction"?1:pattern=="slow"?.1:.0001;
                    const QPointF p=canvas->mapToScene(canvas->documentRect().center()+QPointF(i*step,i*step*.5));
                    QHoverEvent hover(QEvent::HoverMove,p,window->mapToGlobal(p.toPoint()),p);QCoreApplication::sendEvent(window,&hover);
                    mouse(window,QEvent::MouseMove,p,Qt::NoButton,Qt::NoButton);
                    poses.append(QJsonObject{{"pattern",pattern},{"i",i},{"angle",canvas->brushCursorAngle()},{"base_angle",client.baseCursorPose().angle}});return true;});
            }
            report["direction_observations"]=poses;
        }
        heartbeatTimer.stop();report["rows"]=rows;report["trace"]=InteractionTrace::finish();report["trace_dropped"]=double(InteractionTrace::dropped);report["error"]=client.lastError();
        // A final artifact is outside all measured segments.
        window->grabWindow().save(directory+"/window.png");
        QFile file(output);QVERIFY(file.open(QIODevice::WriteOnly));file.write(QJsonDocument(report).toJson());file.close();
        for(const auto &row:rows){const auto r=row.toObject();QCOMPARE(r.value("failures").toInt(),0);QCOMPARE(r.value("timeouts").toInt(),0);}
        QVERIFY(client.lastError().isEmpty());canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
};
int main(int argc,char **argv) {
    QGuiApplication app(argc,argv);initializeDrawVerseUiResources();QQuickStyle::setStyle("Basic");
    qmlRegisterType<MenuSurface>("DrawVerse",1,0,"MenuSurface");qmlRegisterType<SelectionOverlay>("DrawVerse",1,0,"SelectionOutline");
    qmlRegisterType<DocumentManager>("DrawVerse",1,0,"DocumentManager");qmlRegisterType<CanvasItem>("DrawVerse",1,0,"PaintCanvas");qmlRegisterType<ColorWheelItem>("DrawVerse",1,0,"ColorWheel");
    BrushInteractionBaseline test;return QTest::qExec(&test,argc,argv);
}
#include "brush_interaction_baseline.moc"
