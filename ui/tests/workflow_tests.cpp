#include "engine/PaintCoreClient.h"
#include "canvas/SelectionOverlay.h"
#include <QQuickItemGrabResult>
#include <QSGRendererInterface>
#include <QtTest>
#include <QQmlEngine>
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
#include "platform/MenuSurface.h"
#include "workspace/WorkspaceManager.h"

static QQuickItem *findVisual(QQuickItem *item,const QString &name){if(item->objectName()==name)return item;for(auto *child:item->childItems())if(auto *found=findVisual(child,name))return found;return nullptr;}

class ResourcePreviewCacheProbe : public QObject {
    Q_OBJECT
    Q_PROPERTY(quint64 brushResourcePreviewRevision MEMBER revision NOTIFY brushResourcePreviewsChanged)
public:
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
    void outlineTextureCanBeChosenWithoutMainTextureAndImportKeepsTarget() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("outline-texture.ini"));QTRY_VERIFY(client.ready() && client.brushLibraryReady());auto *library=client.brushLibrary();const auto target=library->selectedId();const auto main=library->texture();
        auto outline=library->strokeOutline();outline["enabled"]=true;QVERIFY(library->setStrokeOutline(outline));
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);QQmlComponent component(&engine,QUrl("qrc:/qml/brushes/BrushSettingsPanel.qml"));QScopedPointer<QObject> object(component.createWithInitialProperties({{"category",12}}));QVERIFY2(object,qPrintable(component.errorString()));
        auto *panel=qobject_cast<QQuickItem*>(object.data());QVERIFY(panel);auto *source=findVisual(panel,"settingsOutlineSource");QVERIFY(source);source->setProperty("currentIndex",2);QVERIFY(QMetaObject::invokeMethod(source,"activated",Q_ARG(int,2)));
        auto *picker=findVisual(panel,"settingsOutlineTexturePicker");auto *importButton=findVisual(panel,"importOutlineTexture");QVERIFY(picker && picker->isVisible() && importButton && importButton->isVisible());QCOMPARE(library->strokeOutline(),outline);QCOMPARE(library->texture(),main);
        auto *algorithm=findVisual(panel,"settingsOutlineAlgorithm");QVERIFY(algorithm);QCOMPARE(algorithm->property("text").toString(),QString::fromUtf8("算法：建路径"));
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
    WorkflowTests test;return QTest::qExec(&test,argc,argv);
}
#include "workflow_tests.moc"
