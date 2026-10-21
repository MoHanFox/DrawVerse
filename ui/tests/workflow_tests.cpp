#include "engine/PaintCoreClient.h"
#include "documents/DocumentManager.h"
#include "canvas/SelectionOverlay.h"
#include "canvas/CanvasItem.h"
#include "color/ColorWheelItem.h"
#include <QQmlApplicationEngine>
#include <QDirIterator>
#include <QQuickItemGrabResult>
#include <QSGRendererInterface>
#include <QtTest>
#include <QQmlEngine>
#include <QQmlIncubationController>
#include <QQmlComponent>
#include <QQuickStyle>
#include <QTemporaryDir>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QQmlContext>
#include <QQuickItem>
#include <QQuickWindow>
#include <QBuffer>
#include <QMimeData>
#include <QDragEnterEvent>
#include <QDragMoveEvent>
#include <QDropEvent>
#include <QWheelEvent>
#include "platform/MenuSurface.h"
#include "workspace/WorkspaceManager.h"

static QQuickItem *findVisual(QQuickItem *item,const QString &name){if(item->objectName()==name)return item;for(auto *child:item->childItems())if(auto *found=findVisual(child,name))return found;return nullptr;}

class ResourcePreviewCacheProbe : public QObject {
    Q_OBJECT
    Q_PROPERTY(quint64 brushResourcePreviewRevision MEMBER revision NOTIFY brushResourcePreviewsChanged)
    Q_PROPERTY(QStringList brushResources MEMBER resources CONSTANT)
    Q_PROPERTY(bool brushFileBusy READ busy CONSTANT)
public:
    QStringList resources;
    bool busy() const{return false;}
    Q_INVOKABLE void refreshBrushResources(){}
    Q_INVOKABLE void selectBrushResource(const QString &,int){}
    quint64 revision=0;int requests=0;QHash<QString,QString> cache;
    Q_INVOKABLE QString brushResourcePreview(const QString &id)const{return cache.value(id);}
    Q_INVOKABLE bool requestBrushResourcePreview(const QString &){++requests;return false;}
    void changed(){++revision;emit brushResourcePreviewsChanged();}
signals:
    void brushResourcePreviewsChanged();
    void brushLibraryReadyChanged();
};

class WorkflowTests : public QObject {
    Q_OBJECT
private slots:
    void welcomeProgressUsesExplicitReadiness() {
        QQmlEngine engine;QQmlComponent component(&engine,QUrl("qrc:/qml/app/WelcomeScreen.qml"));
        QScopedPointer<QObject> welcome(component.create());QVERIFY2(welcome,qPrintable(component.errorString()));
        welcome->setProperty("stage",QStringLiteral("正在准备笔刷资源…"));
        auto *status=welcome->findChild<QQuickItem*>("welcomeLoadingStatus");auto *actions=welcome->findChild<QQuickItem*>("welcomeActions");
        QVERIFY(status && actions);QVERIFY(status->isVisible());QVERIFY(!actions->isVisible());
        QCOMPARE(status->property("text").toString(),QStringLiteral("正在准备笔刷资源…"));
        welcome->setProperty("error",QStringLiteral("资源加载失败"));QCOMPARE(status->property("text").toString(),QStringLiteral("资源加载失败"));
        welcome->setProperty("error",QString());welcome->setProperty("ready",true);
        QVERIFY(!status->isVisible());QVERIFY(actions->isVisible());
    }
    void welcomeWorkspaceDocumentLifecycle() {
        QTemporaryDir temp;PaintCoreClient service(nullptr,temp.filePath("settings.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));
        workspace.detachGroup(workspace.groupForPanel("color"));workspace.floatToolStrip(100,100);
        QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors)warnings.append(error.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&service);engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);
        auto *manager=window->findChild<DocumentManager*>("documentManager");QVERIFY(manager);QVERIFY(manager->documents().isEmpty());QVERIFY(manager->activeId().isEmpty());
        auto *welcome=window->findChild<QQuickItem*>("welcomeScreen");auto *editor=window->findChild<QQuickItem*>("editorWorkspace");
        auto *options=window->findChild<QQuickItem*>("brushOptionsBar");QVERIFY(welcome && editor && options);
        QVERIFY(welcome->isVisible());QVERIFY(!editor->isVisible());QVERIFY(!options->isVisible());QCOMPARE(options->height(),qreal(0));
        QTRY_VERIFY_WITH_TIMEOUT(window->property("startupReady").toBool(),30000);QVERIFY(service.brushCatalogPrepared());
        if(qEnvironmentVariable("QSG_RHI_BACKEND")=="d3d11" && qEnvironmentVariable("QT_QUICK_BACKEND")!="software")QCOMPARE(window->rendererInterface()->graphicsApi(),QSGRendererInterface::Direct3D11);
        auto floating=[&]{QList<QQuickWindow*> result;for(auto *candidate:QGuiApplication::allWindows())if(candidate->objectName().startsWith("floatingDock:") || candidate->objectName()=="floatingToolStrip")result.append(qobject_cast<QQuickWindow*>(candidate));return result;};
        QTRY_COMPARE(floating().size(),2);for(auto *panel:floating())QVERIFY(!panel->isVisible());
        const auto layout=workspace.floatingWindows();
        for(const auto &name:{"freeTransformAction","liquifyAction","placeImageAction","selectionAllAction","canvasFlipHorizontalAction"}) {auto *action=window->findChild<QObject*>(name);QVERIFY(action);QVERIFY(!action->property("enabled").toBool());}
        const auto capture=qEnvironmentVariable("DRAWVERSE_WELCOME_CAPTURE");
        if(!capture.isEmpty()){QTest::qWait(100);QVERIFY(window->grabWindow().save(capture));}
        auto *newButton=findVisual(window->contentItem(),"welcomeNewCanvas");QVERIFY(newButton);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,newButton->mapToScene(QPointF(newButton->width()/2,newButton->height()/2)).toPoint());
        auto *dialog=window->findChild<QObject*>("newDocumentDialog");QVERIFY(dialog);QTRY_VERIFY(dialog->property("opened").toBool());QVERIFY(QMetaObject::invokeMethod(dialog,"close"));
        auto *openButton=findVisual(window->contentItem(),"welcomeOpen");QVERIFY(openButton);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,openButton->mapToScene(QPointF(openButton->width()/2,openButton->height()/2)).toPoint());
        auto *openDialog=window->findChild<QObject*>("openDocumentDialog");QVERIFY(openDialog);QTRY_VERIFY(openDialog->property("visible").toBool());QVERIFY(QMetaObject::invokeMethod(openDialog,"close"));
        const auto first=manager->newDocument(96,64,true);QVERIFY(!first.isEmpty());auto *firstClient=manager->client(first);QVERIFY(firstClient && firstClient!=&service);
        QTRY_VERIFY(firstClient->ready());QTRY_COMPARE(QSize(firstClient->documentWidth(),firstClient->documentHeight()),QSize(96,64));
        QTRY_VERIFY(editor->isVisible());QVERIFY(!welcome->isVisible());QVERIFY(options->isVisible());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);QCOMPARE(canvas->client(),firstClient);
        for(auto *panel:floating())QTRY_VERIFY(panel->isVisible());
        const auto second=manager->newDocument(64,48);QVERIFY(!second.isEmpty());QTRY_VERIFY(manager->client(second)->ready());QTRY_COMPARE(manager->client(second)->documentWidth(),64);
        QVERIFY(manager->closeDocument(first,true));QVERIFY(editor->isVisible());
        workspace.setColumnCollapsed(workspace.groupForPanel("layers"),true);
        QQuickItem *rail=nullptr;QTRY_VERIFY((rail=findVisual(window->contentItem(),"railGrip:layers")));QTRY_VERIFY(rail->isVisible());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,rail->mapToScene({14,14}).toPoint());
        QPointer<QQuickWindow> flyout;QTRY_VERIFY([&]{for(auto *candidate:QGuiApplication::allWindows())if(candidate->objectName().startsWith("panelFlyout:") && candidate->isVisible())flyout=qobject_cast<QQuickWindow*>(candidate);return !flyout.isNull();}());
        QVERIFY(manager->closeDocument(second,true));QTRY_VERIFY(!flyout || !flyout->isVisible());QTRY_VERIFY(welcome->isVisible());QVERIFY(!editor->isVisible());QVERIFY(service.ready());
        for(auto *panel:floating())QVERIFY(!panel->isVisible());QCOMPARE(workspace.floatingWindows(),layout);
        // A later open must use the new client, even after the first pane was parked.
        QImage image(32,24,QImage::Format_ARGB32_Premultiplied);image.fill(Qt::red);const auto path=temp.filePath("sample.png");QVERIFY(image.save(path));
        const auto opened=manager->openDocument(QUrl::fromLocalFile(path));QVERIFY(!opened.isEmpty());auto *openedClient=manager->client(opened);QVERIFY(openedClient);
        QTRY_VERIFY_WITH_TIMEOUT(openedClient->ready() && !openedClient->fileBusy() && QSize(openedClient->documentWidth(),openedClient->documentHeight())==image.size(),30000);
        QTRY_VERIFY(editor->isVisible());QVERIFY(manager->closeDocument(opened,true));QTRY_VERIFY(welcome->isVisible());
        QSignalSpy failed(manager,&DocumentManager::fileFailed);const auto missing=manager->openDocument(QUrl::fromLocalFile(temp.filePath("missing.png")));QVERIFY(!missing.isEmpty());
        QTRY_COMPARE(failed.size(),1);QTRY_VERIFY(manager->documents().isEmpty());QTRY_VERIFY(welcome->isVisible());
        QVERIFY2(warnings.isEmpty(),qPrintable(warnings.join('\n')));
        QSignalSpy stopped(manager,&DocumentManager::stopped);manager->shutdown();QTRY_COMPARE(stopped.size(),1);
        QTRY_VERIFY(!engine.incubationController() || engine.incubationController()->incubatingObjectCount()==0);
    }
    void collapsedFlyoutMergesMimeDropsAndKeepsTabOrder() {
        QTemporaryDir temp;const auto layout=temp.filePath("layout.ini");PaintCoreClient client(nullptr,temp.filePath("settings.ini"));WorkspaceManager workspace(layout);
        const auto incoming=workspace.addCustomPanel("Incoming","palette");QVERIFY(!incoming.isEmpty());workspace.detachPanel(workspace.groupForPanel(incoming),incoming);
        const auto source=workspace.groupForPanel(incoming),target=workspace.groupForPanel("color");workspace.setColumnCollapsed(target,true);
        QQmlApplicationEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        auto *rail=findVisual(main->contentItem(),"railGrip:color");QVERIFY(rail);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,rail->mapToScene({14,14}).toPoint());
        QPointer<QQuickWindow> flyout;QTRY_VERIFY([&]{for(auto *w:QGuiApplication::allWindows())if(w->isVisible() && w->objectName().startsWith("panelFlyout:") && w->objectName().endsWith(":color"))flyout=qobject_cast<QQuickWindow*>(w);return !flyout.isNull();}());
        auto *group=findVisual(flyout->contentItem(),"dockGroup:"+target);QVERIFY(group);const auto center=group->mapToScene({group->width()/2,group->height()/2}).toPoint();
        QMimeData mime;mime.setData("application/x-drawverse-panel",QJsonDocument(QJsonObject{{"group",source},{"panel",incoming},{"whole",false}}).toJson(QJsonDocument::Compact));
        QDragEnterEvent enter(center,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(flyout,&enter);QVERIFY(enter.isAccepted());
        QDragMoveEvent move(center,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(flyout,&move);QVERIFY(move.isAccepted());
        QDropEvent drop(center,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(flyout,&drop);QVERIFY(drop.isAccepted());
        QCOMPARE(workspace.groupForPanel(incoming),target);QCOMPARE(workspace.groupDefinition(target).value("panels").toStringList(),(QStringList{"color",incoming}));QVERIFY(workspace.groupDefinition(target).value("icons").toBool());
        QTRY_VERIFY(flyout && flyout->isVisible());
        QTRY_COMPARE(flyout->property("selectedPanel").toString(),incoming);
        auto *colorIcon=findVisual(main->contentItem(),"railPanel:color"),*newIcon=findVisual(main->contentItem(),"railPanel:"+incoming);QVERIFY(colorIcon && newIcon);QTRY_VERIFY(colorIcon->y()<newIcon->y());
        rail=findVisual(main->contentItem(),"railGrip:color");QVERIFY(rail);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,rail->mapToScene({14,14}).toPoint());QTRY_VERIFY(flyout && flyout->isVisible());
        auto *incomingIcon=findVisual(main->contentItem(),"railGrip:"+incoming);QVERIFY(incomingIcon);
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,incomingIcon->mapToScene({14,14}).toPoint());QTRY_VERIFY(flyout && flyout->isVisible());
        int count=0;for(auto *w:QGuiApplication::allWindows())if(w->isVisible() && w->objectName().startsWith("panelFlyout:"))++count;QCOMPARE(count,1);
        QTRY_COMPARE(workspace.groupDefinition(target).value("active").toString(),incoming);
        workspace.swapPanelTabs(target,"color",incoming);const QStringList expected{incoming,"color"};QCOMPARE(workspace.categoryDefinition(target).value("panels").toStringList(),expected);
        auto *firstIcon=findVisual(main->contentItem(),"railPanel:"+incoming),*secondIcon=findVisual(main->contentItem(),"railPanel:color");QVERIFY(firstIcon && secondIcon);QTRY_VERIFY(firstIcon->y()<secondIcon->y());
        auto *firstTab=findVisual(flyout->contentItem(),"panelTab:"+incoming),*secondTab=findVisual(flyout->contentItem(),"panelTab:color");QVERIFY(firstTab && secondTab);QTRY_VERIFY(firstTab->x()<secondTab->x());
        workspace.saveLayout();{WorkspaceManager restored(layout);QCOMPARE(restored.groupForPanel(incoming),target);QCOMPARE(restored.groupDefinition(target).value("panels").toStringList(),expected);QVERIFY(restored.groupDefinition(target).value("icons").toBool());}
        QVERIFY2(warnings.isEmpty(),qPrintable(warnings.join('\n')));auto *manager=main->findChild<DocumentManager*>("documentManager");QVERIFY(manager);QSignalSpy stopped(manager,&DocumentManager::stopped);manager->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void collapsedFlyoutMergesNativeWindowDrag_data() {
        QTest::addColumn<int>("sourceKind");QTest::newRow("single-panel")<<0;QTest::newRow("one-group")<<1;QTest::newRow("whole-column")<<2;
    }
    void collapsedFlyoutMergesNativeWindowDrag() {
        QFETCH(int,sourceKind);const bool multipleGroups=sourceKind==2,singlePanel=sourceKind==0;QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("settings.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));
        const auto target=workspace.groupForPanel("color"),originalBrush=workspace.groupForPanel("brush");
        if(singlePanel)workspace.detachPanel(originalBrush,"brush");
        else if(multipleGroups){workspace.detachPanel(originalBrush,"brush-settings");workspace.detachPanel(workspace.groupForPanel("brush"),"brush");}
        else workspace.detachGroup(originalBrush);
        const auto source=workspace.groupForPanel("brush");
        if(multipleGroups){const auto secondary=workspace.groupForPanel("brush-settings");QVERIFY(workspace.dockPayload(QString::fromUtf8(QJsonDocument(QJsonObject{{"group",secondary},{"whole",true}}).toJson()),"floating",source,"after"));}
        workspace.setColumnCollapsed(target,true);
        QQmlApplicationEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        auto *rail=findVisual(main->contentItem(),"railGrip:color");QVERIFY(rail);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,rail->mapToScene({14,14}).toPoint());
        QPointer<QQuickWindow> flyout,sourceWindow;QTRY_VERIFY([&]{for(auto *w:QGuiApplication::allWindows()){if(w->isVisible() && w->objectName().startsWith("panelFlyout:") && w->objectName().endsWith(":color"))flyout=qobject_cast<QQuickWindow*>(w);if(w->isVisible() && w->objectName().startsWith("floatingDock:") && findVisual(qobject_cast<QQuickWindow*>(w)->contentItem(),"dockGroup:"+source))sourceWindow=qobject_cast<QQuickWindow*>(w);}return flyout && sourceWindow;}());
        auto *group=findVisual(flyout->contentItem(),"dockGroup:"+target);QVERIFY(group);const auto global=group->mapToGlobal({10,18}).toPoint();
        const auto sendMove=[&]{QMouseEvent event(QEvent::MouseMove,sourceWindow->mapFromGlobal(global),global,Qt::NoButton,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(sourceWindow,&event);};
        const auto begin=[&]{QTest::mousePress(sourceWindow,Qt::LeftButton,Qt::NoModifier,{24,4});workspace.beginDrag(source,{},true);};
        begin();QTRY_VERIFY(workspace.dragging());sendMove();QTRY_COMPARE(workspace.dragTarget(),target);QCOMPARE(workspace.dragPlacement(),QString("merge"));QCOMPARE(workspace.dragBeforePanel(),QString("color"));
        QTest::keyPress(sourceWindow,Qt::Key_Control);QTRY_VERIFY(workspace.dragTarget().isEmpty());QTest::keyRelease(sourceWindow,Qt::Key_Control);QTRY_COMPARE(workspace.dragTarget(),target);
        QTest::keyClick(sourceWindow,Qt::Key_Escape);QTRY_VERIFY(!workspace.dragging());QCOMPARE(workspace.groupDefinition(target).value("panels").toStringList(),QStringList{"color"});QCOMPARE(workspace.groupForPanel("brush"),source);
        begin();QTRY_VERIFY(workspace.dragging());sendMove();QTRY_COMPARE(workspace.dragTarget(),target);
        QMouseEvent release(QEvent::MouseButtonRelease,sourceWindow->mapFromGlobal(global),global,Qt::LeftButton,Qt::NoButton,Qt::NoModifier);QCoreApplication::sendEvent(sourceWindow,&release);
        QTRY_VERIFY(!workspace.dragging());QCOMPARE(workspace.groupForPanel("brush"),target);if(!singlePanel)QCOMPARE(workspace.groupForPanel("brush-settings"),target);
        const QStringList expected=singlePanel?QStringList{"brush","color"}:QStringList{"brush","brush-settings","color"};
        QCOMPARE(workspace.groupDefinition(target).value("panels").toStringList(),expected);QVERIFY(workspace.groupDefinition(target).value("icons").toBool());QTRY_VERIFY(flyout && flyout->isVisible());
        const auto capture=qEnvironmentVariable("DRAWVERSE_FLYOUT_CAPTURE");if(!capture.isEmpty()){QTest::qWait(100);QVERIFY(flyout->grabWindow().save(capture+"-"+QString::fromLatin1(QTest::currentDataTag())+".png"));}
        QVERIFY2(warnings.isEmpty(),qPrintable(warnings.join('\n')));auto *manager=main->findChild<DocumentManager*>("documentManager");QVERIFY(manager);QSignalSpy stopped(manager,&DocumentManager::stopped);manager->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void nativeIdentityPressureDoesNotRebuildBrushRows() {
        BrushLibrary library;const auto original=library.snapshot();
        const QJsonObject control{{"curve",QJsonArray{QJsonArray{0.,0.},QJsonArray{1.,1.}}},{"minimum",0.}};
        QSignalSpy settings(&library,&BrushLibrary::settingsChanged),presets(&library,&BrushLibrary::presetsChanged),previews(&library,&BrushLibrary::selectedPreviewChanged);
        for(int version:{5,6,44}) {
            auto native=QJsonDocument::fromJson(library.catalogSnapshot()).object();auto rows=native.value("presets").toArray();auto row=rows[0].toObject();
            auto pressure=QJsonObject{{"size",control},{"flow",control}};if(version>=6){auto opacity=control;opacity["enabled"]=false;pressure["opacity"]=opacity;}
            row["pressureDynamics"]=pressure;rows[0]=row;native["presets"]=rows;native["version"]=version;
            QVERIFY(library.applyNativeCatalog(QJsonDocument(native).toJson()));QCOMPARE(library.snapshot(),original);QCOMPARE(settings.size(),0);QCOMPARE(presets.size(),0);QCOMPARE(previews.size(),0);
            native["version"]=2;QVERIFY(!library.applyNativeCatalog(QJsonDocument(native).toJson()));QCOMPARE(library.snapshot(),original);
        }
        auto native=QJsonDocument::fromJson(library.catalogSnapshot()).object();auto rows=native.value("presets").toArray();auto row=rows[0].toObject();auto changed=control;changed["minimum"]=.25;
        row["pressureDynamics"]=QJsonObject{{"size",changed},{"flow",control}};rows[0]=row;native["presets"]=rows;native["version"]=5;
        QVERIFY(library.applyNativeCatalog(QJsonDocument(native).toJson()));QCOMPARE(presets.size(),1);QCOMPARE(library.pressureDynamics().value("size").toMap().value("minimum").toDouble(),.25);
        const auto accepted=library.snapshot();auto invalid=row.value("pressureDynamics").toObject();invalid["unexpected"]=true;row["pressureDynamics"]=invalid;rows[0]=row;native["presets"]=rows;
        QVERIFY(!library.applyNativeCatalog(QJsonDocument(native).toJson()));QCOMPARE(library.snapshot(),accepted);
    }
    void liveBrushSelectionWithLargePreferences() {
        QTemporaryDir temp;
        const auto source=qEnvironmentVariable("DRAWVERSE_BRUSH_REPRO_SETTINGS");
        const auto path=temp.filePath("brush-switch.ini");
        if(source.isEmpty())QSKIP("Manual responsiveness probe: set DRAWVERSE_BRUSH_REPRO_SETTINGS to a copied INI");
        QVERIFY(QFile::copy(source,path));
        const QDir resources(QFileInfo(source).dir().filePath("brush-resources"));QDirIterator files(resources.absolutePath(),QDir::Files,QDirIterator::Subdirectories);
        while(files.hasNext()){const auto file=files.next();const auto copy=temp.filePath("brush-resources/"+resources.relativeFilePath(file));QVERIFY(QDir().mkpath(QFileInfo(copy).absolutePath()));QVERIFY(QFile::copy(file,copy));}
        QSettings settings(path,QSettings::IniFormat);const auto saved=settings.value("brushes/state").toByteArray();qInfo()<<"copied preset count"<<QJsonDocument::fromJson(saved).object().value("presets").toArray().size()<<"bytes"<<saved.size();
        QList<qint64> coldRestoreUs;
        for(int i=0;i<5;++i){BrushLibrary cold;QElapsedTimer timer;timer.start();QVERIFY(cold.restore(saved));coldRestoreUs.append(timer.nsecsElapsed()/1000);}
        std::sort(coldRestoreUs.begin(),coldRestoreUs.end());qInfo()<<"copied preferences full restore median us"<<coldRestoreUs[2];
        PaintCoreClient client(nullptr,path);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.setInitialProperties({{"createInitialDocument",true}});engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);window->resize(1600,1000);QVERIFY(QTest::qWaitForWindowExposed(window));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        auto *library=client.brushLibrary();const auto rows=library->presetRows();QVERIFY2(rows.size()>4,qPrintable(client.lastError()));
        QElapsedTimer clock;clock.start();qint64 longest=0,previous=0,maxSelect=0;QTimer heartbeat;heartbeat.setInterval(2);connect(&heartbeat,&QTimer::timeout,this,[&]{const auto now=clock.elapsed();longest=std::max(longest,now-previous);previous=now;});heartbeat.start();
        QSignalSpy catalogChanges(library,&BrushLibrary::presetsChanged);
        for(int pass=0;pass<2;++pass){
            maxSelect=longest=0;previous=clock.elapsed();
            for(int i=0;i<16;++i){const auto start=clock.nsecsElapsed();QVERIFY(library->select(rows[(4+i*17)%rows.size()].toMap().value("id").toString()));maxSelect=std::max(maxSelect,(clock.nsecsElapsed()-start)/1000);QTest::qWait(180);}
            qInfo()<<"live settings presets"<<rows.size()<<"pass"<<pass<<"max select us"<<maxSelect<<"longest GUI gap ms"<<longest;
            QCOMPARE(catalogChanges.size(),0);
        }
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);
        const auto originalDocument=documents->activeId();const auto secondDocument=documents->newDocument(32,32,true);QVERIFY(!secondDocument.isEmpty());
        auto *receiver=documents->client(secondDocument);QVERIFY(receiver);QTRY_VERIFY(receiver->ready() && receiver->brushLibraryReady() && !receiver->layerEditBusy());
        QVERIFY(documents->activate(originalDocument));QTest::qWait(200);
        QSignalSpy receivedCatalogChanges(receiver->brushLibrary(),&BrushLibrary::presetsChanged);
        longest=maxSelect=0;previous=clock.elapsed();
        for(int i=0;i<16;++i){const auto start=clock.nsecsElapsed();QVERIFY(library->select(rows[(5+i*17)%rows.size()].toMap().value("id").toString()));maxSelect=std::max(maxSelect,(clock.nsecsElapsed()-start)/1000);QTest::qWait(180);}
        qInfo()<<"live two-document selection max us"<<maxSelect<<"longest GUI gap ms"<<longest<<"receiver catalog rebuilds"<<receivedCatalogChanges.size();
        QCOMPARE(receivedCatalogChanges.size(),0);QCOMPARE(receiver->brushLibrary()->selectedId(),library->selectedId());QVERIFY(receiver->lastError().isEmpty());
        for(int mode=1;mode<=2;++mode) {
            library->setToolIsolation(mode);library->flushPersistence();QTest::qWait(300);
            receiver->activateEraser();receiver->brushLibrary()->flushPersistence();QTest::qWait(300);
            QVERIFY(receiver->restoreBrushPreferences(library->snapshot(),library));receiver->brushLibrary()->snapshot();
            const auto receiverSelected=receiver->brushLibrary()->selectedId();
            receivedCatalogChanges.clear();catalogChanges.clear();longest=maxSelect=0;previous=clock.elapsed();
            for(int i=0;i<16;++i){const auto start=clock.nsecsElapsed();QVERIFY(library->select(rows[(6+i*17)%rows.size()].toMap().value("id").toString()));maxSelect=std::max(maxSelect,(clock.nsecsElapsed()-start)/1000);QTest::qWait(180);}
            qInfo()<<"live cross-tool selection isolation"<<mode<<"max us"<<maxSelect<<"longest GUI gap ms"<<longest<<"receiver catalog rebuilds"<<receivedCatalogChanges.size();
            QCOMPARE(receivedCatalogChanges.size(),0);QCOMPARE(catalogChanges.size(),0);QVERIFY(receiver->eraser());QCOMPARE(receiver->brushLibrary()->toolContext(),1);
            QCOMPARE(receiver->brushLibrary()->selectedId(),mode==2?receiverSelected:library->selectedId());QVERIFY(receiver->lastError().isEmpty());
        }
        heartbeat.stop();QVERIFY(client.lastError().isEmpty());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }

    void fullPreferenceRestoreReusesRowsWithoutSkippingRoleValidation() {
        BrushLibrary source;QVERIFY(source.setPressureControl("size",{QVariantList{0.,0.},QVariantList{1.,1.}},.2));source.setToolIsolation(2);
        source.setToolContext(1);source.setRadius(19);source.setWriteBlend(8);source.setToolContext(2);source.setRadius(23);source.setToolContext(0);
        const auto root=QJsonDocument::fromJson(source.snapshot()).object();BrushLibrary restored;QVERIFY(restored.restore(source.snapshot()));QCOMPARE(restored.snapshot(),source.snapshot());
        restored.setToolContext(1);QCOMPARE(restored.radius(),19.);QCOMPARE(restored.writeBlend(),8);restored.setToolContext(2);QCOMPARE(restored.radius(),23.);
        const auto before=restored.snapshot();QSignalSpy settings(&restored,&BrushLibrary::settingsChanged),presets(&restored,&BrushLibrary::presetsChanged);
        for(int failure=0;failure<6;++failure) {
            auto invalid=root;auto isolation=invalid.value("toolIsolation").toObject();auto profiles=isolation.value("profiles").toArray();auto profile=profiles[2].toObject();auto rows=profile.value("presets").toArray();
            if(failure==0)profile["version"]=2; // Same rows require pressure schema >=5.
            if(failure==1)rows.append(rows.first()); // A cached ID must still reject duplicates.
            if(failure==2){auto row=rows.last().toObject();row["radius"]=-1;rows[rows.size()-1]=row;}
            if(failure==3)profile["selected"]="missing";
            if(failure==4)profile["dualEditors"]=QJsonObject{{"round-pressure",QJsonObject{{"existing",3},{"sourceName","invalid"}}}};
            if(failure==5){auto reset=invalid.value("resetState").toObject();auto resetRows=reset.value("presets").toArray();auto row=resetRows.last().toObject();row["radius"]=-1;resetRows[resetRows.size()-1]=row;reset["presets"]=resetRows;invalid["resetState"]=reset;}
            profile["presets"]=rows;profiles[2]=profile;isolation["profiles"]=profiles;invalid["toolIsolation"]=isolation;
            QVERIFY(!restored.restore(QJsonDocument(invalid).toJson()));QCOMPARE(restored.snapshot(),before);
        }
        QCOMPARE(settings.size(),0);QCOMPARE(presets.size(),0);
        // Per-profile editor metadata is validated and applied after row reuse.
        auto edited=root;auto isolation=edited.value("toolIsolation").toObject();auto profiles=isolation.value("profiles").toArray();auto profile=profiles[2].toObject();
        profile["dualEditors"]=QJsonObject{{"round-pressure",QJsonObject{{"existing",true},{"sourceName","Inherited tip"}}}};profiles[2]=profile;isolation["profiles"]=profiles;edited["toolIsolation"]=isolation;
        QVERIFY(restored.restore(QJsonDocument(edited).toJson()));restored.setToolContext(2);QVERIFY(restored.dualExisting());QCOMPARE(restored.radius(),23.);
    }

    void sharedToolContextUpdatesCachedPreferencesWithoutCatalogWork() {
        BrushLibrary library;const auto catalog=library.catalogSnapshot();library.snapshot();
        QSignalSpy models(&library,&BrushLibrary::presetsChanged),persist(&library,&BrushLibrary::persistRequested),settings(&library,&BrushLibrary::settingsChanged);
        for(const auto tool:{1,2,0,2,1,0}) {
            library.setToolContext(tool);const auto bytes=library.snapshot();
            QCOMPARE(QJsonDocument::fromJson(bytes).object().value("toolIsolation").toObject().value("context").toInt(),tool);
            QCOMPARE(library.catalogSnapshot(),catalog);BrushLibrary restored;QVERIFY(restored.restore(bytes));QCOMPARE(restored.toolContext(),tool);
        }
        QCOMPARE(models.size(),0);QCOMPARE(settings.size(),0);QCOMPARE(persist.size(),0);QVERIFY(!library.persistencePending());
    }

    void selectingBrushReusesRowsAndPreservesAllPreferenceScopes() {
        for(int isolation=0;isolation<=2;++isolation) {
            BrushLibrary library;auto state=QJsonDocument::fromJson(library.catalogSnapshot()).object();auto rows=state.value("presets").toArray();const auto base=rows.first().toObject();
            for(int i=4;i<512;++i){auto row=base;row["id"]=QString("cached-%1").arg(i);row["name"]=QString("Cached %1").arg(i);rows.append(row);}state["presets"]=rows;QVERIFY(library.restore(QJsonDocument(state).toJson()));
            library.setToolIsolation(isolation);library.setToolContext(1);library.setRadius(19);library.setToolContext(0);
            const auto original=QJsonDocument::fromJson(library.snapshot()).object();const auto execution=QJsonDocument::fromJson(library.catalogSnapshot()).object();
            QSignalSpy presets(&library,&BrushLibrary::presetChanged),catalogs(&library,&BrushLibrary::presetsChanged);
            QElapsedTimer timer;timer.start();
            for(int i=0;i<20;++i){QVERIFY(library.selectBrushItems({QVariantMap{{"kind","preset"},{"id",QString("cached-%1").arg(i+4)}}}));library.snapshot();library.catalogSnapshot();}
            qInfo()<<"512 preset cached selection+snapshots isolation"<<isolation<<"20 switches us"<<timer.nsecsElapsed()/1000;
            QCOMPARE(presets.size(),0);QCOMPARE(catalogs.size(),0);
            auto expected=original;expected["selected"]=library.selectedId();auto reset=expected.value("resetState").toObject();reset["selected"]=library.selectedId();expected["resetState"]=reset;
            if(isolation){auto scopes=expected.value("toolIsolation").toObject();auto profiles=scopes.value("profiles").toArray();auto profile=profiles[0].toObject();profile["selected"]=library.selectedId();profiles[0]=profile;scopes["profiles"]=profiles;expected["toolIsolation"]=scopes;}
            QCOMPARE(library.snapshot(),QJsonDocument(expected).toJson(QJsonDocument::Compact));auto expectedExecution=execution;expectedExecution["selected"]=library.selectedId();QCOMPARE(QJsonDocument::fromJson(library.catalogSnapshot()).object(),expectedExecution);
            BrushLibrary restored;QVERIFY(restored.restore(library.snapshot()));QCOMPARE(restored.selectedId(),library.selectedId());QCOMPARE(restored.toolIsolation(),isolation);
            library.setRadius(31);const auto edited=QJsonDocument::fromJson(library.catalogSnapshot()).object().value("presets").toArray();bool found=false;for(const auto &value:edited){const auto row=value.toObject();if(row.value("id").toString()==library.selectedId()){QCOMPARE(row.value("radius").toDouble(),31.);found=true;}}QVERIFY(found);
            const auto changed=library.snapshot();QVERIFY(changed!=QJsonDocument(expected).toJson(QJsonDocument::Compact));
            QCOMPARE(changed,QJsonDocument(QJsonDocument::fromJson(changed).object()).toJson(QJsonDocument::Compact));
            BrushLibrary editedRestore;QVERIFY(editedRestore.restore(changed));QCOMPARE(editedRestore.radius(),31.);
            library.setToolContext(1);if(isolation==1)QVERIFY(library.select("round-pressure"));if(isolation)QCOMPARE(library.radius(),19.);
        }
    }

    void sharedSelectionRestoreKeepsValidatedRowsAndToolProfiles() {
        for(int mode=0;mode<=2;++mode) {
            BrushLibrary source;auto root=QJsonDocument::fromJson(source.catalogSnapshot()).object();auto rows=root.value("presets").toArray();
            for(int i=4;i<512;++i){auto row=rows.first().toObject();row["id"]=QString("shared-%1").arg(i);row["name"]=QString("Shared %1").arg(i);rows.append(row);}
            root["presets"]=rows;QVERIFY(source.restore(QJsonDocument(root).toJson()));source.setToolIsolation(mode);
            source.setToolContext(1);source.setRadius(19);source.setToolContext(0);
            BrushLibrary receiver;QVERIFY(receiver.restore(source.snapshot()));receiver.snapshot();
            QSignalSpy catalog(&receiver,&BrushLibrary::presetsChanged),persist(&receiver,&BrushLibrary::persistRequested);
            QElapsedTimer timer;timer.start();
            for(int i=4;i<10;++i){QVERIFY(source.select(QString("shared-%1").arg(i)));QVERIFY(receiver.restore(source.snapshot()));QCOMPARE(receiver.selectedId(),source.selectedId());}
            qInfo()<<"512 shared selection restores mode"<<mode<<"six updates us"<<timer.nsecsElapsed()/1000;
            QCOMPARE(catalog.size(),0);QCOMPARE(persist.size(),0);QVERIFY(!receiver.persistencePending());QCOMPARE(receiver.snapshot(),source.snapshot());
            // Multi-selection preservation follows the full restore contract.
            QVERIFY(receiver.selectBrushItems({QVariantMap{{"kind","preset"},{"id","round-pressure"}},QVariantMap{{"kind","preset"},{"id","round-fine"}}}));receiver.flushPersistence();
            const auto selection=receiver.selectedBrushItems();QVERIFY(receiver.restore(source.snapshot(),true));QCOMPARE(receiver.selectedBrushItems(),selection);QCOMPARE(catalog.size(),0);
            const auto before=receiver.snapshot();auto invalid=QJsonDocument::fromJson(before).object();invalid["selected"]="missing";
            QVERIFY(!receiver.restore(QJsonDocument(invalid).toJson()));QCOMPARE(receiver.snapshot(),before);
            invalid=QJsonDocument::fromJson(before).object();auto badRows=invalid.value("presets").toArray();auto bad=badRows.last().toObject();bad["radius"]=-1;badRows[badRows.size()-1]=bad;invalid["presets"]=badRows;
            QVERIFY(!receiver.restore(QJsonDocument(invalid).toJson()));QCOMPARE(receiver.snapshot(),before);
            // Real parameter changes still restore, rather than being mistaken for selection.
            source.setRadius(31);QVERIFY(receiver.restore(source.snapshot()));QCOMPARE(receiver.radius(),31.);QCOMPARE(catalog.size(),1);
            receiver.setToolContext(1);if(mode==1)QVERIFY(receiver.select("round-pressure"));if(mode)QCOMPARE(receiver.radius(),19.);
        }
    }

    void sharedSelectionAcrossToolContextsKeepsProfilesAndPreviews() {
        for(int mode=0;mode<=2;++mode)for(int tool=0;tool<3;++tool) {
            BrushLibrary source;auto initial=QJsonDocument::fromJson(source.catalogSnapshot()).object();auto rows=initial.value("presets").toArray();
            for(int i=4;i<512;++i){auto row=rows.first().toObject();row["id"]=QString("context-%1").arg(i);rows.append(row);}
            initial["presets"]=rows;QVERIFY(source.restore(QJsonDocument(initial).toJson()));source.setToolIsolation(mode);
            source.setToolContext(1);source.setRadius(19);source.setToolContext(2);source.setRadius(23);source.setToolContext(0);source.flushPersistence();
            BrushLibrary receiver,reference;QVERIFY(receiver.restore(source.snapshot()));receiver.setToolContext(tool,false);receiver.snapshot();
            quint64 previewToken=0;connect(&receiver,&BrushLibrary::previewRequested,this,[&](const QString &,quint64 token,qreal,qreal,qreal,QVariantMap){previewToken=token;});
            receiver.requestPreview("round-pressure");QVERIFY(previewToken);receiver.acceptPreview("round-pressure",previewToken,"context-preview");
            QSignalSpy catalog(&receiver,&BrushLibrary::presetsChanged),persist(&receiver,&BrushLibrary::persistRequested);
            QElapsedTimer timer;timer.start();qint64 reuseNs=0,referenceNs=0;
            for(int i=4;i<10;++i) {
                source.setToolContext(i%3,false);QVERIFY(source.select(QString("context-%1").arg(i)));source.snapshot();
                auto start=timer.nsecsElapsed();QVERIFY(receiver.restoreSharedSelection(source));reuseNs+=timer.nsecsElapsed()-start;QCOMPARE(receiver.toolContext(),tool);
                start=timer.nsecsElapsed();QVERIFY(reference.restore(source.snapshot()));reference.setToolContext(tool,false);referenceNs+=timer.nsecsElapsed()-start;
                QCOMPARE(receiver.snapshot(),reference.snapshot());
            }
            qInfo()<<"512 cross-tool shared selection mode"<<mode<<"receiver tool"<<tool<<"six restores us reuse/full"<<reuseNs/1000<<referenceNs/1000;
            QCOMPARE(catalog.size(),0);QCOMPARE(persist.size(),0);QVERIFY(!receiver.persistencePending());
            QCOMPARE(receiver.presetInfo("round-pressure").value("preview").toString(),QString("context-preview"));
            const auto before=receiver.snapshot();source.setRadius(31);source.snapshot();
            QVERIFY(!receiver.restoreSharedSelection(source));QCOMPARE(receiver.snapshot(),before);
            QVERIFY(reference.restore(source.snapshot()));reference.setToolContext(tool,false);
            QVERIFY(receiver.restore(source.snapshot()));receiver.setToolContext(tool,false);QCOMPARE(receiver.snapshot(),reference.snapshot());
            source.setActiveColors(Qt::red,Qt::blue);source.snapshot();const auto beforeColor=receiver.snapshot();
            QVERIFY(!receiver.restoreSharedSelection(source));QCOMPARE(receiver.snapshot(),beforeColor);
        }
    }

    void resourceGridConsumesFastWheelAtBothEdges() {
        ResourcePreviewCacheProbe client;for(int i=0;i<100;++i)client.resources.append(QString::number(i));
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);
        QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        QQmlComponent component(&engine);component.setData(R"(
import QtQuick
import "qrc:/qml/brushes"
Flickable {objectName:"outer";width:300;height:380;contentWidth:width;contentHeight:1000;contentY:40;clip:true
    BrushResourcePicker {objectName:"resources";width:280;y:70;expanded:true}
})",QUrl());
        QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));auto *outer=qobject_cast<QQuickItem*>(object.data());QVERIFY(outer);
        QQuickWindow window;window.resize(300,380);outer->setParentItem(window.contentItem());window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));
        auto *grid=findVisual(outer,"resourcesGrid");QVERIFY(grid);QTRY_VERIFY(grid->height()>0 && grid->property("contentHeight").toReal()>grid->height());
        const auto wheel=[&](QQuickItem *target,QPoint angle,QPoint pixels,Qt::ScrollPhase phase){const auto local=target->mapToScene({target->width()/2,target->height()/2}).toPoint();QWheelEvent event(local,window.mapToGlobal(local),pixels,angle,Qt::NoButton,Qt::NoModifier,phase,false);QCoreApplication::sendEvent(&window,&event);};
        const auto first=grid->property("originY").toReal();const auto last=first+grid->property("contentHeight").toReal()-grid->height();
        for(int i=0;i<50;++i)wheel(grid,{0,-120},{},Qt::NoScrollPhase);
        QCOMPARE(grid->property("contentY").toReal(),last);QCOMPARE(outer->property("contentY").toReal(),40.);
        for(int i=0;i<50;++i)wheel(grid,{0,120},{},Qt::NoScrollPhase);
        QCOMPARE(grid->property("contentY").toReal(),first);QCOMPARE(outer->property("contentY").toReal(),40.);
        wheel(grid,{},{0,-37},Qt::ScrollUpdate);QCOMPARE(grid->property("contentY").toReal(),first+37);
        wheel(grid,{},{0,10000},Qt::ScrollMomentum);QCOMPARE(grid->property("contentY").toReal(),first);QCOMPARE(outer->property("contentY").toReal(),40.);
        // Outside the resource grid, the settings page still accepts normal wheel input.
        const QPoint local(290,350);QWheelEvent event(local,window.mapToGlobal(local),{},QPoint(0,-120),Qt::NoButton,Qt::NoModifier,Qt::NoScrollPhase,false);QCoreApplication::sendEvent(&window,&event);QTRY_VERIFY(outer->property("contentY").toReal()>40.);
        QCOMPARE(warnings,QStringList());
    }

    void directHistorySelectionAcrossEditingScopes() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("history-select.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushRadius(6);client.brushLibrary()->setPressureSize(false);client.brushLibrary()->setPressureFlow(false);
        for(int i=0;i<4;++i){QVERIFY(client.beginStroke({{qreal(14+i*8),32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),i+1);}
        const auto jump=[&](int depth,int total){
            const auto revision=client.revision();QVERIFY(client.selectHistory(depth));
            QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy() && !client.transformBusy() && !client.liquifyBusy());
            QCOMPARE(client.redoDepth(),total-depth);QCOMPARE(client.revision(),revision+1);QVERIFY(client.lastError().isEmpty());
        };
        jump(0,4);jump(4,4);
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());
        const auto initial=client.transformPoints();
        for(int i=1;i<=4;++i){auto points=initial;for(auto &v:points)v=v.toPointF()+QPointF(i*2,0);
            QVERIFY(client.updateTransform(points,false,0));QVERIFY(client.checkpointTransform());
            QTRY_COMPARE(client.undoDepth(),i);QTRY_VERIFY(!client.transformBusy());}
        const auto transformed=client.transformPoints();jump(0,4);QCOMPARE(client.transformPoints(),initial);jump(4,4);QCOMPARE(client.transformPoints(),transformed);
        QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive() && !client.layerEditBusy());QCOMPARE(client.undoDepth(),4);
        QVERIFY(client.beginLiquify());QTRY_VERIFY(client.liquifyActive() && !client.liquifyBusy());client.setLiquifySize(24);client.setLiquifyStrength(50);
        for(int i=0;i<3;++i){InputSample sample;sample.position={qreal(20+i*5),32};sample.pressure=1;
            QVERIFY(client.beginStroke(sample));sample.position+=QPointF(5,3);client.strokeTo(sample);client.endStroke();
            QTRY_COMPARE(client.undoDepth(),i+1);QTRY_VERIFY(!client.liquifyBusy());}
        jump(0,3);jump(3,3);QVERIFY(client.finishLiquify(false));QTRY_VERIFY(!client.liquifyActive() && !client.layerEditBusy());
        QCOMPARE(client.undoDepth(),4);QVERIFY(client.lastError().isEmpty());client.shutdown();
    }

    void removedRecentBrushesMigratePreferencesAndKeepCommittedColors() {
        BrushLibrary legacy;legacy.setToolIsolation(2);legacy.setRadius(17);const auto id=legacy.selectedId();
        auto state=QJsonDocument::fromJson(legacy.snapshot()).object();
        state["recentBrushes"]=QJsonObject{{"version",1},{"items",QJsonArray{QJsonObject{{"version",2},{"preset",state.value("presets").toArray().first()},{"writeBlend",0},{"dualEditor",QJsonObject{{"existing",false},{"sourceName",""}}}}}}};
        BrushLibrary restored;QVERIFY(restored.restore(QJsonDocument(state).toJson()));
        QCOMPARE(restored.selectedId(),id);QCOMPARE(restored.radius(),17.);QCOMPARE(restored.toolIsolation(),2);
        QVERIFY(!QJsonDocument::fromJson(restored.snapshot()).object().contains("recentBrushes"));
        QCOMPARE(restored.metaObject()->indexOfProperty("recentBrushes"),-1);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("without-recents.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushRadius(3);client.brushLibrary()->setPressureSize(false);client.brushLibrary()->setPressureFlow(false);
        const auto paint=[&]{QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();};
        client.setBrushColor(QColor("#112233"));paint();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(client.recentColors().contains(QString("#ff112233")));
        client.setBrushColor(QColor("#445566"));QVERIFY(client.beginStroke({{16,16},1}));client.cancelStroke();
        client.setBrushColor(QColor("#778899"));client.setBrushOpacity(0);paint();client.setBrushOpacity(1);
        client.setBrushColor(QColor("#abcdef"));paint();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(client.recentColors().contains(QString("#ffabcdef")));
        QVERIFY(!client.recentColors().contains(QString("#ff445566")));QVERIFY(!client.recentColors().contains(QString("#ff778899")));
        QVERIFY(!QJsonDocument::fromJson(client.brushLibrary()->snapshot()).object().contains("recentBrushes"));
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);QQmlComponent component(&engine,QUrl("qrc:/qml/brushes/BrushLibraryPanel.qml"));
        QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));auto *panel=qobject_cast<QQuickItem*>(object.data());QVERIFY(panel);
        QVERIFY(findVisual(panel,"brushSearch"));QVERIFY(findVisual(panel,"brushPresetList"));for(int i=0;i<8;++i)QVERIFY(!findVisual(panel,QString("recentBrush:%1").arg(i)));
        client.shutdown();
    }

    void tipShapeDialDragsAngleAndRotatedRoundness() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("tip-dial.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());auto *library=client.brushLibrary();
        library->setAngle(0);library->setRoundness(1);
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);
        QQmlComponent component(&engine);
        component.setData("import QtQuick\nimport \"qrc:/qml/brushes\"\nBrushTipShapeControl {angle:PaintClient.brushLibrary.angle; roundness:PaintClient.brushLibrary.roundness; onAngleEdited:value=>PaintClient.brushLibrary.angle=value; onRoundnessEdited:value=>PaintClient.brushLibrary.roundness=value}",QUrl());
        QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));
        auto *item=qobject_cast<QQuickItem*>(object.data());QVERIFY(item);
        QQuickWindow window;window.setColor(QColor("#25262b"));window.resize(300,130);item->setParentItem(window.contentItem());item->setX(10);item->setY(10);item->setWidth(280);item->setHeight(106);window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        auto *dial=findVisual(item,"settingsBrushTipDial");QVERIFY(dial);
        const auto at=[&](double x,double y){return dial->mapToScene({48+x*40,48+y*40}).toPoint();};
        const auto drag=[&](QPoint a,QPoint b){QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,a);QTest::mouseMove(&window,b,10);QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,b);};
        QSignalSpy changes(library,&BrushLibrary::settingsChanged),previews(library,&BrushLibrary::selectedPreviewChanged);
        QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,at(1,0));
        for(int i=1;i<=20;++i){const auto radians=i*3.141592653589793/40;QTest::mouseMove(&window,at(std::cos(radians),std::sin(radians)),10);}
        QCOMPARE(changes.size(),0);QCOMPARE(previews.size(),0);QCOMPARE(item->property("displayedAngle").toDouble(),90.);QCOMPARE(library->angle(),0.);
        QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,at(0,1));QCOMPARE(changes.size(),1);QCOMPARE(previews.size(),1);QCOMPARE(library->angle(),90.);
        library->setAngle(0);
        drag(at(1,0),at(0,1));QCOMPARE(library->angle(),90.);QCOMPARE(library->roundness(),1.);
        drag(at(-1,0),at(-.5,0));QCOMPARE(library->angle(),90.);QCOMPARE(library->roundness(),.5);
        drag(at(-.5,0),at(0,0));QCOMPARE(library->roundness(),.01);
        drag(at(0,0),at(-2,0));QCOMPARE(library->roundness(),1.);QCOMPARE(library->angle(),90.);
        QTest::mouseMove(&window,at(0,-1));QCOMPARE(library->angle(),90.);QCOMPARE(library->roundness(),1.);
        library->setAngle(-45);library->setRoundness(.37);
        auto *angle=findVisual(item,"settingsBrushangleInput");auto *roundness=findVisual(item,"settingsBrushroundnessInput");QVERIFY(angle && roundness);
        QCOMPARE(angle->property("value").toInt(),-45);QCOMPARE(roundness->property("value").toInt(),37);
        if(const auto capture=qEnvironmentVariable("DRAWVERSE_TEST_CAPTURE");!capture.isEmpty()){QTest::qWait(50);QVERIFY(window.grabWindow().save(capture));}
        angle->setProperty("value",-120);QVERIFY(QMetaObject::invokeMethod(angle,"valueModified"));QCOMPARE(library->angle(),-120.);
        roundness->setProperty("value",64);QVERIFY(QMetaObject::invokeMethod(roundness,"valueModified"));QCOMPARE(library->roundness(),.64);
        library->setAngle(12);library->setRoundness(.22);QCOMPARE(angle->property("value").toInt(),12);QCOMPARE(roundness->property("value").toInt(),22);
        auto *text=findVisual(item,"settingsBrushroundnessInputText");QVERIFY(text);text->setProperty("text","0");QVERIFY(!text->property("acceptableInput").toBool());text->setProperty("text","101");QVERIFY(!text->property("acceptableInput").toBool());text->setProperty("text","50");QVERIFY(text->property("acceptableInput").toBool());
        client.shutdown();
    }

    void tipShapeSettingsShareControlAndPreserveIndependentParameters() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("tip-settings.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        auto *library=client.brushLibrary();library->setAngle(23.25);library->setRoundness(.456);
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);
        QQmlComponent component(&engine,QUrl("qrc:/qml/brushes/BrushSettingsPanel.qml"));QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));
        auto *panel=qobject_cast<QQuickItem*>(object.data());QVERIFY(panel);auto *main=findVisual(panel,"settingsBrushTipShape");QVERIFY(main);
        QCOMPARE(library->angle(),23.25);QCOMPARE(library->roundness(),.456);QCOMPARE(main->property("angle").toDouble(),23.25);
        auto dual=library->dualBrush();dual["enabled"]=true;dual["angle"]=-67.;dual["roundness"]=.28;QVERIFY(library->setDualBrush(dual));
        panel->setProperty("category",4);auto *secondary=findVisual(panel,"settingsDualTipShape");QVERIFY(secondary);QVERIFY(secondary->isVisible());
        QCOMPARE(secondary->property("angle").toDouble(),-67.);QCOMPARE(secondary->property("roundness").toDouble(),.28);
        auto *edit=findVisual(panel,"settingsDualangleInput");QVERIFY(edit);edit->setProperty("value",80);QVERIFY(QMetaObject::invokeMethod(edit,"valueModified"));
        QCOMPARE(library->dualBrush().value("angle").toDouble(),80.);QCOMPARE(library->angle(),23.25);QCOMPARE(library->roundness(),.456);
        library->setDualExisting(true);QVERIFY(!secondary->isVisible());library->setDualExisting(false);QVERIFY(secondary->isVisible());
        BrushLibrary restored;QVERIFY(restored.restore(library->snapshot()));QCOMPARE(restored.angle(),23.25);QCOMPARE(restored.roundness(),.456);QCOMPARE(restored.dualBrush().value("angle").toDouble(),80.);QCOMPARE(restored.dualBrush().value("roundness").toDouble(),.28);
        client.shutdown();
    }


    void sharedBrushPreferencesRetainEachDocumentsIsolatedToolWithoutFeedback() {
        QTemporaryDir temp;PaintCoreClient a(nullptr,temp.filePath("a.ini"));QTRY_VERIFY(a.ready() && a.brushLibraryReady());PaintCoreClient b(nullptr,temp.filePath("a.ini"),a.brushCatalogConnection());QTRY_VERIFY2(b.ready() && b.brushLibraryReady(),qPrintable(b.lastError()));
        a.brushLibrary()->setToolIsolation(2);a.brushLibrary()->setRadius(3);a.activateEraser();a.brushLibrary()->setRadius(7);a.activateSmudge();a.brushLibrary()->setRadius(11);a.activateBrush();a.brushLibrary()->flushPersistence();
        b.activateEraser();QVERIFY(b.restoreBrushPreferences(a.brushLibrary()->snapshot(),a.brushLibrary()));QVERIFY(b.eraser());QCOMPARE(b.brushRadius(),7.);QVERIFY(!b.brushLibrary()->persistencePending());
        a.brushLibrary()->setRadius(5);QVERIFY(b.restoreBrushPreferences(a.brushLibrary()->snapshot(),a.brushLibrary()));QCOMPARE(b.brushRadius(),7.);QVERIFY(!b.brushLibrary()->persistencePending());
        const auto before=b.brushLibrary()->snapshot();auto malformed=QJsonDocument::fromJson(a.brushLibrary()->snapshot()).object();malformed["selected"]="missing";
        QVERIFY(!b.restoreBrushPreferences(QJsonDocument(malformed).toJson(),a.brushLibrary()));QCOMPARE(b.brushLibrary()->snapshot(),before);
        b.activateSmudge();QCOMPARE(b.brushRadius(),11.);b.brushLibrary()->setRadius(13);b.brushLibrary()->flushPersistence();a.brushLibrary()->flushPersistence();QVERIFY(a.restoreBrushPreferences(b.brushLibrary()->snapshot(),b.brushLibrary()));QCOMPARE(a.brushRadius(),5.);QVERIFY(!a.brushLibrary()->persistencePending());a.activateSmudge();QCOMPARE(a.brushRadius(),13.);a.shutdown();b.shutdown();
    }


    void brushToolIsolationLevelsPreserveParametersAndSharedCatalog() {
        BrushLibrary b;b.setRadius(10);b.setOpacity(.6);b.setFlow(.8);
        b.setEraser(true);b.setRadius(11);b.setEraser(false);QCOMPARE(b.radius(),11.); // Default remains shared.
        b.setToolIsolation(1);b.setRadius(20);b.setEraser(true);QCOMPARE(b.radius(),11.);
        b.setRadius(30);b.setOpacity(.4);b.setFlow(.3);b.setSpacing(.6);
        b.setToolContext(2);QCOMPARE(b.radius(),11.);b.setRadius(40);
        b.setToolContext(0);QCOMPARE(b.radius(),20.);QCOMPARE(b.opacity(),.6);QCOMPARE(b.flow(),.8);QCOMPARE(b.spacing(),.6);
        QVERIFY(b.select("round-fine"));b.setRadius(7);b.setEraser(true);QCOMPARE(b.selectedId(),QString("round-fine"));QCOMPARE(b.radius(),3.);
        b.setToolContext(0);QCOMPARE(b.radius(),7.);QVERIFY(b.select("round-pressure"));b.setHardness(.25);
        b.setToolIsolation(2);b.setEraser(true);QCOMPARE(b.radius(),20.);b.setRadius(31);b.setHardness(.9);b.setWriteBlend(8);QVERIFY(b.select("round-fine"));
        b.setToolContext(2);QCOMPARE(b.selectedId(),QString("round-pressure"));QCOMPARE(b.radius(),20.);QCOMPARE(b.hardness(),.25);b.setRadius(42);
        b.setToolContext(0);QCOMPARE(b.radius(),20.);QCOMPARE(b.hardness(),.25);QCOMPARE(b.writeBlend(),0);
        const auto copy=b.saveCopy("Shared copy");QVERIFY(!copy.isEmpty());const auto group=b.createGroup("Shared group");QVERIFY(b.movePresetToGroup(copy,group));QVERIFY(b.rename(copy,"Renamed"));const auto organization=b.organization();
        b.setEraser(true);QCOMPARE(b.selectedId(),QString("round-fine"));QCOMPARE(b.writeBlend(),8);QVERIFY(b.select("round-pressure"));QCOMPARE(b.radius(),31.);QCOMPARE(b.hardness(),.9);QCOMPARE(b.organization(),organization);QCOMPARE(b.presetInfo(copy).value("name").toString(),QString("Renamed"));
        QVERIFY(b.select(copy));b.setToolContext(0);QVERIFY(b.remove(copy));b.setEraser(true);QCOMPARE(b.selectedId(),QString("round-pressure"));QVERIFY(b.presetInfo(copy).isEmpty());
        b.setToolIsolation(0);b.setRadius(51);b.setToolContext(2);QCOMPARE(b.radius(),51.);QCOMPARE(b.toolIsolation(),0);
    }

    void brushToolIsolationRoundtripNativeSyncAndInvalidStateAreAtomic() {
        BrushLibrary b;b.setToolIsolation(2);b.setRadius(20);b.setToolContext(1);b.setRadius(31);b.setToolContext(2);b.setRadius(42);
        const auto stored=b.snapshot();BrushLibrary loaded;QVERIFY(loaded.restore(stored));QCOMPARE(loaded.toolContext(),2);QCOMPARE(loaded.radius(),42.);
        loaded.setToolContext(0);QCOMPARE(loaded.radius(),20.);loaded.setToolContext(1);QCOMPARE(loaded.radius(),31.);
        auto native=QJsonDocument::fromJson(loaded.catalogSnapshot()).object();auto rows=native.value("presets").toArray();auto row=rows[0].toObject();row["hardness"]=.5;rows[0]=row;native["presets"]=rows;
        QVERIFY(loaded.applyNativeCatalog(QJsonDocument(native).toJson()));QCOMPARE(loaded.hardness(),.5);loaded.setToolContext(0);QCOMPARE(loaded.hardness(),1.);QCOMPARE(loaded.radius(),20.);
        const auto before=loaded.snapshot();auto root=QJsonDocument::fromJson(before).object();auto isolation=root.value("toolIsolation").toObject();auto profiles=isolation.value("profiles").toArray();auto invalid=profiles[1].toObject();invalid["resetState"]=invalid;profiles[1]=invalid;isolation["profiles"]=profiles;root["toolIsolation"]=isolation;
        QVERIFY(!loaded.restore(QJsonDocument(root).toJson()));QCOMPARE(loaded.snapshot(),before);
        root=QJsonDocument::fromJson(before).object();isolation=root.value("toolIsolation").toObject();isolation["mode"]=1.5;root["toolIsolation"]=isolation;QVERIFY(!loaded.restore(QJsonDocument(root).toJson()));QCOMPARE(loaded.snapshot(),before);
        auto legacy=QJsonDocument::fromJson(before).object();legacy.remove("toolIsolation");QVERIFY(loaded.restore(QJsonDocument(legacy).toJson()));QCOMPARE(loaded.toolIsolation(),0);
        BrushLibrary preview;quint64 token=0;connect(&preview,&BrushLibrary::previewRequested,this,[&](const QString &,quint64 t,qreal,qreal,qreal,QVariantMap){token=t;});preview.requestPreview(preview.selectedId());QVERIFY(token);preview.acceptPreview(preview.selectedId(),token,"cached-preview");preview.setToolIsolation(2);preview.setToolContext(1);preview.setRadius(33);preview.setToolContext(0);QCOMPARE(preview.selectedPreview(),QString("cached-preview"));
    }

    void brushToolIsolationPreferenceAndClientSwitchesPersist() {
        QTemporaryDir temp;const auto path=temp.filePath("isolation.ini");
        {
            PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready() && client.brushLibraryReady());WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
            QQmlComponent component(&engine,QUrl("qrc:/qml/preferences/PreferencesPanel.qml"));QScopedPointer<QObject> dialog(component.create());QVERIFY2(dialog,qPrintable(component.errorString()));auto *combo=dialog->findChild<QObject*>("brushToolIsolation");QVERIFY(combo);QCOMPARE(combo->property("count").toInt(),3);combo->setProperty("currentIndex",2);QVERIFY(QMetaObject::invokeMethod(combo,"activated",Q_ARG(int,2)));auto *b=client.brushLibrary();QCOMPARE(b->toolIsolation(),2);
            b->setRadius(3);b->setPressureSize(false);client.activateEraser();b->setRadius(6);b->setOpacity(.5);client.activateSmudge();b->setRadius(9);b->setHardness(.4);client.activateMoveTool();QCOMPARE(b->toolContext(),2);client.activateBrush();QCOMPARE(client.brushRadius(),3.);QCOMPARE(client.brushOpacity(),1.);
            client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(Qt::black);QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(client.sampleDocumentPixel(32,32).alpha()>245);
            client.activateEraser();QCOMPARE(client.brushRadius(),6.);QCOMPARE(client.brushOpacity(),.5);b->setPressureSize(false);QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(client.sampleDocumentPixel(32,32).alpha()<180);
            client.activateSmudge();QCOMPARE(client.brushRadius(),9.);QCOMPARE(b->hardness(),.4);b->flushPersistence();QTRY_VERIFY(!b->persistencePending());QVERIFY(client.lastError().isEmpty());client.shutdown();
        }
        PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready() && reopened.brushLibraryReady());QCOMPARE(reopened.brushLibrary()->toolIsolation(),2);QCOMPARE(reopened.brushRadius(),3.);reopened.activateEraser();QCOMPARE(reopened.brushRadius(),6.);QCOMPARE(reopened.brushOpacity(),.5);reopened.activateSmudge();QCOMPARE(reopened.brushRadius(),9.);QVERIFY(std::abs(reopened.brushLibrary()->hardness()-.4)<1e-6);QVERIFY(reopened.lastError().isEmpty());reopened.shutdown();
    }

    void selected4kWhiteTransformRetainsOutsidePixelsAndCompactSelection() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("selected-4k.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(4096,2160);QTRY_COMPARE(client.documentWidth(),4096);QTRY_VERIFY(!client.layerEditBusy());
        client.requestViewport(0,{800,400,1200,700},{600,350});QVERIFY(client.editSelectionPath({{1024,512},{1280,512},{1280,768},{1024,768}},0,false));QTRY_VERIFY(!client.layerEditBusy());const auto depth=client.undoDepth();const auto originalSelection=client.selectionSteps();
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());const auto initial=client.transformPoints();QVERIFY(!client.residentPreview());
        for(int i=1;i<=6;++i){const auto revision=client.revision();auto points=initial;for(auto &v:points)v=v.toPointF()+QPointF(i*64,0);QVERIFY(client.updateTransform(points,false,0));QTRY_VERIFY(client.revision()>revision);}
        QTRY_VERIFY(client.sampleDocumentPixel(1100,600).alpha()<10);QVERIFY(client.sampleDocumentPixel(1500,600).alpha()>245);QVERIFY(client.sampleDocumentPixel(3000,1800).alpha()>245);
        QTRY_VERIFY(!client.selectionSteps().isEmpty() && client.selectionSteps().first().toMap().value("shape").toInt()==3);QVERIFY(client.selectionSteps().first().toMap().value("width").toDouble()<=258);QVERIFY(client.selectionSteps().first().toMap().value("height").toDouble()<=258);
        QVERIFY(client.finishTransform(true));QTRY_VERIFY(!client.transformActive());QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.lastError().isEmpty());
        client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(client.selectionSteps(),originalSelection);QVERIFY(client.sampleDocumentPixel(1100,600).alpha()>245);client.redo();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.sampleDocumentPixel(1100,600).alpha()<10);
        const auto before=client.selectionSteps();QVERIFY(client.beginTransform());QTRY_VERIFY(!client.transformBusy());auto points=client.transformPoints();for(auto &v:points)v=v.toPointF()+QPointF(-32,0);QVERIFY(client.updateTransform(points,false,0));QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive());QTRY_COMPARE(client.selectionSteps(),before);QVERIFY(client.lastError().isEmpty());client.shutdown();
    }
    void maskLinkButtonMovesIndependentlyAndPersistsWithUndo() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("links.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());auto owner=client.activeLayer();QVERIFY(client.addMask(owner));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());auto mask=client.activeLayer();
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        QQmlComponent component(&engine,QUrl("qrc:/qml/layers/LayersPanel.qml"));QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));auto *panel=qobject_cast<QQuickItem*>(object.data());QVERIFY(panel);QQuickWindow window;panel->setParentItem(window.contentItem());panel->setSize({280,500});window.resize(280,500);window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));
        const auto node=[&](quint64 id){for(const auto &v:client.layers()){const auto n=v.toMap();if(n.value("id").toULongLong()==id)return n;}return QVariantMap();};
        QQuickItem *link=nullptr;QTRY_VERIFY((link=findVisual(panel,QString("maskLink:%1").arg(owner))) && link->isVisible() && link->width()>0);QVERIFY(link->property("linked").toBool());const auto depth=client.undoDepth();
        QPointer<QQuickItem> retainedLink=link;
        QPointer<QQuickItem> retainedRow=findVisual(panel,QString("layerRow:%1").arg(owner));
        const auto checkRetained=[&]{QVERIFY(retainedLink);QVERIFY(retainedRow);QCOMPARE(findVisual(panel,QString("maskLink:%1").arg(owner)),retainedLink.data());QCOMPARE(retainedLink->opacity(),1.);};
        const auto retainedConnection=connect(&client,&PaintCoreClient::stateChanged,this,checkRetained);
        QTest::mouseClick(&window,Qt::LeftButton,Qt::NoModifier,link->mapToScene({5,12}).toPoint());QTRY_VERIFY(!node(mask).value("maskLinked").toBool());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);
        QVERIFY(client.moveLayer(owner,8,3));QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(node(owner).value("offsetX").toInt(),8);QCOMPARE(node(mask).value("offsetX").toInt(),0);client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(node(owner).value("offsetX").toInt(),0);
        client.undo();QTRY_VERIFY(node(mask).value("maskLinked").toBool());QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_VERIFY(!node(mask).value("maskLinked").toBool());QTRY_VERIFY(!client.layerEditBusy());
        checkRetained();disconnect(retainedConnection);
        const auto path=QUrl::fromLocalFile(temp.filePath("links.ora"));QVERIFY(client.saveDocument(path));QTRY_VERIFY(!client.fileBusy());QVERIFY(QFileInfo::exists(path.toLocalFile()));client.newDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.openDocument(path));QTRY_VERIFY(!client.fileBusy());QTRY_COMPARE(client.documentWidth(),64);QTRY_COMPARE(client.layers().size(),2);for(const auto &v:client.layers()){const auto n=v.toMap();if(n.value("mask").toBool()){mask=n.value("id").toULongLong();owner=n.value("parent").toULongLong();}}QVERIFY(!node(mask).value("maskLinked").toBool());
        link=nullptr;QTRY_VERIFY((link=findVisual(panel,QString("maskLink:%1").arg(owner))) && link->isVisible());QTest::mouseClick(&window,Qt::LeftButton,Qt::NoModifier,link->mapToScene({5,12}).toPoint());QTRY_VERIFY(node(mask).value("maskLinked").toBool());QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.moveLayer(owner,4,2));QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(node(mask).value("offsetX").toInt(),4);
        QVERIFY(client.groupLayer(owner,"group"));QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();QVERIFY(client.addMask(group));QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());const auto gm=client.activeLayer();link=nullptr;QTRY_VERIFY((link=findVisual(panel,QString("maskLink:%1").arg(group))) && link->isVisible());QTest::mouseClick(&window,Qt::LeftButton,Qt::NoModifier,link->mapToScene({5,12}).toPoint());QTRY_VERIFY(!node(gm).value("maskLinked").toBool());QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.moveLayer(group,3,1));QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(node(owner).value("offsetX").toInt(),7);QCOMPARE(node(mask).value("offsetX").toInt(),7);QCOMPARE(node(gm).value("offsetX").toInt(),0);
        QCOMPARE(warnings,QStringList());QVERIFY(client.lastError().isEmpty());window.close();client.shutdown();
    }
    void multipleLayersMoveTogetherAndCancelRestoresAll() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("multi-move.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());const auto a=client.activeLayer();
        client.setBrushRadius(3);QVERIFY(client.beginStroke({{12,12},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.addLayer("second");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto b=client.activeLayer();QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());
        client.selectLayers(a,int(Qt::ControlModifier));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectedLayers().size(),2);const auto selected=client.selectedLayers();const auto depth=client.undoDepth();
        const auto offset=[&](quint64 id){for(const auto &v:client.layers()){const auto l=v.toMap();if(l.value("id").toULongLong()==id)return QPoint(l.value("offsetX").toInt(),l.value("offsetY").toInt());}return QPoint(-999,-999);};
        QVERIFY(client.beginLayerMove({0,0},false));client.updateLayerMove({5,-3});QTRY_COMPARE(offset(a),QPoint(5,-3));QTRY_COMPARE(offset(b),QPoint(5,-3));QCOMPARE(client.undoDepth(),depth);QCOMPARE(client.selectedLayers(),selected);
        client.finishLayerMove({11,4},true);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(offset(a),QPoint(11,4));QCOMPARE(offset(b),QPoint(11,4));QCOMPARE(client.undoDepth(),depth+1);
        client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(offset(a),QPoint());QCOMPARE(offset(b),QPoint());
        QVERIFY(client.beginLayerMove({0,0},false));client.updateLayerMove({7,2});QTRY_COMPARE(offset(b),QPoint(7,2));client.finishLayerMove({},false);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(offset(a),QPoint());QCOMPARE(offset(b),QPoint());QCOMPARE(client.undoDepth(),depth);QVERIFY(client.lastError().isEmpty());client.shutdown();
    }
    void fullImageFlipClientRefreshesPixelsAndUndo() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("flip.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(65,41);QTRY_COMPARE(client.documentWidth(),65);QTRY_VERIFY(!client.layerEditBusy());
        client.requestViewport(0,{0,0,65,41},{65,41});client.setBrushRadius(3);client.setBrushColor(Qt::black);QVERIFY(client.beginStroke({{12,10},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!client.frame().isNull());QTRY_VERIFY(client.frame().pixelColor(12,10).red()<20);
        for(bool horizontal:{true,false}) {
            QVERIFY(client.flipImage(horizontal));QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(!client.transformActive());
            const auto x=horizontal?52:12,y=horizontal?10:30;QTRY_VERIFY(client.frame().pixelColor(x,y).red()<20);QTRY_VERIFY(client.frame().pixelColor(12,10).red()>240);
            client.undo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frame().pixelColor(12,10).red()<20);
        }
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.layerEditBusy());QVERIFY(!client.flipImage(true));QVERIFY(client.finishTransform(false));QTRY_VERIFY(!client.transformActive() && !client.layerEditBusy());QVERIFY(client.lastError().isEmpty());client.shutdown();
    }
    void selectionMaskAnimationDoesNotRasterizeAgain() {
        QQuickWindow window;window.resize(128,128);auto *mask=new SelectionOverlay();mask->setParentItem(window.contentItem());mask->setSize({128,128});mask->setDocumentSize({128,128});mask->setDocumentRect({0,0,128,128});mask->setMaskOnly(true);mask->setEnabledSelection(true);
        const auto step=[](int x){return QVariantMap{{"operation",0},{"shape",0},{"x",x},{"y",8},{"width",60},{"height",80}};};mask->setSteps({step(8)});window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));QTRY_VERIFY(mask->maskRasterizations()>0);
        const auto count=mask->maskRasterizations();QSignalSpy phases(mask,&SelectionOverlay::dashPhaseChanged);QTRY_VERIFY(phases.size()>2);QCOMPARE(mask->maskRasterizations(),count);
        mask->setSteps({step(20)});QTRY_VERIFY(mask->maskRasterizations()>count);window.close();
    }
    void residentSelectionAntsKeepTextureDuringNavigation() {
        QQuickWindow window;window.setColor(Qt::transparent);window.resize(560,320);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));
        if(window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software)QSKIP("Requires a real GPU scene graph");
        QQmlEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        QQmlComponent clientComponent(&engine);clientComponent.setData("import QtQuick; QtObject {property int documentWidth:4096;property int documentHeight:2160;property bool selectionEnabled:true;property var selectionSteps:[]}",QUrl());QScopedPointer<QObject> client(clientComponent.create());QVERIFY2(client,qPrintable(clientComponent.errorString()));
        QQmlComponent viewComponent(&engine);viewComponent.setData("import QtQuick; QtObject {property bool gpuDisplay:true;property real zoom:.125;property rect documentRect:Qt.rect(20,20,512,270);property rect selectionPreview:Qt.rect(0,0,0,0);property int selectionPreviewKind:0;property var selectionPreviewPoints:[]}",QUrl());QScopedPointer<QObject> view(viewComponent.create());QVERIFY2(view,qPrintable(viewComponent.errorString()));
        QVariantList steps;const auto step=[](int op,QRectF r){return QVariantMap{{"operation",op},{"shape",0},{"x",r.x()},{"y",r.y()},{"width",r.width()},{"height",r.height()}};};
        for(int i=0;i<24;++i){steps.append(step(i?1:0,{double(i*160),0,40,2160}));steps.append(step(1,{0,double(i*85),4096,30}));}client->setProperty("selectionSteps",steps);
        QQmlComponent presentation(&engine,QUrl("qrc:/qml/canvas/SelectionPresentation.qml"));QScopedPointer<QObject> object(presentation.createWithInitialProperties({{"client",QVariant::fromValue(client.data())},{"canvasSurface",QVariant::fromValue(view.data())}}));QVERIFY2(object,qPrintable(presentation.errorString()));auto *root=qobject_cast<QQuickItem*>(object.data());QVERIFY(root);root->setParentItem(window.contentItem());root->setSize({560,320});
        SelectionOverlay *mask=nullptr;QTRY_VERIFY((mask=qobject_cast<SelectionOverlay*>(findVisual(root,"selectionOutline"))) && mask->maskOnly());QTRY_VERIFY(mask->maskRasterizations()>0);
        QTRY_VERIFY(findVisual(root,"residentSelectionAntsShader"));auto grab=window.contentItem()->grabToImage();QVERIFY(grab);QTRY_VERIFY(!grab->image().isNull());const auto image=grab->image();if(const auto output=qEnvironmentVariable("DRAWVERSE_SELECTION_PREVIEW");!output.isEmpty())QVERIFY(image.save(output));int dark=0,light=0,clear=0;
        for(int y=0;y<image.height();++y)for(int x=0;x<image.width();++x){const auto c=image.pixelColor(x,y);if(c.alpha()<10)++clear;else if(c.alpha()>30){if(c.red()>180)++light;if(c.red()<50)++dark;}}
        QVERIFY2(dark>50 && light>50,qPrintable(QString("black=%1 white=%2").arg(dark).arg(light)));QVERIFY(clear>image.width()*image.height()/3);
        const auto count=mask->maskRasterizations();const auto path=mask->documentPath();QSignalSpy geometry(mask,&SelectionOverlay::pathChanged);QSignalSpy phases(mask,&SelectionOverlay::dashPhaseChanged);
        for(int i=0;i<12;++i){QSignalSpy frames(&window,&QQuickWindow::frameSwapped);const auto zoom=.12+i*.002;view->setProperty("zoom",zoom);view->setProperty("documentRect",QRectF(20+i,20-i,4096*zoom,2160*zoom));window.update();QTRY_VERIFY(!frames.isEmpty());}
        QTRY_VERIFY(phases.size()>2);QCOMPARE(mask->maskRasterizations(),count);QCOMPARE(geometry.size(),0);QVERIFY(mask->documentPath()==path);QCOMPARE(mask->documentRect(),QRectF(0,0,4096,2160));qInfo("resident ants: %d path elements, mask rasterizations %llu -> %llu across 12 displayed navigation frames",path.elementCount(),static_cast<unsigned long long>(count),static_cast<unsigned long long>(mask->maskRasterizations()));QCOMPARE(warnings,QStringList());window.close();
    }
    void selectionPresentationRetainsSoftwareAndLargeDocumentFallback() {
        QQmlEngine engine;QQmlComponent probe(&engine);probe.setData("import QtQuick; QtObject {property bool gpuDisplay:false;property int documentWidth:64;property int documentHeight:64;property bool selectionEnabled:true;property var selectionSteps:[];property real zoom:1;property rect documentRect:Qt.rect(0,0,64,64);property rect selectionPreview:Qt.rect(0,0,0,0);property int selectionPreviewKind:0;property var selectionPreviewPoints:[]}",QUrl());QScopedPointer<QObject> data(probe.create());QVERIFY2(data,qPrintable(probe.errorString()));data->setProperty("selectionSteps",QVariantList{QVariantMap{{"operation",0},{"shape",0},{"x",8},{"y",8},{"width",32},{"height",32}}});
        QQmlComponent presentation(&engine,QUrl("qrc:/qml/canvas/SelectionPresentation.qml"));QScopedPointer<QObject> object(presentation.createWithInitialProperties({{"client",QVariant::fromValue(data.data())},{"canvasSurface",QVariant::fromValue(data.data())}}));QVERIFY2(object,qPrintable(presentation.errorString()));auto *root=qobject_cast<QQuickItem*>(object.data());QVERIFY(root);auto *outline=qobject_cast<SelectionOverlay*>(findVisual(root,"selectionOutline"));QVERIFY(outline && !outline->maskOnly());QVERIFY(outline->documentPath().contains({10,10}));QVERIFY(!outline->documentPath().contains({50,50}));
        data->setProperty("documentWidth",8192);data->setProperty("gpuDisplay",true);QVERIFY(!root->property("resident").toBool());QVERIFY(!findVisual(root,"residentSelectionAntsShader"));QVERIFY(outline->documentPath().contains({10,10}));
    }
    void repeatedWandClicksDoNotGrowSelectionOrPublishNewGeometry() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("wand-repeat.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());client.newDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());client.setBrushRadius(4);client.setBrushColor(Qt::black);QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.activateWand();
        QVERIFY(client.selectWand({0,0},1));QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.selectWand({32,32},1));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().size(),2);
        const auto before=client.selectionSteps();const auto revision=client.revision();const auto history=client.undoDepth();QSignalSpy changes(&client,&PaintCoreClient::selectionChanged);
        for(int i=0;i<16;++i){QVERIFY(client.selectWand({32,32},1));QTRY_VERIFY(!client.layerEditBusy());}
        QCOMPARE(client.selectionSteps(),before);QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),history);QCOMPARE(changes.size(),0);QVERIFY(client.lastError().isEmpty());client.shutdown();
    }
    void visibleResourceThumbnailSurvivesSharedCacheEviction() {
        ResourcePreviewCacheProbe client;const auto first=QString(64,'a'),second=QString(64,'b');
        QImage image(2,2,QImage::Format_RGBA8888);image.fill(Qt::white);QByteArray bytes;QBuffer buffer(&bytes);QVERIFY(buffer.open(QIODevice::WriteOnly));QVERIFY(image.save(&buffer,"PNG"));
        const auto png="data:image/png;base64,"+QString::fromLatin1(bytes.toBase64());client.cache.insert(first,png);
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);
        QQmlComponent component(&engine,QUrl("qrc:/qml/brushes/BrushResourceThumbnail.qml"));
        QScopedPointer<QObject> thumbnail(component.createWithInitialProperties({{"resource",first}}));QVERIFY2(thumbnail,qPrintable(component.errorString()));
        QCOMPARE(thumbnail->property("imageSource").toString(),png);QCOMPARE(client.requests,0);
        client.cache.clear();for(int i=0;i<70;++i)client.changed();
        QCOMPARE(thumbnail->property("imageSource").toString(),png);QCOMPARE(client.requests,0);
        thumbnail->setProperty("resource",second);QVERIFY(thumbnail->property("imageSource").toString().isEmpty());QCOMPARE(client.requests,1);
        client.cache.insert(second,png);client.changed();QCOMPARE(thumbnail->property("imageSource").toString(),png);
    }
    void selfOutlineTextureIsIndependentRendersAndExchangesToFreshStore_data() {
        QTest::addColumn<bool>("sdf");QTest::newRow("path")<<false;QTest::newRow("sdf")<<true;
    }
    void selfOutlineTextureIsIndependentRendersAndExchangesToFreshStore() {
        QFETCH(bool,sdf);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("self-outline.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());auto *b=client.brushLibrary();QVERIFY(b->select("round-fine"));b->setRadius(4);b->setPressureSize(false);b->setPressureFlow(false);const auto main=b->texture();
        auto outline=b->strokeOutline();outline["enabled"]=true;if(sdf)outline["model"]="stroke-sdf-v1";outline["secondaryColor"]=true;QVERIFY(b->setStrokeOutline(outline));
        const auto paint=[](PaintCoreClient &c){InputSample p;p.position={20.5,32.5};p.pressure=1;p.tool=1;QVERIFY(c.beginStroke(p));p.position={44.5,32.5};c.strokeTo(p);c.endStroke();};
        const auto render=[](PaintCoreClient &c){c.requestViewport(0,{0,0,64,64},{64,64});return c.frameRevision()>=c.revision() && c.frame().size()==QSize(64,64);};
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(Qt::red);b->setActiveColors(Qt::red,Qt::blue);paint(client);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(render(client));const auto plain=client.frame();int blue=0;for(int y=0;y<64;++y)for(int x=0;x<64;++x)if(plain.pixelColor(x,y).blue()>100)++blue;QVERIFY(blue>20);
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});QQmlComponent component(&engine,QUrl("qrc:/qml/brushes/BrushSettingsPanel.qml"));QScopedPointer<QObject> object(component.createWithInitialProperties({{"category",12}}));QVERIFY2(object,qPrintable(component.errorString()));auto *panel=qobject_cast<QQuickItem*>(object.data());QVERIFY(panel);auto *picker=findVisual(panel,"settingsOutlineTexturePicker");QVERIFY(picker && picker->isVisible());QCOMPARE(picker->property("resourceRole").toInt(),4);
        auto *algorithm=findVisual(panel,"settingsOutlineAlgorithm");QVERIFY(algorithm);QCOMPARE(algorithm->property("currentIndex").toInt(),sdf?1:0);
        if(sdf){algorithm->setProperty("currentIndex",0);QVERIFY(QMetaObject::invokeMethod(algorithm,"activated",Q_ARG(int,0)));QCOMPARE(b->strokeOutline().value("model").toString(),QString("stroke-contour-v1"));algorithm->setProperty("currentIndex",1);QVERIFY(QMetaObject::invokeMethod(algorithm,"activated",Q_ARG(int,1)));QCOMPARE(b->strokeOutline().value("model").toString(),QString("stroke-sdf-v1"));QVERIFY(b->usePresetAsOutline("round-pressure"));QVERIFY(findVisual(panel,"settingsOutlinePreset")->isVisible());QVERIFY(!findVisual(panel,"settingsOutlineBrushSize")->isVisible());QVERIFY(b->setStrokeOutline(outline));}
        QImage image(4,4,QImage::Format_RGBA8888);image.fill(Qt::black);const auto path=temp.filePath("black.png");QVERIFY(image.save(path));QSignalSpy files(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushTexture(QUrl::fromLocalFile(path),0,true,true));QTRY_COMPARE(files.size(),1);QVERIFY2(files.last()[0].toBool(),qPrintable(client.lastError()));
        const auto stored=b->strokeOutline();const auto source=stored.value("source").toMap();QCOMPARE(source.value("model").toString(),QString("self-brush"));const auto resource=source.value("texture").toMap().value("resource").toString();QCOMPARE(resource.size(),64);QCOMPARE(b->texture(),main);QCOMPARE(QJsonDocument::fromJson(b->catalogSnapshot()).object().value("version").toInt(),sdf?43:42);
        b->requestPreview(b->selectedId());QTRY_VERIFY2_WITH_TIMEOUT(!b->selectedPreview().isEmpty(),qPrintable(b->previewMessage()),15000);
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());paint(client);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(render(client));const auto expected=client.frame();QVERIFY(expected!=plain);QVERIFY(expected.pixelColor(32,32).red()>200);for(int y=0;y<64;++y)for(int x=0;x<64;++x)QVERIFY(expected.pixelColor(x,y).blue()==0);client.undo();QTRY_COMPARE(client.undoDepth(),0);client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(render(client));QCOMPARE(client.frame(),expected);
        const auto archive=QUrl::fromLocalFile(temp.filePath("self-outline.vbr"));QVERIFY(client.exportBrush(archive));QTRY_COMPARE(files.size(),2);QVERIFY(files.last()[0].toBool());PaintCoreClient target(nullptr,temp.filePath("fresh/state.ini"));QTRY_VERIFY(target.ready() && target.brushLibraryReady());QSignalSpy imported(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(imported.size(),1);QVERIFY2(imported.last()[0].toBool(),qPrintable(target.lastError()));QCOMPARE(target.brushLibrary()->strokeOutline(),stored);target.newTransparentDocument(64,64);QTRY_COMPARE(target.documentWidth(),64);QTRY_VERIFY(!target.layerEditBusy());target.setBrushColor(Qt::red);target.brushLibrary()->setActiveColors(Qt::red,Qt::blue);paint(target);QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(render(target));QCOMPARE(target.frame(),expected);
        const auto before=b->snapshot();auto bad=QJsonDocument::fromJson(before).object();bad["version"]=sdf?42:41;QVERIFY(!b->restore(QJsonDocument(bad).toJson()));QCOMPARE(b->snapshot(),before);QVERIFY(b->setStrokeOutline(outline));QVERIFY(b->setStrokeOutline(stored));QCOMPARE(b->strokeOutline(),stored);QCOMPARE(b->texture(),main);QCOMPARE(warnings,QStringList());b->flushPersistence();QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);target.shutdown();PaintCoreClient reopened(nullptr,temp.filePath("self-outline.ini"));QTRY_VERIFY(reopened.ready() && reopened.brushLibraryReady());QCOMPARE(reopened.brushLibrary()->strokeOutline(),stored);QCOMPARE(reopened.brushLibrary()->texture(),main);reopened.shutdown();
    }

    void selfOutlineTextureAsyncImportKeepsCapturedRoleAndPreset() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("self-target.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());auto *b=client.brushLibrary();const auto target=b->selectedId();const auto main=b->texture();QImage image(4,4,QImage::Format_RGBA8888);image.fill(Qt::white);image.setPixelColor(0,0,Qt::black);const auto path=temp.filePath("texture.png");QVERIFY(image.save(path));QSignalSpy files(&client,&PaintCoreClient::brushFileFinished);
        QVERIFY(client.importBrushTexture(QUrl::fromLocalFile(path),0,true,true));QVERIFY(b->usePresetAsOutline("round-fine"));QVERIFY(b->select("round-fine"));QTRY_COMPARE(files.size(),1);QVERIFY(files.last()[0].toBool());QCOMPARE(b->strokeOutline().value("source").toMap().value("model").toString(),QString("self-brush"));QVERIFY(!b->strokeOutline().value("source").toMap().contains("texture"));QVERIFY(b->select(target));const auto stored=b->strokeOutline();QCOMPARE(stored.value("source").toMap().value("model").toString(),QString("self-brush"));const auto resource=stored.value("source").toMap().value("texture").toMap().value("resource").toString();QCOMPARE(resource.size(),64);QCOMPARE(b->texture(),main);QTRY_VERIFY(client.brushResources().contains(resource));QVERIFY(b->setOutlineTextureResource(target,QString{},true));QVERIFY(!b->strokeOutline().value("source").toMap().contains("texture"));QVERIFY(client.selectBrushResource(resource,4));QCOMPARE(b->strokeOutline(),stored);client.shutdown();
    }

    void outlineTextureCanBeChosenWithoutMainTextureAndImportKeepsTarget() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("outline-texture.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());auto *library=client.brushLibrary();const auto target=library->selectedId();const auto main=library->texture();
        auto outline=library->strokeOutline();outline["enabled"]=true;QVERIFY(library->setStrokeOutline(outline));
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);QQmlComponent component(&engine,QUrl("qrc:/qml/brushes/BrushSettingsPanel.qml"));QScopedPointer<QObject> object(component.createWithInitialProperties({{"category",12}}));QVERIFY2(object,qPrintable(component.errorString()));
        auto *panel=qobject_cast<QQuickItem*>(object.data());QVERIFY(panel);auto *source=findVisual(panel,"settingsOutlineSource");QVERIFY(source);source->setProperty("currentIndex",2);QVERIFY(QMetaObject::invokeMethod(source,"activated",Q_ARG(int,2)));
        auto *picker=findVisual(panel,"settingsOutlineTexturePicker");auto *importButton=findVisual(panel,"importOutlineTexture");QVERIFY(picker && picker->isVisible() && importButton && importButton->isVisible());QCOMPARE(library->strokeOutline(),outline);QCOMPARE(library->texture(),main);
        auto *algorithm=findVisual(panel,"settingsOutlineAlgorithm");QVERIFY(algorithm);QCOMPARE(algorithm->property("currentIndex").toInt(),0);QCOMPARE(algorithm->property("currentText").toString(),QString::fromUtf8("建路径"));
        QImage texture(4,4,QImage::Format_RGBA8888);texture.fill(Qt::white);texture.setPixelColor(0,0,Qt::black);const auto input=temp.filePath("texture.png");QVERIFY(texture.save(input));QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);
        QVERIFY(client.importBrushTexture(QUrl::fromLocalFile(input),0,true));QVERIFY(library->select("round-fine"));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QVERIFY(library->texture().value("resource").isNull());QCOMPARE(library->strokeOutline().value("source").toMap().value("model").toString(),QString("self-brush"));
        QVERIFY(library->select(target));const auto stored=library->strokeOutline();QCOMPARE(stored.value("source").toMap().value("model").toString(),QString("texture"));const auto resource=stored.value("source").toMap().value("texture").toMap().value("resource").toString();QCOMPARE(resource.size(),64);QVERIFY(stored.value("enabled").toBool());QCOMPARE(library->texture(),main);
        BrushLibrary restored;QVERIFY(restored.restore(library->snapshot()));QVERIFY(restored.select(target));QCOMPARE(restored.strokeOutline(),stored);QCOMPARE(restored.texture(),main);
        QTRY_VERIFY(client.brushResources().contains(resource));auto reset=stored;reset["source"]=QVariantMap{{"model","self-brush"}};QVERIFY(library->setStrokeOutline(reset));source->setProperty("currentIndex",2);QVERIFY(QMetaObject::invokeMethod(source,"activated",Q_ARG(int,2)));QVERIFY(picker->isVisible());
        const int resourceIndex=client.brushResources().indexOf(resource);QVERIFY(resourceIndex>=0);QVERIFY(QMetaObject::invokeMethod(picker,"activated",Q_ARG(int,resourceIndex)));QCOMPARE(library->strokeOutline().value("source").toMap().value("model").toString(),QString("texture"));QCOMPARE(library->texture(),main);client.shutdown();
    }
    void multiBrushMoveIsAtomicOrderedAndRejectsCycles() {
        BrushLibrary library;const auto group=library.createGroup("Destination"),child=library.createGroup("Child",group);QVERIFY(!group.isEmpty() && !child.isEmpty());
        const auto preset=[](const QString &id){return QVariantMap{{"kind","preset"},{"id",id}};};
        const auto groupItem=[](const QString &id){return QVariantMap{{"kind","group"},{"id",id}};};
        const auto original=library.organization().value("roots").toList();QStringList ordered;
        for(const auto &value:original){const auto item=value.toMap();if(item.value("kind")=="preset")ordered.append(item.value("id").toString());}QVERIFY(ordered.size()>=3);
        QVERIFY(library.selectBrushItems({preset(ordered[1]),preset(ordered[0])},ordered[0]));
        QSignalSpy changes(&library,&BrushLibrary::presetsChanged);QVERIFY(library.moveSelectedItemsToGroupAt(group,0));QCOMPARE(changes.size(),1);
        const auto itemsOf=[&](const QString &id){for(const auto &value:library.organization().value("groups").toList()){const auto item=value.toMap();if(item.value("id").toString()==id)return item.value("items").toList();}return QVariantList{};};
        auto moved=itemsOf(group);QCOMPARE(moved.size(),3);QCOMPARE(moved[0].toMap().value("id").toString(),ordered[0]);QCOMPARE(moved[1].toMap().value("id").toString(),ordered[1]);
        QVERIFY(library.selectBrushItems({groupItem(group),groupItem(child),preset(ordered[0])}));const auto before=library.snapshot();
        QVERIFY(!library.moveSelectedItemsToGroupAt(child,0));QCOMPARE(library.snapshot(),before);QVERIFY(!library.moveSelectedItemsToGroupAt("missing",0));QCOMPARE(library.snapshot(),before);
        QVERIFY(library.moveSelectedItemsToGroupAt("",0));QCOMPARE(itemsOf(group),moved);QCOMPARE(library.organization().value("roots").toList().first().toMap().value("id").toString(),group);
        QVERIFY(library.selectBrushItems({preset(ordered[0]),preset(ordered[1])},ordered[0]));QVERIFY(library.moveSelectedItemsToGroupAt("",1));
        const auto roots=library.organization().value("roots").toList();QCOMPARE(roots[1].toMap().value("id").toString(),ordered[0]);QCOMPARE(roots[2].toMap().value("id").toString(),ordered[1]);
        QVERIFY(library.moveSelectedItemsToGroupAt("",3));QCOMPARE(library.organization().value("roots").toList(),roots);
        BrushLibrary restored;QVERIFY(restored.restore(library.snapshot()));QCOMPARE(restored.organization(),library.organization());
    }
    void multiBrushDropMovesEntireSelectionIntoGroup() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("brush-drop.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());auto *library=client.brushLibrary();
        const auto group=library->createGroup("Destination");QVERIFY(!group.isEmpty());QVERIFY(library->selectBrushItems({QVariantMap{{"kind","preset"},{"id","round-pressure"}},QVariantMap{{"kind","preset"},{"id","round-fine"}}},"round-pressure"));
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);QQmlComponent component(&engine,QUrl("qrc:/qml/brushes/BrushLibraryPanel.qml"));
        QQuickWindow window;QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));auto *panel=qobject_cast<QQuickItem*>(object.data());QVERIFY(panel);panel->setParentItem(window.contentItem());panel->setWidth(280);panel->setHeight(600);window.resize(280,600);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));
        QQuickItem *row=nullptr;QTRY_VERIFY((row=findVisual(panel,"brushGroup:"+group)) && row->width()>32);
        QMimeData mime;mime.setData("application/x-drawverse-brush","preset:round-pressure");const auto point=row->mapToScene({row->width()/2,row->height()-2}).toPoint();
        QDragEnterEvent enter(point,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(&window,&enter);QDragMoveEvent move(point,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(&window,&move);QDropEvent drop(point,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(&window,&drop);QVERIFY(drop.isAccepted());
        QVariantList items;for(const auto &value:library->organization().value("groups").toList())if(value.toMap().value("id").toString()==group)items=value.toMap().value("items").toList();QCOMPARE(items.size(),2);QSet<QString> ids;for(const auto &value:items)ids.insert(value.toMap().value("id").toString());QVERIFY(ids.contains("round-pressure") && ids.contains("round-fine"));QCOMPARE(library->selectedBrushItems().size(),2);client.shutdown();
    }
    void brushGroupsCreateDirectlyAndRenameInline() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("brush-group-edit.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());auto *library=client.brushLibrary();
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
        QQmlComponent component(&engine,QUrl("qrc:/qml/brushes/BrushLibraryPanel.qml"));QQuickWindow window;QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));
        auto *panel=qobject_cast<QQuickItem*>(object.data());QVERIFY(panel);panel->setParentItem(window.contentItem());panel->setWidth(280);panel->setHeight(600);window.resize(280,600);window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));
        const auto groups=[&](){return library->organization().value("groups").toList();};
        for(const auto &preset:QStringList{"round-pressure","round-fine"}) {
            QVERIFY(library->selectBrushItems({QVariantMap{{"kind","preset"},{"id",preset}}},preset));
            QVERIFY(QMetaObject::invokeMethod(panel,"createSelectedGroup"));
        }
        QTRY_COMPARE(groups().size(),2);QString group;QSet<QString> names;for(const auto &value:groups()){const auto row=value.toMap();names.insert(row.value("name").toString());QCOMPARE(row.value("items").toList().size(),1);if(row.value("name").toString()==QString::fromUtf8("笔刷组1"))group=row.value("id").toString();}QVERIFY(names.contains(QString::fromUtf8("笔刷组1")) && names.contains(QString::fromUtf8("笔刷组2")));QVERIFY(!group.isEmpty());
        const auto nameOf=[&](){for(const auto &value:groups()){const auto row=value.toMap();if(row.value("id").toString()==group)return row.value("name").toString();}return QString();};
        QQuickItem *label=nullptr,*toggle=nullptr,*editor=nullptr,*list=nullptr;
        QTRY_VERIFY((label=findVisual(panel,"brushGroupName:"+group)) && label->width()>32);QTRY_VERIFY((toggle=findVisual(panel,"brushGroupToggle:"+group)));QTRY_VERIFY((list=findVisual(panel,"brushPresetList")));
        const auto count=list->property("count").toInt();
        QTest::mouseClick(&window,Qt::LeftButton,Qt::NoModifier,label->mapToScene({40,15}).toPoint());QCOMPARE(list->property("count").toInt(),count);
        QTest::mouseClick(&window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({10,15}).toPoint());QTRY_COMPARE(list->property("count").toInt(),count-1);
        QTest::mouseClick(&window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({10,15}).toPoint());QTRY_COMPARE(list->property("count").toInt(),count);
        QVERIFY(QMetaObject::invokeMethod(list,"forceLayout"));QSignalSpy frame(&window,&QQuickWindow::frameSwapped);window.update();QTRY_VERIFY(!frame.isEmpty());QTest::mouseDClick(&window,Qt::LeftButton,Qt::NoModifier,label->mapToScene({40,15}).toPoint());
        QTRY_VERIFY((editor=findVisual(panel,"brushGroupNameEditor:"+group)) && editor->isVisible());QTRY_VERIFY(editor->hasActiveFocus());editor->setProperty("text","discard");QTest::keyClick(&window,Qt::Key_Escape);
        QTRY_VERIFY(!editor->isVisible());QCOMPARE(nameOf(),QString::fromUtf8("笔刷组1"));
        QTest::mouseDClick(&window,Qt::LeftButton,Qt::NoModifier,label->mapToScene({40,15}).toPoint());QTRY_VERIFY(editor->isVisible());QTRY_VERIFY(editor->hasActiveFocus());editor->setProperty("text",QString::fromUtf8("原位改名"));QTest::keyClick(&window,Qt::Key_Return);
        QTRY_COMPARE(nameOf(),QString::fromUtf8("原位改名"));client.shutdown();
    }
    void brushGroupCollapseKeepsUnchangedDelegates() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("brush-groups.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());auto *library=client.brushLibrary();
        QVERIFY(library->selectBrushItems({QVariantMap{{"kind","preset"},{"id","round-pressure"}}},"round-pressure"));QVERIFY(library->groupSelectedItems("Group"));
        const auto group=library->organization().value("groups").toList().last().toMap().value("id").toString();QVERIFY(!group.isEmpty());
        QQmlEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
        QQmlComponent component(&engine,QUrl("qrc:/qml/brushes/BrushLibraryPanel.qml"));QQuickWindow window;QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));
        auto *panel=qobject_cast<QQuickItem*>(object.data());QVERIFY(panel);panel->setParentItem(window.contentItem());panel->setWidth(280);panel->setHeight(600);window.resize(280,600);window.show();
        QQuickItem *list=nullptr;QTRY_VERIFY((list=findVisual(panel,"brushPresetList")));QTRY_COMPARE(list->property("count").toInt(),5);
        QPointer<QQuickItem> retained;QTRY_VERIFY((retained=findVisual(panel,"brushPreset:round-fine")));
        for(int i=0;i<3;++i) {
            QVERIFY(QMetaObject::invokeMethod(panel,"toggleGroup",Q_ARG(QVariant,group)));QTRY_COMPARE(list->property("count").toInt(),4);QCoreApplication::processEvents();
            QVERIFY(retained);QCOMPARE(findVisual(panel,"brushPreset:round-fine"),retained.data());
            QVERIFY(QMetaObject::invokeMethod(panel,"toggleGroup",Q_ARG(QVariant,group)));QTRY_COMPARE(list->property("count").toInt(),5);QCoreApplication::processEvents();
            QVERIFY(retained);QCOMPARE(findVisual(panel,"brushPreset:round-fine"),retained.data());
        }
        QCOMPARE(warnings,QStringList());client.shutdown();
    }
    void brushScalarControlsCommitIntegerDisplayUnits() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("integer.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);
        QQmlComponent scalar(&engine,QUrl("qrc:/qml/controls/ParameterSlider.qml"));
        QScopedPointer<QObject> control(scalar.createWithInitialProperties({{"sliderName","integer"},{"value",37.6}}));QVERIFY2(control,qPrintable(scalar.errorString()));
        auto *slider=control->findChild<QObject*>("integer");auto *edit=control->findChild<QObject*>("integerInputText");QVERIFY(slider && edit);
        QSignalSpy commits(control.data(),SIGNAL(committed(double)));QVERIFY(commits.isValid());QCOMPARE(slider->property("stepSize").toInt(),1);
        slider->setProperty("value",37.6);QVERIFY(QMetaObject::invokeMethod(slider,"moved"));QCOMPARE(commits.size(),1);QCOMPARE(commits.last()[0].toDouble(),38.);
        edit->setProperty("text","37.6");QVERIFY(!edit->property("acceptableInput").toBool());edit->setProperty("text","37");QVERIFY(edit->property("acceptableInput").toBool());
        QQmlComponent settings(&engine,QUrl("qrc:/qml/brushes/BrushSettingsPanel.qml"));QScopedPointer<QObject> panel(settings.create());QVERIFY2(panel,qPrintable(settings.errorString()));
        int fields=0;
        for(int category=0;category<=12;++category){panel->setProperty("category",category);QCoreApplication::processEvents();
            for(auto *child:panel->findChildren<QObject*>())if(child->property("precisionFactor").isValid()){QCOMPARE(child->property("decimals").toInt(),0);QCOMPARE(child->property("precisionFactor").toInt(),1);++fields;}
        }
        QVERIFY(fields>40);client.shutdown();
    }
    void groupRowsAreCompactAndHaveNoAggregateThumbnail() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("groups.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        const auto layer=client.activeLayer();QVERIFY(client.groupSelectedLayers("Group"));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
        QQmlComponent component(&engine,QUrl("qrc:/qml/layers/LayersPanel.qml"));QQuickWindow window;
        QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));
        auto *panel=qobject_cast<QQuickItem*>(object.data());QVERIFY(panel);panel->setParentItem(window.contentItem());panel->setWidth(280);panel->setHeight(500);window.resize(280,500);window.show();
        QQuickItem *groupRow=nullptr,*layerRow=nullptr;
        QTRY_VERIFY((groupRow=findVisual(panel,QString("layerRow:%1").arg(group))));
        QTRY_VERIFY((layerRow=findVisual(panel,QString("layerRow:%1").arg(layer))));
        QCOMPARE(groupRow->height(),24.);QCOMPARE(layerRow->height(),34.);
        auto *groupPreview=findVisual(panel,QString("contentLayerThumbnail:%1").arg(group));auto *layerPreview=findVisual(panel,QString("contentLayerThumbnail:%1").arg(layer));
        QVERIFY(groupPreview && layerPreview);QVERIFY(!groupPreview->isVisible());QVERIFY(layerPreview->isVisible());client.shutdown();
    }
    void layerAndGroupNamesEditInlineAndEscapePreservesHistory() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("rename.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        client.newDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());const auto layer=client.activeLayer();
        QVERIFY(client.groupSelectedLayers("Group"));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);
        QQmlComponent component(&engine,QUrl("qrc:/qml/layers/LayersPanel.qml"));QQuickWindow window;
        QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));auto *panel=qobject_cast<QQuickItem*>(object.data());QVERIFY(panel);
        panel->setParentItem(window.contentItem());panel->setWidth(280);panel->setHeight(500);window.resize(280,500);window.show();window.requestActivate();QVERIFY(QTest::qWaitForWindowActive(&window));
        const auto nameOf=[&](quint64 id){for(const auto &value:client.layers()){const auto node=value.toMap();if(node.value("id").toULongLong()==id)return node.value("name").toString();}return QString();};
        for(const auto id:{layer,group}) {
            QQuickItem *label=nullptr,*editor=nullptr;QTRY_VERIFY((label=findVisual(panel,QString("layerName:%1").arg(id))));
            QTRY_VERIFY(label->width()>32 && label->height()>0);
            const auto before=nameOf(id);const auto after=QString::fromUtf8("改名%1").arg(id);const auto depth=client.undoDepth();
            QTest::mouseDClick(&window,Qt::LeftButton,Qt::NoModifier,label->mapToScene({label->width()/2,10}).toPoint());
            QTRY_VERIFY((editor=findVisual(panel,QString("layerNameEditor:%1").arg(id))) && editor->isVisible());
            editor->setProperty("text",after);QTest::keyClick(&window,Qt::Key_Return);QTRY_COMPARE(nameOf(id),after);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);
            label=findVisual(panel,QString("layerName:%1").arg(id));QVERIFY(label);
            QTest::mouseDClick(&window,Qt::LeftButton,Qt::NoModifier,label->mapToScene({label->width()/2,10}).toPoint());
            QTRY_VERIFY((editor=findVisual(panel,QString("layerNameEditor:%1").arg(id))) && editor->isVisible());editor->setProperty("text","cancelled");QTest::keyClick(&window,Qt::Key_Escape);
            QTRY_VERIFY(!editor->isVisible());QCOMPARE(nameOf(id),after);QCOMPARE(client.undoDepth(),depth+1);
            client.undo();QTRY_COMPARE(nameOf(id),before);client.redo();QTRY_COMPARE(nameOf(id),after);QTRY_VERIFY(!client.layerEditBusy());
        }
        QVERIFY(client.lastError().isEmpty());client.shutdown();
    }
    void nativeCatalogRefreshPreservesLargeLibraryMultiSelection() {
        BrushLibrary library;auto state=QJsonDocument::fromJson(library.snapshot()).object();
        auto rows=state.value("presets").toArray();const auto original=rows.first().toObject();
        const auto id=[](int i){return QString("00000000-0000-4000-8000-%1").arg(i+1,12,10,QLatin1Char('0'));};
        for(int i=0;i<400;++i){auto row=original;row["id"]=id(i);row["name"]=QString("Brush %1").arg(i);rows.append(row);}
        state["presets"]=rows;QVERIFY(library.restore(QJsonDocument(state).toJson()));
        const auto group=library.createGroup("Selected group");QVERIFY(!group.isEmpty());
        const QVariantList selected{QVariantMap{{"kind","preset"},{"id",id(0)}},QVariantMap{{"kind","preset"},{"id",id(399)}},QVariantMap{{"kind","group"},{"id",group}}};
        QVERIFY(library.selectBrushItems(selected,id(0)));QVERIFY(library.brushSelectionBlocksPainting());
        auto native=QJsonDocument::fromJson(library.catalogSnapshot()).object();auto changed=native.value("presets").toArray();
        auto row=changed.first().toObject();row["radius"]=19.;changed[0]=row;native["presets"]=changed;
        QVERIFY(library.applyNativeCatalog(QJsonDocument(native).toJson()));QCOMPARE(library.selectedBrushItems(),selected);
        QVERIFY(library.brushSelectionBlocksPainting());QCOMPARE(library.selectedId(),id(0));
        QVERIFY(library.restore(library.snapshot()));QCOMPARE(library.selectedBrushItems().size(),1);
    }
    void toolPercentSlidersAndInputsStaySynchronized() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("percent.ini"));
        QTRY_VERIFY(client.ready() && client.brushLibraryReady());
        QQmlEngine engine;
        for(const auto &tool:QStringList{"bucket","smudge"}) {
            const bool bucket=tool=="bucket";
            QQmlComponent component(&engine,QUrl(bucket?"qrc:/qml/tools/BucketOptionsBar.qml":"qrc:/qml/tools/SmudgeOptionsBar.qml"));
            QScopedPointer<QObject> bar(component.createWithInitialProperties({{"client",QVariant::fromValue(&client)}}));
            QVERIFY2(bar,qPrintable(component.errorString()));
            const auto key=bucket?QString("bucketOpacity"):QString("smudgeStrength");
            auto *slider=bar->findChild<QObject*>(key+"InlineSlider");auto *edit=bar->findChild<QObject*>(key+"Input");
            QVERIFY(slider && edit);QCOMPARE(slider->property("stepSize").toInt(),1);
            slider->setProperty("value",75);QVERIFY(QMetaObject::invokeMethod(slider,"moved"));
            QCOMPARE(bucket?client.bucketOpacity():client.smudgeStrength(),.75);QCOMPARE(edit->property("text").toString(),QString("75%"));
            edit->setProperty("text","40%");QVERIFY(QMetaObject::invokeMethod(edit,"editingFinished"));
            QCOMPARE(bucket?client.bucketOpacity():client.smudgeStrength(),.4);QCOMPARE(slider->property("value").toInt(),40);
            QCOMPARE(client.brushOpacity(),1.);
        }
        client.shutdown();
    }
};
int main(int argc,char **argv) {
    QGuiApplication app(argc,argv);Q_INIT_RESOURCE(ui_resources);QQuickStyle::setStyle("Basic");
    qmlRegisterType<MenuSurface>("DrawVerse",1,0,"MenuSurface");qmlRegisterType<SelectionOverlay>("DrawVerse",1,0,"SelectionOutline");
    qmlRegisterType<DocumentManager>("DrawVerse",1,0,"DocumentManager");qmlRegisterType<CanvasItem>("DrawVerse",1,0,"PaintCanvas");qmlRegisterType<ColorWheelItem>("DrawVerse",1,0,"ColorWheel");
    WorkflowTests test;return QTest::qExec(&test,argc,argv);
}
#include "workflow_tests.moc"
