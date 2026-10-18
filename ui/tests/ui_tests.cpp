#include "canvas/CanvasItem.h"
#include "color/ColorWheelItem.h"
#include "canvas/SelectionOverlay.h"
#include "workspace/WorkspaceManager.h"
#include "documents/DocumentManager.h"
#include "platform/UiScale.h"
#include "platform/MenuSurface.h"
#include <QtTest>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQmlProperty>
#include <QQuickWindow>
#include <QQuickItemGrabResult>
#include <QQuickStyle>
#include <QTemporaryDir>
#include <QSettings>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QCryptographicHash>
#include <QDataStream>
#include <QPointingDevice>
#include <QTabletEvent>
#include <QDir>
#include <QFileInfo>
#include <QMimeData>
#include <QDropEvent>
#include <QDragEnterEvent>
#include <QDragMoveEvent>
#include <QWheelEvent>
#include <QHoverEvent>
#include <QEnterEvent>
#include <QStyleHints>
#include <QPainter>
#include <QScreen>
#include <QScopeGuard>
#include <cmath>
#include <limits>
#ifdef Q_OS_WIN
#include <QtCore/qt_windows.h>
#include <thread>
#endif

namespace {
// Independent generated ABR wire fixture, not an external Photoshop asset.
QByteArray abrEntryFixture(bool bad=false,double opacity=100,bool pressure=false,int scatter=-1,int color=-1,bool perTip=true,int shapeDynamics=-1,int deviceControl=-1) {
    const auto be16=[](quint16 n){QByteArray b;QDataStream out(&b,QIODevice::WriteOnly);out<<n;return b;};
    const auto be32=[](quint32 n){QByteArray b;QDataStream out(&b,QIODevice::WriteOnly);out<<n;return b;};
    const auto text=[&](QString value){auto b=be32(quint32(value.size()));for(const auto unit:value)b+=be16(unit.unicode());return b;};
    const auto id=[&](QByteArray value){return be32(value.size()==4?0:quint32(value.size()))+value;};
    const auto typed=[](QByteArray tag,QByteArray value){return tag+value;};
    const auto object=[&](QByteArray type,QList<QPair<QByteArray,QByteArray>> rows){auto b=text({})+id(type)+be32(quint32(rows.size()));for(const auto &row:rows)b+=id(row.first)+row.second;return b;};
    const auto unit=[&](QByteArray kind,double value){QByteArray b;QDataStream out(&b,QIODevice::WriteOnly);out<<value;return typed("UntF",kind+b);};
    const auto preset=[&](bool sample,bool invalid){QList<QPair<QByteArray,QByteArray>> shape{{"Dmtr",unit("#Pxl",invalid?513:8)},{"Angl",unit("#Ang",0)},{"Rndn",unit("#Prc",deviceControl>=0?50:100)},{"Spcn",unit("#Prc",2)}};if(sample)shape.append({"sampledData",typed("TEXT",text("shared"))});else shape.append({"Hrdn",unit("#Prc",100)});QList<QPair<QByteArray,QByteArray>> entries{{"Nm  ",typed("TEXT",text("Same"))},{"Brsh",typed("Objc",object(sample?"sampledBrush":"computedBrush",shape))},{"unknownTexture",typed("bool",QByteArray(1,char(1)))}};if(opacity!=100){QByteArray value;QDataStream out(&value,QIODevice::WriteOnly);out<<opacity;entries.append({"toolOptions",typed("Objc",object("paintbrushTool",{{"Opct",typed("doub",value)}}))});}if(pressure){for(const auto &key:{"useTipDynamics","usePaintDynamics"})entries.append({key,typed("bool",QByteArray(1,char(QByteArray(key)=="useTipDynamics" && shapeDynamics>=0?shapeDynamics:1)))});entries.append({"minimumDiameter",unit("#Prc",20)});for(const auto &item:QList<QPair<QByteArray,double>>{{"szVr",20},{"prVr",40},{"opVr",30}})entries.append({item.first,typed("Objc",object("brushDynamics",{{"bVTy",typed("long",be32(2))},{"fStp",typed("long",be32(0))},{"jitter",unit("#Prc",item.first=="szVr" && shapeDynamics>=0?60:0)},{"Mnm ",unit("#Prc",item.second)}}))});}if(scatter>=0){entries.append({"useScatter",typed("bool",QByteArray(1,char(scatter)))});entries.append({"bothAxes",typed("bool",QByteArray(1,char(1)))});QByteArray number;QDataStream count(&number,QIODevice::WriteOnly);count<<3.;entries.append({"Cnt ",typed("doub",number)});for(const auto &item:QList<QPair<QByteArray,double>>{{"scatterDynamics",75},{"countDynamics",50}})entries.append({item.first,typed("Objc",object("brushDynamics",{{"bVTy",typed("long",be32(0))},{"fStp",typed("long",be32(25))},{"jitter",unit("#Prc",item.second)},{"Mnm ",unit("#Prc",0)}}))});}if(color>=0){entries.append({"useColorDynamics",typed("bool",QByteArray(1,char(color)))});entries.append({"colorDynamicsPerTip",typed("bool",QByteArray(1,char(perTip)))});for(const auto &item:QList<QPair<QByteArray,double>>{{"H   ",20},{"Strt",30},{"Brgh",10},{"purity",50}})entries.append({item.first,unit("#Prc",item.second)});entries.append({"clVr",typed("Objc",object("brushDynamics",{{"bVTy",typed("long",be32(0))},{"fStp",typed("long",be32(25))},{"jitter",unit("#Prc",80)},{"Mnm ",unit("#Prc",0)}}))});}if(shapeDynamics>=0){entries.append({"minimumRoundness",unit("#Prc",25)});for(const auto &item:QList<QPair<QByteArray,double>>{{"angleDynamics",40},{"roundnessDynamics",50}})entries.append({item.first,typed("Objc",object("brushDynamics",{{"bVTy",typed("long",be32(deviceControl>=0?(item.first=="angleDynamics"?deviceControl:3):0))},{"fStp",typed("long",be32(25))},{"jitter",unit("#Prc",item.second)},{"Mnm ",unit("#Prc",0)}}))});}return typed("Objc",object("brushPreset",entries));};
    const auto section=[&](QByteArray key,QByteArray payload){return QByteArray("8BIM")+key+be32(quint32(payload.size()))+payload;};
    auto descriptor=be32(16)+object("null",{{"Brsh",typed("VlLs",be32(3)+preset(true,false)+preset(false,false)+preset(true,bad))}});
    auto sample=QByteArray(1,char(6))+QByteArray("shared")+QByteArray(264,char(0))+be32(0)+be32(0)+be32(8)+be32(8)+be16(8)+QByteArray(1,char(0));QByteArray coverage(64,char(-1));for(int y=3;y<5;++y)for(int x=3;x<5;++x)coverage[y*8+x]=0;sample+=coverage;
    auto samples=be32(quint32(sample.size()))+sample+QByteArray((4-sample.size()%4)%4,char(0));return be16(6)+be16(2)+section("desc",descriptor)+section("samp",samples);
}
class BlurTestBackdrop final : public QQuickPaintedItem {
public:
    explicit BlurTestBackdrop(QQuickItem *parent):QQuickPaintedItem(parent) { setOpaquePainting(true); }
    void showCornerPattern() {m_cornerPattern=true;update();}
    void paint(QPainter *painter) override {
        for(int x=0;x<width();x+=80)
            painter->fillRect(QRectF(x,0,80,height()),(x/80)%2?QColor(230,40,210):QColor(30,230,60));
        if(m_cornerPattern)for(int y=0;y<24;y+=2)for(int x=0;x<24;x+=2)
            painter->fillRect(QRectF(x,y,2,2),(x/2+y/2)%2?Qt::white:Qt::black);
    }
private:
    bool m_cornerPattern=false;
};
QWindowList applicationWindows(){auto windows=QGuiApplication::allWindows();windows.removeIf([](QWindow *window){return window->objectName()=="applicationOutlineWindow";});return windows;}
int visibleWindowsCount(){int count=0;for(auto *window:applicationWindows())if(window->isVisible())++count;return count;}
QJsonObject groupedBrushState(const BrushLibrary &library) {
    auto root=QJsonDocument::fromJson(library.snapshot()).object();
    const auto selected=root.value("selected").toString();
    const QString outer="12345678-1234-4234-8234-000000000100",empty="12345678-1234-4234-8234-000000000101";
    const auto item=[](const QString &kind,const QString &id){return QJsonObject{{"kind",kind},{"id",id}};};
    QJsonArray roots{item("group",outer)};
    const auto old=root.value("presets").toArray();QJsonArray ordered;
    for(const auto &row:old)if(row.toObject().value("id").toString()==selected)ordered.append(row);
    for(const auto &row:old)if(row.toObject().value("id").toString()!=selected){ordered.append(row);roots.append(item("preset",row.toObject().value("id").toString()));}
    const QJsonObject organization{{"model","ordered-tree-v1"},{"roots",roots},{"groups",QJsonArray{
        QJsonObject{{"id",outer},{"name","same"},{"items",QJsonArray{item("preset",selected),item("group",empty)}}},
        QJsonObject{{"id",empty},{"name","same"},{"items",QJsonArray{}}}}}};
    root["version"]=20;root["presets"]=ordered;root["organization"]=organization;
    auto reset=root.value("resetState").toObject();QJsonArray baselines;
    const auto oldReset=reset.value("presets").toArray();
    for(const auto &row:ordered)for(const auto &baseline:oldReset)if(row.toObject().value("id")==baseline.toObject().value("id"))baselines.append(baseline);
    reset["version"]=20;reset["presets"]=baselines;reset["organization"]=organization;root["resetState"]=reset;return root;
}
QQuickItem *findVisualItem(QQuickItem *root,const QString &name) {
    if(root->objectName()==name) return root;
    for(auto *item:root->childItems()) if(auto *found=findVisualItem(item,name)) return found;
    return nullptr;
}
}

class UiTests final : public QObject {
    Q_OBJECT
    QTemporaryDir m_preferences;
private slots:

    void curvePointKeepsGrabOutsideGraph_data() {
        QTest::addColumn<bool>("speed");QTest::newRow("pressure")<<false;QTest::newRow("speed")<<true;
    }
    void curvePointKeepsGrabOutsideGraph() {
        QFETCH(bool,speed);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("curve.ini"));QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        const QVariantList points{QVariantList{0.,0.},QVariantList{.4,.5},QVariantList{1.,1.}};
        QVERIFY(speed?library->setSpeedControl("flow",points,0):library->setPressureControl("flow",points,0));
        QQmlEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);QQmlComponent component(&engine);
        component.setData(speed?QByteArray("import QtQuick\nimport \"qrc:/qml/brushes\"\nSpeedCurveEditor {library:PaintClient.brushLibrary;target:\"flow\";prefix:\"dragCurve\";curve:library.speedFlowCurve;setControl:(points,minimum)=>library.setSpeedControl(\"flow\",points,minimum)}"):QByteArray("import QtQuick\nimport \"qrc:/qml/brushes\"\nPressureCurveEditor {library:PaintClient.brushLibrary;target:\"flow\";prefix:\"dragCurve\"}"),QUrl());
        QScopedPointer<QObject> object(component.create());QVERIFY2(object,qPrintable(component.errorString()));auto *item=qobject_cast<QQuickItem*>(object.data());QVERIFY(item);
        QQuickWindow window;window.resize(400,350);item->setParentItem(window.contentItem());item->setX(30);item->setY(40);item->setWidth(280);item->setHeight(240);window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));
        auto *graph=findVisualItem(item,"dragCurveCurve");auto *area=findVisualItem(item,"dragCurveDragArea");QVERIFY(graph && area);QVERIFY(area->property("preventStealing").toBool());
        const auto at=[&](qreal x,qreal y){return graph->mapToScene({7+x*(graph->width()-14),graph->height()-7-y*(graph->height()-14)}).toPoint();};
        QTest::mousePress(&window,Qt::LeftButton,Qt::NoModifier,at(.4,.5));QTest::mouseMove(&window,at(1.3,1.4));QTest::mouseMove(&window,at(.6,.65));
        const auto edited=[&](){return speed?library->speedFlowCurve():library->pressureDynamics().value("flow").toMap().value("curve").toList();};
        QTRY_VERIFY(std::abs(edited()[1].toList()[0].toDouble()-.6)<.02);QVERIFY(std::abs(edited()[1].toList()[1].toDouble()-.65)<.03);
        QTest::mouseRelease(&window,Qt::LeftButton,Qt::NoModifier,at(.6,.65));const auto released=edited();QTest::mouseMove(&window,at(.3,.3));QCOMPARE(edited(),released);
        item->setParentItem(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }

    void brushSettingsCreatesOnlyActiveCategoryAndLabelsChoices() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("settings.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());QTRY_VERIFY(client.ready());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);
        for(const auto &v:workspace.leftGroups()+workspace.rightGroups()){const auto g=v.toMap();if(g.value("panels").toStringList().contains("brush-settings"))workspace.setActive(g.value("id").toString(),"brush-settings");}
        QQuickItem *panel=nullptr;QTRY_VERIFY((panel=findVisualItem(window->contentItem(),"brushSettingsPanel")));
        QVERIFY(findVisualItem(panel,"settingsBrushSize"));QVERIFY(!findVisualItem(panel,"pressureSizeCurve"));QVERIFY(!findVisualItem(panel,"settingsDualSizeMode"));
        QVERIFY(findVisualItem(panel,"imageTipInterpretationLabel"));
        panel->setProperty("category",4);
        QQuickItem *mode=nullptr;QTRY_VERIFY((mode=findVisualItem(panel,"settingsDualSizeMode")));
        QVERIFY(findVisualItem(panel,"settingsDualSizeModeLabel"));QVERIFY(!findVisualItem(panel,"settingsBrushSize"));
        mode->setProperty("currentIndex",1);QVERIFY(QMetaObject::invokeMethod(mode,"activated",Q_ARG(int,1)));
        auto *absolute=findVisualItem(panel,"settingsDualdiameter");auto *relative=findVisualItem(panel,"settingsDualsizingdiameterOffset");QVERIFY(absolute && relative);QVERIFY(!absolute->isVisible());QVERIFY(relative->isVisible());
        panel->setProperty("category",6);QTRY_VERIFY(findVisualItem(panel,"pressureSizeCurve")==nullptr);QTRY_VERIFY(findVisualItem(panel,"pressureFlowCurve"));
        panel->setProperty("category",4);QTRY_VERIFY(findVisualItem(panel,"settingsDualSizeMode"));QCOMPARE(client.brushLibrary()->dualBrush().value("sizing").toMap().value("mode").toString(),QString("relative"));
        panel->setProperty("category",7);QTRY_VERIFY(findVisualItem(panel,"brushCursorImageOpacitySlider"));QVERIFY(!findVisualItem(panel,"pressureFlowCurve"));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }

    void standardBrushPreviewKeepsBaseEditsAndUpdatesOnlyItsRow() {
        BrushLibrary library;
        const auto id=library.selectedId();
        QSignalSpy requests(&library,&BrushLibrary::previewRequested);
        library.requestPreview(id);QCOMPARE(requests.size(),1);
        const auto token=requests.first()[1].toULongLong();
        library.setRadius(80);library.setFlow(.12);library.setOpacity(.24);
        library.requestPreview(id);QCOMPARE(requests.size(),1);
        QSignalSpy catalog(&library,&BrushLibrary::presetsChanged);
        QSignalSpy previews(&library,&BrushLibrary::previewChanged);
        library.acceptPreview(id,token,"first");
        QCOMPARE(catalog.size(),0);QCOMPARE(previews.size(),1);
        QCOMPARE(library.selectedPreview(),QString("first"));
        BrushLibrary native;QVERIFY(native.restore(library.snapshot()));
        QVERIFY(library.applyNativeCatalog(native.catalogSnapshot()));
        QCOMPARE(library.selectedPreview(),QString("first"));
        library.retryPreview();QCOMPARE(requests.size(),2);
        QCOMPARE(requests.last()[2].toDouble(),24.);QCOMPARE(requests.last()[3].toDouble(),1.);
        QCOMPARE(requests.last()[5].toMap().value("flow").toDouble(),1.);
        QCOMPARE(requests.last()[5].toMap().value("presetRadius").toDouble(),24.);
        library.acceptPreview(id,requests.last()[1].toULongLong(),"second");
        library.setHardness(.25);library.requestPreview(id);QCOMPARE(requests.size(),3);
        library.acceptPreview(id,token,"stale");QVERIFY(library.selectedPreview().isEmpty());
        QCOMPARE(library.radius(),80.);QVERIFY(std::abs(library.flow()-.12)<1e-6);QVERIFY(std::abs(library.opacity()-.24)<1e-6);
    }

    void brushGroupsState20MirrorBaselinesCopyDeleteAndFlatImports() {
        BrushLibrary source;source.setRadius(7);const auto id=source.saveCopy("grouped baseline");QVERIFY(!id.isEmpty());source.setRadius(11);
        BrushLibrary library;QVERIFY(library.restore(QJsonDocument(groupedBrushState(source)).toJson()));
        const auto organization=library.organization();QCOMPARE(organization.value("groups").toList().size(),2);
        QVERIFY(library.resetPreset());QCOMPARE(library.radius(),7.);QCOMPARE(library.organization(),organization);
        library.setRadius(9);library.setOpacity(.23);const auto second=library.saveCopy("second");QVERIFY(!second.isEmpty());
        auto state=QJsonDocument::fromJson(library.snapshot()).object();QCOMPARE(state.value("version").toInt(),20);
        QCOMPARE(state.value("organization"),state.value("resetState").toObject().value("organization"));
        const auto groups=state.value("organization").toObject().value("groups").toArray();
        QCOMPARE(groups[0].toObject().value("items").toArray().last().toObject().value("id").toString(),second);
        BrushLibrary reopened;QVERIFY(reopened.restore(library.snapshot()));QCOMPARE(reopened.catalogSnapshot(),library.catalogSnapshot());
        reopened.setRadius(13);QVERIFY(reopened.resetPreset());QCOMPARE(reopened.radius(),9.);QCOMPARE(reopened.opacity(),.23);
        QVERIFY(reopened.remove(second));QVERIFY(reopened.remove(id));QCOMPARE(reopened.organization().value("groups").toList().size(),2);
        BrushLibrary flat;QVERIFY(reopened.importPackage(flat.exportPackage()));QCOMPARE(reopened.organization().value("groups").toList().size(),2);
        BrushLibrary verify;QVERIFY(verify.restore(reopened.snapshot()));QCOMPARE(verify.catalogSnapshot(),reopened.catalogSnapshot());
        QVERIFY(verify.applyNativeCatalog(flat.catalogSnapshot()));QVERIFY(verify.organization().isEmpty());QCOMPARE(verify.catalogSnapshot(),flat.catalogSnapshot());
    }
    void brushGroupsInvalidTreesAreAtomicAndExplicitFlatNormalizes() {
        BrushLibrary library;auto state=groupedBrushState(library);QVERIFY(library.restore(QJsonDocument(state).toJson()));const auto good=library.snapshot();
        QList<QJsonObject> invalid;
        auto bad=state;bad["organization"]=QJsonValue::Null;invalid.append(bad);
        bad=state;bad["version"]=19;invalid.append(bad);bad=state;bad["version"]=20.5;invalid.append(bad);
        auto organization=state.value("organization").toObject();auto changed=organization;changed["model"]=QJsonObject{{"ordered-tree-v1",QJsonValue::Null}};bad=state;bad["organization"]=changed;invalid.append(bad);
        changed=organization;changed["extra"]=true;bad=state;bad["organization"]=changed;invalid.append(bad);
        auto roots=organization.value("roots").toArray();roots.append(roots.first());changed=organization;changed["roots"]=roots;bad=state;bad["organization"]=changed;invalid.append(bad);
        auto groups=organization.value("groups").toArray();auto group=groups[0].toObject();group["items"]=QJsonArray{QJsonObject{{"kind","group"},{"id",group.value("id")}}};groups[0]=group;changed=organization;changed["groups"]=groups;bad=state;bad["organization"]=changed;invalid.append(bad);
        bad=state;auto rows=state.value("presets").toArray();const QJsonValue first=rows[0];rows[0]=rows[1];rows[1]=first;bad["presets"]=rows;invalid.append(bad);
        for(const auto &input:invalid){QVERIFY(!library.restore(QJsonDocument(input).toJson()));QCOMPARE(library.snapshot(),good);}
        BrushLibrary flat;auto explicitFlat=QJsonDocument::fromJson(flat.catalogSnapshot()).object();explicitFlat["version"]=20;
        QJsonArray items;for(const auto &row:explicitFlat.value("presets").toArray())items.append(QJsonObject{{"kind","preset"},{"id",row.toObject().value("id")}});
        explicitFlat["organization"]=QJsonObject{{"model","ordered-tree-v1"},{"roots",items},{"groups",QJsonArray{}}};
        BrushLibrary normalized;QVERIFY(normalized.restore(QJsonDocument(explicitFlat).toJson()));QCOMPARE(normalized.catalogSnapshot(),flat.catalogSnapshot());
    }
    void brushGroupsQtOperationsAreAtomicAndPreserveOrder() {
        BrushLibrary library;QVERIFY(library.restore(QJsonDocument(groupedBrushState(library)).toJson()));
        const auto selected=library.selectedId();
        const auto group=library.createGroup("New group");QVERIFY(!group.isEmpty());
        const auto nested=library.createGroup("Nested",group);QVERIFY(!nested.isEmpty());
        QVERIFY(library.movePresetToGroup(selected,nested));
        auto groupRows=QJsonDocument::fromJson(library.snapshot()).object().value("organization").toObject().value("groups").toArray();bool nestedContainsPreset=false;
        for(const auto &row:groupRows)if(row.toObject().value("id").toString()==nested)nestedContainsPreset=row.toObject().value("items").toArray().last().toObject().value("id").toString()==selected;
        QVERIFY(nestedContainsPreset);
        QVERIFY(library.movePresetToGroupAt(selected,nested,0));
        const auto orderedCopy=library.saveCopy("Ordered child");QVERIFY(!orderedCopy.isEmpty());QVERIFY(library.movePresetToGroupAt(orderedCopy,nested,0));
        groupRows=QJsonDocument::fromJson(library.snapshot()).object().value("organization").toObject().value("groups").toArray();
        for(const auto &row:groupRows)if(row.toObject().value("id").toString()==nested){const auto items=row.toObject().value("items").toArray();QCOMPARE(items[0].toObject().value("id").toString(),orderedCopy);QCOMPARE(items[1].toObject().value("id").toString(),selected);}
        const auto invalidMove=library.snapshot();QVERIFY(!library.moveGroupToGroup(group,nested));QCOMPARE(library.snapshot(),invalidMove);
        QVERIFY(library.moveGroupToGroupAt(nested,"",0));
        QCOMPARE(QJsonDocument::fromJson(library.snapshot()).object().value("organization").toObject().value("roots").toArray()[0].toObject().value("id").toString(),nested);
        QVERIFY(library.moveGroupToGroup(nested,group));
        QVERIFY(library.select(selected));
        QVERIFY(library.renameGroup(group,"Renamed"));QVERIFY(!library.renameGroup(group,"  "));
        const auto renamed=library.snapshot();QVERIFY(!library.removeGroup(group,true));QCOMPARE(library.snapshot(),renamed);
        QVERIFY(library.ungroup(nested));QVERIFY(library.ungroup(group));QCOMPARE(library.selectedId(),selected);
        const auto copy=library.saveCopy("Cascade child");QVERIFY(!copy.isEmpty());
        const auto cascadeGroup=library.createGroup("Cascade");QVERIFY(!cascadeGroup.isEmpty());
        QVERIFY(library.movePresetToGroup(copy,cascadeGroup));QVERIFY(library.removeGroup(cascadeGroup,true));
        const auto remaining=library.presets();QVERIFY(std::none_of(remaining.cbegin(),remaining.cend(),[&](const QVariant &row){return row.toMap().value("id").toString()==copy;}));
        QVERIFY(library.createGroup(QString(81,'x')).isEmpty());
    }
    void brushMultiSelectionGroupsMixedItemsAndExportsSelectedTree() {
        BrushLibrary library;const auto outer=library.createGroup("Outer");QVERIFY(!outer.isEmpty());const auto one=library.saveCopy("One");QVERIFY(!one.isEmpty());QVERIFY(library.movePresetToGroup(one,outer));const auto nested=library.createGroup("Nested",outer);QVERIFY(!nested.isEmpty());const auto two=library.saveCopy("Two");QVERIFY(!two.isEmpty());QVERIFY(library.movePresetToGroup(two,nested));QVERIFY(library.select("round-pressure"));const auto three=library.saveCopy("Three");QVERIFY(!three.isEmpty());
        QVERIFY(library.selectBrushItems(QVariantList{QVariantMap{{"kind","group"},{"id",outer}},QVariantMap{{"kind","preset"},{"id",two}},QVariantMap{{"kind","preset"},{"id",three}}},three));QCOMPARE(library.selectedBrushItems().size(),3);QVERIFY(library.multiBrushSelection());QVERIFY(library.brushSelectionBlocksPainting());
        QVERIFY(library.groupSelectedItems("Combined"));const auto selected=library.selectedBrushItems();QCOMPARE(selected.size(),1);const auto combined=selected.first().toMap().value("id").toString();QVERIFY(!combined.isEmpty());QVERIFY(!library.multiBrushSelection());QVERIFY(library.brushSelectionBlocksPainting());
        QJsonObject combinedGroup;for(const auto &value:library.organization().value("groups").toList())if(value.toMap().value("id").toString()==combined)combinedGroup=QJsonObject::fromVariantMap(value.toMap());QCOMPARE(combinedGroup.value("name").toString(),QString("Combined"));QSet<QString> directItems;for(const auto &value:combinedGroup.value("items").toArray())directItems.insert(value.toObject().value("kind").toString()+":"+value.toObject().value("id").toString());QVERIFY(directItems.contains("group:"+outer));QVERIFY(directItems.contains("preset:"+three));QVERIFY(!directItems.contains("preset:"+two));
        const auto package=library.exportSelectionPackage(false);QVERIFY(!package.isEmpty());const auto handoff=QJsonDocument::fromJson(package).object();QCOMPARE(handoff.value("model").toString(),QString("ordered-package-v1"));QCOMPARE(int(handoff.value("package").toObject().value("brushes").toArray().size()),3);BrushLibrary imported;QVERIFY(imported.importPackage(package,"Imported"));QCOMPARE(imported.presets().size(),7);QSet<QString> names;for(const auto &value:imported.organization().value("groups").toList())names.insert(value.toMap().value("name").toString());QVERIFY(names.contains("Combined"));QVERIFY(names.contains("Outer"));QVERIFY(names.contains("Nested"));
        BrushLibrary flat;QVERIFY(flat.selectBrushItems(QVariantList{QVariantMap{{"kind","preset"},{"id","round-pressure"}},QVariantMap{{"kind","preset"},{"id","round-fine"}}},"round-fine"));const auto flatPackage=flat.exportSelectionPackage(false);QVERIFY(!flatPackage.isEmpty());BrushLibrary flatImported;QVERIFY(flatImported.importPackage(flatPackage,"Two"));QCOMPARE(flatImported.presets().size(),6);
    }
    void brushMultiSelectionBlocksClientPainting() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("state.ini"));QTRY_VERIFY(client.ready());QTRY_VERIFY(client.brushLibraryReady());auto *library=client.brushLibrary();QVERIFY(library->selectBrushItems(QVariantList{QVariantMap{{"kind","preset"},{"id","round-pressure"}},QVariantMap{{"kind","preset"},{"id","round-fine"}}},"round-pressure"));QVERIFY(client.brushSelectionBlocksPainting());client.newTransparentDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());QVERIFY(!client.beginStroke({{16,16},1}));QVERIFY(!client.drawing());QVERIFY(library->selectBrushItems(QVariantList{QVariantMap{{"kind","preset"},{"id","round-pressure"}}},"round-pressure"));QVERIFY(!client.brushSelectionBlocksPainting());QVERIFY(client.beginStroke({{16,16},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void brushSelectedGroupVbrRoundtrip() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("source.ini"));QTRY_VERIFY(client.ready());QTRY_VERIFY(client.brushLibraryReady());auto *library=client.brushLibrary();const auto group=library->createGroup("Export group");QVERIFY(!group.isEmpty());const auto child=library->saveCopy("Child");QVERIFY(!child.isEmpty());QVERIFY(library->movePresetToGroup(child,group));QVERIFY(library->selectBrushItems(QVariantList{QVariantMap{{"kind","group"},{"id",group}}}));const auto archive=QUrl::fromLocalFile(temp.filePath("group.vbr"));QSignalSpy exported(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.exportSelectedBrushes(archive,false));QTRY_COMPARE(exported.size(),1);QVERIFY(exported.last()[0].toBool());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        PaintCoreClient target(nullptr,temp.filePath("target.ini"));QTRY_VERIFY(target.ready());QTRY_VERIFY(target.brushLibraryReady());QSignalSpy imported(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());bool groupFound=false,childFound=false;for(const auto &value:target.brushLibrary()->organization().value("groups").toList())if(value.toMap().value("name").toString()=="Export group" && !value.toMap().value("items").toList().isEmpty())groupFound=true;for(const auto &value:target.brushLibrary()->presets())if(value.toMap().value("name").toString()=="Child")childFound=true;QVERIFY(groupFound);QVERIFY(childFound);QSignalSpy stoppedTarget(&target,&PaintCoreClient::stopped);target.shutdown();QTRY_COMPARE(stoppedTarget.size(),1);
    }
    void brushGroupsClientPreparationPixelsHistoryAndRestart() {
        QTemporaryDir temp;const auto path=temp.filePath("state.ini");BrushLibrary seed;seed.setRadius(4);seed.setPressureSize(false);seed.setPressureFlow(false);
        {QSettings prefs(path,QSettings::IniFormat);prefs.setValue("brushes/state",QJsonDocument(groupedBrushState(seed)).toJson(QJsonDocument::Compact));}
        PaintCoreClient client(nullptr,path);QSignalSpy errors(&client,&PaintCoreClient::errorChanged);QTRY_VERIFY(client.ready());QTRY_COMPARE(client.brushLibrary()->organization().value("groups").toList().size(),2);
        client.newTransparentDocument(40,40);QTRY_COMPARE(client.documentWidth(),40);QTRY_VERIFY(client.ready());
        InputSample sample;sample.position={20.5,20.5};sample.pressure=1;sample.tool=1;QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        client.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(40,40));QVERIFY(client.frame().pixelColor(20,20).alpha()>200);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);client.redo();QTRY_COMPARE(client.undoDepth(),1);
        const auto copy=client.brushLibrary()->saveCopy("restart group");QVERIFY(!copy.isEmpty());client.brushLibrary()->setRadius(6);
        QTRY_VERIFY(([&]{QSettings prefs(path,QSettings::IniFormat);return QJsonDocument::fromJson(prefs.value("brushes/state").toByteArray()).object().value("selected").toString()==copy;})());
        const auto organization=client.brushLibrary()->organization();QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QTRY_COMPARE(reopened.brushLibrary()->selectedId(),copy);QCOMPARE(reopened.brushLibrary()->organization(),organization);QVERIFY(reopened.brushLibrary()->resetPreset());QCOMPARE(reopened.brushLibrary()->radius(),4.);QCOMPARE(errors.size(),0);
        QSignalSpy stoppedAgain(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(stoppedAgain.size(),1);
    }
    void brushAllPackageExportPreservesTreeAndStableIds() {
        BrushLibrary library;QVERIFY(library.restore(QJsonDocument(groupedBrushState(library)).toJson()));
        const auto bytes=library.exportAllPackage(false);QVERIFY(!bytes.isEmpty());
        const auto handoff=QJsonDocument::fromJson(bytes).object();
        QCOMPARE(handoff.value("model").toString(),QString("ordered-package-v1"));
        const auto package=handoff.value("package").toObject();const auto brushes=package.value("brushes").toArray();
        QCOMPARE(int(brushes.size()),int(library.presets().size()));
        QSet<QString> ids;for(const auto &value:brushes){const auto id=value.toObject().value("id").toString();QVERIFY(!id.startsWith("round-"));QVERIFY(QUuid(id).isNull()==false);QVERIFY(!ids.contains(id));ids.insert(id);}
        const auto organization=handoff.value("organization").toObject();QCOMPARE(organization.value("model").toString(),QString("ordered-tree-v1"));
        QCOMPARE(int(organization.value("groups").toArray().size()),int(library.organization().value("groups").toList().size()));
        QVERIFY(!organization.value("roots").toArray().isEmpty());
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("state.ini"));QTRY_VERIFY(client.ready());QTRY_VERIFY(client.brushLibraryReady());auto *live=client.brushLibrary();const auto exportGroup=live->createGroup("Export group");QVERIFY(!exportGroup.isEmpty());const auto exportCopy=live->saveCopy("Export copy");QVERIFY(!exportCopy.isEmpty());QVERIFY(live->movePresetToGroup(exportCopy,exportGroup));bool sourceGroupHasCopy=false;for(const auto &value:live->organization().value("groups").toList())if(value.toMap().value("id").toString()==exportGroup)for(const auto &item:value.toMap().value("items").toList())if(item.toMap().value("id").toString()==exportCopy)sourceGroupHasCopy=true;QVERIFY(sourceGroupHasCopy);
        const auto archive=QUrl::fromLocalFile(temp.filePath("all.vbr"));QSignalSpy exported(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.exportAllBrushes(archive,false));QTRY_COMPARE(exported.size(),1);QVERIFY(exported.last()[0].toBool());QVERIFY(QFileInfo(archive.toLocalFile()).size()>0);
        PaintCoreClient target(nullptr,temp.filePath("target.ini"));QTRY_VERIFY(target.ready());QTRY_VERIFY(target.brushLibraryReady());QVERIFY(!target.brushLibrary()->createGroup("Export group").isEmpty());const int targetGroups=int(target.brushLibrary()->organization().value("groups").toList().size()),sourceGroups=int(live->organization().value("groups").toList().size());QSignalSpy imported(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QCOMPARE(int(target.brushLibrary()->presets().size()),4+int(live->presets().size()));QCOMPARE(int(target.brushLibrary()->organization().value("groups").toList().size()),targetGroups+1+sourceGroups);
        bool archiveGroup=false,importedGroup=false,importedCopy=false;for(const auto &value:target.brushLibrary()->organization().value("groups").toList())if(value.toMap().value("name").toString()=="all" && !value.toMap().value("items").toList().isEmpty())archiveGroup=true;else if(value.toMap().value("name").toString()=="Export group" && !value.toMap().value("items").toList().isEmpty())importedGroup=true;for(const auto &value:target.brushLibrary()->presets())if(value.toMap().value("name").toString()=="Export copy")importedCopy=true;QVERIFY(archiveGroup);QVERIFY(importedGroup);QVERIFY(importedCopy);
        QSignalSpy importedAgain(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(importedAgain.size(),1);QVERIFY(importedAgain.last()[0].toBool());QCOMPARE(int(target.brushLibrary()->presets().size()),4+2*int(live->presets().size()));QCOMPARE(int(target.brushLibrary()->organization().value("groups").toList().size()),targetGroups+2+2*sourceGroups);
    }
    void brushGroupsAppearInLibraryTreeAndSearchKeepsParent() {
        QTemporaryDir temp;const auto settings=temp.filePath("groups-ui.ini");BrushLibrary seed;const auto state=groupedBrushState(seed);{QSettings prefs(settings,QSettings::IniFormat);prefs.setValue("brushes/state",QJsonDocument(state).toJson(QJsonDocument::Compact));prefs.sync();}PaintCoreClient client(nullptr,settings);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        workspace.setActive(workspace.groupForPanel("brush"),"brush");auto *library=client.brushLibrary();QVERIFY(library);const auto organization=state.value("organization").toObject();const auto outer=organization.value("roots").toArray().first().toObject().value("id").toString();const auto selected=state.value("selected").toString();QTRY_COMPARE(library->organization().value("groups").toList().size(),2);
        QQuickItem *list=nullptr;QTRY_VERIFY((list=findVisualItem(window->contentItem(),"brushPresetList")));QTRY_COMPARE(list->property("count").toInt(),library->presets().size()+organization.value("groups").toArray().size());auto *group=findVisualItem(window->contentItem(),"brushGroup:"+outer);auto *preset=findVisualItem(window->contentItem(),"brushPreset:"+selected);QVERIFY(group && preset);QCOMPARE(group->property("height").toReal(),qreal(30));QVERIFY(group->y()<preset->y());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,group->mapToScene({group->width()/2,group->height()/2}).toPoint());QTRY_COMPARE(list->property("count").toInt(),library->presets().size());QTRY_VERIFY((group=findVisualItem(window->contentItem(),"brushGroup:"+outer)));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,group->mapToScene({group->width()/2,group->height()/2}).toPoint());QTRY_COMPARE(list->property("count").toInt(),library->presets().size()+organization.value("groups").toArray().size());
        QVERIFY(findVisualItem(window->contentItem(),"createBrushGroup"));auto *groupDialog=engine.rootObjects().first()->findChild<QObject*>("brushGroupDialog");QVERIFY(groupDialog);QVERIFY(QMetaObject::invokeMethod(groupDialog,"open"));QTRY_VERIFY(groupDialog->property("visible").toBool());QVERIFY(QMetaObject::invokeMethod(groupDialog,"close"));
        auto *search=findVisualItem(window->contentItem(),"brushSearch");QVERIFY(search);search->setProperty("text",library->selectedName());QTRY_COMPARE(list->property("count").toInt(),2);QVERIFY(findVisualItem(window->contentItem(),"brushGroup:"+outer));QVERIFY(findVisualItem(window->contentItem(),"brushPreset:"+selected));search->setProperty("text","");QTRY_COMPARE(list->property("count").toInt(),library->presets().size()+organization.value("groups").toArray().size());
        QString filler;for(int i=0;i<20;++i){filler=library->createGroup(QString("Filler %1").arg(i));QVERIFY(!filler.isEmpty());}QTRY_VERIFY(list->property("contentHeight").toReal()>list->height()+40);list->setProperty("contentY",qreal(40));QTRY_COMPARE(list->property("contentY").toReal(),qreal(40));QVERIFY(library->movePresetToGroup(library->selectedId(),filler));QTRY_VERIFY(qAbs(list->property("contentY").toReal()-40)<1);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }

    void brushLibraryShiftControlSelectionAndRootReorder() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("state.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors)warnings.append(error.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());workspace.setActive(workspace.groupForPanel("brush"),"brush");auto *library=client.brushLibrary();QVERIFY(library);
        const auto rows=library->presets();QVERIFY(rows.size()>=3);auto *first=findVisualItem(window->contentItem(),"brushPreset:"+rows[0].toMap().value("id").toString());auto *second=findVisualItem(window->contentItem(),"brushPreset:"+rows[1].toMap().value("id").toString());auto *third=findVisualItem(window->contentItem(),"brushPreset:"+rows[2].toMap().value("id").toString());QVERIFY(first && second && third);
        QTest::mouseClick(window,Qt::LeftButton,Qt::ControlModifier,first->mapToScene({20,20}).toPoint());QTest::mouseClick(window,Qt::LeftButton,Qt::ControlModifier,second->mapToScene({20,20}).toPoint());QTRY_COMPARE(library->selectedBrushItems().size(),2);QVERIFY(client.brushSelectionBlocksPainting());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,first->mapToScene({20,20}).toPoint());QTRY_COMPARE(library->selectedBrushItems().size(),1);QVERIFY(!client.brushSelectionBlocksPainting());QTest::mouseClick(window,Qt::LeftButton,Qt::ShiftModifier,third->mapToScene({20,20}).toPoint());QTRY_COMPARE(library->selectedBrushItems().size(),3);QTest::mouseClick(window,Qt::RightButton,Qt::NoModifier,first->mapToScene({20,20}).toPoint());QObject *presetMenu=nullptr;QTRY_VERIFY((presetMenu=window->findChild<QObject*>("brushPresetContextMenu")) && presetMenu->property("visible").toBool());auto *menuContent=qvariant_cast<QQuickItem*>(presetMenu->property("contentItem"));QVERIFY(menuContent);QQuickItem *exportItem=nullptr;QTRY_VERIFY((exportItem=findVisualItem(menuContent,"glassMenuItem:导出选中笔刷")));QVERIFY(exportItem->height()<30);QObject *moveToGroup=presetMenu->findChild<QObject*>("brushPresetMoveToGroup");QVERIFY(moveToGroup);QVERIFY(!moveToGroup->property("enabled").toBool());QVERIFY(QMetaObject::invokeMethod(presetMenu,"close"));
        const auto target=rows[2].toMap().value("id").toString();QVERIFY(library->movePresetToGroupAt(target,"",0));QCOMPARE(library->presets().first().toMap().value("id").toString(),target);const auto sourceId=rows[3].toMap().value("id").toString();auto *targetRow=findVisualItem(window->contentItem(),"brushPreset:"+target),*sourceRow=findVisualItem(window->contentItem(),"brushPreset:"+sourceId);QVERIFY(targetRow && sourceRow);const auto at=targetRow->mapToScene({targetRow->width()*.5,3}).toPoint();QMimeData mime;mime.setData("application/x-drawverse-brush",("preset:"+sourceId).toUtf8());QDragEnterEvent enter(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&enter);QDragMoveEvent dragMove(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&dragMove);QDropEvent drop(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&drop);QVERIFY(drop.isAccepted());QTRY_COMPARE(library->presets().first().toMap().value("id").toString(),sourceId);auto *rootDrop=findVisualItem(window->contentItem(),"brushRootDrop");QVERIFY(rootDrop);const auto rootAt=rootDrop->mapToScene({rootDrop->width()*.5,rootDrop->height()*.5}).toPoint();QDragEnterEvent rootEnter(rootAt,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&rootEnter);QDragMoveEvent rootMove(rootAt,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&rootMove);QDropEvent rootEvent(rootAt,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&rootEvent);QVERIFY(rootEvent.isAccepted());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void expandedGroupLowerDropInsertsFirstForBrushesAndLayers() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("state.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors)warnings.append(error.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        const auto dropAt=[&](QQuickItem *target,const QByteArray &kind,const QByteArray &payload){QMimeData mime;mime.setData(kind,payload);const auto at=target->mapToScene({target->width()/2,target->height()-2}).toPoint();QDragEnterEvent enter(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&enter);QDragMoveEvent move(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&move);QDropEvent drop(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&drop);return drop.isAccepted();};
        auto *library=client.brushLibrary();const auto brushGroup=library->createGroup("Brush group");QVERIFY(!brushGroup.isEmpty());const auto child=library->saveCopy("Child");QVERIFY(!child.isEmpty());QVERIFY(library->movePresetToGroup(child,brushGroup));QVERIFY(library->select("round-pressure"));const auto rootBrush=library->saveCopy("Root source");QVERIFY(!rootBrush.isEmpty());workspace.setActive(workspace.groupForPanel("brush"),"brush");auto *brushGroupRow=findVisualItem(window->contentItem(),"brushGroup:"+brushGroup);auto *brushSource=findVisualItem(window->contentItem(),"brushPreset:"+rootBrush);QVERIFY(brushGroupRow && brushSource);QVERIFY(dropAt(brushGroupRow,"application/x-drawverse-brush",("preset:"+rootBrush).toUtf8()));QJsonArray brushItems;for(const auto &value:library->organization().value("groups").toList())if(value.toMap().value("id").toString()==brushGroup)brushItems=QJsonArray::fromVariantList(value.toMap().value("items").toList());QVERIFY(!brushItems.isEmpty());QCOMPARE(brushItems.first().toObject().value("id").toString(),rootBrush);
        client.newTransparentDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());quint64 firstLayer=0;for(const auto &value:client.layers()){const auto layer=value.toMap();if(!layer.value("mask").toBool()){firstLayer=layer.value("id").toULongLong();break;}}QVERIFY(firstLayer);QVERIFY(client.groupLayer(firstLayer,"Layer group"));QTRY_VERIFY(!client.layerEditBusy());quint64 layerGroup=0;for(const auto &value:client.layers())if(value.toMap().value("group").toBool())layerGroup=value.toMap().value("id").toULongLong();QVERIFY(layerGroup);client.addDefaultLayer();QTRY_VERIFY(!client.layerEditBusy());const auto layerSource=client.activeLayer();QVERIFY(layerSource && layerSource!=layerGroup);workspace.setActive(workspace.groupForPanel("layers"),"layers");auto *layerGroupRow=findVisualItem(window->contentItem(),"layerRow:"+QString::number(layerGroup)),*layerSourceRow=findVisualItem(window->contentItem(),"layerRow:"+QString::number(layerSource));QVERIFY(layerGroupRow && layerSourceRow);QVERIFY(dropAt(layerGroupRow,"application/x-drawverse-layer",client.layerDragPayload(layerSource).toUtf8()));QTRY_VERIFY(!client.layerEditBusy());int sourceIndex=-1,firstChildIndex=std::numeric_limits<int>::max();const auto layers=client.layers();for(int i=0;i<layers.size();++i){const auto layer=layers[i].toMap();if(layer.value("id").toULongLong()==layerSource)sourceIndex=i;if(layer.value("parent").toULongLong()==layerGroup)firstChildIndex=std::min(firstChildIndex,i);}QVERIFY(sourceIndex>=0);QCOMPARE(sourceIndex,firstChildIndex);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void brushAndLayerListsWheelScrollWhileDragging() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("state.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors)warnings.append(error.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        for(int i=0;i<18;++i)QVERIFY(!client.brushLibrary()->createGroup(QString("Scroll group %1").arg(i)).isEmpty());
        for(int i=0;i<18;++i){client.addDefaultLayer();QTRY_VERIFY(!client.layerEditBusy());}
        workspace.setActive(workspace.groupForPanel("brush"),"brush");workspace.setActive(workspace.groupForPanel("layers"),"layers");
        const auto wheel=[&](QQuickItem *item){const auto local=item->mapToScene({item->width()/2,item->height()/2}).toPoint();QWheelEvent event(local,window->mapToGlobal(local),{},QPoint(0,120),Qt::NoButton,Qt::NoModifier,Qt::NoScrollPhase,false);QCoreApplication::sendEvent(window,&event);};
        auto *brushPanel=findVisualItem(window->contentItem(),"brushLibraryPanel");auto *brushList=findVisualItem(window->contentItem(),"brushPresetList");QVERIFY(brushPanel && brushList);QTRY_VERIFY(brushList->property("contentHeight").toReal()>brushList->height()+80);
        brushPanel->setProperty("draggingBrush",true);brushPanel->setProperty("dragViewportY",qreal(10));brushList->setProperty("contentY",qreal(60));QTRY_VERIFY(brushList->property("contentY").toReal()<60);brushPanel->setProperty("dragViewportY",qreal(-1));brushList->setProperty("contentY",qreal(100));wheel(brushList);QTRY_VERIFY(brushList->property("contentY").toReal()<100);brushPanel->setProperty("draggingBrush",false);
        auto *layerPanel=findVisualItem(window->contentItem(),"layersPanel");auto *layerList=findVisualItem(window->contentItem(),"layerList");QVERIFY(layerPanel && layerList);QTRY_VERIFY(layerList->property("contentHeight").toReal()>layerList->height()+80);
        layerPanel->setProperty("draggingLayer",true);layerPanel->setProperty("dragViewportY",qreal(10));layerList->setProperty("contentY",qreal(60));QTRY_VERIFY(layerList->property("contentY").toReal()<60);layerPanel->setProperty("dragViewportY",qreal(-1));layerList->setProperty("contentY",qreal(100));wheel(layerList);QTRY_VERIFY(layerList->property("contentY").toReal()<100);layerPanel->setProperty("draggingLayer",false);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }

    void allAutomaticBrushPreviewsStayWhiteWithExactAlphaAndColorIndependentCache() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("preview.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        const auto generation=client.generation(),revision=client.revision();const auto history=client.undoDepth();const bool modified=client.modified();
        BrushLibrary factory;
        QImage mask(16,16,QImage::Format_RGBA8888);mask.fill(Qt::white);for(int y=6;y<10;++y)for(int x=6;x<10;++x)mask.setPixelColor(x,y,Qt::transparent);const auto maskPath=temp.filePath("mask.png");QVERIFY(mask.save(maskPath));
        for(int variant=0;variant<5;++variant) {
            QVERIFY(library->restore(factory.snapshot()));library->setRadius(5);library->setOpacity(.35);library->setPressureSize(false);library->setPressureFlow(false);client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);
            if(variant==1){library->setColorDynamicsEnabled(true);library->setForegroundBackgroundJitter(1);library->setHueJitter(1);library->setSaturationJitter(1);library->setValueJitter(1);}
            if(variant==2){auto dual=library->dualBrush();dual["enabled"]=true;dual["diameter"]=24.;dual["color"]=QVariantMap{{"model","separated-stroke-v1"},{"enabled",true},{"useBackground",true},{"overflow",true},{"order","second-behind"},{"primaryBlend","normal"},{"secondaryBlend","normal"}};QVERIFY(library->setDualBrush(dual));}
            if(variant==3){auto wet=library->wetMix();wet["enabled"]=true;wet["autoLoad"]=true;QVERIFY(library->setWetMix(wet));}
            if(variant==4){QSignalSpy imported(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushTip(QUrl::fromLocalFile(maskPath),1));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QVERIFY(!library->maskResource().isEmpty());}
            library->requestPreview(library->selectedId());QTRY_VERIFY2_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),qPrintable(QString::number(variant)+":"+library->previewMessage()),10000);const auto preview=library->selectedPreview();const auto image=QImage::fromData(QByteArray::fromBase64(preview.mid(preview.indexOf(',')+1).toLatin1()));QCOMPARE(image.size(),QSize(240,60));int visible=0,partial=0,transparent=0;
            for(int y=0;y<image.height();++y)for(int x=0;x<image.width();++x){const auto color=image.pixelColor(x,y);if(color.alpha()>0){++visible;partial+=color.alpha()<255;QCOMPARE(color.red(),255);QCOMPARE(color.green(),255);QCOMPARE(color.blue(),255);}else ++transparent;}
            QVERIFY(visible>0);QVERIFY(partial>0);QVERIFY(transparent>0);
            const auto catalog=library->catalogSnapshot();QSignalSpy requests(library,&BrushLibrary::previewRequested);client.setBrushColor(Qt::green);client.setSecondaryBrushColor(Qt::magenta);client.swapBrushColors();library->requestPreview(library->selectedId());QCOMPARE(requests.size(),0);QCOMPARE(library->selectedPreview(),preview);QCOMPARE(library->catalogSnapshot(),catalog);client.setBrushWriteBlend(6);QCOMPARE(library->selectedPreview(),preview);
            QCOMPARE(client.generation(),generation);QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),history);QCOMPARE(client.modified(),modified);
        }
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }

    void highSmoothingMouseAndTabletReleaseNeverConnectsToRawPointer() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("settings.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);auto *library=client.brushLibrary();library->setRadius(2);library->setPressureSize(false);library->setPressureFlow(false);auto smoothing=library->smoothing();smoothing["enabled"]=true;smoothing["positionMs"]=250.;smoothing["pressureMs"]=250.;smoothing["stabilizerPx"]=128.;smoothing["speedResponse"]=0.;QVERIFY(library->setSmoothing(smoothing));client.setBrushColor(Qt::red);
        QPointingDevice pen("Smoothing pen",930,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        for(const bool tablet:{false,true}) {
            client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready() && !client.layerEditBusy());QTRY_VERIFY(canvas->documentRect().width()>0);window->requestActivate();QTRY_VERIFY(window->isActive());canvas->forceActiveFocus();
            const auto local=[&](QPointF point){return canvas->mapToScene(canvas->documentRect().topLeft()+point*canvas->zoom());};
            const auto first=local({16.5,16.5}),last=local({48.5,48.5});
            if(!tablet){QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,first.toPoint());QTest::mouseMove(window,last.toPoint(),16);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,last.toPoint(),16);}
            else {const auto send=[&](QEvent::Type type,QPointF at,qreal pressure){QTabletEvent event(type,&pen,at,window->mapToGlobal(at.toPoint()),pressure,0,0,0,0,0,Qt::NoModifier,type==QEvent::TabletMove?Qt::NoButton:Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&event);};send(QEvent::TabletPress,first,.2);QTest::qWait(16);send(QEvent::TabletMove,last,1.);send(QEvent::TabletRelease,last,0.);}
            QTRY_VERIFY(!client.drawing());QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());canvas->setClient(nullptr);client.requestViewport(0,{0,0,64,64},{64,64});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(64,64));const auto image=client.frame();QVERIFY(image.pixelColor(16,16).alpha()>0);for(int y=24;y<64;++y)for(int x=24;x<64;++x)QCOMPARE(image.pixelColor(x,y).alpha(),0);canvas->setClient(&client);
            client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        }
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }

    void activeAndRecentColorsPersistAcrossRestartAndStayOutOfPresets() {
        QTemporaryDir temp;const auto path=temp.filePath("colors.ini");
        const QColor foreground("#804080c0"),background("#c020b060");QVariantList recent;
        {
            PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());
            client.setBrushColor(foreground);client.setSecondaryBrushColor(background);client.swapBrushColors();
            QCOMPARE(client.brushColor(),background);QCOMPARE(client.secondaryBrushColor(),foreground);
            QVERIFY(client.brushLibrary()->useColor("#aa123456"));QVERIFY(client.brushLibrary()->useColor("#bb654321"));recent=client.recentColors();
            const auto catalog=client.brushLibrary()->catalogSnapshot();QVERIFY(!catalog.contains("colorState"));QVERIFY(!catalog.contains("recentColors"));
            const auto package=client.brushLibrary()->exportPackage();QVERIFY(!package.contains("colorState"));QVERIFY(!package.contains("recentColors"));
            QVERIFY(client.brushLibrary()->resetPreset());QCOMPARE(client.brushColor(),background);QCOMPARE(client.recentColors(),recent);
            // Close without waiting for a save debounce; normal shutdown must flush the preference queue.
            QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        }
        PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());
        QCOMPARE(reopened.brushColor(),background);QCOMPARE(reopened.secondaryBrushColor(),foreground);QCOMPARE(reopened.recentColors(),recent);
        auto *library=reopened.brushLibrary();const auto valid=library->snapshot();
        for(const auto &invalid:QList<QJsonValue>{QJsonValue::Null,QJsonObject{{"foreground",12},{"background","red"}},QJsonObject{{"foreground","bad color"},{"background","red"}},QJsonObject{{"foreground","red"}}}){
            auto state=QJsonDocument::fromJson(valid).object();state["colorState"]=invalid;QVERIFY(!library->restore(QJsonDocument(state).toJson()));QCOMPARE(library->snapshot(),valid);
        }
        auto legacy=QJsonDocument::fromJson(valid).object();legacy.remove("colorState");BrushLibrary old;QVERIFY(old.restore(QJsonDocument(legacy).toJson()));QCOMPARE(old.foregroundColor(),QColor("#2ea99d"));QCOMPARE(old.backgroundColor(),QColor(Qt::white));QCOMPARE(old.recentColors(),recent);
        QSignalSpy stopped(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void canvasZoomKeysAndGlobalColorsWorkAcrossDocumentsAndFloatingWindows() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("settings.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors)warnings.append(error.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());window->requestActivate();QTRY_VERIFY(window->isActive());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->forceActiveFocus();
        const auto generation=client.generation(),revision=client.revision();const auto history=client.undoDepth();const auto changed=client.modified();const auto center=canvas->visibleDocumentRect().center();const auto zoom=canvas->zoom();
        QTest::keyClick(window,Qt::Key_Equal,Qt::ControlModifier);QTRY_VERIFY(canvas->zoom()>zoom);QVERIFY(QLineF(center,canvas->visibleDocumentRect().center()).length()<1e-5);
        QTest::keyClick(window,Qt::Key_Minus,Qt::ControlModifier);QTRY_VERIFY(qAbs(canvas->zoom()-zoom)<1e-8);
        QTest::keyClick(window,Qt::Key_Plus,Qt::ControlModifier|Qt::ShiftModifier);QTRY_VERIFY(canvas->zoom()>zoom);QTest::keyClick(window,Qt::Key_Minus,Qt::ControlModifier);QTRY_VERIFY(qAbs(canvas->zoom()-zoom)<1e-8);
        // A text editor consumes its own keys, even with the document still visible.
        QQmlComponent component(&engine);component.setData("import QtQuick; TextInput {width:150;height:30;text: 'edit'}",QUrl());QScopedPointer<QObject> editorObject(component.create());auto *editor=qobject_cast<QQuickItem*>(editorObject.data());QVERIFY(editor);editor->setParentItem(window->contentItem());editor->forceActiveFocus();QTRY_VERIFY(client.textInputFocused());QTest::keyClick(window,Qt::Key_Equal,Qt::ControlModifier);QCOMPARE(canvas->zoom(),zoom);canvas->forceActiveFocus();
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);const auto id=documents->newDocument(64,64,true);QVERIFY(!id.isEmpty());auto *other=documents->client(id);QTRY_VERIFY(other && other->ready());
        other->setBrushColor(QColor("#80336699"));other->setSecondaryBrushColor(QColor("#c0996633"));QCOMPARE(client.brushColor(),other->brushColor());QCOMPARE(client.secondaryBrushColor(),other->secondaryBrushColor());QVERIFY(other->brushLibrary()->useColor("#cc123456"));QCOMPARE(client.recentColors(),other->recentColors());
        documents->floatDocument(id);const auto findFloating=[&]()->CanvasItem*{for(auto *candidate:QGuiApplication::allWindows())if(auto *quick=qobject_cast<QQuickWindow*>(candidate))if(auto *item=findVisualItem(quick->contentItem(),"documentCanvas:"+id))return qobject_cast<CanvasItem*>(item);return nullptr;};CanvasItem *floating=nullptr;QTRY_VERIFY((floating=findFloating()));auto *floatingWindow=floating->window();QVERIFY(floatingWindow && floatingWindow!=window);floatingWindow->requestActivate();QTRY_VERIFY(floatingWindow->isActive());floating->forceActiveFocus();const auto floatingZoom=floating->zoom();
        QTest::keyClick(floatingWindow,Qt::Key_Plus,Qt::ControlModifier|Qt::KeypadModifier);QTRY_VERIFY(floating->zoom()>floatingZoom);QCOMPARE(canvas->zoom(),zoom);QTest::keyClick(floatingWindow,Qt::Key_Minus,Qt::ControlModifier|Qt::KeypadModifier);QTRY_VERIFY(qAbs(floating->zoom()-floatingZoom)<1e-8);
        QCOMPARE(client.generation(),generation);QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),history);QCOMPARE(client.modified(),changed);QCOMPARE(warnings,QStringList());
        QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }


    void compositeCursorPreferenceIsOptionalStrictAndOutsidePresets() {
        BrushLibrary library;QVERIFY(!library.cursorComposite());const auto definition=library.catalogSnapshot();const auto package=library.exportPackage();QSignalSpy settings(&library,&BrushLibrary::settingsChanged);QSignalSpy preview(&library,&BrushLibrary::previewRequested);QSignalSpy cursor(&library,&BrushLibrary::cursorChanged);library.setCursorComposite(true);QCOMPARE(cursor.size(),1);QCOMPARE(settings.size(),0);QCOMPARE(preview.size(),0);QCOMPARE(library.catalogSnapshot(),definition);QCOMPARE(QJsonDocument::fromJson(library.exportPackage()).object().value("brushes"),QJsonDocument::fromJson(package).object().value("brushes"));QVERIFY(library.resetPreset());QVERIFY(library.cursorComposite());
        BrushLibrary restored;QVERIFY(restored.restore(library.snapshot()));QVERIFY(restored.cursorComposite());auto old=QJsonDocument::fromJson(library.snapshot()).object();auto data=old.value("cursor").toObject();data.remove("composite");old["cursor"]=data;QVERIFY(restored.restore(QJsonDocument(old).toJson()));QVERIFY(!restored.cursorComposite());const auto before=restored.snapshot();for(const auto &bad:QJsonArray{1,"true",QJsonValue::Null}){data["composite"]=bad;old["cursor"]=data;QVERIFY(!restored.restore(QJsonDocument(old).toJson()));QCOMPARE(restored.snapshot(),before);}
    }
    void compositeCursorOptionalPressureImagesAndCapturedStroke() {
        QSKIP("双色绘制已删除，复合光标的双色溢出路径不再验收");
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);QVERIFY(QTest::qWaitForWindowExposed(window));QTRY_VERIFY(canvas->width()>100 && canvas->height()>100);canvas->actualSize();auto *library=client.brushLibrary();QVERIFY(!library->createOutlinePreset(true).isEmpty());
        for(const auto &v:workspace.leftGroups()+workspace.rightGroups()){const auto g=v.toMap();if(g.value("panels").toStringList().contains("brush-settings"))workspace.setActive(g.value("id").toString(),"brush-settings");}QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:7")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());auto *toggle=findVisualItem(window->contentItem(),"brushCursorComposite");QVERIFY(toggle);QTRY_VERIFY(toggle->isVisible() && toggle->width()>20);QTest::qWait(50);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({8,toggle->height()/2}).toPoint());QTRY_VERIFY(library->cursorComposite());
        const QPointF start=canvas->documentRect().center();window->requestActivate();QTRY_VERIFY(window->isActive());QTest::mouseMove(window,canvas->mapToScene(start).toPoint());QTest::qWait(50);const auto hover=[&]{QHoverEvent event(QEvent::HoverMove,start,canvas->mapToGlobal(start),start);QCoreApplication::sendEvent(canvas,&event);};hover();const bool initialVisible=canvas->brushCursorVisible();if(!initialVisible)QTest::keyClick(window,Qt::Key_CapsLock);hover();QTRY_VERIFY(canvas->secondaryCursorVisible());QTRY_COMPARE(canvas->brushCursorRect().width(),24.);QTRY_COMPARE(canvas->secondaryCursorRect().width(),32.);auto *secondary=findVisualItem(window->contentItem(),"secondaryBrushCursorOutline");QVERIFY(secondary);QTRY_VERIFY(secondary->isVisible());
        auto deviceDual=library->dualBrush();deviceDual["roundness"]=.5;deviceDual["tipDynamics"]=QVariantMap{{"angleControl","pen-rotation"},{"tiltRoundness",false},{"minimumRoundness",.01}};QVERIFY(library->setDualBrush(deviceDual));
        auto *pen=new QPointingDevice("Composite pen",899,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure|QInputDevice::Capability::XTilt|QInputDevice::Capability::Rotation,1,2);pen->setParent(qApp);
        QTabletEvent press(QEvent::TabletPress,pen,start,canvas->mapToGlobal(start),.5,0,0,0,90,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(canvas,&press);QVERIFY(client.drawing());QTRY_COMPARE(canvas->brushCursorRect().width(),12.);QTRY_COMPARE(canvas->secondaryCursorRect().width(),20.);QTRY_VERIFY(std::abs(canvas->secondaryCursorAngle()-90)<.001);QCOMPARE(canvas->secondaryCursorRoundness(),.5);QVERIFY(!library->createOutlinePreset(false).isEmpty());QCOMPARE(canvas->secondaryCursorRect().width(),20.);QTabletEvent release(QEvent::TabletRelease,pen,start,canvas->mapToGlobal(start),0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(canvas,&release);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!client.drawing());QTRY_COMPARE(canvas->secondaryCursorRect().width(),32.);
        QTabletEvent pressFollow(QEvent::TabletPress,pen,start,canvas->mapToGlobal(start),.5,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(canvas,&pressFollow);QTRY_COMPARE(canvas->secondaryCursorRect().width(),16.);library->setCursorSimple(true);QTRY_COMPARE(canvas->secondaryCursorRect().width(),32.);QCoreApplication::sendEvent(canvas,&release);QTRY_VERIFY(!client.layerEditBusy());library->setCursorSimple(false);QTRY_VERIFY(!client.layerEditBusy());
        auto dual=library->dualBrush();QVERIFY(library->setDualBrush(dual));QTRY_VERIFY(canvas->secondaryCursorVisible());
        library->setCursorComposite(false);QImage primary(16,16,QImage::Format_RGBA8888);primary.fill(Qt::transparent);QImage child=primary;for(int y=2;y<14;++y)for(int x=5;x<11;++x)primary.setPixelColor(x,y,Qt::white);for(int y=5;y<11;++y)for(int x=2;x<14;++x)child.setPixelColor(x,y,Qt::white);const auto first=temp.filePath("main.png"),second=temp.filePath("second.png");QVERIFY(primary.save(first));QVERIFY(child.save(second));QVERIFY(client.importBrushTip(QUrl::fromLocalFile(first),1));QTRY_VERIFY(!client.brushFileBusy());QVERIFY(client.importDualBrushTip(QUrl::fromLocalFile(second),1));QTRY_VERIFY(!client.brushFileBusy());QVERIFY(!client.cursorSecondaryOutlineReady());library->setCursorComposite(true);QTRY_VERIFY(client.cursorOutlineReady() && client.cursorSecondaryOutlineReady());QVERIFY(client.cursorHasImage() && client.cursorSecondaryHasImage());const auto primaryPath=client.cursorOutlinePath(),childPath=client.cursorSecondaryOutlinePath();QVERIFY(!primaryPath.isEmpty() && !childPath.isEmpty() && primaryPath!=childPath);QTRY_VERIFY(secondary->property("imageOutline").toBool());
        library->setRadius(32);hover();QTRY_COMPARE(canvas->secondaryCursorRect().width(),72.);QTest::qWait(80);const auto screenshot=qEnvironmentVariable("DRAWVERSE_COMPOSITE_CURSOR");if(!screenshot.isEmpty()){const auto dpr=window->devicePixelRatio();const auto center=canvas->mapToScene(start)*dpr;QVERIFY(window->grabWindow().copy(QRect(center.toPoint()-QPoint(qRound(60*dpr),qRound(60*dpr)),QSize(qRound(120*dpr),qRound(120*dpr)))).save(screenshot));}
        QCoreApplication::sendEvent(canvas,&press);QVERIFY(client.drawing());library->useProceduralTip();library->useProceduralDual();QVERIFY(client.cursorHasImage() && client.cursorSecondaryHasImage());QCOMPARE(client.cursorOutlinePath(),primaryPath);QCOMPARE(client.cursorSecondaryOutlinePath(),childPath);QCoreApplication::sendEvent(canvas,&release);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!client.drawing());QVERIFY(!client.cursorHasImage() && !client.cursorSecondaryHasImage());library->setCursorComposite(false);QVERIFY(!canvas->secondaryCursorVisible());library->setCursorComposite(true);client.setEraser(true);QVERIFY(!canvas->secondaryCursorVisible());client.setEraser(false);hover();QTRY_VERIFY(canvas->secondaryCursorVisible());library->setCursorHoverOnly(true);QCoreApplication::sendEvent(canvas,&press);QVERIFY(!canvas->secondaryCursorVisible());QCoreApplication::sendEvent(canvas,&release);QTRY_VERIFY(!client.layerEditBusy());library->setCursorHoverOnly(false);hover();QTRY_VERIFY(canvas->secondaryCursorVisible());
        const auto saved=[&]{QSettings prefs(path,QSettings::IniFormat);return QJsonDocument::fromJson(prefs.value("brushes/state").toByteArray()).object().value("cursor").toObject().value("composite").toBool();};QTRY_VERIFY(saved());if(!initialVisible)QTest::keyClick(window,Qt::Key_CapsLock);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QVERIFY(reopened.brushLibrary()->cursorComposite());QCOMPARE(warnings,QStringList());
    }
    void outlineTemplatesAreAtomicIndependentResettableAndBounded() {
        QSKIP("双色绘制已删除，自动描边预设仅保留兼容代码");
        BrushLibrary library;library.setRadius(37);library.setHardness(.3);library.setWriteBlend(3);library.setCursorCenter(true);const auto original=library.selectedId();const auto before=library.snapshot();
        QSignalSpy changes(&library,&BrushLibrary::settingsChanged);const auto fixed=library.createOutlinePreset(true);QVERIFY(!fixed.isEmpty());QCOMPARE(changes.size(),1);QCOMPARE(library.radius(),12.);QCOMPARE(library.opacity(),1.);QCOMPARE(library.spacing(),.1);QVERIFY(library.pressureSize());QVERIFY(!library.pressureFlow());QCOMPARE(library.hardness(),1.);QCOMPARE(library.writeBlend(),3);QVERIFY(library.cursorCenter());
        const auto dual=library.dualBrush();QCOMPARE(dual.value("model").toString(),QString("paired-dab-v1"));QVERIFY(dual.value("spacing").isNull());QCOMPARE(dual.value("sizing").toMap().value("widthRule").toString(),QString("fixed-width"));QCOMPARE(dual.value("sizing").toMap().value("diameterOffset").toDouble(),8.);const auto color=dual.value("color").toMap();QVERIFY(color.value("useBackground").toBool());QVERIFY(color.value("overflow").toBool());QCOMPARE(color.value("order").toString(),QString("second-behind"));
        QVERIFY(library.select(original));const auto row=[](const QByteArray &bytes,const QString &id){for(const auto &r:QJsonDocument::fromJson(bytes).object().value("presets").toArray())if(r.toObject().value("id").toString()==id)return r.toObject();return QJsonObject{};};QCOMPARE(row(library.snapshot(),original),row(before,original));
        const auto follow=library.createOutlinePreset(false);QVERIFY(!follow.isEmpty() && follow!=fixed);QCOMPARE(library.dualBrush().value("sizing").toMap().value("widthRule").toString(),QString("follow-pressure"));library.setRadius(4);auto edited=library.dualBrush();auto sizing=edited.value("sizing").toMap();sizing["diameterOffset"]=-3.;edited["sizing"]=sizing;QVERIFY(library.setDualBrush(edited));
        BrushLibrary reopened;QVERIFY(reopened.restore(library.snapshot()));QVERIFY(reopened.resetPreset());QCOMPARE(reopened.radius(),12.);QCOMPARE(reopened.dualBrush().value("sizing").toMap().value("diameterOffset").toDouble(),8.);QVERIFY(reopened.select(fixed));QCOMPARE(reopened.dualBrush(),dual);
        BrushLibrary imported;QVERIFY(imported.importPackage(library.exportPackage()));imported.setRadius(6);QVERIFY(imported.resetPreset());QCOMPARE(imported.radius(),4.);
        auto full=QJsonDocument::fromJson(before).object();auto rows=full.value("presets").toArray();const auto base=rows[0].toObject();for(int i=rows.size();i<1024;++i){auto r=base;r["id"]=QString("dba00000-0000-4000-8000-%1").arg(i+10,12,16,QChar('0'));rows.append(r);}full["presets"]=rows;full.remove("resetState");BrushLibrary bounded;QVERIFY(bounded.restore(QJsonDocument(full).toJson()));const auto unchanged=bounded.snapshot();QVERIFY(bounded.createOutlinePreset(true).isEmpty());QCOMPARE(bounded.snapshot(),unchanged);
    }
    void outlinePresetsWithoutCreationButtonsPreservePixelsExchangeAndHistory() {
        QSKIP("双色绘制已删除，自动描边预设仅保留兼容代码");
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto g=value.toMap();if(g.value("panels").toStringList().contains("brush-settings"))workspace.setActive(g.value("id").toString(),"brush-settings");}QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:4")));const auto click=[&](QQuickItem *item){QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,item->mapToScene({item->width()/2,item->height()/2}).toPoint());};click(category);auto *page=findVisualItem(window->contentItem(),"settingsDualPage");QVERIFY(page);page->setProperty("currentIndex",4);QVERIFY(QMetaObject::invokeMethod(page,"activated",Q_ARG(int,4)));QVERIFY(!findVisualItem(window->contentItem(),"createOutlineFixed"));QVERIFY(!findVisualItem(window->contentItem(),"createOutlineFollow"));const auto fixedId=library->createOutlinePreset(true);QVERIFY(!fixedId.isEmpty());library->requestPreview(fixedId);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);const auto point=[](double x,double y,float pressure){InputSample s;s.position={x,y};s.pressure=pressure;s.tool=1;return s;};const auto render=[](PaintCoreClient &c){c.requestViewport(0,{0,0,64,64},{64,64});return c.frameRevision()>=c.revision() && c.frame().size()==QSize(64,64);};
        QVERIFY(client.beginStroke(point(16.5,32.5,.5f)));QVERIFY(!library->createOutlinePreset(false).isEmpty());QVERIFY(library->selectedId()!=fixedId);client.strokeTo(point(48.5,32.5,.5f));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(render(client));const auto fixedPixels=client.frame();QCOMPARE(fixedPixels.pixelColor(32,32),QColor(Qt::red));QCOMPARE(fixedPixels.pixelColor(32,41),QColor(Qt::blue));QCOMPARE(fixedPixels.pixelColor(32,43).alpha(),0);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(render(client));QCOMPARE(client.frame().pixelColor(32,32).alpha(),0);client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(render(client));QCOMPARE(client.frame(),fixedPixels);
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(client.ready());QVERIFY(client.beginStroke(point(16.5,32.5,.5f)));client.strokeTo(point(48.5,32.5,.5f));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(render(client));QCOMPARE(client.frame().pixelColor(32,39),QColor(Qt::blue));QCOMPARE(client.frame().pixelColor(32,41).alpha(),0);
        QVERIFY(library->select(fixedId));library->setOpacity(.4);client.newTransparentDocument(64,64);QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(client.ready());const auto draw=[&](PaintCoreClient &c){if(!c.beginStroke(point(16.5,32.5,1)))return false;for(const auto &p:{QPointF(48.5,32.5),QPointF(32.5,16.5),QPointF(32.5,48.5),QPointF(16.5,32.5),QPointF(48.5,32.5)})c.strokeTo(point(p.x(),p.y(),1));c.endStroke();return true;};QVERIFY(draw(client));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(render(client));const auto crossed=client.frame();const auto center=crossed.pixelColor(32,32);QVERIFY(center.red()>250 && center.blue()<2);QVERIFY(center.alpha()>=101 && center.alpha()<=103);for(int y=0;y<64;++y)for(int x=0;x<64;++x)QVERIFY(crossed.pixelColor(x,y).alpha()<=103);
        const auto archive=QUrl::fromLocalFile(temp.filePath("outline.vbr"));QSignalSpy exported(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.exportBrush(archive));QTRY_COMPARE(exported.size(),1);QVERIFY(exported.last()[0].toBool());PaintCoreClient target(nullptr,temp.filePath("target/storage.ini"));QTRY_VERIFY(target.ready());QSignalSpy imported(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());target.newTransparentDocument(64,64);QTRY_COMPARE(target.documentWidth(),64);QTRY_VERIFY(target.ready());target.setBrushColor(Qt::red);target.setSecondaryBrushColor(Qt::blue);QVERIFY(draw(target));QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(render(target));QCOMPARE(target.frame(),crossed);
        const auto saved=[&]{QSettings prefs(path,QSettings::IniFormat);return QJsonDocument::fromJson(prefs.value("brushes/state").toByteArray()).object().value("selected").toString()==fixedId;};QTRY_VERIFY(saved());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->selectedId(),fixedId);QVERIFY(reopened.brushLibrary()->resetPreset());QCOMPARE(reopened.brushLibrary()->opacity(),1.);QCOMPARE(reopened.brushLibrary()->dualBrush().value("sizing").toMap().value("widthRule").toString(),QString("fixed-width"));QCOMPARE(warnings,QStringList());
    }
    void dualColorClosedValuesHighestVersionResetAndPreviewTokens() {
        QSKIP("双色绘制已删除");
        BrushLibrary library;auto data=library.dualBrush();QVariantMap color{{"model","separated-stroke-v1"},{"enabled",false},{"useBackground",true},{"overflow",true},{"order","second-in-front"},{"primaryBlend","multiply"},{"secondaryBlend","screen"}};data["color"]=color;QVERIFY(library.setDualBrush(data));data=library.dualBrush();
        const auto copy=library.saveCopy("双色初始基准");QVERIFY(!copy.isEmpty());auto edit=data;auto nested=color;nested["enabled"]=true;edit["color"]=nested;QVERIFY(library.setDualBrush(edit));QVERIFY(library.resetPreset());QCOMPARE(library.dualBrush(),data);
        auto interleaved=data;auto stamp=color;stamp["model"]="interleaved-stamp-v1";stamp["enabled"]=true;interleaved["model"]="paired-dab-v1";interleaved["spacing"]=QVariant{};interleaved["color"]=stamp;QVERIFY(library.setDualBrush(interleaved));QCOMPARE(QJsonDocument::fromJson(library.snapshot()).object().value("version").toInt(),22);
        BrushLibrary stampImported;QVERIFY(stampImported.importPackage(library.exportPackage()));QCOMPARE(QJsonObject::fromVariantMap(stampImported.dualBrush()),QJsonObject::fromVariantMap(interleaved));
        auto invalidPath=interleaved;invalidPath["model"]="path-coverage-v1";invalidPath["spacing"]=.25;QVERIFY(!library.setDualBrush(invalidPath));QVERIFY(library.setDualBrush(data));
        QVERIFY(library.select("round-wide"));auto other=library.dualBrush();other["sizing"]=QVariantMap{{"model","relative-size-v1"},{"mode","relative"},{"diameterOffset",8.},{"widthRule","fixed-width"}};QVERIFY(library.setDualBrush(other));QCOMPARE(QJsonDocument::fromJson(library.snapshot()).object().value("version").toInt(),16);QVERIFY(library.select(copy));
        const auto good=library.snapshot();for(const auto &value:QVariantList{QString("pass-through"),17,QVariant{}}){auto bad=data;auto invalid=color;invalid["primaryBlend"]=value;bad["color"]=invalid;QVERIFY(!library.setDualBrush(bad));}QCOMPARE(library.snapshot(),good);auto old=QJsonDocument::fromJson(good).object();old["version"]=15;QVERIFY(!library.restore(QJsonDocument(old).toJson()));QCOMPARE(library.snapshot(),good);
        BrushLibrary restored;QVERIFY(restored.restore(good));QCOMPARE(restored.dualBrush(),data);BrushLibrary imported;QVERIFY(imported.importPackage(library.exportPackage()));QCOMPARE(imported.dualBrush(),data);
        data["enabled"]=true;nested=color;nested["enabled"]=true;data["color"]=nested;QVERIFY(library.setDualBrush(data));QSignalSpy requests(&library,&BrushLibrary::previewRequested);library.requestPreview(library.selectedId());QCOMPARE(requests.size(),1);const auto first=requests.last();library.setActiveColors(Qt::red,Qt::blue);library.acceptPreview(first[0].toString(),first[1].toULongLong(),"white-cache");QCOMPARE(library.selectedPreview(),QString("white-cache"));QCOMPARE(requests.size(),1);
        const auto catalog=library.catalogSnapshot();library.setActiveColors(Qt::green,Qt::yellow);QCOMPARE(library.catalogSnapshot(),catalog);QCOMPARE(library.selectedPreview(),QString("white-cache"));library.setAngle(15);QVERIFY(library.selectedPreview().isEmpty());library.requestPreview(library.selectedId());QCOMPARE(requests.size(),2);QVERIFY(!requests.last()[5].toMap().contains("previewBackground"));library.acceptPreview(first[0].toString(),first[1].toULongLong(),"stale");QVERIFY(library.selectedPreview().isEmpty());

    }
    void dualColorControlsTwoColorPreviewCaptureExchangeAndReopen() {
        QSKIP("双色绘制已删除");
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:4")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        library->setRadius(4);library->setOpacity(.4);library->setSpacing(.25);library->setPressureSize(false);library->setPressureFlow(false);auto data=library->dualBrush();data["enabled"]=true;data["model"]="paired-dab-v1";data["spacing"]=QVariant{};data["diameter"]=16.;data["pressureSize"]=false;data["pressureFlow"]=false;QVERIFY(library->setDualBrush(data));
        auto *page=findVisualItem(window->contentItem(),"settingsDualPage");QVERIFY(page);page->setProperty("currentIndex",4);QVERIFY(QMetaObject::invokeMethod(page,"activated",Q_ARG(int,4)));
        for(const auto &name:{"settingsDualColorEnabled","settingsDualOverflow"}){auto *item=findVisualItem(window->contentItem(),name);QVERIFY(item && item->isVisible());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,item->mapToScene({8,item->height()/2}).toPoint());}
        QTRY_VERIFY(library->dualBrush().value("color").toMap().value("overflow").toBool());QVERIFY(library->dualBrush().value("color").toMap().value("useBackground").toBool());auto *mode=findVisualItem(window->contentItem(),"settingsDualPrimaryBlend");QVERIFY(mode);QCOMPARE(mode->property("count").toInt(),27);
        library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto blue=library->selectedPreview();const auto definition=library->catalogSnapshot();client.setSecondaryBrushColor(Qt::green);QCOMPARE(library->catalogSnapshot(),definition);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QCOMPARE(library->selectedPreview(),blue);client.setSecondaryBrushColor(Qt::blue);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto normal=library->selectedPreview();client.setBrushWriteBlend(3);QCOMPARE(library->selectedPreview(),normal);client.setBrushWriteBlend(0);
        auto *preview=findVisualItem(window->contentItem(),"brushStrokePreviewImage");QVERIFY(preview);QTRY_COMPARE(preview->property("status").toInt(),1);QTest::qWait(50);const auto screenshot=qEnvironmentVariable("DRAWVERSE_DUAL_COLOR_PREVIEW");if(!screenshot.isEmpty())QVERIFY(window->grabWindow().save(screenshot));
        const auto archive=QUrl::fromLocalFile(temp.filePath("color.vbr"));QSignalSpy exported(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.exportBrush(archive));QTRY_COMPARE(exported.size(),1);QVERIFY(exported.last()[0].toBool());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);
        const auto start=[](){InputSample s;s.position={16.5,32.5};s.pressure=1;s.tool=1;return s;};const auto finish=[](PaintCoreClient &c){InputSample p;p.tool=1;p.position={48.5,32.5};p.pressure=1;c.strokeTo(p);c.endStroke();};QVERIFY(client.beginStroke(start()));client.swapBrushColors();finish(client);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        const auto rendered=[](PaintCoreClient &c){c.requestViewport(0,{0,0,64,64},{64,64});return c.frameRevision()>=c.revision() && c.frame().size()==QSize(64,64);};QTRY_VERIFY(rendered(client));const auto expected=client.frame();QVERIFY(expected.pixelColor(32,32).red()>expected.pixelColor(32,32).blue());QVERIFY(expected.pixelColor(32,38).blue()>expected.pixelColor(32,38).red());
        PaintCoreClient target(nullptr,temp.filePath("target/storage.ini"));QTRY_VERIFY(target.ready());QSignalSpy imported(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());target.newTransparentDocument(64,64);QTRY_COMPARE(target.documentWidth(),64);QTRY_VERIFY(target.ready());target.setBrushColor(Qt::red);target.setSecondaryBrushColor(Qt::blue);QVERIFY(target.beginStroke(start()));finish(target);QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(rendered(target));QCOMPARE(target.frame(),expected);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(rendered(client));QCOMPARE(client.frame(),expected);
        auto edit=library->dualBrush();auto color=edit.value("color").toMap();color["enabled"]=false;edit["color"]=color;QVERIFY(library->setDualBrush(edit));const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("dualBrush").toObject()==QJsonObject::fromVariantMap(edit);return false;};QTRY_VERIFY(saved());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->dualBrush(),edit);QCOMPARE(warnings,QStringList());
    }
    void dualColorInterleavedPreviewPixelsAndPersistence() {
        QSKIP("双色绘制已删除");
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);
        library->setRadius(6);library->setOpacity(1.);library->setSpacing(1.);library->setPressureSize(false);library->setPressureFlow(false);
        auto data=library->dualBrush();data["enabled"]=true;data["model"]="paired-dab-v1";data["spacing"]=QVariant{};data["diameter"]=28.;data["pressureSize"]=false;data["pressureFlow"]=false;
        data["color"]=QVariantMap{{"model","interleaved-stamp-v1"},{"enabled",true},{"useBackground",true},{"overflow",true},{"order","second-behind"},{"primaryBlend","normal"},{"secondaryBlend","normal"}};
        QVERIFY(library->setDualBrush(data));library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        InputSample start;start.position={16.5,32.5};start.pressure=1;start.tool=1;QVERIFY(client.beginStroke(start));InputSample end;end.position={48.5,32.5};end.pressure=1;end.tool=1;client.strokeTo(end);client.endStroke();
        QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,64,64},{64,64});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(64,64));
        bool painted=false;for(int y=0;y<64;++y)for(int x=0;x<64;++x)if(client.frame().pixelColor(x,y).alpha()>0)painted=true;QVERIFY(painted);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();if(state.value("version").toInt()!=22)return false;for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("dualBrush").toObject().value("color").toObject().value("model").toString()=="interleaved-stamp-v1";return false;};QTRY_VERIFY(saved());
    }
    void airbrushTicksWhileStationaryAndStopsAtStrokeEnd() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();client.setBrushColor(Qt::red);library->setRadius(6);library->setOpacity(.2);library->setSpacing(.5);library->setPressureSize(false);library->setPressureFlow(false);
        QVERIFY(library->setAirbrush(QVariantMap{{"model","time-interval-v1"},{"enabled",true},{"intervalMs",20.}}));
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        InputSample start;start.position={32.5,32.5};start.pressure=1;start.tool=1;QVERIFY(client.beginStroke(start));QTest::qWait(140);client.endStroke();
        QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,64,64},{64,64});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(64,64));
        QVERIFY(client.frame().pixelColor(32,32).alpha()>60);
        const auto state=QJsonDocument::fromJson(library->snapshot()).object();QCOMPARE(state.value("version").toInt(),23);
    }
    void speedDynamicsPresetsPreviewAndReopen() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        auto data=library->speedDynamics();QVERIFY(!data.value("enabled").toBool());QCOMPARE(data.value("maxSpeed").toDouble(),2000.);data["enabled"]=true;data["maxSpeed"]=1200.;auto size=data.value("size").toMap();size["curve"]=QVariantList{QVariantList{0.,1.},QVariantList{1.,.25}};size["minimum"]=.1;data["size"]=size;QVERIFY(library->setSpeedDynamics(data));QCOMPARE(QJsonDocument::fromJson(library->snapshot()).object().value("version").toInt(),24);
        library->setRadius(6);library->setOpacity(.5);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("speedDynamics").toObject()==QJsonObject::fromVariantMap(data);return false;};QTRY_VERIFY(saved());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->speedDynamics(),data);
    }
    void speedDynamicsQmlToggleAndMax() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:6")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());QTest::qWait(100);
        auto *toggle=findVisualItem(window->contentItem(),"settingsSpeedEnabled");QVERIFY(toggle);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({8,toggle->height()/2}).toPoint());QTRY_VERIFY(library->speedDynamics().value("enabled").toBool());
        auto *maximum=findVisualItem(window->contentItem(),"settingsSpeedMax");QVERIFY(maximum);QVERIFY(maximum->setProperty("value",1200));QVERIFY(QMetaObject::invokeMethod(maximum,"valueModified"));QTRY_COMPARE(library->speedDynamics().value("maxSpeed").toDouble(),1200.);
    }
    void tangentDynamicsPresetsPreviewAndReopen() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        auto data=library->tangentDynamics();QVERIFY(!data.value("enabled").toBool());data["enabled"]=true;auto size=data.value("size").toMap();size["curve"]=QVariantList{QVariantList{0.,.2},QVariantList{1.,1.}};size["minimum"]=0.;data["size"]=size;QVERIFY(library->setTangentDynamics(data));QCOMPARE(QJsonDocument::fromJson(library->snapshot()).object().value("version").toInt(),25);
        library->setRadius(6);library->setOpacity(.5);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("tangentDynamics").toObject()==QJsonObject::fromVariantMap(data);return false;};QTRY_VERIFY(saved());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->tangentDynamics(),data);
    }
    void tangentDynamicsQmlToggleAndCurve() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:6")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());QTest::qWait(100);
        auto *toggle=findVisualItem(window->contentItem(),"settingsTangentEnabled");QVERIFY(toggle);QVERIFY(QMetaObject::invokeMethod(library,"setTangentDynamicsEnabled",Q_ARG(bool,true)));QTRY_VERIFY(toggle->property("checked").toBool());QVERIFY(library->tangentDynamics().value("enabled").toBool());
        QVERIFY(library->setTangentControl("size",QVariantList{QVariantList{0.,.1},QVariantList{1.,1.}},0.));QCOMPARE(library->tangentSizeCurve().size(),2);QVERIFY(findVisualItem(window->contentItem(),"tangentSizeEditor"));
    }
    void distanceDynamicsPresetsPreviewAndReopen() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        auto data=library->distanceDynamics();QVERIFY(!data.value("enabled").toBool());QCOMPARE(data.value("maxDistance").toDouble(),1000.);data["enabled"]=true;data["maxDistance"]=320.;auto size=data.value("size").toMap();size["curve"]=QVariantList{QVariantList{0.,1.},QVariantList{1.,.25}};size["minimum"]=0.;data["size"]=size;QVERIFY(library->setDistanceDynamics(data));QCOMPARE(QJsonDocument::fromJson(library->snapshot()).object().value("version").toInt(),26);
        library->setRadius(6);library->setOpacity(.5);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("distanceDynamics").toObject()==QJsonObject::fromVariantMap(data);return false;};QTRY_VERIFY(saved());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->distanceDynamics(),data);
    }
    void distanceDynamicsQmlToggleAndCurve() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:6")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());QTest::qWait(100);
        auto *toggle=findVisualItem(window->contentItem(),"settingsDistanceEnabled");QVERIFY(toggle);QVERIFY(QMetaObject::invokeMethod(library,"setDistanceDynamicsEnabled",Q_ARG(bool,true)));QTRY_VERIFY(toggle->property("checked").toBool());QVERIFY(library->distanceDynamics().value("enabled").toBool());
        QVERIFY(QMetaObject::invokeMethod(library,"setDistanceDynamicsMaxDistance",Q_ARG(qreal,320.)));QTRY_COMPARE(library->distanceDynamics().value("maxDistance").toDouble(),320.);
        QVERIFY(library->setDistanceControl("size",QVariantList{QVariantList{0.,1.},QVariantList{1.,.2}},0.));QCOMPARE(library->distanceSizeCurve().size(),2);QVERIFY(findVisualItem(window->contentItem(),"distanceSizeEditor"));
    }
    void tiltDynamicsPresetsPreviewAndReopen() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        auto data=library->tiltDynamics();QVERIFY(!data.value("enabled").toBool());data["enabled"]=true;auto size=data.value("size").toMap();size["curve"]=QVariantList{QVariantList{0.,1.},QVariantList{1.,.2}};size["minimum"]=0.;data["size"]=size;QVERIFY(library->setTiltDynamics(data));QCOMPARE(QJsonDocument::fromJson(library->snapshot()).object().value("version").toInt(),27);
        library->setRadius(6);library->setOpacity(.5);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("tiltDynamics").toObject()==QJsonObject::fromVariantMap(data);return false;};QTRY_VERIFY(saved());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->tiltDynamics(),data);
    }
    void tiltDynamicsQmlToggleAndCurve() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:6")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());QTest::qWait(100);
        auto *toggle=findVisualItem(window->contentItem(),"settingsTiltEnabled");QVERIFY(toggle);QVERIFY(QMetaObject::invokeMethod(library,"setTiltDynamicsEnabled",Q_ARG(bool,true)));QTRY_VERIFY(toggle->property("checked").toBool());QVERIFY(library->tiltDynamics().value("enabled").toBool());
        QVERIFY(library->setTiltControl("size",QVariantList{QVariantList{0.,1.},QVariantList{1.,.2}},0.));QCOMPARE(library->tiltSizeCurve().size(),2);QVERIFY(findVisualItem(window->contentItem(),"tiltSizeEditor"));
    }
    void rotationDynamicsPresetsPreviewAndReopen() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        auto data=library->rotationDynamics();QVERIFY(!data.value("enabled").toBool());data["enabled"]=true;auto size=data.value("size").toMap();size["curve"]=QVariantList{QVariantList{0.,1.},QVariantList{1.,.2}};size["minimum"]=0.;data["size"]=size;QVERIFY(library->setRotationDynamics(data));QCOMPARE(QJsonDocument::fromJson(library->snapshot()).object().value("version").toInt(),28);
        library->setRadius(6);library->setOpacity(.5);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("rotationDynamics").toObject()==QJsonObject::fromVariantMap(data);return false;};QTRY_VERIFY(saved());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->rotationDynamics(),data);
    }
    void rotationDynamicsQmlToggleAndCurve() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:6")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());QTest::qWait(100);
        auto *toggle=findVisualItem(window->contentItem(),"settingsRotationEnabled");QVERIFY(toggle);QVERIFY(QMetaObject::invokeMethod(library,"setRotationDynamicsEnabled",Q_ARG(bool,true)));QTRY_VERIFY(toggle->property("checked").toBool());QVERIFY(library->rotationDynamics().value("enabled").toBool());
        QVERIFY(library->setRotationControl("size",QVariantList{QVariantList{0.,1.},QVariantList{1.,.2}},0.));QCOMPARE(library->rotationSizeCurve().size(),2);QVERIFY(findVisualItem(window->contentItem(),"rotationSizeEditor"));
    }
    void directionDynamicsPresetsPreviewAndReopen() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        auto data=library->directionDynamics();QVERIFY(!data.value("enabled").toBool());data["enabled"]=true;auto size=data.value("size").toMap();size["curve"]=QVariantList{QVariantList{0.,1.},QVariantList{1.,.2}};size["minimum"]=0.;data["size"]=size;QVERIFY(library->setDirectionDynamics(data));QCOMPARE(QJsonDocument::fromJson(library->snapshot()).object().value("version").toInt(),29);
        library->setRadius(6);library->setOpacity(.5);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("directionDynamics").toObject()==QJsonObject::fromVariantMap(data);return false;};QTRY_VERIFY(saved());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->directionDynamics(),data);
    }
    void directionDynamicsQmlToggleAndCurve() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:6")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());QTest::qWait(100);
        auto *toggle=findVisualItem(window->contentItem(),"settingsDirectionEnabled");QVERIFY(toggle);QVERIFY(QMetaObject::invokeMethod(library,"setDirectionDynamicsEnabled",Q_ARG(bool,true)));QTRY_VERIFY(toggle->property("checked").toBool());QVERIFY(library->directionDynamics().value("enabled").toBool());
        QVERIFY(library->setDirectionControl("size",QVariantList{QVariantList{0.,1.},QVariantList{1.,.2}},0.));QCOMPARE(library->directionSizeCurve().size(),2);QVERIFY(findVisualItem(window->contentItem(),"directionSizeEditor"));
    }
    void fadeDynamicsPresetsPreviewAndReopen() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        auto data=library->fadeDynamics();QVERIFY(!data.value("enabled").toBool());data["enabled"]=true;data["steps"]=4;auto size=data.value("size").toMap();size["curve"]=QVariantList{QVariantList{0.,1.},QVariantList{1.,.2}};size["minimum"]=0.;data["size"]=size;QVERIFY(library->setFadeDynamics(data));QCOMPARE(QJsonDocument::fromJson(library->snapshot()).object().value("version").toInt(),34);
        library->setRadius(6);library->setOpacity(.5);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("fadeDynamics").toObject()==QJsonObject::fromVariantMap(data);return false;};QTRY_VERIFY(saved());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->fadeDynamics(),data);
    }
    void fadeDynamicsQmlToggleStepsAndCurve() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:6")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());QTest::qWait(100);
        auto *toggle=findVisualItem(window->contentItem(),"settingsFadeEnabled");QVERIFY(toggle);QVERIFY(QMetaObject::invokeMethod(library,"setFadeDynamicsEnabled",Q_ARG(bool,true)));QTRY_VERIFY(toggle->property("checked").toBool());QVERIFY(library->fadeDynamicsEnabled());
        QVERIFY(QMetaObject::invokeMethod(library,"setFadeSteps",Q_ARG(int,6)));QCOMPARE(library->fadeSteps(),6);auto *steps=findVisualItem(window->contentItem(),"settingsFadeSteps");QVERIFY(steps);QTRY_COMPARE(steps->property("value").toInt(),6);
        QVERIFY(library->setFadeControl("size",QVariantList{QVariantList{0.,1.},QVariantList{1.,.2}},0.));QCOMPARE(library->fadeSizeCurve().size(),2);QVERIFY(findVisualItem(window->contentItem(),"fadeSizeEditor"));
    }
    void dynamicsSourceMatrixPersistsPreviewAndReopen() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        auto speed=library->speedDynamics();speed["enabled"]=true;auto size=speed.value("size").toMap();size["curve"]=QVariantList{QVariantList{0.,1.},QVariantList{1.,.3}};size["minimum"]=0.;speed["size"]=size;QVERIFY(library->setSpeedDynamics(speed));
        QVERIFY(library->setDynamicsSource("size","speed"));QVERIFY(library->setDynamicsSource("flow","distance"));QVERIFY(library->setDynamicsSource("opacity","tilt"));
        const auto matrix=library->dynamicsSourceMatrix();QCOMPARE(QJsonDocument::fromJson(library->snapshot()).object().value("version").toInt(),32);
        library->setRadius(6);library->setOpacity(.5);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("sourceMatrix").toObject()==QJsonObject::fromVariantMap(matrix);return false;};QTRY_VERIFY(saved());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->dynamicsSourceMatrix(),matrix);
    }
    void dynamicsSourceMatrixQmlDropdowns() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:6")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());QTest::qWait(100);
        auto *size=findVisualItem(window->contentItem(),"settingsDynamicsSizeSource");auto *flow=findVisualItem(window->contentItem(),"settingsDynamicsFlowSource");auto *opacity=findVisualItem(window->contentItem(),"settingsDynamicsOpacitySource");QVERIFY(size);QVERIFY(flow);QVERIFY(opacity);
        QCOMPARE(size->property("count").toInt(),7);QCOMPARE(flow->property("count").toInt(),7);QCOMPARE(opacity->property("count").toInt(),7);
        QVERIFY(QMetaObject::invokeMethod(library,"setDynamicsSource",Q_ARG(QString,"size"),Q_ARG(QString,"rotation")));QTRY_COMPARE(size->property("currentIndex").toInt(),5);
        QVERIFY(QMetaObject::invokeMethod(library,"setDynamicsSource",Q_ARG(QString,"flow"),Q_ARG(QString,"distance")));QTRY_COMPARE(flow->property("currentIndex").toInt(),3);
        QVERIFY(QMetaObject::invokeMethod(library,"setDynamicsSource",Q_ARG(QString,"opacity"),Q_ARG(QString,"direction")));QTRY_COMPARE(opacity->property("currentIndex").toInt(),6);
    }
    void advancedTextureModesPresetsAndReopen() {
        BrushLibrary library;auto texture=library.texture();QCOMPARE(texture.value("mode").toString(),QString("multiply"));texture["mode"]="hardMix";QVERIFY(library.setTexture(texture));auto state=library.snapshot();QCOMPARE(QJsonDocument::fromJson(state).object().value("version").toInt(),30);BrushLibrary restored;QVERIFY(restored.restore(state));QCOMPARE(restored.texture().value("mode").toString(),QString("hardMix"));
        texture["mode"]="linearHeight";QVERIFY(library.setTexture(texture));state=library.snapshot();QCOMPARE(QJsonDocument::fromJson(state).object().value("version").toInt(),31);QVERIFY(restored.restore(state));QCOMPARE(restored.texture().value("mode").toString(),QString("linearHeight"));
        texture["mode"]="height";QVERIFY(library.setTexture(texture));state=library.snapshot();QCOMPARE(QJsonDocument::fromJson(state).object().value("version").toInt(),31);QVERIFY(restored.restore(state));QCOMPARE(restored.texture().value("mode").toString(),QString("height"));
        texture["mode"]="vividLight";QVERIFY(library.setTexture(texture));QCOMPARE(library.texture().value("mode").toString(),QString("multiply"));texture["mode"]="dissolve";QVERIFY(library.setTexture(texture));QCOMPARE(library.texture().value("mode").toString(),QString("multiply"));
    }
    void advancedTextureModesQmlDropdown() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:3")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());QTest::qWait(100);
        auto *mode=findVisualItem(window->contentItem(),"settingsTextureMode");QVERIFY(mode);QCOMPARE(mode->property("count").toInt(),10);QCOMPARE(mode->property("currentIndex").toInt(),0);mode->setProperty("currentIndex",7);QVERIFY(QMetaObject::invokeMethod(mode,"activated",Q_ARG(int,7)));QCOMPARE(library->texture().value("mode").toString(),QString("hardMix"));
    }
    void dualSizingRetainedModesHighestVersionAndResetBaselines() {
        BrushLibrary library;auto data=library.dualBrush();QVariantMap sizing{{"model","relative-size-v1"},{"mode","relative"},{"diameterOffset",-8.},{"widthRule","fixed-width"}};data["sizing"]=sizing;QVERIFY(library.setDualBrush(data));data=library.dualBrush();
        const auto copy=library.saveCopy("相对尺寸基准");QVERIFY(!copy.isEmpty());auto edit=data;sizing["mode"]="independent";sizing["diameterOffset"]=17.;edit["sizing"]=sizing;QVERIFY(library.setDualBrush(edit));QVERIFY(library.resetPreset());QCOMPARE(library.dualBrush(),data);
        QVERIFY(library.select("round-wide"));auto smooth=library.smoothing();smooth["positionMs"]=24.;QVERIFY(library.setSmoothing(smooth));const auto state=library.snapshot();QCOMPARE(QJsonDocument::fromJson(state).object().value("version").toInt(),15);BrushLibrary restored;QVERIFY(restored.restore(state));QVERIFY(restored.select(copy));QCOMPARE(restored.dualBrush(),data);
        const auto good=restored.snapshot();for(const auto &value:QVariantList{511.000000001,QString("8"),QVariant{}}){auto invalid=data;auto nested=data.value("sizing").toMap();nested["diameterOffset"]=value;invalid["sizing"]=nested;QVERIFY(!restored.setDualBrush(invalid));}QCOMPARE(restored.snapshot(),good);
        auto old=QJsonDocument::fromJson(good).object();old["version"]=14;QVERIFY(!restored.restore(QJsonDocument(old).toJson()));QCOMPARE(restored.snapshot(),good);
        const auto package=restored.exportPackage();BrushLibrary imported;QVERIFY(imported.importPackage(package));QCOMPARE(imported.dualBrush(),data);QCOMPARE(QJsonDocument::fromJson(package).object().value("brushes").toArray()[0].toObject().value("behaviorVersion").toInt(),12);
    }
    void dualFlipQmlToggle() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:4")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());QTest::qWait(100);
        auto *page=findVisualItem(window->contentItem(),"settingsDualPage");auto *mode=findVisualItem(window->contentItem(),"settingsDualMode");QVERIFY(page && mode);QCOMPARE(page->property("count").toInt(),4);QCOMPARE(mode->property("count").toInt(),8);QCOMPARE(mode->property("currentIndex").toInt(),0);mode->setProperty("currentIndex",7);QVERIFY(QMetaObject::invokeMethod(mode,"activated",Q_ARG(int,7)));QCOMPARE(library->dualBrush().value("mode").toString(),QString("linearHeight"));QVERIFY(!findVisualItem(window->contentItem(),"settingsDualColorEnabled"));
        auto *flip=findVisualItem(window->contentItem(),"settingsDualFlip");QVERIFY(flip);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,flip->mapToScene({8,flip->height()/2}).toPoint());QTRY_VERIFY(library->dualBrush().value("flip").toBool());QVERIFY(flip->property("checked").toBool());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,flip->mapToScene({8,flip->height()/2}).toPoint());QTRY_VERIFY(!library->dualBrush().value("flip").toBool());
        auto *flipX=findVisualItem(window->contentItem(),"settingsDualFlipX");auto *flipY=findVisualItem(window->contentItem(),"settingsDualFlipY");QVERIFY(flipX);QVERIFY(flipY);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,flipX->mapToScene({8,flipX->height()/2}).toPoint());QTRY_VERIFY(library->dualBrush().value("flipX").toBool());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,flipY->mapToScene({8,flipY->height()/2}).toPoint());QTRY_VERIFY(library->dualBrush().value("flipY").toBool());
    }
    void dualSizingControlsPreviewCaptureExchangeAndReopen() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:4")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        library->setRadius(6);library->setOpacity(.4);library->setPressureSize(false);library->setPressureFlow(false);
        auto *toggle=findVisualItem(window->contentItem(),"settingsDualEnabled");QVERIFY(toggle);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({8,toggle->height()/2}).toPoint());QTRY_VERIFY(library->dualBrush().value("enabled").toBool());
        library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto plain=library->selectedPreview();
        auto *mode=findVisualItem(window->contentItem(),"settingsDualSizeMode");QVERIFY(mode);mode->setProperty("currentIndex",1);QVERIFY(QMetaObject::invokeMethod(mode,"activated",Q_ARG(int,1)));
        auto *input=findVisualItem(window->contentItem(),"settingsDualsizingdiameterOffsetInput");QVERIFY(input && input->isVisible());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,input->mapToScene({input->width()/2,input->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_Minus);QTest::keyClick(window,Qt::Key_8);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(library->dualBrush().value("sizing").toMap().value("diameterOffset").toDouble(),-8.);
        auto *rule=findVisualItem(window->contentItem(),"settingsDualWidthRule");QVERIFY(rule);rule->setProperty("currentIndex",1);QVERIFY(QMetaObject::invokeMethod(rule,"activated",Q_ARG(int,1)));QTRY_COMPARE(library->dualBrush().value("sizing").toMap().value("widthRule").toString(),QString("fixed-width"));
        library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(library->selectedPreview()!=plain);auto *preview=findVisualItem(window->contentItem(),"brushStrokePreviewImage");QVERIFY(preview);QTRY_COMPARE(preview->property("status").toInt(),1);QTest::qWait(50);const auto screenshot=qEnvironmentVariable("DRAWVERSE_DUAL_SIZE_PREVIEW");if(!screenshot.isEmpty())QVERIFY(window->grabWindow().save(screenshot));
        const auto archive=QUrl::fromLocalFile(temp.filePath("sizing.vbr"));QSignalSpy exported(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.exportBrush(archive));QTRY_COMPARE(exported.size(),1);QVERIFY(exported.last()[0].toBool());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);client.setBrushColor(Qt::red);
        const auto start=[](){InputSample s;s.position={8.5,24.5};s.pressure=.3f;s.tool=1;return s;};const auto finish=[](PaintCoreClient &c){InputSample p;p.tool=1;p.position={32.5,38.5};p.pressure=.4f;c.strokeTo(p);p.position={54.5,24.5};p.pressure=1;c.strokeTo(p);c.endStroke();};
        QVERIFY(client.beginStroke(start()));auto edit=library->dualBrush();auto nested=edit.value("sizing").toMap();nested["diameterOffset"]=8.;edit["sizing"]=nested;QVERIFY(library->setDualBrush(edit));finish(client);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        const auto rendered=[](PaintCoreClient &c){c.requestViewport(0,{0,0,64,64},{64,64});return c.frameRevision()>=c.revision() && c.frame().size()==QSize(64,64);};QTRY_VERIFY(rendered(client));const auto expected=client.frame();
        PaintCoreClient target(nullptr,temp.filePath("target/storage.ini"));QTRY_VERIFY(target.ready());QSignalSpy imported(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QCOMPARE(target.brushLibrary()->dualBrush().value("sizing").toMap().value("diameterOffset").toDouble(),-8.);target.newTransparentDocument(64,64);QTRY_COMPARE(target.documentWidth(),64);QTRY_VERIFY(target.ready());target.setBrushColor(Qt::red);QVERIFY(target.beginStroke(start()));finish(target);QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(rendered(target));QCOMPARE(target.frame(),expected);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(rendered(client));QCOMPARE(client.frame(),expected);
        edit["enabled"]=false;QVERIFY(library->setDualBrush(edit));const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("dualBrush").toObject()==QJsonObject::fromVariantMap(edit);return false;};QTRY_VERIFY(saved());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->dualBrush(),edit);QCOMPARE(warnings,QStringList());
    }
    void taperModelPreservesClosedCurvesResetAndExchange() {
        BrushLibrary library;const auto defaults=library.strokeTaper();QVERIFY(!defaults.value("enabled").toBool());const auto old=library.snapshot();
        auto data=defaults;auto start=data.value("start").toMap();start["lengthPx"]=24.125;auto control=start.value("size").toMap();control["curve"]=QVariantList{QVariantList{0.,0.},QVariantList{.4,.1},QVariantList{1.,1.}};control["interpolation"]="pchip";control["minimum"]=.15;start["size"]=control;data["start"]=start;data["enabled"]=true;QVERIFY(library.setStrokeTaper(data));
        const auto copy=library.saveCopy("起收笔基准");QVERIFY(!copy.isEmpty());auto closed=data;closed["enabled"]=false;QVERIFY(library.setStrokeTaper(closed));
        const auto state=library.snapshot();QCOMPARE(QJsonDocument::fromJson(state).object().value("version").toInt(),18);BrushLibrary restored;QVERIFY(restored.restore(state));QCOMPARE(restored.strokeTaper(),closed);QVERIFY(restored.resetPreset());QCOMPARE(restored.strokeTaper(),data);
        const auto package=library.exportPackage();const auto p=QJsonDocument::fromJson(package).object().value("brushes").toArray().first().toObject();QCOMPARE(p.value("behaviorVersion").toInt(),15);QVERIFY(p.value("requiredCapabilities").toArray().contains("stroke-taper"));BrushLibrary imported;QVERIFY(imported.importPackage(package));QCOMPARE(imported.strokeTaper(),closed);QVERIFY(imported.resetPreset());QCOMPARE(imported.strokeTaper(),closed);
        for(int mutation=0;mutation<9;++mutation){auto bad=closed;auto end=bad.value("start").toMap();auto c=end.value("size").toMap();if(mutation==0)bad["enabled"]=1;if(mutation==1)bad["extra"]=true;if(mutation==2)end["lengthPx"]=4096.00001;if(mutation==3)c["minimum"]=-.01;if(mutation==4)c["interpolation"]="unknown";if(mutation==5)c["curve"]=QVariantList{QVariantList{0.,.1},QVariantList{1.,1.}};if(mutation==6)c["curve"]=QVariantList{QVariantList{0.,0.},QVariantList{.5,.2},QVariantList{.50000000001,.3},QVariantList{1.,1.}};if(mutation==7)c.remove("curve");if(mutation==8)end["lengthPx"]=std::numeric_limits<double>::infinity();end["size"]=c;bad["start"]=end;QVERIFY(!library.setStrokeTaper(bad));QCOMPARE(library.snapshot(),state);}
        auto downgraded=QJsonDocument::fromJson(state).object();downgraded["version"]=17;QVERIFY(!library.restore(QJsonDocument(downgraded).toJson()));QCOMPARE(library.snapshot(),state);
        QVERIFY(library.select("round-wide"));auto wet=library.wetMix();wet["enabled"]=true;QVERIFY(library.setWetMix(wet));QCOMPARE(QJsonDocument::fromJson(library.snapshot()).object().value("version").toInt(),18);
        BrushLibrary plain;QVERIFY(plain.setStrokeTaper(defaults));QCOMPARE(plain.snapshot(),old);QVERIFY(library.setStrokeTaper(defaults));
    }
    void taperControlsCaptureWhitePreviewPixelsVbrAndReopen() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:11")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        auto *scroll=findVisualItem(window->contentItem(),"brushParameterScroll");QVERIFY(scroll);QTRY_VERIFY(scroll->property("contentWidth").toDouble()>=200);workspace.setLeftDockWidth(420);
        auto *length=findVisualItem(window->contentItem(),"settingsTaperLength");QVERIFY(length && length->isVisible());length->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);for(const auto key:{Qt::Key_2,Qt::Key_4,Qt::Key_Period,Qt::Key_2,Qt::Key_5})QTest::keyClick(window,key);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(library->strokeTaper().value("start").toMap().value("lengthPx").toDouble(),24.25);
        auto *curve=findVisualItem(window->contentItem(),"taperSizeEditor");QVERIFY(curve);QVERIFY(QMetaObject::invokeMethod(curve,"addMiddle"));QCOMPARE(library->strokeTaper().value("start").toMap().value("size").toMap().value("interpolation").toString(),QString("pchip"));QCOMPARE(library->strokeTaper().value("start").toMap().value("size").toMap().value("curve").toList().size(),3);
        auto *toggle=findVisualItem(window->contentItem(),"settingsTaperEnabled");QVERIFY(toggle);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({7,toggle->height()/2}).toPoint());QTRY_VERIFY(library->strokeTaper().value("enabled").toBool());
        auto taper=library->strokeTaper();auto end=taper.value("end").toMap();end["lengthPx"]=24.25;taper["end"]=end;QVERIFY(library->setStrokeTaper(taper));library->setRadius(8);library->setOpacity(.5);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY2_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),qPrintable(library->previewMessage()),10000);
        QImage preview;QVERIFY(preview.loadFromData(QByteArray::fromBase64(library->selectedPreview().section(',',1).toLatin1())));bool ink=false;for(int y=0;y<preview.height();++y)for(int x=0;x<preview.width();++x){auto c=preview.pixelColor(x,y);if(c.alpha()){ink=true;QCOMPARE(c.red(),255);QCOMPARE(c.green(),255);QCOMPARE(c.blue(),255);}}QVERIFY(ink);
        const auto archive=QUrl::fromLocalFile(temp.filePath("taper.vbr"));QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.exportBrush(archive));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());
        client.newTransparentDocument(96,64);QTRY_COMPARE(client.documentWidth(),96);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);client.setBrushColor(Qt::red);
        const auto start=[](){InputSample s;s.position={12,32};s.pressure=1;s.tool=1;return s;};
        const auto finish=[](PaintCoreClient &c){for(int x=16;x<=84;x+=4){InputSample s;s.position={qreal(x),32};s.pressure=1;s.tool=1;c.strokeTo(s);}c.endStroke();};
        const auto render=[](PaintCoreClient &c){c.requestViewport(0,{0,0,96,64},{96,64});return c.frameRevision()>=c.revision() && c.frame().size()==QSize(96,64);};
        QVERIFY(client.beginStroke(start()));auto closed=taper;closed["enabled"]=false;QVERIFY(library->setStrokeTaper(closed));finish(client);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(render(client));const auto expected=client.frame();QVERIFY(expected.pixelColor(48,32).alpha()>expected.pixelColor(14,32).alpha());
        PaintCoreClient target(nullptr,temp.filePath("target/storage.ini"));QTRY_VERIFY(target.ready());QSignalSpy imported(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QCOMPARE(target.brushLibrary()->strokeTaper(),taper);
        target.newTransparentDocument(96,64);QTRY_COMPARE(target.documentWidth(),96);QTRY_VERIFY(target.ready());target.setBrushColor(Qt::red);QVERIFY(target.beginStroke(start()));finish(target);QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(render(target));QCOMPARE(target.frame(),expected);
        target.undo();QTRY_COMPARE(target.undoDepth(),0);QTRY_VERIFY(!target.layerEditBusy());target.redo();QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(render(target));QCOMPARE(target.frame(),expected);
        target.newTransparentDocument(96,64);QTRY_COMPARE(target.undoDepth(),0);QTRY_VERIFY(target.ready());QVERIFY(target.brushLibrary()->setStrokeTaper(closed));QVERIFY(target.beginStroke(start()));finish(target);QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(render(target));QVERIFY(target.frame()!=expected);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("strokeTaper").toObject()==QJsonObject::fromVariantMap(closed);return false;};QTRY_VERIFY(saved());
        const auto screenshot=qEnvironmentVariable("DRAWVERSE_TAPER_PREVIEW");if(!screenshot.isEmpty())QVERIFY(window->grabWindow().save(screenshot));
        canvas->setClient(&client);QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->strokeTaper(),closed);
    }
    void smoothingPresetsKeepClosedValuesHighestVersionAndResetBaselines() {
        BrushLibrary library;auto data=library.smoothing();QVERIFY(!data.value("enabled").toBool());QCOMPARE(data.value("positionMs").toDouble(),12.);data["positionMs"]=24.125;data["pressureMs"]=12.;data["speedResponse"]=.7;data["stabilizerPx"]=3.;QVERIFY(library.setSmoothing(data));
        const auto copy=library.saveCopy("平滑初始基准");QVERIFY(!copy.isEmpty());auto edited=data;edited["enabled"]=true;QVERIFY(library.setSmoothing(edited));QVERIFY(library.resetPreset());QCOMPARE(library.smoothing(),data);QVERIFY(library.select("round-wide"));QVERIFY(library.setPressureControl("size",QVariantList{QVariantList{0.,0.},QVariantList{.5,.25},QVariantList{1.,1.}},0.));
        const auto state=library.snapshot();QCOMPARE(QJsonDocument::fromJson(state).object().value("version").toInt(),14);BrushLibrary restored;QVERIFY(restored.restore(state));QVERIFY(restored.select(copy));QCOMPARE(restored.smoothing(),data);
        const auto valid=restored.snapshot();auto bad=data;bad["positionMs"]=250.0000000001;QVERIFY(!restored.setSmoothing(bad));bad=data;bad["model"]="unknown";QVERIFY(!restored.setSmoothing(bad));bad=data;bad["enabled"]=1;QVERIFY(!restored.setSmoothing(bad));QCOMPARE(restored.snapshot(),valid);
        auto old=QJsonDocument::fromJson(valid).object();old["version"]=13;QVERIFY(!restored.restore(QJsonDocument(old).toJson()));QCOMPARE(restored.snapshot(),valid);auto package=restored.exportPackage();BrushLibrary imported;QVERIFY(imported.importPackage(package));QCOMPARE(imported.smoothing(),data);
    }
    void smoothingControlsPreviewCaptureExchangeAndPreferencesReopen() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:9")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        auto *position=findVisualItem(window->contentItem(),"settingsSmoothingpositionMsInput");QVERIFY(position && position->isEnabled());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,position->mapToScene({position->width()/2,position->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_2);QTest::keyClick(window,Qt::Key_4);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(library->smoothing().value("positionMs").toDouble(),24.);QVERIFY(!library->smoothing().value("enabled").toBool());
        auto data=library->smoothing();data["pressureMs"]=12.;data["speedResponse"]=.7;data["stabilizerPx"]=3.;QVERIFY(library->setSmoothing(data));library->setRadius(6);library->setOpacity(.4);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto plain=library->selectedPreview();
        auto *toggle=findVisualItem(window->contentItem(),"settingsSmoothingEnabled");QVERIFY(toggle);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({8,toggle->height()/2}).toPoint());QTRY_VERIFY(library->smoothing().value("enabled").toBool());library->requestPreview(library->selectedId());QTRY_VERIFY2_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),qPrintable(library->previewMessage()),10000);QVERIFY(library->selectedPreview()!=plain);
        auto *previewImage=findVisualItem(window->contentItem(),"brushStrokePreviewImage");QVERIFY(previewImage);QTRY_COMPARE(previewImage->property("status").toInt(),1);QTest::qWait(50);
        const auto screenshot=qEnvironmentVariable("DRAWVERSE_SMOOTHING_PREVIEW");if(!screenshot.isEmpty())QVERIFY(window->grabWindow().save(screenshot));
        const auto archive=QUrl::fromLocalFile(temp.filePath("smoothing.vbr"));QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.exportBrush(archive));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);client.setBrushColor(Qt::red);
        const auto start=[](){InputSample s;s.position={8.5,24.5};s.pressure=.3f;s.tool=1;s.timestamp=1000000;return s;};const auto finish=[](PaintCoreClient &c){auto p=InputSample{};p.tool=1;p.position={32.5,38.5};p.pressure=.4f;p.timestamp=41000000;c.strokeTo(p);p.position={54.5,24.5};p.pressure=1;p.timestamp=81000000;c.strokeTo(p);c.endStroke();};
        QVERIFY(client.beginStroke(start()));data=library->smoothing();data["enabled"]=false;QVERIFY(library->setSmoothing(data));finish(client);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        const auto rendered=[](PaintCoreClient &c){c.requestViewport(0,{0,0,64,64},{64,64});return c.frameRevision()>=c.revision() && c.frame().size()==QSize(64,64);};QTRY_VERIFY(rendered(client));const auto expected=client.frame();
        PaintCoreClient target(nullptr,temp.filePath("target/storage.ini"));QTRY_VERIFY(target.ready());QSignalSpy imported(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QVERIFY(target.brushLibrary()->smoothing().value("enabled").toBool());target.newTransparentDocument(64,64);QTRY_COMPARE(target.documentWidth(),64);QTRY_VERIFY(target.ready());target.setBrushColor(Qt::red);QVERIFY(target.beginStroke(start()));finish(target);QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(rendered(target));QCOMPARE(target.frame(),expected);
        target.newTransparentDocument(64,64);QTRY_COMPARE(target.undoDepth(),0);QTRY_VERIFY(target.ready());QVERIFY(target.brushLibrary()->setSmoothing(data));QVERIFY(target.beginStroke(start()));finish(target);QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(rendered(target));QVERIFY(target.frame()!=expected);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(rendered(client));QCOMPARE(client.frame(),expected);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &r:state.value("presets").toArray())if(r.toObject().value("id").toString()==library->selectedId())return r.toObject().value("smoothing").toObject()==QJsonObject::fromVariantMap(data);return false;};QTRY_VERIFY(saved());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->smoothing(),data);QCOMPARE(warnings,QStringList());
    }
    void toolComboDropdownStaysAboveNativePanels_data(){QTest::addColumn<bool>("fallback");QTest::newRow("window-popup")<<false;QTest::newRow("qt65-fallback")<<true;}
    void toolComboDropdownStaysAboveNativePanels() {
        QFETCH(bool,fallback);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        auto *combo=findVisualItem(main->contentItem(),"brushWriteBlend");QVERIFY(combo);if(fallback)combo->setProperty("windowPopupsAvailable",false);
        const auto group=workspace.groupForPanel("brush");workspace.detachGroup(group);QQuickWindow *panel=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+group)panel=qobject_cast<QQuickWindow*>(w);return panel!=nullptr;}());
        const auto below=combo->mapToGlobal({0,combo->height()}).toPoint();panel->setGeometry({below,QSize(320,360)});auto *backdrop=new BlurTestBackdrop(panel->contentItem());backdrop->setWidth(320);backdrop->setHeight(360);backdrop->setZ(10000);panel->raise();QTest::qWait(80);
        auto *popup=combo->property("popup").value<QObject*>();QVERIFY(popup);
        for(int pass=0;pass<2;++pass) {
            QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,combo->mapToScene({combo->width()/2,combo->height()/2}).toPoint());QQuickItem *option=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(auto *quick=qobject_cast<QQuickWindow*>(w))if((option=findVisualItem(quick->contentItem(),"brushWriteBlendOption:3")))return true;return false;}());
            auto *menu=option->window();QVERIFY(menu && menu!=main && menu!=panel);QTRY_VERIFY(menu->height()>80);QTest::qWait(80);
            const auto location=option->mapToGlobal({option->width()-5,option->height()/2}).toPoint();QVERIFY(panel->geometry().contains(location));const auto capture=menu->screen()->grabWindow(0,location.x(),location.y(),1,1).toImage();QVERIFY(!capture.isNull());const auto color=capture.pixelColor(capture.width()/2,capture.height()/2);QCOMPARE(color,QColor("#1c1e21"));
            const auto screenshot=qEnvironmentVariable("DRAWVERSE_COMBO_PRIORITY_PREVIEW");if(!screenshot.isEmpty() && pass==0)QVERIFY(menu->screen()->grabWindow(0,below.x(),below.y(),320,360).save(screenshot+(fallback?".fallback.png":".native.png")));
            if(pass==0){QTest::keyClick(menu,Qt::Key_Escape);QTRY_VERIFY(!popup->property("visible").toBool());QTRY_VERIFY(!menu->isVisible());}
            else {QTest::mouseClick(menu,Qt::LeftButton,Qt::NoModifier,option->mapToScene({option->width()/2,option->height()/2}).toPoint());QTRY_COMPARE(client.brushWriteBlend(),3);QTRY_VERIFY(!popup->property("visible").toBool());}
        }
        QCOMPARE(warnings,QStringList());
    }
    void toolComboMouseAndTabletOwnerWindowClicksNeverPaintThrough_data() {
        QTest::addColumn<bool>("fallback");QTest::addColumn<bool>("tablet");
        QTest::newRow("native-mouse")<<false<<false;QTest::newRow("native-pen")<<false<<true;
        QTest::newRow("fallback-mouse")<<true<<false;QTest::newRow("fallback-pen")<<true<<true;
    }
    void toolComboMouseAndTabletOwnerWindowClicksNeverPaintThrough() {
        QFETCH(bool,fallback);QFETCH(bool,tablet);QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("state.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(960,640);QTRY_COMPARE(client.documentWidth(),960);QTRY_VERIFY(!client.layerEditBusy());client.setBrushWriteBlend(0);client.setBrushColor(Qt::red);
        auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->zoomBy(2);QTest::qWait(80);
        auto *combo=findVisualItem(main->contentItem(),"brushWriteBlend");QVERIFY(combo);if(fallback)combo->setProperty("windowPopupsAvailable",false);auto *popup=combo->property("popup").value<QObject*>();QVERIFY(popup);
        auto *pen=new QPointingDevice("menu pen",905,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,3);pen->setParent(qApp);
        const auto input=[&](bool press,QPointF global){const auto local=main->mapFromGlobal(global);if(tablet){QTabletEvent event(press?QEvent::TabletPress:QEvent::TabletRelease,pen,local,global,press?.5:0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,press?Qt::LeftButton:Qt::NoButton);QCoreApplication::sendEvent(main,&event);}else{QMouseEvent event(press?QEvent::MouseButtonPress:QEvent::MouseButtonRelease,local,local,global,Qt::LeftButton,press?Qt::LeftButton:Qt::NoButton,Qt::NoModifier);QCoreApplication::sendEvent(main,&event);}};
        const auto render=[&]{client.requestViewport(0,{0,0,960,640},{120,80});return client.frameRevision()>=client.revision() && client.frame().size()==QSize(120,80);};QTRY_VERIFY(render());const auto pixels=client.frame();const auto revision=client.revision();const auto dirty=client.modified();
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,combo->mapToScene({combo->width()/2,combo->height()/2}).toPoint());QQuickItem *option=nullptr;QTRY_VERIFY([&]{for(auto *window:applicationWindows())if(auto *quick=qobject_cast<QQuickWindow*>(window))if((option=findVisualItem(quick->contentItem(),"brushWriteBlendOption:3")))return true;return false;}());
        QVERIFY(option->window()!=main);QTRY_VERIFY(popup->property("visible").toBool());const auto localOption=QPointF(option->width()/2,option->height()/2);const auto canvasPoint=canvas->mapToGlobal(canvas->documentRect().intersected(QRectF(0,0,canvas->width(),canvas->height())).center());option->window()->setPosition((canvasPoint-option->mapToScene(localOption)).toPoint());const auto point=option->mapToGlobal(localOption);QVERIFY(canvas->contains(canvas->mapFromGlobal(point)));QVERIFY(canvas->documentRect().contains(canvas->mapFromGlobal(point)));
        // Model the problematic native delivery: the visible menu covers the
        // document, but the incoming event still names its owner window.
        input(true,point);const bool paintedThrough=client.drawing();input(false,point);QVERIFY(!paintedThrough);QTRY_COMPARE(client.brushWriteBlend(),3);QTRY_VERIFY(!popup->property("visible").toBool());QTest::qWait(30);
        QCOMPARE(client.revision(),revision);QCOMPARE(client.modified(),dirty);QCOMPARE(client.undoDepth(),0);QTRY_VERIFY(render());QCOMPARE(client.frame(),pixels);
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,combo->mapToScene({combo->width()/2,combo->height()/2}).toPoint());QTRY_VERIFY(popup->property("visible").toBool());
        const auto visiblePaper=canvas->documentRect().intersected(QRectF(0,0,canvas->width(),canvas->height())).adjusted(30,30,-30,-30);const auto outside=canvas->mapToGlobal(visiblePaper.bottomRight());QVERIFY(canvas->documentRect().contains(canvas->mapFromGlobal(outside)));input(true,outside);input(false,outside);QTRY_VERIFY(!popup->property("visible").toBool());QVERIFY(!client.drawing());QCOMPARE(client.undoDepth(),0);QCOMPARE(client.revision(),revision);
        // The next independent canvas click must still create exactly one stroke.
        const auto paint=canvas->mapToGlobal(visiblePaper.center());input(true,paint);const bool drawing=client.drawing();input(false,paint);QVERIFY(drawing);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void brushWriteBlendIsStrictGlobalToolStateOutsidePresets() {
        BrushLibrary library;QCOMPARE(library.writeBlend(),0);library.setWriteBlend(3);QCOMPARE(library.writeBlend(),3);
        const auto catalog=library.catalogSnapshot();QVERIFY(!catalog.contains("writeBlend"));QVERIFY(!library.exportPackage().contains("writeBlend"));
        const auto state=library.snapshot();BrushLibrary restored;QVERIFY(restored.restore(state));QCOMPARE(restored.writeBlend(),3);
        QVERIFY(restored.resetPreset());QCOMPARE(restored.writeBlend(),3);QVERIFY(restored.select("round-fine"));QCOMPARE(restored.writeBlend(),3);
        restored.setWriteBlend(27);restored.setWriteBlend(-1);QCOMPARE(restored.writeBlend(),3);
        const auto valid=restored.snapshot();auto invalid=QJsonDocument::fromJson(valid).object();invalid["brushTool"]=QJsonObject{{"writeBlend",3.5}};QVERIFY(!restored.restore(QJsonDocument(invalid).toJson()));QCOMPARE(restored.snapshot(),valid);
        invalid["brushTool"]=QJsonObject{{"writeBlend",27}};QVERIFY(!restored.restore(QJsonDocument(invalid).toJson()));QCOMPARE(restored.snapshot(),valid);
        auto old=QJsonDocument::fromJson(valid).object();old.remove("brushTool");QVERIFY(restored.restore(QJsonDocument(old).toJson()));QCOMPARE(restored.writeBlend(),3);
        QSignalSpy models(&library,&BrushLibrary::presetsChanged);QSignalSpy preview(&library,&BrushLibrary::previewRequested);library.requestPreview(library.selectedId());QCOMPARE(preview.size(),1);const auto first=preview.first()[1].toULongLong();library.setWriteBlend(8);QCOMPARE(models.size(),0);library.acceptPreview(library.selectedId(),first,"captured-normal");QCOMPARE(library.selectedPreview(),QString("captured-normal"));library.requestPreview(library.selectedId());QCOMPARE(preview.size(),1);QCOMPARE(preview.last()[5].toMap().value("writeBlend").toInt(),0);
        BrushLibrary synced;QSignalSpy pending(&synced,&BrushLibrary::previewRequested);synced.requestPreview(synced.selectedId());const auto pendingToken=pending.first()[1].toULongLong();QVERIFY(synced.restore(library.snapshot()));synced.acceptPreview(synced.selectedId(),pendingToken,"shared-normal");QCOMPARE(synced.selectedPreview(),QString("shared-normal"));

    }
    void brushBlendToolbarPreviewCaptureHistoryAndSharedPersistence() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *combo=findVisualItem(window->contentItem(),"brushWriteBlend");QVERIFY(combo);QCOMPARE(combo->property("count").toInt(),27);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,combo->mapToScene({combo->width()/2,combo->height()/2}).toPoint());QQuickItem *option=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(auto *quick=qobject_cast<QQuickWindow*>(w))if((option=findVisualItem(quick->contentItem(),"brushWriteBlendOption:3")))return true;return false;}());QTest::mouseClick(option->window(),Qt::LeftButton,Qt::NoModifier,option->mapToScene({option->width()/2,option->height()/2}).toPoint());QTRY_COMPARE(client.brushWriteBlend(),3);
        auto *library=client.brushLibrary();library->setRadius(6);library->setOpacity(.4);library->setPressureSize(false);library->setPressureFlow(false);const auto beforePreviewRevision=client.revision();const auto beforePreviewHistory=client.undoDepth();const auto beforePreviewModified=client.modified();library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QCOMPARE(client.revision(),beforePreviewRevision);QCOMPARE(client.undoDepth(),beforePreviewHistory);QCOMPARE(client.modified(),beforePreviewModified);
        const auto decode=[](const QString &uri){return QImage::fromData(QByteArray::fromBase64(uri.mid(uri.indexOf(',')+1).toLatin1()));};const auto multiply=decode(library->selectedPreview());QCOMPARE(multiply.size(),QSize(240,60));QCOMPARE(multiply.pixelColor(0,0).alpha(),0);QVERIFY(multiply.pixelColor(120,30).alpha()>0);
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);
        client.activateBucket();QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(QColor(160,160,160));client.setBucketOpacity(1);QVERIFY(client.bucketFill({0,0}));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        const auto rendered=[&]{client.requestViewport(0,{0,0,64,64},{64,64});return client.frameRevision()>=client.revision() && client.frame().size()==QSize(64,64);};QTRY_VERIFY(rendered());const auto original=client.frame();
        client.activateBrush();client.setBrushColor(QColor(128,128,128));InputSample sample;sample.position={16.5,32.5};sample.pressure=1;sample.tool=1;QVERIFY(client.beginStroke(sample));client.setBrushWriteBlend(8);sample.position={48.5,32.5};client.strokeTo(sample);sample.position={16.5,32.5};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(rendered());const auto darkened=client.frame();QVERIFY(darkened.pixelColor(32,32).red()<original.pixelColor(32,32).red());
        client.undo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(rendered());QCOMPARE(client.frame(),original);client.redo();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(rendered());QCOMPARE(client.frame(),darkened);
        sample.position={32.5,32.5};QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(rendered());QVERIFY(client.frame().pixelColor(32,32).red()>darkened.pixelColor(32,32).red());
        library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto screen=decode(library->selectedPreview());QCOMPARE(screen,multiply);library->retryPreview();QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QCOMPARE(decode(library->selectedPreview()),multiply);
        client.setEraser(true);QTRY_VERIFY(!combo->isVisible());sample.position={32.5,32.5};QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),4);QTRY_VERIFY(rendered());QVERIFY(client.frame().pixelColor(32,32).alpha()<255);client.setEraser(false);QTRY_VERIFY(combo->isVisible());QCOMPARE(client.brushWriteBlend(),8);
        const auto screenshot=qEnvironmentVariable("DRAWVERSE_BLEND_PREVIEW");if(!screenshot.isEmpty())QVERIFY(window->grabWindow().save(screenshot));
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);const auto otherId=documents->newDocument(32,32,true);QVERIFY(!otherId.isEmpty());auto *other=documents->client(otherId);QTRY_VERIFY(other && other->ready());QCOMPARE(other->brushWriteBlend(),8);const auto sharedPreview=library->selectedPreview();other->setBrushWriteBlend(9);QCOMPARE(client.brushWriteBlend(),9);QCOMPARE(library->selectedPreview(),sharedPreview);QVERIFY(other->brushLibrary()->resetPreset());QCOMPARE(other->brushWriteBlend(),9);
        const auto firstId=documents->documents().first().toMap().value("id").toString();QSignalSpy firstStopped(&client,&PaintCoreClient::stopped);QVERIFY(documents->closeDocument(firstId,true));QTRY_COMPARE(firstStopped.size(),1);other->setBrushWriteBlend(26);
        const auto saved=[&]{QSettings settings(path,QSettings::IniFormat);return QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object().value("brushTool").toObject().value("writeBlend").toInt(-1)==26;};QTRY_VERIFY(saved());
        QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reloaded(nullptr,path);QTRY_VERIFY(reloaded.ready());QCOMPARE(reloaded.brushWriteBlend(),26);QCOMPARE(warnings,QStringList());
    }
    void resetBaselineSurvivesNativePreferencesReopenAndCapturedStroke() {
        QTemporaryDir temp;const auto path=temp.filePath("preferences.ini");QString copy;
        {
            PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
            library->setRadius(8);library->setPressureSize(false);library->setPressureFlow(false);copy=library->saveCopy("持久基准");QVERIFY(!copy.isEmpty());library->setRadius(3);
            const auto saved=[&]{QSettings prefs(path,QSettings::IniFormat);const auto state=QJsonDocument::fromJson(prefs.value("brushes/state").toByteArray()).object();if(!state.contains("resetState") || state.value("selected").toString()!=copy)return false;for(const auto &row:state.value("presets").toArray())if(row.toObject().value("id").toString()==copy)return row.toObject().value("radius").toDouble()==3.;return false;};QTRY_VERIFY(saved());
            client.shutdown();QTRY_VERIFY(!client.ready());
        }
        PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());auto *library=reopened.brushLibrary();QCOMPARE(library->selectedId(),copy);QCOMPARE(library->radius(),3.);
        reopened.newTransparentDocument(64,64);QTRY_COMPARE(reopened.documentWidth(),64);QTRY_VERIFY(reopened.ready());InputSample p;p.position={16.5,32.5};p.pressure=1;p.tool=1;QVERIFY(reopened.beginStroke(p));
        QVERIFY(library->resetPreset());QCOMPARE(library->radius(),8.);QCOMPARE(reopened.undoDepth(),0);p.position={48.5,32.5};reopened.strokeTo(p);reopened.endStroke();QTRY_COMPARE(reopened.undoDepth(),1);QTRY_VERIFY(!reopened.layerEditBusy());
        const auto rendered=[&]{reopened.requestViewport(0,{0,0,64,64},{64,64});return reopened.frameRevision()>=reopened.revision() && reopened.frame().size()==QSize(64,64);};QTRY_VERIFY(rendered());QCOMPARE(reopened.frame().pixelColor(32,38).alpha(),0);QVERIFY(reopened.frame().pixelColor(32,32).alpha()>0);
        p.position={32.5,32.5};QVERIFY(reopened.beginStroke(p));reopened.endStroke();QTRY_COMPARE(reopened.undoDepth(),2);QTRY_VERIFY(rendered());QVERIFY(reopened.frame().pixelColor(32,38).alpha()>0);
    }
    void resetPresetRestoresFactoryCopyAndImportedBaselinesAtomically() {
        BrushLibrary model;model.setRadius(31);model.setHueJitter(.7);model.setPressureOpacity(true);model.setCursorCenter(true);
        QVERIFY(model.resetPreset());QCOMPARE(model.radius(),12.);QCOMPARE(model.hueJitter(),0.);QVERIFY(!model.pressureOpacity());QVERIFY(model.cursorCenter());
        model.setRadius(23);model.setHardness(.3);auto dual=model.dualBrush();dual["enabled"]=true;dual["diameter"]=41.;QVERIFY(model.setDualBrush(dual));
        const auto copy=model.saveCopy("保存基准");QVERIFY(!copy.isEmpty());const auto original=model.exportPackage();
        model.setRadius(7);model.setHardness(.9);dual["diameter"]=13.;QVERIFY(model.setDualBrush(dual));QVERIFY(model.rename(copy,"新名称"));
        BrushLibrary reopened;QVERIFY(reopened.restore(model.snapshot()));QVERIFY(reopened.resetPreset());QCOMPARE(reopened.radius(),23.);QCOMPARE(reopened.hardness(),.3);QCOMPARE(reopened.dualBrush().value("diameter").toDouble(),41.);QCOMPARE(reopened.selectedName(),QString("新名称"));
        auto native=QJsonDocument::fromJson(reopened.catalogSnapshot()).object();auto rows=native.value("presets").toArray();auto row=rows.last().toObject();row["radius"]=17;rows[rows.size()-1]=row;native["presets"]=rows;
        QVERIFY(reopened.applyNativeCatalog(QJsonDocument(native).toJson()));QCOMPARE(reopened.radius(),17.);QVERIFY(reopened.resetPreset());QCOMPARE(reopened.radius(),23.);
        BrushLibrary importer;QVERIFY(importer.importPackage(original));importer.setRadius(5);QVERIFY(importer.resetPreset());QCOMPARE(importer.radius(),23.);QCOMPARE(importer.dualBrush().value("diameter").toDouble(),41.);
        auto old=QJsonDocument::fromJson(model.snapshot()).object();old.remove("resetState");BrushLibrary migrated;QVERIFY(migrated.restore(QJsonDocument(old).toJson()));migrated.setRadius(9);QVERIFY(migrated.resetPreset());QCOMPARE(migrated.radius(),7.);
        const auto before=reopened.snapshot();auto invalid=QJsonDocument::fromJson(before).object();auto baseline=invalid.value("resetState").toObject();baseline["resetState"]=QJsonObject{};invalid["resetState"]=baseline;QVERIFY(!reopened.restore(QJsonDocument(invalid).toJson()));QCOMPARE(reopened.snapshot(),before);
        QVERIFY(reopened.remove(copy));const auto remaining=QJsonDocument::fromJson(reopened.snapshot()).object().value("resetState").toObject().value("presets").toArray();for(const auto &p:remaining)QVERIFY(p.toObject().value("id").toString()!=copy);
        QVERIFY(!QJsonDocument::fromJson(reopened.catalogSnapshot()).object().contains("resetState"));
    }
    void dualBrushStateRetainsClosedDynamicsAndRejectsInvalidData() {
        BrushLibrary model;
        auto dual=model.dualBrush();
        QCOMPARE(dual.value("model").toString(),QString("path-coverage-v1"));
        QCOMPARE(dual.value("spacing").toDouble(),.25);
        QVERIFY(model.setDualBrush(dual));
        QCOMPARE(QJsonDocument::fromJson(model.snapshot()).object().value("version").toInt(),2);
        dual["diameter"]=37.;dual["spacing"]=.01;
        dual["scatterDynamics"]=QVariantMap{{"enabled",false},{"amount",2.},{"bothAxes",true},{"count",3},{"countJitter",.5},{"seed","ffffffffffffffff"}};
        QVERIFY(model.setDualBrush(dual));
        QVERIFY(model.setDualPressureControl("size",QVariantList{QVariantList{0.,0.},QVariantList{.5,.2},QVariantList{1.,1.}},.1));
        const auto state=model.snapshot();
        QCOMPARE(QJsonDocument::fromJson(state).object().value("version").toInt(),13);
        BrushLibrary restored;QVERIFY(restored.restore(state));QCOMPARE(restored.dualBrush(),model.dualBrush());
        const auto copy=model.saveCopy("第二笔尖副本");QVERIFY(!copy.isEmpty());QCOMPARE(model.dualBrush(),restored.dualBrush());
        auto invalid=model.dualBrush();invalid["spacing"]=.009;QVERIFY(!model.setDualBrush(invalid));
        invalid=model.dualBrush();invalid["colorDynamics"]=QVariantMap{};QVERIFY(!model.setDualBrush(invalid));
        QVERIFY(!model.setDualPressureControl("opacity",{},0.));
        dual=model.dualBrush();dual["model"]="paired-dab-v1";dual["spacing"]=QVariant{};QVERIFY(model.setDualBrush(dual));
        BrushLibrary defaults;auto defaultDual=defaults.dualBrush();auto enabledDefault=defaultDual;enabledDefault["enabled"]=true;QVERIFY(defaults.setDualBrush(enabledDefault));QVERIFY(defaults.setDualBrush(defaultDual));QVERIFY(!defaults.dualBrush().value("enabled").toBool());
        BrushLibrary imported;QVERIFY(imported.importPackage(model.exportPackage()));QCOMPARE(imported.dualBrush(),model.dualBrush());
        QVERIFY(model.setDualResource(model.selectedId(),QString(64,'a')));model.useProceduralDual();
        QCOMPARE(QJsonObject::fromVariantMap(model.dualBrush()),QJsonObject::fromVariantMap(dual));
    }
    void combinedBrushFeaturesPreviewSurvivesStateRoundtrip() {
        QTemporaryDir temp;{QSettings settings(temp.filePath("preview-state.ini"),QSettings::IniFormat);settings.setValue("storage/reserveMiB",0);settings.sync();}PaintCoreClient client(nullptr,temp.filePath("preview-state.ini"));QTRY_VERIFY(client.ready());QTRY_VERIFY(client.brushLibraryReady());auto *library=client.brushLibrary();QVERIFY(library);
        QVERIFY(library->setTransferJitter(QVariantMap{{"model","transfer-jitter-v1"},{"size",.2},{"flow",.2},{"opacity",.2}}));
        auto dual=library->dualBrush();dual["enabled"]=false;dual["diameter"]=30.;QVERIFY(library->setDualBrush(dual));
        auto smoothing=library->smoothing();smoothing["positionMs"]=24.;QVERIFY(library->setSmoothing(smoothing));
        auto wet=library->wetMix();wet["wetness"]=.7;QVERIFY(library->setWetMix(wet));
        auto taper=library->strokeTaper();auto end=taper.value("end").toMap();end["lengthPx"]=24.;taper["end"]=end;QVERIFY(library->setStrokeTaper(taper));
        QSignalSpy requests(library,&BrushLibrary::previewRequested);library->retryPreview();QTRY_COMPARE(requests.size(),1);QTRY_VERIFY2_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),qPrintable(library->previewMessage()),10000);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void initTestCase() {
        QVERIFY(m_preferences.isValid());
        QSettings::setPath(QSettings::IniFormat,QSettings::UserScope,m_preferences.path());
        QSettings::setPath(QSettings::IniFormat,QSettings::SystemScope,m_preferences.path());
    }
    void init() {
        QSettings settings(QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse");
        QVERIFY(settings.fileName().startsWith(m_preferences.path()+"/"));
        settings.clear();settings.sync();QCOMPARE(settings.status(),QSettings::NoError);
    }
    void foldedNeighboursKeepTheirSizes_data() {
        QTest::addColumn<int>("toolWidth");for(int width:{36,55,72})QTest::newRow(qPrintable(QString::number(width)))<<width;
    }
    void foldedNeighboursKeepTheirSizes() {
        QFETCH(int,toolWidth);QTemporaryDir temp;const auto path=temp.filePath("layout.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());QTRY_VERIFY(client.ready());
        const auto brush=workspace.groupForPanel("brush");workspace.detachGroup(brush);workspace.setToolStripWidth(toolWidth);QVERIFY(workspace.dockToolStrip("drawverse-tools-v1","floating",brush,"left"));
        QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+brush)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());floating->resize(420,650);QTest::qWait(50);
        auto *tools=findVisualItem(floating->contentItem(),"dockTile:__toolstrip");QVERIFY(tools);QTRY_COMPARE(tools->width(),qreal(toolWidth));const auto expanded=floating->geometry();
        workspace.setColumnCollapsed(brush,true);QTRY_COMPARE(tools->width(),qreal(toolWidth));QTRY_COMPARE(floating->width(),toolWidth+31);QCOMPARE(floating->position(),expanded.topLeft());QCOMPARE(floating->height(),expanded.height());
        const auto preview=qEnvironmentVariable("DRAWVERSE_FOLD_SIZE_PREVIEW");if(!preview.isEmpty()){QTest::qWait(50);QVERIFY(floating->grabWindow().save(preview+"."+QString::number(toolWidth)+".png"));}
        const auto adjusted=toolWidth==36?72:36;workspace.setToolStripWidth(adjusted);QTRY_COMPARE(tools->width(),qreal(adjusted));QTRY_COMPARE(floating->width(),adjusted+31);workspace.setToolStripWidth(toolWidth);QTRY_COMPARE(floating->width(),toolWidth+31);
        workspace.saveLayout();WorkspaceManager restored(path);const auto saved=restored.floatingWindows().first().toMap();QCOMPARE(saved.value("width").toInt(),toolWidth+31);restored.setColumnCollapsed(brush,false);QCOMPARE(restored.floatingWindows().first().toMap().value("width").toInt(),expanded.width());
        QSettings settings(path,QSettings::IniFormat);auto legacy=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();legacy.remove("retainedWidths");auto windows=legacy.value("windows").toObject();auto oldWindow=windows.value(brush).toObject();oldWindow.insert("width",expanded.width());windows.insert(brush,oldWindow);legacy.insert("windows",windows);settings.setValue("workspace",QJsonDocument(legacy).toJson());settings.sync();WorkspaceManager migrated(path);QCOMPARE(migrated.floatingWindows().first().toMap().value("width").toInt(),toolWidth+31);migrated.setColumnCollapsed(brush,false);QCOMPARE(migrated.floatingWindows().first().toMap().value("width").toInt(),std::min(expanded.width(),floating->screen()->availableGeometry().width()));
        for(int i=0;i<3;++i){workspace.setColumnCollapsed(brush,false);QTRY_COMPARE(floating->geometry(),expanded);QTRY_COMPARE(tools->width(),qreal(toolWidth));workspace.setColumnCollapsed(brush,true);QTRY_COMPARE(floating->width(),toolWidth+31);}
        workspace.setColumnCollapsed(brush,false);QTRY_COMPARE(floating->geometry(),expanded);QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void independentFoldedColumnsKeepExpandedNeighboursAndRestore() {
        QTemporaryDir temp;const auto path=temp.filePath("layout.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());QTRY_VERIFY(client.ready());
        const auto brush=workspace.groupForPanel("brush"),color=workspace.groupForPanel("color");workspace.detachGroup(brush);workspace.detachGroup(color);
        const auto payload=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",color},{"whole",true}}).toJson());QVERIFY(workspace.dockPayload(payload,"floating",brush,"column-right"));
        QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+brush)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());floating->resize(610,550);QTest::qWait(50);
        auto *a=findVisualItem(floating->contentItem(),"dockTile:"+brush),*b=findVisualItem(floating->contentItem(),"dockTile:"+color);QVERIFY(a && b);QTRY_COMPARE(floating->size(),QSize(610,550));QTRY_COMPARE(a->height(),qreal(548));QTRY_COMPARE(b->height(),qreal(548));QTRY_VERIFY(std::abs(a->width()+b->width()-607)<1);const auto original=floating->geometry();const auto as=a->size(),bs=b->size();QCOMPARE(workspace.columnGroups(brush),QStringList{brush});
        // Native host sizes are integral; split ratios can give fractional logical pixels.
        const auto sameSize=[](QSizeF x,QSizeF y){return std::abs(x.width()-y.width())<1 && std::abs(x.height()-y.height())<1;};
        workspace.setColumnCollapsed(brush,true);QTRY_VERIFY2(sameSize(b->size(),bs),qPrintable(QString("neighbour %1x%2 expected %3x%4; host %5x%6").arg(b->width()).arg(b->height()).arg(bs.width()).arg(bs.height()).arg(floating->width()).arg(floating->height())));QTRY_COMPARE(floating->width(),qRound(bs.width())+31);QCOMPARE(floating->height(),original.height());
        workspace.setColumnCollapsed(color,true);QTRY_COMPARE(floating->width(),59);workspace.saveLayout();WorkspaceManager restored(path);restored.setColumnCollapsed(brush,false);restored.setColumnCollapsed(color,false);QCOMPARE(restored.floatingWindows().first().toMap().value("width").toInt(),original.width());
        workspace.setColumnCollapsed(brush,false);QTRY_VERIFY(sameSize(a->size(),as));workspace.setColumnCollapsed(color,false);QTRY_COMPARE(floating->geometry(),original);QTRY_VERIFY(sameSize(b->size(),bs));
        workspace.setColumnCollapsed(brush,true);QTRY_VERIFY(sameSize(b->size(),bs));const auto expandedHeight=b->height();floating->resize(floating->width()+30,floating->height()+40);QTRY_VERIFY(b->width()>bs.width());QTRY_VERIFY(b->height()>expandedHeight);QTRY_COMPARE(b->height(),qreal(floating->height()-2));const auto manual=b->size();workspace.setColumnCollapsed(brush,false);QTRY_VERIFY(sameSize(b->size(),manual));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void categorizedMenusHoverIntoCompactNativeSubmenus_data() {
        QTest::addColumn<bool>("fallback");QTest::newRow("window-popups")<<false;QTest::newRow("side-window-fallback")<<true;
    }
    void categorizedMenusHoverIntoCompactNativeSubmenus() {
        QFETCH(bool,fallback);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));
        QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto area=main->screen()->availableGeometry();main->setPosition(area.topLeft());main->resize(std::min(1100,area.width()),std::min(700,area.height()));
        const auto hover=[&](QObject *menu,const QString &text) -> QQuickItem* {
            auto *content=qobject_cast<QQuickItem*>(menu->property("contentItem").value<QObject*>());if(!content || !content->window())return nullptr;
            auto *row=findVisualItem(content,"glassMenuItem:"+text);if(!row)return nullptr;
            QTest::qWait(30);
            // On a narrow high-DPI screen a submenu may flip left and cover its parent's
            // center. Hover the exposed trailing edge, where the category arrow lives.
            QTest::mouseMove(content->window(),row->mapToScene({row->width()-12,row->height()/2}).toPoint());return row;
        };
        auto *image=main->findChild<QObject*>("imageMenu"),*adjust=main->findChild<QObject*>("imageAdjustSubmenu"),*rotate=main->findChild<QObject*>("imageRotateSubmenu");QVERIFY(image && adjust && rotate);
        if(fallback)for(const auto &name:{"imageMenu","imageAdjustSubmenu","imageRotateSubmenu","filterMenu","filterBlurSubmenu"})main->findChild<QObject*>(name)->setProperty("windowPopupsAvailable",false);
        auto *entry=findVisualItem(main->contentItem(),"menuEntry:图像");QVERIFY(entry);
        QTRY_COMPARE(main->position(),area.topLeft());QTRY_VERIFY(main->isExposed());QTest::qWait(50);
        if(QGuiApplication::platformName()!="offscreen"){QCursor::setPos(entry->mapToGlobal({entry->width()/2,14}).toPoint());QTest::qWait(30);}
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,entry->mapToScene({entry->width()/2,14}).toPoint());QTRY_VERIFY(image->property("opened").toBool());auto *row=hover(image,"调整");QVERIFY(row);QTRY_VERIFY(row->property("hovered").toBool());QTRY_VERIFY(adjust->property("opened").toBool());
        auto *arrow=qobject_cast<QQuickItem*>(row->property("arrow").value<QObject*>());QVERIFY(arrow);QCOMPARE(arrow->width(),qreal(6));QCOMPARE(arrow->height(),qreal(8));QVERIFY(row->height()<30);
        auto *parentContent=qobject_cast<QQuickItem*>(image->property("contentItem").value<QObject*>()),*childContent=qobject_cast<QQuickItem*>(adjust->property("contentItem").value<QObject*>());QVERIFY(parentContent && childContent);QVERIFY(childContent->window()!=parentContent->window());
        if(fallback){QVERIFY(image->property("sideWindow").value<QObject*>());QVERIFY(adjust->property("sideWindow").value<QObject*>());}
        const auto preview=qEnvironmentVariable("DRAWVERSE_MENU_LAYER_PREVIEW");if(!preview.isEmpty() && !fallback){QTest::qWait(50);QVERIFY(childContent->window()->grabWindow().save(preview+".menu.png"));}
        const auto childGeometry=childContent->window()->geometry();QVERIFY(area.adjusted(-1,-1,1,1).contains(childGeometry));
        // Cross the first row's edge gradually, with the menu bar actively owning the chain.
        const auto start=row->mapToGlobal({row->width()-12,row->height()/2}).toPoint();
        const auto finish=childContent->mapToGlobal({20,10}).toPoint();
        QPointer<QQuickWindow> parentHost=parentContent->window(),childHost=childContent->window();
        const int distance=std::max(std::abs(finish.x()-start.x()),std::abs(finish.y()-start.y()));
        for(int step=1;step<=distance;++step) {
            const auto global=start+(finish-start)*qreal(step)/distance;
            auto *host=childGeometry.contains(global)?childHost.data():parentHost.data();QVERIFY2(host,"menu host vanished during first-row traversal");
            QTest::mouseMove(host,host->mapFromGlobal(global),1);
            QVERIFY2(image->property("opened").toBool(),qPrintable(QString("first-row move %1/%2 at %3,%4 (start %5,%6 child %7,%8) closed parent").arg(step).arg(distance).arg(global.x()).arg(global.y()).arg(start.x()).arg(start.y()).arg(childGeometry.x()).arg(childGeometry.y())));
            QVERIFY(!main->findChild<QObject*>("fileMenuPopup")->property("visible").toBool());
        }
        if(QGuiApplication::platformName()!="offscreen") {
            // Exercise real native hover dispatch, including popup mouse grabs.
            QCursor::setPos(start);QTest::qWait(30);
            for(int step=1;step<=distance;++step) {
                QCursor::setPos(start+(finish-start)*qreal(step)/distance);QTest::qWait(5);
                QVERIFY2(image->property("opened").toBool(),qPrintable(QString("native first-row move %1/%2 closed parent").arg(step).arg(distance)));
                QVERIFY(!main->findChild<QObject*>("fileMenuPopup")->property("visible").toBool());
            }
        }
        QTest::mouseMove(childContent->window(),childContent->mapToScene({20,10}).toPoint());QVERIFY(image->property("opened").toBool());QVERIFY(adjust->property("opened").toBool());
        auto *rotateRow=hover(image,"画布旋转");QVERIFY(rotateRow);QTRY_VERIFY(rotateRow->property("hovered").toBool());QTRY_VERIFY(rotate->property("opened").toBool());QTRY_VERIFY(!adjust->property("visible").toBool());
        QVERIFY(!main->findChild<QObject*>("canvasRotateCwAction")->property("enabled").toBool());
        auto *fileEntry=findVisualItem(main->contentItem(),"menuEntry:文件");QVERIFY(fileEntry);
        auto *rotateContent=qobject_cast<QQuickItem*>(rotate->property("contentItem").value<QObject*>());QVERIFY(rotateContent && rotateContent->window());
        QTest::mouseMove(rotateContent->window(),rotateContent->window()->mapFromGlobal(fileEntry->mapToGlobal({fileEntry->width()/2,14}).toPoint()));
        auto *file=main->findChild<QObject*>("fileMenuPopup");QTRY_VERIFY(file->property("opened").toBool());QTRY_VERIFY(!image->property("visible").toBool());QTRY_VERIFY(!rotate->property("visible").toBool());
        QVERIFY(QMetaObject::invokeMethod(file,"close"));
        auto *filter=main->findChild<QObject*>("filterMenu"),*blur=main->findChild<QObject*>("filterBlurSubmenu");QVERIFY(filter && blur);
        QVERIFY(QMetaObject::invokeMethod(filter,"open"));QTRY_VERIFY(filter->property("opened").toBool());QVERIFY(hover(filter,"模糊"));QTRY_VERIFY(blur->property("opened").toBool());
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()){auto *content=qobject_cast<QQuickItem*>(blur->property("contentItem").value<QObject*>());QVERIFY(GetWindowLongPtr(reinterpret_cast<HWND>(content->window()->winId()),GWL_EXSTYLE)&WS_EX_TOPMOST);}
#endif
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,QPoint(main->width()*.8,main->height()*.8));QTRY_VERIFY(!filter->property("visible").toBool());QTRY_VERIFY(!blur->property("visible").toBool());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void selectedLayersDeleteDragAndGroupChoicesAreAtomic() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));
        QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        for(int i=0;i<3;++i){client.addLayer(QString("Layer %1").arg(i));QTRY_COMPARE(client.layers().size(),i+2);QTRY_VERIFY(!client.layerEditBusy());}
        client.selectLayers(2,0);QTRY_VERIFY(!client.layerEditBusy());client.selectLayers(4,Qt::ControlModifier);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectedLayers().size(),2);
        const auto payload=client.layerDragPayload(2);QCOMPARE(client.layerDragIds(payload).size(),2);QCOMPARE(client.layerDragIds("0:2,4").size(),0);QCOMPARE(client.layerDragIds(QString::number(client.generation())+":2,2").size(),0);
        const int depth=client.undoDepth();QQuickItem *target=nullptr;QTRY_VERIFY((target=findVisualItem(main->contentItem(),"layerRow:1")));
        const auto point=target->mapToScene({target->width()*.72,target->height()-3}).toPoint();QMimeData mime;mime.setData("application/x-drawverse-layer",payload.toUtf8());
        QDragEnterEvent enter(point,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(main,&enter);
        QDragMoveEvent move(point,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(main,&move);
        QDropEvent drop(point,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(main,&drop);QVERIFY(drop.isAccepted());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);
        QCOMPARE(client.layers()[2].toMap().value("id").toULongLong(),quint64(4));QCOMPARE(client.layers()[3].toMap().value("id").toULongLong(),quint64(2));QCOMPARE(client.selectedLayers().size(),2);
        auto *trash=findVisualItem(main->contentItem(),"deleteLayer");QTRY_VERIFY((trash=findVisualItem(main->contentItem(),"deleteLayer")));QVERIFY(QMetaObject::invokeMethod(trash,"clicked"));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+2);
        client.undo();QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());
        client.selectLayers(2,0);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.groupLayer(2,"Group"));QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();QCOMPARE(client.selectedLayers(),QVariantList{QVariant::fromValue(group)});
        auto *dialog=main->findChild<QObject*>("groupDeleteDialog");QVERIFY(dialog);const auto revision=client.revision();
        client.requestSelectedLayerRemoval();QTRY_VERIFY(dialog->property("opened").toBool());
        const auto preview=qEnvironmentVariable("DRAWVERSE_MENU_LAYER_PREVIEW");if(!preview.isEmpty()){QTest::qWait(50);QVERIFY(main->grabWindow().save(preview+".group.png"));}
        QVERIFY(QMetaObject::invokeMethod(main->findChild<QObject*>("deleteGroupCancel"),"clicked"));QTRY_VERIFY(!dialog->property("visible").toBool());QCOMPARE(client.revision(),revision);
        client.requestSelectedLayerRemoval();QTRY_VERIFY(dialog->property("opened").toBool());QVERIFY(QMetaObject::invokeMethod(main->findChild<QObject*>("deleteGroupKeepChildren"),"clicked"));QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());
        const auto kept=client.layers();QVERIFY(std::any_of(kept.begin(),kept.end(),[](const QVariant &v){return v.toMap().value("id").toULongLong()==2;}));
        client.undo();QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());client.selectLayers(group,0);QTRY_VERIFY(!client.layerEditBusy());
        client.requestSelectedLayerRemoval();QTRY_VERIFY(dialog->property("opened").toBool());QVERIFY(QMetaObject::invokeMethod(main->findChild<QObject*>("deleteGroupSubtree"),"clicked"));QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());
        client.undo();QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());client.selectLayers(group,0);QTRY_VERIFY(!client.layerEditBusy());client.requestSelectedLayerRemoval();
        client.newTransparentDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());QVERIFY(!client.confirmSelectedLayerRemoval(2));QVERIFY(QMetaObject::invokeMethod(dialog,"close"));QCOMPARE(client.layers().size(),1);
        QVERIFY(!client.acceptLayerDrop(payload,0,0));QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void newGroupContainsWholeSelectionFromBothUiEntrypoints() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        for(int i=0;i<3;++i){client.addLayer(QString("L%1").arg(i));QTRY_COMPARE(client.layers().size(),i+2);QTRY_VERIFY(!client.layerEditBusy());}
        for(bool menu:{false,true}) {
            client.selectLayers(2,0);QTRY_VERIFY(!client.layerEditBusy());client.selectLayers(4,Qt::ControlModifier);QTRY_VERIFY(!client.layerEditBusy());
            QCOMPARE(client.selectedLayers().size(),2);const auto depth=client.undoDepth();
            if(menu) {auto *action=main->findChild<QObject*>("newLayerGroupAction");QVERIFY(action);QVERIFY(QMetaObject::invokeMethod(action,"trigger"));}
            else {QQuickItem *button=nullptr;QTRY_VERIFY((button=findVisualItem(main->contentItem(),"groupLayer")));QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,button->mapToScene({button->width()/2,button->height()/2}).toPoint());}
            QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);
            const auto group=client.activeLayer();QCOMPARE(client.selectedLayers(),QVariantList{QVariant::fromValue(group)});
            for(const auto &value:client.layers()) {const auto layer=value.toMap();const auto id=layer.value("id").toULongLong();if(id==2 || id==4)QCOMPARE(layer.value("parent").toULongLong(),group);if(id==1 || id==3)QCOMPARE(layer.value("parent").toULongLong(),quint64(0));}
            client.undo();QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());
            for(const auto &value:client.layers())QCOMPARE(value.toMap().value("parent").toULongLong(),quint64(0));
            client.redo();QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());client.undo();QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());
        }
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void firstLayersPanelOpenIncubatesWithoutBlockingTheEventLoop() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(client.ready());
        for(int i=0;i<79;++i){client.addLayer(QString("Layer %1").arg(i));QTRY_COMPARE(client.layers().size(),i+2);QTRY_VERIFY(!client.layerEditBusy());}
        WorkspaceManager workspace(temp.filePath("layout.ini"));workspace.hidePanel(workspace.groupForPanel("layers"),"layers");workspace.saveLayout();QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTest::qWait(100);
        QVERIFY(!findVisualItem(main->contentItem(),"layersPanel"));QElapsedTimer timer;timer.start();qint64 previous=0,longest=0;int ticks=0;QTimer heartbeat;heartbeat.setInterval(5);
        connect(&heartbeat,&QTimer::timeout,this,[&]{const auto now=timer.elapsed();longest=std::max(longest,now-previous);previous=now;++ticks;});heartbeat.start();
        workspace.showPanel("layers");const auto dispatch=timer.elapsed();QVERIFY2(dispatch<100,qPrintable(QString("opening dispatch blocked for %1 ms").arg(dispatch)));
        QTRY_VERIFY(findVisualItem(main->contentItem(),"layerRow:80"));QTest::qWait(100);heartbeat.stop();
        auto *loader=findVisualItem(main->contentItem(),"panelContentLoader:layers");QVERIFY(loader);QVERIFY(loader->property("asynchronous").toBool());QVERIFY(ticks>1);QVERIFY2(longest<200,qPrintable(QString("opening blocked heartbeat for %1 ms").arg(longest)));
        int instantiated=0;std::function<void(QQuickItem*)> count=[&](QQuickItem *item){if(item->objectName().startsWith("layerRow:"))++instantiated;for(auto *child:item->childItems())count(child);};count(main->contentItem());QVERIFY(instantiated>0 && instantiated<40);
        qInfo()<<"first layers panel dispatch"<<dispatch<<"ms; longest heartbeat gap"<<longest<<"ms; rows"<<instantiated;
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void automaticClippingReleaseKeepsMetadataAndHistoryLive() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(client.ready());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        client.addLayer("Clip");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto clip=client.activeLayer();
        QVERIFY(client.setLayerClipping(clip,true));QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(client.dropLayer(1,clip,1));QTRY_VERIFY(!client.layerEditBusy());
        QCOMPARE(client.historyEntries().last().toMap().value("title").toString(),QString("释放剪贴蒙版"));
        QCOMPARE(client.historyEntries().last().toMap().value("icon").toString(),QString("layers"));
        const auto released=[&]{for(const auto &value:client.layers()){const auto layer=value.toMap();if(layer.value("id").toULongLong()==clip)return !layer.value("clipped").toBool();}return false;};
        QVERIFY(released());client.undo();QTRY_VERIFY(!client.layerEditBusy());QVERIFY(!released());client.redo();QTRY_VERIFY(!client.layerEditBusy());QVERIFY(released());
        client.addLayer("Still responsive");QTRY_COMPARE(client.layers().size(),3);QVERIFY(client.lastError().isEmpty());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void layerThumbnailsUpdateWithoutRebuildingRows() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        for(int i=0;i<7;++i){client.addLayer(QString("Preview %1").arg(i));QTRY_COMPARE(client.layers().size(),i+2);QTRY_VERIFY(!client.layerEditBusy());}
        QPointer<QQuickItem> row=findVisualItem(main->contentItem(),"layerRow:"+QString::number(client.activeLayer()));QVERIFY(row);
        QSignalSpy structure(&client,&PaintCoreClient::layersChanged),thumbnails(&client,&PaintCoreClient::layerThumbnailsChanged);
        QTRY_VERIFY_WITH_TIMEOUT(thumbnails.size()>=4,10000);
        QCOMPARE(structure.size(),0);QVERIFY(row);QCOMPARE(findVisualItem(main->contentItem(),row->objectName()),row.data());
        QTRY_COMPARE_WITH_TIMEOUT(client.layerThumbnails().size(),8,10000);const auto cached=client.layerThumbnails();
        QElapsedTimer reopen;reopen.start();const auto group=workspace.groupForPanel("layers");workspace.setActive(group,"history");QTest::qWait(50);workspace.setActive(group,"layers");
        QTRY_VERIFY(findVisualItem(main->contentItem(),"layerRow:"+QString::number(client.activeLayer())));QVERIFY(reopen.elapsed()<1000);QCOMPARE(client.layerThumbnails(),cached);
        const auto generation=client.generation();client.newTransparentDocument(32,32);QTRY_VERIFY(client.generation()>generation);QTRY_VERIFY(client.ready());QVERIFY(client.layerThumbnails().size()<=1);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void closingDockedCategoryKeepsOtherHeightsAndFullToolColumn() {
        QTemporaryDir temp;const auto path=temp.filePath("layout.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.groupForPanel("color"),layers=workspace.groupForPanel("layers");QVERIFY(color!=layers);
        QVERIFY(workspace.dockToolStrip("drawverse-tools-v1","right",layers,"left"));QTest::qWait(60);
        auto *remaining=findVisualItem(main->contentItem(),"dockTile:"+layers),*tools=findVisualItem(main->contentItem(),"dockTile:__toolstrip");QVERIFY(remaining && tools);const auto height=remaining->height(),toolHeight=tools->height();
        workspace.hidePanel(color);QTRY_VERIFY(!findVisualItem(main->contentItem(),"dockTile:"+color));QCOMPARE(remaining->height(),height);QCOMPARE(tools->height(),toolHeight);QVERIFY(tools->height()>remaining->height()+50);
        auto *background=findVisualItem(main->contentItem(),"__panelBackground:"+layers);QVERIFY(background);QVERIFY(background->height()>remaining->height());
        const auto shot=main->grabWindow();const auto bottom=background->mapToScene({background->width()/2,background->height()-2});const auto scale=shot.width()/qreal(main->width());QCOMPARE(shot.pixelColor(qRound(bottom.x()*scale),qRound(bottom.y()*scale)),QColor("#1c1e21"));
        workspace.saveLayout();WorkspaceManager restored(path);const auto *area=background->parentItem();bool found=false;
        for(const auto &value:restored.layoutItems("main",qRound(area->width()),qRound(area->height()))){const auto item=value.toMap();if(item.value("id")==layers){QCOMPARE(item.value("rect").toRectF().height(),height);found=true;}}QVERIFY(found);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void historyUsesRealToolLabelsAndHidesEvictedInitialState() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());for(auto *canvas:window->findChildren<CanvasItem*>())canvas->setClient(nullptr);
        QTRY_COMPARE(client.historyEntries().size(),1);QCOMPARE(client.historyEntries().first().toMap().value("title").toString(),QString("初始状态"));
        client.setBrushRadius(1);InputSample sample;sample.position={16,16};sample.pressure=1;
        for(int i=0;i<150;++i) {
            const auto previous=client.revision();client.setBrushColor(i%2?Qt::black:Qt::white);QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_VERIFY(client.revision()>previous);QTRY_VERIFY(!client.layerEditBusy());
        }
        QVERIFY(client.undoDepth()>0 && client.undoDepth()<=100);QCOMPARE(client.redoDepth(),0);
        QTRY_COMPARE(client.historyEntries().size(),client.undoDepth());
        for(const auto &value:client.historyEntries()){const auto record=value.toMap();QCOMPARE(record.value("title").toString(),QString("画笔"));QCOMPARE(record.value("icon").toString(),QString("brush"));QVERIFY(record.value("depth").toInt()>0);}
        client.selectAll();QTRY_COMPARE(client.historyEntries().last().toMap().value("title").toString(),QString("全选"));
        client.setEraser(true);QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.historyEntries().last().toMap().value("title").toString(),QString("橡皮擦"));QCOMPARE(client.historyEntries().last().toMap().value("icon").toString(),QString("eraser"));
        const auto records=client.historyEntries();client.undo();QTRY_COMPARE(client.redoDepth(),1);QCOMPARE(client.historyEntries(),records);client.redo();QTRY_COMPARE(client.redoDepth(),0);QCOMPARE(client.historyEntries(),records);
        const auto group=workspace.groupForPanel("history");workspace.setActive(group,"history");QQuickItem *title=nullptr;QTRY_VERIFY((title=findVisualItem(window->contentItem(),"historyTitle:"+QString::number(client.undoDepth()))));QCOMPARE(title->property("text").toString(),QString("橡皮擦"));
        auto *icon=findVisualItem(window->contentItem(),"historyIcon:"+QString::number(client.undoDepth()));QVERIFY(icon);QCOMPARE(icon->property("name").toString(),QString("eraser"));QVERIFY(!findVisualItem(window->contentItem(),"historyEntry:0"));
        QQuickItem *previous=nullptr;QTRY_VERIFY((previous=findVisualItem(window->contentItem(),"historyEntry:"+QString::number(client.undoDepth()-1))));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,previous->mapToScene({20,12}).toPoint());QTRY_COMPARE(client.redoDepth(),1);QCOMPARE(client.historyEntries(),records);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void liveTabletPoseTracksTiltRotationPressureAndCapturedStroke() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors)warnings.append(error.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);QVERIFY(QTest::qWaitForWindowExposed(window));QTRY_VERIFY(canvas->width()>100 && canvas->height()>100);canvas->actualSize();auto *library=client.brushLibrary();library->setRadius(8);library->setRoundness(.5);library->setAngleControl("pen-rotation");library->setTiltRoundness(true);library->setMinimumRoundness(.2);
        auto *pen=new QPointingDevice("Pose pen",898,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure|QInputDevice::Capability::XTilt|QInputDevice::Capability::Rotation,1,2);pen->setParent(qApp);const QPointF start=canvas->documentRect().center();window->requestActivate();QTRY_VERIFY(window->isActive());QTest::mouseMove(window,canvas->mapToScene(start).toPoint());QTest::qWait(50);
        QTabletEvent hover(QEvent::TabletMove,pen,start,canvas->mapToGlobal(start),0,60,0,0,90,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(canvas,&hover);const bool initialVisible=canvas->brushCursorVisible();if(!initialVisible)QTest::keyClick(window,Qt::Key_CapsLock);QTRY_VERIFY2(std::abs(canvas->brushCursorAngle()-90)<.001,qPrintable(([&]{InputSample probe;probe.position=canvas->documentPoint(start);probe.rotation=90;probe.tiltX=60;probe.capabilities=7;probe.tool=1;const auto native=client.brushCursorPose(probe,{},false,false);return QString("angle=%1 roundness=%2 visible=%3 width=%4 directReady=%5 directAngle=%6 control=%7").arg(canvas->brushCursorAngle()).arg(canvas->brushCursorRoundness()).arg(canvas->brushCursorVisible()).arg(canvas->brushCursorRect().width()).arg(native.ready).arg(native.angle).arg(library->angleControl());})()));QTRY_VERIFY(std::abs(canvas->brushCursorRoundness()-.25)<.001);QHoverEvent synthesizedHover(QEvent::HoverMove,start,canvas->mapToGlobal(start),start,Qt::NoModifier,pen);QCoreApplication::sendEvent(canvas,&synthesizedHover);QCOMPARE(canvas->brushCursorAngle(),90.);QCOMPARE(canvas->brushCursorRoundness(),.25);QCOMPARE(canvas->brushCursorRect().width(),16.);auto *outline=findVisualItem(window->contentItem(),"brushCursorOutline");QVERIFY(outline);QTRY_VERIFY(std::abs(outline->property("rotation").toDouble()-90)<.001);
        QTabletEvent press(QEvent::TabletPress,pen,start,canvas->mapToGlobal(start),.5,60,0,0,90,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(canvas,&press);QVERIFY(client.drawing());QTRY_VERIFY(std::abs(canvas->brushCursorRect().width()-8)<.001);
        library->setAngleControl("fixed");library->setRoundness(1);library->setRadius(20);const auto moved=start+QPointF(10,0);QTabletEvent move(QEvent::TabletMove,pen,moved,canvas->mapToGlobal(moved),.75,60,0,0,45,0,Qt::NoModifier,Qt::NoButton,Qt::LeftButton);QCoreApplication::sendEvent(window,&move);QTRY_VERIFY(std::abs(canvas->brushCursorAngle()-45)<.001);QTRY_VERIFY(std::abs(canvas->brushCursorRoundness()-.25)<.001);QTRY_VERIFY(std::abs(canvas->brushCursorRect().width()-12)<.001);
        QTabletEvent release(QEvent::TabletRelease,pen,moved,canvas->mapToGlobal(moved),0,60,0,0,45,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(canvas,&release);QVERIFY(!client.drawing());QTRY_VERIFY(std::abs(canvas->brushCursorRect().width()-40)<.001);QTRY_VERIFY(std::abs(canvas->brushCursorAngle())<.001);library->setAngleControl("direction");library->setTiltRoundness(false);library->setRoundness(.2);auto *mouse=new QPointingDevice("Pose mouse",900,QInputDevice::DeviceType::Mouse,QPointingDevice::PointerType::Generic,QInputDevice::Capability::Position,1,3);mouse->setParent(qApp);const QPointF mouseStart=canvas->mapFromScene(canvas->mapToScene(start).toPoint());QTest::mouseMove(window,canvas->mapToScene(mouseStart).toPoint());QTest::qWait(50);QHoverEvent first(QEvent::HoverMove,mouseStart,canvas->mapToGlobal(mouseStart),moved,Qt::NoModifier,mouse);QCoreApplication::sendEvent(canvas,&first);QTest::qWait(50);const auto lower=mouseStart+QPointF(0,10);QTest::mouseMove(window,canvas->mapToScene(lower).toPoint());QTest::qWait(50);QHoverEvent second(QEvent::HoverMove,lower,canvas->mapToGlobal(lower),mouseStart,Qt::NoModifier,mouse);QCoreApplication::sendEvent(canvas,&first);QCoreApplication::sendEvent(canvas,&second);QTRY_VERIFY2(std::abs(canvas->brushCursorAngle()-90)<.001,qPrintable(QString("mouse angle=%1 position=%2,%3 expected=%4,%5 control=%6").arg(canvas->brushCursorAngle()).arg(canvas->cursorPosition().x()).arg(canvas->cursorPosition().y()).arg(lower.x()).arg(lower.y()).arg(library->angleControl())));
        if(!initialVisible)QTest::keyClick(window,Qt::Key_CapsLock);QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void tabletDirectHoverEntryAndWindowCursorRestore() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));
        QQuickWindow window;window.resize(640,480);window.setCursor(Qt::WaitCursor);
        auto *canvas=new CanvasItem(window.contentItem());canvas->setSize({500,400});canvas->setPosition({40,30});canvas->setClient(&client);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));QTRY_VERIFY(client.ready());canvas->actualSize();
        // Qt may resolve its initial item cursor while exposing the native window.
        window.setCursor(Qt::WaitCursor);
        auto *pen=new QPointingDevice("Direct hover pen",901,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,2);pen->setParent(qApp);
        const QPointF entry(120,150),next(170,190);
        // Reset the restoration baseline for each acquisition; Quick may resolve a different normal cursor after the previous handoff at high DPI.
        const auto hover=[&](QEvent::Type type,QPointF point){if(type==QEvent::HoverEnter)window.setCursor(Qt::WaitCursor);QHoverEvent event(type,point,canvas->mapToGlobal(point),entry,Qt::NoModifier,pen);QCoreApplication::sendEvent(canvas,&event);};
        hover(QEvent::HoverEnter,entry);const bool initiallyVisible=canvas->brushCursorVisible();if(!initiallyVisible)QTest::keyClick(&window,Qt::Key_CapsLock);
        hover(QEvent::HoverEnter,entry);QVERIFY(canvas->brushCursorVisible());QCOMPARE(canvas->cursorPosition(),entry);QCOMPARE(window.cursor().shape(),Qt::BlankCursor);
        hover(QEvent::HoverMove,next);QCOMPARE(canvas->brushCursorRect().center(),next);QCOMPARE(window.cursor().shape(),Qt::BlankCursor);QCOMPARE(client.undoDepth(),0);
        const auto start=canvas->documentRect().center(),end=start+QPointF(10,8);
        QTabletEvent press(QEvent::TabletPress,pen,start,canvas->mapToGlobal(start),.5,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(canvas,&press);QVERIFY(client.drawing());
        hover(QEvent::HoverMove,entry);QCOMPARE(canvas->cursorPosition(),start);
        QTabletEvent release(QEvent::TabletRelease,pen,end,canvas->mapToGlobal(end),0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(&window,&release);QVERIFY(!client.drawing());
        hover(QEvent::HoverMove,next);QCOMPARE(canvas->brushCursorRect().center(),next);QCOMPARE(window.cursor().shape(),Qt::BlankCursor);
        // Leaving restores the window cursor without requiring a real mouse move.
        hover(QEvent::HoverLeave,next);QVERIFY(!canvas->brushCursorVisible());QVERIFY(window.cursor().shape()!=Qt::BlankCursor);
        // Observe restoration before Quick resolves the new mouse target and legitimately changes the window cursor again.
        QObject mouseReceiver;hover(QEvent::HoverEnter,entry);QMouseEvent realMouse(QEvent::MouseMove,next,canvas->mapToGlobal(next),Qt::NoButton,Qt::NoButton,Qt::NoModifier);QCoreApplication::sendEvent(&mouseReceiver,&realMouse);QCOMPARE(window.cursor().shape(),Qt::WaitCursor);QHoverEvent mouse(QEvent::HoverMove,next,canvas->mapToGlobal(next),entry);QCoreApplication::sendEvent(canvas,&mouse);QCOMPARE(canvas->cursorPosition(),next);
        hover(QEvent::HoverEnter,entry);QMouseEvent outside(QEvent::MouseMove,{600,450},window.mapToGlobal(QPointF(600,450)),Qt::NoButton,Qt::NoButton,Qt::NoModifier);QCoreApplication::sendEvent(&mouseReceiver,&outside);QCOMPARE(window.cursor().shape(),Qt::WaitCursor);
        hover(QEvent::HoverEnter,entry);QEvent windowLeave(QEvent::Leave);QCoreApplication::sendEvent(&window,&windowLeave);QVERIFY(window.cursor().shape()!=Qt::BlankCursor);QVERIFY(!canvas->brushCursorVisible());
        hover(QEvent::HoverEnter,entry);canvas->setVisible(false);QVERIFY(window.cursor().shape()!=Qt::BlankCursor);canvas->setVisible(true);
        hover(QEvent::HoverEnter,entry);QTabletEvent leave(QEvent::TabletLeaveProximity,pen,entry,canvas->mapToGlobal(entry),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(&window,&leave);QVERIFY(window.cursor().shape()!=Qt::BlankCursor);QVERIFY(!canvas->brushCursorVisible());
        // Finish cursor assertions before pumping native desktop input for async history.
        QTRY_COMPARE(client.undoDepth(),1);
        if(!initiallyVisible)QTest::keyClick(&window,Qt::Key_CapsLock);
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void tabletHoverTracksOutlineThroughWindowAndItemDelivery() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();client.setBrushRadius(7);
        QHoverEvent mouse(QEvent::HoverMove,{50,60},canvas->mapToGlobal({50,60}),{40,50});QCoreApplication::sendEvent(canvas,&mouse);const bool initiallyVisible=canvas->brushCursorVisible();if(!initiallyVisible)QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());
        // QGuiApplication retains the latest tablet device between delivered events.
        auto *pen=new QPointingDevice("Hover pen",888,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,2);pen->setParent(qApp);
        for(const bool itemDelivery:{false,true}) {
            const QPointF local=itemDelivery?QPointF(123.5,180.25):QPointF(80.25,100.5);
            QTabletEvent hover(QEvent::TabletMove,pen,itemDelivery?local:canvas->mapToScene(local),canvas->mapToGlobal(local),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);
            QCoreApplication::sendEvent(itemDelivery?static_cast<QObject*>(canvas):static_cast<QObject*>(window),&hover);QVERIFY(hover.isAccepted());QVERIFY(canvas->brushCursorVisible());QCOMPARE(canvas->brushCursorRect().center(),local);QCOMPARE(client.undoDepth(),0);
        }
        const QPointF start=canvas->documentRect().center(),finish=start+QPointF(12,8);
        QTabletEvent press(QEvent::TabletPress,pen,start,canvas->mapToGlobal(start),.5,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(canvas,&press);QVERIFY(client.drawing());
        QTabletEvent draw(QEvent::TabletMove,pen,finish,canvas->mapToGlobal(finish),.7,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::LeftButton);QCoreApplication::sendEvent(window,&draw);QCOMPARE(canvas->cursorPosition(),finish);QVERIFY(canvas->brushCursorVisible());
        QHoverEvent staleHover(QEvent::HoverMove,{50,60},canvas->mapToGlobal({50,60}),{40,50});QCoreApplication::sendEvent(canvas,&staleHover);QCOMPARE(canvas->cursorPosition(),finish);
        QMouseEvent staleMouse(QEvent::MouseMove,{50,60},canvas->mapToGlobal({50,60}),Qt::NoButton,Qt::NoButton,Qt::NoModifier);QCoreApplication::sendEvent(canvas,&staleMouse);QCOMPARE(canvas->cursorPosition(),finish);
        QTabletEvent release(QEvent::TabletRelease,pen,finish,canvas->mapToGlobal(finish),0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(canvas,&release);QVERIFY(!client.drawing());QTRY_COMPARE(client.undoDepth(),1);
        QCoreApplication::sendEvent(canvas,&staleHover);QCOMPARE(canvas->cursorPosition(),finish);
        QMouseEvent actualMouse(QEvent::MouseMove,{50,60},canvas->mapToGlobal({50,60}),Qt::NoButton,Qt::NoButton,Qt::NoModifier);QCoreApplication::sendEvent(canvas,&actualMouse);QCoreApplication::sendEvent(canvas,&staleHover);QCOMPARE(canvas->cursorPosition(),QPointF(50,60));
        QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(!canvas->brushCursorVisible());QCOMPARE(canvas->cursor().shape(),Qt::CrossCursor);
        QTabletEvent precise(QEvent::TabletMove,pen,{200,150},canvas->mapToGlobal({200,150}),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(canvas,&precise);QCOMPARE(canvas->brushCursorRect().center(),QPointF(200,150));QCOMPARE(canvas->cursor().shape(),Qt::CrossCursor);
        QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());QTabletEvent leave(QEvent::TabletLeaveProximity,pen,{200,150},canvas->mapToGlobal({200,150}),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(window,&leave);QVERIFY(!canvas->brushCursorVisible());
        if(!initiallyVisible)QTest::keyClick(window,Qt::Key_CapsLock);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void windowsInkTabletPositionSurvivesCachedQuickHoverAndFrames() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("state.ini"));
        QQuickWindow window;window.resize(640,480);
        auto *canvas=new CanvasItem(window.contentItem());canvas->setPosition({40,30});canvas->setSize({500,400});canvas->setClient(&client);
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));QTRY_VERIFY(client.ready());canvas->actualSize();
        // Keep live desktop mouse input from replacing the controlled tablet
        // sequence while waiting for actual Quick frames. Synthetic test mouse
        // events and all tablet/Quick hover events still pass normally.
        class NativeMouseIsolation final:public QObject {
        public:explicit NativeMouseIsolation(QObject *target):m_target(target){qApp->installEventFilter(this);}~NativeMouseIsolation(){qApp->removeEventFilter(this);}
        protected:bool eventFilter(QObject *watched,QEvent *event) override{return watched==m_target && event->spontaneous() && (event->type()==QEvent::MouseMove || event->type()==QEvent::Enter || event->type()==QEvent::Leave);}
        private:QObject *m_target;
        } isolation(&window);
        auto *pen=new QPointingDevice("Windows Ink hover regression",904,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure|QInputDevice::Capability::Hover|QInputDevice::Capability::MouseEmulation,1,3);pen->setParent(qApp);
        const QPointF entry(100,110),next(180,190),last(220,210);
        const auto seedMouseEntry=[&]{const auto scene=canvas->mapToScene(entry);QEnterEvent enter(scene,scene,canvas->mapToGlobal(entry));QCoreApplication::sendEvent(&window,&enter);};
        seedMouseEntry();const bool initial=canvas->brushCursorVisible();if(!initial)QTest::keyClick(&window,Qt::Key_CapsLock);
        window.setCursor(Qt::WaitCursor);
        const auto tablet=[&](QEvent::Type type,QPointF point){const bool down=type==QEvent::TabletPress;QTabletEvent event(type,pen,canvas->mapToScene(point),canvas->mapToGlobal(point),down?.5:0,0,0,0,0,0,Qt::NoModifier,down || type==QEvent::TabletRelease?Qt::LeftButton:Qt::NoButton,down?Qt::LeftButton:Qt::NoButton);QCoreApplication::sendEvent(&window,&event);QVERIFY(event.isAccepted());};
        const auto cachedHover=[&](QEvent::Type type){QHoverEvent event(type,entry,canvas->mapToGlobal(entry),entry);event.setTimestamp(0);QCoreApplication::sendEvent(canvas,&event);};
        tablet(QEvent::TabletMove,next);QCOMPARE(canvas->cursorPosition(),next);QVERIFY(canvas->brushCursorVisible());QCOMPARE(window.cursor().shape(),Qt::BlankCursor);
        for(const auto type:{QEvent::HoverEnter,QEvent::HoverMove,QEvent::HoverLeave}){cachedHover(type);QCOMPARE(canvas->cursorPosition(),next);QVERIFY(canvas->brushCursorVisible());}
        QSignalSpy frames(&window,&QQuickWindow::afterAnimating);canvas->update();QTRY_VERIFY(frames.size()>0);
        QCOMPARE(canvas->cursorPosition(),next);QVERIFY(canvas->brushCursorVisible());QCOMPARE(window.cursor().shape(),Qt::BlankCursor);QCOMPARE(client.undoDepth(),0);
        // Neither a frame-sync hover nor accompanying mouse traffic may move
        // the live pen or restore the native arrow while its stroke is active.
        const auto start=canvas->documentRect().center();tablet(QEvent::TabletPress,start);QVERIFY(client.drawing());
        QMouseEvent companion(QEvent::MouseMove,canvas->mapToScene(entry),canvas->mapToGlobal(entry),Qt::NoButton,Qt::NoButton,Qt::NoModifier);QCoreApplication::sendEvent(&window,&companion);cachedHover(QEvent::HoverMove);QCOMPARE(canvas->cursorPosition(),start);QCOMPARE(window.cursor().shape(),Qt::BlankCursor);
        tablet(QEvent::TabletRelease,start);QVERIFY(!client.drawing());cachedHover(QEvent::HoverMove);QCOMPARE(canvas->cursorPosition(),start);
        tablet(QEvent::TabletMove,last);const auto frameCount=frames.size();canvas->update();QTRY_VERIFY(frames.size()>frameCount);QTRY_COMPARE(client.undoDepth(),1);
        QCOMPARE(canvas->cursorPosition(),last);QCOMPARE(window.cursor().shape(),Qt::BlankCursor);
        // Releasing proximity also prevents a cached frame from resurrecting
        // the old entry outline; the pending cursor refresh must be cancelled.
        cachedHover(QEvent::HoverMove);QTabletEvent leave(QEvent::TabletLeaveProximity,pen,canvas->mapToScene(last),canvas->mapToGlobal(last),0,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(&window,&leave);
        cachedHover(QEvent::HoverMove);QCoreApplication::processEvents();QVERIFY(!canvas->brushCursorVisible());QVERIFY(window.cursor().shape()!=Qt::BlankCursor);
        // A real window mouse move is the explicit handoff; it is delivered
        // through Quick, rather than fabricating a mouse-labelled Hover alone.
        const QPointF mouse(260,230);QMouseEvent move(QEvent::MouseMove,canvas->mapToScene(mouse),canvas->mapToGlobal(mouse),Qt::NoButton,Qt::NoButton,Qt::NoModifier);QCoreApplication::sendEvent(&window,&move);
        QCOMPARE(canvas->cursorPosition(),mouse);QVERIFY(canvas->brushCursorVisible());
        QEvent windowLeave(QEvent::Leave);QCoreApplication::sendEvent(&window,&windowLeave);QVERIFY(!canvas->brushCursorVisible());
        if(!initial)QTest::keyClick(&window,Qt::Key_CapsLock);
        canvas->setClient(nullptr);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void floatingSplitRemovalKeepsRemainingPaneSizeAndPosition() {
        for(const auto &edge:QStringList{"left","right","before","after"}) {
            QTemporaryDir temp;WorkspaceManager workspace(temp.filePath("layout.ini"));
            workspace.detachPanel(workspace.groupForPanel("brush"),"brush");
            workspace.detachPanel(workspace.groupForPanel("color"),"color");
            const auto brush=workspace.groupForPanel("brush"),color=workspace.groupForPanel("color");
            auto data=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",brush},{"whole",true}}).toJson());
            QVERIFY(workspace.dockPayload(data,"floating",color,edge));
            const auto host=workspace.groupDefinition(brush).value("host").toString();QVERIFY(!host.isEmpty());
            workspace.updateGeometry(host,100,100,600,600);
            QRectF remaining;
            for(const auto &value:workspace.layoutItems(host,598,598)) {const auto item=value.toMap();if(item.value("id")==brush && item.value("kind")=="leaf")remaining=item.value("rect").toRectF();}
            QVERIFY(!remaining.isEmpty());workspace.detachPanel(color,"color");
            QVariantMap window;
            for(const auto &value:workspace.floatingWindows())if(value.toMap().value("groups").toStringList().contains(brush))window=value.toMap();
            QVERIFY(!window.isEmpty());
            QCOMPARE(window.value("width").toInt(),remaining.size().toSize().width()+2);
            QCOMPARE(window.value("height").toInt(),remaining.size().toSize().height()+2);
            QCOMPARE(window.value("x").toInt(),100+remaining.topLeft().toPoint().x());
            QCOMPARE(window.value("y").toInt(),100+remaining.topLeft().toPoint().y());
        }
    }
    void layerBlendHoverPreviewsWithoutHistoryAndCommitsOnce_data() {
        QTest::addColumn<bool>("floating");QTest::newRow("docked")<<false;QTest::newRow("floating")<<true;
    }
    void layerBlendHoverPreviewsWithoutHistoryAndCommitsOnce() {
        QFETCH(bool,floating);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());
        client.setBrushRadius(8);client.setBrushColor(Qt::black);InputSample sample;sample.position={16,16};sample.pressure=1;QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);
        client.addLayer("red");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(Qt::red);QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();
        // Keep the real canvas connected: hover must repaint without any viewport scroll.
        QTRY_VERIFY(!client.frame().isNull());
        const auto center=[&](int view){const auto image=client.frame(view);return image.isNull()?QColor():image.pixelColor(image.width()/2,image.height()/2);};
        QTRY_COMPARE(center(0),QColor(Qt::red));
        client.requestViewport(1,{0,0,32,32},{32,32});QTRY_COMPARE(center(1),QColor(Qt::red));
        const auto revision=client.revision();const auto depth=client.undoDepth();const auto top=client.activeLayer();
        QQuickWindow *panelWindow=window;
        if(floating) {
            workspace.detachPanel(workspace.groupForPanel("layers"),"layers");const auto group=workspace.groupForPanel("layers");panelWindow=nullptr;
            QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+group && w->isVisible())panelWindow=qobject_cast<QQuickWindow*>(w);return panelWindow!=nullptr;}());
        }
        QQuickItem *combo=nullptr;QTRY_VERIFY((combo=findVisualItem(panelWindow->contentItem(),"layerBlendMode")));auto *popup=combo->property("popup").value<QObject*>();QVERIFY(popup);
        auto hoverMultiply=[&]()->QQuickItem* {
            if(!QMetaObject::invokeMethod(popup,"open"))return nullptr;
            auto *content=qobject_cast<QQuickItem*>(popup->property("contentItem").value<QObject*>());if(!content)return nullptr;
            QTest::qWait(30);auto *option=findVisualItem(content,"layerBlendModeOption:3");if(option)QTest::mouseMove(panelWindow,option->mapToScene({20,10}).toPoint());return option;
        };
        auto *option=hoverMultiply();QVERIFY(option);QCOMPARE(option->height(),qreal(20));
        QTRY_COMPARE(center(0),QColor(Qt::black));QTRY_COMPARE(center(1),QColor(Qt::black));QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),depth);
        auto *content=qobject_cast<QQuickItem*>(popup->property("contentItem").value<QObject*>());QVERIFY(content);
        for(int index:{0,3,0,3}) {
            auto *next=findVisualItem(content,"layerBlendModeOption:"+QString::number(index));QVERIFY(next);
            QTest::mouseMove(panelWindow,next->mapToScene({20,10}).toPoint());
            QTRY_COMPARE(center(0),index==3?QColor(Qt::black):QColor(Qt::red));
            QTRY_COMPARE(center(1),index==3?QColor(Qt::black):QColor(Qt::red));
            QCOMPARE(client.undoDepth(),depth);QCOMPARE(client.revision(),revision);
        }
        // Opening on a later current item can move the list under a stationary pointer.
        // Exercise that placement/scroll change without relying on a wheel event to refresh hover.
        auto *normal=findVisualItem(content,"layerBlendModeOption:0");QVERIFY(normal);
        QTest::mouseMove(panelWindow,normal->mapToScene({20,10}).toPoint());QTRY_COMPARE(center(0),QColor(Qt::red));
        QVERIFY(content->setProperty("contentY",qreal(60)));QTRY_COMPARE(content->property("contentY").toReal(),qreal(60));
        QTRY_COMPARE(center(0),QColor(Qt::black));QTRY_COMPARE(center(1),QColor(Qt::black));
        QVERIFY(content->setProperty("contentY",qreal(0)));QTRY_COMPARE(center(0),QColor(Qt::red));QTRY_COMPARE(center(1),QColor(Qt::red));
        QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),depth);
        for(const auto &value:client.layers())if(value.toMap().value("id").toULongLong()==top)QCOMPARE(value.toMap().value("blendMode").toInt(),0);
        QVERIFY(QMetaObject::invokeMethod(popup,"close"));QTRY_COMPARE(center(0),QColor(Qt::red));QTRY_COMPARE(center(1),QColor(Qt::red));QCOMPARE(client.undoDepth(),depth);
        option=hoverMultiply();QVERIFY(option);QTRY_COMPARE(center(0),QColor(Qt::black));QTest::mouseClick(panelWindow,Qt::LeftButton,Qt::NoModifier,option->mapToScene({20,10}).toPoint());
        QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(center(0),QColor(Qt::black));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void compactLayerSliderAndHsvMarkersUseSharedStyles() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        QQuickItem *opacity=nullptr;QTRY_VERIFY((opacity=findVisualItem(window->contentItem(),"layerOpacity")));QVERIFY(QMetaObject::invokeMethod(opacity,"openSlider"));
        auto *popup=opacity->findChild<QObject*>("layerOpacityPopup");QVERIFY(popup);QCOMPARE(popup->property("width").toReal(),qreal(127.5));QCOMPARE(popup->property("padding").toReal(),qreal(7.5));
        auto *slider=qobject_cast<QQuickItem*>(popup->property("contentItem").value<QObject*>());QVERIFY(slider);QCOMPARE(slider->implicitHeight(),qreal(16.5));
        auto *handle=qobject_cast<QQuickItem*>(slider->property("handle").value<QObject*>());QVERIFY(handle);QCOMPARE(handle->width(),qreal(6.75));QCOMPARE(handle->height(),qreal(9));QVERIFY(QMetaObject::invokeMethod(popup,"close"));
        for(int axis=0;axis<3;++axis) {
            auto *marker=findVisualItem(window->contentItem(),"hsvMarker:"+QString::number(axis));QVERIFY(marker);auto *control=marker->parentItem();QVERIFY(control);auto *gradient=qobject_cast<QQuickItem*>(control->property("background").value<QObject*>());QVERIFY(gradient);QVERIFY(marker->y()>gradient->y()+gradient->height());
        }
        auto *bar=findVisualItem(window->contentItem(),"brushOptionsBar");QVERIFY(bar);
        for(auto *child:bar->childItems())for(auto *item:child->childItems())QVERIFY(!(item->width()==16 && item->height()==16));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void maximizedTitleBarDragRestoresAndFollowsPointer() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        window->resize(1050,670);window->setPosition(window->screen()->availableGeometry().topLeft()+QPoint(80,70));
        QTRY_COMPARE(window->property("normalGeometry").toRect(),window->geometry());
        const auto normalSize=window->size();
        auto *grip=findVisualItem(window->contentItem(),"mainWindowDragArea");QVERIFY(grip);
        const auto titleAt=[&](qreal fraction) {
            qreal first=30;
            for(const auto &title:QStringList{"文件","编辑","图像","图层","选择","滤镜","窗口"})if(auto *entry=findVisualItem(window->contentItem(),"menuEntry:"+title))first=std::max(first,entry->mapToScene({entry->width(),0}).x()+8);
            const qreal last=grip->width()-106;
            // The offscreen two-DPI screen can be narrower than the app's
            // minimum width. Its logo-side gap remains a real draggable area.
            return QPoint(first<=last?qRound(first+(last-first)*fraction):(fraction<.65?2:6),14);
        };
        for(qreal fraction:{.55,.75}) {
            window->showMaximized();QTRY_COMPARE(window->visibility(),QWindow::Maximized);QTest::qWait(80);
            const auto at=titleAt(fraction);
            const auto pressGlobal=window->mapToGlobal(at);
            QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at);QVERIFY(grip->property("pressed").toBool());QCOMPARE(window->visibility(),QWindow::Maximized);
            QTest::mouseMove(window,at+QPoint(1,1));QCOMPARE(window->visibility(),QWindow::Maximized);
            const auto global=pressGlobal+QPoint(90,70),offset=QPoint(qRound(normalSize.width()*qreal(at.x())/grip->width()),14);
            QTest::mouseMove(window,window->mapFromGlobal(global));QTRY_COMPARE(window->visibility(),QWindow::Windowed);QCOMPARE(window->size(),normalSize);QTRY_COMPARE(window->position(),global-offset);
            const auto next=global+QPoint(-40,30);QTest::mouseMove(window,window->mapFromGlobal(next));QTRY_COMPARE(window->position(),next-offset);
            QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,window->mapFromGlobal(next));
            // A deferred continuation after release must finish immediately, not retain a
            // stale pressed flag from the global drag's consumed release event.
            workspace.continueRestoredWindowDrag(window,next,offset);
            QTest::mouseMove(window,window->mapFromGlobal(next+QPoint(5,5)));QCOMPARE(window->position(),next-offset);
            QTRY_COMPARE(window->property("normalGeometry").toRect(),window->geometry());
            const auto moved=window->geometry();window->showMinimized();QTRY_COMPARE(window->visibility(),QWindow::Minimized);
#ifdef Q_OS_WIN
            if(workspace.windowsWindowFrames())SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_RESTORE,0);else
#endif
                window->showNormal();
            QTRY_COMPARE(window->visibility(),QWindow::Windowed);QTRY_COMPARE(window->geometry(),moved);
        }
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            window->showMaximized();QTRY_COMPARE(window->visibility(),QWindow::Maximized);
            const auto handle=reinterpret_cast<HWND>(window->winId());
            SetWindowPos(handle,HWND_TOPMOST,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE);window->requestActivate();QTRY_VERIFY(window->isActive());
            POINT physical{qRound(grip->width()*.65*window->devicePixelRatio()),qRound(14*window->devicePixelRatio())};ClientToScreen(handle,&physical);SetCursorPos(physical.x,physical.y);
            INPUT down{};down.type=INPUT_MOUSE;down.mi.dwFlags=MOUSEEVENTF_LEFTDOWN;
            const auto release=qScopeGuard([]{INPUT up{};up.type=INPUT_MOUSE;up.mi.dwFlags=MOUSEEVENTF_LEFTUP;SendInput(1,&up,sizeof(INPUT));});
            QCOMPARE(SendInput(1,&down,sizeof(INPUT)),UINT(1));QTRY_VERIFY(grip->property("pressed").toBool());QCOMPARE(window->visibility(),QWindow::Maximized);
            SetCursorPos(physical.x+qRound(100*window->devicePixelRatio()),physical.y+qRound(90*window->devicePixelRatio()));
            QTRY_COMPARE(window->visibility(),QWindow::Windowed);QCOMPARE(window->size(),normalSize);
            const auto offset=QPoint(qRound(normalSize.width()*grip->property("pressFraction").toReal()),qRound(grip->property("pressY").toReal()));
            QTRY_VERIFY((window->position()-(QCursor::pos()-offset)).manhattanLength()<=2);
            const auto firstPosition=window->position(),firstCursor=QCursor::pos();GetCursorPos(&physical);
            SetCursorPos(physical.x+qRound(40*window->devicePixelRatio()),physical.y+qRound(25*window->devicePixelRatio()));
            QTRY_VERIFY((window->position()-(firstPosition+QCursor::pos()-firstCursor)).manhattanLength()<=2);
            const auto rounded=[](HWND hwnd) {const auto region=CreateRectRgn(0,0,0,0);const bool result=GetWindowRgn(hwnd,region)==COMPLEXREGION && !PtInRegion(region,0,0);DeleteObject(region);return result;};
            const auto blur=reinterpret_cast<HWND>(workspace.menuBlurWindowHandle());QVERIFY(blur);QTRY_VERIFY(rounded(handle) && rounded(blur));
            RECT mainBounds{},blurBounds{};QVERIFY(GetWindowRect(handle,&mainBounds));QVERIFY(GetWindowRect(blur,&blurBounds));QCOMPARE(blurBounds.left,mainBounds.left);QCOMPARE(blurBounds.top,mainBounds.top);QCOMPARE(blurBounds.right,mainBounds.right);
        }
#endif
        window->showMaximized();QTRY_COMPARE(window->visibility(),QWindow::Maximized);QTest::qWait(80);
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            const auto at=titleAt(.5);POINT point{qRound(at.x()*window->devicePixelRatio()),qRound(at.y()*window->devicePixelRatio())};ClientToScreen(reinterpret_cast<HWND>(window->winId()),&point);SetCursorPos(point.x,point.y);
            INPUT clicks[4]{};for(int i=0;i<4;++i){clicks[i].type=INPUT_MOUSE;clicks[i].mi.dwFlags=i%2?MOUSEEVENTF_LEFTUP:MOUSEEVENTF_LEFTDOWN;}QCOMPARE(SendInput(4,clicks,sizeof(INPUT)),UINT(4));
        } else
#endif
            QTest::mouseDClick(window,Qt::LeftButton,Qt::NoModifier,titleAt(.5));
        QTRY_COMPARE(window->visibility(),QWindow::Windowed);QCOMPARE(window->size(),normalSize);
        window->showFullScreen();QTest::qWait(80);const auto fullGeometry=window->geometry();const auto fullAt=titleAt(.25);
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,fullAt);QTest::mouseMove(window,fullAt+QPoint(60,25));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,fullAt+QPoint(60,25));QCOMPARE(window->visibility(),QWindow::FullScreen);QCOMPARE(window->geometry(),fullGeometry);
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            QVERIFY(PostMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_MOVE,0));QTest::qWait(50);QCOMPARE(window->visibility(),QWindow::FullScreen);
        }
#endif
        window->showNormal();QTRY_COMPARE(window->visibility(),QWindow::Windowed);
        const auto normalGeometry=window->geometry();const auto normalAt=titleAt(.5);QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,normalAt);QTest::mouseMove(window,normalAt+QPoint(1,1));QCOMPARE(window->geometry(),normalGeometry);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,normalAt+QPoint(1,1));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void nativeMenuBlurLayerFollowsWindowAndHidesBeforeMinimize() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            const auto main=reinterpret_cast<HWND>(window->winId()),blur=reinterpret_cast<HWND>(workspace.menuBlurWindowHandle());QVERIFY(blur);QVERIFY(IsWindow(blur));
            const auto style=GetWindowLongPtrW(blur,GWL_EXSTYLE);QVERIFY(style&WS_EX_TOOLWINDOW);QVERIFY(style&WS_EX_NOACTIVATE);QVERIFY(style&WS_EX_TRANSPARENT);QVERIFY(!GetWindow(blur,GW_OWNER));
            const auto correct=[&](bool rounded) {
                RECT a{},b{};if(!GetWindowRect(main,&a) || !GetWindowRect(blur,&b))return false;
                const auto region=CreateRectRgn(0,0,0,0);const int kind=GetWindowRgn(blur,region);
                const bool shape=kind!=ERROR && bool(PtInRegion(region,0,0))==!rounded && PtInRegion(region,0,b.bottom-b.top-1) && PtInRegion(region,(b.right-b.left)/2,0);
                DeleteObject(region);
                return IsWindowVisible(blur) && a.left==b.left && a.top==b.top && a.right==b.right && b.bottom-b.top==qRound(28*window->devicePixelRatio()) && shape && GetWindow(main,GW_HWNDNEXT)==blur;
            };
            QTRY_VERIFY(correct(true));window->setPosition(window->position()+QPoint(37,29));window->resize(1050,670);QTRY_VERIFY(correct(true));
            for(bool maximized:{false,true})for(int cycle=0;cycle<3;++cycle) {
                if(maximized)window->showMaximized();else window->showNormal();QTRY_VERIFY(correct(!maximized));
                SendMessageW(main,WM_SYSCOMMAND,SC_MINIMIZE,0);QVERIFY(!IsWindowVisible(blur));QVERIFY(!IsIconic(blur));
                SendMessageW(main,WM_SYSCOMMAND,SC_RESTORE,0);QTRY_COMPARE(window->visibility(),maximized?QWindow::Maximized:QWindow::Windowed);QTRY_VERIFY(correct(!maximized));
                window->showNormal();QTRY_VERIFY(correct(true));
            }
            window->hide();QVERIFY(!IsWindowVisible(blur));window->show();QTRY_VERIFY(correct(true));
            QVERIFY(workspace.setMenuBarBlur(window,false));QVERIFY(!IsWindowVisible(blur));QVERIFY(workspace.setMenuBarBlur(window,true));QTRY_VERIFY(correct(true));
            engine.clearComponentCache();engine.rootObjects().first()->deleteLater();QTRY_VERIFY(!IsWindow(blur));
        } else
#endif
            QCOMPARE(workspace.menuBlurWindowHandle(),quintptr(0));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void brushPresetClicksKeepEraserAndUseSelectedPresetSize() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();QVERIFY(library);
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);
        const auto stroke=[&] {InputSample sample;sample.position={32,64};sample.pressure=1;sample.tool=1;if(!client.beginStroke(sample))return false;sample.position={96,64};client.strokeTo(sample);client.endStroke();return true;};
        const auto rendered=[&] {client.requestViewport(0,{0,0,128,128},{128,128});return client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,128);};
        library->setRadius(12);client.setBrushColor(Qt::black);QVERIFY(stroke());QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(rendered());const int painted=client.frame().pixelColor(64,64).alpha();QVERIFY(painted>200);
        client.setEraser(true);
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()) {
            const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush"))workspace.setActive(group.value("id").toString(),"brush");
        }
        QQuickItem *fine=nullptr;QTRY_VERIFY((fine=findVisualItem(window->contentItem(),"brushPreset:round-fine")));
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,fine->mapToScene({fine->width()/2,fine->height()/2}).toPoint());QTRY_COMPARE(library->selectedId(),QString("round-fine"));QVERIFY(client.eraser());
        library->setRadius(8);library->setOpacity(.5);library->setSpacing(.25);QCOMPARE(client.brushRadius(),qreal(8));QCOMPARE(client.brushOpacity(),qreal(.5));QCOMPARE(client.brushSpacing(),qreal(.25));
        QVERIFY(stroke());QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(rendered());const int erased=client.frame().pixelColor(64,64).alpha();QVERIFY(erased<painted);
        auto *brush=findVisualItem(window->contentItem(),"brushTool");QVERIFY(brush);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,brush->mapToScene({brush->width()/2,brush->height()/2}).toPoint());QVERIFY(!client.eraser());QCOMPARE(library->selectedId(),QString("round-fine"));QCOMPARE(client.brushRadius(),qreal(8));QCOMPARE(client.brushOpacity(),qreal(.5));QCOMPARE(client.brushSpacing(),qreal(.25));
        QVERIFY(stroke());QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(rendered());QVERIFY(client.frame().pixelColor(64,64).alpha()>erased);
        canvas->setClient(&client);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void dualSettingsImportPreviewPixelsHistoryAndExchange() {
        QTemporaryDir temp;QImage texture(4,1,QImage::Format_RGBA8888);
        for(int x=0;x<4;++x)texture.setPixelColor(x,0,x<2?Qt::white:Qt::black);
        const auto input=temp.filePath("secondary.png");QVERIFY(texture.save(input));
        PaintCoreClient client(nullptr,temp.filePath("source/storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:4")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        auto *toggle=findVisualItem(window->contentItem(),"settingsDualEnabled");QVERIFY(toggle);
        QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);const auto original=library->selectedId();
        QVERIFY(client.importDualBrushTip(QUrl::fromLocalFile(input),0));QVERIFY(library->select("round-fine"));
        QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QVERIFY(library->dualBrush().value("resource").isNull());
        QVERIFY(library->select(original));QCOMPARE(library->dualBrush().value("resource").toString().size(),64);QVERIFY(library->maskResource().isEmpty());QVERIFY(library->texture().value("resource").isNull());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({toggle->width()/2,toggle->height()/2}).toPoint());QTRY_VERIFY(library->dualBrush().value("enabled").toBool());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({toggle->width()/2,toggle->height()/2}).toPoint());QTRY_VERIFY(!library->dualBrush().value("enabled").toBool());
        auto *size=findVisualItem(window->contentItem(),"settingsDualdiameterInput");QVERIFY(size && size->isEnabled());
        for(auto *parent=size->parentItem();parent;parent=parent->parentItem())if(parent->clip()){QVERIFY(size->mapToItem(parent,{0,0}).x()>=-1);QVERIFY2(size->mapToItem(parent,{size->width(),0}).x()<=parent->width()+1,qPrintable(QString("size %1 parent %2 width %3 right %4").arg(size->width()).arg(parent->objectName()).arg(parent->width()).arg(size->mapToItem(parent,{size->width(),0}).x())));}
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,size->mapToScene({size->width()/2,size->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_2);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(library->dualBrush().value("diameter").toDouble(),20.);
        auto *page=findVisualItem(window->contentItem(),"settingsDualPage");QVERIFY(page);
        page->setProperty("currentIndex",1);QVERIFY(QMetaObject::invokeMethod(page,"activated",Q_ARG(int,1)));
        auto *add=findVisualItem(window->contentItem(),"dualPressureSizeAdd");QVERIFY(add);QVERIFY(QMetaObject::invokeMethod(add,"clicked"));QTRY_COMPARE(library->dualBrush().value("pressureDynamics").toMap().value("size").toMap().value("curve").toList().size(),3);
        page->setProperty("currentIndex",2);QVERIFY(QMetaObject::invokeMethod(page,"activated",Q_ARG(int,2)));
        auto *scatter=findVisualItem(window->contentItem(),"settingsDualScatterEnabled");QVERIFY(scatter);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,scatter->mapToScene({scatter->width()/2,scatter->height()/2}).toPoint());QTRY_VERIFY(library->dualBrush().value("scatterDynamics").toMap().value("enabled").toBool());
        page->setProperty("currentIndex",3);QVERIFY(QMetaObject::invokeMethod(page,"activated",Q_ARG(int,3)));
        auto *shape=findVisualItem(window->contentItem(),"settingsDualShapeEnabled");QVERIFY(shape);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,shape->mapToScene({shape->width()/2,shape->height()/2}).toPoint());QTRY_VERIFY(library->dualBrush().value("shapeJitter").toMap().value("enabled").toBool());
        page->setProperty("currentIndex",0);QVERIFY(QMetaObject::invokeMethod(page,"activated",Q_ARG(int,0)));
        library->setRadius(8);library->setOpacity(.4);library->setSpacing(.02);library->setPressureSize(false);library->setPressureFlow(false);
        library->requestPreview(original);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto plain=library->selectedPreview();
        auto dual=library->dualBrush();dual["enabled"]=true;dual["diameter"]=16.;dual["spacing"]=1.;dual["pressureSize"]=false;dual["pressureFlow"]=false;QVERIFY(library->setDualBrush(dual));
        library->requestPreview(original);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(library->selectedPreview()!=plain);
        const auto firstDualPreview=library->selectedPreview();library->setRadius(4);library->requestPreview(original);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(library->selectedPreview()!=firstDualPreview);library->setRadius(8);
        client.setBrushColor(Qt::red);client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);
        InputSample sample;sample.position={16.5,32.5};sample.pressure=1;sample.tool=1;QVERIFY(client.beginStroke(sample));
        auto changed=dual;changed["mode"]="subtract";QVERIFY(library->setDualBrush(changed));
        sample.position={48.5,32.5};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        const auto rendered=[&]{client.requestViewport(0,{0,0,64,64},{64,64});return client.frameRevision()>=client.revision() && client.frame().size()==QSize(64,64);};QTRY_VERIFY(rendered());const auto expected=client.frame();
        int lo=255,hi=0;for(int x=20;x<44;++x){const auto a=expected.pixelColor(x,32).alpha();lo=std::min(lo,a);hi=std::max(hi,a);}QVERIFY(hi-lo>40);QVERIFY(hi<=103);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(rendered());QCOMPARE(client.frame().pixelColor(32,32).alpha(),0);client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(rendered());QCOMPARE(client.frame(),expected);
        QVERIFY(library->setDualBrush(dual));const auto archive=QUrl::fromLocalFile(temp.filePath("dual.vbr"));QVERIFY(client.exportBrush(archive));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QVERIFY(QFile::remove(input));
        PaintCoreClient target(nullptr,temp.filePath("target/storage.ini"));QTRY_VERIFY(target.ready());QSignalSpy targetFinished(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(targetFinished.size(),1);QVERIFY(targetFinished.last()[0].toBool());QCOMPARE(target.brushLibrary()->dualBrush(),dual);
        target.setBrushColor(Qt::red);target.newTransparentDocument(64,64);QTRY_COMPARE(target.documentWidth(),64);QTRY_VERIFY(target.ready());sample.position={16.5,32.5};QVERIFY(target.beginStroke(sample));sample.position={48.5,32.5};target.strokeTo(sample);target.endStroke();QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(!target.layerEditBusy());
        const auto targetRendered=[&]{target.requestViewport(0,{0,0,64,64},{64,64});return target.frameRevision()>=target.revision() && target.frame().size()==QSize(64,64);};QTRY_VERIFY(targetRendered());QCOMPARE(target.frame(),expected);
        const auto resetCopy=library->saveCopy("真实资源重置");QVERIFY(!resetCopy.isEmpty());library->useProceduralDual();library->setRadius(4);
        auto *reset=findVisualItem(window->contentItem(),"resetBrushPreset");QVERIFY(reset);QVERIFY(!reset->isVisible());
        auto *sourceCategory=findVisualItem(window->contentItem(),"brushCategory:8");QVERIFY(sourceCategory);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,sourceCategory->mapToScene({sourceCategory->width()/2,sourceCategory->height()/2}).toPoint());QTRY_VERIFY(reset->isVisible());
        auto *scroll=findVisualItem(window->contentItem(),"brushParameterScroll");QVERIFY(scroll);auto *flickable=qvariant_cast<QQuickItem*>(scroll->property("contentItem"));QVERIFY(flickable);
        flickable->setProperty("contentY",std::max(0.,flickable->property("contentHeight").toDouble()-flickable->height()));
        QTRY_VERIFY(reset->mapToItem(scroll,{0,0}).y()>=0);QVERIFY(reset->mapToItem(scroll,{0,reset->height()}).y()<=scroll->height()+1);
        auto *preview=findVisualItem(window->contentItem(),"brushStrokePreview");QVERIFY(preview);QVERIFY(reset->mapToScene({0,reset->height()}).y()<preview->mapToScene({0,0}).y());
        const auto resetScreenshot=qEnvironmentVariable("DRAWVERSE_RESET_PREVIEW");if(!resetScreenshot.isEmpty())QVERIFY(window->grabWindow().save(resetScreenshot));
        const auto history=client.undoDepth();QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,reset->mapToScene({reset->width()/2,reset->height()/2}).toPoint());QTRY_COMPARE(library->dualBrush(),dual);QCOMPARE(library->radius(),8.);QCOMPARE(client.undoDepth(),history);QVERIFY(library->select(original));QVERIFY(library->remove(resetCopy));
        const auto deletionTarget=library->saveCopy("导入期间删除目标");QVERIFY(!deletionTarget.isEmpty());
        const auto reimport=temp.filePath("reimport.png");QVERIFY(texture.save(reimport));
        QVERIFY(client.importDualBrushTip(QUrl::fromLocalFile(reimport),1));QVERIFY(library->remove(deletionTarget));QVERIFY(library->select(original));
        QTRY_COMPARE(finished.size(),3);QVERIFY(!finished.last()[0].toBool());QCOMPARE(library->dualBrush(),dual);
        const auto screenshot=qEnvironmentVariable("DRAWVERSE_DUAL_PREVIEW");if(!screenshot.isEmpty())QVERIFY(window->grabWindow().save(screenshot));
        QVERIFY2(warnings.isEmpty(),qPrintable(warnings.join("\n")));
    }
    void textureSettingsCaptureImportTargetPreviewPixelsAndExchange() {
        QTemporaryDir temp;
        QImage texture(4,1,QImage::Format_RGBA8888);
        for(int x=0;x<4;++x){const int v=x==0?0:x==2?255:128;texture.setPixelColor(x,0,QColor(v,v,v,255));}
        const auto input=temp.filePath("texture.png");QVERIFY(texture.save(input));
        PaintCoreClient client(nullptr,temp.filePath("source/storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:3")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        auto *toggle=findVisualItem(window->contentItem(),"settingsTextureEnabled");QVERIFY(toggle && !toggle->isEnabled());
        QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);const auto original=library->selectedId();
        QVERIFY(client.importBrushTexture(QUrl::fromLocalFile(input),0));QVERIFY(library->select("round-fine"));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QVERIFY(library->texture().value("resource").isNull());QVERIFY(library->maskResource().isEmpty());
        QVERIFY(library->select(original));const auto identity=library->texture().value("resource").toString();QCOMPARE(identity.size(),64);QVERIFY(library->maskResource().isEmpty());QVERIFY(!library->texture().value("enabled").toBool());QTRY_VERIFY(toggle->isEnabled());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({toggle->width()/2,toggle->height()/2}).toPoint());QTRY_VERIFY(library->texture().value("enabled").toBool());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({toggle->width()/2,toggle->height()/2}).toPoint());QTRY_VERIFY(!library->texture().value("enabled").toBool());
        auto *scale=findVisualItem(window->contentItem(),"settingsTexturescaleInput");QVERIFY(scale && scale->isEnabled());for(auto *parent=scale->parentItem();parent;parent=parent->parentItem()){if(parent->clip()){const auto left=scale->mapToItem(parent,{0,0}).x();const auto right=scale->mapToItem(parent,{scale->width(),0}).x();QVERIFY(left>=-1);QVERIFY(right<=parent->width()+1);}}scale->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_2);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(library->texture().value("scale").toDouble(),2.);QVERIFY(!library->texture().value("enabled").toBool());
        const auto closed=library->texture();BrushLibrary restored;QVERIFY(restored.restore(library->snapshot()));QCOMPARE(restored.texture(),closed);QCOMPARE(QJsonDocument::fromJson(restored.snapshot()).object().value("version").toInt(),11);
        const auto good=restored.snapshot();auto bad=closed;bad["scale"]=.009999999;QVERIFY(!restored.setTexture(bad));bad=closed;bad["resource"]=42;QVERIFY(!restored.setTexture(bad));bad=closed;bad.remove("resource");QVERIFY(!restored.setTexture(bad));bad=closed;bad["model"]="other";QVERIFY(!restored.setTexture(bad));QCOMPARE(restored.snapshot(),good);
        auto old=QJsonDocument::fromJson(good).object();old["version"]=10;QVERIFY(!restored.restore(QJsonDocument(old).toJson()));QCOMPARE(restored.snapshot(),good);
        QVERIFY(restored.select("round-wide"));restored.setHueJitter(.2);QCOMPARE(QJsonDocument::fromJson(restored.snapshot()).object().value("version").toInt(),11);
        auto batch=QJsonDocument::fromJson(library->exportPackage()).object();auto brushes=batch.value("brushes").toArray();brushes.append(QJsonDocument::fromJson(restored.exportPackage()).object().value("brushes").toArray().first());batch["brushes"]=brushes;BrushLibrary importer;QVERIFY(importer.importPackage(QJsonDocument(batch).toJson()));QCOMPARE(importer.texture(),closed);QCOMPARE(QJsonDocument::fromJson(importer.snapshot()).object().value("version").toInt(),11);
        const auto copy=library->saveCopy("纹理副本");QVERIFY(!copy.isEmpty());QCOMPARE(library->texture(),closed);
        library->setRadius(8);library->setOpacity(.4);library->setSpacing(.02);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(copy);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto plain=library->selectedPreview();
        auto data=library->texture();data["enabled"]=true;data["scale"]=1.;QVERIFY(library->setTexture(data));library->requestPreview(copy);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(library->selectedPreview()!=plain);
        client.setBrushColor(Qt::red);client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);
        InputSample sample;sample.position={16.5,32.5};sample.pressure=1;sample.tool=1;QVERIFY(client.beginStroke(sample));
        auto changed=library->texture();changed["invert"]=true;changed["depth"]=.25;QVERIFY(library->setTexture(changed));sample.position={48.5,32.5};client.strokeTo(sample);sample.position={16.5,32.5};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        const auto rendered=[&]{client.requestViewport(0,{0,0,64,64},{64,64});return client.frameRevision()>=client.revision() && client.frame().size()==QSize(64,64);};QTRY_VERIFY(rendered());const auto expected=client.frame();
        int lo=255,hi=0;for(int x=20;x<44;++x){const auto a=expected.pixelColor(x,32).alpha();lo=std::min(lo,a);hi=std::max(hi,a);}QVERIFY(hi-lo>40);QVERIFY(hi<=103);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(rendered());QCOMPARE(client.frame().pixelColor(32,32).alpha(),0);client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(rendered());QCOMPARE(client.frame(),expected);
        QVERIFY(library->setTexture(data));const auto archive=QUrl::fromLocalFile(temp.filePath("texture.vbr"));QVERIFY(client.exportBrush(archive));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QVERIFY(QFile::remove(input));
        PaintCoreClient target(nullptr,temp.filePath("target/storage.ini"));QTRY_VERIFY(target.ready());QSignalSpy targetFinished(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(targetFinished.size(),1);QVERIFY(targetFinished.last()[0].toBool());QCOMPARE(target.brushLibrary()->texture(),data);QVERIFY(target.brushLibrary()->maskResource().isEmpty());
        target.setBrushColor(Qt::red);target.newTransparentDocument(64,64);QTRY_COMPARE(target.documentWidth(),64);QTRY_VERIFY(target.ready());sample.position={16.5,32.5};QVERIFY(target.beginStroke(sample));sample.position={48.5,32.5};target.strokeTo(sample);sample.position={16.5,32.5};target.strokeTo(sample);target.endStroke();QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(!target.layerEditBusy());
        const auto targetRendered=[&]{target.requestViewport(0,{0,0,64,64},{64,64});return target.frameRevision()>=target.revision() && target.frame().size()==QSize(64,64);};QTRY_VERIFY(targetRendered());QCOMPARE(target.frame(),expected);
        auto *remove=findVisualItem(window->contentItem(),"removeBrushTexture");QVERIFY(remove);QVERIFY(QMetaObject::invokeMethod(remove,"clicked"));QVERIFY(library->texture().value("resource").isNull());QVERIFY(!library->texture().value("enabled").toBool());
        auto noResource=library->texture();noResource["enabled"]=true;QVERIFY(!library->setTexture(noResource));
        const auto preview=qEnvironmentVariable("DRAWVERSE_TEXTURE_PREVIEW");if(!preview.isEmpty()){QTest::qWait(100);QVERIFY(window->grabWindow().save(preview));}
        QVERIFY(texture.save(input));const auto doomed=library->saveCopy("待删除");QVERIFY(client.importBrushTexture(QUrl::fromLocalFile(input),0));QVERIFY(library->remove(doomed));QTRY_COMPARE(finished.size(),3);QVERIFY(!finished.last()[0].toBool());
        QVERIFY2(warnings.isEmpty(),qPrintable(warnings.join('\n')));canvas->setClient(&client);QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);QSignalSpy targetStopped(&target,&PaintCoreClient::stopped);target.shutdown();QTRY_COMPARE(targetStopped.size(),1);
    }
    void wetSmudgeUsesOwnToolStrengthAndFingerPolicyWithUndo() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("wet-smudge.ini"));QTRY_VERIFY(client.ready());
        client.newTransparentDocument(128,64);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());
        auto *library=client.brushLibrary();library->setRadius(4);library->setPressureSize(false);library->setPressureFlow(false);library->setFlow(1);
        auto wet=library->wetMix();wet["enabled"]=true;for(const auto &key:{"wetness","pickup","mix","consumption"})wet[key]=0.;wet["autoClean"]=true;wet["autoLoad"]=true;wet["load"]=1.;wet["deposit"]=1.;QVERIFY(library->setWetMix(wet));
        client.activateSmudge();QVERIFY(client.smudgeTool());client.setSmudgeStrength(.4);client.setSmudgePressure(true);client.setSmudgeFingerPainting(false);client.setBrushColor(Qt::red);
        InputSample sample;sample.position={32.5,32.5};sample.pressure=.5;sample.tool=1;sample.capabilities=1;
        QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!client.drawing());QCOMPARE(client.undoDepth(),0);
        client.setSmudgeFingerPainting(true);QVERIFY(client.beginStroke(sample));client.setSmudgeStrength(1);client.setSmudgeFingerPainting(false);client.setBrushColor(Qt::blue);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,64));const auto after=client.frame();const auto c=after.pixelColor(32,32);QVERIFY(c.red()>=250);QCOMPARE(c.blue(),0);QVERIFY(c.alpha()>=50 && c.alpha()<=52);QCOMPARE(client.historyEntries().last().toMap().value("kind").toInt(),26);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame().pixelColor(32,32).alpha(),0);
        client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),after);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }

    void wetDualModelPreservesClosedPolicyBaselineAndStrictState19() {
        QSKIP("双色绘制已删除");
        for(bool wetFirst:{false,true}) {
            BrushLibrary library;
            auto wet=library.wetMix();wet["enabled"]=true;wet["load"]=.4;
            auto dual=library.dualBrush();dual["enabled"]=true;dual["color"]=QVariantMap{{"model","separated-stroke-v1"},{"enabled",true},{"useBackground",true},{"overflow",true},{"order","second-behind"},{"primaryBlend","normal"},{"secondaryBlend","screen"}};
            if(wetFirst){QVERIFY(library.setWetMix(wet));QVERIFY(library.setDualBrush(dual));}else{QVERIFY(library.setDualBrush(dual));QVERIFY(library.setWetMix(wet));}
            QCOMPARE(library.wetMix().value("dualColor").toString(),QString("independent-carriers-v1"));
            const auto active=library.wetMix();const auto copy=library.saveCopy("双色湿混基准");QVERIFY(!copy.isEmpty());
            auto closed=active;closed["enabled"]=false;closed["pickup"]=.23;QVERIFY(library.setWetMix(closed));dual["enabled"]=false;QVERIFY(library.setDualBrush(dual));
            const auto state=library.snapshot();QCOMPARE(QJsonDocument::fromJson(state).object().value("version").toInt(),19);
            BrushLibrary restored;QVERIFY(restored.restore(state));QCOMPARE(restored.wetMix(),closed);QCOMPARE(restored.dualBrush(),library.dualBrush());QVERIFY(restored.resetPreset());QCOMPARE(restored.wetMix(),active);QVERIFY(restored.dualBrush().value("enabled").toBool());
            const auto package=library.exportPackage();auto root=QJsonDocument::fromJson(package).object();auto brushes=root.value("brushes").toArray();const auto b=brushes.first().toObject();QCOMPARE(b.value("behaviorVersion").toInt(),16);QVERIFY(b.value("requiredCapabilities").toArray().contains("wet-dual-color"));
            BrushLibrary imported;QVERIFY(imported.importPackage(package));QCOMPARE(imported.wetMix(),closed);QVERIFY(imported.resetPreset());QCOMPARE(imported.wetMix(),closed);
            const auto good=library.snapshot();for(const auto &value:QVariantList{QVariant{},true,"unknown",1}){auto bad=closed;bad["dualColor"]=value;QVERIFY(!library.setWetMix(bad));QCOMPARE(library.snapshot(),good);}
            auto downgraded=QJsonDocument::fromJson(good).object();downgraded["version"]=18;QVERIFY(!library.restore(QJsonDocument(downgraded).toJson()));QCOMPARE(library.snapshot(),good);
            auto unmarked=QJsonDocument::fromJson(restored.snapshot()).object();auto rows=unmarked.value("presets").toArray();for(int i=0;i<rows.size();++i){auto row=rows[i].toObject();if(row.value("id").toString()==copy){auto data=row.value("wetMix").toObject();data.remove("dualColor");row["wetMix"]=data;rows[i]=row;}}unmarked["presets"]=rows;
            QVERIFY(!library.restore(QJsonDocument(unmarked).toJson()));QCOMPARE(library.snapshot(),good);
            auto wrong=b;wrong["behaviorVersion"]=15;brushes[0]=wrong;root["brushes"]=brushes;QVERIFY(!library.importPackage(QJsonDocument(root).toJson()));QCOMPARE(library.snapshot(),good);
            QVERIFY(library.applyNativeCatalog(library.catalogSnapshot()));QCOMPARE(library.snapshot(),good);
        }
    }
    void wetDualControlsLoadBothCapturedColorsPreviewAndHistory() {
        QSKIP("双色绘制已删除");
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("settings.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();library->setRadius(4);library->setOpacity(1);library->setPressureSize(false);library->setPressureFlow(false);
        auto dual=library->dualBrush();dual["enabled"]=true;dual["diameter"]=16.;dual["pressureSize"]=false;dual["pressureFlow"]=false;dual["color"]=QVariantMap{{"model","separated-stroke-v1"},{"enabled",true},{"useBackground",true},{"overflow",true},{"order","second-behind"},{"primaryBlend","normal"},{"secondaryBlend","normal"}};QVERIFY(library->setDualBrush(dual));
        auto wet=library->wetMix();wet["autoClean"]=false;wet["autoLoad"]=false;for(const auto &key:{"wetness","pickup","mix","consumption"})wet[key]=0.;wet["load"]=1.;wet["deposit"]=1.;QVERIFY(library->setWetMix(wet));
        for(const auto &v:workspace.leftGroups()+workspace.rightGroups()){const auto g=v.toMap();if(g.value("panels").toStringList().contains("brush-settings"))workspace.setActive(g.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:10")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());auto *toggle=findVisualItem(window->contentItem(),"settingsWetEnabled");QVERIFY(toggle && toggle->isVisible());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({7,toggle->height()/2}).toPoint());QTRY_VERIFY(library->wetMix().value("enabled").toBool());QCOMPARE(library->wetMix().value("dualColor").toString(),QString("independent-carriers-v1"));QVERIFY(findVisualItem(window->contentItem(),"settingsWetDualNotice")->isVisible());
        client.newTransparentDocument(128,64);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);
        client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);QSignalSpy action(&client,&PaintCoreClient::wetBrushFinished);
        auto *scroll=findVisualItem(window->contentItem(),"brushParameterScroll");QVERIFY(scroll);auto *flickable=qvariant_cast<QQuickItem*>(scroll->property("contentItem"));QVERIFY(flickable);flickable->setProperty("contentY",std::max(0.,flickable->property("contentHeight").toDouble()-flickable->height()));
        auto *load=findVisualItem(window->contentItem(),"wetLoadBrush");QVERIFY(load && load->isEnabled());QCOMPARE(load->property("text").toString(),QString("载入双颜色"));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,load->mapToScene({load->width()/2,load->height()/2}).toPoint());
        // GUI colors can change before the queued worker executes; the accepted load owns both colors.
        client.setBrushColor(Qt::green);client.setSecondaryBrushColor(Qt::yellow);QTRY_COMPARE(action.size(),1);QVERIFY(action.last()[0].toBool());
        library->requestPreview(library->selectedId());QTRY_VERIFY2_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),qPrintable(library->previewMessage()),10000);const auto preview=library->selectedPreview();const auto image=QImage::fromData(QByteArray::fromBase64(preview.section(',',1).toLatin1()));QCOMPARE(image.size(),QSize(240,60));int visible=0,clear=0;for(int y=0;y<image.height();++y)for(int x=0;x<image.width();++x){const auto c=image.pixelColor(x,y);if(c.alpha()){++visible;QCOMPARE(c.red(),255);QCOMPARE(c.green(),255);QCOMPARE(c.blue(),255);}else ++clear;}QVERIFY(visible>0 && clear>0);QCOMPARE(client.undoDepth(),0);
        QSignalSpy requests(library,&BrushLibrary::previewRequested);client.swapBrushColors();library->requestPreview(library->selectedId());QCOMPARE(requests.size(),0);QCOMPARE(library->selectedPreview(),preview);
        InputSample sample;sample.position={32.5,32.5};sample.pressure=1;QVERIFY(client.beginStroke(sample));QVERIFY(!client.loadWetBrush());auto changed=library->dualBrush();changed["diameter"]=2.;QVERIFY(library->setDualBrush(changed));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        const auto render=[&]{client.requestViewport(0,{0,0,128,64},{128,64});return client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,64);};QTRY_VERIFY(render());QCOMPARE(client.frame().pixelColor(32,32),QColor(Qt::red));QCOMPARE(client.frame().pixelColor(38,32),QColor(Qt::blue));
        QVERIFY(library->setDualBrush(dual));sample.position={80.5,32.5};QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(render());QCOMPARE(client.frame().pixelColor(80,32),QColor(Qt::red));QCOMPARE(client.frame().pixelColor(86,32),QColor(Qt::blue));const auto painted=client.frame();
        client.undo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(render());QCOMPARE(client.frame(),painted);
        auto *clean=findVisualItem(window->contentItem(),"wetCleanBrush");QVERIFY(clean && clean->isEnabled());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,clean->mapToScene({clean->width()/2,clean->height()/2}).toPoint());QTRY_COMPARE(action.size(),2);QVERIFY(action.last()[0].toBool());sample.position={110.5,32.5};QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!client.drawing());QCOMPARE(client.undoDepth(),2);
        canvas->setClient(&client);QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void wetDualImageVbrFreshStoreAndReopenMatchCapturedPixels() {
        QSKIP("双色绘制已删除");
        QTemporaryDir temp;const auto settings=temp.filePath("source/settings.ini");PaintCoreClient client(nullptr,settings);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        QImage mask(16,16,QImage::Format_RGBA8888);mask.fill(Qt::white);const auto path=temp.filePath("mask.png");QVERIFY(mask.save(path));QSignalSpy file(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushTip(QUrl::fromLocalFile(path),1));QTRY_COMPARE(file.size(),1);QVERIFY(file.last()[0].toBool());const auto resource=library->maskResource();QVERIFY(!resource.isEmpty());
        library->setRadius(4);library->setOpacity(.5);library->setPressureSize(false);library->setPressureFlow(false);auto dual=library->dualBrush();dual["enabled"]=true;dual["diameter"]=16.;dual["pressureSize"]=false;dual["pressureFlow"]=false;dual["resource"]=resource;dual["color"]=QVariantMap{{"model","separated-stroke-v1"},{"enabled",true},{"useBackground",true},{"overflow",true},{"order","second-behind"},{"primaryBlend","normal"},{"secondaryBlend","screen"}};QVERIFY(library->setDualBrush(dual));QVERIFY(library->setTextureResource(library->selectedId(),resource));auto texture=library->texture();texture["enabled"]=true;QVERIFY(library->setTexture(texture));
        auto wet=library->wetMix();wet["enabled"]=true;wet["autoClean"]=true;wet["autoLoad"]=true;wet["wetness"]=.3;wet["pickup"]=.2;wet["mix"]=.4;wet["load"]=.7;QVERIFY(library->setWetMix(wet));wet=library->wetMix();for(const auto &key:{"wetness","pickup","mix","load","deposit","consumption"})wet[key]=double(float(wet.value(key).toDouble()));
        const auto id=library->saveCopy("湿混双色图片");QVERIFY(!id.isEmpty());const auto archive=QUrl::fromLocalFile(temp.filePath("dual.vbr"));QVERIFY(client.exportBrush(archive));QTRY_COMPARE(file.size(),2);QVERIFY(file.last()[0].toBool());
        client.newTransparentDocument(96,64);QTRY_COMPARE(client.documentWidth(),96);QTRY_VERIFY(client.ready());client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);const auto begin=[](){InputSample s;s.position={16.5,32.5};s.pressure=1;return s;};const auto finish=[](PaintCoreClient &c){InputSample s;s.position={80.5,32.5};s.pressure=1;c.strokeTo(s);c.endStroke();};const auto render=[](PaintCoreClient &c){c.requestViewport(0,{0,0,96,64},{96,64});return c.frameRevision()>=c.revision() && c.frame().size()==QSize(96,64);};
        QVERIFY(client.beginStroke(begin()));client.setBrushColor(Qt::green);client.setSecondaryBrushColor(Qt::yellow);auto closed=wet;closed["enabled"]=false;QVERIFY(library->setWetMix(closed));finish(client);QTRY_VERIFY2(client.undoDepth()==1,qPrintable(client.lastError()));QTRY_VERIFY(render(client));const auto expected=client.frame();QVERIFY(expected.pixelColor(48,32).alpha()>0);
        const auto targetSettings=temp.filePath("fresh/settings.ini");PaintCoreClient target(nullptr,targetSettings);QTRY_VERIFY(target.ready());QSignalSpy imported(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(imported.size(),1);QVERIFY2(imported.last()[0].toBool(),qPrintable(target.lastError()));QCOMPARE(target.brushLibrary()->wetMix(),wet);QCOMPARE(target.brushLibrary()->maskResource(),resource);QCOMPARE(target.brushLibrary()->dualBrush().value("resource").toString(),resource);QCOMPARE(target.brushLibrary()->texture().value("resource").toString(),resource);
        target.newTransparentDocument(96,64);QTRY_COMPARE(target.documentWidth(),96);QTRY_VERIFY(target.ready());target.setBrushColor(Qt::red);target.setSecondaryBrushColor(Qt::blue);QVERIFY(target.beginStroke(begin()));finish(target);QTRY_VERIFY2(target.undoDepth()==1,qPrintable(target.lastError()));QTRY_VERIFY(render(target));QCOMPARE(target.frame(),expected);
        auto modified=wet;modified["enabled"]=false;modified["load"]=double(float(.12));QVERIFY(target.brushLibrary()->setWetMix(modified));QVERIFY(target.brushLibrary()->resetPreset());QCOMPARE(target.brushLibrary()->wetMix(),wet);QVERIFY(target.brushLibrary()->setWetMix(modified));const auto importedId=target.brushLibrary()->selectedId();
        const auto saved=[&]{QSettings settings(targetSettings,QSettings::IniFormat);const auto root=QJsonDocument::fromJson(settings.value("brushes/state").toByteArray()).object();for(const auto &value:root.value("presets").toArray()){const auto row=value.toObject();if(row.value("id").toString()==importedId)return row.value("wetMix").toObject()==QJsonObject::fromVariantMap(modified);}return false;};QTRY_VERIFY(saved());
        const auto closedArchive=QUrl::fromLocalFile(temp.filePath("closed-dual.vbr"));QVERIFY(target.exportBrush(closedArchive));QTRY_COMPARE(imported.size(),2);QVERIFY(imported.last()[0].toBool());
        PaintCoreClient closedImport(nullptr,temp.filePath("closed/settings.ini"));QTRY_VERIFY(closedImport.ready());QSignalSpy closedFile(&closedImport,&PaintCoreClient::brushFileFinished);QVERIFY(closedImport.importBrushes(closedArchive));QTRY_COMPARE(closedFile.size(),1);QVERIFY(closedFile.last()[0].toBool());QCOMPARE(closedImport.brushLibrary()->wetMix(),modified);QSignalSpy closedStopped(&closedImport,&PaintCoreClient::stopped);closedImport.shutdown();QTRY_COMPARE(closedStopped.size(),1);
        QSignalSpy stopped(&target,&PaintCoreClient::stopped);target.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,targetSettings);QTRY_VERIFY(reopened.ready());QTRY_COMPARE(reopened.brushLibrary()->selectedId(),importedId);QCOMPARE(reopened.brushLibrary()->wetMix(),modified);QVERIFY(reopened.brushLibrary()->resetPreset());QCOMPARE(reopened.brushLibrary()->wetMix(),wet);reopened.newTransparentDocument(96,64);QTRY_COMPARE(reopened.documentWidth(),96);QTRY_VERIFY(reopened.ready());reopened.setBrushColor(Qt::red);reopened.setSecondaryBrushColor(Qt::blue);QVERIFY(reopened.beginStroke(begin()));finish(reopened);QTRY_COMPARE(reopened.undoDepth(),1);QTRY_VERIFY(render(reopened));QCOMPARE(reopened.frame(),expected);
        QSignalSpy end(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(end.size(),1);QSignalSpy reopenedEnd(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(reopenedEnd.size(),1);
    }

    void wetModelPreservesClosedPolicyResetBaselineAndExchange() {
        BrushLibrary library;QVERIFY(!library.wetMix().value("enabled").toBool());QCOMPARE(QJsonDocument::fromJson(library.snapshot()).object().value("version").toInt(),2);
        auto data=library.wetMix();data["enabled"]=true;data["allLayers"]=true;data["autoClean"]=false;data["autoLoad"]=false;data["wetness"]=.37;data["pickup"]=.25;data["mix"]=.75;data["load"]=.5;data["deposit"]=.8;data["consumption"]=.125;
        QVERIFY(library.setWetMix(data));const auto copy=library.saveCopy("湿混基准");QVERIFY(!copy.isEmpty());QCOMPARE(library.wetMix(),data);
        auto closed=data;closed["enabled"]=false;closed["load"]=.23;QVERIFY(library.setWetMix(closed));QCOMPARE(QJsonDocument::fromJson(library.snapshot()).object().value("version").toInt(),17);
        const auto state=library.snapshot();BrushLibrary restored;QVERIFY(restored.restore(state));QCOMPARE(restored.wetMix(),closed);QVERIFY(restored.resetPreset());QCOMPARE(restored.wetMix(),data);
        const auto package=library.exportPackage();const auto p=QJsonDocument::fromJson(package).object().value("brushes").toArray().first().toObject();QCOMPARE(p.value("behaviorVersion").toInt(),14);QVERIFY(p.value("requiredCapabilities").toArray().contains("wet-mix"));
        BrushLibrary imported;QVERIFY(imported.importPackage(package));QCOMPARE(imported.wetMix(),closed);QVERIFY(imported.resetPreset());QCOMPARE(imported.wetMix(),closed);
        const auto before=library.snapshot();auto bad=closed;bad["deposit"]=1.0000000001;QVERIFY(!library.setWetMix(bad));bad=closed;bad["enabled"]=1;QVERIFY(!library.setWetMix(bad));bad=closed;bad.remove("load");QVERIFY(!library.setWetMix(bad));bad=closed;bad["model"]="unknown";QVERIFY(!library.setWetMix(bad));QCOMPARE(library.snapshot(),before);
        auto old=QJsonDocument::fromJson(before).object();old["version"]=16;QVERIFY(!library.restore(QJsonDocument(old).toJson()));QCOMPARE(library.snapshot(),before);
        auto dual=library.dualBrush();dual["enabled"]=true;dual["color"]=QVariantMap{{"model","separated-stroke-v1"},{"enabled",true},{"useBackground",true},{"overflow",true},{"order","second-behind"},{"primaryBlend","normal"},{"secondaryBlend","normal"}};
        QVERIFY(library.setDualBrush(dual));QVERIFY(library.setWetMix(data));QVERIFY(!library.wetMix().contains("dualColor"));
        BrushLibrary defaults;auto base=defaults.wetMix();base["enabled"]=true;QVERIFY(defaults.setWetMix(base));base["enabled"]=false;base["consumption"]=double(float(.1));QVERIFY(defaults.setWetMix(base));QCOMPARE(QJsonDocument::fromJson(defaults.snapshot()).object().value("version").toInt(),2);
    }
    void legacyDualColorSettingsAreRemovedOnRestore() {
        BrushLibrary library;auto dual=library.dualBrush();dual["enabled"]=true;dual["color"]=QVariantMap{{"model","separated-stroke-v1"},{"enabled",true},{"useBackground",true},{"overflow",true},{"order","second-behind"},{"primaryBlend","normal"},{"secondaryBlend","normal"}};QVERIFY(library.setDualBrush(dual));
        auto wet=library.wetMix();wet["enabled"]=true;wet["dualColor"]="independent-carriers-v1";QVERIFY(library.setWetMix(wet));
        BrushLibrary restored;QVERIFY(restored.restore(library.snapshot()));QVERIFY(!restored.dualBrush().contains("color"));QVERIFY(!restored.wetMix().contains("dualColor"));
    }
    void transferJitterStateRoundtripKeepsThreeChannels() {
        BrushLibrary library;auto state=QJsonDocument::fromJson(library.snapshot()).object();state["version"]=37;auto rows=state.value("presets").toArray();auto row=rows[0].toObject();row["transferJitter"]=QJsonObject{{"model","transfer-jitter-v1"},{"size",.6},{"flow",.5},{"opacity",.25}};rows[0]=row;state["presets"]=rows;
        BrushLibrary restored;QVERIFY(restored.restore(QJsonDocument(state).toJson()));const auto jitter=restored.tipSettings().value("transferJitter").toMap();QCOMPARE(jitter.value("size").toDouble(),.6);QCOMPARE(jitter.value("flow").toDouble(),.5);QCOMPARE(jitter.value("opacity").toDouble(),.25);QCOMPARE(QJsonDocument::fromJson(restored.snapshot()).object().value("version").toInt(),37);
    }
    void brushTexturePreviewStateVersionIncludesDepthFields() {
        QCOMPARE(BrushLibrary::textureStateVersion({}),0);
        QCOMPARE(BrushLibrary::textureStateVersion(QVariantMap{{"mode","multiply"}}),11);
        QCOMPARE(BrushLibrary::textureStateVersion(QVariantMap{{"mode","multiply"},{"minimumDepth",1.}}),21);
        QCOMPARE(BrushLibrary::textureStateVersion(QVariantMap{{"mode","overlay"},{"depthPressure",true}}),30);
        QCOMPARE(BrushLibrary::textureStateVersion(QVariantMap{{"mode","linearHeight"}}),31);
    }
    void brushPackageImportWrapsFileNameAndKeepsInnerGroups() {
        BrushLibrary source;const auto inner=source.createGroup("Inner"),nested=source.createGroup("Nested",inner);QVERIFY(!inner.isEmpty() && !nested.isEmpty());QVERIFY(source.movePresetToGroup(source.selectedId(),nested));const auto package=source.exportPackage();QVERIFY(!package.isEmpty());
        BrushLibrary target;QVERIFY(target.importPackage(package,"Imported File"));const auto organization=QJsonObject::fromVariantMap(target.organization());const auto groups=organization.value("groups").toArray();QString outerId,innerId,nestedId;QJsonArray outerItems,innerItems,nestedItems;
        for(const auto &value:groups){const auto group=value.toObject();const auto name=group.value("name").toString(),id=group.value("id").toString();if(name=="Imported File"){outerId=id;outerItems=group.value("items").toArray();}else if(name=="Inner"){innerId=id;innerItems=group.value("items").toArray();}else if(name=="Nested"){nestedId=id;nestedItems=group.value("items").toArray();}}
        QVERIFY(!outerId.isEmpty());const auto roots=organization.value("roots").toArray();QVERIFY(std::any_of(roots.begin(),roots.end(),[&](const QJsonValue &value){return value.toObject().value("kind").toString()=="group" && value.toObject().value("id").toString()==outerId;}));QVERIFY(!innerId.isEmpty());QVERIFY(std::any_of(outerItems.begin(),outerItems.end(),[&](const QJsonValue &value){return value.toObject().value("kind").toString()=="group" && value.toObject().value("id").toString()==innerId;}));
        QVERIFY(!nestedId.isEmpty());QVERIFY(std::any_of(innerItems.begin(),innerItems.end(),[&](const QJsonValue &value){return value.toObject().value("kind").toString()=="group" && value.toObject().value("id").toString()==nestedId;}));QVERIFY(std::any_of(nestedItems.begin(),nestedItems.end(),[&](const QJsonValue &value){return value.toObject().value("kind").toString()=="preset";}));
    }
    void brushImportUsesFileNameAsOuterGroup() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("state.ini"));QTRY_VERIFY(client.ready());QTRY_VERIFY(client.brushLibraryReady());
        const auto path=QUrl::fromLocalFile(temp.filePath("Imported Brushes.vbr"));QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.exportBrush(path));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QVERIFY(client.importBrushes(path));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());
        bool found=false;for(const auto &value:client.brushLibrary()->organization().value("groups").toList())if(value.toMap().value("name").toString()=="Imported Brushes" && !value.toMap().value("items").toList().isEmpty())found=true;QVERIFY(found);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void wetSettingsPageManualInkAndPreviewStaySeparateFromCanvas() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:10")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        auto *toggle=findVisualItem(window->contentItem(),"settingsWetEnabled");QVERIFY(toggle && toggle->isVisible());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({toggle->width()/2,toggle->height()/2}).toPoint());QTRY_VERIFY(library->wetMix().value("enabled").toBool());
        auto *wetness=findVisualItem(window->contentItem(),"settingsWetwetnessInput");QVERIFY(wetness);window->requestActivate();QTRY_VERIFY(window->isActive());auto *wetText=findVisualItem(window->contentItem(),"settingsWetwetnessInputText");QVERIFY(wetText);QTRY_VERIFY(wetText->width()>0);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,wetText->mapToScene({wetText->width()/2,wetText->height()/2}).toPoint());QTRY_VERIFY(wetText->hasActiveFocus());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_1);QTest::keyClick(window,Qt::Key_5);QTest::keyClick(window,Qt::Key_Return);QTRY_VERIFY(qAbs(library->wetMix().value("wetness").toDouble()-.15)<1e-7);
        library->setRadius(4);library->setOpacity(1);library->setPressureSize(false);library->setPressureFlow(false);auto data=library->wetMix();for(const auto &key:{"wetness","pickup","mix","consumption"})data[key]=0.;data["load"]=.5;data["deposit"]=1.;data["autoClean"]=false;data["autoLoad"]=false;QVERIFY(library->setWetMix(data));
        client.newTransparentDocument(128,64);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);client.setBrushColor(Qt::red);
        QSignalSpy action(&client,&PaintCoreClient::wetBrushFinished);auto *scroll=findVisualItem(window->contentItem(),"brushParameterScroll");QVERIFY(scroll);auto *flickable=qvariant_cast<QQuickItem*>(scroll->property("contentItem"));QVERIFY(flickable);flickable->setProperty("contentY",std::max(0.,flickable->property("contentHeight").toDouble()-flickable->height()));
        auto *load=findVisualItem(window->contentItem(),"wetLoadBrush");QVERIFY(load && load->isEnabled());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,load->mapToScene({load->width()/2,load->height()/2}).toPoint());QTRY_COMPARE(action.size(),1);QVERIFY(action.last()[0].toBool());QVERIFY(!client.wetActionBusy());
        library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto preview=library->selectedPreview();const auto previewData=QByteArray::fromBase64(preview.mid(preview.indexOf(',')+1).toLatin1());const auto previewImage=QImage::fromData(previewData);QVERIFY(!previewImage.isNull());QVERIFY(previewImage.pixelColor(60,30).red()>150);QVERIFY(previewImage.pixelColor(180,30).blue()>150);QCOMPARE(client.undoDepth(),0);
        auto mixing=data;mixing["wetness"]=.5;mixing["pickup"]=.5;mixing["mix"]=.5;mixing["autoLoad"]=true;QVERIFY(library->setWetMix(mixing));library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto mixed=library->selectedPreview();const auto mixedImage=QImage::fromData(QByteArray::fromBase64(mixed.mid(mixed.indexOf(',')+1).toLatin1()));QVERIFY(!mixedImage.isNull());for(int y=0;y<mixedImage.height();++y)for(int x=0;x<mixedImage.width();++x){const auto c=mixedImage.pixelColor(x,y);if(c.alpha()>0){QCOMPARE(c.red(),255);QCOMPARE(c.green(),255);QCOMPARE(c.blue(),255);}}QCOMPARE(client.undoDepth(),0);QVERIFY(library->setWetMix(data));library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);

        client.setBrushColor(Qt::blue);InputSample sample;sample.position={32.5,32.5};sample.pressure=1;QVERIFY(client.beginStroke(sample));QVERIFY(!client.cleanWetBrush());client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,64));const auto red=client.frame().pixelColor(32,32);QVERIFY(red.red()>=250);QCOMPARE(red.blue(),0);QVERIFY(red.alpha()>=127 && red.alpha()<=128);const auto painted=client.frame();
        auto *clean=findVisualItem(window->contentItem(),"wetCleanBrush");QVERIFY(clean && clean->isEnabled());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,clean->mapToScene({clean->width()/2,clean->height()/2}).toPoint());QTRY_COMPARE(action.size(),2);QVERIFY(action.last()[0].toBool());sample.position={80.5,32.5};QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!client.drawing());QCOMPARE(client.undoDepth(),1);
        QVERIFY(client.loadWetBrush());QTRY_COMPARE(action.size(),3);QVERIFY(action.last()[0].toBool());QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.frame().pixelColor(80,32).blue()>=250);
        client.undo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),painted);client.redo();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());
        canvas->setClient(&client);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QTest::qWait(60);const auto screenshot=qEnvironmentVariable("DRAWVERSE_WET_PREVIEW");if(!screenshot.isEmpty())QVERIFY(window->grabWindow().save(screenshot));QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void wetPolicyPersistsSharesAcrossDocumentsAndImportsFromVbr() {
        QTemporaryDir temp;const auto path=temp.filePath("storage.ini");PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();library->setRadius(4);library->setOpacity(1);library->setPressureSize(false);library->setPressureFlow(false);auto data=library->wetMix();data["enabled"]=true;data["autoClean"]=false;data["autoLoad"]=false;for(const auto &key:{"wetness","pickup","mix","consumption"})data[key]=0.;data["load"]=.5;QVERIFY(library->setWetMix(data));const auto id=library->saveCopy("跨文档湿混");QVERIFY(!id.isEmpty());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());client.setBrushColor(Qt::red);QSignalSpy action(&client,&PaintCoreClient::wetBrushFinished);QVERIFY(client.loadWetBrush());QTRY_COMPARE(action.size(),1);QVERIFY(action.last()[0].toBool());
        PaintCoreClient other(nullptr,path,client.brushCatalogConnection());other.setBrushPreferencePersistenceEnabled(false);QTRY_VERIFY(other.ready());QTRY_VERIFY(other.brushLibrary()->wetMix().value("enabled").toBool());other.newTransparentDocument(64,64);QTRY_COMPARE(other.documentWidth(),64);QTRY_VERIFY(other.ready());other.setBrushColor(Qt::blue);
        QSignalSpy file(&client,&PaintCoreClient::brushFileFinished);const auto vbr=QUrl::fromLocalFile(temp.filePath("wet.vbr"));QVERIFY(client.exportBrush(vbr));QTRY_COMPARE(file.size(),1);QVERIFY(file.last()[0].toBool());const auto saved=library->wetMix();
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);InputSample sample;sample.position={32.5,32.5};sample.pressure=1;QVERIFY(other.beginStroke(sample));other.endStroke();QTRY_COMPARE(other.undoDepth(),1);QTRY_VERIFY(!other.layerEditBusy());other.requestViewport(0,{0,0,64,64},{64,64});QTRY_VERIFY(other.frameRevision()>=other.revision() && other.frame().size()==QSize(64,64));QVERIFY(other.frame().pixelColor(32,32).red()>=250);QCOMPARE(other.frame().pixelColor(32,32).blue(),0);QSignalSpy otherStopped(&other,&PaintCoreClient::stopped);other.shutdown();QTRY_COMPARE(otherStopped.size(),1);
        PaintCoreClient reopened(nullptr,path);QTRY_VERIFY(reopened.ready());QTRY_COMPARE(reopened.brushLibrary()->selectedId(),id);QCOMPARE(reopened.brushLibrary()->wetMix(),saved);auto changed=saved;changed["load"]=.25;QVERIFY(reopened.brushLibrary()->setWetMix(changed));QVERIFY(reopened.brushLibrary()->resetPreset());QCOMPARE(reopened.brushLibrary()->wetMix(),saved);QSignalSpy reopenedStopped(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(reopenedStopped.size(),1);
        PaintCoreClient imported(nullptr,temp.filePath("fresh/storage.ini"));QTRY_VERIFY(imported.ready());QSignalSpy importedFile(&imported,&PaintCoreClient::brushFileFinished);QVERIFY(imported.importBrushes(vbr));QTRY_COMPARE(importedFile.size(),1);QVERIFY(importedFile.last()[0].toBool());QCOMPARE(imported.brushLibrary()->wetMix(),saved);QSignalSpy importedStopped(&imported,&PaintCoreClient::stopped);imported.shutdown();QTRY_COMPARE(importedStopped.size(),1);
    }
    void colorDynamicSettingsUseCapturedForegroundBackgroundAndRoundTrip() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:5")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());auto *toggle=findVisualItem(window->contentItem(),"settingsColorDynamicsEnabled");QVERIFY(toggle);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({toggle->width()/2,toggle->height()/2}).toPoint());QTRY_VERIFY(library->colorDynamicsEnabled());auto *mix=findVisualItem(window->contentItem(),"settingsForegroundBackgroundJitterInput");QVERIFY(mix);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,mix->mapToScene({mix->width()/2,mix->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_1);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(library->foregroundBackgroundJitter(),1.);auto *perTip=findVisualItem(window->contentItem(),"settingsColorPerTip");QVERIFY(perTip);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,perTip->mapToScene({perTip->width()/2,perTip->height()/2}).toPoint());QTRY_VERIFY(!library->colorPerTip());
        library->setHueJitter(.2);library->setSaturationJitter(.3);library->setValueJitter(.1);auto config=library->colorDynamics();config["seed"]="ffffffffffffffff";QVERIFY(library->setColorDynamics(config));library->setColorDynamicsEnabled(false);auto *hue=findVisualItem(window->contentItem(),"settingsHueJitterInput");QVERIFY(hue && hue->isEnabled());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,hue->mapToScene({hue->width()/2,hue->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_4);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(library->hueJitter(),.4);QVERIFY(!library->colorDynamicsEnabled());const auto closed=library->colorDynamics();BrushLibrary restored;QVERIFY(restored.restore(library->snapshot()));QCOMPARE(restored.colorDynamics(),closed);QCOMPARE(QJsonDocument::fromJson(restored.snapshot()).object().value("version").toInt(),10);const auto good=restored.snapshot();auto bad=closed;bad["model"]="srgb-hsv";QVERIFY(!restored.setColorDynamics(bad));bad=closed;bad["hue"]=1.00000001;QVERIFY(!restored.setColorDynamics(bad));bad=closed;bad["seed"]=qulonglong(-1);QVERIFY(!restored.setColorDynamics(bad));QCOMPARE(restored.snapshot(),good);auto old=QJsonDocument::fromJson(good).object();old["version"]=9;QVERIFY(!restored.restore(QJsonDocument(old).toJson()));QCOMPARE(restored.snapshot(),good);QVERIFY(restored.select("round-wide"));restored.setShapeSizeJitter(.5);QCOMPARE(QJsonDocument::fromJson(restored.snapshot()).object().value("version").toInt(),10);auto batch=QJsonDocument::fromJson(library->exportPackage()).object();auto brushes=batch.value("brushes").toArray();brushes.append(QJsonDocument::fromJson(restored.exportPackage()).object().value("brushes").toArray().first());batch["brushes"]=brushes;BrushLibrary importer;QVERIFY(importer.importPackage(QJsonDocument(batch).toJson()));QCOMPARE(importer.colorDynamics(),closed);QCOMPARE(QJsonDocument::fromJson(importer.snapshot()).object().value("version").toInt(),10);BrushLibrary defaults;defaults.setColorDynamicsEnabled(true);defaults.setColorDynamicsEnabled(false);QCOMPARE(QJsonDocument::fromJson(defaults.snapshot()).object().value("version").toInt(),2);const auto copy=library->saveCopy("颜色副本");QVERIFY(!copy.isEmpty());QVERIFY(library->select("round-fine"));QVERIFY(!library->colorDynamicsEnabled());QVERIFY(library->select(copy));QCOMPARE(library->colorDynamics(),closed);
        QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);const auto path=QUrl::fromLocalFile(temp.filePath("color.vbr"));QVERIFY(client.exportBrush(path));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QVERIFY(client.importBrushes(path));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QCOMPARE(library->colorDynamics(),closed);library->setRadius(8);library->setSpacing(.2);library->setOpacity(.25);library->setPressureSize(false);library->setPressureFlow(false);library->setHueJitter(0);library->setSaturationJitter(0);library->setValueJitter(0);library->setColorPerTip(true);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto plain=library->selectedPreview();library->setColorDynamicsEnabled(true);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QCOMPARE(library->selectedPreview(),plain);
        client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);InputSample sample;sample.position={32,64};sample.pressure=1;sample.tool=1;QVERIFY(client.beginStroke(sample));client.setBrushColor(Qt::green);client.setSecondaryBrushColor(Qt::green);library->setColorDynamicsEnabled(false);sample.position={96,64};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,128));const auto painted=client.frame();int purple=0;QSet<QRgb> colors;for(int y=0;y<128;++y)for(int x=0;x<128;++x){const auto c=painted.pixelColor(x,y);QVERIFY(c.alpha()<=65);if(c.alpha()>0){QCOMPARE(c.green(),0);if(c.red()>0 && c.blue()>0)++purple;if(c.alpha()>32)colors.insert(c.rgb());}}QVERIFY(purple>50);QVERIFY(colors.size()>5);client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),painted);canvas->setClient(&client);const auto saved=library->colorDynamics();QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(reopened.ready());QTRY_COMPARE(reopened.brushLibrary()->colorDynamics(),saved);QCOMPARE(reopened.brushLibrary()->radius(),8.);QSignalSpy restopped(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(restopped.size(),1);
    }
    void shapeJitterSettingsPersistExchangePreviewAndCaptureRealPixels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:1")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());auto *page=findVisualItem(window->contentItem(),"settingsShapePage");QVERIFY(page);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,page->mapToScene({page->width()/2,page->height()/2}).toPoint());QQuickItem *option=nullptr;QTRY_VERIFY((option=findVisualItem(window->contentItem(),"settingsShapePageOption:1")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,option->mapToScene({option->width()/2,option->height()/2}).toPoint());
        auto *toggle=findVisualItem(window->contentItem(),"settingsShapeJitterEnabled");QVERIFY(toggle && toggle->isVisible());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({toggle->width()/2,toggle->height()/2}).toPoint());QTRY_VERIFY(library->shapeJitterEnabled());auto *size=findVisualItem(window->contentItem(),"settingsShapeSizeJitterInput");QVERIFY(size);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,size->mapToScene({size->width()/2,size->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_5);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(library->shapeSizeJitter(),.5);
        library->setShapeMinimumSize(.2);library->setShapeAngleJitter(1);library->setShapeRoundnessJitter(.5);library->setShapeMinimumRoundness(.2);auto config=library->shapeJitter();config["seed"]="ffffffffffffffff";QVERIFY(library->setShapeJitter(config));library->setShapeJitterEnabled(false);QVERIFY(size->isEnabled());const auto closed=library->shapeJitter();BrushLibrary restored;QVERIFY(restored.restore(library->snapshot()));QCOMPARE(restored.shapeJitter(),closed);QCOMPARE(QJsonDocument::fromJson(restored.snapshot()).object().value("version").toInt(),9);const auto good=restored.snapshot();auto bad=closed;bad["size"]=1.00000001;QVERIFY(!restored.setShapeJitter(bad));bad=closed;bad["seed"]=qulonglong(-1);QVERIFY(!restored.setShapeJitter(bad));QCOMPARE(restored.snapshot(),good);auto old=QJsonDocument::fromJson(good).object();old["version"]=8;QVERIFY(!restored.restore(QJsonDocument(old).toJson()));QCOMPARE(restored.snapshot(),good);
        QVERIFY(restored.select("round-wide"));restored.setScatterAmount(1);QCOMPARE(QJsonDocument::fromJson(restored.snapshot()).object().value("version").toInt(),9);auto batch=QJsonDocument::fromJson(library->exportPackage()).object();auto brushes=batch.value("brushes").toArray();brushes.append(QJsonDocument::fromJson(restored.exportPackage()).object().value("brushes").toArray().first());batch["brushes"]=brushes;BrushLibrary importer;QVERIFY(importer.importPackage(QJsonDocument(batch).toJson()));QCOMPARE(QJsonDocument::fromJson(importer.snapshot()).object().value("version").toInt(),9);QCOMPARE(importer.shapeJitter(),closed);BrushLibrary defaults;defaults.setShapeJitterEnabled(true);defaults.setShapeJitterEnabled(false);QCOMPARE(QJsonDocument::fromJson(defaults.snapshot()).object().value("version").toInt(),2);const auto copy=library->saveCopy("抖动副本");QVERIFY(!copy.isEmpty());QVERIFY(library->select("round-fine"));QVERIFY(!library->shapeJitterEnabled());QVERIFY(library->select(copy));QCOMPARE(library->shapeJitter(),closed);
        QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);const auto path=QUrl::fromLocalFile(temp.filePath("shape.vbr"));QVERIFY(client.exportBrush(path));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QVERIFY(client.importBrushes(path));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QCOMPARE(library->shapeJitter(),closed);
        library->setRadius(10);library->setRoundness(.3);library->setSpacing(1);library->setOpacity(.4);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto plain=library->selectedPreview();library->setShapeJitterEnabled(true);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(library->selectedPreview()!=plain);
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);InputSample sample;sample.position={32,64};sample.pressure=1;sample.tool=1;QVERIFY(client.beginStroke(sample));library->setShapeJitterEnabled(false);sample.position={96,64};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,128));const auto painted=client.frame();int outside=0;for(int y=0;y<128;++y)for(int x=0;x<128;++x){const auto alpha=painted.pixelColor(x,y).alpha();QVERIFY(alpha<=103);if(alpha>0 && std::abs(y-64)>4)++outside;}QVERIFY(outside>0);client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),painted);canvas->setClient(&client);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(reopened.ready());QTRY_COMPARE(reopened.brushLibrary()->shapeJitter(),closed);QCOMPARE(reopened.brushLibrary()->radius(),10.);QSignalSpy restopped(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(restopped.size(),1);
    }
    void scatterDeviceControlsPersistAndExportNativeContract() {
        BrushLibrary library;
        library.setScatterEnabled(true);
        library.setScatterAmount(2);
        library.setScatterCount(4);
        auto data=library.scatterDynamics();
        data["amountControl"]=QVariantMap{{"source","pressure"},{"minimum",0.25}};
        data["countControl"]=QVariantMap{{"source","fade"},{"minimum",0.5},{"fadeSteps",6}};
        QVERIFY(library.setScatterDynamics(data));
        const auto saved=library.snapshot();
        QCOMPARE(QJsonDocument::fromJson(saved).object().value("version").toInt(),35);
        BrushLibrary restored;
        QVERIFY(restored.restore(saved));
        QCOMPARE(restored.scatterDynamics(),data);
        const auto package=restored.exportPackage();
        const auto preset=QJsonDocument::fromJson(package).object().value("brushes").toArray().first().toObject();
        QCOMPARE(preset.value("behaviorVersion").toInt(),27);
        QVERIFY(preset.value("requiredCapabilities").toArray().contains("scatter-controls"));
        BrushLibrary imported;
        QVERIFY(imported.importPackage(package));
        QCOMPARE(imported.scatterDynamics(),data);
        auto bad=data;
        bad["amountControl"]=QVariantMap{{"source","pressure"},{"minimum",1.1}};
        QVERIFY(!restored.setScatterDynamics(bad));
        bad=data;
        bad["countControl"]=QVariantMap{{"source","fade"},{"minimum",0.5},{"fadeSteps",0}};
        QVERIFY(!restored.setScatterDynamics(bad));
        auto old=QJsonDocument::fromJson(saved).object();
        old["version"]=34;
        QVERIFY(!restored.restore(QJsonDocument(old).toJson()));
        const QJsonObject scatter{{"enabled",true},{"amount",2.},{"bothAxes",true},{"count",4},{"countJitter",0.},{"seed","0000000000000000"},{"amountControl",QJsonObject{{"source","pressure"},{"minimum",0.25}}},{"countControl",QJsonObject{{"source","fade"},{"minimum",0.5},{"fadeSteps",6}}}};
        const QJsonObject parameters{{"diameter",8.},{"spacing",0.04},{"dabOpacity",1.},{"hardness",1.},{"roundness",1.},{"angle",0.},{"flow",1.},{"pressureSize",false},{"pressureFlow",false},{"scatterDynamics",scatter}};
        const QJsonObject abrPreset{{"name","handoff-controls"},{"parameters",parameters},{"tipResource",QJsonValue::Null},{"texture",QJsonValue::Null},{"dualBrush",QJsonValue::Null},{"sourceResource",QString(64,'a')},{"partial",true}};
        QJsonObject handoff{{"version",11},{"format","abr"},{"presets",QJsonArray{abrPreset}}};
        BrushLibrary fromAbr;
        QVERIFY(fromAbr.importAbr(QJsonDocument(handoff).toJson()));
        QCOMPARE(fromAbr.scatterDynamics().value("amountControl").toMap().value("source").toString(),QString("pressure"));
        QCOMPARE(fromAbr.scatterDynamics().value("countControl").toMap().value("fadeSteps").toInt(),6);
        handoff["version"]=10;
        BrushLibrary oldAbr;
        QVERIFY(!oldAbr.importAbr(QJsonDocument(handoff).toJson()));
        auto dual=library.dualBrush();
        dual["enabled"]=true;
        dual["scatterDynamics"]=QVariantMap{{"enabled",true},{"amount",0.5},{"bothAxes",true},{"count",3},{"countJitter",0.25},{"seed","0000000000000000"},{"amountControl",QVariantMap{{"source","pressure"},{"minimum",0.2}}},{"countControl",QVariantMap{{"source","fade"},{"minimum",0.4},{"fadeSteps",5}}}};
        QVERIFY(library.setDualBrush(dual));
        const auto dualSaved=library.snapshot();
        QCOMPARE(QJsonDocument::fromJson(dualSaved).object().value("version").toInt(),35);
        BrushLibrary dualRestored;
        QVERIFY(dualRestored.restore(dualSaved));
        QCOMPARE(dualRestored.dualBrush(),library.dualBrush());
        const auto dualPackage=dualRestored.exportPackage();
        const auto dualPreset=QJsonDocument::fromJson(dualPackage).object().value("brushes").toArray().first().toObject();
        QCOMPARE(dualPreset.value("behaviorVersion").toInt(),27);
        QVERIFY(dualPreset.value("requiredCapabilities").toArray().contains("scatter-controls"));
        BrushLibrary dualImported;
        QVERIFY(dualImported.importPackage(dualPackage));
        QCOMPARE(dualImported.dualBrush(),library.dualBrush());
        dual=library.dualBrush();
        dual["mode"]="overlay";
        QVERIFY(library.setDualBrush(dual));
        const auto overlaySaved=library.snapshot();
        QCOMPARE(QJsonDocument::fromJson(overlaySaved).object().value("version").toInt(),36);
        BrushLibrary overlayRestored;
        QVERIFY(overlayRestored.restore(overlaySaved));
        QCOMPARE(overlayRestored.dualBrush().value("mode").toString(),QString("overlay"));
        const auto overlayPackage=overlayRestored.exportPackage();
        const auto overlayPreset=QJsonDocument::fromJson(overlayPackage).object().value("brushes").toArray().first().toObject();
        QCOMPARE(overlayPreset.value("behaviorVersion").toInt(),28);
        QVERIFY(overlayPreset.value("requiredCapabilities").toArray().contains("dual-cover-extended"));
        BrushLibrary overlayImported;
        QVERIFY(overlayImported.importPackage(overlayPackage));
        QCOMPARE(overlayImported.dualBrush().value("mode").toString(),QString("overlay"));
    }
    void scatterSettingsPersistExchangePreviewAndCapturedPixels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors)warnings.append(error.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY2(client.ready(),qPrintable(client.lastError()));auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:2")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        auto *toggle=findVisualItem(window->contentItem(),"settingsScatterEnabled");QVERIFY(toggle);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({toggle->width()/2,toggle->height()/2}).toPoint());QTRY_VERIFY(library->scatterEnabled());
        auto *axes=findVisualItem(window->contentItem(),"settingsScatterBothAxes");QVERIFY(axes);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,axes->mapToScene({axes->width()/2,axes->height()/2}).toPoint());QTRY_VERIFY(library->scatterBothAxes());
        auto *count=findVisualItem(window->contentItem(),"settingsScatterCountInput");QVERIFY(count);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,count->mapToScene({count->width()/2,count->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_4);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(library->scatterCount(),4);
        library->setScatterAmount(2);library->setScatterCountJitter(.5);auto config=library->scatterDynamics();config["seed"]="ffffffffffffffff";QVERIFY(library->setScatterDynamics(config));library->setScatterEnabled(false);QVERIFY(count->isEnabled());QCOMPARE(library->scatterAmount(),2.);library->setAngleControl("direction");
        const auto saved=library->snapshot();BrushLibrary restored;QVERIFY(restored.restore(saved));QCOMPARE(restored.scatterDynamics(),library->scatterDynamics());QCOMPARE(QJsonDocument::fromJson(saved).object().value("version").toInt(),8);
        const auto before=restored.snapshot();auto bad=config;bad["seed"]=qulonglong(-1);QVERIFY(!restored.setScatterDynamics(bad));bad=config;bad["count"]=2.5;QVERIFY(!restored.setScatterDynamics(bad));QCOMPARE(restored.snapshot(),before);auto old=QJsonDocument::fromJson(saved).object();old["version"]=7;QVERIFY(!restored.restore(QJsonDocument(old).toJson()));QCOMPARE(restored.snapshot(),before);
        // A later device-only row must not downgrade a directory containing scatter.
        QVERIFY(restored.select("round-fine"));restored.setAngleControl("tilt");QCOMPARE(QJsonDocument::fromJson(restored.snapshot()).object().value("version").toInt(),8);BrushLibrary mixed;QVERIFY(mixed.restore(restored.snapshot()));auto batch=QJsonDocument::fromJson(library->exportPackage()).object();auto brushes=batch.value("brushes").toArray();brushes.append(QJsonDocument::fromJson(restored.exportPackage()).object().value("brushes").toArray().first());batch["brushes"]=brushes;BrushLibrary importer;QVERIFY(importer.importPackage(QJsonDocument(batch).toJson()));QCOMPARE(QJsonDocument::fromJson(importer.snapshot()).object().value("version").toInt(),8);QCOMPARE(importer.scatterDynamics().value("seed").toString(),QString("ffffffffffffffff"));BrushLibrary defaults;defaults.setScatterEnabled(true);defaults.setScatterEnabled(false);QCOMPARE(QJsonDocument::fromJson(defaults.snapshot()).object().value("version").toInt(),2);
        const auto copy=library->saveCopy("散布副本");QVERIFY(!copy.isEmpty());QCOMPARE(library->scatterDynamics().value("seed").toString(),QString("ffffffffffffffff"));QVERIFY(library->select("round-fine"));QVERIFY(!library->scatterEnabled());QVERIFY(library->select(copy));
        QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);const auto path=QUrl::fromLocalFile(temp.filePath("scatter.vbr"));QVERIFY(client.exportBrush(path));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QVERIFY(client.importBrushes(path));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QCOMPARE(library->scatterDynamics().value("seed").toString(),QString("ffffffffffffffff"));QVERIFY(!library->scatterEnabled());QCOMPARE(library->scatterCountJitter(),.5);
        library->setRadius(4);library->setSpacing(.2);library->setOpacity(.4);library->setFlow(1);library->setPressureSize(false);library->setPressureFlow(false);library->setAngleControl("fixed");library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto plainPreview=library->selectedPreview();library->setScatterEnabled(true);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(library->selectedPreview()!=plainPreview);
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);InputSample sample;sample.position={32,64};sample.pressure=1;sample.tool=1;QVERIFY(client.beginStroke(sample));library->setScatterEnabled(false);sample.position={96,64};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,128));const auto painted=client.frame();int outside=0;for(int y=0;y<128;++y)for(int x=0;x<128;++x){const int alpha=painted.pixelColor(x,y).alpha();QVERIFY(alpha<=103);if(alpha>0 && std::abs(y-64)>5)++outside;}QVERIFY(outside>50);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame().pixelColor(64,64).alpha(),0);client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),painted);canvas->setClient(&client);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(reopened.ready());QTRY_COMPARE(reopened.brushLibrary()->scatterDynamics().value("seed").toString(),QString("ffffffffffffffff"));QCOMPARE(reopened.brushLibrary()->scatterAmount(),2.);QVERIFY(!reopened.brushLibrary()->scatterEnabled());QSignalSpy restopped(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(restopped.size(),1);
    }
    void deviceDynamicSettingsSaveExchangeAndExecuteCapturedGeometry() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors)warnings.append(error.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:1")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        auto *control=findVisualItem(window->contentItem(),"settingsAngleControl");QVERIFY(control);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,control->mapToScene({control->width()/2,control->height()/2}).toPoint());QQuickItem *rotationOption=nullptr;QTRY_VERIFY((rotationOption=findVisualItem(window->contentItem(),"settingsAngleControlOption:2")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,rotationOption->mapToScene({rotationOption->width()/2,rotationOption->height()/2}).toPoint());QTRY_COMPARE(library->angleControl(),QString("pen-rotation"));
        auto *toggle=findVisualItem(window->contentItem(),"settingsTiltRoundness");QVERIFY(toggle);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({toggle->width()/2,toggle->height()/2}).toPoint());QTRY_VERIFY(library->tiltRoundness());library->setMinimumRoundness(.2);library->setTiltRoundness(false);QCOMPARE(library->minimumRoundness(),.2);
        const auto saved=library->snapshot();BrushLibrary restored;QVERIFY(restored.restore(saved));QCOMPARE(restored.tipDynamics(),library->tipDynamics());QCOMPARE(QJsonDocument::fromJson(saved).object().value("version").toInt(),7);const auto validState=restored.snapshot();auto bad=QJsonDocument::fromJson(saved).object();bad["version"]=6;QVERIFY(!restored.restore(QJsonDocument(bad).toJson()));QCOMPARE(restored.snapshot(),validState);BrushLibrary defaults;defaults.setAngleControl("direction");defaults.setAngleControl("fixed");QCOMPARE(QJsonDocument::fromJson(defaults.snapshot()).object().value("version").toInt(),2);const auto copy=library->saveCopy("设备笔尖副本");QVERIFY(!copy.isEmpty());QCOMPARE(library->angleControl(),QString("pen-rotation"));library->setAngleControl("invalid");library->setMinimumRoundness(0);QCOMPARE(library->angleControl(),QString("pen-rotation"));QCOMPARE(library->minimumRoundness(),.2);
        QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);const auto path=QUrl::fromLocalFile(temp.filePath("device.vbr"));QVERIFY(client.exportBrush(path));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());library->setAngleControl("fixed");QVERIFY(client.importBrushes(path));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QCOMPARE(library->angleControl(),QString("pen-rotation"));QCOMPARE(library->minimumRoundness(),.2);
        library->setRoundness(.2);library->setRadius(8);library->setOpacity(1);library->setFlow(1);library->setPressureSize(false);library->setPressureFlow(false);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto rotatingPreview=library->selectedPreview();library->setAngleControl("fixed");library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(library->selectedPreview()!=rotatingPreview);library->setAngleControl("pen-rotation");
        client.newTransparentDocument(40,40);QTRY_COMPARE(client.documentWidth(),40);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);InputSample sample;sample.position={16.5,16.5};sample.pressure=1;sample.tool=1;sample.capabilities=4;sample.rotation=90;QVERIFY(client.beginStroke(sample));library->setAngleControl("fixed");client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(40,40));QVERIFY(client.frame().pixelColor(16,22).alpha()>200);QCOMPARE(client.frame().pixelColor(22,16).alpha(),0);canvas->setClient(&client);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reopened(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(reopened.ready());QTRY_COMPARE(reopened.brushLibrary()->angleControl(),QString("fixed"));QTRY_COMPARE(reopened.brushLibrary()->minimumRoundness(),.2);QCOMPARE(reopened.brushLibrary()->radius(),8.);QSignalSpy restopped(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(restopped.size(),1);
    }
    void pressureCurveEditorPersistsAndExchangesNativeCurves() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &error:errors)warnings.append(error.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:1")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        QQuickItem *graph=nullptr;QTRY_VERIFY((graph=findVisualItem(window->contentItem(),"pressureSizeCurve")));auto *add=findVisualItem(window->contentItem(),"pressureSizeAdd");QVERIFY(add);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,add->mapToScene({add->width()/2,add->height()/2}).toPoint());
        QTRY_COMPARE(library->pressureDynamics().value("size").toMap().value("curve").toList().size(),3);
        const QVariantList curve{QVariantList{0.,.2},QVariantList{.5,.8},QVariantList{1.,1.}};QVERIFY(library->setPressureControl("size",curve,.1));QVERIFY(!library->setPressureControl("size",QVariantList{QVariantList{0.,0.},QVariantList{0.,1.}},0));
        QCOMPARE(library->pressureDynamics().value("size").toMap().value("interpolation").toString(),QString("smooth-pchip-v1"));
        auto *editor=findVisualItem(window->contentItem(),"pressureSizeEditor");QVERIFY(editor);QTRY_COMPARE(editor->property("samples").toList().size(),257);QVERIFY(std::abs(editor->property("samples").toList()[64].toDouble()-.5625)<0.00001);
        const auto preview=qEnvironmentVariable("DRAWVERSE_PRESSURE_SMOOTH_PREVIEW");if(!preview.isEmpty())QVERIFY(window->grabWindow().save(preview));
        library->setPressureSize(false);BrushLibrary restored;QVERIFY(restored.restore(library->snapshot()));QCOMPARE(restored.pressureDynamics(),library->pressureDynamics());QVERIFY(!restored.pressureSize());QCOMPARE(QJsonDocument::fromJson(library->snapshot()).object().value("version").toInt(),13);
        category=findVisualItem(window->contentItem(),"brushCategory:6");QVERIFY(category);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        QQuickItem *opacityToggle=nullptr;QTRY_VERIFY((opacityToggle=findVisualItem(window->contentItem(),"settingsPressureOpacity")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,opacityToggle->mapToScene({opacityToggle->width()/2,opacityToggle->height()/2}).toPoint());QTRY_VERIFY(library->pressureOpacity());
        QVERIFY(library->setPressureControl("opacity",curve,.2));library->setPressureOpacity(false);QVERIFY(!library->pressureOpacity());QCOMPARE(library->pressureDynamics().value("opacity").toMap().value("minimum").toDouble(),.2);library->setPressureOpacity(true);
        BrushLibrary opacityRestored;QVERIFY(opacityRestored.restore(library->snapshot()));QVERIFY(opacityRestored.pressureOpacity());QCOMPARE(opacityRestored.pressureDynamics(),library->pressureDynamics());QCOMPARE(QJsonDocument::fromJson(library->snapshot()).object().value("version").toInt(),13);
        library->setPressureSize(true);library->requestPreview(library->selectedId());QTRY_VERIFY2_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),qPrintable(library->previewMessage()),10000);
        QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);const auto path=temp.filePath("curves.vbr");QVERIFY(client.exportBrush(QUrl::fromLocalFile(path)));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());const auto expected=library->pressureDynamics();library->resetPressureControl("size");QVERIFY(client.importBrushes(QUrl::fromLocalFile(path)));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QCOMPARE(library->pressureDynamics(),expected);
        client.newTransparentDocument(40,40);QTRY_COMPARE(client.documentWidth(),40);QTRY_VERIFY(client.ready());library->setRadius(8);library->setOpacity(.4);library->setPressureSize(false);library->setPressureFlow(false);library->setFlow(1);QVERIFY(library->setPressureControl("opacity",QVariantList{QVariantList{0.,0.},QVariantList{.5,.25},QVariantList{1.,1.}},.2));
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);InputSample sample;sample.position={16.5,16.5};sample.pressure=.25;sample.tool=1;sample.capabilities=1;QVERIFY(client.beginStroke(sample));library->setPressureOpacity(false);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(40,40));QVERIFY(std::abs(client.frame().pixelColor(16,16).alpha()-27)<=1);canvas->setClient(&client);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void proceduralBrushTipPersistencePreviewCursorAndPixels() {
        BrushLibrary model;model.setHardness(.3);model.setRoundness(.2);model.setAngle(90);model.setFlow(.4);model.setPressureSize(false);model.setPressureFlow(false);
        const auto state=model.snapshot();BrushLibrary restored;QVERIFY(restored.restore(state));QCOMPARE(restored.hardness(),.3);QCOMPARE(restored.roundness(),.2);QCOMPARE(restored.angle(),90.);QCOMPARE(restored.flow(),.4);QVERIFY(!restored.pressureSize() && !restored.pressureFlow());
        const auto restoredState=restored.snapshot();auto bad=QJsonDocument::fromJson(state).object();bad["selected"]="round-fine";bad["recentColors"]=QJsonArray{"invalid color"};QVERIFY(!restored.restore(QJsonDocument(bad).toJson()));QCOMPARE(restored.snapshot(),restoredState);
        auto old=QJsonDocument::fromJson(state).object();old["version"]=1;QVERIFY(restored.restore(QJsonDocument(old).toJson()));QCOMPARE(restored.hardness(),1.);QCOMPARE(restored.roundness(),1.);QVERIFY(restored.pressureSize() && restored.pressureFlow());
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();library->setRadius(10);library->setRoundness(.2);library->setAngle(90);library->setPressureSize(false);library->setPressureFlow(false);
        library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto first=library->selectedPreview();library->setHardness(.2);library->requestPreview(library->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(first!=library->selectedPreview());library->setHardness(1);
        client.newTransparentDocument(40,40);QTRY_COMPARE(client.documentWidth(),40);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->setClient(nullptr);
        InputSample sample;sample.position={20.5,20.5};sample.pressure=.1f;sample.tool=1;QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        client.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(40,40));QVERIFY(client.frame().pixelColor(20,27).alpha()>200);QCOMPARE(client.frame().pixelColor(27,20).alpha(),0);
        canvas->setClient(&client);auto *cursor=findVisualItem(window->contentItem(),"brushCursorOutline");QVERIFY(cursor);QCOMPARE(cursor->property("rotation").toDouble(),90.);QVERIFY(std::abs(cursor->height()/cursor->width()-.2)<1e-6);
        const auto screenshot=qEnvironmentVariable("DRAWVERSE_BRUSH_PREVIEW");if(!screenshot.isEmpty()){auto group=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(group);for(auto *w:applicationWindows())if(auto *q=qobject_cast<QQuickWindow*>(w))if(findVisualItem(q->contentItem(),"selectedBrushName")){q->resize(350,440);QTest::qWait(200);QVERIFY(q->grabWindow().save(screenshot+".tip.png"));}}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void vbrSourceScopeAndFreshStorePreserveOriginalAfterCurrentTipChanges() {
        QTemporaryDir temp;const auto inputPath=temp.filePath("input.abr");QFile input(inputPath);QVERIFY(input.open(QIODevice::WriteOnly));input.write(abrEntryFixture());input.close();const auto exported=QUrl::fromLocalFile(temp.filePath("source.vbr"));QString sourceId,baselineId;
        {
            PaintCoreClient client(nullptr,temp.filePath("origin/state.ini"));QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushes(QUrl::fromLocalFile(inputPath)));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());sourceId=library->sourceResource();baselineId=library->maskResource();QCOMPARE(baselineId.size(),64);
            WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &error:errors)warnings.append(error.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);
            for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush"))workspace.setActive(group.value("id").toString(),"brush");}
            QQuickItem *panel=nullptr;QTRY_VERIFY((panel=findVisualItem(window->contentItem(),"brushLibraryPanel")));QVERIFY(QMetaObject::invokeMethod(panel,"beginSelectionExport"));auto *options=window->findChild<QObject*>("brushSourceExportOptions");QVERIFY(options);QTRY_VERIFY(options->property("visible").toBool());auto *check=window->findChild<QQuickItem*>("includeOriginalAbr");auto *confirm=window->findChild<QQuickItem*>("confirmSourceExport");QVERIFY(check && confirm);QVERIFY(!check->property("checked").toBool());QVERIFY(!confirm->isEnabled());
            QVERIFY(!client.exportBrush(exported));QVERIFY(!QFileInfo::exists(exported.toLocalFile()));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,check->mapToScene({check->width()/2,check->height()/2}).toPoint());QTRY_VERIFY(check->property("checked").toBool());QTRY_VERIFY(confirm->isEnabled());QVERIFY(QMetaObject::invokeMethod(options,"reject"));
            const auto expected=library->selectedId();QVERIFY(library->select("round-fine"));QVERIFY(!client.exportBrush(exported,true,expected));QVERIFY(!QFileInfo::exists(exported.toLocalFile()));QVERIFY(library->select(expected));library->useProceduralTip();QVERIFY(library->maskResource().isEmpty());QCOMPARE(library->sourceResource(),sourceId);library->setScatterAmount(2);library->setScatterCount(3);auto scatter=library->scatterDynamics();scatter["seed"]="ffffffffffffffff";QVERIFY(library->setScatterDynamics(scatter));library->setShapeSizeJitter(.5);auto shape=library->shapeJitter();shape["seed"]="ffffffffffffffff";QVERIFY(library->setShapeJitter(shape));library->setHueJitter(.5);auto color=library->colorDynamics();color["seed"]="ffffffffffffffff";QVERIFY(library->setColorDynamics(color));
            QVERIFY(library->setTextureResource(library->selectedId(),baselineId));auto texture=library->texture();texture["scale"]=2.;texture["angle"]=17.;texture["depth"]=.25;QVERIFY(library->setTexture(texture));
            QVERIFY(client.exportBrush(exported,true,expected));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QVERIFY(QFileInfo::exists(exported.toLocalFile()));QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        }
        QVERIFY(QFile::remove(inputPath));QVERIFY(QDir(temp.filePath("origin")).removeRecursively());
        const auto destination=temp.filePath("fresh/state.ini");QString selected;
        {
            PaintCoreClient client(nullptr,destination);QTRY_VERIFY(client.ready());QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushes(exported));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());auto *library=client.brushLibrary();QCOMPARE(library->presets().size(),5);QCOMPARE(library->sourceResource(),sourceId);QVERIFY(library->maskResource().isEmpty());QVERIFY(QFileInfo::exists(temp.filePath("fresh/brush-resources/"+baselineId+".png")));QCOMPARE(QJsonDocument::fromJson(library->snapshot()).object().value("version").toInt(),20);QCOMPARE(library->colorDynamics().value("seed").toString(),QString("ffffffffffffffff"));QVERIFY(!library->colorDynamicsEnabled());QCOMPARE(library->shapeJitter().value("seed").toString(),QString("ffffffffffffffff"));QVERIFY(!library->shapeJitterEnabled());QCOMPARE(library->scatterDynamics().value("seed").toString(),QString("ffffffffffffffff"));QVERIFY(!library->scatterEnabled());QCOMPARE(library->scatterCount(),3);QCOMPARE(library->texture().value("resource").toString(),baselineId);QCOMPARE(library->texture().value("angle").toDouble(),17.);QVERIFY(!library->texture().value("enabled").toBool());selected=library->selectedId();
            QVERIFY(client.inspectBrushSource());QTRY_VERIFY(!client.brushSourceBusy());QCOMPARE(client.brushSourceInfo().value("originalName").toString(),QString("Same"));QCOMPARE(client.brushSourceInfo().value("tipResource").toString(),baselineId);QVERIFY(!client.brushSourceInfo().value("report").toList().isEmpty());
            QVERIFY(client.importBrushes(exported));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QCOMPARE(library->presets().size(),6);QVERIFY(library->selectedId()!=selected);QCOMPARE(library->sourceResource(),sourceId);selected=library->selectedId();library->setRadius(4);
            library->requestPreview(selected);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);client.newTransparentDocument(24,24);QTRY_COMPARE(client.documentWidth(),24);QTRY_VERIFY(client.ready());InputSample sample;sample.position={12.5,12.5};QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);client.requestViewport(0,{0,0,24,24},{24,24});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(24,24));QVERIFY(client.frame().pixelColor(12,12).alpha()>200);
            QVERIFY(client.exportBrush(QUrl::fromLocalFile(temp.filePath("fresh-copy.vbr")),true));QTRY_COMPARE(finished.size(),3);QVERIFY(finished.last()[0].toBool());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        }
        PaintCoreClient restored(nullptr,destination);QTRY_VERIFY(restored.ready());QTRY_COMPARE(restored.brushLibrary()->selectedId(),selected);QCOMPARE(restored.brushLibrary()->sourceResource(),sourceId);QCOMPARE(restored.brushLibrary()->texture().value("resource").toString(),baselineId);QCOMPARE(restored.brushLibrary()->texture().value("depth").toDouble(),.25);QVERIFY(restored.inspectBrushSource());QTRY_VERIFY(!restored.brushSourceBusy());QCOMPARE(restored.brushSourceInfo().value("tipResource").toString(),baselineId);QSignalSpy stopped(&restored,&PaintCoreClient::stopped);restored.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void abrOpacityImportsDrawsResetsAndExchangesWithVersionedSource() {
        QTemporaryDir temp;const auto path=temp.filePath("opacity.abr");QFile file(path);QVERIFY(file.open(QIODevice::WriteOnly));file.write(abrEntryFixture(false,25.5));file.close();
        PaintCoreClient client(nullptr,temp.filePath("opacity.ini"));QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();QSignalSpy files(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushes(QUrl::fromLocalFile(path)));QTRY_COMPARE(files.size(),1);QVERIFY2(files.last()[0].toBool(),qPrintable(client.lastError()));QCOMPARE(library->presets().size(),7);QVERIFY(library->select(library->presets()[5].toMap().value("id").toString()));QVERIFY(qAbs(library->opacity()-.255)<1e-6);
        QVERIFY(client.inspectBrushSource());QTRY_VERIFY(!client.brushSourceBusy());QCOMPARE(client.brushSourceInfo().value("converterVersion").toInt(),2);QVERIFY(qAbs(client.brushSourceInfo().value("baseline").toMap().value("dabOpacity").toDouble()-.255)<1e-6);const auto source=library->sourceResource();
        library->setOpacity(.8);QVERIFY(library->resetPreset());QVERIFY(qAbs(library->opacity()-.255)<1e-6);QCOMPARE(library->sourceResource(),source);
        client.newTransparentDocument(128,64);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());InputSample sample;sample.position={32.5,32.5};sample.pressure=1;QVERIFY(client.beginStroke(sample));for(const double x:{96.5,32.5,96.5,32.5}){sample.position={x,32.5};client.strokeTo(sample);}client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,64));const auto painted=client.frame();QVERIFY(painted.pixelColor(48,32).alpha()>=64);for(int y=0;y<64;++y)for(int x=0;x<128;++x)QVERIFY(painted.pixelColor(x,y).alpha()<=66);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),painted);
        const auto vbr=QUrl::fromLocalFile(temp.filePath("opacity.vbr"));QVERIFY(client.exportBrush(vbr,true));QTRY_COMPARE(files.size(),2);QVERIFY(files.last()[0].toBool());QVERIFY(QFile::remove(path));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient restored(nullptr,temp.filePath("fresh/import.ini"));QTRY_VERIFY(restored.ready());QSignalSpy imported(&restored,&PaintCoreClient::brushFileFinished);QVERIFY(restored.importBrushes(vbr));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QVERIFY(qAbs(restored.brushLibrary()->opacity()-.255)<1e-6);QCOMPARE(restored.brushLibrary()->sourceResource(),source);QVERIFY(restored.inspectBrushSource());QTRY_VERIFY(!restored.brushSourceBusy());QCOMPARE(restored.brushSourceInfo().value("converterVersion").toInt(),2);QSignalSpy restopped(&restored,&PaintCoreClient::stopped);restored.shutdown();QTRY_COMPARE(restopped.size(),1);
    }
    void abrPressureImportsResetsPersistsAndExchangesRealPixels() {
        QTemporaryDir temp;const auto path=temp.filePath("opacity.abr");QFile file(path);QVERIFY(file.open(QIODevice::WriteOnly));file.write(abrEntryFixture(false,25.5,true));file.close();
        PaintCoreClient client(nullptr,temp.filePath("opacity.ini"));QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();QSignalSpy files(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushes(QUrl::fromLocalFile(path)));QTRY_COMPARE(files.size(),1);QVERIFY(files.last()[0].toBool());QCOMPARE(library->presets().size(),7);QVERIFY(library->select(library->presets()[5].toMap().value("id").toString()));QVERIFY(qAbs(library->opacity()-.255)<1e-6);
        QVERIFY(client.inspectBrushSource());QTRY_VERIFY(!client.brushSourceBusy());QCOMPARE(client.brushSourceInfo().value("converterVersion").toInt(),3);QVERIFY(qAbs(client.brushSourceInfo().value("baseline").toMap().value("dabOpacity").toDouble()-.255)<1e-6);const auto source=library->sourceResource();
        const auto pressure=library->pressureDynamics();QVERIFY(library->pressureSize() && library->pressureFlow() && library->pressureOpacity());QVERIFY(qAbs(pressure.value("size").toMap().value("minimum").toDouble()-.2)<1e-6);QVERIFY(qAbs(pressure.value("flow").toMap().value("minimum").toDouble()-.4)<1e-6);QVERIFY(qAbs(pressure.value("opacity").toMap().value("minimum").toDouble()-.3)<1e-6);
        const auto before=library->snapshot();QJsonObject downgrade{{"version",1},{"format","abr"},{"presets",QJsonArray{QJsonObject{{"name","guard"},{"parameters",QJsonObject::fromVariantMap(client.brushSourceInfo().value("baseline").toMap())},{"sourceResource",source},{"tipResource",QJsonValue::Null},{"partial",false}}}}};QVERIFY(!library->importAbr(QJsonDocument(downgrade).toJson()));QCOMPARE(library->snapshot(),before);
        library->setPressureSize(false);library->setPressureFlow(false);library->setPressureOpacity(false);
        library->setOpacity(.8);QVERIFY(library->resetPreset());QVERIFY(qAbs(library->opacity()-.255)<1e-6);QCOMPARE(library->sourceResource(),source);QCOMPARE(library->pressureDynamics(),pressure);QVERIFY(library->pressureSize() && library->pressureFlow() && library->pressureOpacity());
        client.newTransparentDocument(128,64);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());InputSample sample;sample.position={32.5,32.5};sample.pressure=.5;sample.tool=1;sample.capabilities=17;QVERIFY(client.beginStroke(sample));for(const double x:{96.5,32.5,96.5,32.5}){sample.position={x,32.5};client.strokeTo(sample);}client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,64));const auto painted=client.frame();QVERIFY(painted.pixelColor(48,32).alpha()>=41);for(int y=0;y<64;++y)for(int x=0;x<128;++x)QVERIFY(painted.pixelColor(x,y).alpha()<=43);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),painted);
        const auto vbr=QUrl::fromLocalFile(temp.filePath("opacity.vbr"));QVERIFY(client.exportBrush(vbr,true));QTRY_COMPARE(files.size(),2);QVERIFY(files.last()[0].toBool());QVERIFY(QFile::remove(path));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient restored(nullptr,temp.filePath("fresh/import.ini"));QTRY_VERIFY(restored.ready());QSignalSpy imported(&restored,&PaintCoreClient::brushFileFinished);QVERIFY(restored.importBrushes(vbr));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QVERIFY(qAbs(restored.brushLibrary()->opacity()-.255)<1e-6);QCOMPARE(restored.brushLibrary()->sourceResource(),source);QVERIFY(restored.inspectBrushSource());QTRY_VERIFY(!restored.brushSourceBusy());QCOMPARE(restored.brushSourceInfo().value("converterVersion").toInt(),3);QCOMPARE(restored.brushLibrary()->pressureDynamics(),pressure);QVERIFY(restored.brushLibrary()->pressureSize() && restored.brushLibrary()->pressureFlow() && restored.brushLibrary()->pressureOpacity());
        restored.newTransparentDocument(128,64);QTRY_COMPARE(restored.documentWidth(),128);QTRY_VERIFY(!restored.layerEditBusy());sample.position={32.5,32.5};QVERIFY(restored.beginStroke(sample));for(const double x:{96.5,32.5,96.5,32.5}){sample.position={x,32.5};restored.strokeTo(sample);}restored.endStroke();QTRY_COMPARE(restored.undoDepth(),1);QTRY_VERIFY(!restored.layerEditBusy());restored.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(restored.frameRevision()>=restored.revision() && restored.frame().size()==QSize(128,64));QCOMPARE(restored.frame(),painted);
        QSignalSpy restopped(&restored,&PaintCoreClient::stopped);restored.shutdown();QTRY_COMPARE(restopped.size(),1);PaintCoreClient reopened(nullptr,temp.filePath("fresh/import.ini"));QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->pressureDynamics(),pressure);QVERIFY(reopened.brushLibrary()->pressureOpacity());QSignalSpy again(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(again.size(),1);
    }
    void abrScatterClosedAndActiveImportResetPersistAndExchangeRealPixels() {
        for(const bool enabled:{false,true}) {
        QTemporaryDir temp;const auto path=temp.filePath("opacity.abr");QFile file(path);QVERIFY(file.open(QIODevice::WriteOnly));file.write(abrEntryFixture(false,25.5,true,int(enabled)));file.close();
        PaintCoreClient client(nullptr,temp.filePath("opacity.ini"));QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();QSignalSpy files(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushes(QUrl::fromLocalFile(path)));QTRY_COMPARE(files.size(),1);QVERIFY(files.last()[0].toBool());QCOMPARE(library->presets().size(),7);QVERIFY(library->select(library->presets()[5].toMap().value("id").toString()));QVERIFY(qAbs(library->opacity()-.255)<1e-6);
        QVERIFY(client.inspectBrushSource());QTRY_VERIFY(!client.brushSourceBusy());QCOMPARE(client.brushSourceInfo().value("converterVersion").toInt(),4);QVERIFY(qAbs(client.brushSourceInfo().value("baseline").toMap().value("dabOpacity").toDouble()-.255)<1e-6);const auto source=library->sourceResource();
        const auto pressure=library->pressureDynamics();QVERIFY(library->pressureSize() && library->pressureFlow() && library->pressureOpacity());QVERIFY(qAbs(pressure.value("size").toMap().value("minimum").toDouble()-.2)<1e-6);QVERIFY(qAbs(pressure.value("flow").toMap().value("minimum").toDouble()-.4)<1e-6);QVERIFY(qAbs(pressure.value("opacity").toMap().value("minimum").toDouble()-.3)<1e-6);
        const auto scatter=library->scatterDynamics();QCOMPARE(library->scatterEnabled(),enabled);QCOMPARE(library->scatterBothAxes(),true);QCOMPARE(library->scatterAmount(),.75);QCOMPARE(library->scatterCount(),3);QCOMPARE(library->scatterCountJitter(),.5);
        const auto before=library->snapshot();QJsonObject downgrade{{"version",2},{"format","abr"},{"presets",QJsonArray{QJsonObject{{"name","guard"},{"parameters",QJsonObject::fromVariantMap(client.brushSourceInfo().value("baseline").toMap())},{"sourceResource",source},{"tipResource",QJsonValue::Null},{"partial",false}}}}};QVERIFY(!library->importAbr(QJsonDocument(downgrade).toJson()));QCOMPARE(library->snapshot(),before);
        library->setScatterEnabled(!enabled);library->setScatterAmount(.1);
        library->setPressureSize(false);library->setPressureFlow(false);library->setPressureOpacity(false);
        library->setOpacity(.8);QVERIFY(library->resetPreset());QVERIFY(qAbs(library->opacity()-.255)<1e-6);QCOMPARE(library->sourceResource(),source);QCOMPARE(library->pressureDynamics(),pressure);QCOMPARE(library->scatterDynamics(),scatter);QVERIFY(library->pressureSize() && library->pressureFlow() && library->pressureOpacity());
        client.newTransparentDocument(128,64);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());InputSample sample;sample.position={32.5,32.5};sample.pressure=.5;sample.tool=1;sample.capabilities=17;QVERIFY(client.beginStroke(sample));for(const double x:{96.5,32.5,96.5,32.5}){sample.position={x,32.5};client.strokeTo(sample);}client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,64));const auto painted=client.frame();QVERIFY(painted.pixelColor(48,32).alpha()>=41);for(int y=0;y<64;++y)for(int x=0;x<128;++x)QVERIFY(painted.pixelColor(x,y).alpha()<=43);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),painted);
        const auto vbr=QUrl::fromLocalFile(temp.filePath("opacity.vbr"));QVERIFY(client.exportBrush(vbr,true));QTRY_COMPARE(files.size(),2);QVERIFY(files.last()[0].toBool());QVERIFY(QFile::remove(path));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient restored(nullptr,temp.filePath("fresh/import.ini"));QTRY_VERIFY(restored.ready());QSignalSpy imported(&restored,&PaintCoreClient::brushFileFinished);QVERIFY(restored.importBrushes(vbr));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QVERIFY(qAbs(restored.brushLibrary()->opacity()-.255)<1e-6);QCOMPARE(restored.brushLibrary()->sourceResource(),source);QVERIFY(restored.inspectBrushSource());QTRY_VERIFY(!restored.brushSourceBusy());QCOMPARE(restored.brushSourceInfo().value("converterVersion").toInt(),4);QCOMPARE(restored.brushLibrary()->pressureDynamics(),pressure);QCOMPARE(restored.brushLibrary()->scatterDynamics(),scatter);QVERIFY(restored.brushLibrary()->pressureSize() && restored.brushLibrary()->pressureFlow() && restored.brushLibrary()->pressureOpacity());
        restored.newTransparentDocument(128,64);QTRY_COMPARE(restored.documentWidth(),128);QTRY_VERIFY(!restored.layerEditBusy());sample.position={32.5,32.5};QVERIFY(restored.beginStroke(sample));for(const double x:{96.5,32.5,96.5,32.5}){sample.position={x,32.5};restored.strokeTo(sample);}restored.endStroke();QTRY_COMPARE(restored.undoDepth(),1);QTRY_VERIFY(!restored.layerEditBusy());restored.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(restored.frameRevision()>=restored.revision() && restored.frame().size()==QSize(128,64));QCOMPARE(restored.frame(),painted);
        QSignalSpy restopped(&restored,&PaintCoreClient::stopped);restored.shutdown();QTRY_COMPARE(restopped.size(),1);PaintCoreClient reopened(nullptr,temp.filePath("fresh/import.ini"));QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->pressureDynamics(),pressure);QCOMPARE(reopened.brushLibrary()->scatterDynamics(),scatter);QVERIFY(reopened.brushLibrary()->pressureOpacity());QSignalSpy again(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(again.size(),1);
        }
    }
    void abrColorClosedPerStrokeAndPerTipImportResetPersistAndExchangeRealPixels() {
        for(const int config:{0,1,2}) { const bool enabled=config!=0;const bool perTip=config!=1;
        QTemporaryDir temp;const auto path=temp.filePath("opacity.abr");QFile file(path);QVERIFY(file.open(QIODevice::WriteOnly));file.write(abrEntryFixture(false,25.5,true,1,int(enabled),perTip));file.close();
        PaintCoreClient client(nullptr,temp.filePath("opacity.ini"));QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();QSignalSpy files(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushes(QUrl::fromLocalFile(path)));QTRY_COMPARE(files.size(),1);QVERIFY(files.last()[0].toBool());QCOMPARE(library->presets().size(),7);QVERIFY(library->select(library->presets()[5].toMap().value("id").toString()));QVERIFY(qAbs(library->opacity()-.255)<1e-6);
        QVERIFY(client.inspectBrushSource());QTRY_VERIFY(!client.brushSourceBusy());QCOMPARE(client.brushSourceInfo().value("converterVersion").toInt(),5);QVERIFY(qAbs(client.brushSourceInfo().value("baseline").toMap().value("dabOpacity").toDouble()-.255)<1e-6);const auto source=library->sourceResource();
        const auto pressure=library->pressureDynamics();QVERIFY(library->pressureSize() && library->pressureFlow() && library->pressureOpacity());QVERIFY(qAbs(pressure.value("size").toMap().value("minimum").toDouble()-.2)<1e-6);QVERIFY(qAbs(pressure.value("flow").toMap().value("minimum").toDouble()-.4)<1e-6);QVERIFY(qAbs(pressure.value("opacity").toMap().value("minimum").toDouble()-.3)<1e-6);
        const auto scatter=library->scatterDynamics();QCOMPARE(library->scatterEnabled(),true);QCOMPARE(library->scatterBothAxes(),true);QCOMPARE(library->scatterAmount(),.75);QCOMPARE(library->scatterCount(),3);QCOMPARE(library->scatterCountJitter(),.5);
        const auto color=library->colorDynamics();QCOMPARE(library->colorDynamicsEnabled(),enabled);QCOMPARE(library->colorPerTip(),perTip);QVERIFY(qAbs(library->foregroundBackgroundJitter()-.8)<1e-6);QVERIFY(qAbs(color.value("hue").toDouble()-.2)<1e-6);QVERIFY(qAbs(color.value("saturation").toDouble()-.3)<1e-6);QVERIFY(qAbs(color.value("value").toDouble()-.1)<1e-6);
        const auto before=library->snapshot();QJsonObject downgrade{{"version",3},{"format","abr"},{"presets",QJsonArray{QJsonObject{{"name","guard"},{"parameters",QJsonObject::fromVariantMap(client.brushSourceInfo().value("baseline").toMap())},{"sourceResource",source},{"tipResource",QJsonValue::Null},{"partial",false}}}}};QVERIFY(!library->importAbr(QJsonDocument(downgrade).toJson()));QCOMPARE(library->snapshot(),before);
        library->setColorDynamicsEnabled(!enabled);library->setForegroundBackgroundJitter(.1);library->setColorPerTip(!perTip);
        library->setPressureSize(false);library->setPressureFlow(false);library->setPressureOpacity(false);
        library->setOpacity(.8);QVERIFY(library->resetPreset());QVERIFY(qAbs(library->opacity()-.255)<1e-6);QCOMPARE(library->sourceResource(),source);QCOMPARE(library->pressureDynamics(),pressure);QCOMPARE(library->scatterDynamics(),scatter);QCOMPARE(library->colorDynamics(),color);QVERIFY(library->pressureSize() && library->pressureFlow() && library->pressureOpacity());
        client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);client.newTransparentDocument(128,64);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());InputSample sample;sample.position={32.5,32.5};sample.pressure=.5;sample.tool=1;sample.capabilities=17;QVERIFY(client.beginStroke(sample));for(const double x:{96.5,32.5,96.5,32.5}){sample.position={x,32.5};client.strokeTo(sample);}client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,64));const auto painted=client.frame();if(enabled){bool blue=false;for(int y=0;y<64;++y)for(int x=0;x<128;++x)blue|=painted.pixelColor(x,y).blue()>0 && painted.pixelColor(x,y).alpha()>0;QVERIFY(blue);}QVERIFY(painted.pixelColor(48,32).alpha()>=41);for(int y=0;y<64;++y)for(int x=0;x<128;++x)QVERIFY(painted.pixelColor(x,y).alpha()<=43);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),painted);
        const auto vbr=QUrl::fromLocalFile(temp.filePath("opacity.vbr"));QVERIFY(client.exportBrush(vbr,true));QTRY_COMPARE(files.size(),2);QVERIFY(files.last()[0].toBool());QVERIFY(QFile::remove(path));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient restored(nullptr,temp.filePath("fresh/import.ini"));QTRY_VERIFY(restored.ready());QSignalSpy imported(&restored,&PaintCoreClient::brushFileFinished);QVERIFY(restored.importBrushes(vbr));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QVERIFY(qAbs(restored.brushLibrary()->opacity()-.255)<1e-6);QCOMPARE(restored.brushLibrary()->sourceResource(),source);QVERIFY(restored.inspectBrushSource());QTRY_VERIFY(!restored.brushSourceBusy());QCOMPARE(restored.brushSourceInfo().value("converterVersion").toInt(),5);QCOMPARE(restored.brushLibrary()->pressureDynamics(),pressure);QCOMPARE(restored.brushLibrary()->scatterDynamics(),scatter);QCOMPARE(restored.brushLibrary()->colorDynamics(),color);QVERIFY(restored.brushLibrary()->pressureSize() && restored.brushLibrary()->pressureFlow() && restored.brushLibrary()->pressureOpacity());
        restored.setBrushColor(Qt::red);restored.setSecondaryBrushColor(Qt::blue);restored.newTransparentDocument(128,64);QTRY_COMPARE(restored.documentWidth(),128);QTRY_VERIFY(!restored.layerEditBusy());sample.position={32.5,32.5};QVERIFY(restored.beginStroke(sample));for(const double x:{96.5,32.5,96.5,32.5}){sample.position={x,32.5};restored.strokeTo(sample);}restored.endStroke();QTRY_COMPARE(restored.undoDepth(),1);QTRY_VERIFY(!restored.layerEditBusy());restored.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(restored.frameRevision()>=restored.revision() && restored.frame().size()==QSize(128,64));QCOMPARE(restored.frame(),painted);
        QSignalSpy restopped(&restored,&PaintCoreClient::stopped);restored.shutdown();QTRY_COMPARE(restopped.size(),1);PaintCoreClient reopened(nullptr,temp.filePath("fresh/import.ini"));QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->pressureDynamics(),pressure);QCOMPARE(reopened.brushLibrary()->scatterDynamics(),scatter);QCOMPARE(reopened.brushLibrary()->colorDynamics(),color);QVERIFY(reopened.brushLibrary()->pressureOpacity());QSignalSpy again(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(again.size(),1);
        }
    }
    void abrShapeClosedAndActiveImportResetPersistAndExchangeRealPixels() {
        for(const bool enabled:{false,true}) {const bool perTip=true;
        QTemporaryDir temp;const auto path=temp.filePath("opacity.abr");QFile file(path);QVERIFY(file.open(QIODevice::WriteOnly));file.write(abrEntryFixture(false,25.5,true,1,1,perTip,int(enabled)));file.close();
        PaintCoreClient client(nullptr,temp.filePath("opacity.ini"));QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();QSignalSpy files(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushes(QUrl::fromLocalFile(path)));QTRY_COMPARE(files.size(),1);QVERIFY(files.last()[0].toBool());QCOMPARE(library->presets().size(),7);QVERIFY(library->select(library->presets()[5].toMap().value("id").toString()));QVERIFY(qAbs(library->opacity()-.255)<1e-6);
        QVERIFY(client.inspectBrushSource());QTRY_VERIFY(!client.brushSourceBusy());QCOMPARE(client.brushSourceInfo().value("converterVersion").toInt(),6);QVERIFY(qAbs(client.brushSourceInfo().value("baseline").toMap().value("dabOpacity").toDouble()-.255)<1e-6);const auto source=library->sourceResource();
        const auto pressure=library->pressureDynamics();QCOMPARE(library->pressureSize(),enabled);QVERIFY(library->pressureFlow() && library->pressureOpacity());if(enabled)QVERIFY(qAbs(pressure.value("size").toMap().value("minimum").toDouble()-.2)<1e-6);QVERIFY(qAbs(pressure.value("flow").toMap().value("minimum").toDouble()-.4)<1e-6);QVERIFY(qAbs(pressure.value("opacity").toMap().value("minimum").toDouble()-.3)<1e-6);
        const auto scatter=library->scatterDynamics();QCOMPARE(library->scatterEnabled(),true);QCOMPARE(library->scatterBothAxes(),true);QCOMPARE(library->scatterAmount(),.75);QCOMPARE(library->scatterCount(),3);QCOMPARE(library->scatterCountJitter(),.5);
        const auto color=library->colorDynamics();QCOMPARE(library->colorDynamicsEnabled(),true);QCOMPARE(library->colorPerTip(),perTip);QVERIFY(qAbs(library->foregroundBackgroundJitter()-.8)<1e-6);QVERIFY(qAbs(color.value("hue").toDouble()-.2)<1e-6);QVERIFY(qAbs(color.value("saturation").toDouble()-.3)<1e-6);QVERIFY(qAbs(color.value("value").toDouble()-.1)<1e-6);
        const auto shape=library->shapeJitter();QCOMPARE(library->shapeJitterEnabled(),enabled);QVERIFY(qAbs(library->shapeSizeJitter()-.6)<1e-6);QVERIFY(qAbs(library->shapeMinimumSize()-.2)<1e-6);QVERIFY(qAbs(library->shapeAngleJitter()-.4)<1e-6);QVERIFY(qAbs(library->shapeRoundnessJitter()-.5)<1e-6);QVERIFY(qAbs(library->shapeMinimumRoundness()-.25)<1e-6);
        const auto before=library->snapshot();QJsonObject downgrade{{"version",4},{"format","abr"},{"presets",QJsonArray{QJsonObject{{"name","guard"},{"parameters",QJsonObject::fromVariantMap(client.brushSourceInfo().value("baseline").toMap())},{"sourceResource",source},{"tipResource",QJsonValue::Null},{"partial",false}}}}};QVERIFY(!library->importAbr(QJsonDocument(downgrade).toJson()));QCOMPARE(library->snapshot(),before);
        library->setShapeJitterEnabled(!enabled);library->setShapeSizeJitter(.1);library->setShapeAngleJitter(.1);
        library->setPressureSize(false);library->setPressureFlow(false);library->setPressureOpacity(false);
        library->setOpacity(.8);QVERIFY(library->resetPreset());QVERIFY(qAbs(library->opacity()-.255)<1e-6);QCOMPARE(library->sourceResource(),source);QCOMPARE(library->pressureDynamics(),pressure);QCOMPARE(library->scatterDynamics(),scatter);QCOMPARE(library->colorDynamics(),color);QCOMPARE(library->shapeJitter(),shape);QCOMPARE(library->pressureSize(),enabled);QVERIFY(library->pressureFlow() && library->pressureOpacity());
        client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);client.newTransparentDocument(128,64);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());InputSample sample;sample.position={32.5,32.5};sample.pressure=.5;sample.tool=1;sample.capabilities=17;QVERIFY(client.beginStroke(sample));for(const double x:{96.5,32.5,96.5,32.5}){sample.position={x,32.5};client.strokeTo(sample);}client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,64));const auto painted=client.frame();if(enabled){bool blue=false;for(int y=0;y<64;++y)for(int x=0;x<128;++x)blue|=painted.pixelColor(x,y).blue()>0 && painted.pixelColor(x,y).alpha()>0;QVERIFY(blue);}int maxAlpha=0;for(int y=0;y<64;++y)for(int x=0;x<128;++x)maxAlpha=std::max(maxAlpha,painted.pixelColor(x,y).alpha());QVERIFY(maxAlpha>=41);for(int y=0;y<64;++y)for(int x=0;x<128;++x)QVERIFY(painted.pixelColor(x,y).alpha()<=43);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),painted);
        const auto vbr=QUrl::fromLocalFile(temp.filePath("opacity.vbr"));QVERIFY(client.exportBrush(vbr,true));QTRY_COMPARE(files.size(),2);QVERIFY(files.last()[0].toBool());QVERIFY(QFile::remove(path));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient restored(nullptr,temp.filePath("fresh/import.ini"));QTRY_VERIFY(restored.ready());QSignalSpy imported(&restored,&PaintCoreClient::brushFileFinished);QVERIFY(restored.importBrushes(vbr));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QVERIFY(qAbs(restored.brushLibrary()->opacity()-.255)<1e-6);QCOMPARE(restored.brushLibrary()->sourceResource(),source);QVERIFY(restored.inspectBrushSource());QTRY_VERIFY(!restored.brushSourceBusy());QCOMPARE(restored.brushSourceInfo().value("converterVersion").toInt(),6);QCOMPARE(restored.brushLibrary()->pressureDynamics(),pressure);QCOMPARE(restored.brushLibrary()->scatterDynamics(),scatter);QCOMPARE(restored.brushLibrary()->colorDynamics(),color);QCOMPARE(restored.brushLibrary()->shapeJitter(),shape);QCOMPARE(restored.brushLibrary()->pressureSize(),enabled);QVERIFY(restored.brushLibrary()->pressureFlow() && restored.brushLibrary()->pressureOpacity());
        restored.setBrushColor(Qt::red);restored.setSecondaryBrushColor(Qt::blue);restored.newTransparentDocument(128,64);QTRY_COMPARE(restored.documentWidth(),128);QTRY_VERIFY(!restored.layerEditBusy());sample.position={32.5,32.5};QVERIFY(restored.beginStroke(sample));for(const double x:{96.5,32.5,96.5,32.5}){sample.position={x,32.5};restored.strokeTo(sample);}restored.endStroke();QTRY_COMPARE(restored.undoDepth(),1);QTRY_VERIFY(!restored.layerEditBusy());restored.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(restored.frameRevision()>=restored.revision() && restored.frame().size()==QSize(128,64));QCOMPARE(restored.frame(),painted);
        QSignalSpy restopped(&restored,&PaintCoreClient::stopped);restored.shutdown();QTRY_COMPARE(restopped.size(),1);PaintCoreClient reopened(nullptr,temp.filePath("fresh/import.ini"));QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->pressureDynamics(),pressure);QCOMPARE(reopened.brushLibrary()->scatterDynamics(),scatter);QCOMPARE(reopened.brushLibrary()->colorDynamics(),color);QCOMPARE(reopened.brushLibrary()->shapeJitter(),shape);QVERIFY(reopened.brushLibrary()->pressureOpacity());QSignalSpy again(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(again.size(),1);
        }
    }
    void abrDeviceDirectionRotationTiltImportResetPersistAndExchangeRealPixels() {
        for(const int control:{0,3,6,8}) {const bool enabled=control!=0,perTip=true;
        QTemporaryDir temp;const auto path=temp.filePath("opacity.abr");QFile file(path);QVERIFY(file.open(QIODevice::WriteOnly));file.write(abrEntryFixture(false,25.5,true,1,1,perTip,int(enabled),control));file.close();
        PaintCoreClient client(nullptr,temp.filePath("opacity.ini"));QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();QSignalSpy files(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushes(QUrl::fromLocalFile(path)));QTRY_COMPARE(files.size(),1);QVERIFY(files.last()[0].toBool());QCOMPARE(library->presets().size(),7);QVERIFY(library->select(library->presets()[5].toMap().value("id").toString()));QVERIFY(qAbs(library->opacity()-.255)<1e-6);
        QVERIFY(client.inspectBrushSource());QTRY_VERIFY(!client.brushSourceBusy());QCOMPARE(client.brushSourceInfo().value("converterVersion").toInt(),enabled?7:6);QVERIFY(qAbs(client.brushSourceInfo().value("baseline").toMap().value("dabOpacity").toDouble()-.255)<1e-6);const auto source=library->sourceResource();
        const auto device=library->tipDynamics();QCOMPARE(library->angleControl(),QString(!enabled?"fixed":control==3?"tilt":control==6?"direction":"pen-rotation"));QCOMPARE(library->tiltRoundness(),enabled);if(enabled)QVERIFY(qAbs(library->minimumRoundness()-.5)<1e-6);
        const auto pressure=library->pressureDynamics();QCOMPARE(library->pressureSize(),enabled);QVERIFY(library->pressureFlow() && library->pressureOpacity());if(enabled)QVERIFY(qAbs(pressure.value("size").toMap().value("minimum").toDouble()-.2)<1e-6);QVERIFY(qAbs(pressure.value("flow").toMap().value("minimum").toDouble()-.4)<1e-6);QVERIFY(qAbs(pressure.value("opacity").toMap().value("minimum").toDouble()-.3)<1e-6);
        const auto scatter=library->scatterDynamics();QCOMPARE(library->scatterEnabled(),true);QCOMPARE(library->scatterBothAxes(),true);QCOMPARE(library->scatterAmount(),.75);QCOMPARE(library->scatterCount(),3);QCOMPARE(library->scatterCountJitter(),.5);
        const auto color=library->colorDynamics();QCOMPARE(library->colorDynamicsEnabled(),true);QCOMPARE(library->colorPerTip(),perTip);QVERIFY(qAbs(library->foregroundBackgroundJitter()-.8)<1e-6);QVERIFY(qAbs(color.value("hue").toDouble()-.2)<1e-6);QVERIFY(qAbs(color.value("saturation").toDouble()-.3)<1e-6);QVERIFY(qAbs(color.value("value").toDouble()-.1)<1e-6);
        const auto shape=library->shapeJitter();QCOMPARE(library->shapeJitterEnabled(),enabled);QVERIFY(qAbs(library->shapeSizeJitter()-.6)<1e-6);QVERIFY(qAbs(library->shapeMinimumSize()-.2)<1e-6);QVERIFY(qAbs(library->shapeAngleJitter()-.4)<1e-6);QVERIFY(qAbs(library->shapeRoundnessJitter()-.5)<1e-6);QVERIFY(qAbs(library->shapeMinimumRoundness()-.5)<1e-6);
        const auto before=library->snapshot();QJsonObject downgrade{{"version",enabled?5:4},{"format","abr"},{"presets",QJsonArray{QJsonObject{{"name","guard"},{"parameters",QJsonObject::fromVariantMap(client.brushSourceInfo().value("baseline").toMap())},{"sourceResource",source},{"tipResource",QJsonValue::Null},{"partial",false}}}}};QVERIFY(!library->importAbr(QJsonDocument(downgrade).toJson()));QCOMPARE(library->snapshot(),before);
        library->setAngleControl("fixed");library->setTiltRoundness(false);library->setMinimumRoundness(.1);library->setShapeJitterEnabled(!enabled);library->setShapeSizeJitter(.1);library->setShapeAngleJitter(.1);
        library->setPressureSize(false);library->setPressureFlow(false);library->setPressureOpacity(false);
        library->setOpacity(.8);QVERIFY(library->resetPreset());QVERIFY(qAbs(library->opacity()-.255)<1e-6);QCOMPARE(library->sourceResource(),source);QCOMPARE(library->tipDynamics(),device);QCOMPARE(library->pressureDynamics(),pressure);QCOMPARE(library->scatterDynamics(),scatter);QCOMPARE(library->colorDynamics(),color);QCOMPARE(library->shapeJitter(),shape);QCOMPARE(library->pressureSize(),enabled);QVERIFY(library->pressureFlow() && library->pressureOpacity());
        client.setBrushColor(Qt::red);client.setSecondaryBrushColor(Qt::blue);client.newTransparentDocument(128,64);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());InputSample sample;sample.position={32.5,32.5};sample.pressure=.5;sample.tool=1;sample.capabilities=23;sample.tiltX=65;sample.rotation=90;QVERIFY(client.beginStroke(sample));for(const double x:{96.5,32.5,96.5,32.5}){sample.position={x,32.5};client.strokeTo(sample);}client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,64));const auto painted=client.frame();if(enabled){bool blue=false;for(int y=0;y<64;++y)for(int x=0;x<128;++x)blue|=painted.pixelColor(x,y).blue()>0 && painted.pixelColor(x,y).alpha()>0;QVERIFY(blue);}int maxAlpha=0;for(int y=0;y<64;++y)for(int x=0;x<128;++x)maxAlpha=std::max(maxAlpha,painted.pixelColor(x,y).alpha());QVERIFY(maxAlpha>=41);for(int y=0;y<64;++y)for(int x=0;x<128;++x)QVERIFY(painted.pixelColor(x,y).alpha()<=43);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),painted);
        const auto vbr=QUrl::fromLocalFile(temp.filePath("opacity.vbr"));QVERIFY(client.exportBrush(vbr,true));QTRY_COMPARE(files.size(),2);QVERIFY(files.last()[0].toBool());QVERIFY(QFile::remove(path));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient restored(nullptr,temp.filePath("fresh/import.ini"));QTRY_VERIFY(restored.ready());QSignalSpy imported(&restored,&PaintCoreClient::brushFileFinished);QVERIFY(restored.importBrushes(vbr));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QVERIFY(qAbs(restored.brushLibrary()->opacity()-.255)<1e-6);QCOMPARE(restored.brushLibrary()->sourceResource(),source);QVERIFY(restored.inspectBrushSource());QTRY_VERIFY(!restored.brushSourceBusy());QCOMPARE(restored.brushSourceInfo().value("converterVersion").toInt(),enabled?7:6);QCOMPARE(restored.brushLibrary()->tipDynamics(),device);QCOMPARE(restored.brushLibrary()->pressureDynamics(),pressure);QCOMPARE(restored.brushLibrary()->scatterDynamics(),scatter);QCOMPARE(restored.brushLibrary()->colorDynamics(),color);QCOMPARE(restored.brushLibrary()->shapeJitter(),shape);QCOMPARE(restored.brushLibrary()->pressureSize(),enabled);QVERIFY(restored.brushLibrary()->pressureFlow() && restored.brushLibrary()->pressureOpacity());
        restored.setBrushColor(Qt::red);restored.setSecondaryBrushColor(Qt::blue);restored.newTransparentDocument(128,64);QTRY_COMPARE(restored.documentWidth(),128);QTRY_VERIFY(!restored.layerEditBusy());sample.position={32.5,32.5};QVERIFY(restored.beginStroke(sample));for(const double x:{96.5,32.5,96.5,32.5}){sample.position={x,32.5};restored.strokeTo(sample);}restored.endStroke();QTRY_COMPARE(restored.undoDepth(),1);QTRY_VERIFY(!restored.layerEditBusy());restored.requestViewport(0,{0,0,128,64},{128,64});QTRY_VERIFY(restored.frameRevision()>=restored.revision() && restored.frame().size()==QSize(128,64));QCOMPARE(restored.frame(),painted);
        QSignalSpy restopped(&restored,&PaintCoreClient::stopped);restored.shutdown();QTRY_COMPARE(restopped.size(),1);PaintCoreClient reopened(nullptr,temp.filePath("fresh/import.ini"));QTRY_VERIFY(reopened.ready());QCOMPARE(reopened.brushLibrary()->tipDynamics(),device);QCOMPARE(reopened.brushLibrary()->pressureDynamics(),pressure);QCOMPARE(reopened.brushLibrary()->scatterDynamics(),scatter);QCOMPARE(reopened.brushLibrary()->colorDynamics(),color);QCOMPARE(reopened.brushLibrary()->shapeJitter(),shape);QVERIFY(reopened.brushLibrary()->pressureOpacity());QSignalSpy again(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(again.size(),1);
        }
    }
    void abrTextureHandoffImportsMappedModeIntoState11() {
        BrushLibrary library;
        const QString textureResource(64,'a');
        const QJsonObject texture{{"model","periodic-coverage-v1"},{"enabled",true},{"space","tip"},{"mode","colorBurn"},{"scale",1.0},{"angle",0.0},{"depth",1.0},{"brightness",-0.25},{"contrast",0.5},{"minimumDepth",0.2},{"depthJitter",0.5},{"depthPressure",true},{"invert",false},{"offsetX",0.0},{"offsetY",0.0},{"resource",textureResource}};
        const QJsonObject parameters{{"diameter",8.0},{"spacing",0.02},{"dabOpacity",1.0},{"hardness",1.0},{"roundness",1.0},{"angle",0.0},{"flow",1.0},{"pressureSize",false},{"pressureFlow",false}};
        const QJsonObject preset{{"name","Texture"},{"parameters",parameters},{"tipResource",QJsonValue::Null},{"texture",texture},{"sourceResource",QString(64,'b')},{"partial",false}};
        const QJsonObject handoff{{"version",7},{"format","abr"},{"presets",QJsonArray{preset}}};
        QVERIFY(library.importAbr(QJsonDocument(handoff).toJson(QJsonDocument::Compact)));
        QCOMPARE(QJsonDocument::fromJson(library.snapshot()).object().value("version").toInt(),21);
        QCOMPARE(library.texture(),texture.toVariantMap());
        QVariantMap baseline{{"diameter",8.0},{"spacing",0.02},{"dabOpacity",1.0},{"hardness",1.0},{"roundness",1.0},{"angle",0.0},{"flow",1.0},{"pressureSize",false},{"pressureFlow",false}};
        QVariantMap sourceInfo{{"baseline",baseline},{"tipResource",textureResource},{"texture",texture.toVariantMap()}};
        library.setRadius(24);library.setOpacity(.2);
        QVERIFY(library.restoreSourceBaseline(sourceInfo));
        QCOMPARE(library.radius(),4.);QCOMPARE(library.opacity(),1.);QCOMPARE(library.texture(),texture.toVariantMap());
        QVERIFY(library.resetPreset());
        QCOMPARE(library.texture(),texture.toVariantMap());
        auto downgraded=handoff;downgraded["version"]=6;
        const auto before=library.snapshot();
        QVERIFY(!library.importAbr(QJsonDocument(downgraded).toJson(QJsonDocument::Compact)));
        QCOMPARE(library.snapshot(),before);
    }
    void abrDeviceSourceHandoffImportsTiltRotationDirectionAndMatrix() {
        BrushLibrary library;
        const QJsonObject control{{"curve",QJsonArray{QJsonArray{0.0,0.0},QJsonArray{1.0,1.0}}},{"minimum",0.0}};
        const QJsonObject tilt{{"model","tilt-v1"},{"enabled",true},{"size",control},{"flow",control},{"opacity",control}};
        const QJsonObject rotation{{"model","rotation-v1"},{"enabled",true},{"size",control},{"flow",control},{"opacity",control}};
        const QJsonObject direction{{"model","direction-v1"},{"enabled",true},{"size",control},{"flow",control},{"opacity",control}};
        const QJsonObject matrix{{"model","source-matrix-v1"},{"size","tilt"},{"flow","rotation"},{"opacity","direction"}};
        const QJsonObject parameters{{"diameter",8.0},{"spacing",0.02},{"dabOpacity",1.0},{"hardness",1.0},{"roundness",1.0},{"angle",0.0},{"flow",1.0},{"pressureSize",false},{"pressureFlow",false},{"tiltDynamics",tilt},{"rotationDynamics",rotation},{"directionDynamics",direction},{"sourceMatrix",matrix}};
        const QJsonObject preset{{"name","Device sources"},{"parameters",parameters},{"tipResource",QJsonValue::Null},{"sourceResource",QString(64,'c')},{"partial",true}};
        const QJsonObject handoff{{"version",8},{"format","abr"},{"presets",QJsonArray{preset}}};
        QVERIFY(library.importAbr(QJsonDocument(handoff).toJson(QJsonDocument::Compact)));
        QCOMPARE(QJsonDocument::fromJson(library.snapshot()).object().value("version").toInt(),32);
        QCOMPARE(library.tiltDynamics(),tilt.toVariantMap());
        QCOMPARE(library.rotationDynamics(),rotation.toVariantMap());
        QCOMPARE(library.directionDynamics(),direction.toVariantMap());
        QCOMPARE(library.dynamicsSourceMatrix(),matrix.toVariantMap());
        auto downgraded=handoff;downgraded["version"]=7;
        const auto before=library.snapshot();
        QVERIFY(!library.importAbr(QJsonDocument(downgraded).toJson(QJsonDocument::Compact)));
        QCOMPARE(library.snapshot(),before);
    }
    void abrFadeHandoffImportsStampStepsIntoState34() {
        BrushLibrary library;
        const QJsonObject control{{"curve",QJsonArray{QJsonArray{0.0,0.0},QJsonArray{1.0,1.0}}},{"minimum",0.0}};
        const QJsonObject fade{{"model","fade-v1"},{"enabled",true},{"steps",4},{"size",control},{"flow",control},{"opacity",control}};
        const QJsonObject parameters{{"diameter",8.0},{"spacing",0.02},{"dabOpacity",1.0},{"hardness",1.0},{"roundness",1.0},{"angle",0.0},{"flow",1.0},{"pressureSize",false},{"pressureFlow",false},{"fadeDynamics",fade}};
        const QJsonObject preset{{"name","Fade"},{"parameters",parameters},{"tipResource",QJsonValue::Null},{"sourceResource",QString(64,'d')},{"partial",true}};
        const QJsonObject handoff{{"version",9},{"format","abr"},{"presets",QJsonArray{preset}}};
        QVERIFY(library.importAbr(QJsonDocument(handoff).toJson(QJsonDocument::Compact)));
        QCOMPARE(QJsonDocument::fromJson(library.snapshot()).object().value("version").toInt(),34);
        QCOMPARE(library.fadeDynamics(),fade.toVariantMap());
        auto downgraded=handoff;downgraded["version"]=8;
        const auto before=library.snapshot();
        QVERIFY(!library.importAbr(QJsonDocument(downgraded).toJson(QJsonDocument::Compact)));
        QCOMPARE(library.snapshot(),before);
    }
    void abrComputedDualHandoffImportsIntoState12() {
        BrushLibrary library;
        auto dual=library.dualBrush();dual["enabled"]=true;dual["diameter"]=8.0;
        const QJsonObject dualObject=QJsonObject::fromVariantMap(dual);
        const QJsonObject parameters{{"diameter",16.0},{"spacing",0.02},{"dabOpacity",1.0},{"hardness",1.0},{"roundness",1.0},{"angle",0.0},{"flow",1.0},{"pressureSize",false},{"pressureFlow",false}};
        const QJsonObject preset{{"name","Dual"},{"parameters",parameters},{"dualBrush",dualObject},{"tipResource",QJsonValue::Null},{"sourceResource",QString(64,'e')},{"partial",true}};
        const QJsonObject handoff{{"version",10},{"format","abr"},{"presets",QJsonArray{preset}}};
        QVERIFY(library.importAbr(QJsonDocument(handoff).toJson(QJsonDocument::Compact)));
        QVERIFY(QJsonDocument::fromJson(library.snapshot()).object().value("version").toInt()>=12);
        QVERIFY(library.dualBrush().value("enabled").toBool());
        QCOMPARE(library.dualBrush().value("diameter").toDouble(),8.0);
        auto downgraded=handoff;downgraded["version"]=9;
        const auto before=library.snapshot();
        QVERIFY(!library.importAbr(QJsonDocument(downgraded).toJson(QJsonDocument::Compact)));
        QCOMPARE(library.snapshot(),before);
    }
    void licensedRealAbrImportsWholePackAndExchangesPixelsWithOriginalSource() {
        const auto fixture=QFINDTESTDATA("../../core/crates/paint-io/tests/fixtures/abr/myer-settlement-brushes.abr");QVERIFY(!fixture.isEmpty());
        QTemporaryDir temp;const auto path=temp.filePath("real.abr");QVERIFY(QFile::copy(fixture,path));PaintCoreClient client(nullptr,temp.filePath("settings.ini"));QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);QVERIFY(client.importBrushes(QUrl::fromLocalFile(path)));QTRY_COMPARE_WITH_TIMEOUT(finished.size(),1,15000);QVERIFY(finished.last()[0].toBool());QCOMPARE(library->presets().size(),152);QCOMPARE(library->selectedName(),QString("Individual Tree 001"));const auto first=library->selectedId(),source=library->sourceResource(),mask=library->maskResource();QVERIFY(!mask.isEmpty());QVERIFY(library->select(library->presets().last().toMap().value("id").toString()));QVERIFY(library->selectedId()!=first);QVERIFY(library->select(first));
        QVERIFY(client.inspectBrushSource());QTRY_VERIFY(!client.brushSourceBusy());QCOMPARE(client.brushSourceInfo().value("originalHash").toString(),QString("4cb12679582ee908ed8f546d7b45071a2de48fb9af194e649a227f259838f1d6"));
        client.newTransparentDocument(256,256);QTRY_COMPARE(client.documentWidth(),256);QTRY_VERIFY(!client.layerEditBusy());InputSample sample;sample.position={128.5,128.5};QVERIFY(client.beginStroke(sample));sample.position={164.5,128.5};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,256,256},{256,256});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(256,256));const auto pixels=client.frame();bool painted=false;for(int y=0;y<256;++y)for(int x=0;x<256;++x)painted|=pixels.pixelColor(x,y).alpha()>0;QVERIFY(painted);client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());client.redo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,256,256},{256,256});QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),pixels);
        const auto file=QUrl::fromLocalFile(temp.filePath("real.vbr"));QVERIFY(client.exportBrush(file,true));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QVERIFY(QFile::remove(path));QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        PaintCoreClient fresh(nullptr,temp.filePath("fresh/state.ini"));QTRY_VERIFY(fresh.ready());QSignalSpy imported(&fresh,&PaintCoreClient::brushFileFinished);QVERIFY(fresh.importBrushes(file));QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QCOMPARE(fresh.brushLibrary()->sourceResource(),source);QCOMPARE(fresh.brushLibrary()->maskResource(),mask);QVERIFY(fresh.inspectBrushSource());QTRY_VERIFY(!fresh.brushSourceBusy());QCOMPARE(fresh.brushSourceInfo().value("originalHash").toString(),QString("4cb12679582ee908ed8f546d7b45071a2de48fb9af194e649a227f259838f1d6"));fresh.newTransparentDocument(256,256);QTRY_COMPARE(fresh.documentWidth(),256);QTRY_VERIFY(!fresh.layerEditBusy());sample.position={128.5,128.5};QVERIFY(fresh.beginStroke(sample));sample.position={164.5,128.5};fresh.strokeTo(sample);fresh.endStroke();QTRY_COMPARE(fresh.undoDepth(),1);QTRY_VERIFY(!fresh.layerEditBusy());fresh.requestViewport(0,{0,0,256,256},{256,256});QTRY_VERIFY(fresh.frameRevision()>=fresh.revision() && fresh.frame().size()==QSize(256,256));QCOMPARE(fresh.frame(),pixels);QSignalSpy end(&fresh,&PaintCoreClient::stopped);fresh.shutdown();QTRY_COMPARE(end.size(),1);
    }
    void abrUserImportAddsWholePackageWithProvenanceAndSurvivesSourceRemoval() {
        QTemporaryDir temp;const auto settingsPath=temp.filePath("brush.ini");const auto source=temp.filePath("brush.ABR");QFile input(source);QVERIFY(input.open(QIODevice::WriteOnly));input.write(abrEntryFixture());input.close();
        PaintCoreClient client(nullptr,settingsPath);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();library->setCursorCenter(true);QSignalSpy imported(&client,&PaintCoreClient::brushFileFinished);
        QVERIFY(client.importBrushes(QUrl::fromLocalFile(source)));QVERIFY(client.brushFileBusy());QVERIFY(!client.importBrushes(QUrl::fromLocalFile(source)));library->select("round-fine");library->rename("round-fine","Existing renamed");
        QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QCOMPARE(library->presets().size(),7);QVERIFY(library->cursorCenter());QVERIFY(client.brushImportMessage().contains("3"));const auto first=library->selectedId();const auto sourceId=library->sourceResource();const auto tipId=library->maskResource();QCOMPARE(sourceId.size(),64);QCOMPARE(tipId.size(),64);QVERIFY(first!="round-fine");QCOMPARE(library->selectedName(),QString("Same"));QCOMPARE(library->radius(),4.);QCOMPARE(library->presets()[4].toMap().value("radius").toDouble(),4.);QVERIFY(!library->pressureSize() && !library->pressureFlow());
        QVERIFY(client.inspectBrushSource());QTRY_VERIFY(!client.brushSourceBusy());QCOMPARE(client.brushSourceInfo().value("originalName").toString(),QString("Same"));QVERIFY(!client.brushSourceInfo().value("report").toList().isEmpty());
        library->requestPreview(first);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto preview=library->selectedPreview();
        QVERIFY(client.importBrushes(QUrl::fromLocalFile(source)));QTRY_COMPARE(imported.size(),2);QVERIFY(imported.last()[0].toBool());QCOMPARE(library->presets().size(),10);QVERIFY(library->selectedId()!=first);QCOMPARE(library->sourceResource(),sourceId);QCOMPARE(library->maskResource(),tipId);const auto selected=library->selectedId();
        const auto before=library->snapshot();const auto bad=temp.filePath("bad.abr");QFile invalid(bad);QVERIFY(invalid.open(QIODevice::WriteOnly));invalid.write(abrEntryFixture(true));invalid.close();QVERIFY(client.importBrushes(QUrl::fromLocalFile(bad)));QTRY_COMPARE(imported.size(),3);QVERIFY(!imported.last()[0].toBool());QCOMPARE(library->snapshot(),before);
        library->setRadius(4);
        client.newTransparentDocument(24,24);QTRY_COMPARE(client.documentWidth(),24);QTRY_VERIFY(client.ready());InputSample sample;sample.position={12.5,12.5};QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);client.requestViewport(0,{0,0,24,24},{24,24});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(24,24));QCOMPARE(client.frame().pixelColor(12,12).alpha(),0);QVERIFY(client.frame().pixelColor(15,12).alpha()>200);
        QVERIFY(library->select("round-fine"));QCOMPARE(library->selectedName(),QString("Existing renamed"));QVERIFY(library->select(selected));QVERIFY(QFile::remove(source));QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        PaintCoreClient recovered(nullptr,settingsPath);QTRY_VERIFY(recovered.ready());QTRY_COMPARE(recovered.brushLibrary()->selectedId(),selected);QCOMPARE(recovered.brushLibrary()->presets().size(),10);QCOMPARE(recovered.brushLibrary()->sourceResource(),sourceId);QCOMPARE(recovered.brushLibrary()->maskResource(),tipId);QVERIFY(recovered.brushLibrary()->cursorCenter());recovered.brushLibrary()->requestPreview(selected);QTRY_VERIFY_WITH_TIMEOUT(!recovered.brushLibrary()->selectedPreview().isEmpty(),10000);QCOMPARE(recovered.brushLibrary()->selectedPreview(),preview);QVERIFY(recovered.inspectBrushSource());QTRY_VERIFY(!recovered.brushSourceBusy());QCOMPARE(recovered.brushSourceInfo().value("originalName").toString(),QString("Same"));QSignalSpy recoveredStopped(&recovered,&PaintCoreClient::stopped);recovered.shutdown();QTRY_COMPARE(recoveredStopped.size(),1);
    }
    void brushSourceV4PreservesProvenanceAcrossEditsCopiesAndVbrImport() {
        BrushLibrary model;
        const auto source=QString(64,QChar('a'));const auto image=QString(64,QChar('b'));
        auto state=QJsonDocument::fromJson(model.snapshot()).object();auto rows=state.value("presets").toArray();auto row=rows[0].toObject();row["sourceResource"]=source;rows[0]=row;row=rows[3].toObject();row["maskResource"]=image;rows[3]=row;state["version"]=4;state["presets"]=rows;
        QVERIFY(model.restore(QJsonDocument(state).toJson()));QCOMPARE(model.sourceResource(),source);QCOMPARE(QJsonDocument::fromJson(model.snapshot()).object().value("version").toInt(),4);
        QVERIFY(model.rename("round-pressure","renamed"));model.setHardness(.3);const auto copy=model.saveCopy("copy");QVERIFY(!copy.isEmpty());QCOMPARE(model.sourceResource(),source);QVERIFY(model.exportPackage().isEmpty());
        QVERIFY(model.setMaskResource(copy,image));QCOMPARE(model.sourceResource(),source);model.useProceduralTip();QCOMPARE(model.sourceResource(),source);
        BrushLibrary restored;QVERIFY(restored.restore(model.snapshot()));QCOMPARE(restored.sourceResource(),source);
        QVERIFY(restored.applyNativeCatalog(model.catalogSnapshot()));QCOMPARE(restored.sourceResource(),source);
        BrushLibrary imported;auto package=QJsonDocument::fromJson(imported.exportPackage()).object();auto brushes=package.value("brushes").toArray();auto brush=brushes[0].toObject();brush["tipResource"]=image;brushes[0]=brush;package["brushes"]=brushes;
        QVERIFY(restored.importPackage(QJsonDocument(package).toJson()));QCOMPARE(QJsonDocument::fromJson(restored.snapshot()).object().value("version").toInt(),4);QVERIFY(restored.sourceResource().isEmpty());QVERIFY(restored.select(copy));QCOMPARE(restored.sourceResource(),source);
        auto invalid=QJsonDocument::fromJson(model.snapshot()).object();invalid["version"]=3;const auto before=restored.snapshot();QVERIFY(!restored.restore(QJsonDocument(invalid).toJson()));QCOMPARE(restored.snapshot(),before);
        rows=state.value("presets").toArray();row=rows[0].toObject();row["sourceResource"]="../bad";rows[0]=row;state["presets"]=rows;QVERIFY(!restored.restore(QJsonDocument(state).toJson()));QCOMPARE(restored.snapshot(),before);
    }
    void brushSourceQueriesUseNativeVerificationAndDiscardStaleResponses() {
        QTemporaryDir temp;const auto settingsPath=temp.filePath("brush.ini");const auto directory=temp.filePath("brush-resources/sources");QVERIFY(QDir().mkpath(directory));
        // Opaque generated source header; this tests provenance IO, not complete ABR parsing.
        const auto original=QByteArray::fromHex("000600022a");const auto originalHash=QString::fromLatin1(QCryptographicHash::hash(original,QCryptographicHash::Sha256).toHex());
        BrushLibrary model;const auto baseline=QJsonDocument::fromJson(model.exportPackage()).object().value("brushes").toArray()[0].toObject().value("parameters");
        const QJsonObject info{{"version",1},{"format","abr"},{"originalHash",originalHash},{"fileVersion",6},{"fileSubversion",2},{"presetIndex",0},{"originalName","<b>Original</b>"},{"converter","drawverse.abr.basic"},{"converterVersion",1},{"baseline",baseline},{"tipResource",QJsonValue::Null},{"report",QJsonArray{QJsonObject{{"path","texture.Depth"},{"status","notExecuted"},{"reason","Texture not executed"}}}}};
        const auto record=QJsonDocument(info).toJson(QJsonDocument::Compact);const auto identity=QString::fromLatin1(QCryptographicHash::hash(record,QCryptographicHash::Sha256).toHex());
        QFile abr(directory+"/"+originalHash+".abr");QVERIFY(abr.open(QIODevice::WriteOnly));QCOMPARE(abr.write(original),qint64(original.size()));abr.close();
        QFile json(directory+"/"+identity+".json");QVERIFY(json.open(QIODevice::WriteOnly));QCOMPARE(json.write(record),qint64(record.size()));json.close();
        auto state=QJsonDocument::fromJson(model.snapshot()).object();auto rows=state.value("presets").toArray();auto row=rows[0].toObject();row["sourceResource"]=identity;rows[0]=row;state["presets"]=rows;state["version"]=4;const auto saved=QJsonDocument(state).toJson(QJsonDocument::Compact);
        {QSettings settings(settingsPath,QSettings::IniFormat);settings.setValue("brushes/state",saved);settings.sync();}
        PaintCoreClient client(nullptr,settingsPath);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();QTRY_COMPARE(library->sourceResource(),identity);
        WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &error:errors)warnings.append(error.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush-settings"))workspace.setActive(group.value("id").toString(),"brush-settings");}
        QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:8")));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
        QTRY_VERIFY(!client.brushSourceBusy() && !client.brushSourceInfo().isEmpty());QCOMPARE(client.brushSourceInfo().value("originalName").toString(),QString("<b>Original</b>"));QCOMPARE(client.brushSourceInfo().value("report").toList().size(),1);
        auto *label=window->findChild<QObject*>("brushSourceName");QVERIFY(label);QCOMPARE(label->property("text").toString(),QString("<b>Original</b>"));QCOMPARE(label->property("textFormat").toInt(),int(Qt::PlainText));
        library->setRadius(32);library->rename(library->selectedId(),"Edited");QCOMPARE(client.brushSourceInfo().value("baseline").toMap().value("diameter").toDouble(),24.);QVERIFY(!client.brushSourceBusy());
        QVERIFY(!client.exportBrush(QUrl::fromLocalFile(temp.filePath("source.vbr"))));QVERIFY(!client.brushFileBusy());QVERIFY(!QFileInfo::exists(temp.filePath("source.vbr")));
        QVERIFY(client.inspectBrushSource());QVERIFY(library->select("round-fine"));QVERIFY(client.brushSourceInfo().isEmpty());QTest::qWait(100);QVERIFY(client.brushSourceInfo().isEmpty());QVERIFY(!client.brushSourceBusy());
        QVERIFY(library->select("round-pressure"));QTRY_VERIFY(!client.brushSourceInfo().isEmpty());const auto copy=library->saveCopy("Source copy");QVERIFY(!copy.isEmpty());QCOMPARE(library->sourceResource(),identity);
        library->requestPreview(copy);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);
        QVERIFY(abr.open(QIODevice::WriteOnly|QIODevice::Truncate));abr.write("corrupt");abr.close();QVERIFY(client.inspectBrushSource());QTRY_VERIFY(!client.brushSourceBusy());QVERIFY(!client.brushSourceMessage().isEmpty());QVERIFY(client.brushSourceInfo().isEmpty());
        auto *selectedName=window->findChild<QObject*>("selectedBrushName");QVERIFY(selectedName);QCOMPARE(selectedName->property("textFormat").toInt(),int(Qt::PlainText));
        for(const auto &value:workspace.leftGroups()+workspace.rightGroups()){const auto group=value.toMap();if(group.value("panels").toStringList().contains("brush"))workspace.setActive(group.value("id").toString(),"brush");}
        QObject *dialog=nullptr;QTRY_VERIFY((dialog=window->findChild<QObject*>("brushImportDialog")));QVERIFY(dialog->property("nameFilters").toStringList().contains("Photoshop 笔刷 (*.abr)"));QVERIFY(findVisualItem(window->contentItem(),"importBrushPackage"));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        QByteArray persisted;{QSettings settings(settingsPath,QSettings::IniFormat);persisted=settings.value("brushes/state").toByteArray();}QVERIFY(persisted.contains("sourceResource"));
        PaintCoreClient cold(nullptr,settingsPath);QTRY_VERIFY(cold.ready());QTRY_VERIFY_WITH_TIMEOUT(!cold.lastError().isEmpty(),10000);{QSettings settings(settingsPath,QSettings::IniFormat);QCOMPARE(settings.value("brushes/state").toByteArray(),persisted);}QSignalSpy coldStopped(&cold,&PaintCoreClient::stopped);cold.shutdown();QTRY_COMPARE(coldStopped.size(),1);
    }
    void imageBrushImportPreviewPersistenceAndCapturedTarget() {
        QTemporaryDir temp;const auto settingsPath=temp.filePath("brush.ini");
        QImage tip(16,16,QImage::Format_RGBA8888);tip.fill(Qt::white);
        for(int y=6;y<10;++y)for(int x=6;x<10;++x)tip.setPixelColor(x,y,Qt::transparent);
        const auto source=temp.filePath("tip.png");QVERIFY(tip.save(source));
        PaintCoreClient client(nullptr,settingsPath);QTRY_VERIFY(client.ready());auto *library=client.brushLibrary();
        QSignalSpy imported(&client,&PaintCoreClient::brushFileFinished);
        QVERIFY(client.importBrushTip(QUrl::fromLocalFile(source),1));library->select("round-fine");
        QTRY_COMPARE(imported.size(),1);QVERIFY(imported.last()[0].toBool());QVERIFY(library->maskResource().isEmpty());
        library->select("round-pressure");const auto identity=library->maskResource();QCOMPARE(identity.size(),64);
        QVERIFY(QFileInfo::exists(temp.filePath("brush-resources/"+identity+".png")));
        const auto copy=library->saveCopy("图片副本");QVERIFY(!copy.isEmpty());QCOMPARE(library->maskResource(),identity);
        library->setRadius(8);library->setPressureSize(false);library->setPressureFlow(false);
        library->requestPreview(copy);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto preview=library->selectedPreview();
        QVERIFY(client.exportBrush(QUrl::fromLocalFile(temp.filePath("image.vbr"))));QTRY_COMPARE(imported.size(),2);QVERIFY(imported.last()[0].toBool());QVERIFY(QFileInfo::exists(temp.filePath("image.vbr")));
        client.newTransparentDocument(40,40);QTRY_COMPARE(client.documentWidth(),40);QTRY_VERIFY(client.ready());
        InputSample sample;sample.position={20.5,20.5};sample.pressure=1;sample.tool=1;QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);
        client.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(40,40));
        QCOMPARE(client.frame().pixelColor(20,20).alpha(),0);QVERIFY(client.frame().pixelColor(25,20).alpha()>200);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);QVERIFY(QFile::remove(source));
        PaintCoreClient recovered(nullptr,settingsPath);QTRY_VERIFY(recovered.ready());auto *restored=recovered.brushLibrary();QCOMPARE(restored->maskResource(),identity);QCOMPARE(restored->selectedId(),copy);
        restored->requestPreview(copy);QTRY_VERIFY_WITH_TIMEOUT(!restored->selectedPreview().isEmpty(),10000);QCOMPARE(restored->selectedPreview(),preview);
        restored->useProceduralTip();QVERIFY(restored->maskResource().isEmpty());QVERIFY(!restored->exportPackage().isEmpty());restored->requestPreview(copy);QTRY_VERIFY_WITH_TIMEOUT(!restored->selectedPreview().isEmpty(),10000);QVERIFY(restored->selectedPreview()!=preview);
        QSignalSpy recoveredStopped(&recovered,&PaintCoreClient::stopped);recovered.shutdown();QTRY_COMPARE(recoveredStopped.size(),1);
        QByteArray persisted;{QSettings settings(settingsPath,QSettings::IniFormat);persisted=settings.value("brushes/state").toByteArray();}
        QVERIFY(QFile::remove(temp.filePath("brush-resources/"+identity+".png")));
        PaintCoreClient missing(nullptr,settingsPath);QTRY_VERIFY(missing.ready());QTRY_VERIFY_WITH_TIMEOUT(!missing.lastError().isEmpty(),10000);
        {QSettings settings(settingsPath,QSettings::IniFormat);QCOMPARE(settings.value("brushes/state").toByteArray(),persisted);}
        QSignalSpy missingStopped(&missing,&PaintCoreClient::stopped);missing.shutdown();QTRY_COMPARE(missingStopped.size(),1);
    }
    void vbrImagePackageImportsIntoFreshStoreAndExecutesAfterOriginalRemoval() {
        QTemporaryDir temp;const auto archive=QUrl::fromLocalFile(temp.filePath("image.vbr"));QString identity;
        {
            PaintCoreClient source(nullptr,temp.filePath("source/state.ini"));QTRY_VERIFY(source.ready());QImage image(16,16,QImage::Format_RGBA8888);image.fill(Qt::white);for(int y=6;y<10;++y)for(int x=6;x<10;++x)image.setPixelColor(x,y,Qt::transparent);
            const auto imagePath=temp.filePath("source.png");QVERIFY(image.save(imagePath));QVERIFY(source.importBrushTip(QUrl::fromLocalFile(imagePath),1));QTRY_VERIFY(!source.brushFileBusy());identity=source.brushLibrary()->maskResource();QVERIFY(!identity.isEmpty());source.brushLibrary()->setRadius(8);source.brushLibrary()->setPressureSize(false);source.brushLibrary()->setPressureFlow(false);
            QSignalSpy finished(&source,&PaintCoreClient::brushFileFinished);QVERIFY(source.exportBrush(archive));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QSignalSpy stopped(&source,&PaintCoreClient::stopped);source.shutdown();QTRY_COMPARE(stopped.size(),1);QVERIFY(QFile::remove(imagePath));QVERIFY(QFile::remove(temp.filePath("source/brush-resources/"+identity+".png")));
        }
        PaintCoreClient target(nullptr,temp.filePath("target/state.ini"));QTRY_VERIFY(target.ready());QSignalSpy finished(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QCOMPARE(target.brushLibrary()->maskResource(),identity);QVERIFY(QFileInfo::exists(temp.filePath("target/brush-resources/"+identity+".png")));const auto first=target.brushLibrary()->selectedId();
        QVERIFY(target.importBrushes(archive));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QVERIFY(target.brushLibrary()->selectedId()!=first);QCOMPARE(target.brushLibrary()->maskResource(),identity);target.brushLibrary()->setRadius(8);
        target.newTransparentDocument(40,40);QTRY_COMPARE(target.documentWidth(),40);QTRY_VERIFY(target.ready());InputSample sample;sample.position={20.5,20.5};sample.pressure=1;sample.tool=1;QVERIFY(target.beginStroke(sample));target.endStroke();QTRY_COMPARE(target.undoDepth(),1);target.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(target.frameRevision()>=target.revision() && target.frame().size()==QSize(40,40));QCOMPARE(target.frame().pixelColor(20,20).alpha(),0);QVERIFY(target.frame().pixelColor(25,20).alpha()>200);
        target.brushLibrary()->requestPreview(target.brushLibrary()->selectedId());QTRY_VERIFY_WITH_TIMEOUT(!target.brushLibrary()->selectedPreview().isEmpty(),10000);QSignalSpy stopped(&target,&PaintCoreClient::stopped);target.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void scatterImageExchangeKeepsActualPixelsAfterSourceRemoval() {
        QTemporaryDir temp;const auto archive=QUrl::fromLocalFile(temp.filePath("scatter-image.vbr"));QImage expected;QString identity;
        const auto renderStroke=[&](PaintCoreClient &client){InputSample sample;sample.position={16,32};sample.pressure=1;sample.tool=1;if(!client.beginStroke(sample))return false;sample.position={48,32};client.strokeTo(sample);client.endStroke();return true;};
        {
            PaintCoreClient source(nullptr,temp.filePath("source/state.ini"));QTRY_VERIFY(source.ready());QImage image(16,16,QImage::Format_RGBA8888);image.fill(Qt::white);for(int y=5;y<11;++y)for(int x=5;x<11;++x)image.setPixelColor(x,y,Qt::transparent);const auto input=temp.filePath("tip.png");QVERIFY(image.save(input));QVERIFY(source.importBrushTip(QUrl::fromLocalFile(input),1));QTRY_VERIFY(!source.brushFileBusy());auto *library=source.brushLibrary();identity=library->maskResource();QVERIFY(!identity.isEmpty());library->setRadius(4);library->setPressureSize(false);library->setPressureFlow(false);library->setScatterAmount(1);library->setScatterCount(3);library->setScatterBothAxes(true);library->setScatterEnabled(true);auto cfg=library->scatterDynamics();cfg["seed"]="ffffffffffffffff";QVERIFY(library->setScatterDynamics(cfg));library->setOpacity(.3);library->setShapeSizeJitter(.4);library->setShapeMinimumSize(.5);library->setShapeAngleJitter(.7);library->setShapeRoundnessJitter(.3);library->setShapeMinimumRoundness(.5);library->setShapeJitterEnabled(true);auto shape=library->shapeJitter();shape["seed"]="ffffffffffffffff";QVERIFY(library->setShapeJitter(shape));library->setForegroundBackgroundJitter(1);library->setColorDynamicsEnabled(true);auto color=library->colorDynamics();color["seed"]="ffffffffffffffff";QVERIFY(library->setColorDynamics(color));source.setBrushColor(Qt::red);source.setSecondaryBrushColor(Qt::blue);
            QVERIFY(source.brushLibrary()->setTextureResource(source.brushLibrary()->selectedId(),identity));auto texture=source.brushLibrary()->texture();texture["enabled"]=true;texture["space"]="tip";texture["scale"]=2.;texture["angle"]=23.;texture["depth"]=.7;texture["invert"]=true;texture["offsetX"]=.2;texture["offsetY"]=-.4;QVERIFY(source.brushLibrary()->setTexture(texture));source.newTransparentDocument(64,64);QTRY_COMPARE(source.documentWidth(),64);QTRY_VERIFY(source.ready());QVERIFY(renderStroke(source));QTRY_COMPARE(source.undoDepth(),1);QTRY_VERIFY(!source.layerEditBusy());source.requestViewport(0,{0,0,64,64},{64,64});QTRY_VERIFY(source.frameRevision()>=source.revision() && source.frame().size()==QSize(64,64));expected=source.frame();
            QSignalSpy finished(&source,&PaintCoreClient::brushFileFinished);QVERIFY(source.exportBrush(archive));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QSignalSpy stopped(&source,&PaintCoreClient::stopped);source.shutdown();QTRY_COMPARE(stopped.size(),1);QVERIFY(QFile::remove(input));
        }
        QVERIFY(QDir(temp.filePath("source")).removeRecursively());PaintCoreClient target(nullptr,temp.filePath("target/state.ini"));QTRY_VERIFY(target.ready());QSignalSpy finished(&target,&PaintCoreClient::brushFileFinished);QVERIFY(target.importBrushes(archive));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QCOMPARE(target.brushLibrary()->maskResource(),identity);QCOMPARE(target.brushLibrary()->texture().value("resource").toString(),identity);QVERIFY(target.brushLibrary()->texture().value("enabled").toBool());QCOMPARE(target.brushLibrary()->texture().value("space").toString(),QString("tip"));QCOMPARE(target.brushLibrary()->texture().value("offsetY").toDouble(),-.4);QCOMPARE(target.brushLibrary()->scatterDynamics().value("seed").toString(),QString("ffffffffffffffff"));QVERIFY(target.brushLibrary()->scatterEnabled());QVERIFY(target.brushLibrary()->shapeJitterEnabled());QCOMPARE(target.brushLibrary()->shapeJitter().value("seed").toString(),QString("ffffffffffffffff"));QVERIFY(target.brushLibrary()->colorDynamicsEnabled());target.setBrushColor(Qt::red);target.setSecondaryBrushColor(Qt::blue);target.newTransparentDocument(64,64);QTRY_COMPARE(target.documentWidth(),64);QTRY_VERIFY(target.ready());QVERIFY(renderStroke(target));QTRY_COMPARE(target.undoDepth(),1);QTRY_VERIFY(!target.layerEditBusy());target.requestViewport(0,{0,0,64,64},{64,64});QTRY_VERIFY(target.frameRevision()>=target.revision() && target.frame().size()==QSize(64,64));QCOMPARE(target.frame(),expected);QSignalSpy stopped(&target,&PaintCoreClient::stopped);target.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void imageBrushCursorPreservesHolesTransformsAndScreenWidth() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("state.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();
        QImage tip(32,16,QImage::Format_RGBA8888);tip.fill(Qt::transparent);
        for(int y=1;y<15;++y)for(int x=1;x<23;++x)tip.setPixelColor(x,y,Qt::white);
        for(int y=5;y<11;++y)for(int x=7;x<17;++x)tip.setPixelColor(x,y,Qt::transparent);
        for(int y=4;y<12;++y)for(int x=27;x<31;++x)tip.setPixelColor(x,y,Qt::white);
        const auto source=temp.filePath("tip.png");QVERIFY(tip.save(source));QVERIFY(client.importBrushTip(QUrl::fromLocalFile(source),1));QTRY_VERIFY(!client.brushFileBusy());QTRY_VERIFY(client.brushOutlineReady());QVERIFY(!client.brushOutlinePath().isEmpty());
        const auto path=client.brushOutlinePath();auto *library=client.brushLibrary();library->setRadius(64);library->setRoundness(.5);library->setAngle(30);library->setCursorWidth(2);
        auto *outline=findVisualItem(window->contentItem(),"brushCursorOutline");QVERIFY(outline);QTRY_VERIFY(outline->property("imageOutline").toBool());QCOMPARE(outline->rotation(),30.);QCOMPARE(outline->height()/outline->width(),.5);
        auto *outer=window->findChild<QObject*>("imageBrushOuterPath");auto *inner=window->findChild<QObject*>("imageBrushInnerPath");QVERIFY(outer && inner);QCOMPARE(outer->property("strokeWidth").toDouble(),4.);QCOMPARE(inner->property("strokeWidth").toDouble(),2.);
        const auto revision=client.revision();const auto undo=client.undoDepth();QSignalSpy changes(&client,&PaintCoreClient::brushOutlineChanged);
        QHoverEvent hover(QEvent::HoverMove,{canvas->width()/2,canvas->height()/2},canvas->mapToGlobal({canvas->width()/2,canvas->height()/2}),{0,0});QCoreApplication::sendEvent(canvas,&hover);const bool initial=canvas->brushCursorVisible();if(!initial)QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());
        QTest::mouseMove(window,canvas->mapToScene({canvas->width()/2,canvas->height()/2}).toPoint());QTRY_VERIFY(outline->isVisible());QTest::qWait(120);QVERIFY(outline->isVisible());
        const auto center=canvas->mapToScene(canvas->cursorPosition());const auto dpr=window->devicePixelRatio();
        const auto captured=window->grabWindow().copy(QRect((center*dpr).toPoint()-QPoint(qRound(120*dpr),qRound(120*dpr)),QSize(qRound(240*dpr),qRound(240*dpr))));
        int dark=0;for(int y=0;y<captured.height();++y)for(int x=0;x<captured.width();++x)if(captured.pixelColor(x,y).lightness()<80)++dark;
        QVERIFY2(dark>20,"image outline must be present in actual rendered pixels");
        const auto screenshot=qEnvironmentVariable("DRAWVERSE_BRUSH_OUTLINE");if(!screenshot.isEmpty())QVERIFY(captured.save(screenshot));
        library->setRadius(100);QCOMPARE(client.brushOutlinePath(),path);QCOMPARE(outer->property("strokeWidth").toDouble(),4.);QCOMPARE(changes.size(),0);
        library->setCursorSimple(true);QVERIFY(!outline->property("imageOutline").toBool());QCOMPARE(outline->height(),outline->width());library->setCursorSimple(false);QVERIFY(outline->property("imageOutline").toBool());
        QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),undo);InputSample held;held.position={20,20};held.pressure=1;held.tool=1;QVERIFY(client.beginStroke(held));QVERIFY(client.cursorHasImage());const auto capturedPath=client.cursorOutlinePath();library->useProceduralTip();QVERIFY(client.cursorHasImage());QCOMPARE(client.cursorOutlinePath(),capturedPath);QVERIFY(client.cursorOutlineReady());client.cancelStroke();QVERIFY(!client.cursorHasImage());if(!initial)QTest::keyClick(window,Qt::Key_CapsLock);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void vbrPackageImportExportIsAtomicAndPreservesParameters() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();library->setHardness(.25);library->setRoundness(.3);library->setAngle(-35);library->setFlow(.6);library->setPressureSize(false);library->setSpacing(.02);
        const auto revision=client.revision();const auto undo=client.undoDepth();const auto path=QUrl::fromLocalFile(temp.filePath("brush.vbr"));QSignalSpy finished(&client,&PaintCoreClient::brushFileFinished);
        QVERIFY(client.exportBrush(path));QVERIFY(client.brushFileBusy());QVERIFY(!client.exportBrush(path));QTRY_COMPARE(finished.size(),1);QVERIFY(finished.last()[0].toBool());QVERIFY(!client.brushFileBusy());QVERIFY(QFileInfo::exists(path.toLocalFile()));
        library->setHardness(1);QVERIFY(client.importBrushes(path));QTRY_COMPARE(finished.size(),2);QVERIFY(finished.last()[0].toBool());QCOMPARE(library->presets().size(),5);QCOMPARE(library->hardness(),.25);QCOMPARE(library->roundness(),.3);QCOMPARE(library->angle(),-35.);QCOMPARE(library->flow(),.6);QCOMPARE(library->spacing(),.02);QVERIFY(!library->pressureSize());
        const auto imported=library->selectedId();QVERIFY(client.importBrushes(path));QTRY_COMPARE(finished.size(),3);QVERIFY(finished.last()[0].toBool());QCOMPARE(library->presets().size(),6);QVERIFY(library->selectedId()!=imported);
        const auto originalState=library->snapshot();QFile bad(temp.filePath("bad.vbr"));QVERIFY(bad.open(QIODevice::WriteOnly));bad.write("invalid archive");bad.close();QVERIFY(client.importBrushes(QUrl::fromLocalFile(bad.fileName())));QTRY_COMPARE(finished.size(),4);QVERIFY(!finished.last()[0].toBool());QCOMPARE(library->snapshot(),originalState);QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),undo);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void nativeBrushRevisionStartsImmediatelyAndBadMigrationKeepsFile() {
        QTemporaryDir temp;const auto path=temp.filePath("state.ini");
        PaintCoreClient client(nullptr,path);QTRY_VERIFY(client.ready());client.newTransparentDocument(40,40);QTRY_COMPARE(client.documentWidth(),40);QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();library->setRadius(10);library->setPressureSize(false);library->setPressureFlow(false);library->setFlow(1);
        library->setRoundness(.2);library->setAngle(0);
        InputSample sample;sample.position={20.5,20.5};sample.pressure=.1f;sample.tool=1;
        QVERIFY(client.beginStroke(sample));library->setRoundness(1);library->setFlow(0);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        client.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(40,40));QVERIFY(client.frame().pixelColor(27,20).alpha()>200);QCOMPARE(client.frame().pixelColor(20,27).alpha(),0);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);library->setRoundness(.2);library->setAngle(90);library->setFlow(1);QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.requestViewport(0,{0,0,40,40},{40,40});QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.frame().pixelColor(20,27).alpha()>200);QCOMPARE(client.frame().pixelColor(27,20).alpha(),0);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
        BrushLibrary old;const auto copy=old.saveCopy("Old custom");QVERIFY(!copy.isEmpty());auto invalid=QJsonDocument::fromJson(old.snapshot()).object();auto rows=invalid.value("presets").toArray();auto row=rows.last().toObject();row["id"]="not-a-uuid";rows[rows.size()-1]=row;invalid["presets"]=rows;invalid["selected"]="not-a-uuid";const auto bad=QJsonDocument(invalid).toJson(QJsonDocument::Compact);const auto badPath=temp.filePath("bad.ini");{QSettings settings(badPath,QSettings::IniFormat);settings.setValue("brushes/state",bad);settings.sync();}
        PaintCoreClient recovered(nullptr,badPath);QTRY_VERIFY(recovered.ready());QTRY_VERIFY_WITH_TIMEOUT(recovered.lastError().contains("UUID"),10000);QTRY_COMPARE(recovered.brushLibrary()->presets().size(),4);{QSettings settings(badPath,QSettings::IniFormat);QCOMPARE(settings.value("brushes/state").toByteArray(),bad);}
        QVERIFY(recovered.beginStroke(sample));recovered.endStroke();QTRY_COMPARE(recovered.undoDepth(),1);QSignalSpy recoveredStopped(&recovered,&PaintCoreClient::stopped);recovered.shutdown();QTRY_COMPARE(recoveredStopped.size(),1);
    }
    void cursorImagePreferencesAreStrictGlobalAndOutsideBrushDefinitions() {
        BrushLibrary library;QVERIFY(!library.cursorImage());QCOMPARE(library.cursorImageOpacity(),25);
        const auto catalog=library.catalogSnapshot();const auto package=library.exportPackage();
        QSignalSpy settings(&library,&BrushLibrary::settingsChanged),preview(&library,&BrushLibrary::previewRequested);
        library.setCursorImage(true);library.setCursorImageOpacity(73);
        QCOMPARE(settings.size(),0);QCOMPARE(preview.size(),0);QCOMPARE(library.catalogSnapshot(),catalog);
        QCOMPARE(QJsonDocument::fromJson(library.exportPackage()).object().value("brushes"),QJsonDocument::fromJson(package).object().value("brushes"));
        BrushLibrary restored;QVERIFY(restored.restore(library.snapshot()));QVERIFY(restored.cursorImage());QCOMPARE(restored.cursorImageOpacity(),73);
        const auto good=restored.snapshot();
        for(const auto &bad:QJsonArray{0,101,1.5,"25",QJsonValue::Null}) {
            auto state=QJsonDocument::fromJson(good).object();auto cursor=state.value("cursor").toObject();cursor["imageOpacity"]=bad;state["cursor"]=cursor;
            QVERIFY(!restored.restore(QJsonDocument(state).toJson()));QCOMPARE(restored.snapshot(),good);
        }
        for(const auto &bad:QJsonArray{1,"true",QJsonValue::Null}) {
            auto state=QJsonDocument::fromJson(good).object();auto cursor=state.value("cursor").toObject();cursor["image"]=bad;state["cursor"]=cursor;
            QVERIFY(!restored.restore(QJsonDocument(state).toJson()));QCOMPARE(restored.snapshot(),good);
        }
        auto old=QJsonDocument::fromJson(good).object();auto cursor=old.value("cursor").toObject();cursor.remove("image");cursor.remove("imageOpacity");old["cursor"]=cursor;
        QVERIFY(restored.restore(QJsonDocument(old).toJson()));QVERIFY(!restored.cursorImage());QCOMPARE(restored.cursorImageOpacity(),25);
        QVERIFY(!library.saveCopy("Image cursor independent").isEmpty());QVERIFY(library.resetPreset());QVERIFY(library.cursorImage());QCOMPARE(library.cursorImageOpacity(),73);
    }
    void cursorImageQmlWhiteAlphaIndependentOutlineCapturedStrokeAndRestart() {
        QTemporaryDir temp;const auto preferences=temp.filePath("state.ini");
        {
            PaintCoreClient client(nullptr,preferences);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
            QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
            engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));
            QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QVERIFY(QTest::qWaitForWindowExposed(window));QTRY_VERIFY(client.ready());
            auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);
            client.newDocument(256,256);QTRY_VERIFY(client.ready() && client.documentWidth()==256);
            const auto background=client.activeLayer();client.addLayer("transparent cursor background");QTRY_VERIFY(client.activeLayer()!=background);client.removeLayer(background);QTRY_COMPARE(client.layers().size(),1);canvas->actualSize();
            QImage tip(32,16,QImage::Format_RGBA8888);tip.fill(Qt::white);
            for(int y=4;y<12;++y)for(int x=12;x<20;++x)tip.setPixelColor(x,y,Qt::transparent);
            const auto input=temp.filePath("cursor.png");QVERIFY(tip.save(input));QVERIFY(client.importBrushTip(QUrl::fromLocalFile(input),1));QTRY_VERIFY(!client.brushFileBusy());
            auto *library=client.brushLibrary();library->setRadius(48);library->setRoundness(.5);library->setAngle(0);
            const auto hoverAt=canvas->documentRect().center();QHoverEvent hover(QEvent::HoverMove,hoverAt,canvas->mapToGlobal(hoverAt),hoverAt);QCoreApplication::sendEvent(canvas,&hover);
            const bool initial=canvas->brushCursorVisible();if(!initial)QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());
            for(const auto &v:workspace.leftGroups()+workspace.rightGroups()){const auto g=v.toMap();if(g.value("panels").toStringList().contains("brush-settings"))workspace.setActive(g.value("id").toString(),"brush-settings");}
            QQuickItem *category=nullptr;QTRY_VERIFY((category=findVisualItem(window->contentItem(),"brushCategory:7")));
            QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,category->mapToScene({category->width()/2,category->height()/2}).toPoint());
            auto *toggle=findVisualItem(window->contentItem(),"brushCursorImageEnabled");QVERIFY(toggle);QTRY_VERIFY(toggle->isVisible());
            QVERIFY(client.cursorTipImage().isEmpty());QVERIFY(!library->cursorImage());
            QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({toggle->width()/2,toggle->height()/2}).toPoint());QVERIFY(library->cursorImage());
            QTRY_VERIFY(!client.cursorTipImage().isEmpty());const auto source=client.cursorTipImage();
            const auto raster=QImage::fromData(QByteArray::fromBase64(source.mid(source.indexOf(',')+1).toLatin1()));QCOMPARE(raster.size(),QSize(256,256));
            QCOMPARE(raster.pixelColor(128,128).alpha(),0);QVERIFY(raster.pixelColor(30,128).alpha()>250);
            for(int y=0;y<256;++y)for(int x=0;x<256;++x){const auto c=raster.pixelColor(x,y);if(c.alpha()>0)QCOMPARE(c.red(),255);}
            auto *image=findVisualItem(window->contentItem(),"brushCursorImage");QVERIFY(image);
            library->setCursorOutline(false);QCoreApplication::sendEvent(canvas,&hover);QTRY_VERIFY(image->isVisible());QVERIFY(canvas->brushCursorVisible());QCOMPARE(image->property("cache").toBool(),false);
            auto *outline=findVisualItem(window->contentItem(),"brushCursorOutline");QVERIFY(outline);QVERIFY(!outline->isVisible());
            const auto revision=client.revision();const auto depth=client.undoDepth();
            library->setCursorImageOpacity(73);QTRY_COMPARE(image->opacity(),.73);QCOMPARE(image->height()/image->width(),.5);
            library->setAngle(30);QCoreApplication::sendEvent(canvas,&hover);QTRY_COMPARE(image->rotation(),30.);
            library->setAngle(0);QCoreApplication::sendEvent(canvas,&hover);
            client.setBrushColor(QColor("#ef3456"));client.setSecondaryBrushColor(QColor("#123456"));QCOMPARE(client.cursorTipImage(),source);
            library->setCursorImage(false);QTRY_VERIFY(!image->isVisible());QVERIFY(client.cursorTipImage().isEmpty());QCOMPARE(canvas->cursor().shape(),Qt::CrossCursor);
            const auto before=window->grabWindow();library->setCursorImage(true);QTRY_VERIFY(image->isVisible());QTRY_VERIFY(!client.cursorTipImage().isEmpty());QTest::qWait(100);
            const auto after=window->grabWindow();const auto point=image->mapToScene({image->width()*.15,image->height()*.5})*window->devicePixelRatio();
            const auto oldPixel=before.pixelColor(point.toPoint()),newPixel=after.pixelColor(point.toPoint());QVERIFY2(newPixel.red()>oldPixel.red()+10,qPrintable(QString("overlay %1 -> %2").arg(oldPixel.name(),newPixel.name())));
            if(!qEnvironmentVariable("DRAWVERSE_CURSOR_IMAGE_CAPTURE").isEmpty())QVERIFY(after.save(qEnvironmentVariable("DRAWVERSE_CURSOR_IMAGE_CAPTURE")));
            QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(!image->isVisible());QTest::keyClick(window,Qt::Key_CapsLock);QTRY_VERIFY(image->isVisible());
            client.activateEyedropper();QVERIFY(!image->isVisible());client.activateBrush();QCoreApplication::sendEvent(canvas,&hover);QTRY_VERIFY(image->isVisible());
            library->setCursorSimple(true);QVERIFY(!image->isVisible());library->setCursorSimple(false);QTRY_VERIFY(image->isVisible());
            QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),depth);
            InputSample sample;sample.position={128,128};QVERIFY(client.beginStroke(sample));const auto captured=client.cursorTipImage();QVERIFY(!captured.isEmpty());
            library->useProceduralTip();QCOMPARE(client.cursorTipImage(),captured);
            library->setCursorImage(false);library->setCursorImage(true);QTRY_COMPARE(client.cursorTipImage(),captured);
            library->setCursorHoverOnly(true);QVERIFY(!image->isVisible());library->setCursorHoverOnly(false);QTRY_VERIFY(image->isVisible());
            client.endStroke();QTRY_VERIFY(client.cursorTipImage().isEmpty());QTRY_VERIFY(client.undoDepth()>depth);
            QVERIFY(!library->createOutlinePreset(true).isEmpty());library->setRadius(36);
            QVERIFY(client.importBrushTip(QUrl::fromLocalFile(input),1));QTRY_VERIFY(!client.brushFileBusy());
            QImage secondTip(16,32,QImage::Format_RGBA8888);secondTip.fill(Qt::transparent);
            for(int y=3;y<29;++y)for(int x=2;x<14;++x)secondTip.setPixelColor(x,y,Qt::white);
            const auto secondInput=temp.filePath("second-cursor.png");QVERIFY(secondTip.save(secondInput));
            QVERIFY(client.importDualBrushTip(QUrl::fromLocalFile(secondInput),1));QTRY_VERIFY(!client.brushFileBusy());
            QVERIFY(client.cursorSecondaryTipImage().isEmpty());library->setCursorComposite(true);
            QTRY_VERIFY(!client.cursorTipImage().isEmpty() && !client.cursorSecondaryTipImage().isEmpty());
            const auto mainSource=client.cursorTipImage(),secondSource=client.cursorSecondaryTipImage();QVERIFY(mainSource!=secondSource);
            QCoreApplication::sendEvent(canvas,&hover);auto *secondImage=findVisualItem(window->contentItem(),"secondaryBrushCursorImage");QVERIFY(secondImage);QTRY_VERIFY(secondImage->isVisible());
            QVERIFY(client.beginStroke(sample));library->useProceduralTip();auto dual=library->dualBrush();dual["resource"]=QVariant{};QVERIFY(library->setDualBrush(dual));
            QCOMPARE(client.cursorTipImage(),mainSource);QCOMPARE(client.cursorSecondaryTipImage(),secondSource);
            library->setCursorComposite(false);QVERIFY(!secondImage->isVisible());library->setCursorComposite(true);QTRY_COMPARE(client.cursorSecondaryTipImage(),secondSource);
            library->setCursorImage(false);library->setCursorImage(true);QTRY_COMPARE(client.cursorTipImage(),mainSource);QTRY_COMPARE(client.cursorSecondaryTipImage(),secondSource);
            client.cancelStroke();QTRY_VERIFY(client.cursorTipImage().isEmpty() && client.cursorSecondaryTipImage().isEmpty());
            client.clearError();QVERIFY(library->setMaskResource(library->selectedId(),QString(64,'0')));QTRY_VERIFY(!client.lastError().isEmpty());QTRY_VERIFY(library->maskResource()!=QString(64,'0'));
            library->useProceduralTip();QTRY_VERIFY(client.cursorImageMessage().isEmpty());QVERIFY(client.cursorTipImage().isEmpty());
            if(!initial)QTest::keyClick(window,Qt::Key_CapsLock);
            auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);
            const auto peerId=documents->newDocument(16,16,true);QVERIFY(!peerId.isEmpty());auto *peer=documents->client(peerId);QVERIFY(peer);QTRY_VERIFY(peer->ready());QVERIFY(peer->brushLibrary()->cursorImage());QCOMPARE(peer->brushLibrary()->cursorImageOpacity(),73);
            peer->brushLibrary()->setCursorImageOpacity(42);QCOMPARE(library->cursorImageOpacity(),42);library->setCursorImageOpacity(73);QCOMPARE(peer->brushLibrary()->cursorImageOpacity(),73);
            QVERIFY(warnings.isEmpty());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
        }
        PaintCoreClient reopened(nullptr,preferences);QTRY_VERIFY(reopened.ready());QVERIFY(reopened.brushLibrary()->cursorImage());QCOMPARE(reopened.brushLibrary()->cursorImageOpacity(),73);
        QSignalSpy stopped(&reopened,&PaintCoreClient::stopped);reopened.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void brushCursorPreferencesAreGlobalPersistentAndNeverExported() {
        BrushLibrary model;QSignalSpy preview(&model,&BrushLibrary::previewRequested);QSignalSpy settings(&model,&BrushLibrary::settingsChanged);
        model.setCursorCenter(true);model.setCursorHoverOnly(true);model.setCursorWidth(3);model.setCursorOutline(false);model.setCursorSimple(true);model.setCursorColor("#123456");model.setCursorSecondaryColor("#abcdef");QCOMPARE(settings.size(),0);QCOMPARE(preview.size(),0);
        BrushLibrary restored;QVERIFY(restored.restore(model.snapshot()));QVERIFY(restored.cursorCenter() && restored.cursorHoverOnly());QVERIFY(!restored.cursorOutline());QCOMPARE(restored.cursorWidth(),3);QVERIFY(restored.cursorSimple());QCOMPARE(restored.cursorColor(),QString("#123456"));QCOMPARE(restored.cursorSecondaryColor(),QString("#abcdef"));restored.setCursorColor("invalid");QCOMPARE(restored.cursorColor(),QString("#123456"));
        const auto state=restored.snapshot();auto invalid=QJsonDocument::fromJson(state).object();invalid["cursor"]=QJsonObject{{"outline",true},{"center",false},{"hoverOnly",false},{"width",8}};QVERIFY(!restored.restore(QJsonDocument(invalid).toJson()));QCOMPARE(restored.snapshot(),state);
        QVERIFY(!model.exportPackage().contains("cursor"));const auto copy=model.saveCopy("Cursor independent");QVERIFY(!copy.isEmpty());QVERIFY(!model.cursorOutline());auto old=QJsonDocument::fromJson(state).object();old.remove("cursor");QVERIFY(restored.restore(QJsonDocument(old).toJson()));QVERIFY(restored.cursorOutline() && !restored.cursorCenter());QCOMPARE(restored.cursorWidth(),1);
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();
        QHoverEvent hover(QEvent::HoverMove,{50,60},canvas->mapToGlobal({50,60}),{40,50});QCoreApplication::sendEvent(canvas,&hover);const bool initial=canvas->brushCursorVisible();if(!initial)QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());
        auto *library=client.brushLibrary();library->setRoundness(.2);library->setAngle(90);library->setCursorSimple(true);auto *outline=findVisualItem(window->contentItem(),"brushCursorOutline");QVERIFY(outline);QCOMPARE(outline->height(),outline->width());QCOMPARE(outline->rotation(),0.);library->setCursorOutline(false);QVERIFY(!canvas->brushCursorVisible());QCOMPARE(canvas->cursor().shape(),Qt::CrossCursor);library->setCursorCenter(true);QVERIFY(canvas->brushCursorVisible());auto *cross=findVisualItem(window->contentItem(),"brushCursorCenterCross");QVERIFY(cross);QVERIFY(cross->isVisible());
        library->setCursorHoverOnly(true);InputSample sample;sample.position={10,10};QVERIFY(client.beginStroke(sample));QVERIFY(!canvas->brushCursorVisible());client.endStroke();QVERIFY(canvas->brushCursorVisible());
        QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(!canvas->brushCursorVisible());QTest::keyClick(window,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());if(!initial)QTest::keyClick(window,Qt::Key_CapsLock);
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);const auto id=documents->newDocument(32,32,true);QVERIFY(!id.isEmpty());auto *other=documents->client(id);QVERIFY(other);QTRY_VERIFY(other->ready());QVERIFY(other->brushLibrary()->cursorCenter());QVERIFY(!other->brushLibrary()->cursorOutline());other->brushLibrary()->setCursorWidth(2);QCOMPARE(library->cursorWidth(),2);library->setHardness(.25);QCOMPARE(other->brushLibrary()->hardness(),.25);
        const auto firstId=documents->documents().first().toMap().value("id").toString();QSignalSpy firstStopped(&client,&PaintCoreClient::stopped);QVERIFY(documents->closeDocument(firstId,true));QTRY_COMPARE(firstStopped.size(),1);other->brushLibrary()->setCursorWidth(3);
        const auto thirdId=documents->newDocument(16,16,true);QVERIFY(!thirdId.isEmpty());auto *third=documents->client(thirdId);QVERIFY(third);QTRY_VERIFY(third->ready());QCOMPARE(third->brushLibrary()->cursorWidth(),3);QCOMPARE(third->brushLibrary()->hardness(),.25);
        QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
        PaintCoreClient reloaded(nullptr,temp.filePath("storage.ini"));QTRY_VERIFY(reloaded.ready());QVERIFY(reloaded.brushLibrary()->cursorCenter());QCOMPARE(reloaded.brushLibrary()->cursorWidth(),3);QCOMPARE(reloaded.brushLibrary()->hardness(),.25);QSignalSpy stoppedAgain(&reloaded,&PaintCoreClient::stopped);reloaded.shutdown();QTRY_COMPARE(stoppedAgain.size(),1);
    }
    void mainWindowRestoresWithoutNativeFrameOrClippedContent() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
#ifdef Q_OS_WIN
        if(QGuiApplication::platformName()=="windows") {
            const auto style=GetWindowLongPtrW(reinterpret_cast<HWND>(window->winId()),GWL_STYLE);
            QVERIFY2(style&WS_MINIMIZEBOX,"Taskbar minimization requires WS_MINIMIZEBOX on the frameless HWND");
            QVERIFY(style&WS_MAXIMIZEBOX);QVERIFY(style&WS_SYSMENU);QVERIFY(!(style&WS_CAPTION));
        }
#endif
        for(bool maximized:{false,true})for(int cycle=0;cycle<3;++cycle) {
            const auto normalBefore=window->geometry();
            if(maximized)window->showMaximized();else window->showNormal();QTest::qWait(80);
            const auto before=window->geometry();
#ifdef Q_OS_WIN
            if(QGuiApplication::platformName()=="windows" && cycle!=1)SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_MINIMIZE,0);
            else
#endif
                {auto *minimize=findVisualItem(window->contentItem(),"windowMinimize");QVERIFY(minimize);QVERIFY(QMetaObject::invokeMethod(minimize,"clicked"));}
            QTRY_COMPARE(window->visibility(),QWindow::Minimized);QTest::qWait(80);
#ifdef Q_OS_WIN
            if(QGuiApplication::platformName()=="windows" && maximized && cycle==2) {
                const auto handle=reinterpret_cast<HWND>(window->winId());
                // A second taskbar action can arrive before the queued Qt
                // maximized-state restoration; it must stay minimized.
                SendMessageW(handle,WM_SYSCOMMAND,SC_RESTORE,0);
                SendMessageW(handle,WM_SYSCOMMAND,SC_MINIMIZE,0);
                QTest::qWait(100);QTRY_COMPARE(window->visibility(),QWindow::Minimized);QVERIFY(IsIconic(handle));
            }
            if(QGuiApplication::platformName()=="windows") {
                const auto handle=reinterpret_cast<HWND>(window->winId());QVERIFY(IsIconic(handle));
                RECT bounds{};QVERIFY(GetWindowRect(handle,&bounds));
                QVERIFY2(!QRect(bounds.left,bounds.top,bounds.right-bounds.left,bounds.bottom-bounds.top).intersects(before),"Minimization must remove the window, including its native iconic caption, from the workspace");
                SendMessageW(handle,WM_SYSCOMMAND,SC_RESTORE,0);
            }
            else
#endif
                {if(maximized)window->showMaximized();else window->showNormal();}
            QTRY_COMPARE(window->visibility(),maximized?QWindow::Maximized:QWindow::Windowed);QTRY_COMPARE(window->geometry(),before);QTest::qWait(100);
#ifdef Q_OS_WIN
            if(QGuiApplication::platformName()=="windows") {
                const auto handle=reinterpret_cast<HWND>(window->winId());RECT outer{},inner{};QVERIFY(GetWindowRect(handle,&outer));QVERIFY(GetClientRect(handle,&inner));
                QVERIFY(!(GetWindowLongPtrW(handle,GWL_STYLE)&WS_CAPTION));QVERIFY(!IsIconic(handle));
                QCOMPARE(inner.right,outer.right-outer.left);QCOMPARE(inner.bottom,outer.bottom-outer.top);
                const auto region=CreateRectRgn(0,0,0,0);const int kind=GetWindowRgn(handle,region);
                const bool centerVisible=kind==ERROR || PtInRegion(region,inner.right/2,inner.bottom/2);
                const bool rounded=kind==COMPLEXREGION && !PtInRegion(region,0,0) && !PtInRegion(region,inner.right-1,0);
                DeleteObject(region);QVERIFY(centerVisible);QCOMPARE(rounded,!maximized);
            }
#endif
            const auto image=window->grabWindow();QVERIFY(!image.isNull());QVERIFY(std::abs(image.width()-qRound(window->width()*window->devicePixelRatio()))<=1);QVERIFY(std::abs(image.height()-qRound(window->height()*window->devicePixelRatio()))<=1);QVERIFY(image.pixelColor(image.width()/2,image.height()/2).alpha()>240);
            auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);QVERIFY(canvas->width()>0);QVERIFY(canvas->height()>0);
            if(maximized) {window->showNormal();QTRY_COMPARE(window->geometry(),normalBefore);}
        }
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void dockSeparatorsUseThinDarkGrayAcrossMainAndFloatingColumns() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        const auto check=[&](QQuickWindow *host,const QString &id) {
            int count=0;
            for(const auto &value:workspace.layoutItems(id,host->width(),host->height())) {
                const auto data=value.toMap();if(data.value("kind")!="split")continue;
                auto *divider=findVisualItem(host->contentItem(),"dockDivider:"+data.value("id").toString());
                if(!divider || divider->property("color").value<QColor>()!=QColor("#1b1c1f"))return false;
                if(data.value("axis")=="horizontal"?divider->width()!=1:divider->height()!=1)return false;
                ++count;
            }
            return count>0;
        };
        QTest::mouseMove(window,{500,300});QTRY_VERIFY(check(window,"main"));
        const auto first=workspace.leftGroups().first().toMap().value("id").toString(),second=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachGroup(first);
        const auto payload=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",second},{"whole",true}}).toJson());
        QVERIFY(workspace.dockPayload(payload,"floating",first,"after"));
        QQuickWindow *floating=nullptr;
        QTRY_VERIFY((floating=window->findChild<QQuickWindow*>("floatingDock:"+first)));
        QTest::mouseMove(floating,{100,100});QTRY_VERIFY(check(floating,first));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void flexibleDockTreeSupportsFourEdgesAndValidatesPersistence() {
        QTemporaryDir temp;const auto path=temp.filePath("docks.ini");WorkspaceManager workspace(path);
        const auto data=[](const QString &id){return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",id},{"whole",true}}).toJson());};
        const auto rect=[&](const QString &host,const QString &id){for(const auto &v:workspace.layoutItems(host,1400,800))if(v.toMap().value("id")==id)return v.toMap().value("rect").toRectF();return QRectF();};
        for(const auto &v:workspace.leftGroups()+workspace.rightGroups())for(const auto &edge:QStringList{"left","right"}) {
            const auto target=v.toMap().value("id").toString(),location=v.toMap().value("location").toString();
            QVERIFY(workspace.dockToolStrip("drawverse-tools-v1",location,target,edge));const auto a=rect("main","__toolstrip"),b=rect("main",target);QCOMPARE(a.width(),qreal(36));
            if(edge=="left")QVERIFY(a.right()<=b.left()-1);else QVERIFY(b.right()<=a.left()-1);
        }
        QVERIFY(!workspace.dockToolStrip("drawverse-tools-v1","center","__canvas","right"));
        QVERIFY(!workspace.dockPayload(data("__canvas"),"floating"));
        for(const auto &edge:QStringList{"left","right","top","bottom"}) {
            workspace.resetLayout();const auto source=workspace.leftGroups().first().toMap().value("id").toString(),target=workspace.rightGroups().first().toMap().value("id").toString();
            QVERIFY(workspace.dockPayload(data(source),"right",target,edge));const auto a=rect("main",source),b=rect("main",target);QVERIFY(!a.intersects(b));
            if(edge=="left")QVERIFY(a.right()<=b.left()-1);if(edge=="right")QVERIFY(b.right()<=a.left()-1);
            if(edge=="top")QVERIFY(a.bottom()<=b.top()-1);if(edge=="bottom")QVERIFY(b.bottom()<=a.top()-1);
        }
        workspace.resetLayout();const auto left=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(left);
        QCOMPARE(rect("main","__canvas").left(),rect("main","__toolstrip").right()+1);
        const auto color=workspace.rightGroups().first().toMap().value("id").toString();
        QVERIFY(workspace.dockPayload(data(color),"floating",left,"after"));QCOMPARE(workspace.columnGroups(color),QStringList({left,color}));
        workspace.setColumnCollapsed(left,true);workspace.setColumnCollapsed(color,true);
        const auto a=rect(left,left),b=rect(left,color);QCOMPARE(b.top(),a.bottom()+8);QVERIFY(b.bottom()<200);
        workspace.setColumnCollapsed(color,true);workspace.saveLayout();const auto layout=workspace.layoutItems(left,1400,800);WorkspaceManager restored(path);QCOMPARE(restored.layoutItems(left,1400,800),layout);
        QSettings settings(path,QSettings::IniFormat);const auto valid=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();
        QCOMPARE(valid.value("version").toInt(),6);
        for(const auto &invalid:QStringList{"duplicate","missing","ratio","window","geometry","icons"}) {
            auto root=valid;auto docks=root.value("docks").toObject();
            if(invalid=="duplicate")docks.insert(left,DockTree::leaf("__canvas"));
            if(invalid=="missing")docks.insert(left,DockTree::leaf(color));
            if(invalid=="ratio"){auto node=docks.value(left).toObject();node.insert("ratio",1.5);docks.insert(left,node);}
            if(invalid=="window")docks.insert("orphan",DockTree::leaf("unknown"));
            if(invalid=="geometry"){auto windows=root.value("windows").toObject();windows.insert(left,QJsonObject{{"width",-1}});root.insert("windows",windows);}
            if(invalid=="icons")root.insert("iconGroups",QJsonArray{"unknown"});
            root.insert("docks",docks);settings.setValue("workspace",QJsonDocument(root).toJson());settings.sync();
            QVERIFY(!restored.restoreLayout());QCOMPARE(restored.layoutItems(left,1400,800),layout);
        }
        auto oldTree=valid;oldTree.insert("version",5);oldTree.remove("iconGroups");auto mixed=oldTree.value("docks").toObject();auto mainTree=mixed.value("main").toObject();QVERIFY(DockTree::remove(mainTree,"__canvas"));mixed.insert("main",mainTree);mixed.insert(left,DockTree::split(mixed.value(left).toObject(),DockTree::leaf("__canvas"),"horizontal",.5));oldTree.insert("docks",mixed);settings.setValue("workspace",QJsonDocument(oldTree).toJson());settings.sync();WorkspaceManager separated(path);QCOMPARE(separated.groupDefinition("__canvas").value("host").toString(),QString("main"));QCOMPARE(separated.columnGroups(left),QStringList({left,color}));QCOMPARE(separated.floatingWindows().size(),1);
        auto legacy=valid;legacy.insert("version",4);legacy.remove("docks");legacy.remove("windows");settings.setValue("workspace",QJsonDocument(legacy).toJson());settings.sync();WorkspaceManager migrated(path);QCOMPARE(migrated.visiblePanels().size(),6);
        settings.sync();QCOMPARE(QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object().value("version").toInt(),6);
    }
    void floatingCanvasAndToolsDockAcrossNativeWindows() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;
        QStringList warnings;connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));
        QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);QPointer<CanvasItem> retained=canvas;
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.frame().isNull());
        auto *grip=findVisualItem(main->contentItem(),"canvasGrip");QVERIFY(grip);
        QTest::mouseDClick(main,Qt::LeftButton,Qt::NoModifier,grip->mapToScene({grip->width()/2,grip->height()/2}).toPoint());
        QTRY_COMPARE(applicationWindows().size(),2);QTRY_VERIFY(canvas->window()!=main);
        auto *floating=canvas->window();QVERIFY(floating);QPointer<QQuickWindow> retainedWindow=floating;
        floating->requestActivate();QTRY_VERIFY(floating->isActive());QTRY_VERIFY(canvas->height()>200);canvas->actualSize();
        client.setBrushRadius(4);client.setBrushColor(Qt::red);
        const auto paintAt=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(32,32)).toPoint();
        const auto centerColor=[&] {
            const auto region=client.frameRegion();const auto frame=client.frame();
            if(frame.isNull() || region.isEmpty())return QColor();
            return frame.pixelColor(qRound((32-region.x())/region.width()*frame.width()),qRound((32-region.y())/region.height()*frame.height()));
        };
        QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,paintAt);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(centerColor().red()>200);
        QTest::keyClick(floating,Qt::Key_Z,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),0);
        QTest::keyClick(floating,Qt::Key_Y,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),1);
        const auto at=canvas->mapToScene({canvas->width()/2,canvas->height()/2});QWheelEvent wheel(at,floating->mapToGlobal(at.toPoint()),{},QPoint(0,120),Qt::NoButton,Qt::NoModifier,Qt::NoScrollPhase,false);
        QCoreApplication::sendEvent(floating,&wheel);QVERIFY(canvas->zoom()>1);const auto zoom=canvas->zoom();
        const auto panAt=canvas->mapToScene({canvas->width()/2,canvas->height()/2}).toPoint();
        QTest::mousePress(floating,Qt::MiddleButton,Qt::NoModifier,panAt);QTest::mouseMove(floating,panAt+QPoint(12,8));QTest::mouseRelease(floating,Qt::MiddleButton,Qt::NoModifier,panAt+QPoint(12,8));
        const auto pan=canvas->documentRect().center()-QPointF(canvas->width()/2,canvas->height()/2);QCOMPARE(pan,QPointF(12,8));
        auto *documents=main->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QCOMPARE(documents->windows().size(),1);
        const auto first=documents->activeId(),host=documents->windows().first().toMap().value("id").toString();
        auto *extraTitle=findVisualItem(floating->contentItem(),"documentWindowTitle:"+host);QVERIFY(extraTitle);QCOMPARE(extraTitle->height(),qreal(0));
        auto *documentArea=findVisualItem(floating->contentItem(),"documentArea:"+host);QVERIFY(documentArea);QCOMPARE(documentArea->y(),qreal(2));
        auto *singleTitle=findVisualItem(floating->contentItem(),"documentTab:"+first);QVERIFY(singleTitle);QCOMPARE(singleTitle->property("color").value<QColor>(),floating->color());QCOMPARE(floating->color(),QColor("#202226"));
        for(auto *item:documentArea->findChildren<QObject*>())QVERIFY(item->property("tooltip").toString()!=QStringLiteral("新建画布"));
        const auto singlePreview=qEnvironmentVariable("DRAWVERSE_DOCK_PREVIEW");if(!singlePreview.isEmpty()){QTest::qWait(80);QVERIFY(floating->grabWindow().save(singlePreview+".single.png"));}
        const auto second=documents->newDocument(96,48,true);QVERIFY(!second.isEmpty());auto *other=documents->client(second);QVERIFY(other && other!=&client);QTRY_VERIFY(other->ready() && other->documentWidth()==96);
        other->setBrushColor(Qt::blue);other->setBrushRadius(3);InputSample sample;sample.position={20,20};QVERIFY(other->beginStroke(sample));other->endStroke();QTRY_COMPARE(other->undoDepth(),1);QCOMPARE(client.undoDepth(),1);
        const auto url=QUrl::fromLocalFile(temp.filePath("second.ora"));QSignalSpy files(other,&PaintCoreClient::fileFinished);QVERIFY(other->saveDocument(url));QTRY_COMPARE_WITH_TIMEOUT(files.size(),1,10000);QVERIFY(files.first().first().toBool());
        QVERIFY(documents->moveDocument(second,host));QTRY_COMPARE(documents->windows().size(),1);QTRY_COMPARE(applicationWindows().size(),2);
        QTRY_COMPARE(extraTitle->height(),qreal(22));QCOMPARE(floating->title(),QString("second.ora"));
        QVERIFY(documents->activate(first));QTRY_VERIFY(floating->title().startsWith(QStringLiteral("未命名")));QTRY_COMPARE(canvas->window(),floating);QCOMPARE(retained.data(),canvas);QCOMPARE(canvas->zoom(),zoom);QCOMPARE(canvas->documentRect().center()-QPointF(canvas->width()/2,canvas->height()/2),pan);
        // Each document keeps its own frame, history and layer tree in a tab group.
        QVERIFY(documents->activate(second));QTRY_COMPARE(floating->title(),QString("second.ora"));other->undo();QTRY_COMPARE(other->undoDepth(),0);QCOMPARE(client.undoDepth(),1);QVERIFY(documents->activate(first));
        auto *empty=findVisualItem(main->contentItem(),"documentArea:main");QVERIFY(empty);QCOMPARE(empty->property("color").value<QColor>(),QColor("#17191C"));
        const auto tile=workspace.groupDefinition(workspace.leftGroups().first().toMap().value("id").toString());QVERIFY(tile.value("host")=="main");
        QPointingDevice pen("Floating tablet",2001,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,2);
        const auto tabletAt=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(48,40)*canvas->zoom());
        QTabletEvent press(QEvent::TabletPress,&pen,tabletAt,floating->mapToGlobal(tabletAt.toPoint()),.75,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(floating,&press);QVERIFY(press.isAccepted());
        QTabletEvent release(QEvent::TabletRelease,&pen,tabletAt,floating->mapToGlobal(tabletAt.toPoint()),0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(floating,&release);QTRY_COMPARE(client.undoDepth(),2);
        QVERIFY(!documents->closeDocument(first));QCOMPARE(documents->documents().size(),2);
        const auto preview=qEnvironmentVariable("DRAWVERSE_DOCK_PREVIEW");if(!preview.isEmpty()){QTest::qWait(100);QVERIFY(floating->grabWindow().save(preview));}
        const auto original=floating->position();const auto gripAt=QPoint(24,10);QTest::mousePress(floating,Qt::LeftButton,Qt::NoModifier,gripAt);documents->beginDrag(first,true);QTRY_VERIFY(documents->dragging());
        QTest::mouseMove(floating,gripAt+QPoint(16,12));QTRY_VERIFY(floating->position()!=original);QVERIFY(documents->dragTarget().isEmpty());QTest::keyClick(floating,Qt::Key_Escape);QTRY_VERIFY(!documents->dragging());QTRY_COMPARE(floating->position(),original);QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,gripAt);
        QTest::mousePress(floating,Qt::LeftButton,Qt::NoModifier,gripAt);documents->beginDrag(first,true);QTRY_VERIFY(documents->dragging());
        const auto body=empty->mapToGlobal({empty->width()/2,100}).toPoint();QTest::mouseMove(floating,floating->mapFromGlobal(body));QVERIFY(documents->dragTarget().isEmpty());
        const auto edge=empty->mapToGlobal({empty->width()/2,10}).toPoint();QTest::mouseMove(floating,floating->mapFromGlobal(edge));QTRY_COMPARE(documents->dragTarget(),QString("main"));QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,floating->mapFromGlobal(edge));
        QTRY_COMPARE(applicationWindows().size(),1);QTRY_COMPARE(canvas->window(),main);QVERIFY(retainedWindow.isNull());QCOMPARE(canvas->zoom(),zoom);
        // A multi-document float returns to one title row after removing its other tab.
        documents->floatDocument(first);QTRY_COMPARE(documents->windows().size(),1);const auto oneHost=documents->windows().first().toMap().value("id").toString();
        QQuickWindow *oneWindow=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="documentWindow:"+oneHost)oneWindow=qobject_cast<QQuickWindow*>(w);return oneWindow!=nullptr;}());
        auto *oneTitle=findVisualItem(oneWindow->contentItem(),"documentWindowTitle:"+oneHost);QVERIFY(oneTitle);QCOMPARE(oneTitle->height(),qreal(0));
        QVERIFY(documents->moveDocument(second,oneHost));QTRY_COMPARE(oneTitle->height(),qreal(22));QVERIFY(documents->closeDocument(second,true));QTRY_COMPARE(oneTitle->height(),qreal(0));
        documents->returnWindow(oneHost);QTRY_COMPARE(applicationWindows().size(),1);
        const auto replacement=documents->newDocument(96,48,true);QVERIFY(!replacement.isEmpty());QTRY_VERIFY(documents->client(replacement)->ready() && documents->client(replacement)->documentWidth()==96);
        const auto opened=documents->openDocument(url);QVERIFY(!opened.isEmpty());auto *reopened=documents->client(opened);QVERIFY(reopened);QTRY_VERIFY(reopened->ready() && !reopened->fileBusy() && reopened->documentUrl()==url);QCOMPARE(reopened->documentWidth(),96);QCOMPARE(reopened->undoDepth(),0);
        // A later save failure must not reuse the initial-open failure handler.
        QSignalSpy reopenedFiles(reopened,&PaintCoreClient::fileFinished);
        QVERIFY(main->setProperty("savingBeforeAction",true));QVERIFY(main->setProperty("exiting",true));QVERIFY(main->setProperty("pendingAction","closeDocument"));
        QVERIFY(reopened->saveDocument(QUrl::fromLocalFile(temp.filePath("second.ora/blocked.ora"))));
        QTRY_COMPARE_WITH_TIMEOUT(reopenedFiles.size(),1,10000);QVERIFY(!reopenedFiles.first().first().toBool());
        QCOMPARE(documents->documents().size(),3);QCOMPARE(documents->client(opened),reopened);QVERIFY(reopened->ready());
        QVERIFY(!main->property("exiting").toBool());QVERIFY(!main->property("savingBeforeAction").toBool());QVERIFY(main->property("pendingAction").toString().isEmpty());
        QSignalSpy openFailures(documents,&DocumentManager::fileFailed);
        const auto missing=documents->openDocument(QUrl::fromLocalFile(temp.filePath("missing.ora")));QVERIFY(!missing.isEmpty());
        QTRY_COMPARE_WITH_TIMEOUT(openFailures.size(),1,10000);QTRY_COMPARE(documents->documents().size(),3);QVERIFY(!documents->client(missing));
        documents->requestClose(first);auto *unsaved=main->findChild<QObject*>("unsavedDocumentDialog");QVERIFY(unsaved);QTRY_VERIFY(unsaved->property("opened").toBool());QVERIFY(!documents->activate(replacement));QVERIFY(QMetaObject::invokeMethod(unsaved,"reject"));QTRY_VERIFY(!unsaved->property("opened").toBool());QCOMPARE(documents->documents().size(),3);
        // Closing the application first resolves the correct document, even with a file task running.
        QSignalSpy exitFiles(&client,&PaintCoreClient::fileFinished);
        QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath("first.ora"))));QVERIFY(client.fileBusy());
        QVERIFY(QMetaObject::invokeMethod(main,"close"));auto *busy=main->findChild<QObject*>("busyDocumentDialog");QVERIFY(busy);
        QTRY_VERIFY(busy->property("opened").toBool());QCOMPARE(main->property("pendingDocument").toString(),first);QCOMPARE(main->property("pendingAction").toString(),QString("closeDocument"));
        QVERIFY(QMetaObject::invokeMethod(busy,"reject"));QTRY_VERIFY(!busy->property("opened").toBool());QVERIFY(!main->property("exiting").toBool());QCOMPARE(documents->documents().size(),3);
        QTRY_COMPARE_WITH_TIMEOUT(exitFiles.size(),1,10000);QVERIFY(exitFiles.first().first().toBool());
        QVERIFY(documents->closeDocument(first,true));QVERIFY(documents->closeDocument(replacement,true));QVERIFY(documents->closeDocument(opened));QCOMPARE(documents->documents().size(),0);
        const auto fresh=documents->newDocument(24,24);QVERIFY(!fresh.isEmpty());QTRY_VERIFY(documents->client(fresh)->ready() && documents->client(fresh)->documentWidth()==24);QVERIFY(documents->closeDocument(fresh,true));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void panelColumnsUseOneHeaderAndDragTheActualWindow() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("workspace.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto first=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(first);workspace.detachPanel(first,"brush-settings");
        QString second;for(const auto &v:workspace.floatingGroups())if(v.toMap().value("active")=="brush-settings")second=v.toMap().value("id").toString();QVERIFY(!second.isEmpty());
        const auto payload=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",second},{"whole",true}}).toJson());QVERIFY(workspace.dockPayload(payload,"floating",first,"after"));
        QTRY_COMPARE(applicationWindows().size(),2);QQuickWindow *floating=nullptr;for(auto *w:applicationWindows())if(w!=main)floating=qobject_cast<QQuickWindow*>(w);QVERIFY(floating);
        auto *top=findVisualItem(floating->contentItem(),"columnHeader:"+first),*bottom=findVisualItem(floating->contentItem(),"columnHeader:"+second);QVERIFY(top && bottom);QVERIFY(top->isVisible());QVERIFY(!bottom->isVisible());
        auto *lowerBar=findVisualItem(floating->contentItem(),"panelTabBar:"+second),*backing=findVisualItem(floating->contentItem(),"floatingColumnBody");QVERIFY(lowerBar && backing);
        QVERIFY(std::abs(top->property("color").value<QColor>().alphaF()-.75)<.005);QCOMPARE(lowerBar->property("color").value<QColor>(),QColor("#05080a"));QCOMPARE(backing->property("color").value<QColor>(),QColor("#1c1e21"));
        const auto rendered=[&](QQuickItem *item,QPointF point){const auto image=floating->grabWindow();const auto at=item->mapToScene(point)*floating->devicePixelRatio();return image.pixelColor(at.toPoint());};
        QTRY_COMPARE(rendered(lowerBar,{lowerBar->width()/2,1}),QColor("#05080a"));QTRY_COMPARE(rendered(backing,{2,backing->height()-2}),QColor("#1c1e21"));
        const auto expanded=floating->size();workspace.setColumnCollapsed(first,true);
        QTRY_COMPARE(floating->width(),30);QTRY_COMPARE(floating->height(),expanded.height());
        auto *a=findVisualItem(floating->contentItem(),"dockTile:"+first),*b=findVisualItem(floating->contentItem(),"dockTile:"+second);QVERIFY(a && b);QTRY_COMPARE(b->mapToScene({0,0}).y(),a->mapToScene({0,0}).y()+a->height()+8);
        QTRY_VERIFY(findVisualItem(floating->contentItem(),"railPanel:brush-settings"));
        workspace.setColumnCollapsed(first,false);QTRY_COMPARE(floating->size(),expanded);
        const auto available=floating->screen()->availableGeometry();floating->setPosition({available.left()+(available.width()-floating->width())/2,available.top()});
        const auto before=floating->position();floating->requestActivate();QTRY_VERIFY(floating->isActive());
        const auto at=QPoint(30,4);QTest::mousePress(floating,Qt::LeftButton,Qt::NoModifier,at);workspace.beginDrag(first,"",true);QTRY_VERIFY(workspace.dragging());
        QTest::mouseMove(floating,at+QPoint(12,16));QTRY_VERIFY(floating->position()!=before);QVERIFY(workspace.dragTarget().isEmpty());
        const auto live=workspace.floatingWindows().first().toMap();QCOMPARE(live.value("x").toInt(),floating->x());QCOMPARE(live.value("y").toInt(),floating->y());
        QTest::keyClick(floating,Qt::Key_Escape);QTRY_VERIFY(!workspace.dragging());QTRY_COMPARE(floating->position(),before);QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,at);
        // A release near a different panel dynamically docks the whole column.
        const auto target=workspace.rightGroups().first().toMap().value("id").toString();auto *targetItem=findVisualItem(main->contentItem(),"dockGroup:"+target);QVERIFY(targetItem);
        QTest::mousePress(floating,Qt::LeftButton,Qt::NoModifier,at);workspace.beginDrag(first,"",true);QTRY_VERIFY(workspace.dragging());
        const auto global=targetItem->mapToGlobal({4,60}).toPoint();QTest::mouseMove(floating,floating->mapFromGlobal(global));QTRY_COMPARE(workspace.dragTarget(),target);QCOMPARE(workspace.dragPlacement(),QString("column-left"));
        QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,floating->mapFromGlobal(global));QTRY_VERIFY(!workspace.dragging());QTRY_COMPARE(applicationWindows().size(),1);QCOMPARE(workspace.groupDefinition(first).value("host").toString(),QString("main"));QCOMPARE(workspace.columnGroups(first),QStringList({first,second}));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void iconRailsSwitchSlideDismissAndDragPanels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.rightGroups().first().toMap().value("id").toString(),layers=workspace.rightGroups().last().toMap().value("id").toString(),brush=workspace.leftGroups().first().toMap().value("id").toString();
        workspace.detachPanel(layers,"history");const auto historyClass=workspace.groupForPanel("history");QVERIFY(workspace.dockPayload(QString::fromUtf8(QJsonDocument(QJsonObject{{"group",historyClass},{"whole",true}}).toJson()),"right",layers,"after"));
        workspace.setColumnCollapsed(color,true);workspace.detachPanel(brush,"brush-settings");QString incoming;
        for(const auto &v:workspace.floatingGroups())if(v.toMap().value("active")=="brush-settings")incoming=v.toMap().value("id").toString();QVERIFY(!incoming.isEmpty());
        const auto data=[](const QString &id){return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",id},{"whole",true}}).toJson());};
        QVERIFY(workspace.dockPayload(data(incoming),"right",layers,"after"));QTRY_COMPARE(applicationWindows().size(),1);
        const auto custom=workspace.addCustomPanel("Rail colors","palette");QVERIFY(!custom.isEmpty());
        const auto split=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",color},{"panel",custom},{"whole",false}}).toJson());
        QVERIFY(workspace.dockPayload(split,"right",color,"after"));
        for(const auto &id:workspace.columnGroups(color)) {QVERIFY(workspace.groupDefinition(id).value("icons").toBool());}
        for(const auto &v:workspace.layoutItems("main",main->width(),600)){const auto item=v.toMap();if(workspace.columnGroups(color).contains(item.value("id").toString())){QVERIFY(item.value("rail").toBool());QCOMPARE(item.value("rect").toRectF().width(),qreal(28));}}
        const auto click=[&](const QString &id){auto *icon=findVisualItem(main->contentItem(),"railPanel:"+id);if(!icon)return false;QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());return true;};
        const auto flyout=[&](const QString &panel)->QQuickWindow*{for(auto *w:applicationWindows())if(w->objectName().endsWith(":"+panel) && w->objectName().startsWith("panelFlyout:") && w->isVisible())return qobject_cast<QQuickWindow*>(w);return nullptr;};
        QVERIFY(click("history"));QTRY_VERIFY(flyout("history"));QPointer<QQuickWindow> history=flyout("history");QTRY_VERIFY(findVisualItem(history->contentItem(),"historyEntry:0"));
        QVERIFY(click("layers"));QTRY_VERIFY(flyout("layers"));QPointer<QQuickWindow> layer=flyout("layers");QVERIFY(history);QTRY_VERIFY(findVisualItem(layer->contentItem(),"layerList"));QVERIFY(findVisualItem(history->contentItem(),"historyEntry:0"));
        QCOMPARE(history->property("selectedPanel").toString(),QString("history"));QCOMPARE(layer->property("selectedPanel").toString(),QString("layers"));
        QVERIFY(click("color"));QTRY_VERIFY(flyout("color"));QPointer<QQuickWindow> palette=flyout("color");QTRY_COMPARE(applicationWindows().size(),4);
        QVERIFY(click("history"));QTRY_VERIFY(history.isNull());QVERIFY(layer && palette);QTRY_COMPARE(applicationWindows().size(),3);
        QVERIFY(click("history"));QTRY_VERIFY(flyout("history"));history=flyout("history");QTRY_VERIFY(history->isActive());QVERIFY(layer && palette);QTRY_COMPARE(applicationWindows().size(),4);
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            HWND front=nullptr;for(auto hwnd=GetTopWindow(nullptr);hwnd;hwnd=GetWindow(hwnd,GW_HWNDNEXT))if(hwnd==reinterpret_cast<HWND>(history->winId()) || hwnd==reinterpret_cast<HWND>(layer->winId()) || hwnd==reinterpret_cast<HWND>(palette->winId())){front=hwnd;break;}
            QCOMPARE(front,reinterpret_cast<HWND>(history->winId()));
        }
#endif
        auto *background=findVisualItem(main->contentItem(),"__railBackground:"+color);QVERIFY(background);QCOMPARE(background->property("color").value<QColor>(),QColor("#1c1e21"));
        const auto blank=background->mapToScene({background->width()/2,background->height()-12}).toPoint();QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,blank);
        QEvent deactivate(QEvent::WindowDeactivate);QCoreApplication::sendEvent(history,&deactivate);QTest::qWait(30);QVERIFY(history && layer && palette);QCOMPARE(applicationWindows().size(),4);
        const auto image=main->grabWindow();const auto dpr=main->devicePixelRatio();QCOMPARE(image.pixelColor(qRound(blank.x()*dpr),qRound(blank.y()*dpr)),QColor("#1c1e21"));
        palette->resize(std::min(260,palette->width()),palette->minimumHeight());QTest::qWait(30);
        const auto before=palette->position();auto *grip=findVisualItem(palette->contentItem(),"groupGrip:"+color);QVERIFY(grip);
        auto *area=findVisualItem(main->contentItem(),"mainDockWorkspace");QVERIFY(area);const auto visibleBottom=std::min(qreal(workspace.availableScreenGeometry(palette).bottom()+1),area->mapToGlobal({0,0}).y()+area->height());
        const int delta=palette->y()+palette->height()>=visibleBottom-1?-30:30;
        const auto at=grip->mapToScene({10,4}).toPoint();QTest::mousePress(palette,Qt::LeftButton,Qt::NoModifier,at);QTest::mouseMove(palette,at+QPoint(0,delta));QTest::mouseRelease(palette,Qt::LeftButton,Qt::NoModifier,at+QPoint(0,delta));QTRY_VERIFY(palette->y()!=before.y());QCOMPARE(palette->x(),before.x());
        auto *retract=findVisualItem(layer->contentItem(),"flyoutRetract:layers");QVERIFY(retract);QTest::mouseClick(layer,Qt::LeftButton,Qt::NoModifier,retract->mapToScene({retract->width()/2,retract->height()/2}).toPoint());QTRY_VERIFY(layer.isNull());QVERIFY(history && palette);
        QTest::keyClick(history,Qt::Key_Escape);QTRY_VERIFY(history.isNull());QVERIFY(palette);
        auto *icon=findVisualItem(main->contentItem(),"railGrip:history");QVERIFY(icon);const auto iconAt=icon->mapToScene({14,14}).toPoint();QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,iconAt);QTest::mouseMove(main,iconAt+QPoint(-50,12));QTRY_VERIFY(workspace.dragging());
        QQuickWindow *detached=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName().startsWith("floatingDock:"))detached=qobject_cast<QQuickWindow*>(w);return detached!=nullptr;}());QTest::mouseRelease(detached,Qt::LeftButton,Qt::NoModifier,{24,4});QTRY_VERIFY(!workspace.dragging());QVERIFY(palette);
        QString historyGroup;for(const auto &v:workspace.floatingGroups())if(v.toMap().value("panels").toStringList().contains("history"))historyGroup=v.toMap().value("id").toString();QVERIFY(!historyGroup.isEmpty());QVERIFY(!workspace.groupDefinition(historyGroup).value("icons").toBool());
        workspace.returnGroup(historyGroup);QTRY_VERIFY(workspace.floatingWindows().isEmpty());QVERIFY(!workspace.groupDefinition(historyGroup).value("icons").toBool());QVERIFY(palette);
        workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));for(const auto &id:restored.columnGroups(color))QVERIFY(restored.groupDefinition(id).value("icons").toBool());
        const auto preview=qEnvironmentVariable("DRAWVERSE_RAIL_PREVIEW");if(!preview.isEmpty()){QTest::qWait(50);QVERIFY(main->grabWindow().save(preview+".main.png"));QVERIFY(palette->grabWindow().save(preview+".color.png"));}
        QSettings settings(temp.filePath("layout.ini"),QSettings::IniFormat);auto saved=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();auto groups=saved.value("groups").toArray();
        for(int i=0;i<groups.size();++i){auto g=groups[i].toObject();if(g.value("id")==brush){g.insert("collapsed",true);groups[i]=g;}}
        saved.insert("groups",groups);settings.setValue("workspace",QJsonDocument(saved).toJson());settings.sync();WorkspaceManager migrated(temp.filePath("layout.ini"));QVERIFY(migrated.groupDefinition(brush).value("icons").toBool());QVERIFY(!migrated.groupDefinition(brush).value("collapsed").toBool());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void floatingIconRailsKeepBackgroundSidePanelsAndReturnState() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.rightGroups().first().toMap().value("id").toString(),group=workspace.rightGroups().last().toMap().value("id").toString();workspace.detachPanel(group,"history");const auto historyClass=workspace.groupForPanel("history");const auto detachColumn=[&]{workspace.detachGroup(group);QVERIFY(workspace.dockPayload(QString::fromUtf8(QJsonDocument(QJsonObject{{"group",historyClass},{"whole",true}}).toJson()),"floating",group,"after"));};
        workspace.setColumnCollapsed(group,true);detachColumn();
        const auto floatingWindow=[&]()->QQuickWindow*{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+group && w->isVisible())return qobject_cast<QQuickWindow*>(w);return nullptr;};
        QTRY_VERIFY(floatingWindow());QQuickWindow *rail=floatingWindow();const auto available=workspace.availableScreenGeometry(rail);
        rail->setPosition(available.left(),available.top()+20);rail->resize(30,130);QTRY_COMPARE(rail->height(),130);
        const auto click=[&](const QString &panel){auto *icon=findVisualItem(rail->contentItem(),"railPanel:"+panel);if(!icon)return false;QTest::mouseClick(rail,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());return true;};
        const auto flyout=[&](const QString &panel)->QQuickWindow*{for(auto *w:applicationWindows())if(w->objectName().startsWith("panelFlyout:") && w->objectName().endsWith(":"+panel) && w->isVisible())return qobject_cast<QQuickWindow*>(w);return nullptr;};
        QVERIFY(click("layers"));QTRY_VERIFY(flyout("layers"));QPointer<QQuickWindow> layers=flyout("layers");
        QVERIFY(click("history"));QTRY_VERIFY(flyout("history"));QPointer<QQuickWindow> history=flyout("history");QVERIFY(layers);
        QTRY_VERIFY(findVisualItem(layers->contentItem(),"layerList"));QTRY_VERIFY(findVisualItem(history->contentItem(),"historyEntry:0"));
        QVERIFY(layers->height()>rail->height()+80);QVERIFY(layers->y()+layers->height()>rail->y()+rail->height());
        auto *historyIcon=findVisualItem(rail->contentItem(),"railPanel:history");QVERIFY(historyIcon);
        QVERIFY(std::abs(history->y()-historyIcon->mapToGlobal({0,0}).y())<=1);QCOMPARE(history->x(),rail->x()+rail->width()-1);
        auto *resizeFrame=findVisualItem(layers->contentItem(),"panelFlyoutResizeFrame");QVERIFY(resizeFrame && resizeFrame->isVisible());
        layers->resize(260,layers->minimumHeight());QTRY_COMPARE(layers->width(),260);QTRY_COMPARE(layers->height(),layers->minimumHeight());
        layers->resize(300,layers->minimumHeight()+12);QTRY_COMPARE(layers->width(),300);QTRY_COMPARE(layers->height(),layers->minimumHeight()+12);QCOMPARE(rail->height(),130);
        const auto historyY=history->y();rail->setY(rail->y()-8);QTRY_COMPARE(history->y(),historyY-8);
        QTest::mouseClick(rail,Qt::LeftButton,Qt::NoModifier,{14,rail->height()-10});QEvent deactivate(QEvent::WindowDeactivate);QCoreApplication::sendEvent(history,&deactivate);QTest::qWait(30);QVERIFY(layers && history);
        const auto lastSize=layers->size();const auto lastPosition=layers->position();
        QVERIFY(click("layers"));QTRY_VERIFY(layers.isNull());QVERIFY(history);
        QVERIFY(click("layers"));QTRY_VERIFY(flyout("layers"));layers=flyout("layers");QTRY_VERIFY(layers->isActive());QCOMPARE(layers->size(),lastSize);QCOMPARE(layers->position(),lastPosition);QVERIFY(history);
        resizeFrame=findVisualItem(layers->contentItem(),"panelFlyoutResizeFrame");QVERIFY(resizeFrame);
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {
            HWND front=nullptr;for(auto hwnd=GetTopWindow(nullptr);hwnd;hwnd=GetWindow(hwnd,GW_HWNDNEXT))if(hwnd==reinterpret_cast<HWND>(layers->winId()) || hwnd==reinterpret_cast<HWND>(history->winId())){front=hwnd;break;}
            QCOMPARE(front,reinterpret_cast<HWND>(layers->winId()));
            const auto original=layers->size();RECT rect{};QVERIFY(GetWindowRect(reinterpret_cast<HWND>(layers->winId()),&rect));
            QTRY_COMPARE(resizeFrame->width(),qreal(layers->width()));QTRY_COMPARE(resizeFrame->height(),qreal(layers->height()));
            const auto inset=static_cast<LONG>(std::lround(3*layers->devicePixelRatio()));
            const POINT corner{rect.right-inset,rect.bottom-inset};QVERIFY(SetCursorPos(corner.x,corner.y));QTest::qWait(30);
            INPUT down{};down.type=INPUT_MOUSE;down.mi.dwFlags=MOUSEEVENTF_LEFTDOWN;
            const auto release=qScopeGuard([]{INPUT up{};up.type=INPUT_MOUSE;up.mi.dwFlags=MOUSEEVENTF_LEFTUP;SendInput(1,&up,sizeof(INPUT));});
            QCOMPARE(SendInput(1,&down,sizeof(INPUT)),UINT(1));
            // Release from outside the UI thread so a native sizing loop cannot block it.
            std::jthread resize([corner]{Sleep(400);SetCursorPos(corner.x+30,corner.y+20);Sleep(100);INPUT up{};up.type=INPUT_MOUSE;up.mi.dwFlags=MOUSEEVENTF_LEFTUP;SendInput(1,&up,sizeof(INPUT));});
            QTRY_VERIFY(layers->width()>original.width() && layers->height()>original.height());QCOMPARE(rail->height(),130);
        }
#endif
        const auto image=rail->grabWindow(),mainImage=main->grabWindow();const auto dpr=rail->devicePixelRatio();QCOMPARE(image.pixelColor(qRound(14*dpr),qRound((rail->height()-10)*dpr)),QColor("#1c1e21"));
        auto *background=findVisualItem(main->contentItem(),"__railBackground:"+color);QVERIFY(background);const auto blank=background->mapToScene({14,background->height()-12});
        QCOMPARE(mainImage.pixelColor(qRound(blank.x()*main->devicePixelRatio()),qRound(blank.y()*main->devicePixelRatio())),QColor("#1c1e21"));
        const auto preview=qEnvironmentVariable("DRAWVERSE_FLOATING_RAIL_PREVIEW");if(!preview.isEmpty()){QTest::qWait(50);QVERIFY(rail->grabWindow().save(preview+".rail.png"));QVERIFY(layers->grabWindow().save(preview+".layers.png"));QVERIFY(history->grabWindow().save(preview+".history.png"));}
        workspace.setColumnCollapsed(group,false);QTRY_VERIFY(layers.isNull() && history.isNull());workspace.returnWindow(group);QTRY_VERIFY(workspace.floatingWindows().isEmpty());
        QVERIFY(!workspace.groupDefinition(group).value("icons").toBool());QVERIFY(workspace.groupDefinition(color).value("icons").toBool());QCOMPARE(workspace.columnGroups(group),QStringList({group,historyClass}));
        detachColumn();workspace.setColumnCollapsed(group,true);workspace.returnWindow(group);QTRY_VERIFY(workspace.floatingWindows().isEmpty());QVERIFY(workspace.groupDefinition(group).value("icons").toBool());QCOMPARE(workspace.columnGroups(group),QStringList({group,historyClass}));
        for(const bool collapsed:{false,true}) {
            detachColumn();workspace.setColumnCollapsed(group,collapsed);QTRY_VERIFY(floatingWindow());rail=floatingWindow();QTRY_VERIFY(rail->isExposed());QTest::qWait(30);auto *area=findVisualItem(main->contentItem(),"mainDockWorkspace");QVERIFY(area);
            const QPoint at(10,7);QTest::mousePress(rail,Qt::LeftButton,Qt::NoModifier,at);workspace.beginDrag(group,"",true);QTRY_VERIFY(workspace.dragging());
            const auto global=area->mapToGlobal({area->width()+4,100}).toPoint();QTest::mouseMove(rail,rail->mapFromGlobal(global));QTRY_COMPARE(workspace.dragTarget(),QString("__workspace_right"));
            QTest::mouseRelease(rail,Qt::LeftButton,Qt::NoModifier,rail->mapFromGlobal(global));QTRY_VERIFY(!workspace.dragging());QTRY_VERIFY(workspace.floatingWindows().isEmpty());
            QCOMPARE(workspace.groupDefinition(group).value("icons").toBool(),collapsed);QVERIFY(workspace.groupDefinition(color).value("icons").toBool());QCOMPARE(workspace.columnGroups(group),QStringList({group,historyClass}));
        }
        workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));QVERIFY(restored.groupDefinition(group).value("icons").toBool());QCOMPARE(restored.columnGroups(group),QStringList({group,historyClass}));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void columnEdgesAndInnerRowsPreserveCategoryTrees() {
        QTemporaryDir temp;WorkspaceManager workspace(temp.filePath("layout.ini"));
        const auto color=workspace.groupForPanel("color"),layers=workspace.groupForPanel("layers");
        const auto data=[](const QString &id){return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",id},{"whole",true}}).toJson());};
        workspace.detachPanel(layers,"navigator");const auto nav=workspace.groupForPanel("navigator");
        QVERIFY(workspace.dockPayload(data(nav),"right",color,"left"));
        QCOMPARE(workspace.categoryGroups(nav),QStringList({nav,color}));QCOMPARE(workspace.columnGroups(nav),QStringList({nav,color,layers}));
        workspace.detachPanel(layers,"history");const auto history=workspace.groupForPanel("history");QVERIFY(workspace.dockPayload(data(history),"right",color,"merge"));
        QCOMPARE(workspace.categoryDefinition(nav).value("panels").toStringList(),QStringList({"navigator","color","history"}));
        const auto brush=workspace.groupForPanel("brush");workspace.detachGroup(brush);QVERIFY(workspace.dockPayload(data(brush),"right",color,"column-left"));
        QCOMPARE(workspace.columnGroups(brush),QStringList({brush}));QCOMPARE(workspace.columnGroups(nav),QStringList({nav,color,layers}));
        workspace.setColumnCollapsed(color,true);
        const auto items=workspace.layoutItems("main",1600,800);int separators=0,rows=0;
        for(const auto &v:items){const auto item=v.toMap();if(item.value("kind")=="railSeparator"){++separators;QCOMPARE(item.value("rect").toRectF().height(),qreal(8));}
            if(item.value("id")==nav){++rows;QCOMPARE(item.value("panels").toStringList(),QStringList({"navigator","color","history"}));QCOMPARE(item.value("rect").toRectF().width(),qreal(28));}}
        QCOMPARE(separators,1);QCOMPARE(rows,1);
        int expanded=0;for(const auto &v:workspace.layoutItems("__category:"+nav,600,400)){const auto item=v.toMap();if(item.value("kind")=="leaf"){++expanded;QVERIFY(!item.value("rail").toBool());}}
        QCOMPARE(expanded,2);workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));QCOMPARE(restored.columnGroups(nav),workspace.columnGroups(nav));QCOMPARE(restored.categoryGroups(nav),workspace.categoryGroups(nav));QVERIFY(restored.groupDefinition(color).value("icons").toBool());
        QSet<QString> seen,splits;auto bad=DockTree::split(DockTree::leaf(nav),DockTree::leaf(color),"vertical");bad.insert("role","row");QVERIFY(!DockTree::validate(bad,QSet<QString>{nav,color},seen,splits));
        // A legacy horizontal split without the new role stays two independent columns.
        restored.resetLayout();const auto left=restored.groupForPanel("brush"),right=restored.groupForPanel("color");restored.detachGroup(left);QVERIFY(restored.dockPayload(data(left),"right",right,"column-right"));QCOMPARE(restored.categoryGroups(left),QStringList({left}));
    }
    void categoryFlyoutsShareRowsTabsAndRetractWithoutExpanding() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.groupForPanel("color"),layers=workspace.groupForPanel("layers");const auto data=[](const QString &id){return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",id},{"whole",true}}).toJson());};
        workspace.detachPanel(layers,"navigator");const auto nav=workspace.groupForPanel("navigator");QVERIFY(workspace.dockPayload(data(nav),"right",color,"left"));workspace.detachPanel(layers,"history");QVERIFY(workspace.dockPayload(data(workspace.groupForPanel("history")),"right",color,"merge"));QTRY_COMPARE(applicationWindows().size(),1);
        QCOMPARE(workspace.sidePlacement(nav,"left"),QString("column-left"));QCOMPARE(workspace.sidePlacement(nav,"right"),QString("right"));QCOMPARE(workspace.sidePlacement(color,"left"),QString("left"));QCOMPARE(workspace.sidePlacement(color,"right"),QString("column-right"));
        workspace.setColumnCollapsed(nav,true);auto *line=findVisualItem(main->contentItem(),"railSeparatorLine");QVERIFY(line);QCOMPARE(line->width(),qreal(18));QVERIFY(std::abs(line->x()+line->width()/2-line->parentItem()->width()/2)<.01);QCOMPARE(line->property("color").value<QColor>(),QColor("#3b3d42"));
        const auto click=[&](const QString &panel){auto *icon=findVisualItem(main->contentItem(),"railPanel:"+panel);if(!icon)return false;QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());return true;};
        const auto flyout=[&](const QString &panel)->QQuickWindow*{for(auto *w:applicationWindows())if(w->objectName().startsWith("panelFlyout:") && w->objectName().endsWith(":"+panel) && w->isVisible())return qobject_cast<QQuickWindow*>(w);return nullptr;};
        QVERIFY(click("navigator"));QTRY_VERIFY(flyout("navigator"));QPointer<QQuickWindow> row=flyout("navigator");QTRY_VERIFY(findVisualItem(row->contentItem(),"navigatorCanvas"));QTRY_VERIFY(findVisualItem(row->contentItem(),"historyList"));QVERIFY(findVisualItem(row->contentItem(),"panelTab:history"));QVERIFY(findVisualItem(row->contentItem(),"panelTab:color"));
        auto *navTile=findVisualItem(row->contentItem(),"dockTile:"+nav),*colorTile=findVisualItem(row->contentItem(),"dockTile:"+color);QVERIFY(navTile && colorTile);QVERIFY(navTile->x()+navTile->width()<colorTile->x()+.1);QCOMPARE(navTile->y(),colorTile->y());
        auto *historyList=findVisualItem(row->contentItem(),"historyList");QVERIFY(historyList);for(auto *control:historyList->parentItem()->findChildren<QObject*>())QVERIFY(control->property("glyph").toString()!="undo" && control->property("glyph").toString()!="redo");
        QVERIFY(click("color"));QTRY_VERIFY(findVisualItem(row->contentItem(),"colorWheel"));QCOMPARE(row->property("selectedPanel").toString(),QString("color"));QTRY_COMPARE(applicationWindows().size(),2);
        row->resize(row->minimumWidth()+20,row->minimumHeight()+35);QTest::qWait(30);const auto lastSize=row->size();QVERIFY(click("layers"));QTRY_VERIFY(flyout("layers"));QPointer<QQuickWindow> other=flyout("layers");QCOMPARE(applicationWindows().size(),3);
        auto *grip=findVisualItem(row->contentItem(),"groupGrip:"+nav);QVERIFY(grip);QTest::mouseDClick(row,Qt::LeftButton,Qt::NoModifier,grip->mapToScene({10,4}).toPoint());QTRY_VERIFY(row.isNull());QVERIFY(other);for(const auto &id:workspace.columnGroups(nav))QVERIFY(workspace.groupDefinition(id).value("icons").toBool());
        QVERIFY(click("history"));QTRY_VERIFY(flyout("history"));row=flyout("history");QCOMPARE(row->size(),lastSize);QTRY_VERIFY(findVisualItem(row->contentItem(),"navigatorCanvas"));QTRY_VERIFY(findVisualItem(row->contentItem(),"historyList"));QVERIFY(click("history"));QTRY_VERIFY(row.isNull());QVERIFY(other);
        QVERIFY(click("navigator"));QTRY_VERIFY(flyout("navigator"));row=flyout("navigator");const auto preview=qEnvironmentVariable("DRAWVERSE_CATEGORY_PREVIEW");if(!preview.isEmpty()){QTest::qWait(80);QVERIFY(main->grabWindow().save(preview+".main.png"));QVERIFY(row->grabWindow().save(preview+".row.png"));}
        QCOMPARE(QQmlProperty::read(row,"palette.button").value<QColor>(),QColor("#25262b"));QCOMPARE(QQmlProperty::read(row,"palette.buttonText").value<QColor>(),QColor("#bababa"));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void navigatorTracksVisibleDocumentAndSharesContentAcrossWindows() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto group=workspace.groupForPanel("navigator");workspace.setActive(group,"navigator");
        const auto check=[&](QQuickWindow *window,CanvasItem *view) {
            auto *mini=qobject_cast<CanvasItem*>(findVisualItem(window->contentItem(),"navigatorCanvas"));auto *frame=findVisualItem(window->contentItem(),"navigatorViewFrame");auto *preview=findVisualItem(window->contentItem(),"navigatorPreview");if(!mini || !frame || !preview || !frame->isVisible())return false;
            const auto rect=view->visibleDocumentRect();const auto expected=QRectF(mini->documentRect().topLeft()+rect.topLeft()*mini->zoom(),rect.size()*mini->zoom());
            for(auto *object:preview->parentItem()->findChildren<QObject*>())if(object->property("text").toString()==QStringLiteral("适合窗口  F") || object->property("text").toString()==QStringLiteral("实际像素  100%"))return false;
            return QLineF(frame->position(),expected.topLeft()).length()<.01 && std::abs(frame->width()-expected.width())<.01 && std::abs(frame->height()-expected.height())<.01 && QQmlProperty::read(frame,"border.color").value<QColor>()==QColor("#ff3434") && mini->size()==preview->size();
        };
        auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->fitToView();QTRY_VERIFY(check(main,canvas));QCOMPARE(canvas->visibleDocumentRect(),QRectF(0,0,960,640));canvas->zoomBy(3);QTRY_VERIFY(check(main,canvas));QVERIFY(canvas->visibleDocumentRect().width()<960);
        const auto before=canvas->visibleDocumentRect();const auto center=canvas->mapToScene({canvas->width()/2,canvas->height()/2}).toPoint();QTest::mousePress(main,Qt::MiddleButton,Qt::NoModifier,center);QTest::mouseMove(main,center+QPoint(25,15));QTest::mouseRelease(main,Qt::MiddleButton,Qt::NoModifier,center+QPoint(25,15));QTRY_VERIFY(canvas->visibleDocumentRect()!=before);QTRY_VERIFY(check(main,canvas));QCOMPARE(client.undoDepth(),0);
        workspace.setColumnCollapsed(group,true);auto *icon=findVisualItem(main->contentItem(),"railPanel:navigator");QVERIFY(icon);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName().endsWith(":navigator"))floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());QTRY_VERIFY(check(floating,canvas));
        const auto retained=canvas->visibleDocumentRect();auto *documents=main->findChild<DocumentManager*>("documentManager");QVERIFY(documents);const auto first=documents->activeId(),second=documents->newDocument(128,64,true);QVERIFY(!second.isEmpty());QTRY_VERIFY(documents->activeClient()->ready());auto current=[&]{return qobject_cast<CanvasItem*>(main->property("canvas").value<QObject*>());};QTRY_VERIFY(current() && current()!=canvas);QTRY_VERIFY(check(floating,current()));
        documents->floatDocument(second);QTRY_VERIFY(current()->window()!=main);current()->zoomBy(3);QTRY_VERIFY(check(floating,current()));QVERIFY(documents->activate(first));QTRY_VERIFY(check(floating,canvas));QCOMPARE(canvas->visibleDocumentRect(),retained);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void movingWholeCategoryColumnsPreservesRowsAndOuterEdges() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.groupForPanel("color"),layers=workspace.groupForPanel("layers"),brush=workspace.groupForPanel("brush");workspace.detachPanel(layers,"navigator");const auto nav=workspace.groupForPanel("navigator");const auto data=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",nav},{"whole",true}}).toJson());QVERIFY(workspace.dockPayload(data,"right",color,"left"));
        const auto members=workspace.columnGroups(nav),row=workspace.categoryGroups(nav);QCOMPARE(row,QStringList({nav,color}));QCOMPARE(members,QStringList({nav,color,layers}));
        auto *grip=findVisualItem(main->contentItem(),"groupGrip:"+nav);QVERIFY(grip);const auto at=grip->mapToScene({10,4}).toPoint();QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,at);workspace.beginDrag(nav,"",true);QTRY_VERIFY(workspace.dragging());
        QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+nav && w->isVisible())floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());QCOMPARE(workspace.categoryGroups(nav),row);QCOMPARE(workspace.columnGroups(nav),members);
        QTest::keyClick(floating,Qt::Key_Escape);QTRY_VERIFY(!workspace.dragging());QTRY_VERIFY(workspace.floatingWindows().isEmpty());QCOMPARE(workspace.categoryGroups(nav),row);QTest::mouseRelease(main,Qt::LeftButton,Qt::NoModifier,at);
        grip=findVisualItem(main->contentItem(),"groupGrip:"+nav);QVERIFY(grip);QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,grip->mapToScene({10,4}).toPoint());workspace.beginDrag(nav,"",true);QTRY_VERIFY(workspace.dragging());floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+nav && w->isVisible())floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());
        workspace.setColumnCollapsed(brush,true);QTest::qWait(30);
        auto *target=findVisualItem(main->contentItem(),"dockTile:"+brush),*railBackground=findVisualItem(main->contentItem(),"__railBackground:"+brush);QVERIFY(target && railBackground);QVERIFY(railBackground->height()>target->height()+100);
        const auto global=railBackground->mapToGlobal({railBackground->width()-4,railBackground->height()-60}).toPoint();QTest::mouseMove(floating,floating->mapFromGlobal(global));QTRY_COMPARE(workspace.dragTarget(),brush);QCOMPARE(workspace.dragPlacement(),QString("column-right"));
        auto *glow=findVisualItem(main->contentItem(),"dockColumnPreview:main");QVERIFY(glow && glow->isVisible());QVERIFY(glow->height()>target->height()-1);
        QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,floating->mapFromGlobal(global));QTRY_VERIFY(!workspace.dragging());QTRY_VERIFY(workspace.floatingWindows().isEmpty());QCOMPARE(workspace.categoryGroups(nav),row);QCOMPARE(workspace.columnGroups(nav),members);QCOMPARE(workspace.columnGroups(brush),QStringList({brush}));
        workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));QCOMPARE(restored.categoryGroups(nav),row);QCOMPARE(restored.columnGroups(nav),members);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void panelViewsPersistIndependentlyAndIgnoreInvalidOptionalState() {
        QTemporaryDir temp;const auto path=temp.filePath("layout.ini");WorkspaceManager workspace(path);
        const auto group=workspace.rightGroups().last().toMap().value("id").toString();workspace.setColumnCollapsed(group,true);
        workspace.updatePanelView("layers",260,340,48);workspace.updatePanelView("color",230,210,-20);workspace.saveLayout();
        WorkspaceManager restored(path);QCOMPARE(restored.panelView("layers"),workspace.panelView("layers"));QCOMPARE(restored.panelView("color"),workspace.panelView("color"));QVERIFY(restored.groupDefinition(group).value("icons").toBool());
        restored.updatePanelView("layers",0,200,0);restored.updatePanelView("unknown",200,200,0);restored.updatePanelView("color",200,200,std::numeric_limits<int>::min());
        QCOMPARE(restored.panelView("layers"),workspace.panelView("layers"));QVERIFY(restored.panelView("unknown").isEmpty());QCOMPARE(restored.panelView("color"),workspace.panelView("color"));
        QSettings settings(path,QSettings::IniFormat);auto saved=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();auto views=saved.value("panelViews").toObject();
        views.insert("layers",QJsonObject{{"width",0},{"height",200},{"offset",0}});views.insert("color",QJsonObject{{"width",200},{"height",200},{"offset",std::numeric_limits<int>::min()}});saved.insert("panelViews",views);settings.setValue("workspace",QJsonDocument(saved).toJson());settings.sync();
        WorkspaceManager invalid(path);QVERIFY(invalid.panelView("layers").isEmpty());QVERIFY(invalid.panelView("color").isEmpty());QVERIFY(invalid.groupDefinition(group).value("icons").toBool());
        saved.remove("panelViews");settings.setValue("workspace",QJsonDocument(saved).toJson());settings.sync();WorkspaceManager legacy(path);QVERIFY(legacy.panelView("layers").isEmpty());QVERIFY(legacy.groupDefinition(group).value("icons").toBool());
        restored.resetLayout();QVERIFY(restored.panelView("layers").isEmpty());
    }
    void panelPresentationRemembersFlyoutsAndPaintsActualHeaderAlpha() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto brush=workspace.leftGroups().first().toMap().value("id").toString(),color=workspace.rightGroups().first().toMap().value("id").toString();workspace.setActive(brush,"brush");
        auto *nav=findVisualItem(main->contentItem(),"columnHeader:"+brush),*tabs=findVisualItem(main->contentItem(),"panelTabBar:"+brush),*tab=findVisualItem(main->contentItem(),"panelTab:brush");QVERIFY(nav && tabs && tab);
        QCOMPARE(nav->property("color").value<QColor>(),QColor("#05080a"));QCOMPARE(tabs->property("color").value<QColor>(),QColor("#05080a"));QCOMPARE(tab->property("radius").toReal(),qreal(3));
        auto *preset=findVisualItem(main->contentItem(),"brushPreset:round-pressure");QTRY_VERIFY(preset);QCOMPARE(QQmlProperty::read(preset,"border.color").value<QColor>(),QColor("#1b1c1f"));
        workspace.setColumnCollapsed(color,true);const auto click=[&](const QString &panel){auto *icon=findVisualItem(main->contentItem(),"railPanel:"+panel);if(!icon)return false;QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());return true;};
        const auto flyout=[&](const QString &panel)->QQuickWindow*{for(auto *w:applicationWindows())if(w->objectName().endsWith(":"+panel) && w->objectName().startsWith("panelFlyout:") && w->isVisible())return qobject_cast<QQuickWindow*>(w);return nullptr;};
        QVERIFY(click("color"));QTRY_VERIFY(flyout("color"));QPointer<QQuickWindow> panel=flyout("color");panel->resize(250,panel->minimumHeight());QTest::qWait(30);
        auto *grip=findVisualItem(panel->contentItem(),"groupGrip:"+color);QVERIFY(grip);const auto from=grip->mapToScene({10,4}).toPoint();
        const auto initial=panel->position();auto *area=findVisualItem(main->contentItem(),"mainDockWorkspace");QVERIFY(area);const int delta=panel->y()+panel->height()>=std::min(workspace.availableScreenGeometry(panel).bottom()+1,qRound(area->mapToGlobal({0,0}).y()+area->height()))-1?-20:20;
        QTest::mousePress(panel,Qt::LeftButton,Qt::NoModifier,from);QTest::mouseMove(panel,from+QPoint(0,delta));QTest::mouseRelease(panel,Qt::LeftButton,Qt::NoModifier,from+QPoint(0,delta));QTRY_VERIFY(panel->position()!=initial);
        const auto size=panel->size();const auto position=panel->position();const auto view=workspace.panelView("color");QVERIFY(!view.isEmpty());
        QVERIFY(click("history"));QTRY_VERIFY(flyout("history"));QPointer<QQuickWindow> history=flyout("history");
        QVERIFY(click("color"));QTRY_VERIFY(panel.isNull());QVERIFY(history);QCOMPARE(workspace.panelView("color"),view);
        QVERIFY(click("color"));QTRY_VERIFY(flyout("color"));panel=flyout("color");QCOMPARE(panel->size(),size);QCOMPARE(panel->position(),position);QVERIFY(history);
        workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));QCOMPARE(restored.panelView("color"),workspace.panelView("color"));
        const auto checkHeader=[&](QQuickWindow *window,const QString &id) {
            auto *header=findVisualItem(window->contentItem(),"columnHeader:"+id),*bar=findVisualItem(window->contentItem(),"panelTabBar:"+id);if(!header || !bar)return false;
            const auto c=header->property("color").value<QColor>();if(c.red()!=5 || c.green()!=8 || c.blue()!=10 || std::abs(c.alphaF()-.75)>.005)return false;
            const auto image=window->grabWindow();if(image.isNull())return false;const auto at=header->mapToScene({header->width()/2,4});const auto pixel=image.pixelColor(qRound(at.x()*window->devicePixelRatio()),qRound(at.y()*window->devicePixelRatio()));
            const auto body=image.pixelColor(qRound(3*window->devicePixelRatio()),qRound((window->height()-40)*window->devicePixelRatio()));
            // The software renderer returns RGB32 with premultiplied channels; other backends retain alpha.
            const auto expected=image.hasAlphaChannel()?QColor(5,8,10,191):QColor(4,6,7);
            return window->color()==Qt::transparent && std::abs(pixel.alpha()-expected.alpha())<=1 && std::abs(pixel.red()-expected.red())<=1 && std::abs(pixel.green()-expected.green())<=1 && std::abs(pixel.blue()-expected.blue())<=1 && body==QColor("#1c1e21");
        };
        QTRY_VERIFY(checkHeader(panel,color));workspace.detachGroup(brush);QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+brush)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());QTRY_VERIFY(checkHeader(floating,brush));
        const auto preview=qEnvironmentVariable("DRAWVERSE_PANEL_PRESENTATION_PREVIEW");if(!preview.isEmpty()){QTest::qWait(80);QVERIFY(main->grabWindow().save(preview+".main.png"));QVERIFY(panel->grabWindow().save(preview+".color.png"));QVERIFY(floating->grabWindow().save(preview+".brush.png"));}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void compactStatusReflectsDocumentAndZoomControlsPreserveCenter() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);
        const auto label=[&](const QString &name){auto *item=findVisualItem(main->contentItem(),name);return item?item->property("text").toString():QString();};
        QTRY_COMPARE(label("statusTool"),QStringLiteral("工具  画笔工具"));client.setEraser(true);QTRY_COMPARE(label("statusTool"),QStringLiteral("工具  橡皮擦工具"));client.brushLibrary()->select("round-fine");QTRY_COMPARE(label("statusBrush"),QStringLiteral("画笔  细线圆笔"));QVERIFY(client.eraser());
        client.setMoveTool(true);QTRY_COMPARE(label("statusTool"),QStringLiteral("工具  移动工具"));client.setSelectionTool(2);QTRY_COMPARE(label("statusTool"),QStringLiteral("工具  椭圆选框"));client.setSelectionTool(0);client.setMoveTool(false);
        QTRY_COMPARE(label("statusCanvas"),QStringLiteral("画布  960 × 640"));client.addDefaultLayer();QTRY_COMPARE(label("statusLayers"),QStringLiteral("图层  2"));QTRY_VERIFY(!client.layerEditBusy());
        auto *strip=findVisualItem(main->contentItem(),"toolStrip");QVERIFY(strip);for(auto *object:strip->findChildren<QObject*>())QVERIFY(object->property("tooltip").toString()!=QStringLiteral("适合窗口 F"));
        const auto click=[&](const QString &name){auto *item=findVisualItem(main->contentItem(),name);if(!item)return false;QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,item->mapToScene({item->width()/2,item->height()/2}).toPoint());return true;};
        canvas->actualSize();const QPointF center(canvas->width()/2,canvas->height()/2);const auto documentPoint=canvas->documentPoint(center);const int undo=client.undoDepth();
        QVERIFY(click("statusZoomIn"));QTRY_VERIFY(std::abs(canvas->zoom()-1.15)<.0001);QVERIFY(QLineF(documentPoint,canvas->documentPoint(center)).length()<.001);QTRY_COMPARE(label("statusZoom"),QString("115.0%"));
        QVERIFY(click("statusZoomOut"));QTRY_VERIFY(std::abs(canvas->zoom()-1)<.0001);QVERIFY(QLineF(documentPoint,canvas->documentPoint(center)).length()<.001);QCOMPARE(client.undoDepth(),undo);
        canvas->zoomBy(std::numeric_limits<double>::quiet_NaN());canvas->zoomBy(-1);QCOMPARE(canvas->zoom(),qreal(1));canvas->zoomBy(100);QCOMPARE(canvas->zoom(),qreal(8));QVERIFY(click("statusFit"));QTRY_VERIFY(canvas->zoom()<8);QCOMPARE(client.undoDepth(),undo);
        auto *documents=main->findChild<DocumentManager*>("documentManager");QVERIFY(documents);const auto first=documents->activeId();const auto second=documents->newDocument(128,256,true);QVERIFY(!second.isEmpty());QTRY_VERIFY(documents->activeClient()->ready());QTRY_COMPARE(label("statusCanvas"),QStringLiteral("画布  128 × 256"));QTRY_COMPARE(label("statusLayers"),QStringLiteral("图层  1"));
        QVERIFY(documents->activate(first));QTRY_COMPARE(label("statusCanvas"),QStringLiteral("画布  960 × 640"));QTRY_COMPARE(label("statusLayers"),QStringLiteral("图层  2"));QVERIFY(documents->closeDocument(first,true));QVERIFY(documents->closeDocument(second,true));
        QTRY_COMPARE(label("statusCanvas"),QStringLiteral("画布  —"));QTRY_VERIFY(!findVisualItem(main->contentItem(),"statusFit")->isEnabled());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void panelsReturnToCanvasOnlyAndEmptyWorkspaceEdges() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());auto *documents=main->findChild<DocumentManager*>("documentManager");QVERIFY(documents);
        const auto source=workspace.leftGroups().first().toMap().value("id").toString();const auto groups=workspace.leftGroups()+workspace.rightGroups();for(const auto &v:groups)workspace.detachGroup(v.toMap().value("id").toString());workspace.floatToolStrip(main->x()+20,main->y()+90);
        auto *area=findVisualItem(main->contentItem(),"mainDockWorkspace");QVERIFY(area);QTRY_VERIFY(area->width()>900);
        for(const auto &side:QStringList{"left","right"}) {
            if(side=="right") {QVERIFY(documents->closeDocument(documents->activeId(),true));QCOMPARE(documents->documents().size(),0);workspace.detachGroup(source);}
            QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+source)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());
            const auto grip=QPoint(24,4);QTest::mousePress(floating,Qt::LeftButton,Qt::NoModifier,grip);workspace.beginDrag(source,"",true);QTRY_VERIFY(workspace.dragging());
            // Accept the window boundary and a small outside tolerance, not the canvas body.
            const auto global=area->mapToGlobal({side=="left"?-4:area->width()+4,100}).toPoint();QTest::mouseMove(floating,floating->mapFromGlobal(global));QTRY_COMPARE(workspace.dragTarget(),"__workspace_"+side);
            QTest::mouseRelease(floating,Qt::LeftButton,Qt::NoModifier,floating->mapFromGlobal(global));QTRY_COMPARE(workspace.groupDefinition(source).value("host").toString(),QString("main"));
            bool canvasRoom=false;for(const auto &v:workspace.layoutItems("main",main->width(),600))if(v.toMap().value("id")=="__canvas")canvasRoom=v.toMap().value("rect").toRectF().width()>320;QVERIFY(canvasRoom);
        }
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void draggingPanelsSelectsNearestExposedEdgeAndShowsEdgeGlow() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        for(const auto &edge:QStringList{"left","right","before","after"}) {
            workspace.resetLayout();const auto source=workspace.leftGroups().first().toMap().value("id").toString(),target=workspace.rightGroups().first().toMap().value("id").toString();
            workspace.detachGroup(target);workspace.detachGroup(source);QTRY_COMPARE(applicationWindows().size(),3);
            QQuickWindow *receiver=nullptr,*moving=nullptr;for(auto *w:applicationWindows()) {
                if(w->objectName()=="floatingDock:"+source)moving=qobject_cast<QQuickWindow*>(w);
                if(w->objectName()=="floatingDock:"+target)receiver=qobject_cast<QQuickWindow*>(w);
            }
            QVERIFY(receiver && moving);receiver->setPosition(main->position()+QPoint(380,160));receiver->raise();
            auto *tile=findVisualItem(receiver->contentItem(),"dockTile:"+target);QVERIFY(tile);
            const QRectF area(tile->mapToGlobal({0,0}),QSizeF(tile->width(),tile->height()));
            const QPoint grip(24,4);QTest::mousePress(moving,Qt::LeftButton,Qt::NoModifier,grip);workspace.beginDrag(source,"",true);QTRY_VERIFY(workspace.dragging());
            const auto body=area.center().toPoint();QTest::mouseMove(moving,moving->mapFromGlobal(body));QVERIFY(workspace.dragTarget().isEmpty());
            if(edge=="left") {
                const auto offset=body-moving->position();
                const QPoint nearWindowEdge(qRound(area.left())-moving->width()+offset.x(),qRound(area.center().y())+offset.y());
                QTest::mouseMove(moving,moving->mapFromGlobal(nearWindowEdge));
                QVERIFY(std::abs(moving->geometry().right()+1-area.left())<=1);
                QVERIFY(std::abs(nearWindowEdge.x()-area.left())>24);
                QVERIFY(workspace.dragTarget()!=target);
            }
            QPointF at=area.center();if(edge=="left")at.setX(area.left()-4);else if(edge=="right")at.setX(area.right()+4);else if(edge=="before")at.setY(area.top()-4);else at.setY(area.bottom()+4);
            QTest::mouseMove(moving,moving->mapFromGlobal(at.toPoint()));QTRY_COMPARE(workspace.dragTarget(),target);QCOMPARE(workspace.dragPlacement(),edge=="left" || edge=="right"?"column-"+edge:edge);
            auto *glow=findVisualItem(receiver->contentItem(),edge=="left" || edge=="right"?"dockColumnPreview:"+target:"dockPreview:"+target);QVERIFY(glow);QVERIFY(glow->isVisible());QCOMPARE(glow->property("mode").toString(),edge);
            QQuickWindow *outsideHint=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="dockingHintWindow")outsideHint=qobject_cast<QQuickWindow*>(w);return outsideHint && outsideHint->isVisible();}());
            const auto bounds=outsideHint->geometry();if(edge=="left")QVERIFY(bounds.right()+1<=std::ceil(area.left()));else if(edge=="right")QVERIFY(bounds.left()>=std::floor(area.right()));else if(edge=="before")QVERIFY(bounds.bottom()+1<=std::ceil(area.top()));else QVERIFY(bounds.top()>=std::floor(area.bottom()));QVERIFY(outsideHint->flags().testFlag(Qt::WindowTransparentForInput));
            const auto preview=qEnvironmentVariable("DRAWVERSE_EDGE_PREVIEW");if(!preview.isEmpty() && edge=="right"){QTest::qWait(80);QVERIFY(receiver->grabWindow().save(preview));}
            QTest::mouseRelease(moving,Qt::LeftButton,Qt::NoModifier,moving->mapFromGlobal(at.toPoint()));QTRY_VERIFY(!workspace.dragging());QTRY_COMPARE(workspace.groupDefinition(source).value("host").toString(),target);
            QTRY_COMPARE(applicationWindows().size(),2);
        }
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void panelTabsSwapWithinTheirBarBeforeDetaching() {
        QTemporaryDir temp;const auto path=temp.filePath("layout.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());const auto group=workspace.rightGroups().last().toMap().value("id").toString();
        const auto swap=[&](QQuickWindow *window,const QString &first,const QString &second) {
            auto *source=findVisualItem(window->contentItem(),"panelTabGrip:"+first),*target=findVisualItem(window->contentItem(),"panelTabGrip:"+second);if(!source || !target)return false;
            const auto from=source->mapToScene({source->width()/2,source->height()/2}).toPoint(),to=target->mapToScene({target->width()/2,target->height()/2}).toPoint();
            const auto original=workspace.groupDefinition(group).value("panels").toStringList();
            QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,from);QTest::mouseMove(window,to);
            if(workspace.dragging() || workspace.groupDefinition(group).value("panels").toStringList()!=original)return false;
            QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,to);return true;
        };
        QVERIFY(swap(main,"layers","navigator"));QTRY_COMPARE(workspace.groupDefinition(group).value("panels").toStringList(),(QStringList{"navigator","history","layers"}));QCOMPARE(workspace.groupDefinition(group).value("active").toString(),QString("layers"));QVERIFY(workspace.floatingWindows().isEmpty());
        workspace.detachGroup(group);QQuickWindow *floating=nullptr;
        QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+group)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());
        QVERIFY(swap(floating,"history","layers"));QTRY_COMPARE(workspace.groupDefinition(group).value("panels").toStringList(),(QStringList{"navigator","layers","history"}));
        workspace.returnGroup(group);QTRY_VERIFY(workspace.floatingWindows().isEmpty());workspace.setColumnCollapsed(group,true);
        auto *icon=findVisualItem(main->contentItem(),"railPanel:history");QVERIFY(icon);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());
        QQuickWindow *flyout=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="panelFlyout:"+group+":history")flyout=qobject_cast<QQuickWindow*>(w);return flyout!=nullptr;}());
        QPointer<QQuickWindow> retained=flyout;QTRY_VERIFY(findVisualItem(flyout->contentItem(),"historyEntry:0"));QVERIFY(findVisualItem(flyout->contentItem(),"panelTabGrip:layers"));
        QVERIFY(swap(flyout,"layers","navigator"));QTRY_COMPARE(workspace.groupDefinition(group).value("panels").toStringList(),(QStringList{"layers","navigator","history"}));
        QVERIFY(swap(flyout,"navigator","layers"));workspace.setActive(group,"history");
        QCOMPARE(workspace.groupDefinition(group).value("active").toString(),QString("history"));QVERIFY(!workspace.dragging());
        workspace.saveLayout();WorkspaceManager restored(path);QCOMPARE(restored.groupDefinition(group).value("panels"),workspace.groupDefinition(group).value("panels"));
        auto *grip=findVisualItem(flyout->contentItem(),"panelTabGrip:history");QVERIFY(grip);const auto at=grip->mapToScene({grip->width()/2,grip->height()/2}).toPoint();
        QTest::mousePress(flyout,Qt::LeftButton,Qt::NoModifier,at);QTest::mouseMove(flyout,at+QPoint(0,60));QTRY_VERIFY(workspace.dragging());QTRY_VERIFY(retained.isNull());
        QQuickWindow *detached=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName().startsWith("floatingDock:"))detached=qobject_cast<QQuickWindow*>(w);return detached!=nullptr;}());
        QTest::keyClick(detached,Qt::Key_Escape);QTRY_VERIFY(!workspace.dragging());QTRY_COMPARE(workspace.groupDefinition(group).value("panels").toStringList(),(QStringList{"navigator","layers","history"}));QTest::mouseRelease(main,Qt::LeftButton,Qt::NoModifier,{500,200});
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void closingOlderPopupDoesNotUntrackNewPopup() {
        QTemporaryDir temp; WorkspaceManager workspace(temp.filePath("layout.ini"));
        QWindow older,newer; older.setGeometry(10,10,60,60); newer.setGeometry(200,10,60,60);
        older.show(); newer.show(); QTRY_VERIFY(older.isVisible()); QTRY_VERIFY(newer.isVisible());
        workspace.watchMenuWindow(&older,true);
        workspace.watchMenuWindow(&newer,true);
        workspace.watchMenuWindow(&older,false);
        QTest::mouseClick(&older,Qt::LeftButton,Qt::NoModifier,QPoint(5,5));
        QTRY_VERIFY(!newer.isVisible()); QVERIFY(older.isVisible());
        workspace.watchMenuWindow(&newer,false); older.close();
    }
    void menuSurfaceKeepsOnlyTopCornersRoundAndFlatTintAt50Percent() {
        MenuSurface surface; surface.setWidth(60); surface.setHeight(28);
        QColor tint("#202226"); tint.setAlphaF(.5); surface.setTint(tint); surface.setTopCornersOnly(true);
        QImage image(60,28,QImage::Format_ARGB32_Premultiplied); image.fill(Qt::transparent);
        QPainter painter(&image); surface.paint(&painter); painter.end();
        QCOMPARE(image.pixelColor(0,0).alpha(),0); QCOMPARE(image.pixelColor(59,0).alpha(),0);
        QVERIFY(std::abs(image.pixelColor(0,27).alphaF()-.5)<.005);
        QVERIFY(std::abs(image.pixelColor(59,27).alphaF()-.5)<.005);
        QCOMPARE(image.pixelColor(30,2),image.pixelColor(30,25));
        surface.setTopCornersOnly(false); image.fill(Qt::transparent); painter.begin(&image); surface.paint(&painter); painter.end();
        QCOMPARE(image.pixelColor(0,27).alpha(),0);
        surface.setRadius(0); painter.begin(&image); surface.paint(&painter); painter.end();
        QVERIFY(image.pixelColor(0,0).alpha()>0);
        surface.setRadius(10); painter.begin(&image); surface.paint(&painter); painter.end();
        QCOMPARE(image.pixelColor(0,0).alpha(),0); // Repaint clears former square corners.
        surface.setTopRightCornerOnly(true); surface.setTint(QColor("#d8323e"));
        painter.begin(&image); surface.paint(&painter); painter.end();
        QCOMPARE(image.pixelColor(0,0),QColor("#d8323e"));
        QCOMPARE(image.pixelColor(59,0).alpha(),0);
        QCOMPARE(image.pixelColor(0,27),QColor("#d8323e"));
        QCOMPARE(image.pixelColor(59,27),QColor("#d8323e"));
    }
    void menusReopenWithoutGhostsAndFloatBesidePanels() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini")); QQmlApplicationEngine engine;
        QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &e:errors) warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/app/Main.qml")); QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window); QTRY_VERIFY(client.ready());
        // Native mouse moves use the desktop cursor: keep the title buttons on
        // the available screen even when the default window is wider at high DPI.
        if(QGuiApplication::platformName()!=QStringLiteral("offscreen")) {
            const auto available=window->screen()->availableGeometry();
            window->resize(std::min(window->width(),available.width()),std::min(window->height(),available.height()));
            window->setPosition(available.topLeft());
            QTest::qWait(100);
        }
        auto *bar=qobject_cast<MenuSurface*>(findVisualItem(window->contentItem(),"menuBarGlassBackground"));
        QVERIFY(bar); QVERIFY(bar->tint().alpha()>0 && bar->tint().alpha()<255); QCOMPARE(bar->radius(),qreal(10));
        QCOMPARE(bar->tint().name(),QString("#05080a")); QVERIFY(std::abs(bar->tint().alphaF()-.75)<.005); QVERIFY(bar->topCornersOnly());
        const auto barPixel=[&] {
            const auto image=window->grabWindow();
            const auto point=bar->mapToScene({bar->width()/2,bar->height()/2})*window->devicePixelRatio();
            return image.pixelColor(point.toPoint());
        };
        auto *body=findVisualItem(window->contentItem(),"opaqueWorkspaceBackground"); QVERIFY(body);
        QCOMPARE(body->mapToScene({0,0}).y(),bar->height());
        if(window->grabWindow().hasAlphaChannel()) QTRY_COMPARE(barPixel().alpha(),191);
        else QTRY_COMPARE(barPixel(),QColor(4,6,7)); // Software grab is RGB32 on a black clear surface.
        QCOMPARE(window->grabWindow().pixelColor(qRound(window->width()*window->devicePixelRatio()/2),
                 qRound(60*window->devicePixelRatio())).alpha(),255);
        auto *closeButton=findVisualItem(window->contentItem(),"windowClose"); QVERIFY(closeButton);
        auto *closeBackground=qobject_cast<MenuSurface*>(closeButton->property("background").value<QObject*>()); QVERIFY(closeBackground);
        QCOMPARE(closeBackground->radius(),qreal(10)); QVERIFY(closeBackground->topRightCornerOnly());
        QTest::mouseMove(window,QPoint(500,100));
        QTRY_COMPARE(closeBackground->tint(),QColor(Qt::transparent));
        QCOMPARE(closeButton->property("contentItem").value<QObject*>()->property("color").value<QColor>(),QColor(Qt::white));
        const auto closeCenter=closeButton->mapToScene({closeButton->width()/2,closeButton->height()/2}).toPoint();
        window->raise();window->requestActivate();QTRY_VERIFY(window->isActive());
        QTest::mouseMove(window,closeCenter);
        QTRY_VERIFY(closeButton->property("hovered").toBool()); QTRY_COMPARE(closeBackground->tint(),QColor("#f04450"));
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,closeCenter);
        QTRY_COMPARE(closeBackground->tint(),QColor("#b5222c"));
        QTest::mouseMove(window,QPoint(500,50));
        QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,QPoint(500,50));
        QTRY_COMPARE(closeBackground->tint(),QColor(Qt::transparent)); QVERIFY(window->isVisible());
        auto *entry=findVisualItem(window->contentItem(),QStringLiteral("menuEntry:窗口")); QVERIFY(entry);
        auto *focus=qobject_cast<QQuickItem*>(entry->property("background").value<QObject*>()); QVERIFY(focus);
        QCOMPARE(focus->property("radius").toReal(),qreal(0));
        auto *menu=window->findChild<QObject*>("windowMenu"); QVERIFY(menu);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,entry->mapToScene({entry->width()/2,entry->height()/2}).toPoint());
        QTRY_VERIFY(menu->property("opened").toBool());
        QCOMPARE(visibleWindowsCount(),2);
        auto *glass=qobject_cast<MenuSurface*>(menu->property("background").value<QObject*>()); QVERIFY(glass);
        QCOMPARE(glass->radius(),qreal(0)); QCOMPARE(glass->tint().alpha(),255);
        QCOMPARE(glass->tint().name(),QString("#1c1e21")); QVERIFY(!glass->topCornersOnly());
        auto *list=qobject_cast<QQuickItem*>(menu->property("contentItem").value<QObject*>()); QVERIFY(list);
        auto *row=findVisualItem(list,QStringLiteral("glassMenuItem:自定义面板…")); QVERIFY(row);
        auto *label=qobject_cast<QQuickItem*>(row->property("contentItem").value<QObject*>()); QVERIFY(label);
        QCOMPARE(row->property("topPadding").toReal(),qreal(3)); QCOMPARE(row->property("bottomPadding").toReal(),qreal(3));
        QCOMPARE(row->implicitHeight()-label->implicitHeight(),qreal(6));
        QCOMPARE(row->property("background").value<QObject*>()->property("radius").toReal(),qreal(0));
        const auto preview=qEnvironmentVariable("DRAWVERSE_MENU_PREVIEW");
        if(!preview.isEmpty()) { QTest::qWait(100); QVERIFY(window->grabWindow().save(preview)); }
        QTest::keyClick(list->window(),Qt::Key_Escape); QTRY_VERIFY(!menu->property("visible").toBool());
        const auto menuImage=[&] {
            const auto image=list->window()->grabWindow();
            const auto position=glass->mapToScene({0,0}); const auto dpr=list->window()->devicePixelRatio();
            // Exclude the fractional outer pixel coverage at 1.1x scale. All
            // interior text/background pixels must stay identical over changing content.
            const int fringe=qEnvironmentVariable("QT_QUICK_BACKEND")==QStringLiteral("software") && std::fmod(dpr,1)!=0?static_cast<int>(std::ceil(dpr)):1;
            return image.copy(QRect(qRound(position.x()*dpr),qRound(position.y()*dpr),qRound(glass->width()*dpr),qRound(glass->height()*dpr)).adjusted(1,1,-fringe,-fringe));
        };
        QTest::mouseMove(window,{20,window->height()-20});
        QVERIFY(QMetaObject::invokeMethod(menu,"open")); QTRY_VERIFY(menu->property("opened").toBool());
        QTest::qWait(30); const auto firstMenu=menuImage(); QVERIFY(!firstMenu.isNull());
        QCOMPARE(firstMenu.pixelColor(0,0).alpha(),255);
        QCOMPARE(firstMenu.pixelColor(firstMenu.width()-1,firstMenu.height()-1).alpha(),255);
        auto *options=findVisualItem(window->contentItem(),"brushOptionsBar"); QVERIFY(options);
        options->setVisible(false);QTRY_COMPARE(menuImage(),firstMenu);
        options->setVisible(true);QTRY_COMPARE(menuImage(),firstMenu);
        for(int cycle=0;cycle<6;++cycle) {
            QVERIFY(QMetaObject::invokeMethod(menu,"close")); QTRY_VERIFY(!menu->property("visible").toBool());
            QVERIFY(QMetaObject::invokeMethod(menu,"open")); QTRY_VERIFY(menu->property("opened").toBool());
            QTRY_COMPARE(menuImage(),firstMenu); QCOMPARE(bar->radius(),qreal(10));
        }
        QVERIFY(QMetaObject::invokeMethod(menu,"close")); QTRY_VERIFY(!menu->property("visible").toBool());
        const auto normalGeometry=window->geometry();
        window->showMaximized(); QTRY_COMPARE(bar->radius(),qreal(0));
        QCOMPARE(closeBackground->radius(),qreal(0));
        QTRY_COMPARE(window->geometry(),window->screen()->availableGeometry());
        window->showNormal(); QTRY_COMPARE(bar->radius(),qreal(10));
        QCOMPARE(closeBackground->radius(),qreal(10));
        QTRY_COMPARE(window->geometry(),normalGeometry);
        window->raise();window->requestActivate();QTRY_VERIFY(window->isActive());
        auto *windowEntry=findVisualItem(window->contentItem(),QStringLiteral("menuEntry:窗口")); QVERIFY(windowEntry);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,windowEntry->mapToScene({windowEntry->width()/2,windowEntry->height()/2}).toPoint());
        auto *windowMenu=window->findChild<QObject*>("windowMenu"); QVERIFY(windowMenu); QTRY_VERIFY(windowMenu->property("opened").toBool());
        auto *panelItem=windowMenu->findChild<QObject*>("windowPanel:color"); QVERIFY(panelItem); QVERIFY(panelItem->property("checked").toBool());
        const auto menuPreview=qEnvironmentVariable("DRAWVERSE_WINDOW_MENU_PREVIEW");
        if(!menuPreview.isEmpty()) {
            QTest::qWait(120);
            auto *menuGhost=qobject_cast<QQuickItem*>(windowMenu->property("contentItem").value<QObject*>());
            QVERIFY(menuGhost && menuGhost->window());
            QVERIFY(menuGhost->window()->grabWindow().save(menuPreview));
        }
        // The offscreen backend cannot grab the keyboard, so keyboard Escape is unreliable here;
        // close the top-level menu explicitly and verify it really goes away.
        QVERIFY(QMetaObject::invokeMethod(windowMenu,"close"));QTRY_VERIFY(!windowMenu->property("visible").toBool());
        const auto group=workspace.rightGroups().first().toMap().value("id").toString(); workspace.detachPanel(group,"color");
        QTRY_COMPARE(visibleWindowsCount(),2);
        QQuickWindow *floating=nullptr; for(auto *candidate:applicationWindows()) if(candidate->isVisible() && candidate->objectName().startsWith("floatingDock:")) floating=qobject_cast<QQuickWindow*>(candidate);
        QVERIFY(floating); const auto floatingId=workspace.floatingGroups().first().toMap().value("id").toString();
        floating->resize(150,300); floating->setPosition(floating->screen()->availableGeometry().topLeft()+QPoint(10,10));
        auto *button=findVisualItem(floating->contentItem(),"panelMenu:"+floatingId); QVERIFY(button);
        // Synthetic clicks do not activate a native Qt Tool window like real clicks do.
        floating->requestActivate(); QTRY_VERIFY(floating->isActive());
        QTRY_VERIFY(button->mapToScene({button->width()/2,button->height()/2}).x()<floating->width());
        QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,button->mapToScene({button->width()/2,button->height()/2}).toPoint());
        auto *panelMenu=floating->findChild<QObject*>("panelOperationsMenu:"+floatingId); QVERIFY(panelMenu); QTRY_VERIFY(panelMenu->property("opened").toBool());
        QTRY_COMPARE(visibleWindowsCount(),3);
        auto *host=qobject_cast<QQuickWindow*>(panelMenu->property("sideWindow").value<QObject*>()); QVERIFY(host);
        QTRY_VERIFY(host->isVisible());
        QVERIFY(host->x()>=floating->x()+floating->width() || host->x()+host->width()<=floating->x());
        QTRY_VERIFY(host->isActive());
        QTRY_VERIFY(panelMenu->property("activeFocus").toBool());
        auto *floatingGlass=qobject_cast<MenuSurface*>(panelMenu->property("background").value<QObject*>()); QVERIFY(floatingGlass);
        QCOMPARE(floatingGlass->radius(),qreal(0)); QCOMPARE(floatingGlass->tint().alpha(),255);
        QVERIFY(panelMenu->setProperty("currentIndex",-1));
        const auto sideFrame=[](QQuickWindow *popup) {
            auto frame=popup->grabWindow();
            // Qt's software readback at fractional DPR has an unpainted
            // fringe of up to one logical pixel, also with native RGBA windows.
            if(qEnvironmentVariable("QT_QUICK_BACKEND")==QStringLiteral("software") && std::fmod(popup->devicePixelRatio(),1)!=0) {
                const int fringe=static_cast<int>(std::ceil(popup->devicePixelRatio()));
                frame=frame.copy(0,0,frame.width()-fringe,frame.height()-fringe);
            }
            return frame;
        };
        QTest::qWait(100); const auto firstSide=sideFrame(host); QVERIFY(!firstSide.isNull());
        QCOMPARE(firstSide.pixelColor(0,0).alpha(),255);
        QCOMPARE(firstSide.pixelColor(firstSide.width()-1,firstSide.height()-1).alpha(),255);
        if(!preview.isEmpty()) { QTest::qWait(30); QVERIFY(host->grabWindow().save(preview+".side.png")); }
        QTest::keyClick(host,Qt::Key_Down); QTest::keyClick(host,Qt::Key_Escape);
        QTRY_VERIFY(!panelMenu->property("visible").toBool());
        QTRY_COMPARE(visibleWindowsCount(),2);
        for(int cycle=0;cycle<3;++cycle) {
            QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,button->mapToScene({button->width()/2,button->height()/2}).toPoint());
            QTRY_VERIFY(panelMenu->property("opened").toBool());
            host=qobject_cast<QQuickWindow*>(panelMenu->property("sideWindow").value<QObject*>()); QVERIFY(host);
            QVERIFY(panelMenu->setProperty("currentIndex",-1));
            if(!preview.isEmpty()) {
                QVERIFY(firstSide.save(preview+".side-expected.png"));
                QVERIFY(sideFrame(host).save(preview+".side-reopened.png"));
            }
            const bool sameFrame=QTest::qWaitFor([&]{return sideFrame(host)==firstSide;},5000);
            if(!sameFrame) {
                firstSide.save("menu-side-expected.png"); sideFrame(host).save("menu-side-reopened.png");
                qInfo()<<"Side dimensions"<<firstSide.size()<<sideFrame(host).size();
            }
            QVERIFY(sameFrame);
            // Both owner-panel space and the main canvas dismiss the popup.
            auto *outside=cycle%2==0?floating:window;
            const QPoint blank=outside==floating?QPoint(5,250):QPoint(500,500);
            QTest::mouseClick(outside,Qt::LeftButton,Qt::NoModifier,blank);
            QTRY_VERIFY(!panelMenu->property("visible").toBool());
            QTRY_COMPARE(visibleWindowsCount(),2);
            QCOMPARE(client.modified(),false);
        }
        floating->setX(floating->screen()->availableGeometry().right()-floating->width());
        QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,button->mapToScene({button->width()/2,button->height()/2}).toPoint());
        QTRY_VERIFY(panelMenu->property("opened").toBool());
        host=qobject_cast<QQuickWindow*>(panelMenu->property("sideWindow").value<QObject*>()); QVERIFY(host);
        QVERIFY(host->x()+host->width()<=floating->x());
        auto *returnItem=findVisualItem(host->contentItem(),QStringLiteral("glassMenuItem:返回工作区")); QVERIFY(returnItem);
        QTest::mouseClick(host,Qt::LeftButton,Qt::NoModifier,returnItem->mapToScene({returnItem->width()/2,returnItem->height()/2}).toPoint());
        QTRY_COMPARE(workspace.floatingGroups().size(),0); QTRY_COMPARE(visibleWindowsCount(),1);
        QCOMPARE(warnings,QStringList()); QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void nativeMenuBarBlursLiveBackdropWithoutMenuSnapshots() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini")); QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/app/Main.qml")); QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window); QTRY_VERIFY(client.ready());
        if(QGuiApplication::platformName()!=QStringLiteral("windows")) {
            QVERIFY(!window->property("menuBarBlurActive").toBool());
            QVERIFY(!workspace.setMenuBarBlur(window,true));
        } else {
            QVERIFY(window->property("menuBarBlurActive").toBool());
            const auto available=window->screen()->availableGeometry();
            window->resize(std::min(window->width(),available.width()),std::min(window->height(),available.height()));
            window->setPosition(available.topLeft());QTest::qWait(100);
#ifdef Q_OS_WIN
            const auto hasRoundedNativeCorners=[&] {
                const auto handle=reinterpret_cast<HWND>(window->winId());
                RECT bounds{}; if(!GetWindowRect(handle,&bounds)) return false;
                const auto region=CreateRectRgn(0,0,0,0);
                const int radius=qRound(10*window->devicePixelRatio());
                const bool rounded=GetWindowRgn(handle,region)==COMPLEXREGION
                    && !PtInRegion(region,0,0) && !PtInRegion(region,bounds.right-bounds.left-1,0)
                    && !PtInRegion(region,radius/2,0) && PtInRegion(region,radius,0)
                    && PtInRegion(region,(bounds.right-bounds.left)/2,0)
                    && PtInRegion(region,(bounds.right-bounds.left)/2,bounds.bottom-bounds.top-1);
                DeleteObject(region); return rounded;
            };
            QTRY_VERIFY(hasRoundedNativeCorners());
            const auto originalGeometry=window->geometry();
            window->resize(1000,650); QTRY_VERIFY(hasRoundedNativeCorners());
            QTRY_COMPARE(window->property("normalGeometry").toRect(),window->geometry());
            window->showMaximized(); QTRY_COMPARE(window->geometry(),window->screen()->availableGeometry());
            QTRY_VERIFY(!hasRoundedNativeCorners());
            window->showNormal(); QTRY_VERIFY(hasRoundedNativeCorners());
            window->setGeometry(originalGeometry); QTRY_VERIFY(hasRoundedNativeCorners());
            QTRY_COMPARE(window->property("normalGeometry").toRect(),originalGeometry);
#endif
            QQuickWindow backdrop;
            backdrop.setFlags(Qt::Window|Qt::FramelessWindowHint); backdrop.setColor(Qt::black);
            backdrop.setGeometry(window->geometry());
            BlurTestBackdrop stripes(backdrop.contentItem()); stripes.setSize(backdrop.size());
            backdrop.show(); backdrop.raise(); window->raise(); window->requestActivate(); QTRY_VERIFY(window->isActive());
#ifdef Q_OS_WIN
            // Desktop capture needs both test windows above other applications,
            // independently of Windows' foreground activation restrictions.
            SetWindowPos(reinterpret_cast<HWND>(backdrop.winId()),HWND_TOPMOST,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE);
            SetWindowPos(reinterpret_cast<HWND>(window->winId()),HWND_TOPMOST,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE);
#endif
            const auto capture=[&] {
                QTest::qWait(300);
                return window->screen()->grabWindow(0,window->x()+500,window->y()+8,320,12).toImage();
            };
            const auto sharpestEdge=[](const QImage &image) {
                int edge=0; const int row=image.height()/2;
                for(int x=1;x<image.width();++x) {
                    const auto a=image.pixelColor(x-1,row),b=image.pixelColor(x,row);
                    edge=std::max(edge,std::abs(a.red()-b.red())+std::abs(a.green()-b.green())+std::abs(a.blue()-b.blue()));
                }
                return edge;
            };
            QVERIFY(workspace.setMenuBarBlur(window,false)); const auto sharp=capture(); QVERIFY(!sharp.isNull());
            const auto sharpTitle=window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).toImage();
            QVERIFY(!sharpTitle.isNull());
            QVERIFY(workspace.setMenuBarBlur(window,true)); const auto blurred=capture(); QVERIFY(!blurred.isNull());
            const auto preview=qEnvironmentVariable("DRAWVERSE_MENU_PREVIEW");
            if(!preview.isEmpty()) {
                QVERIFY(sharp.save(preview+".sharp.png")); QVERIFY(blurred.save(preview+".blur.png"));
                QVERIFY(window->grabWindow().save(preview+".render.png"));
                QVERIFY(window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).save(preview+".title.png"));
            }
            const auto evidence=QString("Sharp edge=%1, blurred edge=%2").arg(sharpestEdge(sharp)).arg(sharpestEdge(blurred));
            QVERIFY2(sharpestEdge(sharp)>100,qPrintable(evidence));
            QVERIFY2(sharpestEdge(blurred)<sharpestEdge(sharp)/2,qPrintable(evidence));
#ifdef Q_OS_WIN
            // Verify actual compositor pixels after restore, not just the QML
            // surface: a leaked Windows iconic caption or square blur patch
            // can be invisible in QQuickWindow::grabWindow().
            const auto originalTitle=window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).toImage();
            if(!preview.isEmpty())QVERIFY(originalTitle.save(preview+".original.png"));
            const QRect captionArea(qRound(12*originalTitle.devicePixelRatio()),qRound(3*originalTitle.devicePixelRatio()),qRound(230*originalTitle.devicePixelRatio()),qRound(22*originalTitle.devicePixelRatio()));
            for(bool maximized:{false,true})for(int cycle=0;cycle<3;++cycle) {
                if(maximized)window->showMaximized();else window->showNormal();
                QTest::qWait(300);
                const auto before=window->geometry();
                const auto beforeTitle=window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).toImage();
                const auto expectedVisibility=maximized?QWindow::Maximized:QWindow::Windowed;
                SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_MINIMIZE,0);
                QTRY_COMPARE(window->visibility(),QWindow::Minimized);
                SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_RESTORE,0);
                QTRY_COMPARE(window->visibility(),expectedVisibility);QTRY_COMPARE(window->geometry(),before);
                QTRY_COMPARE(hasRoundedNativeCorners(),!maximized);window->raise();window->requestActivate();
                const auto restored=capture();QVERIFY(sharpestEdge(restored)<sharpestEdge(sharp)/2);
                const auto restoredTitle=window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).toImage();
                const auto corner=restoredTitle.pixelColor(0,0);if(!maximized)QVERIFY(corner.green()>190 && corner.red()<40);
                // The native caption occupies the left side. Its restored
                // pixels must match the original custom title/menu rendering.
                if(!preview.isEmpty())QVERIFY(restoredTitle.save(preview+".restored.png"));
                QVERIFY(restoredTitle.rect().contains(captionArea));
                for(int y=captionArea.top();y<=captionArea.bottom();++y)for(int x=captionArea.left();x<=captionArea.right();++x) {
                    const auto a=beforeTitle.pixelColor(x,y),b=restoredTitle.pixelColor(x,y);
                    // DWM may round a text blend by one channel level on restore.
                    // A system caption changes hundreds of pixels by much more.
                    QVERIFY2(std::abs(a.red()-b.red())<=2 && std::abs(a.green()-b.green())<=2 && std::abs(a.blue()-b.blue())<=2,qPrintable(QString("Restored caption pixel differs at %1,%2: %3 -> %4").arg(x).arg(y).arg(a.name()).arg(b.name())));
                }
            }
            window->showNormal();QTRY_COMPARE(window->geometry(),originalGeometry);QTRY_VERIFY(hasRoundedNativeCorners());QTest::qWait(300);
#endif
            auto *close=findVisualItem(window->contentItem(),"windowClose");QVERIFY(close);
            const auto closeCenter=close->mapToScene({close->width()/2,close->height()/2}).toPoint();
            QCursor::setPos(window->mapToGlobal(closeCenter));
            QTest::mouseMove(window,closeCenter);QTRY_VERIFY(close->property("hovered").toBool());
            QTest::qWait(100); // Let the compositor present the new hover color.
            const auto title=window->screen()->grabWindow(0,window->x(),window->y(),window->width(),28).toImage();
            QVERIFY(!title.isNull());
            // The excluded corner keeps the backdrop (including the system shadow).
            const auto corner=title.pixelColor(0,0),sourceCorner=sharpTitle.pixelColor(0,0);
            QVERIFY(std::abs(corner.hueF()-sourceCorner.hueF())<.005);
            QVERIFY(corner.valueF()>=sourceCorner.valueF()*.8 && corner.valueF()<=sourceCorner.valueF());
            QVERIFY(corner.green()>190 && corner.red()<40);
            const auto dpr=title.devicePixelRatio();
            const auto red=title.pixelColor(title.width()-qRound(5*dpr),qRound(20*dpr));
            if(!preview.isEmpty()) QVERIFY(title.save(preview+".title.png"));
            QVERIFY2(red.red()>180 && red.green()<90 && red.blue()<100,qPrintable(QString("Hover=%1, color=%2").arg(close->property("hovered").toBool()).arg(red.name())));
            // A pure-color corner cannot reveal a rectangular blur beneath a
            // transparent QML corner. Sharp checker edges outside the native
            // backdrop region must survive every state transition.
            stripes.showCornerPattern();window->hide();QTest::qWait(200);
            const auto cornerCapture=[&]{return window->screen()->grabWindow(0,window->x(),window->y(),5,2).toImage();};
            const auto sharpCorner=cornerCapture();QVERIFY(sharpestEdge(sharpCorner)>250);
            if(!preview.isEmpty())QVERIFY(sharpCorner.save(preview+".corner-source.png"));
            window->showNormal();QTest::qWait(200);
#ifdef Q_OS_WIN
            for(bool maximized:{false,true})for(int cycle=0;cycle<3;++cycle) {
                if(maximized)window->showMaximized();
                SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_MINIMIZE,0);
                QVERIFY(!IsWindowVisible(reinterpret_cast<HWND>(workspace.menuBlurWindowHandle())));
                SendMessageW(reinterpret_cast<HWND>(window->winId()),WM_SYSCOMMAND,SC_RESTORE,0);
                QTRY_COMPARE(window->visibility(),maximized?QWindow::Maximized:QWindow::Windowed);
                window->showNormal();QTRY_VERIFY(hasRoundedNativeCorners());QTest::qWait(200);
                const auto cornerImage=cornerCapture();
                if(!preview.isEmpty())QVERIFY(cornerImage.save(preview+".sharp-corner.png"));
                QVERIFY2(sharpestEdge(cornerImage)>sharpestEdge(sharpCorner)*.6,qPrintable(QString("Excluded-corner edge %1, source %2; maximized=%3, cycle=%4, geometry=%5,%6").arg(sharpestEdge(cornerImage)).arg(sharpestEdge(sharpCorner)).arg(maximized).arg(cycle).arg(window->x()).arg(window->y())));
            }
#endif
        }
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void uiScaleAppliesToMainPanelAndToolWindowsWithoutChangingLayout() {
        QTemporaryDir temp;
        PaintCoreClient client(nullptr,temp.filePath("storage.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini"));
        QQmlApplicationEngine engine;
        QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors) {
            for(const auto &error:errors) warnings.append(error.toString());
        });
        engine.rootContext()->setContextProperty("PaintClient",&client);
        engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/app/Main.qml"));
        QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());
        QVERIFY(window); QTRY_VERIFY(client.ready());
        auto *options=findVisualItem(window->contentItem(),"brushOptionsBar");
        QVERIFY(options); QCOMPARE(options->height(),qreal(36));
        const auto group=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachPanel(group,"color");
        workspace.floatToolStrip(window->x()+60,window->y()+80);
        QTRY_COMPARE(applicationWindows().size(),3);
        const qreal expected=qEnvironmentVariable("QT_SCALE_FACTOR").toDouble();
        QVERIFY(expected>=1.1);
        for(auto *nativeWindow:applicationWindows()) {
            auto *quick=qobject_cast<QQuickWindow*>(nativeWindow); QVERIFY(quick);
            if(qEnvironmentVariable("QT_QPA_PLATFORM")=="offscreen")
                QVERIFY(std::abs(quick->devicePixelRatio()-expected)<.000001);
            QCOMPARE(quick->devicePixelRatio(),window->devicePixelRatio());
            QTRY_VERIFY(!quick->grabWindow().isNull());
            const auto image=quick->grabWindow();
            QVERIFY(std::abs(image.width()-quick->width()*quick->devicePixelRatio())<=1);
            QVERIFY(std::abs(image.height()-quick->height()*quick->devicePixelRatio())<=1);
        }
        QCOMPARE(workspace.floatingGroups().first().toMap().value("panels").toStringList(),QStringList{"color"});
        workspace.saveLayout();
        WorkspaceManager restored(temp.filePath("layout.ini"));
        QCOMPARE(restored.floatingGroups(),workspace.floatingGroups());
        QVERIFY(restored.toolsFloating());
        QCOMPARE(warnings,QStringList());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);
        client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void selectionOutlineCombinesInDocumentCoordinatesWithoutRedrawOnIdenticalState() {
        SelectionOverlay item;item.setDocumentRect({20,30,200,200});item.setZoom(2);
        const auto step=[](int op,int shape,QRectF r){return QVariantMap{{"operation",op},{"shape",shape},{"x",r.x()},{"y",r.y()},{"width",r.width()},{"height",r.height()}};};
        const QVariantList steps{step(0,0,{10,10,50,50}),step(2,1,{20,20,20,20})};
        item.setSteps(steps);item.setEnabledSelection(true);QVERIFY(item.documentPath().contains({15,15}));QVERIFY(!item.documentPath().contains({30,30}));QVERIFY(!item.documentPath().contains({90,90}));
        QSignalSpy changed(&item,&SelectionOverlay::geometryChanged);item.setSteps(steps);item.setDocumentRect({20,30,200,200});item.setZoom(2);QCOMPARE(changed.size(),0);
        item.setSteps(steps+QVariantList{step(4,0,{})});QVERIFY(!item.documentPath().contains({15,15}));QVERIFY(item.documentPath().contains({30,30}));QVERIFY(item.documentPath().contains({90,90}));
        item.setDocumentRect({40,50,200,200});QVERIFY(item.documentPath().contains({90,90}));
        QImage image(260,260,QImage::Format_ARGB32_Premultiplied);image.fill(Qt::transparent);QPainter painter(&image);item.paint(&painter);painter.end();QVERIFY(image.pixelColor(40,100).alpha()>0);QCOMPARE(image.pixelColor(0,0).alpha(),0);
    }
    void shiftDragConstrainsSelectionToSquareAndCircle() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());
        auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();main->requestActivate();
        const auto at=[&](QPointF point){return canvas->mapToScene(canvas->documentRect().topLeft()+point*canvas->zoom()).toPoint();};
        // Unconstrained drag keeps the dragged proportions.
        client.setSelectionTool(1);QTRY_COMPARE(client.selectionTool(),1);
        QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,at({10,10}));QTest::mouseMove(main,at({70,40}));QTest::mouseRelease(main,Qt::LeftButton,Qt::NoModifier,at({70,40}));
        QTRY_COMPARE(client.selectionSteps().size(),1);
        auto step=client.selectionSteps().first().toMap();
        QCOMPARE(step.value("width").toDouble(),60.);QCOMPARE(step.value("height").toDouble(),30.);
        // Shift while dragging constrains to a square that keeps the anchor corner.
        const auto square=CanvasItem::constrainedRect({20,20},{80,50});
        QCOMPARE(square,QRectF(20,20,60,60));
        // Dragging up-left keeps the pressed corner instead of flipping the box.
        QCOMPARE(CanvasItem::constrainedRect({80,50},{20,20}),QRectF(20,-10,60,60));
        QCOMPARE(CanvasItem::constrainedRect({10,10},{40,90}),QRectF(10,10,80,80));
        QCOMPARE(CanvasItem::constrainedRect({-30,-30},{10,-60}),QRectF(-30,-70,40,40));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void layerSearchRefocusAfterToolsAndCanvasBlank() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("search.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);window->requestActivate();QTRY_VERIFY(window->isActive());QVERIFY(QTest::qWaitForWindowExposed(window));QTRY_VERIFY(client.ready());
        QQuickItem *search=nullptr;QTRY_VERIFY((search=findVisualItem(window->contentItem(),"layerSearch")));QTRY_VERIFY(search->width()>40);QTest::qWait(50);
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);auto *tool=findVisualItem(window->contentItem(),"eraserTool");QVERIFY(tool);
        const auto click=[&](QQuickItem *i){QTest::mouseClick(i->window(),Qt::LeftButton,Qt::NoModifier,i->mapToScene({i->width()/2,i->height()/2}).toPoint());};
        click(search);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(window,Qt::Key_A);QCOMPARE(search->property("text").toString(),QStringLiteral("a"));
        for(int i=0;i<3;++i){
            click(tool);QTRY_VERIFY(canvas->hasActiveFocus());QVERIFY(client.eraser());click(search);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(window,Qt::Key_B);QCOMPARE(search->property("text").toString(),QStringLiteral("a")+QString(2*i+1,QLatin1Char('b')));QVERIFY(client.eraser());
            QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,canvas->mapToScene({2,2}).toPoint());QTRY_VERIFY(canvas->hasActiveFocus());click(search);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(window,Qt::Key_B);QCOMPARE(search->property("text").toString(),QStringLiteral("a")+QString(2*i+2,QLatin1Char('b')));QVERIFY(client.eraser());
        }
        QCOMPARE(findVisualItem(window->contentItem(),"layerSearch"),search);
        // Icon-expanded panels are separate native windows: send keys to the actual focus window.
        workspace.setColumnCollapsed(workspace.groupForPanel("layers"),true);
        QQuickItem *rail=nullptr;QTRY_VERIFY((rail=findVisualItem(window->contentItem(),"railPanel:layers")));click(rail);
        QQuickWindow *flyout=nullptr;const auto findFlyout=[&](){for(auto *w:applicationWindows())if(w->objectName().startsWith("panelFlyout:") && w->objectName().endsWith(":layers") && w->isVisible())return qobject_cast<QQuickWindow*>(w);return static_cast<QQuickWindow*>(nullptr);};QTRY_VERIFY((flyout=findFlyout()));
        QTRY_VERIFY((search=findVisualItem(flyout->contentItem(),"layerSearch")));QTRY_VERIFY(search->width()>40);QVERIFY(QTest::qWaitForWindowExposed(flyout));QTest::qWait(50);click(search);QTRY_COMPARE(QGuiApplication::focusWindow(),flyout);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(QGuiApplication::focusWindow(),Qt::Key_A);QCOMPARE(search->property("text").toString(),QStringLiteral("a"));
        for(int i=0;i<3;++i){
            click(tool);QTRY_COMPARE(QGuiApplication::focusWindow(),window);QTRY_VERIFY(canvas->hasActiveFocus());click(search);QTRY_COMPARE(QGuiApplication::focusWindow(),flyout);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(QGuiApplication::focusWindow(),Qt::Key_B);QCOMPARE(search->property("text").toString(),QStringLiteral("a")+QString(2*i+1,QLatin1Char('b')));QVERIFY(client.eraser());
            window->requestActivate();QTRY_VERIFY(window->isActive());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,canvas->mapToScene({2,2}).toPoint());QTRY_VERIFY(canvas->hasActiveFocus());click(search);QTRY_COMPARE(QGuiApplication::focusWindow(),flyout);QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(QGuiApplication::focusWindow(),Qt::Key_B);QCOMPARE(search->property("text").toString(),QStringLiteral("a")+QString(2*i+2,QLatin1Char('b')));QVERIFY(client.eraser());QVERIFY(flyout->isVisible());
        }
        QCOMPARE(findVisualItem(flyout->contentItem(),"layerSearch"),search);QCOMPARE(warnings,QStringList());auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void firstLayerSearchClickAndCtrlCanvasPickMove() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("pick.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);window->requestActivate();QTRY_VERIFY(window->isActive());QTRY_VERIFY(client.ready());
        QQuickItem *search=nullptr;QTRY_VERIFY((search=findVisualItem(window->contentItem(),"layerSearch")));QTRY_VERIFY(search->width()>40 && search->height()>0);QVERIFY(QTest::qWaitForWindowExposed(window));QTest::qWait(50);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,search->mapToScene({search->width()/2,search->height()/2}).toPoint());QTRY_VERIFY(search->hasActiveFocus());QTest::keyClick(window,Qt::Key_A);QCOMPARE(search->property("text").toString(),QStringLiteral("a"));QTest::keyClick(window,Qt::Key_Escape);QCOMPARE(search->property("text").toString(),QString());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());const auto bottom=client.activeLayer();client.setBrushRadius(4);QVERIFY(client.beginStroke({{16,16},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());client.addLayer("top");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto top=client.activeLayer();QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto screen=[&](QPointF pt){return canvas->mapToScene(canvas->documentRect().topLeft()+pt*canvas->zoom()).toPoint();};
        QTRY_COMPARE(client.undoDepth(),3);const auto depth=client.undoDepth();client.activateEraser();QTest::mouseClick(window,Qt::LeftButton,Qt::ControlModifier,screen({16,16}));QTRY_COMPARE(client.activeLayer(),bottom);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth);QVERIFY(client.eraser());QVERIFY(!client.moveTool());
        QTest::mousePress(window,Qt::LeftButton,Qt::ControlModifier,screen({32,32}));QTest::mouseMove(window,screen({38,35}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,screen({38,35}));QTRY_COMPARE(client.activeLayer(),top);QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.eraser());
        const auto offset=[&](quint64 id){for(const auto &v:client.layers()){const auto l=v.toMap();if(l.value("id").toULongLong()==id)return QPoint(l.value("offsetX").toInt(),l.value("offsetY").toInt());}return QPoint(-999,-999);};
        QCOMPARE(offset(top),QPoint(6,3));QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier);QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(offset(top),QPoint(0,0));QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(offset(top),QPoint(6,3));QTRY_VERIFY(!client.layerEditBusy());
        QTest::mousePress(window,Qt::LeftButton,Qt::ControlModifier,screen({60,60}));QTest::mouseRelease(window,Qt::LeftButton,Qt::ControlModifier,screen({55,55}));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);QCOMPARE(offset(top),QPoint(6,3));
        client.activateMarquee();QTest::mouseClick(window,Qt::LeftButton,Qt::ControlModifier,screen({16,16}));QTRY_COMPARE(client.activeLayer(),bottom);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionTool(),1);QVERIFY(!client.selectionEnabled());
        QCOMPARE(warnings,QStringList());auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void placedImageFreeTransformRasterizeAndShortcuts() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("transform.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QStringList warnings;QQmlApplicationEngine engine;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());QImage image(16,16,QImage::Format_RGBA8888);image.fill(Qt::red);image.setPixelColor(0,0,Qt::blue);const auto path=temp.filePath("placed.png");QVERIFY(image.save(path));QVERIFY(client.placeImage(QUrl::fromLocalFile(path)));QTRY_VERIFY(!client.fileBusy() && client.transformActive());QTRY_VERIFY(!client.transformBusy());QVERIFY(client.activeImage());QCOMPARE(client.undoDepth(),0);auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();window->requestActivate();QTRY_VERIFY(window->isActive());canvas->forceActiveFocus();QTRY_VERIFY(client.frameRevision()>=client.revision());
        auto *bar=findVisualItem(window->contentItem(),"transformOptionsBar");QVERIFY(bar);QVERIFY(bar->isVisible());auto *overlay=findVisualItem(window->contentItem(),"transformOverlay");QVERIFY(overlay && overlay->isVisible());const auto p=client.transformPoints();QCOMPARE(p.size(),16);const auto screen=[&](QPointF pt){return canvas->mapToScene(canvas->documentRect().topLeft()+pt*canvas->zoom()).toPoint();};
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,screen(p[15].toPointF()));QTest::mouseMove(window,screen(p[15].toPointF()+QPointF(8,8)));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,screen(p[15].toPointF()+QPointF(8,8)));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.transformBusy());QVERIFY(client.transformPoints()!=p);canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.transformBusy());QCOMPARE(client.transformPoints(),p);QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.transformBusy());
        auto *warp=findVisualItem(window->contentItem(),"transformWarp");QVERIFY(warp);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,warp->mapToScene({12,12}).toPoint());QTRY_VERIFY(client.transformWarp());QTRY_VERIFY(!client.transformBusy());
        const auto beforeGrid=client.transformPoints();QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,screen(beforeGrid[5].toPointF()));QTest::mouseMove(window,screen(beforeGrid[5].toPointF()+QPointF(3,-2)));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,screen(beforeGrid[5].toPointF()+QPointF(3,-2)));QTRY_VERIFY(!client.transformBusy());QTRY_VERIFY(client.transformPoints()!=beforeGrid);client.undo();QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformPoints(),beforeGrid);
        if(const auto preview=qEnvironmentVariable("DRAWVERSE_PASS_PREVIEW");!preview.isEmpty()){canvas->zoomBy(8);QTest::qWait(120);QVERIFY(window->grabWindow().save(preview));canvas->actualSize();}
        canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Right,Qt::ShiftModifier);QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformPoints()[0].toPointF().x(),beforeGrid[0].toPointF().x()+10.);client.undo();QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformPoints(),beforeGrid);
        canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Return);QTRY_VERIFY(!client.transformActive());QTRY_COMPARE(client.undoDepth(),1);QVERIFY(client.activeImage());QVERIFY(client.modified());const auto placed=client.activeLayer();QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath("placed.ora"))));QTRY_VERIFY(!client.fileBusy());
        canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_T,Qt::ControlModifier);QTRY_VERIFY(client.transformActive() && !client.transformBusy());QTest::keyClick(window,Qt::Key_Escape);QTRY_VERIFY(!client.transformActive());QCOMPARE(client.undoDepth(),1);
        QVERIFY(client.beginTransform());QTRY_VERIFY(client.transformActive() && !client.transformBusy());const auto beforeNumeric=client.transformPoints();auto *numeric=findVisualItem(window->contentItem(),"transformX");QVERIFY(numeric);QCOMPARE(numeric->height(),qreal(20));QCOMPARE(numeric->width(),qreal(52));QCOMPARE(numeric->property("font").value<QFont>().pixelSize(),9);numeric->setProperty("text",QString::number((beforeNumeric[0].toPointF().x()+beforeNumeric[15].toPointF().x())*.5+2.));QVERIFY(QMetaObject::invokeMethod(numeric,"editingFinished"));QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformPoints()[0].toPointF().x(),beforeNumeric[0].toPointF().x()+2.);auto *skew=findVisualItem(window->contentItem(),"transformSH");QVERIFY(skew);skew->setProperty("text","10");QVERIFY(QMetaObject::invokeMethod(skew,"editingFinished"));QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformSkewH(),10.);client.undo();QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformSkewH(),0.);client.redo();QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformSkewH(),10.);client.undo();QTRY_VERIFY(!client.transformBusy());QTRY_COMPARE(client.transformSkewH(),0.);
        client.activateMoveTool();QTRY_VERIFY(!client.transformActive());QTRY_VERIFY(client.moveTool());QTRY_COMPARE(client.undoDepth(),2);client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(client.undoDepth(),1);
        QVERIFY(client.rasterizeImage());QTRY_VERIFY(!client.transformBusy());QTRY_VERIFY(!client.activeImage());QTRY_COMPARE(client.undoDepth(),2);client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.activeImage());QCOMPARE(client.activeLayer(),placed);
        QVERIFY(client.openDocument(QUrl::fromLocalFile(temp.filePath("placed.ora"))));QTRY_VERIFY(!client.fileBusy() && client.ready());QVERIFY(client.activeImage());QCOMPARE(client.undoDepth(),0);documents->floatDocument(documents->activeId());QTRY_VERIFY(canvas->window()!=window);auto *floating=canvas->window();floating->requestActivate();QTest::qWait(80);canvas->forceActiveFocus();QTest::keyClick(floating,Qt::Key_T,Qt::ControlModifier);QTRY_VERIFY(client.transformActive() && !client.transformBusy());QVERIFY(findVisualItem(floating->contentItem(),"transformOptionsBar"));QTest::keyClick(floating,Qt::Key_Escape);QTRY_VERIFY(!client.transformActive());QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void liquifyCanvasTransactionOptionsAndDocumentLock() {
        QTemporaryDir temp;const auto settings=temp.filePath("liquify.ini");PaintCoreClient client(nullptr,settings);WorkspaceManager workspace(temp.filePath("layout.ini"));QStringList warnings;QQmlApplicationEngine engine;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *documents=window->findChild<DocumentManager*>("documentManager");QVERIFY(documents);const auto first=documents->activeId();const auto other=documents->newDocument(32,32,true);QVERIFY(!other.isEmpty());QTRY_VERIFY(documents->client(other)->ready());documents->floatDocument(other);QVERIFY(documents->activate(first));
        QQuickWindow *foreign=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName().startsWith("documentWindow:"))foreign=qobject_cast<QQuickWindow*>(w);return foreign!=nullptr;}());QVERIFY(foreign->isVisible());
        workspace.floatToolStrip(window->x()+20,window->y()+90);QQuickWindow *tools=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingToolStrip")tools=qobject_cast<QQuickWindow*>(w);return tools!=nullptr;}());QVERIFY(tools->isVisible());
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());client.setBrushRadius(12);client.setBrushColor(Qt::red);QVERIFY(client.beginStroke({{45,45},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());const auto target=client.activeLayer();client.addLayer("other");QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(Qt::blue);QVERIFY(client.beginStroke({{85,85},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());client.selectLayer(target);QTRY_COMPARE(client.activeLayer(),target);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath("clean.ora"))));QTRY_VERIFY(!client.fileBusy() && !client.modified());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();window->requestActivate();QTRY_VERIFY(window->isActive());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(client.frame().size(),QSize(qCeil(128*window->devicePixelRatio()),qCeil(128*window->devicePixelRatio())));const auto before=client.frame();const auto depth=client.undoDepth();workspace.setActive(workspace.groupForPanel("history"),"history");QQuickItem *historyList=nullptr;QTRY_VERIFY((historyList=findVisualItem(window->contentItem(),"historyList")));historyList->setHeight(48);QTest::qWait(80);historyList->setProperty("contentY",24);QTRY_COMPARE(client.documentHistoryScrollPosition(),qreal(24));QCOMPARE(client.liquifyStrength(),50);QCOMPARE(client.liquifyDensity(),100);QCOMPARE(client.liquifySpeed(),100);QVERIFY(!client.liquifyPressure());client.setLiquifySize(48);
        QTest::qWait(100);window->requestActivate();QTest::qWait(100);QVERIFY(documents->activate(first));auto *action=window->findChild<QObject*>("liquifyAction");QVERIFY(action);QCOMPARE(documents->activeId(),first);QVERIFY(action->property("enabled").toBool());QVERIFY(QMetaObject::invokeMethod(action,"trigger"));QTRY_VERIFY2(client.liquifyActive() && !client.liquifyBusy(),qPrintable(client.lastError()+" active="+QString::number(client.liquifyActive())+" busy="+QString::number(client.liquifyBusy())));auto *panel=findVisualItem(window->contentItem(),"liquifyOptionsBar");QVERIFY(panel);QTRY_VERIFY(panel->isVisible());auto *focus=findVisualItem(window->contentItem(),"liquifyFocus");QVERIFY(focus);auto *indicator=focus->property("indicator").value<QQuickItem*>();QVERIFY(indicator);QCOMPARE(indicator->width(),qreal(13));QCOMPARE(indicator->height(),qreal(13));auto *separator=findVisualItem(window->contentItem(),"liquifyTitleSeparator");QVERIFY(separator);QCOMPARE(separator->width(),qreal(1));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,focus->mapToScene({8,focus->height()/2}).toPoint());QTRY_VERIFY(client.liquifyFocus() && !client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.frame()!=before);QVERIFY(!client.modified());client.setLiquifyFocus(false);QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),before);QTRY_VERIFY(!foreign->isVisible());QTRY_VERIFY(!tools->isVisible());QVERIFY(!documents->activate(other));QCOMPARE(documents->activeId(),first);QVERIFY(documents->newDocument(8,8,true).isEmpty());const auto count=client.layers().size();client.addDefaultLayer();QCOMPARE(client.layers().size(),count);
        canvas->forceActiveFocus();const auto brushSize=client.brushRadius();QTest::keyClick(window,Qt::Key_BracketRight);QCOMPARE(client.liquifySize(),55);QTest::keyClick(window,Qt::Key_BracketLeft);QCOMPARE(client.liquifySize(),48);QCOMPARE(client.brushRadius(),brushSize);QCOMPARE(client.undoDepth(),0);QTRY_COMPARE(historyList->property("count").toInt(),1);QCOMPARE(client.historyEntries().first().toMap().value("title").toString(),QStringLiteral("液化起始状态"));
        auto *pressure=findVisualItem(window->contentItem(),"liquifyPressure");QVERIFY(pressure);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,pressure->mapToScene({16,14}).toPoint());QVERIFY(client.liquifyPressure());auto *speed=findVisualItem(window->contentItem(),"liquifySpeedInput");QVERIFY(speed);QVERIFY(!speed->isEnabled());const auto fixed=panel->position();canvas->zoomBy(2.);canvas->actualSize();QCOMPARE(panel->position(),fixed);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({45,45}));QVERIFY(!client.modified());QTest::mouseMove(window,at({55,45}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({55,45}));QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.frame()!=before);QTRY_COMPARE(client.undoDepth(),1);QVERIFY(!client.modified());const auto preview=client.frame();canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),before);QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),preview);
        QVERIFY(client.resetLiquify());QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),before);client.setLiquifyTool(1);QVERIFY(speed->isEnabled());QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({45,45}));QTest::qWait(120);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({45,45}));QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.frame()!=before);canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Return);QTRY_VERIFY(!client.liquifyActive());QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_COMPARE(historyList->property("contentY").toReal(),qreal(24));QVERIFY(client.modified());QTRY_VERIFY(foreign->isVisible());QTRY_VERIFY(tools->isVisible());QVERIFY(!panel->isVisible());QTest::qWait(50);QCOMPARE(documents->activeId(),first);client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(client.frame(),before);
        const auto documentHistory=client.historyEntries();const auto savedHistoryScroll=client.documentHistoryScrollPosition();QVERIFY(!findVisualItem(tools->contentItem(),"liquifyTool"));QVERIFY(QMetaObject::invokeMethod(action,"trigger"));QTRY_VERIFY(client.liquifyActive() && !client.liquifyBusy());window->requestActivate();QTest::qWait(80);
        QPointingDevice pen("liquify pen",712,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);auto sendPen=[&](QEvent::Type type){const auto p=at({45,45});QTabletEvent e(type,&pen,p,window->mapToGlobal(p),.5,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&e);return e.isAccepted();};QVERIFY(sendPen(QEvent::TabletPress));QVERIFY(client.drawing());QTest::qWait(100);QVERIFY(sendPen(QEvent::TabletRelease));QTRY_VERIFY(!client.liquifyBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.frame()!=before);QTRY_COMPARE(client.undoDepth(),1);canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.liquifyBusy());QTest::keyClick(window,Qt::Key_Z,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.liquifyBusy());canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_Escape);QTRY_VERIFY(!client.liquifyActive());QCOMPARE(client.undoDepth(),depth);QCOMPARE(client.historyEntries(),documentHistory);QCOMPARE(client.documentHistoryScrollPosition(),savedHistoryScroll);QTRY_COMPARE(historyList->property("contentY").toReal(),savedHistoryScroll);documents->floatDocument(first);QTRY_VERIFY(canvas->window()!=window);auto *floating=canvas->window();floating->requestActivate();QTest::qWait(100);QVERIFY(documents->activate(first));QVERIFY(client.beginLiquify());QTRY_VERIFY(client.liquifyActive() && !client.liquifyBusy());auto *floatingOptions=findVisualItem(floating->contentItem(),"liquifyOptionsBar");QVERIFY(floatingOptions);QVERIFY(floatingOptions->isVisible());canvas->forceActiveFocus();client.setLiquifySize(1);QTest::keyClick(floating,Qt::Key_BracketRight);QCOMPARE(client.liquifySize(),2);QTest::keyClick(floating,Qt::Key_BracketLeft);QCOMPARE(client.liquifySize(),1);client.setLiquifySize(48);QVERIFY(client.beginStroke({{45,45},1}));client.endStroke();QTRY_VERIFY(!client.liquifyBusy());QTRY_COMPARE(client.undoDepth(),1);QTest::keyClick(floating,Qt::Key_Z,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.liquifyBusy());QTest::keyClick(floating,Qt::Key_Z,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.liquifyBusy());QVERIFY(!documents->activate(other));QVERIFY(client.finishLiquify(false));QTRY_VERIFY(!client.liquifyActive());QTRY_VERIFY(!findVisualItem(floating->contentItem(),"liquifyOptionsBar") || !findVisualItem(floating->contentItem(),"liquifyOptionsBar")->isVisible());client.setLiquifyStrength(62);client.setLiquifyDensity(81);client.setLiquifySpeed(73);QCOMPARE(warnings,QStringList());QSignalSpy stopped(documents,&DocumentManager::stopped);documents->shutdown();QTRY_COMPARE(stopped.size(),1);
        PaintCoreClient restored(nullptr,settings);QTRY_VERIFY(restored.ready());QCOMPARE(restored.liquifySize(),48);QCOMPARE(restored.liquifyTool(),1);QCOMPARE(restored.liquifyStrength(),62);QCOMPARE(restored.liquifyDensity(),81);QCOMPARE(restored.liquifySpeed(),73);QVERIFY(restored.liquifyPressure());QSignalSpy stoppedAgain(&restored,&PaintCoreClient::stopped);restored.shutdown();QTRY_COMPARE(stoppedAgain.size(),1);
    }
    void layerContextClipboardFocusShortcutsAndLowerEdgeDrop() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("actions.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);window->requestActivate();QTRY_VERIFY(window->isActive());
        QTRY_VERIFY(client.ready());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        client.addLayer("top");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto top=client.activeLayer();client.setBrushRadius(8);client.setBrushColor(Qt::blue);QVERIFY(client.beginStroke({{16,16},1}));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.addMask(top));QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());const auto mask=client.activeLayer();
        const auto item=[&](const QString &name){return findVisualItem(window->contentItem(),name);};
        auto *maskFrame=item("thumbnailFocus:"+QString::number(mask));QVERIFY(maskFrame);QVERIFY(maskFrame->isVisible());QCOMPARE(QQmlProperty::read(maskFrame,"border.color").value<QColor>(),QColor(Qt::white));
        auto *contentFrame=item("thumbnailFocus:"+QString::number(top));QVERIFY(contentFrame);QVERIFY(!contentFrame->isVisible());
        auto *panel=item("layersPanel");QVERIFY(panel);auto *row=item("layerRow:"+QString::number(top));QVERIFY(row);QCOMPARE(QQmlProperty::read(row,"border.width").toInt(),0);
        const auto rightClick=[&](quint64 id){auto *r=item("layerRow:"+QString::number(id));if(!r)return false;QTest::mouseClick(window,Qt::RightButton,Qt::NoModifier,r->mapToScene({r->width()*.7,r->height()/2}).toPoint());return true;};
        QVERIFY(rightClick(top));QTRY_COMPARE(client.activeLayer(),top);auto *menu=window->findChild<QObject*>("layerContextMenu");QVERIFY(menu);QTRY_VERIFY(menu->property("visible").toBool());auto *merge=window->findChild<QObject*>("contextMergeLayers");QVERIFY(merge);QCOMPARE(merge->property("text").toString(),QStringLiteral("向下合并"));QVERIFY(merge->property("enabled").toBool());QVERIFY(QMetaObject::invokeMethod(menu,"close"));
        panel->forceActiveFocus();QTRY_VERIFY(client.layerShortcutsAllowed());QTest::keyClick(window,Qt::Key_C,Qt::ControlModifier);QTRY_VERIFY(client.canPasteLayers());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.layers().size(),3);
        QTest::keyClick(window,Qt::Key_V,Qt::ControlModifier);QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());const auto pasted=client.activeLayer();QVERIFY(pasted!=top);QTest::keyClick(window,Qt::Key_Delete);QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());
        client.selectLayers(top,0);QTRY_COMPARE(client.activeLayer(),top);QTRY_VERIFY(!client.layerEditBusy());panel->forceActiveFocus();QTest::keyClick(window,Qt::Key_J,Qt::ControlModifier);QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());const auto duplicate=client.activeLayer();quint64 duplicateMask=0;for(const auto &v:client.layers()){const auto l=v.toMap();if(l.value("mask").toBool() && l.value("parent").toULongLong()==duplicate)duplicateMask=l.value("id").toULongLong();}QVERIFY(duplicateMask);
        client.selectLayer(duplicateMask);QTRY_COMPARE(client.activeLayer(),duplicateMask);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(item("thumbnailFocus:"+QString::number(duplicateMask))->isVisible());QVERIFY(!item("thumbnailFocus:"+QString::number(duplicate))->isVisible());panel->forceActiveFocus();QTest::keyClick(window,Qt::Key_Delete);QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());bool duplicateExists=false;for(const auto &v:client.layers())if(v.toMap().value("id").toULongLong()==duplicate)duplicateExists=true;QVERIFY(duplicateExists);
        auto *search=item("layerSearch");QVERIFY(search);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,search->mapToScene({search->width()/2,search->height()/2}).toPoint());QTRY_VERIFY(search->hasActiveFocus());QTRY_VERIFY(!client.layerShortcutsAllowed());QVERIFY(search->setProperty("text","abc"));QVERIFY(search->setProperty("cursorPosition",0));const auto count=client.layers().size();QTest::keyClick(window,Qt::Key_Delete);QCOMPARE(search->property("text").toString(),QStringLiteral("bc"));QCOMPARE(client.layers().size(),count);
        auto *list=item("layerList");QVERIFY(list);QTRY_COMPARE(list->property("count").toInt(),0);QVERIFY(item("layerSearchEmpty")->isVisible());
        search->setProperty("text"," TOP ");QTRY_COMPARE(list->property("count").toInt(),2);QVERIFY(!item("layerSearchEmpty")->isVisible());
        client.activateEraser();search->setProperty("text","");QTest::keyClick(window,Qt::Key_B);QCOMPARE(search->property("text").toString(),QStringLiteral("b"));QVERIFY(client.eraser());QTest::keyClick(window,Qt::Key_Escape);QCOMPARE(search->property("text").toString(),QString());QTRY_VERIFY(client.layerShortcutsAllowed());QTest::keyClick(window,Qt::Key_B);QVERIFY(!client.eraser());
        search->forceActiveFocus();search->setProperty("text","top");QTest::keyClick(window,Qt::Key_Return);QTRY_VERIFY(client.layerShortcutsAllowed());QCOMPARE(search->property("text").toString(),QStringLiteral("top"));
        search->forceActiveFocus();auto *eraser=item("eraserTool");QVERIFY(eraser);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,eraser->mapToScene({13,13}).toPoint());QVERIFY(client.eraser());QTRY_VERIFY(client.layerShortcutsAllowed());QTest::keyClick(window,Qt::Key_B);QVERIFY(!client.eraser());
        // Clipboard shortcuts must work after a tool click has released search focus.
        QTest::keyClick(window,Qt::Key_C,Qt::ControlModifier);QTRY_VERIFY(!client.layerEditBusy());QTest::keyClick(window,Qt::Key_V,Qt::ControlModifier);QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());client.undo();QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());search->setProperty("text","");
        QVERIFY(!window->findChild<QObject*>("contextCopyLayers"));QVERIFY(!window->findChild<QObject*>("contextPasteLayers"));QVERIFY(!window->findChild<QObject*>("copyLayersAction"));QVERIFY(!window->findChild<QObject*>("pasteLayersAction"));QVERIFY(!item("liquifyTool"));
        client.selectLayers(top,0);QTRY_COMPARE(client.activeLayer(),top);QTRY_VERIFY(!client.layerEditBusy());panel->forceActiveFocus();QTest::keyClick(window,Qt::Key_G,Qt::ControlModifier);QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();client.toggleGroupExpanded(group);QTRY_COMPARE(list->property("count").toInt(),3);search->setProperty("text"," TOP ");QTRY_COMPARE(list->property("count").toInt(),3);QVERIFY(item("layerRow:"+QString::number(top)));search->setProperty("text","组");QTRY_COMPARE(list->property("count").toInt(),2);search->setProperty("text","   ");QTRY_COMPARE(list->property("count").toInt(),3);client.toggleGroupExpanded(group);search->setProperty("text","");QTRY_COMPARE(list->property("count").toInt(),4);QVERIFY(rightClick(group));QTRY_VERIFY(menu->property("visible").toBool());QCOMPARE(merge->property("text").toString(),QStringLiteral("合并组"));QVERIFY(QMetaObject::invokeMethod(menu,"close"));
        client.selectLayers(group,0);client.selectLayers(duplicate,int(Qt::ControlModifier));QTRY_COMPARE(client.activeLayer(),duplicate);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(rightClick(group));QTRY_VERIFY(menu->property("visible").toBool());QCOMPARE(client.selectedLayers().size(),2);QCOMPARE(merge->property("text").toString(),QStringLiteral("合并图层"));QVERIFY(QMetaObject::invokeMethod(menu,"close"));
        // Real drop events exercise the lower blue line and destination-parent inheritance.
        client.selectLayers(duplicate,0);QTRY_COMPARE(client.activeLayer(),duplicate);QTRY_VERIFY(!client.layerEditBusy());auto *target=item("layerRow:"+QString::number(top));QVERIFY(target);const auto at=target->mapToScene({target->width()*.72,target->height()-3}).toPoint();QMimeData mime;mime.setData("application/x-drawverse-layer",client.layerDragPayload(duplicate).toUtf8());QDragEnterEvent enter(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&enter);QDragMoveEvent move(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&move);QDropEvent drop(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(window,&drop);QVERIFY(drop.isAccepted());QTRY_VERIFY(!client.layerEditBusy());
        QVariantMap moved;for(const auto &v:client.layers())if(v.toMap().value("id").toULongLong()==duplicate)moved=v.toMap();QCOMPARE(moved.value("parent").toULongLong(),group);QCOMPARE(client.lastError(),QString());
        client.selectLayers(group,0);QTRY_COMPARE(client.activeLayer(),group);QTRY_VERIFY(!client.layerEditBusy());const auto depth=client.undoDepth();panel->forceActiveFocus();QTest::keyClick(window,Qt::Key_E,Qt::ControlModifier);QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.undoDepth(),depth+1);client.undo();QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());
        auto *mainDuplicate=window->findChild<QObject*>("duplicateLayersAction");QVERIFY(mainDuplicate);QVERIFY(mainDuplicate->property("enabled").toBool());QVERIFY(QMetaObject::invokeMethod(mainDuplicate,"trigger"));QTRY_COMPARE(client.layers().size(),9);QTRY_VERIFY(!client.layerEditBusy());
        workspace.setActive(workspace.groupForPanel("brush"),"brush");QQuickItem *brushSearch=nullptr;QTRY_VERIFY((brushSearch=item("brushSearch")));auto *presets=item("brushPresetList");QVERIFY(presets);brushSearch->forceActiveFocus();QVERIFY(client.textInputFocused());brushSearch->setProperty("text"," 细线 ");QTRY_COMPARE(presets->property("count").toInt(),1);QTest::keyClick(window,Qt::Key_Return);QTRY_VERIFY(client.layerShortcutsAllowed());brushSearch->forceActiveFocus();QTest::keyClick(window,Qt::Key_Escape);QTRY_COMPARE(presets->property("count").toInt(),client.brushLibrary()->presets().size());QTRY_VERIFY(client.layerShortcutsAllowed());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void groupPassThroughIsGroupOnlyAndChangesBackdropWithUndo() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("pass.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();client.setBrushRadius(64);client.setBrushColor(QColor("#404040"));QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.addLayer("dodge");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto top=client.activeLayer();client.setBrushRadius(12);client.setBrushColor(QColor("#808080"));QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.setLayerBlend(top,9));QTRY_COMPARE(client.undoDepth(),4);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());
        const auto pixel=[&](){const auto r=client.frameRegion();const auto &f=client.frame();return f.pixelColor(qFloor((32.5-r.x())*f.width()/r.width()),qFloor((32.5-r.y())*f.height()/r.height()));};const auto flat=pixel();QVERIFY(client.groupLayer(top,"组 1"));QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();QVERIFY(group!=top);QTRY_VERIFY(client.frameRevision()>=client.revision());QVERIFY(client.setLayerBlend(group,0));QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());const auto isolated=pixel();QVERIFY(flat!=isolated);auto *blend=findVisualItem(window->contentItem(),"layerBlendMode");QVERIFY(blend);QTRY_COMPARE(blend->property("count").toInt(),28);QCOMPARE(blend->property("currentIndex").toInt(),1);const auto depth=client.undoDepth();const auto revision=client.revision();
        client.previewLayerBlend(group,27);QTRY_COMPARE(pixel(),flat);QCOMPARE(client.undoDepth(),depth);QCOMPARE(client.revision(),revision);client.clearLayerBlendPreview();QTRY_COMPARE(pixel(),isolated);QVERIFY(blend->setProperty("currentIndex",0));QVERIFY(QMetaObject::invokeMethod(blend,"activated",Q_ARG(int,0)));QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(),flat);QCOMPARE(blend->property("currentText").toString(),QStringLiteral("穿透"));client.undo();QTRY_COMPARE(client.undoDepth(),depth);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(),isolated);client.redo();QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(),flat);
        const auto preview=qEnvironmentVariable("DRAWVERSE_PASS_PREVIEW");if(!preview.isEmpty())QVERIFY(window->grabWindow().save(preview));client.selectLayer(top);QTRY_COMPARE(client.activeLayer(),top);QTRY_VERIFY(!client.layerEditBusy());QTRY_COMPARE(blend->property("count").toInt(),27);QVERIFY(!client.setLayerBlend(top,27));QCOMPARE(client.undoDepth(),depth+1);QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void eyedropperRingComparesColorsDuringMouseAndTabletDrag() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("ring.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushRadius(12);client.setBrushColor(Qt::red);QVERIFY(client.beginStroke({{32,64},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());client.setBrushColor(Qt::green);QVERIFY(client.beginStroke({{96,64},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);QTRY_VERIFY(client.frameRevision()>=client.revision());client.activateEyedropper();client.setBrushColor(Qt::blue);const auto revision=client.revision();const int depth=client.undoDepth();
        auto *ring=findVisualItem(window->contentItem(),"eyedropperSamplingRing");QVERIFY(ring);auto *show=findVisualItem(window->contentItem(),"showSamplingRing");QVERIFY(show);QVERIFY(client.samplingRingEnabled());QCOMPARE(QQmlProperty::read(show,"ToolTip.delay",qmlContext(show)).toInt(),1000);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};const auto red=at({32,64}),green=at({96,64});QTest::mouseMove(window,red);QVERIFY(!ring->isVisible());QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,red);QTRY_VERIFY(ring->isVisible());QCOMPARE(canvas->previousSampleColor(),QColor(Qt::blue));QCOMPARE(canvas->sampledColor(),QColor(Qt::red));QCOMPARE(client.brushColor(),QColor(Qt::red));QCOMPARE(ring->width(),120.);
        // Inspect the actual native composition: top=new, bottom=old, neutral outer ring, hollow center.
        QTest::qWait(80);const auto image=window->grabWindow();QVERIFY(!image.isNull());const auto center=canvas->mapToScene(canvas->cursorPosition());const qreal scale=qreal(image.width())/window->width();const auto screenPixel=[&](QPointF offset){const auto p=(center+offset)*scale;return image.pixelColor(qRound(p.x()),qRound(p.y()));};QCOMPARE(screenPixel({0,-36}),QColor(Qt::red));QCOMPARE(screenPixel({0,36}),QColor(Qt::blue));QCOMPARE(screenPixel({0,-51}),QColor("#808080"));QCOMPARE(screenPixel({4,4}),QColor(Qt::red));const auto preview=qEnvironmentVariable("DRAWVERSE_RING_PREVIEW");if(!preview.isEmpty())QVERIFY(image.save(preview));
        QTest::mouseMove(window,green);QTRY_COMPARE(canvas->sampledColor(),QColor(Qt::green));QCOMPARE(canvas->previousSampleColor(),QColor(Qt::blue));QCOMPARE(client.brushColor(),QColor(Qt::green));QTest::mouseMove(window,at({-4,64}));QVERIFY(!ring->isVisible());QCOMPARE(client.brushColor(),QColor(Qt::green));QTest::mouseMove(window,red);QTRY_VERIFY(ring->isVisible());QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,red);QVERIFY(!ring->isVisible());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,show->mapToScene({7,show->height()/2}).toPoint());QVERIFY(!client.samplingRingEnabled());QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,green);QVERIFY(!ring->isVisible());QCOMPARE(client.brushColor(),QColor(Qt::green));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,green);client.setSamplingRingEnabled(true);
        client.activateBrush();client.setBrushColor(Qt::blue);canvas->forceActiveFocus();QTest::keyPress(window,Qt::Key_Alt);QTest::mousePress(window,Qt::LeftButton,Qt::AltModifier,red);QTRY_VERIFY(ring->isVisible());QCOMPARE(canvas->previousSampleColor(),QColor(Qt::blue));QTest::mouseMove(window,green);QTRY_COMPARE(canvas->sampledColor(),QColor(Qt::green));QTest::mouseRelease(window,Qt::LeftButton,Qt::AltModifier,green);QTest::keyRelease(window,Qt::Key_Alt);QVERIFY(!ring->isVisible());QVERIFY(!client.drawing());
        client.activateEyedropper();client.setBrushColor(Qt::blue);QPointingDevice pen("ring pen",126,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);auto send=[&](QEvent::Type type,QPoint p){QTabletEvent e(type,&pen,p,window->mapToGlobal(p),.5,0,0,0,0,0,Qt::NoModifier,type==QEvent::TabletMove?Qt::NoButton:Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&e);return e.isAccepted();};QVERIFY(send(QEvent::TabletPress,red));QVERIFY(ring->isVisible());QCOMPARE(canvas->previousSampleColor(),QColor(Qt::blue));QVERIFY(send(QEvent::TabletMove,green));QCOMPARE(canvas->sampledColor(),QColor(Qt::green));QVERIFY(send(QEvent::TabletRelease,green));QVERIFY(!ring->isVisible());
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,red);QVERIFY(ring->isVisible());QEvent deactivate(QEvent::WindowDeactivate);QCoreApplication::sendEvent(window,&deactivate);QVERIFY(!ring->isVisible());QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,red);QCOMPARE(client.undoDepth(),depth);QCOMPARE(client.revision(),revision);QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void altEyedropperSamplesWithoutHistoryAndBracketKeysResize() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(client.ready());
        auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();main->requestActivate();QTest::qWait(60);
        client.setSelectionTool(0);QTRY_COMPARE(client.selectionTool(),0);
        // Paint a known colour, then sample it back through the eyedropper path.
        client.setBrushRadius(6);client.setBrushColor(QColor("#c81e3a"));
        InputSample stroke;stroke.position={32,32};stroke.pressure=1;
        QVERIFY(client.beginStroke(stroke));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),10000);
        const auto sampled=client.sampleDocumentPixel(32,32);
        QVERIFY2(sampled.isValid(),"eyedropper returned no colour");
        QVERIFY2(sampled.red()>150 && sampled.green()<90,qPrintable(sampled.name()));
        // Alt is the eyedropper: it adopts the colour and writes nothing to the document.
        client.setBrushColor(QColor("#1188ee"));QTRY_COMPARE(client.brushColor(),QColor("#1188ee"));
        const auto historyBefore=client.undoDepth();const auto revisionBefore=client.revision();
        const auto center=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(32,32)*canvas->zoom()).toPoint();
        QTest::mousePress(main,Qt::LeftButton,Qt::AltModifier,center);
        QTest::mouseRelease(main,Qt::LeftButton,Qt::AltModifier,center);
        QTRY_VERIFY(client.brushColor().red()>150 && client.brushColor().green()<90);
        QCOMPARE(client.undoDepth(),historyBefore);QCOMPARE(client.revision(),revisionBefore);
        // Switching away from the marquee and back keeps the chosen shape instead of resetting it.
        client.setSelectionTool(2);QTRY_COMPARE(client.selectionTool(),2);
        client.activateBrush();QTRY_VERIFY(!client.selectionTool());
        auto *selectionSlot=findVisualItem(main->contentItem(),"selectionTool");QVERIFY(selectionSlot);QCOMPARE(selectionSlot->property("glyph").toString(),QString("ellipseSelection"));
        auto *move=findVisualItem(main->contentItem(),"moveLayerTool");QVERIFY(move);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,move->mapToScene({13,13}).toPoint());QVERIFY(client.moveTool());QCOMPARE(selectionSlot->property("glyph").toString(),QString("ellipseSelection"));
        client.activateMarquee();QTRY_COMPARE(client.selectionTool(),2);
        QVERIFY(!client.moveTool() && !client.eraser());
        client.setSelectionTool(1);QTRY_COMPARE(client.selectionTool(),1);
        client.activateEraser();QTRY_VERIFY(!client.selectionTool());
        client.activateMarquee();QTRY_COMPARE(client.selectionTool(),1);
        client.setSelectionTool(0);QTRY_VERIFY(!client.selectionTool());        // Alt is the eyedropper: the brush ring goes away and the pointer becomes a crosshair.
        canvas->forceActiveFocus();QTest::mouseMove(main,center);
        QTest::keyPress(main,Qt::Key_Alt);QTest::qWait(40);
        QVERIFY2(!canvas->brushCursorVisible(),"brush ring still shown while Alt is held");
        auto *cross=findVisualItem(main->contentItem(),"eyedropperCursor");QVERIFY(cross);QVERIFY(cross->isVisible());QCOMPARE(canvas->cursor().shape(),Qt::BlankCursor);
        QTest::mouseMove(main,center+QPoint(5,3));QTRY_VERIFY(QLineF(cross->mapToScene({8,8}),center+QPoint(5,3)).length()<1);
        QPointingDevice pen("Picker pen",1901,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,2);
        const QPointF penPoint=center+QPoint(11,8);QTabletEvent hover(QEvent::TabletMove,&pen,penPoint,main->mapToGlobal(penPoint.toPoint()),0,0,0,0,0,0,Qt::AltModifier,Qt::NoButton,Qt::NoButton);QCoreApplication::sendEvent(main,&hover);QVERIFY(hover.isAccepted());QVERIFY(canvas->eyedropperCursorVisible());QVERIFY(QLineF(cross->mapToScene({8,8}),penPoint).length()<1);
        const auto pickerImage=main->grabWindow();const auto blackPixel=cross->mapToScene({0,7});const auto dpi=pickerImage.width()/qreal(main->width());QCOMPARE(pickerImage.pixelColor(qRound(blackPixel.x()*dpi),qRound(blackPixel.y()*dpi)),QColor(Qt::black));
        client.setBrushColor(QColor("#1188ee"));QTabletEvent pickPress(QEvent::TabletPress,&pen,center,main->mapToGlobal(center),.6,0,0,0,0,0,Qt::AltModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(main,&pickPress);QTabletEvent pickRelease(QEvent::TabletRelease,&pen,center,main->mapToGlobal(center),0,0,0,0,0,0,Qt::AltModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(main,&pickRelease);QVERIFY(pickPress.isAccepted() && pickRelease.isAccepted());QVERIFY(client.brushColor().red()>150 && client.brushColor().green()<90);QCOMPARE(client.undoDepth(),historyBefore);
        client.setSelectionTool(2);QVERIFY(!cross->isVisible());client.activateBrush();
        QTest::keyRelease(main,Qt::Key_Alt);QTest::qWait(40);QVERIFY(!cross->isVisible());
        const auto beforePan=canvas->documentRect().topLeft();QTest::keyPress(main,Qt::Key_Space);
        QTabletEvent panPress(QEvent::TabletPress,&pen,center,main->mapToGlobal(center),.6,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(main,&panPress);const auto panEnd=center+QPoint(12,9);QTabletEvent panMove(QEvent::TabletMove,&pen,panEnd,main->mapToGlobal(panEnd),.6,0,0,0,0,0,Qt::NoModifier,Qt::NoButton,Qt::LeftButton);QCoreApplication::sendEvent(main,&panMove);QTabletEvent panRelease(QEvent::TabletRelease,&pen,panEnd,main->mapToGlobal(panEnd),0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(main,&panRelease);QTest::keyRelease(main,Qt::Key_Space);QCOMPARE(canvas->documentRect().topLeft(),beforePan+QPointF(12,9));QCOMPARE(client.undoDepth(),historyBefore);QVERIFY(!client.drawing());
        // Bracket keys resize the active painting tool.
        client.setBrushRadius(20);const auto wide=client.brushRadius();
        QTest::keyClick(main,Qt::Key_BracketLeft);QTRY_VERIFY(client.brushRadius()<wide);
        const auto narrow=client.brushRadius();
        QTest::keyClick(main,Qt::Key_BracketRight);QTRY_VERIFY(client.brushRadius()>narrow);
        auto *picker=findVisualItem(main->contentItem(),"eyedropperTool");QVERIFY(picker);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,picker->mapToScene({13,13}).toPoint());QVERIFY(client.eyedropperTool());QVERIFY(picker->property("checked").toBool());QVERIFY(!client.selectionTool() && !client.moveTool() && !client.eraser());
        client.setBrushColor(QColor("#1188ee"));const auto pickedAt=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(32,32)*canvas->zoom()).toPoint();QTest::mouseMove(main,pickedAt);QVERIFY(canvas->eyedropperCursorVisible());QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,pickedAt);QVERIFY(client.brushColor().red()>150 && client.brushColor().green()<90);QCOMPARE(client.undoDepth(),historyBefore);QCOMPARE(client.revision(),revisionBefore);
        auto *toolStatus=findVisualItem(main->contentItem(),"statusTool");QVERIFY(toolStatus);QVERIFY(toolStatus->property("text").toString().contains(QStringLiteral("吸色")));
        canvas->forceActiveFocus();QTest::keyClick(main,Qt::Key_B);QVERIFY(!client.eyedropperTool());QTest::keyClick(main,Qt::Key_I);QVERIFY(client.eyedropperTool());
        workspace.floatToolStrip(250,150);QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingToolStrip")floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());auto *floatingPicker=findVisualItem(floating->contentItem(),"eyedropperTool");QVERIFY(floatingPicker);QVERIFY(floatingPicker->property("checked").toBool());client.activateBrush();QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,floatingPicker->mapToScene({13,13}).toPoint());QVERIFY(client.eyedropperTool());QVERIFY(floatingPicker->y()+floatingPicker->height()<floatingPicker->parentItem()->height());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }    void selectionOutlineMarchesWhileVisibleAndStopsOtherwise() {
        QQuickWindow window;window.setObjectName("antsHost");window.setColor(Qt::transparent);
        auto *item=new SelectionOverlay();item->setObjectName("antsOverlay");item->setParentItem(window.contentItem());
        item->setDocumentRect({0,0,64,64});item->setZoom(1);
        const auto step=[](int op,int shape,QRectF r){return QVariantMap{{"operation",op},{"shape",shape},{"x",r.x()},{"y",r.y()},{"width",r.width()},{"height",r.height()}};};
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));
        QSignalSpy phaseChanged(item,&SelectionOverlay::dashPhaseChanged);QSignalSpy geometryChanged(item,&SelectionOverlay::geometryChanged);
        // With no selection there is nothing to confirm, so no animation runs.
        QTest::qWait(200);QCOMPARE(item->dashPhase(),0);QCOMPARE(phaseChanged.size(),0);
        item->setSteps(QVariantList{step(0,0,{8,8,40,40})});item->setEnabledSelection(true);
        // The dash phase advances so the boundary visibly marches; geometry stays untouched.
        QTRY_VERIFY_WITH_TIMEOUT(item->dashPhase()!=0,1500);
        const auto settled=geometryChanged.size();
        QTRY_VERIFY_WITH_TIMEOUT(phaseChanged.size()>1,1500);
        QCOMPARE(geometryChanged.size(),settled);
        // Closing the window stops the timer instead of repainting forever.
        window.close();QTest::qWait(30);
        const auto stopped=phaseChanged.size();QTest::qWait(250);QCOMPARE(phaseChanged.size(),stopped);
    }

    void denseLassoPaintingUpdatesFramesAndRestoresSelectionAfterUndo() {
        QTemporaryDir temp;
        PaintCoreClient client(nullptr,temp.filePath("selection.ini"));
        QTRY_VERIFY(client.ready());
        client.newTransparentDocument(1024,1024);
        QTRY_VERIFY(client.ready() && client.documentWidth()==1024);
        client.requestViewport(0,{0,0,1024,1024},{1024,1024});
        QVector<QPointF> points;
        for(int i=0;i<4096;++i) {
            const double angle=i*6.283185307179586/4096;
            points.append({512+410*std::cos(angle),512+410*std::sin(angle)});
        }
        client.activateBrush();client.setBrushRadius(32);client.setBrushColor(Qt::black);
        for(const double feather:{0.,8.}) {
            client.setSelectionFeather(feather);
            QVERIFY(client.editSelectionPath(points,0,false));
            QTRY_VERIFY(!client.layerEditBusy());
            const auto depth=client.undoDepth();
            QCOMPARE(client.selectionSteps().last().toMap().value("points").toList().size(),4096);
            int ticks=0;
            QTimer heartbeat;heartbeat.setInterval(1);
            connect(&heartbeat,&QTimer::timeout,this,[&]{++ticks;});heartbeat.start();
            InputSample sample;sample.position={180,512};
            QVERIFY(client.beginStroke(sample));
            for(int i=1;i<=120;++i) {
                sample.position={180+i*5.5,512+std::sin(i*.07)*160};
                client.strokeTo(sample);
            }
            client.endStroke();
            QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),depth+1,10000);
            QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),10000);
            heartbeat.stop();QVERIFY(ticks>0);
            QCOMPARE(client.frame().size(),QSize(1024,1024));
            QVERIFY(client.frame().pixelColor(180,512).alpha()>0);
            QCOMPARE(client.frame().pixelColor(10,10).alpha(),0);
            client.activateEraser();client.setBrushRadius(32);
            const auto paintedAlpha=client.frame().pixelColor(180,512).alpha();
            sample.position={180,512};QVERIFY(client.beginStroke(sample));client.endStroke();
            QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),depth+2,10000);
            QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),10000);
            QVERIFY(client.frame().pixelColor(180,512).alpha()<paintedAlpha);
            QCOMPARE(client.frame().pixelColor(10,10).alpha(),0);
            client.undo();QTRY_COMPARE(client.undoDepth(),depth+1);
            QTRY_VERIFY(client.frameRevision()>=client.revision());
            QCOMPARE(client.frame().pixelColor(180,512).alpha(),paintedAlpha);
            client.activateBrush();
            client.undo();QTRY_COMPARE(client.undoDepth(),depth);
            QTRY_VERIFY(client.frameRevision()>=client.revision());
            QCOMPARE(client.frame().pixelColor(180,512).alpha(),0);
            // Clearing and undoing the selection must invalidate the coverage cache.
            QVERIFY(client.clearSelection());QTRY_VERIFY(!client.layerEditBusy());
            QVERIFY(!client.selectionEnabled());
            client.undo();QTRY_COMPARE(client.undoDepth(),depth);
            QTRY_VERIFY(client.selectionEnabled() && !client.layerEditBusy());
            sample.position={100,512};QVERIFY(client.beginStroke(sample));client.endStroke();
            QTRY_COMPARE(client.undoDepth(),depth+1);
            QTRY_VERIFY(client.frameRevision()>=client.revision());
            QCOMPARE(client.frame().pixelColor(80,512).alpha(),0);
            QVERIFY(client.frame().pixelColor(110,512).alpha()>0);
            client.undo();QTRY_COMPARE(client.undoDepth(),depth);
            QTRY_VERIFY(!client.layerEditBusy());
        }
        QVERIFY(client.lastError().isEmpty());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);
        client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void deletingFinalLayerShowsTransparencyAndAllowsUndoPasteAndNewLayer() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("empty.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.layers().size(),1);
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);
        const auto original=client.activeLayer();QVERIFY(client.copySelectedLayers());QTRY_VERIFY(client.canPasteLayers());QTRY_VERIFY(!client.layerEditBusy());
        QTest::keyClick(window,Qt::Key_Delete);QTRY_COMPARE(client.layers().size(),0);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.activeLayer(),quint64(0));QVERIFY(client.selectedLayers().isEmpty());
        auto *trash=findVisualItem(window->contentItem(),"deleteLayer");QVERIFY(trash);QVERIFY(!trash->isEnabled());
        canvas->setClient(nullptr);const auto rendered=[&]{client.requestViewport(0,{0,0,32,32},{32,32});return client.frameRevision()>=client.revision() && client.frame().size()==QSize(32,32);};QTRY_VERIFY(rendered());for(int y=0;y<32;++y)for(int x=0;x<32;++x)QCOMPARE(client.frame().pixelColor(x,y).alpha(),0);
        client.undo();QTRY_COMPARE(client.layers().size(),1);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.activeLayer(),original);client.redo();QTRY_COMPARE(client.layers().size(),0);QTRY_VERIFY(!client.layerEditBusy());
        canvas->setClient(&client);canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_V,Qt::ControlModifier);QTRY_COMPARE(client.layers().size(),1);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.activeLayer()!=0);QVERIFY(trash->isEnabled());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,trash->mapToScene({trash->width()/2,trash->height()/2}).toPoint());QTRY_COMPARE(client.layers().size(),0);QTRY_VERIFY(!client.layerEditBusy());client.addLayer("new after empty");QTRY_COMPARE(client.layers().size(),1);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.activeLayer()!=0);
        QVERIFY(client.lastError().isEmpty());QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void brushToolbarTransferTracksActivePresetForPaintAndErase() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("transfer.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());window->requestActivate();QTest::qWait(80);
        auto *library=client.brushLibrary();const auto original=library->selectedId();
        auto *opacity=findVisualItem(window->contentItem(),"brushToolbarOpacity"),*flow=findVisualItem(window->contentItem(),"brushToolbarFlow"),*opacityPressure=findVisualItem(window->contentItem(),"brushToolbarPressureOpacity"),*flowPressure=findVisualItem(window->contentItem(),"brushToolbarPressureFlow");
        QVERIFY(opacity && flow && opacityPressure && flowPressure);
        for(const bool eraser:{false,true}) {
            client.activateBrush();client.setEraser(eraser);library->setOpacity(.4);library->setFlow(.3);library->setPressureOpacity(false);library->setPressureFlow(false);
            QTRY_COMPARE(opacity->property("value").toDouble(),.4);QTRY_COMPARE(flow->property("value").toDouble(),.3);
            for(auto *slider:{opacity,flow}) {QVERIFY(slider->isVisible());slider->forceActiveFocus();QTest::keyClick(window,Qt::Key_Right);}
            QVERIFY(qAbs(library->opacity()-.41)<.00001);QVERIFY(qAbs(library->flow()-.31)<.00001);QCOMPARE(client.brushOpacity(),library->opacity());
            const auto curves=library->pressureDynamics();
            for(auto *toggle:{opacityPressure,flowPressure}) {QVERIFY(toggle->isVisible());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,toggle->mapToScene({7,toggle->height()/2}).toPoint());QVERIFY(toggle->property("checked").toBool());QCOMPARE(QQmlProperty::read(toggle,"ToolTip.delay",qmlContext(toggle)).toInt(),1000);}
            QVERIFY(library->pressureOpacity() && library->pressureFlow());QCOMPARE(client.eraser(),eraser);
            library->setPressureOpacity(false);library->setPressureFlow(false);
            QTRY_VERIFY(!opacityPressure->property("checked").toBool());QTRY_VERIFY(!flowPressure->property("checked").toBool());
            QCOMPARE(library->pressureDynamics(),curves);
        }
        QVERIFY(library->select("round-fine"));library->setOpacity(.7);library->setFlow(.2);library->setPressureOpacity(true);library->setPressureFlow(true);
        QTRY_COMPARE(opacity->property("value").toDouble(),.7);QTRY_COMPARE(flow->property("value").toDouble(),.2);QTRY_VERIFY(opacityPressure->property("checked").toBool());QTRY_VERIFY(flowPressure->property("checked").toBool());
        BrushLibrary restored;QVERIFY(restored.restore(library->snapshot()));QCOMPARE(restored.opacity(),.7);QCOMPARE(restored.flow(),.2);QVERIFY(restored.pressureOpacity() && restored.pressureFlow());
        QVERIFY(library->select(original));QVERIFY(qAbs(opacity->property("value").toDouble()-.41)<.00001);QVERIFY(qAbs(flow->property("value").toDouble()-.31)<.00001);QVERIFY(!opacityPressure->property("checked").toBool());QVERIFY(!flowPressure->property("checked").toBool());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void smudgeSharedBrushParametersInputAndDelayedHelp() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("smudge.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);QTest::keyClick(window,Qt::Key_R);QTRY_VERIFY(client.smudgeTool());
        QCOMPARE(client.smudgeStrength(),.5);QVERIFY(client.smudgePressure() && !client.smudgeAllLayers() && !client.smudgeFingerPainting());auto *bar=findVisualItem(window->contentItem(),"smudgeOptionsBar");QVERIFY(bar && bar->isVisible());
        QVERIFY(!findVisualItem(window->contentItem(),"smudgeBrushPreset"));auto *separator=findVisualItem(window->contentItem(),"smudgeOptionsSeparator");QVERIFY(separator && separator->isVisible());QCOMPARE(separator->width(),1.);
        QVERIFY(client.brushLibrary()->select("round-fine"));QVERIFY(client.smudgeTool());
        client.brushLibrary()->setSpacing(.02);QCOMPARE(client.brushSpacing(),.02);BrushLibrary restored;QVERIFY(restored.restore(client.brushLibrary()->snapshot()));QCOMPARE(restored.spacing(),.02);
        auto *strength=findVisualItem(window->contentItem(),"smudgeStrengthInput");QVERIFY(strength);strength->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_8);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QCOMPARE(client.smudgeStrength(),.8);
        auto click=[&](const QString &name){auto *item=findVisualItem(window->contentItem(),name);if(!item)return false;QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,item->mapToScene({7,item->height()/2}).toPoint());return true;};
        QVERIFY(click("smudgePressure"));QVERIFY(!client.smudgePressure());QVERIFY(click("smudgeAllLayers"));QVERIFY(client.smudgeAllLayers());QVERIFY(click("smudgeAllLayers"));QVERIFY(click("smudgeFingerPainting"));QVERIFY(client.smudgeFingerPainting());
        for(const auto name:{"smudgePressure","smudgeAllLayers","smudgeFingerPainting","wandAntialias","wandContiguous","wandAllLayers","bucketAntialias","bucketContiguous","bucketAllLayers","selectionAntialias"}) {
            auto *item=findVisualItem(window->contentItem(),name);QVERIFY(item);QCOMPARE(QQmlProperty::read(item,"ToolTip.delay",qmlContext(item)).toInt(),1000);QVERIFY(!QQmlProperty::read(item,"ToolTip.text",qmlContext(item)).toString().isEmpty());
        }
        auto *helpItem=findVisualItem(window->contentItem(),"smudgeFingerPainting");auto *tip=QQmlProperty::read(helpItem,"ToolTip.toolTip",qmlContext(helpItem)).value<QObject*>();QVERIFY(tip);
        {
            // A live desktop can deliver native cursor moves between QTest events.
            // Isolate only this timed hover; real clicks/tablet input stay unfiltered.
            class HoverInput final : public QObject {
                QWindow *host;
                bool eventFilter(QObject *,QEvent *event) override {return event->spontaneous() && (event->type()==QEvent::MouseMove || event->type()==QEvent::Enter || event->type()==QEvent::Leave);}
            public:
                explicit HoverInput(QWindow *window):host(window){host->installEventFilter(this);}
                ~HoverInput() override{host->removeEventFilter(this);}
                void move(QPointF position){QMouseEvent event(QEvent::MouseMove,position,host->mapToGlobal(position),Qt::NoButton,Qt::NoButton,Qt::NoModifier);QCoreApplication::sendEvent(host,&event);}
            } input(window);
            input.move({2,100});input.move(helpItem->mapToScene({7,helpItem->height()/2}));QTRY_VERIFY(helpItem->property("hovered").toBool());QTest::qWait(400);QVERIFY(!tip->property("visible").toBool());QTRY_VERIFY_WITH_TIMEOUT(tip->property("visible").toBool(),1500);input.move({2,100});QTRY_VERIFY(!tip->property("visible").toBool());
        }
        client.setBrushRadius(4);client.setBrushOpacity(1);client.setBrushColor(Qt::blue);const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({20,32}));QTest::mouseMove(window,at({32,32}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({32,32}));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());
        QCOMPARE(client.historyEntries().last().toMap().value("kind").toInt(),26);QCOMPARE(client.historyEntries().last().toMap().value("title").toString(),QStringLiteral("涂抹"));
        const auto pixel=[&](int x,int y){const auto r=client.frameRegion();const auto &f=client.frame();return f.pixelColor(qFloor((x+.5-r.x())*f.width()/r.width()),qFloor((y+.5-r.y())*f.height()/r.height()));};QVERIFY(pixel(20,32).blue()>0);client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(20,32).alpha(),0);
        client.setSmudgePressure(true);QPointingDevice pen("smudge pen",125,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        for(const auto type:{QEvent::TabletPress,QEvent::TabletMove,QEvent::TabletRelease}) {const auto local=at(type==QEvent::TabletPress?QPointF(20,32):QPointF(32,32));QTabletEvent event(type,&pen,local,window->mapToGlobal(local),.5,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&event);QVERIFY(event.isAccepted());}QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        const auto preview=qEnvironmentVariable("DRAWVERSE_SMUDGE_PREVIEW");if(!preview.isEmpty())QVERIFY(window->grabWindow().save(preview));client.activateBrush();QVERIFY(!client.smudgeTool());QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void bucketOptionsMouseTabletAndFloatingCanvasFillOnce() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("bucket.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);
        QTest::keyClick(window,Qt::Key_G);QTRY_VERIFY(client.bucketTool());QVERIFY(!client.eraser() && !client.selectionTool() && !client.moveTool() && !client.eyedropperTool());
        QCOMPARE(client.bucketTolerance(),0);QCOMPARE(client.bucketOpacity(),1.);QVERIFY(client.bucketAntialias() && client.bucketContiguous() && !client.bucketAllLayers());
        auto *bar=findVisualItem(window->contentItem(),"bucketOptionsBar");QVERIFY(bar && bar->isVisible());
        auto *bucketIcon=findVisualItem(window->contentItem(),"bucketOptionsIcon");QVERIFY(bucketIcon && bucketIcon->isVisible());QCOMPARE(bucketIcon->property("name").toString(),QStringLiteral("bucket"));QCOMPARE(bucketIcon->width(),17.);QCOMPARE(bucketIcon->height(),17.);
        auto *statusTool=findVisualItem(window->contentItem(),"statusTool");QVERIFY(statusTool);QCOMPARE(statusTool->property("text").toString(),QStringLiteral("工具  油漆桶工具"));
        const auto preview=qEnvironmentVariable("DRAWVERSE_BUCKET_PREVIEW");if(!preview.isEmpty()){QTest::qWait(100);QVERIFY(window->grabWindow().save(preview));}
        auto toggle=[&](const QString &name){auto *item=findVisualItem(window->contentItem(),name);if(!item)return false;QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,item->mapToScene({7,item->height()/2}).toPoint());return true;};
        QVERIFY(toggle("bucketAntialias"));QVERIFY(!client.bucketAntialias());QVERIFY(toggle("bucketContiguous"));QVERIFY(!client.bucketContiguous());QVERIFY(toggle("bucketAllLayers"));QVERIFY(client.bucketAllLayers());
        QVERIFY(toggle("bucketAntialias"));QVERIFY(toggle("bucketContiguous"));QVERIFY(toggle("bucketAllLayers"));
        auto *opacity=findVisualItem(window->contentItem(),"bucketOpacityInput");auto *tolerance=findVisualItem(window->contentItem(),"bucketTolerance");QVERIFY(opacity && tolerance);
        opacity->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_4);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QCOMPARE(client.bucketOpacity(),.4);QCOMPARE(client.brushOpacity(),1.);
        QVERIFY(tolerance->setProperty("value",32));QVERIFY(QMetaObject::invokeMethod(tolerance,"valueModified"));QCOMPARE(client.bucketTolerance(),32);
        client.setBucketTolerance(-1);QCOMPARE(client.bucketTolerance(),32);client.setBucketTolerance(256);QCOMPARE(client.bucketTolerance(),32);client.setBucketOpacity(2);QCOMPARE(client.bucketOpacity(),.4);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        client.setBrushColor(Qt::red);QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({8,8}));QTest::mouseMove(window,at({40,40}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({40,40}));
        QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),1,10000);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(!client.drawing());
        QCOMPARE(client.historyEntries().last().toMap().value("kind").toInt(),25);QCOMPARE(client.historyEntries().last().toMap().value("title").toString(),QStringLiteral("油漆桶"));QCOMPARE(client.historyEntries().last().toMap().value("icon").toString(),QStringLiteral("bucket"));
        const auto pixel=[&](int x,int y){const auto r=client.frameRegion();const auto &f=client.frame();if(f.isNull() || r.isEmpty())return QColor();const int px=qFloor((x+.5-r.x())*f.width()/r.width()),py=qFloor((y+.5-r.y())*f.height()/r.height());return px>=0 && py>=0 && px<f.width() && py<f.height()?f.pixelColor(px,py):QColor();};
        QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(pixel(63,63).alpha(),102);
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(8,8).alpha(),0);
        client.activateEraser();QVERIFY(!client.bucketTool());client.activateBucket();QCOMPARE(client.bucketOpacity(),.4);QCOMPARE(client.bucketTolerance(),32);client.setBucketOpacity(1);client.setBucketTolerance(0);client.setBrushColor(Qt::blue);
        QPointingDevice pen("bucket pen",123,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto send=[&](QEvent::Type type,QPointF p){const auto local=at(p);QTabletEvent event(type,&pen,local,window->mapToGlobal(local),.7,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&event);return event.isAccepted();};
        QVERIFY(send(QEvent::TabletPress,{8,8}));QVERIFY(send(QEvent::TabletMove,{40,40}));QVERIFY(send(QEvent::TabletRelease,{40,40}));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QCOMPARE(pixel(63,63),QColor(Qt::blue));
        client.undo();QTRY_COMPARE(client.undoDepth(),0);QTRY_VERIFY(!client.layerEditBusy());
        QQuickWindow floating;floating.resize(200,180);auto *floatingCanvas=new CanvasItem(floating.contentItem());floatingCanvas->setSize({200,180});floatingCanvas->setClient(&client);floating.show();floatingCanvas->actualSize();QTest::qWait(80);
        auto point=floatingCanvas->mapToScene(floatingCanvas->documentRect().topLeft()+QPointF(8,8)*floatingCanvas->zoom()).toPoint();QTest::mouseClick(&floating,Qt::LeftButton,Qt::NoModifier,point);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        floatingCanvas->setClient(nullptr);floating.hide();QCOMPARE(warnings,QStringList());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void wandOptionsSelectRealMaskOnceAcrossMouseTabletAndFloatingCanvas() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("wand.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(64,64);QTRY_COMPARE(client.documentWidth(),64);QTRY_VERIFY(!client.layerEditBusy());
        // Paint a round island: its true mask outline must exclude the bounding-box corner.
        client.setBrushRadius(12);client.setBrushColor(Qt::red);QVERIFY(client.beginStroke({{32,32},1}));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);QTest::keyClick(window,Qt::Key_W);QTRY_COMPARE(client.selectionTool(),5);
        auto *bar=findVisualItem(window->contentItem(),"wandOptionsBar");QVERIFY(bar && bar->isVisible());QVERIFY(!findVisualItem(window->contentItem(),"selectionOptionsBar")->isVisible());QCOMPARE(client.wandSampleSize(),1);QCOMPARE(client.wandTolerance(),10);QCOMPARE(client.wandOperation(),1);QVERIFY(client.wandAntialias() && client.wandContiguous() && !client.wandAllLayers());
        auto *sample=findVisualItem(window->contentItem(),"wandSampleSize");QVERIFY(sample);QCOMPARE(sample->property("count").toInt(),7);QVERIFY(QMetaObject::invokeMethod(sample,"activated",Q_ARG(int,6)));QCOMPARE(client.wandSampleSize(),101);client.setWandSampleSize(1);client.setWandSampleSize(2);QCOMPARE(client.wandSampleSize(),1);
        auto *tolerance=findVisualItem(window->contentItem(),"wandTolerance");QVERIFY(tolerance);QVERIFY(tolerance->setProperty("value",0));QVERIFY(QMetaObject::invokeMethod(tolerance,"valueModified"));QCOMPARE(client.wandTolerance(),0);client.setWandTolerance(256);QCOMPARE(client.wandTolerance(),0);
        const auto click=[&](const QString &name){auto *item=findVisualItem(window->contentItem(),name);if(!item)return false;QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,item->mapToScene({7,item->height()/2}).toPoint());return true;};
        QVERIFY(click("wandAntialias"));QVERIFY(!client.wandAntialias());QVERIFY(click("wandAntialias"));QVERIFY(click("wandContiguous"));QVERIFY(!client.wandContiguous());QVERIFY(click("wandContiguous"));QVERIFY(click("wandAllLayers"));QVERIFY(client.wandAllLayers());QVERIFY(click("wandAllLayers"));
        QVERIFY(click("wandMode:1"));QCOMPARE(client.wandOperation(),1);QVERIFY(click("wandMode:0"));QCOMPARE(client.wandOperation(),0);QCOMPARE(client.selectionOperation(),0);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({32,32}));QTest::mouseMove(window,at({2,2}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({2,2}));QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.selectionEnabled());QCOMPARE(client.historyEntries().last().toMap().value("title").toString(),QStringLiteral("魔棒"));QVERIFY(!client.drawing());
        auto *outline=window->findChild<SelectionOverlay*>("selectionOutline");QVERIFY(outline);QTRY_VERIFY(outline->documentPath().contains({32.5,32.5}));const auto maskStep=client.selectionSteps().first().toMap();QVERIFY(!outline->documentPath().contains({maskStep.value("x").toDouble()+.5,maskStep.value("y").toDouble()+.5}));QVERIFY(maskStep.value("mask").toByteArray().size()>0);
        const auto preview=qEnvironmentVariable("DRAWVERSE_WAND_PREVIEW");if(!preview.isEmpty()){QTest::qWait(80);QVERIFY(window->grabWindow().save(preview));}
        client.undo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(!client.selectionEnabled());
        QPointingDevice pen("wand pen",124,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        for(const auto type:{QEvent::TabletPress,QEvent::TabletMove,QEvent::TabletRelease}) {const auto local=at({32,32});QTabletEvent event(type,&pen,local,window->mapToGlobal(local),.2,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&event);QVERIFY(event.isAccepted());}
        QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());client.undo();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        QQuickWindow floating;floating.resize(200,180);auto *floatingCanvas=new CanvasItem(floating.contentItem());floatingCanvas->setSize({200,180});floatingCanvas->setClient(&client);floating.show();floatingCanvas->actualSize();QTest::qWait(80);const auto point=floatingCanvas->mapToScene(floatingCanvas->documentRect().topLeft()+QPointF(32,32)*floatingCanvas->zoom()).toPoint();QTest::mouseClick(&floating,Qt::LeftButton,Qt::NoModifier,point);QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());
        floatingCanvas->setClient(nullptr);floating.hide();client.activateLasso();QCOMPARE(client.selectionTool(),3);client.activateWand();QCOMPARE(client.wandTolerance(),0);QCOMPARE(client.wandOperation(),0);QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void lassoPolygonAndMarqueeShareOptionsHistoryAndPersistence() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("selection.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        client.setSelectionTool(3);auto *options=findVisualItem(window->contentItem(),"selectionOptionsBar");QVERIFY(options);QTRY_VERIFY(options->isVisible());
        for(int i=0;i<4;++i){auto *button=findVisualItem(options,QString("selectionMode:%1").arg(i));QVERIFY(button);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,button->mapToScene({13,13}).toPoint());QCOMPARE(client.selectionOperation(),i);}
        auto *featherText=findVisualItem(options,"selectionFeatherText");QVERIFY(featherText);featherText->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_4);QTest::keyClick(window,Qt::Key_2);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(client.selectionFeather(),42.);
        auto *antialias=findVisualItem(options,"selectionAntialias");QVERIFY(antialias);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,antialias->mapToScene({6,6}).toPoint());QTRY_VERIFY(!client.selectionAntialias());
        client.setSelectionOperation(0);client.setSelectionFeather(3);canvas->forceActiveFocus();
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({16,16}));QTest::mouseMove(window,at({96,16}));QTest::mouseMove(window,at({16,96}));QVERIFY(canvas->selectionPreviewPoints().size()>=3);QCOMPARE(client.undoDepth(),0);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({16,16}));
        QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());auto step=client.selectionSteps().last().toMap();QCOMPARE(step.value("shape").toInt(),2);QCOMPARE(step.value("feather").toDouble(),3.);QVERIFY(!step.value("antialias").toBool());QCOMPARE(client.historyEntries().last().toMap().value("title").toString(),QString::fromUtf8("套索"));
        auto *outline=qobject_cast<SelectionOverlay*>(findVisualItem(window->contentItem(),"selectionOutline"));QVERIFY(outline);QTRY_VERIFY(outline->documentPath().contains({30,30}));QVERIFY(!outline->documentPath().contains({90,90}));
        auto *lassoButton=findVisualItem(window->contentItem(),"lassoTool");QVERIFY(lassoButton);
        QTest::mouseClick(window,Qt::RightButton,Qt::NoModifier,lassoButton->mapToScene({13,13}).toPoint());auto *variants=window->findChild<QObject*>("lassoVariantPanel");QVERIFY(variants);QTRY_VERIFY(variants->property("visible").toBool());auto *variantContent=qobject_cast<QQuickItem*>(variants->property("contentItem").value<QObject*>());QVERIFY(variantContent);auto *polygon=findVisualItem(variantContent,"variantTrigger:polygon");QVERIFY(polygon);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,polygon->mapToScene({20,15}).toPoint());QTRY_COMPARE(client.selectionTool(),4);QTRY_VERIFY(!variants->property("visible").toBool());
        canvas->forceActiveFocus();QCOMPARE(client.selectionFeather(),3.);QCOMPARE(client.selectionOperation(),0);QVERIFY(!client.selectionAntialias());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({32,32}));QVERIFY(!canvas->selectionPreviewPoints().isEmpty());
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({100,32}));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({100,100}));QCOMPARE(client.undoDepth(),1);
        QTest::keyClick(window,Qt::Key_Backspace);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({100,100}));QTest::keyClick(window,Qt::Key_Return);
        QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(canvas->selectionPreviewPoints().isEmpty());QCOMPARE(client.historyEntries().last().toMap().value("kind").toInt(),24);QCOMPARE(client.selectionSteps().last().toMap().value("points").toList().size(),3);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({16,16}));QTest::keyClick(window,Qt::Key_Escape);QVERIFY(canvas->selectionPreviewPoints().isEmpty());QCOMPARE(client.undoDepth(),2);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({16,16}));client.activateBrush();QVERIFY(canvas->selectionPreviewPoints().isEmpty());client.activateLasso();QCOMPARE(client.selectionTool(),4);
        client.setSelectionTool(1);QCOMPARE(client.selectionFeather(),3.);client.setSelectionOperation(1);QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({4,4}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({16,16}));QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());step=client.selectionSteps().last().toMap();QCOMPARE(step.value("operation").toInt(),1);QCOMPARE(step.value("feather").toDouble(),3.);
        const auto saved=client.selectionSteps();QSignalSpy files(&client,&PaintCoreClient::fileFinished);const auto path=QUrl::fromLocalFile(temp.filePath("lasso.ora"));QVERIFY(client.saveDocument(path));QTRY_COMPARE(files.size(),1);QVERIFY(files.last()[0].toBool());QVERIFY(client.clearSelection());QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.openDocument(path));QTRY_COMPARE(files.size(),2);QVERIFY(files.last()[0].toBool());QTRY_VERIFY(client.ready());QCOMPARE(client.selectionSteps(),saved);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void polygonDoubleClickClosureAndTabletFreehandUseSameInputPath() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("selection.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(80);
        const auto at=[&](QPointF p){return canvas->mapToScene(canvas->documentRect().topLeft()+p*canvas->zoom()).toPoint();};
        client.setSelectionTool(4);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({20,20}));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({100,20}));QTest::mouseDClick(window,Qt::LeftButton,Qt::NoModifier,at({20,100}));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().last().toMap().value("points").toList().size(),3);
        // Space changes only the viewport, never completes a partially collected polygon.
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({40,40}));const auto points=canvas->selectionPreviewPoints();QTest::keyPress(window,Qt::Key_Space);QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,at({60,60}));QTest::mouseMove(window,at({65,65}));QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,at({65,65}));QTest::keyRelease(window,Qt::Key_Space);QCOMPARE(client.undoDepth(),1);QCOMPARE(canvas->selectionPreviewPoints().size(),points.size());QTest::keyClick(window,Qt::Key_Escape);canvas->actualSize();
        client.setSelectionTool(3);QPointingDevice pen("test pen",321,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,1);
        const auto send=[&](QEvent::Type type,QPointF p){const QPointF local=at(p);QTabletEvent event(type,&pen,local,window->mapToGlobal(local.toPoint()),.7,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,type==QEvent::TabletRelease?Qt::NoButton:Qt::LeftButton);QCoreApplication::sendEvent(window,&event);return event.isAccepted();};
        QVERIFY(send(QEvent::TabletPress,{24,24}));QVERIFY(send(QEvent::TabletMove,{104,24}));QVERIFY(send(QEvent::TabletMove,{24,104}));QVERIFY(send(QEvent::TabletRelease,{24,24}));QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.historyEntries().last().toMap().value("kind").toInt(),21);
        client.setSelectionTool(4);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({20,20}));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({100,20}));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({20,100}));QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({20,20}));QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(canvas->selectionPreviewPoints().isEmpty());
        client.setSelectionTool(4);QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,at({20,20}));QTest::keyClick(window,Qt::Key_Return);QCOMPARE(client.undoDepth(),3);QVERIFY(canvas->selectionPreviewPoints().isEmpty());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void selectionsDragCombineUndoConstrainAndPersist() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("selection.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError> &errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());QVERIFY(!findVisualItem(window->contentItem(),"selectionOutline"));auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->forceActiveFocus();window->requestActivate();QTest::qWait(100);
        const auto position=[&](QPointF point){return canvas->mapToScene(canvas->documentRect().topLeft()+point*canvas->zoom()).toPoint();};
        QTest::keyClick(window,Qt::Key_M);QTRY_COMPARE(client.selectionTool(),1);
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,position({16,16}));QTest::mouseMove(window,position({80,80}));QVERIFY(!canvas->selectionPreview().isEmpty());QCOMPARE(client.undoDepth(),0);QVERIFY(!client.selectionEnabled());QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,position({80,80}));QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(client.selectionEnabled());QCOMPARE(client.selectionSteps().size(),1);
        QTest::keyClick(window,Qt::Key_M,Qt::ShiftModifier);QTRY_COMPARE(client.selectionTool(),2);
        QTest::mousePress(window,Qt::LeftButton,Qt::AltModifier,position({32,32}));QTest::mouseMove(window,position({64,64}));QTest::mouseRelease(window,Qt::LeftButton,Qt::AltModifier,position({64,64}));QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().last().toMap().value("operation").toInt(),2);QCOMPARE(client.selectionSteps().last().toMap().value("shape").toInt(),1);
        const auto outline=[&](){return qobject_cast<SelectionOverlay*>(findVisualItem(window->contentItem(),"selectionOutline"));};QVERIFY(outline());QTRY_VERIFY(outline() && outline()->documentPath().contains({24,48}));QVERIFY(!outline()->documentPath().contains({48,48}));QVERIFY(!outline()->documentPath().contains({100,48}));
        QTest::mousePress(window,Qt::LeftButton,Qt::ShiftModifier,position({96,16}));QTest::mouseMove(window,position({120,40}));QTest::mouseRelease(window,Qt::LeftButton,Qt::ShiftModifier,position({120,40}));QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().last().toMap().value("operation").toInt(),1);QTRY_VERIFY(outline() && outline()->documentPath().contains({108,28}));
        QTest::mousePress(window,Qt::LeftButton,Qt::ShiftModifier|Qt::AltModifier,position({8,8}));QTest::mouseMove(window,position({50,100}));QTest::mouseRelease(window,Qt::LeftButton,Qt::ShiftModifier|Qt::AltModifier,position({50,100}));QTRY_COMPARE(client.undoDepth(),4);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().last().toMap().value("operation").toInt(),3);client.undo();QTRY_COMPARE(client.undoDepth(),3);QTRY_VERIFY(!client.layerEditBusy());
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,position({8,8}));QTest::mouseMove(window,position({120,120}));QTest::keyClick(window,Qt::Key_Escape);QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,position({120,120}));QVERIFY(canvas->selectionPreview().isEmpty());QCOMPARE(client.undoDepth(),3);
        QTest::keyClick(window,Qt::Key_B);QTRY_COMPARE(client.selectionTool(),0);client.setBrushRadius(5);client.setBrushColor(Qt::black);InputSample sample;sample.position={8,48};QVERIFY(client.beginStroke(sample));sample.position={120,48};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),4);QTRY_VERIFY(!client.layerEditBusy());canvas->setClient(nullptr);client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frame().size()==QSize(128,128) && client.frameRevision()>=client.revision());QVERIFY(client.frame().pixelColor(24,48).alpha()>0);QCOMPARE(client.frame().pixelColor(48,48).alpha(),0);QCOMPARE(client.frame().pixelColor(100,48).alpha(),0);canvas->setClient(&client);canvas->actualSize();
        const auto savedSteps=client.selectionSteps();QSignalSpy files(&client,&PaintCoreClient::fileFinished);const auto path=QUrl::fromLocalFile(temp.filePath("selection.ora"));QVERIFY(client.saveDocument(path));QTRY_COMPARE(files.size(),1);QVERIFY(files.last()[0].toBool());QVERIFY(client.clearSelection());QTRY_VERIFY(!client.selectionEnabled());QTRY_VERIFY(!client.layerEditBusy());client.undo();QTRY_VERIFY(client.selectionEnabled());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps(),savedSteps);
        const auto generation=client.generation();QVERIFY(client.openDocument(path));QTRY_COMPARE(files.size(),2);QVERIFY(files.last()[0].toBool());QTRY_VERIFY(client.generation()>generation);QTRY_VERIFY(client.ready());QCOMPARE(client.selectionSteps(),savedSteps);QCOMPARE(client.undoDepth(),0);
        workspace.floatToolStrip(250,150);QTRY_COMPARE(applicationWindows().size(),2);QQuickWindow *floating=nullptr;for(auto *w:applicationWindows())if(w!=window)floating=qobject_cast<QQuickWindow*>(w);QVERIFY(floating);auto *tool=findVisualItem(floating->contentItem(),"selectionTool");QVERIFY(tool);QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,tool->mapToScene({tool->width()/2,tool->height()/2}).toPoint());QTRY_COMPARE(client.selectionTool(),2);
        window->requestActivate();canvas->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().size(),1);QTest::keyClick(window,Qt::Key_I,Qt::ControlModifier|Qt::ShiftModifier);QTRY_COMPARE(client.undoDepth(),2);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(outline() && outline()->documentPath().isEmpty());QVERIFY(client.selectionEnabled());
        QTest::keyClick(window,Qt::Key_D,Qt::ControlModifier);QTRY_VERIFY(!client.selectionEnabled());QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!findVisualItem(window->contentItem(),"selectionOutline"));
        client.setSelectionTool(2);QPointingDevice pen("Selection tablet",1001,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,QInputDevice::Capability::Position|QInputDevice::Capability::Pressure,1,2);
        const QPointF start=position({20,20}),end=position({100,100});QTabletEvent press(QEvent::TabletPress,&pen,start,window->mapToGlobal(start.toPoint()),.6,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);QCoreApplication::sendEvent(window,&press);QVERIFY(press.isAccepted());QTabletEvent release(QEvent::TabletRelease,&pen,end,window->mapToGlobal(end.toPoint()),0,0,0,0,0,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);QCoreApplication::sendEvent(window,&release);QVERIFY(release.isAccepted());QTRY_VERIFY(client.selectionEnabled());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(client.selectionSteps().size(),1);QCOMPARE(client.selectionSteps().first().toMap().value("shape").toInt(),1);
        const QPoint center=position({60,60});const auto before=canvas->documentPoint(canvas->mapFromScene(center));QWheelEvent wheel(center,window->mapToGlobal(center),{},QPoint(0,120),Qt::NoButton,Qt::NoModifier,Qt::NoScrollPhase,false);QCoreApplication::sendEvent(window,&wheel);QVERIFY(QLineF(before,canvas->documentPoint(canvas->mapFromScene(center))).length()<.001);QTRY_VERIFY(outline() && outline()->documentPath().contains({60,60}));QVERIFY(!outline()->documentPath().contains({20,20}));
        const auto preview=qEnvironmentVariable("DRAWVERSE_SELECTION_PREVIEW");if(!preview.isEmpty()){QTest::qWait(200);QVERIFY(window->grabWindow().save(preview));}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void toolsDockAlongWholeColumnAndStayResizableAfterPanelLeaves() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        const auto color=workspace.groupForPanel("color"),layers=workspace.groupForPanel("layers");workspace.detachGroup(color);QVERIFY(workspace.dockToolStrip("drawverse-tools-v1","floating",color,"left"));
        QQuickWindow *mixed=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+color)mixed=qobject_cast<QQuickWindow*>(w);return mixed!=nullptr;}());mixed->resize(420,550);QTest::qWait(30);
        workspace.detachGroup(color);QTRY_COMPARE(mixed->objectName(),QString("floatingToolStrip"));QTRY_COMPARE(mixed->width(),38);QCOMPARE(mixed->maximumWidth(),74);QCOMPARE(mixed->minimumWidth(),38);
        mixed->resize(74,600);QTRY_COMPARE(mixed->width(),74);QTRY_COMPARE(workspace.toolStripWidth(),72);mixed->resize(38,450);QTRY_COMPARE(workspace.toolStripWidth(),36);QTRY_COMPARE(mixed->height(),450);
        QVERIFY(workspace.dockPayload(QString::fromUtf8(QJsonDocument(QJsonObject{{"group",color},{"whole",true}}).toJson()),"right",layers,"after"));QCOMPARE(workspace.columnGroups(layers),QStringList({layers,color}));QTest::qWait(30);
        QVERIFY(workspace.dockToolStrip("drawverse-tools-v1","right",layers,"left"));QTRY_VERIFY(!workspace.toolsFloating());QTest::qWait(30);auto *tools=findVisualItem(main->contentItem(),"dockTile:__toolstrip"),*upper=findVisualItem(main->contentItem(),"dockTile:"+layers);QVERIFY(tools && upper);QCOMPARE(tools->y(),upper->y());QVERIFY(tools->height()>upper->height()+100);
        workspace.setToolStripWidth(999);QTRY_COMPARE(tools->width(),qreal(72));workspace.setToolStripWidth(1);QTRY_COMPARE(tools->width(),qreal(36));workspace.setToolStripWidth(55);QTRY_COMPARE(tools->width(),qreal(55));const auto preview=qEnvironmentVariable("DRAWVERSE_WORKSPACE_TOOLS_PREVIEW");if(!preview.isEmpty()){QTest::qWait(80);QVERIFY(main->grabWindow().save(preview+".main.png"));}workspace.saveLayout();WorkspaceManager restored(temp.filePath("layout.ini"));QCOMPARE(restored.toolStripWidth(),55);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void collapsedExpandButtonDragsWithoutExpandingAndPixelsStayInPlace() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();canvas->zoomBy(2);
        const auto origin=canvas->mapToScene(canvas->documentRect().topLeft());const auto zoom=canvas->zoom();const auto left=workspace.groupForPanel("brush"),right=workspace.groupForPanel("color");
        for(const auto &id:QStringList{right,left})for(const bool folded:{true,false,true}) {workspace.setColumnCollapsed(id,folded);QTRY_VERIFY(QLineF(origin,canvas->mapToScene(canvas->documentRect().topLeft())).length()<.01);QCOMPARE(canvas->zoom(),zoom);}
        QQuickWindow *frameWindow=nullptr;for(auto *w:QGuiApplication::allWindows())if(w->objectName()=="applicationOutlineWindow")frameWindow=qobject_cast<QQuickWindow*>(w);QVERIFY(frameWindow);
        auto *outline=findVisualItem(frameWindow->contentItem(),"applicationWindowOutline");QVERIFY(outline);auto stroke=QQmlProperty::read(outline,"border.color").value<QColor>();QCOMPARE(stroke.rgb(),QColor("#63666b").rgb());QVERIFY(std::abs(stroke.alphaF()-.5)<.01);QCOMPARE(frameWindow->geometry(),main->geometry().adjusted(-1,-1,1,1));
        QVERIFY(frameWindow->flags().testFlag(Qt::WindowTransparentForInput));main->showMaximized();QTRY_VERIFY(!frameWindow->isVisible());main->showNormal();QTRY_VERIFY(frameWindow->isVisible());QTest::qWait(50);

        auto *bar=findVisualItem(main->contentItem(),"brushOptionsBar");QVERIFY(bar);QCOMPARE(bar->height(),qreal(36));auto *documents=main->findChild<DocumentManager*>("documentManager");QVERIFY(documents);auto *tab=findVisualItem(main->contentItem(),"documentTab:"+documents->activeId());QVERIFY(tab);QCOMPARE(tab->property("radius").toReal(),qreal(10));QCOMPARE(tab->y(),qreal(3));QCOMPARE(tab->property("color").value<QColor>(),bar->property("color").value<QColor>());QCOMPARE(tab->mapToScene({0,0}).x()-findVisualItem(main->contentItem(),"documentTitleRow:main")->mapToScene({0,0}).x(),qreal(8));
        auto *grip=findVisualItem(main->contentItem(),"railExpandGrip:"+right);QVERIFY(grip);const auto at=grip->mapToScene({20,5}).toPoint();QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,at);QVERIFY(grip->property("pressed").toBool());QTest::mouseMove(main,at+QPoint(-80,30));QTRY_VERIFY(workspace.dragging());QVERIFY(workspace.groupDefinition(right).value("icons").toBool());QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+right)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());QTest::keyClick(floating,Qt::Key_Escape);QTRY_VERIFY(!workspace.dragging());QTest::mouseRelease(main,Qt::LeftButton,Qt::NoModifier,at);QTRY_VERIFY(workspace.floatingWindows().isEmpty());QVERIFY(workspace.groupDefinition(right).value("icons").toBool());
        grip=findVisualItem(main->contentItem(),"railExpandGrip:"+right);QVERIFY(grip);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,grip->mapToScene({20,5}).toPoint());QTRY_VERIFY(!workspace.groupDefinition(right).value("icons").toBool());QTRY_VERIFY(QLineF(origin,canvas->mapToScene(canvas->documentRect().topLeft())).length()<.01);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void colorClicksAndCapsLockControlBrushOutline() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.setBrushColor(QColor("#112233"));client.setSecondaryBrushColor(QColor("#abcdef"));auto *swatch=findVisualItem(main->contentItem(),"foregroundColorSwatch");QVERIFY(swatch);const auto click=swatch->mapToScene({12,12}).toPoint();QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,click);QCOMPARE(client.brushColor(),QColor("#abcdef"));QCOMPARE(client.secondaryBrushColor(),QColor("#112233"));
        QTest::mouseDClick(main,Qt::LeftButton,Qt::NoModifier,click);QVERIFY(!findVisualItem(main->contentItem(),"brushColorHex"));

        auto *canvas=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->forceActiveFocus();canvas->actualSize();client.setBrushRadius(15);QHoverEvent hover(QEvent::HoverMove,{100,100},canvas->mapToGlobal({100,100}),{90,90});QCoreApplication::sendEvent(canvas,&hover);const bool originallyVisible=canvas->brushCursorVisible();if(!originallyVisible)QTest::keyClick(main,Qt::Key_CapsLock);QTRY_VERIFY(canvas->brushCursorVisible());QCOMPARE(canvas->brushCursorRect(),QRectF(85,85,30,30));QCOMPARE(canvas->cursor().shape(),Qt::BlankCursor);auto *outline=findVisualItem(main->contentItem(),"brushCursorOutline");QVERIFY(outline && outline->isVisible());
        canvas->zoomBy(2);QCOMPARE(canvas->brushCursorRect().width(),qreal(60));client.setEraser(true);client.setBrushRadius(6);QCOMPARE(canvas->brushCursorRect().width(),qreal(24));client.setEraser(false);QCOMPARE(client.brushRadius(),qreal(6));QTest::keyClick(main,Qt::Key_CapsLock);QVERIFY(!canvas->brushCursorVisible());QCOMPARE(canvas->cursor().shape(),Qt::CrossCursor);QTest::keyClick(main,Qt::Key_CapsLock);QVERIFY(canvas->brushCursorVisible());client.setMoveTool(true);QVERIFY(!canvas->brushCursorVisible());QCOMPARE(canvas->cursor().shape(),Qt::SizeAllCursor);client.setEraser(false);QVERIFY(canvas->brushCursorVisible());if(!originallyVisible)QTest::keyClick(main,Qt::Key_CapsLock);
        const auto brushGroup=workspace.groupForPanel("brush");workspace.setActive(brushGroup,"brush");QQuickItem *preset=nullptr;QTRY_VERIFY((preset=findVisualItem(main->contentItem(),"brushPreset:round-fine")));QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,preset->mapToScene({20,20}).toPoint());preset->forceActiveFocus();QVERIFY(!canvas->hasActiveFocus());
        const auto oldOrigin=canvas->mapToScene(canvas->documentRect().topLeft());const auto center=canvas->mapToScene({canvas->width()/2,canvas->height()/2}).toPoint();QTest::keyPress(main,Qt::Key_Space);QTest::mousePress(main,Qt::LeftButton,Qt::NoModifier,center);QTest::mouseMove(main,center+QPoint(30,20));QTest::mouseRelease(main,Qt::LeftButton,Qt::NoModifier,center+QPoint(30,20));QTest::keyRelease(main,Qt::Key_Space);QTRY_VERIFY(QLineF(canvas->mapToScene(canvas->documentRect().topLeft()),oldOrigin+QPointF(30,20)).length()<.01);QCOMPARE(client.undoDepth(),0);
        canvas->fitToView();const auto outside=canvas->mapToScene(canvas->documentRect().bottomRight()+QPointF(4,6))*main->devicePixelRatio();QTRY_COMPARE(main->grabWindow().pixelColor(outside.toPoint()),QColor("#17191c"));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void topMenusUseNativeWindowsAboveFloatingPanels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());const auto group=workspace.groupForPanel("color");workspace.detachGroup(group);QQuickWindow *floating=nullptr;QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName()=="floatingDock:"+group)floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}());
        auto *menu=main->findChild<QObject*>("fileMenuPopup");QVERIFY(menu);QVERIFY(QMetaObject::invokeMethod(menu,"open"));QTRY_VERIFY(menu->property("opened").toBool());auto *content=qobject_cast<QQuickItem*>(menu->property("contentItem").value<QObject*>());QVERIFY(content);QQuickWindow *popup=nullptr;QTRY_VERIFY((popup=content->window()) && popup!=main);floating->raise();
#ifdef Q_OS_WIN
        if(workspace.windowsWindowFrames()) {QTRY_VERIFY((GetWindowLongPtr(reinterpret_cast<HWND>(popup->winId()),GWL_EXSTYLE)&WS_EX_TOPMOST)!=0);HWND front=nullptr;for(auto w=GetTopWindow(nullptr);w;w=GetWindow(w,GW_HWNDNEXT))if(w==reinterpret_cast<HWND>(popup->winId()) || w==reinterpret_cast<HWND>(floating->winId())){front=w;break;}QCOMPARE(front,reinterpret_cast<HWND>(popup->winId()));}
#endif
        QVERIFY(QMetaObject::invokeMethod(menu,"close"));QTRY_VERIFY(!menu->property("opened").toBool());QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void brushPresetsKeepIndependentSettingsAndRejectStalePreviews() {
        BrushLibrary library;QCOMPARE(library.radius(),qreal(12));library.setRadius(22);library.setOpacity(.4);library.setSpacing(.3);QVERIFY(library.select("round-fine"));QCOMPARE(library.radius(),qreal(3));QCOMPARE(library.opacity(),qreal(1));QVERIFY(library.select("round-pressure"));QCOMPARE(library.radius(),qreal(22));QCOMPARE(library.spacing(),qreal(.3));
        QSignalSpy requests(&library,&BrushLibrary::previewRequested);library.requestPreview(library.selectedId());QCOMPARE(requests.size(),1);QCOMPARE(requests.first()[2].toReal(),qreal(24));const auto oldToken=requests.first()[1].toULongLong();library.setRadius(24);QCOMPARE(requests.size(),1);library.setSpacing(.4);library.acceptPreview(library.selectedId(),oldToken,"stale");QVERIFY(library.selectedPreview().isEmpty());library.requestPreview(library.selectedId());QCOMPARE(requests.size(),2);library.acceptPreview(library.selectedId(),requests.last()[1].toULongLong(),"fresh");QCOMPARE(library.selectedPreview(),QString("fresh"));
        const auto copy=library.saveCopy(QStringLiteral("自定义圆笔"));QVERIFY(!copy.isEmpty());QCOMPARE(library.selectedId(),copy);QVERIFY(library.rename(copy,QStringLiteral("自定义圆笔二")));QVERIFY(!library.rename(copy," "));QVERIFY(!library.remove("round-pressure"));
        BrushLibrary restored;QVERIFY(restored.restore(library.snapshot()));QCOMPARE(restored.selectedId(),copy);QCOMPARE(restored.radius(),qreal(24));QCOMPARE(restored.opacity(),qreal(.4));QCOMPARE(restored.presets().size(),5);
        auto invalid=QJsonDocument::fromJson(library.snapshot()).object();auto rows=invalid.value("presets").toArray();auto row=rows.first().toObject();row["radius"]=-1;rows[0]=row;invalid["presets"]=rows;QVERIFY(!restored.restore(QJsonDocument(invalid).toJson()));QCOMPARE(restored.radius(),qreal(24));QVERIFY(!restored.restore("not json"));library.setRadius(std::numeric_limits<double>::quiet_NaN());QCOMPARE(library.radius(),qreal(24));
        library.setEraser(true);QCOMPARE(library.radius(),qreal(24));library.setRadius(40);QVERIFY(library.select("round-wide"));QCOMPARE(library.radius(),qreal(48));library.setEraser(false);QCOMPARE(library.radius(),qreal(48));QVERIFY(library.select(copy));QCOMPARE(library.radius(),qreal(40));
        BrushLibrary both;QVERIFY(both.restore(library.snapshot()));QCOMPARE(both.radius(),qreal(40));both.setEraser(true);QCOMPARE(both.radius(),qreal(40));
        auto legacy=QJsonDocument::fromJson(library.snapshot()).object();legacy.remove("toolRadii");BrushLibrary old;QVERIFY(old.restore(QJsonDocument(legacy).toJson()));
        QVERIFY(restored.remove(copy));QCOMPARE(restored.selectedId(),QString("round-pressure"));QCOMPARE(restored.presets().size(),4);QVERIFY(!restored.select("missing"));
        library.retryPreview();const auto failedToken=requests.last()[1].toULongLong();library.acceptPreview(library.selectedId(),failedToken,{});QCOMPARE(library.previewMessage(),QStringLiteral("预览暂不可用"));const auto count=requests.size();library.requestPreview(library.selectedId());QCOMPARE(requests.size(),count);library.retryPreview();QCOMPARE(requests.size(),count+1);
    }
    void brushLibrarySettingsAndEnginePreviewsShareSelectionAcrossWindows() {
        QTemporaryDir temp;const auto preferences=temp.filePath("storage.ini");PaintCoreClient client(nullptr,preferences);WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *library=client.brushLibrary();QVERIFY(library);const auto revision=client.revision();const int undo=client.undoDepth();const bool modified=client.modified();
        const auto group=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachPanel(group,"brush");QTRY_COMPARE(applicationWindows().size(),2);QQuickWindow *floating=nullptr;for(auto *w:applicationWindows())if(w!=window)floating=qobject_cast<QQuickWindow*>(w);QVERIFY(floating);floating->resize(190,500);
        QTRY_VERIFY2(findVisualItem(floating->contentItem(),"brushPreset:round-fine"),qPrintable(warnings.join('\n')));auto *fine=findVisualItem(floating->contentItem(),"brushPreset:round-fine");QTest::mouseClick(floating,Qt::LeftButton,Qt::NoModifier,fine->mapToScene({fine->width()/2,fine->height()/2}).toPoint());QTRY_COMPARE(library->selectedId(),QString("round-fine"));auto *label=findVisualItem(window->contentItem(),"selectedBrushName");QVERIFY(label);QTRY_COMPARE(label->property("text").toString(),QStringLiteral("细线圆笔"));
        auto *size=findVisualItem(window->contentItem(),"settingsBrushSizeText");QVERIFY(size);window->requestActivate();size->forceActiveFocus();QTRY_VERIFY(size->hasActiveFocus());QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,size->mapToScene({size->width()/2,size->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_4);QTest::keyClick(window,Qt::Key_0);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(client.brushRadius(),qreal(20));
        QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto image=[](const QString &url){QImage result;result.loadFromData(QByteArray::fromBase64(url.mid(url.indexOf(',')+1).toLatin1()));return result;};const auto opaque=image(library->selectedPreview());QVERIFY(!opaque.isNull());QCOMPARE(opaque.size(),QSize(240,60));QVERIFY(opaque.pixelColor(0,0).alpha()==0);int alpha=0;for(int y=0;y<60;++y)for(int x=0;x<240;++x)alpha+=opaque.pixelColor(x,y).alpha();QVERIFY(alpha>1000);
        const auto fixedPreview=library->selectedPreview();library->setRadius(100);QCOMPARE(library->selectedPreview(),fixedPreview);library->setRadius(20);QCOMPARE(library->selectedPreview(),fixedPreview);
        library->setOpacity(.15);QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);const auto transparent=image(library->selectedPreview());QVERIFY(!transparent.isNull());int fadedAlpha=0;for(int y=0;y<60;++y)for(int x=0;x<240;++x)fadedAlpha+=transparent.pixelColor(x,y).alpha();QVERIFY(fadedAlpha<alpha);QCOMPARE(client.revision(),revision);QCOMPARE(client.undoDepth(),undo);QCOMPARE(client.modified(),modified);
        library->setSpacing(.6);QCOMPARE(client.brushSpacing(),qreal(.6));QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);QVERIFY(image(library->selectedPreview())!=transparent);QVERIFY(library->select("round-pressure"));QCOMPARE(client.brushRadius(),qreal(20));QVERIFY(library->select("round-fine"));QCOMPARE(client.brushRadius(),qreal(20));QCOMPARE(client.brushOpacity(),qreal(.15));
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());client.setBrushColor(Qt::black);InputSample sample;sample.position={20,64};sample.pressure=.6f;sample.tool=1;QVERIFY(client.beginStroke(sample));sample.position={100,64};client.strokeTo(sample);client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());auto *previewCanvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(previewCanvas);previewCanvas->setClient(nullptr);client.requestViewport(0,{0,0,128,128},{128,128});QTRY_VERIFY(client.frameRevision()>=client.revision() && client.frame().size()==QSize(128,128));const int paintedAlpha=client.frame().pixelColor(60,64).alpha();QVERIFY(paintedAlpha>0 && paintedAlpha<250);previewCanvas->setClient(&client);
        const auto copy=library->saveCopy(QStringLiteral("保留的画笔"));QVERIFY(!copy.isEmpty());QTRY_COMPARE(label->property("text").toString(),QStringLiteral("保留的画笔"));
        const auto preview=qEnvironmentVariable("DRAWVERSE_BRUSH_PREVIEW");if(!preview.isEmpty()){QTRY_VERIFY_WITH_TIMEOUT(!library->selectedPreview().isEmpty(),10000);floating->resize(250,560);QTest::qWait(250);QVERIFY(floating->grabWindow().save(preview+".library.png"));auto settingsGroup=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(settingsGroup);QTRY_COMPARE(applicationWindows().size(),3);for(auto *w:applicationWindows())if(auto *q=qobject_cast<QQuickWindow*>(w))if(findVisualItem(q->contentItem(),"selectedBrushName")){q->resize(280,440);QTest::qWait(200);QVERIFY(q->grabWindow().save(preview+".settings.png"));}}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);PaintCoreClient reloaded(nullptr,preferences);QTRY_VERIFY(reloaded.ready());QTRY_COMPARE(reloaded.brushLibrary()->selectedId(),copy);QCOMPARE(reloaded.brushRadius(),qreal(20));QCOMPARE(reloaded.brushSpacing(),qreal(.6));QSignalSpy stoppedAgain(&reloaded,&PaintCoreClient::stopped);reloaded.shutdown();QTRY_COMPARE(stoppedAgain.size(),1);
    }
    void toolStripFloatsDocksPersistsAndTabsJoinContent() {
        QTemporaryDir temp;const auto path=temp.filePath("tools.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());QCOMPARE(applicationWindows().size(),1);
        const auto group=workspace.rightGroups().last().toMap().value("id").toString();auto *panel=findVisualItem(window->contentItem(),"dockGroup:"+group);QVERIFY(panel);
        auto *selected=findVisualItem(panel,"panelTab:layers");auto *inactive=findVisualItem(panel,"panelTab:history");QVERIFY(selected && inactive);
        QCOMPARE(selected->property("color"),panel->property("color"));QVERIFY(inactive->property("color").value<QColor>().lightness()<selected->property("color").value<QColor>().lightness());
        workspace.setActive(group,"history");QTRY_COMPARE(inactive->property("color"),panel->property("color"));QVERIFY(selected->property("color").value<QColor>().lightness()<inactive->property("color").value<QColor>().lightness());workspace.setActive(group,"layers");
        QVERIFY(!workspace.dockToolStrip("invalid"));QVERIFY(!workspace.toolsFloating());
        auto *grip=findVisualItem(window->contentItem(),"toolStripGrip");QVERIFY(grip);QTest::mouseDClick(window,Qt::LeftButton,Qt::NoModifier,grip->mapToScene({grip->width()/2,grip->height()/2}).toPoint());QTRY_VERIFY(workspace.toolsFloating());QTRY_COMPARE(applicationWindows().size(),2);
        QQuickWindow *tools=nullptr;for(auto *w:applicationWindows())if(w->objectName()=="floatingToolStrip")tools=qobject_cast<QQuickWindow*>(w);QVERIFY(tools);QCOMPARE(tools->width(),38);
        // Floating recreates the strip; the docked grip can already be destroyed.
        auto *floatingGrip=findVisualItem(tools->contentItem(),"toolStripGrip");QVERIFY(floatingGrip);
        for(auto *object:floatingGrip->findChildren<QObject*>())QVERIFY(!object->property("text").toString().contains(QStringLiteral("拖动工具条")));
        auto *eraser=findVisualItem(tools->contentItem(),"eraserTool");QVERIFY(eraser);QTest::mouseClick(tools,Qt::LeftButton,Qt::NoModifier,eraser->mapToScene({13,13}).toPoint());QTRY_VERIFY(client.eraser());
        auto *brush=findVisualItem(tools->contentItem(),"brushTool");QVERIFY(brush);QTest::mouseClick(tools,Qt::LeftButton,Qt::NoModifier,brush->mapToScene({13,13}).toPoint());QTRY_VERIFY(!client.eraser());
        tools->setPosition(window->x()+70,window->y()+130);QTRY_COMPARE(workspace.toolStripY(),tools->y());workspace.saveLayout();WorkspaceManager restored(path);QVERIFY(restored.toolsFloating());QCOMPARE(restored.toolStripX(),tools->x());QCOMPARE(restored.toolStripY(),tools->y());
        auto drop=[&](Qt::KeyboardModifiers modifiers,const QByteArray &data){QMimeData mime;mime.setData("application/x-drawverse-tool-strip",data);const QPoint point(12,100);QDragEnterEvent enter(point,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(window,&enter);QDragMoveEvent move(point,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(window,&move);QDropEvent end(point,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(window,&end);return end.isAccepted();};
#ifdef Q_OS_MACOS
        const auto suppress=Qt::MetaModifier;
#else
        const auto suppress=Qt::ControlModifier;
#endif
        QVERIFY(!drop(suppress,"drawverse-tools-v1"));QVERIFY(workspace.toolsFloating());QVERIFY(!drop(Qt::NoModifier,"bad"));QVERIFY(workspace.toolsFloating());QVERIFY(drop(Qt::NoModifier,"drawverse-tools-v1"));QTRY_VERIFY(!workspace.toolsFloating());QTRY_COMPARE(applicationWindows().size(),1);
        workspace.floatToolStrip(window->x()+90,window->y()+150);QTRY_COMPARE(applicationWindows().size(),2);for(auto *w:applicationWindows())if(w->objectName()=="floatingToolStrip")w->close();QTRY_VERIFY(!workspace.toolsFloating());QTRY_COMPARE(applicationWindows().size(),1);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void defaultLayoutRestoresTwoDockedColumnsAndMenuEntry() {
        QTemporaryDir temp;const auto path=temp.filePath("default.ini");PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(path);QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        QCOMPARE(workspace.leftGroups().size(),1);QCOMPARE(workspace.rightGroups().size(),2);QVERIFY(workspace.floatingWindows().isEmpty());
        workspace.detachGroup(workspace.leftGroups().first().toMap().value("id").toString());QTRY_COMPARE(applicationWindows().size(),2);
        QObject *action=nullptr;for(auto *object:window->findChildren<QObject*>()) {
            const auto text=object->property("text").toString();QVERIFY(text!=QStringLiteral("参考图布局"));QVERIFY(text!=QStringLiteral("双列停靠布局"));
            if(object->objectName()=="defaultLayoutAction")action=object;
        }
        QVERIFY(action);QVERIFY(QMetaObject::invokeMethod(action,"triggered"));QTRY_COMPARE(applicationWindows().size(),1);
        QCOMPARE(workspace.leftGroups().size(),1);QCOMPARE(workspace.rightGroups().size(),2);QVERIFY(!workspace.leftCollapsed());QVERIFY(!workspace.rightCollapsed());
        workspace.saveLayout();WorkspaceManager restored(path);QCOMPARE(restored.leftGroups().size(),1);QCOMPARE(restored.rightGroups().size(),2);QVERIFY(restored.floatingWindows().isEmpty());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void workspacePlacementPersistenceAndHiddenPanels() {
        QTemporaryDir temp;const auto path=temp.filePath("workspace-v3.ini");WorkspaceManager workspace(path);
        const auto first=workspace.rightGroups().first().toMap().value("id").toString();
        const auto second=workspace.rightGroups().last().toMap().value("id").toString();
        auto data=[](const QString &group,const QString &panel,bool whole=false){return QString::fromUtf8(QJsonDocument(QJsonObject{{"group",group},{"panel",panel},{"whole",whole}}).toJson());};
        auto tabs=[&](const QString &group){return workspace.groupDefinition(group).value("panels").toStringList();};
        const auto custom=workspace.addCustomPanel("Custom colors","palette");QVERIFY(!custom.isEmpty());
        QVERIFY(workspace.dockPayload(data(first,custom),"right",first,"merge","color"));QCOMPARE(tabs(first).first(),custom);
        QVERIFY(!workspace.dockPayload(data(first,custom),"left",first));QCOMPARE(tabs(first).first(),custom);
        QVERIFY(!workspace.dockPayload(data(first,custom),"right",first,"merge","unknown"));
        QVERIFY(workspace.dockPayload(data(first,custom),"right",second,"before"));
        QCOMPARE(workspace.rightGroups().size(),3);const auto inserted=workspace.rightGroups()[1].toMap().value("id").toString();QCOMPARE(tabs(inserted),QStringList{custom});
        QVERIFY(workspace.dockPayload(data(inserted,{},true),"right",second,"after"));QCOMPARE(workspace.rightGroups().last().toMap().value("id").toString(),inserted);
        QVERIFY(workspace.dockPayload(data(inserted,custom),"right",second,"merge","history"));QVERIFY(tabs(second).indexOf(custom)<tabs(second).indexOf("history"));
        QSignalSpy structural(&workspace,&WorkspaceManager::groupsChanged);
        workspace.setActive(second,"history");QCOMPARE(structural.size(),0);workspace.setColumnCollapsed(second,true);
        workspace.setLeftDockWidth(274);workspace.setRightDockWidth(354);workspace.setRightCollapsed(true);workspace.updateDockHeight(first,279);
        workspace.hidePanel(second,custom);QVERIFY(!workspace.visiblePanels().contains(custom));QVERIFY(workspace.allPanels().contains(custom));
        workspace.saveLayout();WorkspaceManager reopened(path);QCOMPARE(reopened.leftDockWidth(),274);QCOMPARE(reopened.rightDockWidth(),354);QVERIFY(reopened.rightCollapsed());
        QVERIFY(reopened.groupDefinition(second).value("icons").toBool());QCOMPARE(reopened.groupDefinition(second).value("active").toString(),QString("history"));
        QCOMPARE(reopened.groupDefinition(first).value("dockHeight").toInt(),279);QVERIFY(!reopened.visiblePanels().contains(custom));
        reopened.showPanel(custom);QVERIFY(reopened.visiblePanels().contains(custom));QVERIFY(reopened.rightCollapsed());
        const auto left=reopened.leftGroups().first().toMap().value("id").toString();reopened.detachGroup(left);QCOMPARE(reopened.floatingGroups().size(),1);reopened.returnGroup(left);QCOMPARE(reopened.leftGroups().size(),1);
        QStringList visible;for(const auto &v:reopened.leftGroups()+reopened.rightGroups()+reopened.floatingGroups())visible.append(v.toMap().value("panels").toStringList());
        QSet<QString> seen;for(const auto &id:visible){QVERIFY(!seen.contains(id));seen.insert(id);}QCOMPARE(seen.size(),7);
    }
    void referenceWorkspaceDockingAndColorPicker() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("ui-layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY(client.ready());
        auto *logo=findVisualItem(window->contentItem(),"applicationLogo");QVERIFY(logo);QTRY_COMPARE(logo->property("status").toInt(),1);
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);QTRY_VERIFY(canvas->width()>650);
        auto *radiusInput=findVisualItem(window->contentItem(),"brushRadiusText");QVERIFY(radiusInput);QVERIFY(radiusInput->width()>=28);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,radiusInput->mapToScene({radiusInput->width()/2,radiusInput->height()/2}).toPoint());QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);QTest::keyClick(window,Qt::Key_3);QTest::keyClick(window,Qt::Key_7);QTest::keyClick(window,Qt::Key_Return);QTRY_COMPARE(client.brushRadius(),qreal(18.5));
        auto *wheel=qobject_cast<ColorWheelItem*>(findVisualItem(window->contentItem(),"colorWheel"));QVERIFY(wheel);QTRY_VERIFY(wheel->height()>80);
        client.setBrushColor(Qt::red);QTRY_COMPARE(wheel->color(),QColor(Qt::red));
        const qreal radius=(qMin(wheel->width(),wheel->height())-6)/2;const QPointF center(wheel->width()/2,wheel->height()/2);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,wheel->mapToScene(center+QPointF(-.4625*radius,-.801*radius)).toPoint());QVERIFY(client.brushColor().greenF()>.9 && client.brushColor().redF()<.1);
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,wheel->mapToScene(center+QPointF(-.39*radius,-.67*radius)).toPoint());QVERIFY(client.brushColor().valueF()>.95 && client.brushColor().hsvSaturationF()<.1);
        const auto preview=qEnvironmentVariable("DRAWVERSE_WORKSPACE_PREVIEW");
        auto capture=[&](const QString &suffix){QTest::mouseMove(window,{20,window->height()-20});QTest::qWait(120);return preview.isEmpty() || window->grabWindow().save(preview+suffix+".png");};
        QVERIFY(capture(".main"));
        window->resize(980,700);QTest::qWait(100);auto *compactList=findVisualItem(window->contentItem(),"layerList");QVERIFY(compactList);QVERIFY(compactList->height()>=60);QVERIFY(canvas->width()>=400);QVERIFY(capture(".compact"));window->resize(1480,940);QTest::qWait(100);
        auto first=workspace.rightGroups().first().toMap().value("id").toString();auto second=workspace.rightGroups().last().toMap().value("id").toString();
        auto payload=[](const QString &group,const QString &panel){return QJsonDocument(QJsonObject{{"group",group},{"panel",panel},{"whole",false}}).toJson();};
        auto drop=[&](const QString &source,const QString &panel,const QString &target,qreal x,qreal y,Qt::KeyboardModifiers modifiers=Qt::NoModifier,QQuickWindow *host=nullptr){
            if(!host)host=window;
            auto *item=findVisualItem(host->contentItem(),"dockGroup:"+target);if(!item)return false;
            const auto at=item->mapToScene({x<0?item->width()/2:x,y<0?item->height()-4:y}).toPoint();QMimeData mime;mime.setData("application/x-drawverse-panel",payload(source,panel));
            QDragEnterEvent enter(at,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(host,&enter);if(!enter.isAccepted()){QDragLeaveEvent leave;QCoreApplication::sendEvent(host,&leave);return false;}
            QDragMoveEvent move(at,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(host,&move);
            auto *hint=findVisualItem(host->contentItem(),"dockPreview:"+target);if(!hint || !hint->isVisible())return false;
            if(!preview.isEmpty()){QTest::qWait(100);if(!host->grabWindow().save(preview+".drop.png"))return false;}
            QDropEvent event(at,Qt::MoveAction,&mime,Qt::LeftButton,modifiers);QCoreApplication::sendEvent(host,&event);return event.isAccepted();
        };
        QVERIFY(drop(second,"history",first,-1,18));QTRY_COMPARE(workspace.groupDefinition(first).value("panels").toStringList(),(QStringList{"color","history"}));
        QTest::qWait(50);QVERIFY(drop(first,"history",first,8,20));QTRY_COMPARE(workspace.groupDefinition(first).value("panels").toStringList().first(),QString("history"));
        QTest::qWait(50);QVERIFY(drop(first,"history",second,-1,4));QTRY_COMPARE(workspace.rightGroups().size(),3);const auto middle=workspace.rightGroups()[1].toMap().value("id").toString();
        QTest::qWait(50);QVERIFY(drop(middle,"history",second,-1,-1));QTRY_COMPARE(workspace.rightGroups().last().toMap().value("panels").toStringList(),QStringList{"history"});
        QTest::qWait(50);QVERIFY(!drop(first,"color",second,-1,80,Qt::ControlModifier));QCOMPARE(workspace.rightGroups().size(),3);
        workspace.resetLayout();QTest::qWait(80);first=workspace.rightGroups().first().toMap().value("id").toString();
        auto click=[&](const QString &name){auto *item=findVisualItem(window->contentItem(),name);if(!item)return false;QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,item->mapToScene({item->width()/2,item->height()/2}).toPoint());return true;};
        QVERIFY(!findVisualItem(window->contentItem(),"groupCollapse:"+first));
        QVERIFY(click("dockCollapse:right"));QTRY_VERIFY(findVisualItem(window->contentItem(),"railPanel:layers"));QTRY_VERIFY(findVisualItem(window->contentItem(),"railPanel:layers")->isVisible());QVERIFY(capture(".collapsed"));
        QVERIFY(click("railPanel:layers"));QQuickWindow *peek=nullptr;
        QTRY_VERIFY([&]{for(auto *w:applicationWindows())if(w->objectName().startsWith("panelFlyout:"))peek=qobject_cast<QQuickWindow*>(w);return peek!=nullptr;}());
        QTRY_VERIFY(findVisualItem(peek->contentItem(),"layerList"));
        QVERIFY(click("dockCollapse:right"));QTRY_VERIFY(!findVisualItem(window->contentItem(),"railPanel:layers"));QTRY_COMPARE(applicationWindows().size(),1);
        workspace.hidePanel(first,"color");QTRY_VERIFY(!workspace.visiblePanels().contains("color"));workspace.showPanel("color");QTRY_VERIFY(workspace.visiblePanels().contains("color"));
        const auto brush=workspace.leftGroups().first().toMap().value("id").toString();workspace.detachGroup(brush);QTRY_COMPARE(applicationWindows().size(),2);
        QQuickWindow *floating=nullptr;for(auto *w:applicationWindows())if(w!=window)floating=qobject_cast<QQuickWindow*>(w);QVERIFY(floating);
        if(!preview.isEmpty()){QTest::qWait(150);QVERIFY(floating->grabWindow().save(preview+".floating.png"));}
        QPointer<QQuickWindow> retained=floating;
        const auto custom=workspace.addCustomPanel("Saved colors","palette");QVERIFY(!custom.isEmpty());QTest::qWait(50);QVERIFY(retained);QCOMPARE(applicationWindows().size(),2);
        second=workspace.rightGroups().last().toMap().value("id").toString();QVERIFY(drop(second,"navigator",brush,-1,18,Qt::NoModifier,floating));QTRY_VERIFY(workspace.groupDefinition(brush).value("panels").toStringList().contains("navigator"));QVERIFY(retained);QCOMPARE(applicationWindows().size(),2);
        const auto railHeight=floating->height();workspace.setColumnCollapsed(brush,true);QTRY_COMPARE(floating->height(),railHeight);workspace.setColumnCollapsed(brush,false);QTRY_VERIFY(floating->height()>=200);
        floating->close();QTRY_COMPARE(workspace.leftGroups().size(),1);QTRY_COMPARE(applicationWindows().size(),1);
        QTRY_COMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void inlineMasksClippingAltClicksAndOpaquePreviewsWorkInFloatingPanels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        auto layer=[&](quint64 id){for(const auto &v:client.layers())if(v.toMap().value("id").toULongLong()==id)return v.toMap();return QVariantMap();};
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);
        client.newDocument(641,479);QTRY_COMPARE(client.documentWidth(),641);QTRY_VERIFY(client.ready());
        QTRY_VERIFY_WITH_TIMEOUT(!layer(1).value("thumbnail").toString().isEmpty(),10000);
        const auto encoded=layer(1).value("thumbnail").toString().section(',',1).toLatin1();const auto image=QImage::fromData(QByteArray::fromBase64(encoded));QVERIFY(!image.isNull());
        for(int y=0;y<image.height();++y)for(int x=0;x<image.width();++x)QCOMPARE(image.pixelColor(x,y),QColor(Qt::white));
        auto *thumb=findVisualItem(window->contentItem(),"layerThumbnail:1");auto *checker=findVisualItem(window->contentItem(),"thumbnailTransparency:1");QVERIFY(thumb && checker);QTRY_VERIFY(thumb->property("paintedWidth").toReal()>0);QCOMPARE(checker->width(),thumb->property("paintedWidth").toReal());QCOMPARE(checker->height(),thumb->property("paintedHeight").toReal());
        const auto opaquePreview=qEnvironmentVariable("DRAWVERSE_CLIPPING_PREVIEW");if(!opaquePreview.isEmpty()){QTRY_VERIFY(client.frameRevision()>=client.revision());QTest::qWait(100);QVERIFY(window->grabWindow().save(opaquePreview+".opaque.png"));}
        client.newTransparentDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());canvas->actualSize();
        InputSample point;point.position={64.5,64.5};client.setBrushColor(Qt::white);client.setBrushRadius(25);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(!client.layerEditBusy());
        client.addDefaultLayer();QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto top=client.activeLayer();client.setBrushColor(QColor("#d67065"));client.setBrushRadius(60);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());
        auto sample=[&](qreal x,qreal y){const auto r=client.frameRegion();const auto &f=client.frame();if(f.isNull() || r.isEmpty())return QColor();const int px=qFloor((x-r.x())*f.width()/r.width()),py=qFloor((y-r.y())*f.height()/r.height());return px>=0 && py>=0 && px<f.width() && py<f.height()?f.pixelColor(px,py):QColor();};
        QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(sample(100.5,64.5).alpha(),255);
        auto altClick=[&](QQuickWindow *w,quint64 id){auto *b=findVisualItem(w->contentItem(),"clippingBoundary:"+QString::number(id));if(!b || !b->isEnabled())return false;QTest::mouseClick(w,Qt::LeftButton,Qt::AltModifier,b->mapToScene({b->width()*0.6,b->height()/2}).toPoint());return true;};
        QVERIFY(altClick(window,top));QTRY_VERIFY(layer(top).value("clipped").toBool());QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(top).value("clipBase").toULongLong(),quint64(1));QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(sample(100.5,64.5).alpha(),0);QCOMPARE(sample(64.5,64.5).alpha(),255);
        auto *arrow=findVisualItem(window->contentItem(),"clippingArrow:"+QString::number(top));QVERIFY(arrow);QVERIFY(arrow->isVisible());QVERIFY(findVisualItem(window->contentItem(),"layerName:1")->property("font").value<QFont>().underline());
        auto clickLowerEdge=[&](){auto *r=findVisualItem(window->contentItem(),"layerRow:1");if(!r)return false;QTest::mouseClick(window,Qt::LeftButton,Qt::AltModifier,r->mapToScene({r->width()*0.6,2}).toPoint());return true;};
        QVERIFY(clickLowerEdge());QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!layer(top).value("clipped").toBool());QVERIFY(clickLowerEdge());QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(layer(top).value("clipped").toBool());
        client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!layer(top).value("clipped").toBool());client.redo();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(layer(top).value("clipped").toBool());
        QVERIFY(client.addMask(1));QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());const auto mask=client.activeLayer();
        auto *list=findVisualItem(window->contentItem(),"layerList");QVERIFY(list);QTRY_COMPARE(list->property("count").toInt(),2);QVERIFY(!findVisualItem(window->contentItem(),"layerRow:"+QString::number(mask)));
        auto *raw=findVisualItem(window->contentItem(),"layerThumbnail:1");auto *maskThumb=findVisualItem(window->contentItem(),"layerThumbnail:"+QString::number(mask));QVERIFY(raw && maskThumb);
        // Delegate creation precedes the asynchronous Row layout, especially at high DPI.
        QTRY_VERIFY(maskThumb->mapToScene({0,0}).x()>raw->mapToScene({raw->width(),0}).x());QTRY_COMPARE(maskThumb->mapToScene({0,0}).y(),raw->mapToScene({0,0}).y());
        auto hit=[&](QQuickWindow *w,quint64 id,Qt::KeyboardModifiers modifiers=Qt::NoModifier){QTest::qWait(50);auto *i=findVisualItem(w->contentItem(),"layerThumbnailHit:"+QString::number(id));if(!i || !i->isVisible())return false;QTest::mouseClick(w,Qt::LeftButton,modifiers,i->mapToScene({i->width()/2,i->height()/2}).toPoint());return true;};
        QTRY_VERIFY(list->height()>=46);
        QVERIFY(hit(window,1));QTRY_COMPARE(client.activeLayer(),quint64(1));QTRY_VERIFY(!client.layerEditBusy());QVERIFY(hit(window,mask));QTRY_COMPARE(client.activeLayer(),mask);QTRY_VERIFY(!client.layerEditBusy());
        client.setBrushColor(Qt::black);client.setBrushRadius(9);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(sample(64.5,64.5).alpha(),0);
        QVERIFY(hit(window,mask,Qt::ShiftModifier));QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!layer(mask).value("visible").toBool());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(sample(64.5,64.5).alpha(),255);QVERIFY(hit(window,mask,Qt::ShiftModifier));QTRY_VERIFY(!client.layerEditBusy());
        const auto dock=workspace.rightGroups().last().toMap().value("id").toString();workspace.detachPanel(dock,"layers");QTRY_COMPARE(workspace.floatingGroups().size(),1);QQuickWindow *floating=nullptr;QTRY_VERIFY(([&](){for(auto *w:applicationWindows())if(w!=window){floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}return false;})());QTRY_VERIFY(findVisualItem(floating->contentItem(),"clippingBoundary:"+QString::number(top)));
        QVERIFY(altClick(floating,top));QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(!layer(top).value("clipped").toBool());QVERIFY(altClick(floating,top));QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(layer(top).value("clipped").toBool());QVERIFY(hit(floating,mask));QTRY_COMPARE(client.activeLayer(),mask);QTRY_VERIFY(!client.layerEditBusy());
        const auto preview=qEnvironmentVariable("DRAWVERSE_CLIPPING_PREVIEW");if(!preview.isEmpty()){floating->resize(360,700);canvas->fitToView();QTest::mouseMove(floating,{24,floating->height()-100});QTest::qWait(500);QVERIFY(window->grabWindow().save(preview));QVERIFY(floating->grabWindow().save(preview+".layers.png"));}
        const auto path=QUrl::fromLocalFile(temp.filePath("clipping.ora"));QSignalSpy files(&client,&PaintCoreClient::fileFinished);QVERIFY(client.saveDocument(path));QTRY_COMPARE_WITH_TIMEOUT(files.size(),1,10000);QVERIFY(files.first().first().toBool());client.newDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());QVERIFY(client.openDocument(path));QTRY_COMPARE_WITH_TIMEOUT(files.size(),2,10000);QVERIFY(files.last().first().toBool());QTRY_VERIFY(client.ready());int clips=0;for(const auto &v:client.layers())clips+=v.toMap().value("clipped").toBool()?1:0;QCOMPARE(clips,1);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void masksDefaultsDepthAndDragDropWorkAcrossFloatingPanels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors)warnings.append(error.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        auto layer=[&](quint64 id){for(const auto &v:client.layers())if(v.toMap().value("id").toULongLong()==id)return v.toMap();return QVariantMap();};
        QCOMPARE(layer(1).value("name").toString(),QStringLiteral("背景"));QCOMPARE(client.undoDepth(),0);
        client.newDocument(128,128);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(client.ready());QTRY_VERIFY(client.frameRevision()>=client.revision() && !client.frame().isNull());QTRY_COMPARE(client.frame().pixelColor(client.frame().width()/2,client.frame().height()/2),QColor(Qt::white));
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas");QVERIFY(canvas);canvas->actualSize();
        auto centerColor=[&](){const auto region=client.frameRegion();const auto &frame=client.frame();if(frame.isNull() || region.isEmpty())return QColor();const int x=qFloor((64.5-region.x())*frame.width()/region.width());const int y=qFloor((64.5-region.y())*frame.height()/region.height());if(x<0 || y<0 || x>=frame.width() || y>=frame.height())return QColor();return frame.pixelColor(x,y);};
        auto *maskButton=findVisualItem(window->contentItem(),"addLayerMask");QVERIFY(maskButton);QVERIFY(QMetaObject::invokeMethod(maskButton,"clicked"));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());const auto mask=client.activeLayer();QVERIFY(layer(mask).value("mask").toBool());QCOMPARE(layer(mask).value("depth").toInt(),1);
        auto *inlineMask=findVisualItem(window->contentItem(),"inlineMask:1");QVERIFY(inlineMask);QVERIFY(inlineMask->isVisible());QVERIFY(!findVisualItem(window->contentItem(),"layerRow:"+QString::number(mask)));
        client.setBrushRadius(12);client.setBrushColor(Qt::black);InputSample point;point.position={64.5,64.5};const auto depth=client.undoDepth();QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(centerColor().alpha(),0);
        client.setBrushColor(Qt::white);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),depth+2);QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(centerColor(),QColor(Qt::white));client.undo();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(centerColor().alpha(),0);
        client.setBrushColor(QColor("#808080"));QVERIFY(client.beginStroke(point));client.endStroke();QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(centerColor().alpha(),128);client.undo();QTRY_VERIFY(!client.layerEditBusy());
        client.setLayerProperties(mask,false,1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTRY_COMPARE(centerColor(),QColor(Qt::white));client.setLayerProperties(mask,true,1);QTRY_VERIFY(!client.layerEditBusy());
        QQuickItem *opacity=nullptr;QTRY_VERIFY((opacity=findVisualItem(window->contentItem(),"layerOpacity")));QVERIFY(QMetaObject::invokeMethod(opacity,"committed",Q_ARG(double,.5)));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(1).value("opacity").toFloat(),.5f);QCOMPARE(layer(mask).value("opacity").toFloat(),1.f);client.undo();QTRY_VERIFY(!client.layerEditBusy());
        auto *density=findVisualItem(window->contentItem(),"maskDensity");QVERIFY(density);QVERIFY(QMetaObject::invokeMethod(density,"committed",Q_ARG(double,.5)));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(mask).value("opacity").toFloat(),.5f);QCOMPARE(layer(1).value("opacity").toFloat(),1.f);client.undo();QTRY_VERIFY(!client.layerEditBusy());
        client.selectLayer(1);QTRY_VERIFY(!client.layerEditBusy());client.groupLayer(1,"绘画组");QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());const auto group=client.activeLayer();QCOMPARE(layer(mask).value("depth").toInt(),2);
        auto *thumbnail=findVisualItem(window->contentItem(),"layerThumbnail:"+QString::number(group));QVERIFY(thumbnail);QVERIFY(!thumbnail->isVisible());
        client.selectLayer(group);QTRY_VERIFY(!client.layerEditBusy());client.addDefaultLayer();QTRY_COMPARE(client.layers().size(),4);QTRY_VERIFY(!client.layerEditBusy());const auto first=client.activeLayer();QCOMPARE(layer(first).value("name").toString(),QStringLiteral("图层一"));
        client.addDefaultLayer();QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(!client.layerEditBusy());const auto second=client.activeLayer();QCOMPARE(layer(second).value("name").toString(),QStringLiteral("图层二"));
        QVERIFY(client.dropLayer(first,0,0));QTRY_COMPARE(layer(first).value("parent").toULongLong(),quint64(0));QTRY_VERIFY(!client.layerEditBusy());
        const auto dockGroup=workspace.rightGroups().last().toMap().value("id").toString();workspace.detachPanel(dockGroup,"layers");QTRY_COMPARE(workspace.floatingGroups().size(),1);QQuickWindow *floating=nullptr;QTRY_VERIFY(([&](){for(auto *w:applicationWindows())if(w!=window){floating=qobject_cast<QQuickWindow*>(w);return floating!=nullptr;}return false;})());
        QTRY_VERIFY(findVisualItem(floating->contentItem(),"layerRow:"+QString::number(group)));
        auto drop=[&](quint64 source,quint64 target,int placement){auto *row=findVisualItem(floating->contentItem(),target ? "layerRow:"+QString::number(target) : "layerRootDrop");if(!row)return false;const auto at=row->mapToScene({row->width()*0.72,placement==0?row->height()/2:placement==1?3.:row->height()-3.}).toPoint();QMimeData mime;mime.setData("application/x-drawverse-layer",client.layerDragPayload(source).toUtf8());QDragEnterEvent enter(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(floating,&enter);QDragMoveEvent move(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(floating,&move);QDropEvent event(at,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);QCoreApplication::sendEvent(floating,&event);return event.isAccepted();};
        QVERIFY(drop(first,group,0));QTRY_COMPARE(layer(first).value("parent").toULongLong(),group);QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(drop(first,second,2));QTRY_VERIFY(!client.layerEditBusy());auto ids=[&](){QList<quint64> ids;for(const auto &v:client.layers())ids.append(v.toMap().value("id").toULongLong());return ids;};QVERIFY(ids().indexOf(first)>ids().indexOf(second));client.undo();QTRY_VERIFY(!client.layerEditBusy());QVERIFY(ids().indexOf(first)<ids().indexOf(second));
        QVERIFY(drop(first,0,0));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(first).value("parent").toULongLong(),quint64(0));QCOMPARE(ids().last(),first);client.undo();QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(first).value("parent").toULongLong(),group);
        const auto preview=qEnvironmentVariable("DRAWVERSE_MASKS_PREVIEW");
        if(!preview.isEmpty()){floating->resize(360,760);client.selectLayer(mask);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());canvas->fitToView();QTest::qWait(250);QVERIFY(window->grabWindow().save(preview));QVERIFY(floating->grabWindow().save(preview+".layers.png"));}
        const auto path=QUrl::fromLocalFile(temp.filePath("masks.ora"));QSignalSpy files(&client,&PaintCoreClient::fileFinished);QVERIFY(client.saveDocument(path));QTRY_COMPARE_WITH_TIMEOUT(files.size(),1,10000);QVERIFY(files.first().first().toBool());
        const auto bad=client.layerDragPayload(first);client.newDocument(32,32);QTRY_COMPARE(client.documentWidth(),32);QTRY_VERIFY(client.ready());QVERIFY(!client.acceptLayerDrop(bad,0,0));
        QVERIFY(client.openDocument(path));QTRY_COMPARE_WITH_TIMEOUT(files.size(),2,10000);QVERIFY(files.last().first().toBool());QTRY_COMPARE(client.layers().size(),5);QTRY_VERIFY(client.ready());int masks=0;for(const auto &v:client.layers())if(v.toMap().value("mask").toBool())++masks;QCOMPARE(masks,1);
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void groupsCollapseMoveUndoAndSaveWorkAcrossFloatingPanels() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors) warnings.append(error.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(window);QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        auto layer=[&](quint64 id) {for(const auto &value:client.layers()) if(value.toMap().value("id").toULongLong()==id) return value.toMap();return QVariantMap();};
        client.newTransparentDocument(128,96);QTRY_COMPARE(client.documentWidth(),128);QTRY_VERIFY(!client.layerEditBusy());
        InputSample point;point.position={32.5,48.5};client.setBrushColor(QColor("#e17662"));client.setBrushRadius(12);QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),1);
        auto *groupButton=findVisualItem(window->contentItem(),"groupLayer");QVERIFY(groupButton);QVERIFY(QMetaObject::invokeMethod(groupButton,"clicked"));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());
        const auto group=client.activeLayer();QVERIFY(layer(group).value("group").toBool());QCOMPARE(layer(1).value("parent").toULongLong(),group);QCOMPARE(layer(1).value("depth").toInt(),1);
        QVERIFY(!client.beginStroke(point));client.clearError();QVERIFY(!findVisualItem(window->contentItem(),"lockTransparency")->isEnabled());
        client.addLayer("组内颜色");QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());const auto child=client.activeLayer();QCOMPARE(layer(child).value("parent").toULongLong(),group);
        client.setBrushColor(QColor("#2ea99d"));point.position={96.5,48.5};QVERIFY(client.beginStroke(point));client.endStroke();QTRY_COMPARE(client.undoDepth(),4);
        auto *list=findVisualItem(window->contentItem(),"layerList");QVERIFY(list);QTRY_COMPARE(list->property("count").toInt(),3);
        auto *expand=findVisualItem(window->contentItem(),"groupExpand:"+QString::number(group));QVERIFY(expand);QVERIFY(QMetaObject::invokeMethod(expand,"clicked"));QTRY_COMPARE(list->property("count").toInt(),1);QCOMPARE(client.undoDepth(),4);
        const auto dock=workspace.rightGroups().last().toMap().value("id").toString();workspace.detachPanel(dock,"layers");QTRY_COMPARE(applicationWindows().size(),2);
        QQuickWindow *floating=nullptr;for(auto *candidate:applicationWindows()) if(candidate!=window) floating=qobject_cast<QQuickWindow*>(candidate);QVERIFY(floating);
        QTRY_VERIFY((list=findVisualItem(floating->contentItem(),"layerList")));QTRY_COMPARE(list->property("count").toInt(),1);QTRY_VERIFY((expand=findVisualItem(floating->contentItem(),"groupExpand:"+QString::number(group))));QVERIFY(QMetaObject::invokeMethod(expand,"clicked"));QTRY_COMPARE(list->property("count").toInt(),3);
        client.selectLayer(group);QTRY_COMPARE(client.activeLayer(),group);QTRY_VERIFY(!client.layerEditBusy());client.setLayerProperties(group,true,.5);QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(client.moveLayer(group,4,2));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(1).value("offsetX").toInt(),4);QCOMPARE(layer(child).value("offsetY").toInt(),2);
        QVERIFY(client.setLayerLocks(group,4));QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(child).value("effectiveLocks").toInt(),4);
        client.selectLayer(child);QTRY_COMPARE(client.activeLayer(),child);QTRY_VERIFY(!client.layerEditBusy());QVERIFY(!client.beginStroke(point));client.clearError();QVERIFY(!findVisualItem(floating->contentItem(),"layerOpacity")->isEnabled());
        QVERIFY(client.setLayerLocks(group,0));QTRY_VERIFY(!client.layerEditBusy());
        auto *menuButton=findVisualItem(floating->contentItem(),"layerGroupMenu");QVERIFY(menuButton);QVERIFY(QMetaObject::invokeMethod(menuButton,"clicked"));
        auto *menu=floating->findChild<QObject*>("groupOperationsMenu");QVERIFY(menu);QTRY_VERIFY(menu->property("visible").toBool());
        QObject *moveInto=nullptr;QTRY_VERIFY((moveInto=floating->findChild<QObject*>("moveLayerInto:"+QString::number(group))));QVERIFY(!moveInto->property("enabled").toBool());
        auto *moveOut=floating->findChild<QObject*>("moveLayerOut");QVERIFY(moveOut);QVERIFY(moveOut->property("enabled").toBool());QVERIFY(QMetaObject::invokeMethod(moveOut,"triggered"));QVERIFY(QMetaObject::invokeMethod(menu,"close"));
        QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(child).value("parent").toULongLong(),quint64(0));client.undo();QTRY_VERIFY(!client.layerEditBusy());QCOMPARE(layer(child).value("parent").toULongLong(),group);
        QVERIFY(client.ungroupLayer(group));QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());client.undo();QTRY_COMPARE(client.layers().size(),3);QTRY_VERIFY(!client.layerEditBusy());
        client.selectLayer(group);QTRY_COMPARE(client.activeLayer(),group);QTRY_VERIFY(!client.layerEditBusy());
        const auto url=QUrl::fromLocalFile(temp.filePath("groups.ora"));QSignalSpy files(&client,&PaintCoreClient::fileFinished);QVERIFY(client.saveDocument(url,4));QTRY_COMPARE_WITH_TIMEOUT(files.size(),1,10000);QVERIFY(files.first().first().toBool());
        client.newTransparentDocument(16,16);QTRY_COMPARE(client.documentWidth(),16);QVERIFY(client.openDocument(url));QTRY_COMPARE_WITH_TIMEOUT(files.size(),2,10000);QVERIFY(files.last().first().toBool());QTRY_COMPARE(client.layers().size(),3);
        const auto loadedGroup=client.activeLayer();QVERIFY(layer(loadedGroup).value("group").toBool());QCOMPARE(layer(loadedGroup).value("opacity").toFloat(),.5f);
        workspace.returnGroup(workspace.floatingGroups().first().toMap().value("id").toString());QTRY_COMPARE(applicationWindows().size(),1);
        const auto preview=qEnvironmentVariable("DRAWVERSE_GROUPS_PREVIEW");if(!preview.isEmpty()) {
            window->findChild<CanvasItem*>("mainCanvas")->fitToView();QTRY_VERIFY(client.frameRevision()>=client.revision());QTest::qWait(300);QVERIFY(window->grabWindow().save(preview));
        }
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void transparentCanvasAndNavigatorUseCheckerboardWithoutChangingPixels() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini")); WorkspaceManager workspace(temp.filePath("layout.ini"));
        QQmlApplicationEngine engine; QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &error:errors) warnings.append(error.toString());});
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/app/Main.qml")); QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas"); QVERIFY(canvas); QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(128,96); QTRY_COMPARE(client.documentWidth(),128); QTRY_VERIFY(!client.layerEditBusy()); canvas->actualSize();
        auto *grid=findVisualItem(window->contentItem(),"canvasTransparency"); QVERIFY(grid);
        auto sample=[&](QQuickItem *item,QPointF local) {
            const auto shot=window->grabWindow(); if(shot.isNull()) return QColor();
            const auto point=item->mapToScene(local); const auto scale=static_cast<qreal>(shot.width())/window->width();
            return shot.pixelColor(qFloor(point.x()*scale),qFloor(point.y()*scale));
        };
        auto checkGrid=[&](QQuickItem *item) {
            return sample(item,{6,6})==QColor(Qt::white) && sample(item,{18,6})==QColor(204,204,204)
                && sample(item,{6,18})==QColor(204,204,204) && sample(item,{18,18})==QColor(Qt::white);
        };
        QTRY_VERIFY_WITH_TIMEOUT(checkGrid(grid),10000);
        const auto center=canvas->mapToScene(canvas->documentRect().center());
        QWheelEvent zoom(center,window->mapToGlobal(center.toPoint()),{},QPoint(0,120),Qt::NoButton,Qt::NoModifier,Qt::NoScrollPhase,false);
        QCoreApplication::sendEvent(window,&zoom);QVERIFY(canvas->zoom()>1);QTRY_VERIFY(checkGrid(grid));canvas->actualSize();QTRY_VERIFY(checkGrid(grid));
        client.setBrushColor(Qt::black); client.setBrushRadius(10); InputSample point; point.position={64.5,48.5}; QVERIFY(client.beginStroke(point));client.endStroke();
        QTRY_COMPARE(client.undoDepth(),1);QTRY_VERIFY(client.frameRevision()>=client.revision());
        QTRY_COMPARE(sample(canvas,canvas->documentRect().topLeft()+QPointF(64.5,48.5)),QColor(Qt::black));
        client.addLayer("second");QTRY_COMPARE(client.layers().size(),2);QTRY_VERIFY(!client.layerEditBusy());
        client.setLayerProperties(2,false,1);QTRY_VERIFY(!client.layerEditBusy());
        client.setLayerProperties(1,false,1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());
        QTRY_VERIFY(checkGrid(grid));
        QCOMPARE(sample(canvas,canvas->documentRect().topLeft()+QPointF(66,54)),QColor(204,204,204));
        QCOMPARE(qAlpha(client.frame().pixel(client.frame().width()/2,client.frame().height()/2)),0);
        const auto path=temp.filePath("transparent.png");QSignalSpy saved(&client,&PaintCoreClient::fileFinished);
        QVERIFY(client.saveDocument(QUrl::fromLocalFile(path),1));QTRY_COMPARE_WITH_TIMEOUT(saved.size(),1,10000);QVERIFY(saved.first().first().toBool());
        const QImage exported(path);QVERIFY(!exported.isNull());QCOMPARE(exported.pixelColor(64,48).alpha(),0);QCOMPARE(exported.pixelColor(6,6).alpha(),0);
        const auto group=workspace.rightGroups().last().toMap().value("id").toString();
        auto *dock=findVisualItem(window->contentItem(),"dockGroup:"+group);QVERIFY(dock);QVERIFY(dock->setProperty("selected","navigator"));workspace.setActive(group,"navigator");
        QQuickItem *miniGrid=nullptr;QTRY_VERIFY((miniGrid=findVisualItem(window->contentItem(),"navigatorTransparency")));
        QTRY_VERIFY_WITH_TIMEOUT(checkGrid(miniGrid),10000);
        client.setLayerProperties(1,true,1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision(1)>=client.revision());
        QCOMPARE(qAlpha(client.frame(1).pixel(client.frame(1).width()/2,client.frame(1).height()/2)),255);
        QTRY_VERIFY(checkGrid(miniGrid));
        const auto preview=qEnvironmentVariable("DRAWVERSE_TRANSPARENCY_PREVIEW");
        if(!preview.isEmpty()) {client.setLayerProperties(1,false,1);QTRY_VERIFY(!client.layerEditBusy());QTRY_VERIFY(client.frameRevision()>=client.revision());QTest::qWait(100);QVERIFY(window->grabWindow().save(preview));}
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void layerControlsCommitOnceAndWorkInFloatingPanels() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini")); WorkspaceManager workspace(temp.filePath("layout.ini"));
        QQmlApplicationEngine engine; QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){ for(const auto &error:errors) warnings.append(error.toString()); });
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/app/Main.qml")); QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        auto layer=[&](){ for(const auto &value:client.layers()) if(value.toMap().value("id").toULongLong()==client.activeLayer()) return value.toMap(); return QVariantMap(); };
        QQuickItem *opacity=nullptr;QTRY_VERIFY((opacity=findVisualItem(window->contentItem(),"layerOpacity")));
        auto *edit=opacity->findChild<QObject*>("layerOpacityInput"); QVERIFY(edit);
        QVERIFY(edit->setProperty("text","0%")); QVERIFY(QMetaObject::invokeMethod(edit,"commit"));
        QTRY_COMPARE(layer().value("opacity").toFloat(),0.f); QTRY_VERIFY(!client.layerEditBusy()); QCOMPARE(client.undoDepth(),1);
        QVERIFY(edit->setProperty("text","100%")); QVERIFY(QMetaObject::invokeMethod(edit,"commit"));
        QTRY_COMPARE(layer().value("opacity").toFloat(),1.f); QTRY_VERIFY(!client.layerEditBusy()); QCOMPARE(client.undoDepth(),2);
        QVERIFY(QMetaObject::invokeMethod(opacity,"openSlider"));
        auto *slider=qobject_cast<QQuickItem*>(opacity->findChild<QObject*>("layerOpacitySlider")); QVERIFY(slider);
        QTRY_VERIFY(slider->isVisible() && slider->width()>100);
        const auto start=slider->mapToScene({slider->width()*.8,slider->height()/2}).toPoint();
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,start);
        for(int i=0;i<8;++i) QTest::mouseMove(window,slider->mapToScene({slider->width()*(.7-i*.04),slider->height()/2}).toPoint());
        QCOMPARE(client.undoDepth(),2);
        QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,slider->mapToScene({slider->width()*.42,slider->height()/2}).toPoint());
        QTRY_COMPARE(client.undoDepth(),3); QTRY_VERIFY(!client.layerEditBusy()); QVERIFY(layer().value("opacity").toFloat()<.6f);
        QTest::keyClick(window,Qt::Key_Escape);
        auto *blend=findVisualItem(window->contentItem(),"layerBlendMode"); QVERIFY(blend); QCOMPARE(blend->property("count").toInt(),27);
        QVERIFY(blend->setProperty("currentIndex",3)); QVERIFY(QMetaObject::invokeMethod(blend,"activated",Q_ARG(int,3)));
        QTRY_COMPARE(layer().value("blendMode").toInt(),3); QTRY_VERIFY(!client.layerEditBusy());
        auto *fill=findVisualItem(window->contentItem(),"layerFill"); QVERIFY(fill);
        QVERIFY(QMetaObject::invokeMethod(fill,"committed",Q_ARG(double,.35))); QTRY_COMPARE(layer().value("fill").toFloat(),.35f); QTRY_VERIFY(!client.layerEditBusy());
        for(const auto &name:QStringList{"lockTransparency","lockPosition","lockAll"}) {
            auto *button=findVisualItem(window->contentItem(),name); QVERIFY(button); QVERIFY(QMetaObject::invokeMethod(button,"clicked")); QTRY_VERIFY(button->property("selected").toBool()); QTRY_VERIFY(!client.layerEditBusy());
        }
        QCOMPARE(layer().value("locks").toInt(),7); QVERIFY(!opacity->isEnabled()); QVERIFY(!blend->isEnabled()); QVERIFY(!fill->isEnabled());
        InputSample sample; sample.position={32,32}; QVERIFY(!client.beginStroke(sample)); client.clearError();
        auto *eye=findVisualItem(window->contentItem(),"layerVisibility:1"); QVERIFY(eye); QVERIFY(QMetaObject::invokeMethod(eye,"clicked"));
        QTRY_VERIFY(!layer().value("visible").toBool()); QTRY_VERIFY(!client.layerEditBusy());
        QTRY_VERIFY_WITH_TIMEOUT(!layer().value("thumbnail").toString().isEmpty(),10000);
        const auto group=workspace.rightGroups().last().toMap().value("id").toString(); workspace.detachPanel(group,"layers");
        QTRY_COMPARE(applicationWindows().size(),2);
        QQuickWindow *floating=nullptr; for(auto *candidate:applicationWindows()) if(candidate!=window) floating=qobject_cast<QQuickWindow*>(candidate);
        QVERIFY(floating); QQuickItem *unlock=nullptr; QTRY_VERIFY((unlock=findVisualItem(floating->contentItem(),"lockAll")));
        QVERIFY(QMetaObject::invokeMethod(unlock,"clicked")); QTRY_COMPARE(layer().value("locks").toInt(),3); QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(findVisualItem(floating->contentItem(),"layerOpacity")->isEnabled());
        workspace.returnGroup(workspace.floatingGroups().first().toMap().value("id").toString()); QTRY_COMPARE(applicationWindows().size(),1);
        const auto preview=qEnvironmentVariable("DRAWVERSE_LAYERS_PREVIEW");
        if(!preview.isEmpty()) {
            workspace.resetLayout();
            client.addLayer(QStringLiteral("上色 · 柔光")); QTRY_COMPARE(client.layers().size(),2); QTRY_VERIFY(!client.layerEditBusy());
            client.setBrushColor(QColor("#2ea99d")); client.setBrushRadius(28);
            for(int row=0;row<3;++row) {
                InputSample point; point.capabilities=1; point.tool=1; point.pressure=.3f; point.position={160.,170.+row*120}; QVERIFY(client.beginStroke(point));
                for(int i=1;i<=80;++i) { point.position={160.+i*8.,170.+row*120+std::sin(i*.07)*60}; point.pressure=static_cast<float>(.2+.7*std::sin(i*.02)); client.strokeTo(point); }
                client.endStroke();
            }
            QTRY_VERIFY(client.frameRevision()>=client.revision());
            QVERIFY(client.setLayerBlend(client.activeLayer(),13)); QTRY_VERIFY(!client.layerEditBusy());
            QVERIFY(client.setLayerFill(client.activeLayer(),.8)); QTRY_VERIFY(!client.layerEditBusy());
            client.setLayerProperties(client.activeLayer(),true,.85); QTRY_VERIFY(!client.layerEditBusy());
            QVERIFY(client.setLayerLocks(client.activeLayer(),2)); QTRY_VERIFY(!client.layerEditBusy());
            QTest::qWait(350); QVERIFY(window->grabWindow().save(preview));
        }
        QCOMPARE(warnings,QStringList()); QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void moveToolMouseKeyboardAndPositionLock() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini")); WorkspaceManager workspace(temp.filePath("layout.ini")); QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace); engine.load(QUrl("qrc:/qml/app/Main.qml")); QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); auto *canvas=window->findChild<CanvasItem*>("mainCanvas"); QVERIFY(canvas); QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(128,64); QTRY_COMPARE(client.documentWidth(),128); QTRY_VERIFY(client.ready()); canvas->actualSize();
        InputSample sample; sample.position={16.5,16.5}; QVERIFY(client.beginStroke(sample)); client.endStroke(); QTRY_COMPARE(client.undoDepth(),1);
        client.setMoveTool(true); const auto pos=canvas->mapToScene(canvas->documentRect().topLeft()+QPointF(16,16)).toPoint();
        QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,pos); QTest::mouseMove(window,pos+QPoint(20,3)); QCOMPARE(client.undoDepth(),1);
        QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,pos+QPoint(20,3)); QTRY_COMPARE(client.undoDepth(),2); QTRY_VERIFY(!client.layerEditBusy());
        auto layer=[&](){ return client.layers().first().toMap(); }; QCOMPARE(layer().value("offsetX").toInt(),20); QCOMPARE(layer().value("offsetY").toInt(),3);
        QTest::keyClick(window,Qt::Key_Left,Qt::ShiftModifier); QTRY_COMPARE(layer().value("offsetX").toInt(),10); QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(client.setLayerLocks(1,2)); QTRY_VERIFY(!client.layerEditBusy()); const auto revision=client.revision();
        QTest::keyClick(window,Qt::Key_Right); QTRY_VERIFY(!client.lastError().isEmpty()); QTRY_VERIFY(!client.layerEditBusy()); QCOMPARE(client.revision(),revision); QCOMPARE(layer().value("offsetX").toInt(),10);
        client.clearError(); client.setEraser(false); sample.position={2.5,16.5}; const auto paintDepth=client.undoDepth(); QVERIFY(client.beginStroke(sample)); client.endStroke(); QTRY_COMPARE(client.undoDepth(),paintDepth+1); QTRY_VERIFY(client.revision()>revision);
        const auto depth=client.undoDepth(); client.setMoveTool(true); QTest::mousePress(window,Qt::LeftButton,Qt::NoModifier,pos); QTest::keyClick(window,Qt::Key_Escape); QTest::mouseRelease(window,Qt::LeftButton,Qt::NoModifier,pos+QPoint(10,0)); QTest::qWait(50); QCOMPARE(client.undoDepth(),depth);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void layerMetadataAndThumbnailsSurviveAsyncSaveOpen() {
        QTemporaryDir temp; PaintCoreClient client(nullptr,temp.filePath("storage.ini")); QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        InputSample sample; sample.position={40.5,40.5}; QVERIFY(client.beginStroke(sample)); client.endStroke(); QTRY_COMPARE(client.undoDepth(),1);
        QVERIFY(client.moveLayer(1,-17,4)); QTRY_VERIFY(!client.layerEditBusy()); QVERIFY(client.setLayerFill(1,.42)); QTRY_VERIFY(!client.layerEditBusy());
        QVERIFY(client.setLayerBlend(1,8)); QTRY_VERIFY(!client.layerEditBusy()); QVERIFY(client.setLayerLocks(1,7)); QTRY_VERIFY(!client.layerEditBusy());
        const auto before=client.layers().first().toMap(); const auto path=QUrl::fromLocalFile(temp.filePath("layer-settings.ora")); QSignalSpy saved(&client,&PaintCoreClient::fileFinished);
        QVERIFY(client.saveDocument(path)); QTRY_COMPARE(saved.size(),1); QVERIFY(saved.last().first().toBool()); QVERIFY(!client.modified());
        client.newTransparentDocument(64,64); QTRY_COMPARE(client.documentWidth(),64); QTRY_VERIFY(client.ready()); QVERIFY(client.openDocument(path)); QTRY_COMPARE(saved.size(),2); QVERIFY(saved.last().first().toBool());
        QTRY_VERIFY(client.ready()); const auto after=client.layers().first().toMap();
        for(const auto &key:QStringList{"fill","blendMode","locks","offsetX","offsetY","dissolveSeed"}) QCOMPARE(after.value(key),before.value(key));
        client.requestLayerPreview(client.activeLayer());
        QTRY_VERIFY_WITH_TIMEOUT(!client.layers().first().toMap().value("thumbnail").toString().isEmpty(),10000);
        QImage thumbnail; const auto base64=client.layers().first().toMap().value("thumbnail").toString().section(',',1).toLatin1(); QVERIFY(thumbnail.loadFromData(QByteArray::fromBase64(base64),"PNG")); QVERIFY(thumbnail.width()<=64 && thumbnail.height()<=64);
        QVERIFY(client.lastError().isEmpty()); QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void storageSettingsDialogShowsAppliedAndPendingValues() {
        QTemporaryDir temp; QVERIFY(temp.isValid()); PaintCoreClient client(nullptr,temp.filePath("storage.ini"));
        WorkspaceManager workspace(temp.filePath("layout.ini")); QQmlApplicationEngine engine; QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){ for(const auto &error:errors) warnings.append(error.toString()); });
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/app/Main.qml")); QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        QTRY_VERIFY_WITH_TIMEOUT(client.ready() && !client.storageBusy(),10000);
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        auto *options=findVisualItem(window->contentItem(),"brushOptionsBar");QVERIFY(options);
        for(auto *item:options->findChildren<QObject*>()) {
            QVERIFY(item->property("tooltip").toString()!=QStringLiteral("性能与暂存盘"));
            QVERIFY(item->property("tooltip").toString()!=QStringLiteral("创建自定义面板"));
        }
        auto *customAction=window->findChild<QObject*>("customPanelAction");QVERIFY(customAction);
        auto *customDialog=window->findChild<QObject*>("customPanelDialog");QVERIFY(customDialog);
        QVERIFY(QMetaObject::invokeMethod(customAction,"triggered"));QTRY_VERIFY(customDialog->property("opened").toBool());
        QVERIFY(QMetaObject::invokeMethod(customDialog,"reject"));QTRY_VERIFY(!customDialog->property("opened").toBool());
        auto *storageAction=window->findChild<QObject*>("storagePreferencesAction");QVERIFY(storageAction);QVERIFY(storageAction->property("enabled").toBool());
        auto *dialog=window->findChild<QObject*>("preferencesDialog"); QVERIFY(dialog);QVERIFY(QMetaObject::invokeMethod(storageAction,"triggered"));QTRY_VERIFY(dialog->property("opened").toBool());QTRY_VERIFY(!client.storageBusy());
        auto *memory=dialog->findChild<QObject*>("storageMemory"); QVERIFY(memory); QCOMPARE(memory->property("currentIndex").toInt(),2);
        auto *scratch=dialog->findChild<QObject*>("storageScratch"); QVERIFY(scratch); QCOMPARE(scratch->property("value").toInt(),8);
        auto *reserve=dialog->findChild<QObject*>("storageReserve"); QVERIFY(reserve); QCOMPARE(reserve->property("value").toInt(),512);
        auto *directory=dialog->findChild<QObject*>("storageDirectory"); QVERIFY(directory); QVERIFY(directory->property("text").toString().isEmpty());
        const auto preview=qEnvironmentVariable("DRAWVERSE_STORAGE_PREVIEW");
        if(!preview.isEmpty()) { QTest::qWait(200); QVERIFY(window->grabWindow().save(preview)); }
        QVERIFY(memory->setProperty("currentIndex",0)); QVERIFY(scratch->setProperty("value",2));
        QVERIFY(reserve->setProperty("value",0)); QVERIFY(directory->setProperty("text",temp.path()));
        QSignalSpy saved(&client,&PaintCoreClient::storageFinished);
        auto *button=dialog->findChild<QObject*>("saveStorageSettings"); QVERIFY(button); QVERIFY(QMetaObject::invokeMethod(button,"clicked"));
        QTRY_COMPARE(saved.size(),1); QVERIFY(saved.first().first().toBool());
        QCOMPARE(client.storageSettings().value("memoryMiB").toInt(),64);
        QCOMPARE(client.activeStorageSettings().value("memoryMiB").toInt(),256);
        QVERIFY(QMetaObject::invokeMethod(dialog,"close")); QVERIFY(QMetaObject::invokeMethod(dialog,"open")); QTRY_VERIFY(!client.storageBusy());
        QCOMPARE(memory->property("currentIndex").toInt(),0); QCOMPARE(reserve->property("value").toInt(),0);
        QCOMPARE(warnings,QStringList()); QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void storagePreferencesPersistOnlyAfterValidationAndApplyAfterRestart() {
        QTemporaryDir temp; QVERIFY(temp.isValid()); const auto settings=temp.filePath("storage.ini");
        QVERIFY(QDir().mkpath(temp.filePath(QStringLiteral("暂存盘"))));
        const auto chosen=temp.filePath(QStringLiteral("暂存盘"));
        {
            PaintCoreClient client(nullptr,settings); QTRY_VERIFY_WITH_TIMEOUT(client.ready() && !client.storageBusy(),10000);
            QSignalSpy result(&client,&PaintCoreClient::storageFinished);
            client.addLayer("retained"); QTRY_COMPARE(client.undoDepth(),1);
            QVERIFY(client.saveStorageSettings(chosen,64,2,0));
            QVERIFY(!client.saveStorageSettings(chosen,128,4,0));
            QTRY_COMPARE(result.size(),1); QVERIFY(result.last().first().toBool());
            QCOMPARE(client.storageSettings().value("directory").toString(),chosen);
            QCOMPARE(client.activeStorageSettings().value("memoryMiB").toInt(),256);
            QCOMPARE(client.undoDepth(),1); QCOMPARE(client.layers().size(),2);
            QVERIFY(client.storageMessage().contains(QStringLiteral("重启")));
            QVERIFY(client.saveStorageSettings(temp.filePath("missing"),128,4,0));
            QTRY_COMPARE(result.size(),2); QVERIFY(!result.last().first().toBool());
            QCOMPARE(client.storageSettings().value("directory").toString(),chosen);
            QSettings persisted(settings,QSettings::IniFormat); QCOMPARE(persisted.value("storage/memoryMiB").toInt(),64);
            QCOMPARE(persisted.value("storage/directory").toString(),chosen);
            client.undo(); QTRY_COMPARE(client.redoDepth(),1);
            QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
        }
        PaintCoreClient restarted(nullptr,settings); QTRY_VERIFY_WITH_TIMEOUT(restarted.ready() && !restarted.storageBusy(),10000);
        QCOMPARE(restarted.activeStorageSettings().value("directory").toString(),chosen);
        QCOMPARE(restarted.activeStorageSettings().value("memoryMiB").toInt(),64);
        QCOMPARE(restarted.activeStorageSettings().value("scratchGiB").toInt(),2);
        QVERIFY(restarted.storageInfo().value("availableGiB").toDouble()>0);
        QSignalSpy stopped(&restarted,&PaintCoreClient::stopped); restarted.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void invalidStartupStorageCanBeCorrectedFromQml() {
        QTemporaryDir temp; QVERIFY(temp.isValid()); const auto settings=temp.filePath("bad-storage.ini");
        { QSettings bad(settings,QSettings::IniFormat); bad.setValue("storage/directory",temp.filePath("missing")); bad.sync(); }
        PaintCoreClient client(nullptr,settings); WorkspaceManager workspace(temp.filePath("layout.ini"));
        QQmlApplicationEngine engine; QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){ for(const auto &error:errors) warnings.append(error.toString()); });
        engine.rootContext()->setContextProperty("PaintClient",&client); engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/app/Main.qml")); QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        QTRY_VERIFY_WITH_TIMEOUT(!client.storageBusy() && !client.lastError().isEmpty(),10000); QVERIFY(!client.ready());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        auto *dialog=window->findChild<QObject*>("preferencesDialog"); QVERIFY(dialog);
        QVERIFY(QMetaObject::invokeMethod(dialog,"open")); QTRY_VERIFY(!client.storageBusy());
        auto *directory=dialog->findChild<QObject*>("storageDirectory"); QVERIFY(directory);
        QVERIFY(directory->setProperty("text",temp.path()));
        auto *save=dialog->findChild<QObject*>("saveStorageSettings"); QVERIFY(save);
        QSignalSpy result(&client,&PaintCoreClient::storageFinished); QVERIFY(QMetaObject::invokeMethod(save,"clicked"));
        QTRY_VERIFY(!result.isEmpty()); QVERIFY(result.last().first().toBool()); QVERIFY(!client.ready());
        QCOMPARE(client.storageSettings().value("directory").toString(),temp.path());
        QCOMPARE(warnings,QStringList());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void largePagedDocumentPaintUndoRedoAndSaveRemainAsynchronous() {
        QTemporaryDir temp; QVERIFY(temp.isValid());
        QImage source(4096,4160,QImage::Format_RGBA8888); source.fill(QColor(30,80,120));
        const auto path=temp.filePath("large.png"); QVERIFY(source.save(path)); source=QImage();
        PaintCoreClient client; QSignalSpy files(&client,&PaintCoreClient::fileFinished);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        QVERIFY(client.openDocument(QUrl::fromLocalFile(path)));
        QTRY_COMPARE_WITH_TIMEOUT(client.documentHeight(),4160,30000);
        QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy(),30000);
        QTRY_VERIFY_WITH_TIMEOUT(!client.frame().isNull() && client.frameRevision()>=client.revision(),30000);
        const auto center=[&client] {return client.frame().pixel(client.frame().width()/2,client.frame().height()/2);};
        QCOMPARE(qRed(center()),30);
        client.setBrushRadius(256); client.setBrushColor(Qt::red);
        InputSample sample; sample.position={2048,2080};
        QVERIFY(client.beginStroke(sample));
        QElapsedTimer elapsed; elapsed.start(); client.endStroke(); QVERIFY(elapsed.elapsed()<1000);
        QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),1,30000);
        QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),30000);
        QVERIFY(qRed(center())>240 && qGreen(center())<10);
        client.undo(); QTRY_COMPARE_WITH_TIMEOUT(client.redoDepth(),1,30000);
        QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),30000);
        QCOMPARE(qRed(center()),30);
        client.redo(); QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),1,30000);
        QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),30000);
        QVERIFY(qRed(center())>240 && qGreen(center())<10);
        QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath("painted.png")),1));
        QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy(),30000);
        QVERIFY(QFileInfo::exists(temp.filePath("painted.png")));
        QVERIFY2(client.lastError().isEmpty(),qPrintable(client.lastError()));
        QVERIFY(files.size()>=2); QVERIFY(files.last().first().toBool());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void fullCanvasStrokeExceedsOldHistoryLimitAndRemainsUndoable() {
        PaintCoreClient client; QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(1536,1536); QTRY_COMPARE(client.documentWidth(),1536);
        client.setBrushRadius(64); InputSample sample; sample.position={32,32};
        QVERIFY(client.beginStroke(sample));
        for(int row=0;row<24;++row) for(int column=0;column<24;++column) {
            const int x=row%2==0?column:23-column;
            sample.position={x*64.+32.,row*64.+32.}; client.strokeTo(sample);
        }
        QElapsedTimer elapsed; elapsed.start(); client.endStroke();
        QVERIFY(elapsed.elapsed()<1000); QVERIFY(client.modified());
        QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),1,30000);
        QTRY_VERIFY_WITH_TIMEOUT(!client.frame().isNull() && client.frameRevision()>=client.revision(),30000);
        QCOMPARE(qAlpha(client.frame().pixel(client.frame().width()/2,client.frame().height()/2)),255);
        QVERIFY2(client.lastError().isEmpty(),qPrintable(client.lastError()));
        client.undo(); QTRY_COMPARE_WITH_TIMEOUT(client.redoDepth(),1,30000);
        QTRY_VERIFY(client.frameRevision()>=client.revision());
        QCOMPARE(qAlpha(client.frame().pixel(client.frame().width()/2,client.frame().height()/2)),0);
        client.redo(); QTRY_COMPARE_WITH_TIMEOUT(client.undoDepth(),1,30000);
        QTRY_VERIFY(client.frameRevision()>=client.revision());
        QCOMPARE(qAlpha(client.frame().pixel(client.frame().width()/2,client.frame().height()/2)),255);
        QVERIFY(client.lastError().isEmpty());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void drawingKeepsQmlLayerDelegatesAlive() {
        QTemporaryDir temp;
        PaintCoreClient client; WorkspaceManager workspace(temp.filePath("performance.ini"));
        QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty("PaintClient",&client);
        engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/app/Main.qml"));
        QVERIFY(!engine.rootObjects().isEmpty());
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready() && !client.frame().isNull(),10000);
        QPointer<QQuickItem> row;
        const auto rowName="layerRow:"+QString::number(client.activeLayer());
        QTRY_VERIFY((row=findVisualItem(window->contentItem(),rowName)));
        QSignalSpy layers(&client,&PaintCoreClient::layersChanged);
        QSignalSpy frames(&client,&PaintCoreClient::frameChanged);
        const auto initialRevision=client.frameRevision();
        client.setBrushRadius(8); InputSample sample; sample.position={40,40};
        QVERIFY(client.beginStroke(sample));
        for(int i=1;i<=60;++i) {
            sample.position={40.+i*3,40.+(i%3)}; sample.pressure=.3f+(i%5)*.1f;
            client.strokeTo(sample); QTest::qWait(8);
        }
        client.endStroke(); QTRY_COMPARE(client.undoDepth(),1);
        QTRY_VERIFY(client.frameRevision()>initialRevision && client.frameRevision()>=client.revision());
        QVERIFY(frames.size()>1); QCOMPARE(layers.size(),0);
        QVERIFY(row); QCOMPARE(findVisualItem(window->contentItem(),rowName),row.data());
        QVERIFY(client.modified());
        client.setLayerProperties(client.activeLayer(),false,.3);
        QTRY_VERIFY(!client.layers().first().toMap().value("visible").toBool());
        QVERIFY(!layers.isEmpty());
        QTRY_VERIFY((row=findVisualItem(window->contentItem(),rowName)));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
        delete window;
    }
    void fileCancellationAndShutdownStayAsynchronous() {
        QTemporaryDir temp; PaintCoreClient client; QSignalSpy files(&client,&PaintCoreClient::fileFinished);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(2048,2048); QTRY_COMPARE(client.documentWidth(),2048);
        const auto path=temp.filePath("cancelled.png");
        QVERIFY(client.saveDocument(QUrl::fromLocalFile(path),1)); client.cancelFile();
        QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy(),10000); QCOMPARE(files.size(),1);
        if(!files.last().first().toBool()) QVERIFY(!QFileInfo::exists(path));
        client.clearError();
        QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath("shutdown.png")),1));
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); QElapsedTimer elapsed; elapsed.start();
        client.shutdown(); QVERIFY(elapsed.elapsed()<1000); QTRY_COMPARE_WITH_TIMEOUT(stopped.size(),1,10000);
    }
    void asynchronousFileRoundTripAndFailure() {
        QTemporaryDir temp; QVERIFY(temp.isValid());
        PaintCoreClient client; QSignalSpy files(&client,&PaintCoreClient::fileFinished);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(96,96); QTRY_COMPARE(client.documentWidth(),96);
        client.setBrushColor(Qt::red); InputSample s; s.position={32,32};
        QVERIFY(client.beginStroke(s)); QVERIFY(client.modified()); client.endStroke();
        QVERIFY(client.modified()); // Protects immediate close before the worker's metadata arrives.
        QTRY_COMPARE(client.undoDepth(),1);
        QVERIFY(client.modified());
        for(const auto &entry:QList<QPair<QString,int>>{{"image.png",1},{"image.jpg",2},{"image.webp",3}}) {
            QVERIFY(client.saveDocument(QUrl::fromLocalFile(temp.filePath(entry.first)),entry.second));
            QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy(),10000); QVERIFY(files.last().first().toBool());
            QVERIFY(QFileInfo::exists(temp.filePath(entry.first))); QVERIFY(client.modified());
        }
        const auto url=QUrl::fromLocalFile(temp.filePath(QStringLiteral("绘画.ora")));
        QVERIFY(client.saveDocument(url)); QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy(),10000);
        QVERIFY(files.last().first().toBool()); QVERIFY(!client.modified()); QCOMPARE(client.documentUrl(),url);
        client.newTransparentDocument(32,32); QTRY_COMPARE(client.documentWidth(),32);
        QVERIFY(client.documentUrl().isEmpty());
        QVERIFY(client.openDocument(url)); QVERIFY(!client.ready());
        QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy() && client.ready(),10000);
        QVERIFY(files.last().first().toBool()); QCOMPARE(client.documentWidth(),96); QCOMPARE(client.undoDepth(),0);
        QTRY_VERIFY(!client.frame().isNull() && client.frame().width()==96);
        QTRY_VERIFY(qAlpha(client.frame().pixel(32,32))>0);
        const auto generation=client.generation();
        QVERIFY(client.openDocument(QUrl::fromLocalFile(temp.filePath("missing.png"))));
        QTRY_VERIFY_WITH_TIMEOUT(!client.fileBusy() && client.ready(),10000);
        QVERIFY(!files.last().first().toBool()); QVERIFY(!client.lastError().isEmpty());
        QCOMPARE(client.generation(),generation); QCOMPARE(client.documentWidth(),96); QCOMPARE(client.documentUrl(),url);
        const auto misleading=QUrl::fromLocalFile(temp.filePath("image-content.ora"));
        QVERIFY(client.saveDocument(misleading,1)); QTRY_VERIFY(!client.fileBusy());
        QVERIFY(client.openDocument(misleading)); QTRY_VERIFY(!client.fileBusy() && client.ready());
        QVERIFY(files.last().first().toBool()); QVERIFY(client.documentUrl().isEmpty());
        QCOMPARE(client.documentName(),QString("image-content.ora"));
        QVERIFY(!client.saveDocument(QUrl("https://example.invalid/file"))); QVERIFY(!client.fileBusy());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void asynchronousHistoryAndLayers() {
        PaintCoreClient client; QSignalSpy completed(&client,&PaintCoreClient::commandCompleted);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready() && !client.frame().isNull(),10000);
        client.newTransparentDocument(128,128);
        QTRY_COMPARE(client.documentWidth(),128);
        QTRY_COMPARE(client.frame().width(),128);
        client.setBrushColor(Qt::red); client.setBrushRadius(12);
        InputSample s; s.position={32,32}; s.pressure=.8f; s.capabilities=1; s.tool=1;
        QVERIFY(client.beginStroke(s)); s.position={80,32}; client.strokeTo(s); client.endStroke();
        QTRY_COMPARE(client.undoDepth(),1);
        QTRY_VERIFY(qAlpha(client.frame().pixel(50,32))>0);
        client.undo(); QTRY_COMPARE(client.redoDepth(),1);
        QTRY_COMPARE(qAlpha(client.frame().pixel(50,32)),0);
        client.redo(); QTRY_COMPARE(client.undoDepth(),1);
        QTRY_VERIFY(qAlpha(client.frame().pixel(50,32))>0);
        client.addLayer(QStringLiteral("测试 🖌")); QTRY_COMPARE(client.layers().size(),2);
        const auto id=client.activeLayer();
        QCOMPARE(client.layers().first().toMap().value("name").toString(),QStringLiteral("测试 🖌"));
        client.setLayerProperties(id,false,.3); QTRY_VERIFY(!client.layers().first().toMap().value("visible").toBool());
        client.removeLayer(id); QTRY_COMPARE(client.layers().size(),1);
        QVERIFY(!completed.isEmpty()); for(const auto &event:completed) QVERIFY(event.first().toBool());
        QVERIFY(client.lastError().isEmpty());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void pressureCancelAndValidation() {
        PaintCoreClient client; QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(128,128); QTRY_COMPARE(client.documentWidth(),128);
        QTRY_COMPARE(client.frame().width(),128);
        client.setBrushRadius(20);
        InputSample low; low.position={30,64}; low.pressure=.2f; low.capabilities=1; low.tool=1;
        QVERIFY(client.beginStroke(low)); client.endStroke();
        QTRY_COMPARE(client.undoDepth(),1);
        InputSample high=low; high.position={90,64}; high.pressure=1;
        QVERIFY(client.beginStroke(high)); client.endStroke(); QTRY_COMPARE(client.undoDepth(),2);
        QTRY_VERIFY(qAlpha(client.frame().pixel(90,76))>0);
        QCOMPARE(qAlpha(client.frame().pixel(30,76)),0);
        InputSample cancelled=high; cancelled.position={64,20};
        QVERIFY(client.beginStroke(cancelled)); client.cancelStroke();
        QTest::qWait(100); QCOMPARE(client.undoDepth(),2);
        QTRY_COMPARE(qAlpha(client.frame().pixel(64,20)),0);
        client.newTransparentDocument(1000001,1); QVERIFY(!client.lastError().isEmpty()); QCOMPARE(client.documentWidth(),128);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void workspaceRoundTripAndCorruption() {
        QTemporaryDir temp; QVERIFY(temp.isValid()); const QString path=temp.filePath("workspace.ini");
        WorkspaceManager workspace(path);
        QVERIFY(workspace.addCustomPanel("Note","notes").isEmpty());
        const auto id=workspace.addCustomPanel(QStringLiteral("自定义色板"),"palette"); QVERIFY(!id.isEmpty());
        const auto group=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachPanel(group,id); QCOMPARE(workspace.floatingGroups().size(),1);
        const auto floating=workspace.floatingGroups().first().toMap().value("id").toString();
        workspace.updateGeometry(floating,-90000,-90000,400,300); workspace.saveLayout();
        WorkspaceManager restored(path); QCOMPARE(restored.floatingGroups().size(),1);
        QCOMPARE(restored.panelDefinition(id).value("title").toString(),QStringLiteral("自定义色板"));
        QVERIFY(restored.floatingGroups().first().toMap().value("x").toInt()>-90000);
        restored.returnGroup(floating); QCOMPARE(restored.floatingGroups().size(),0);
        const auto target=restored.rightGroups().first().toMap().value("id").toString();
        const auto payload=QString::fromUtf8(QJsonDocument(QJsonObject{{"group",floating},{"panel",id},{"whole",true}}).toJson());
        QVERIFY(restored.dockPayload(payload,"right",target));
        QCOMPARE(restored.rightGroups().size(),2);
        QVERIFY(!restored.dockPayload("{}","right",target));
        QSettings settings(path,QSettings::IniFormat); settings.setValue("workspace",QByteArray("{bad json}")); settings.sync();
        WorkspaceManager fallback(path); QCOMPARE(fallback.rightGroups().size(),2); QCOMPARE(fallback.panelDefinition("layers").value("kind").toString(),QString("layers"));
    }
    void sparseLargeCanvasAndViewportReplacement() {
        PaintCoreClient client; QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        const quint64 previousGeneration=client.generation();
        QElapsedTimer elapsed; elapsed.start(); client.newTransparentDocument(1000000,1000000);
        QVERIFY(elapsed.elapsed()<1000); QVERIFY(!client.ready());
        QTRY_VERIFY(client.ready() && client.generation()>previousGeneration);
        QCOMPARE(client.documentWidth(),1000000);
        QTRY_VERIFY_WITH_TIMEOUT(!client.frame().isNull(),10000);
        QVERIFY(client.frame().width()<=1024 && client.frame().height()<=1024);
        client.setBrushColor(Qt::red); client.setBrushRadius(8);
        InputSample sample; sample.position={900032,900032};
        QVERIFY(client.beginStroke(sample)); client.endStroke(); QTRY_COMPARE(client.undoDepth(),1);
        client.requestViewport(0,QRectF(0,0,64,64),QSize(64,64));
        const QRectF target(900000,900000,64,64);
        client.requestViewport(0,target,QSize(128,128));
        QTRY_COMPARE_WITH_TIMEOUT(client.frameRegion(),target,10000);
        QTRY_VERIFY_WITH_TIMEOUT(client.frameRevision()>=client.revision(),10000);
        QCOMPARE(client.frame().size(),QSize(128,128));
        QVERIFY(qAlpha(client.frame().pixel(64,64))>0);
        QCOMPARE(qAlpha(client.frame().pixel(0,0)),0);
        // Navigation is a separate frame channel with a hard display-memory bound.
        client.requestViewport(1,QRectF(0,0,1000000,1000000),QSize(8192,8192));
        QTRY_VERIFY_WITH_TIMEOUT(!client.frame(1).isNull(),10000);
        QVERIFY(qint64(client.frame(1).width())*client.frame(1).height()<=4194304);
        QVERIFY(client.frame(1).sizeInBytes()<=16777216);
        client.undo(); QTRY_COMPARE(client.redoDepth(),1);
        QTRY_VERIFY(client.frameRevision()>=client.revision());
        QCOMPARE(qAlpha(client.frame().pixel(64,64)),0);
        client.requestViewport(1,{}, {},false); QVERIFY(client.frame(1).isNull());
        client.newTransparentDocument(64,64); QTRY_VERIFY(client.ready() && client.documentWidth()==64);
        QTRY_COMPARE(client.frame().size(),QSize(64,64));
        QCOMPARE(client.frameRegion(),QRectF(0,0,64,64));
        QVERIFY(client.lastError().isEmpty());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void retiredNotesMigrateWithoutResettingWorkspace() {
        QTemporaryDir temp; const auto path=temp.filePath("legacy.ini");
        WorkspaceManager original(path); const auto palette=original.addCustomPanel("Colors","palette");
        const auto source=original.rightGroups().first().toMap().value("id").toString();
        original.detachPanel(source,palette); original.saveLayout();
        QSettings settings(path,QSettings::IniFormat);
        auto root=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();
        auto panels=root.value("panels").toObject();
        panels.insert("old-note",QJsonObject{{"kind","notes"},{"title",QStringLiteral("灵感随笔")},{"content","old text"},{"custom",true}});
        auto groups=root.value("groups").toArray();
        auto mixed=groups.first().toObject(); auto tabs=mixed.value("panels").toArray();
        tabs.append("old-note"); mixed.insert("panels",tabs); mixed.insert("active","old-note"); groups[0]=mixed;
        panels.insert("floating-note",QJsonObject{{"kind","notes"},{"title","Note"}});
        groups.append(QJsonObject{{"id","legacy-note-window"},{"location","floating"},{"active","floating-note"},{"panels",QJsonArray{"floating-note"}}});
        root.insert("version",1); root.insert("groups",groups); root.insert("panels",panels);
        settings.setValue("workspace",QJsonDocument(root).toJson()); settings.sync();
        WorkspaceManager migrated(path);
        QCOMPARE(migrated.floatingGroups().size(),1); // The floating palette survives.
        QCOMPARE(migrated.panelDefinition(palette).value("kind").toString(),QString("palette"));
        QVERIFY(migrated.panelDefinition("old-note").isEmpty());
        QVERIFY(migrated.panelDefinition("floating-note").isEmpty());
        QCOMPARE(migrated.rightGroups().first().toMap().value("active").toString(),QString("color"));
        settings.sync(); const auto saved=QJsonDocument::fromJson(settings.value("workspace").toByteArray()).object();
        QCOMPARE(saved.value("version").toInt(),6); QVERIFY(!saved.value("panels").toObject().contains("old-note"));
        WorkspaceManager reopened(path); QCOMPARE(reopened.floatingGroups().size(),1);
    }
    void queueBackpressureRollsBack() {
        PaintCoreClient client; QTRY_VERIFY_WITH_TIMEOUT(client.ready(),10000);
        client.newTransparentDocument(128,128); QTRY_COMPARE(client.frame().width(),128);
        client.setBrushRadius(8);
        InputSample s; s.position={32,64}; QVERIFY(client.beginStroke(s));
        // A burst deliberately outruns the worker. The GUI loop must not wait for it.
        QElapsedTimer elapsed; elapsed.start();
        for(int i=0;i<30000;++i) { s.position={32.+(i%2)*32,64}; client.strokeTo(s); }
        client.endStroke(); QVERIFY(elapsed.elapsed()<3000);
        QVERIFY(!client.lastError().isEmpty());
        QTest::qWait(100);
        QTRY_COMPARE_WITH_TIMEOUT(qAlpha(client.frame().pixel(32,64)),0,10000);
        QCOMPARE(client.undoDepth(),0);
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
    }
    void qmlMouseTabletAndFloatingWindows() {
        QTemporaryDir temp;
        PaintCoreClient client; WorkspaceManager workspace(temp.filePath("qml.ini"));
        QQmlApplicationEngine engine; QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors) { for(const auto &error:errors) warnings.append(error.toString()); });
        engine.rootContext()->setContextProperty("PaintClient",&client);
        engine.rootContext()->setContextProperty("Workspace",&workspace);
        engine.load(QUrl("qrc:/qml/app/Main.qml"));
        QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));
        auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first()); QVERIFY(window);
        QTRY_VERIFY_WITH_TIMEOUT(client.ready() && !client.frame().isNull(),10000);
        auto *canvas=window->findChild<CanvasItem*>("mainCanvas"); QVERIFY(canvas);
        auto *busy=findVisualItem(window->contentItem(),"canvasBusy"); QVERIFY(busy);
        QTRY_VERIFY(!busy->isVisible());
        QCOMPARE(applicationWindows().size(),1);
        QCOMPARE(workspace.floatingGroups().size(),0);
        QTRY_VERIFY(canvas->width()>300); canvas->fitToView();
        const QString navigatorGroup=workspace.rightGroups().last().toMap().value("id").toString();
        auto *navigatorDock=findVisualItem(window->contentItem(),"dockGroup:"+navigatorGroup); QVERIFY(navigatorDock);
        QVERIFY(navigatorDock->setProperty("selected","navigator"));
        workspace.setActive(navigatorGroup,"navigator");
        QTRY_VERIFY(!client.frame(1).isNull());
        QVERIFY(navigatorDock->setProperty("selected","layers"));
        workspace.setActive(navigatorGroup,"layers");
        QTRY_COMPARE(client.frame().width(),qCeil(canvas->documentRect().width()*window->devicePixelRatio()));
        QVERIFY(qint64(client.frame().width())*client.frame().height()<=4194304);
        const auto rect=canvas->documentRect();
        QCOMPARE(canvas->documentPoint(rect.topLeft()),QPointF(0,0));
        const QPoint center=canvas->mapToScene(rect.center()).toPoint();
        QTest::mouseClick(window,Qt::LeftButton,Qt::NoModifier,center);
        QTRY_COMPARE(client.undoDepth(),1);
        QPointingDevice pen("Test tablet",999,QInputDevice::DeviceType::Stylus,QPointingDevice::PointerType::Pen,
                            QInputDevice::Capability::Position|QInputDevice::Capability::Pressure|QInputDevice::Capability::XTilt|QInputDevice::Capability::Rotation,1,2);
        const QPointF pos=center+QPoint(60,30);
        QTabletEvent press(QEvent::TabletPress,&pen,pos,window->mapToGlobal(pos.toPoint()),.75f,20,10,.1f,30,0,Qt::NoModifier,Qt::LeftButton,Qt::LeftButton);
        QCoreApplication::sendEvent(window,&press); QVERIFY(press.isAccepted());
        QTabletEvent release(QEvent::TabletRelease,&pen,pos,window->mapToGlobal(pos.toPoint()),0,20,10,.1f,30,0,Qt::NoModifier,Qt::LeftButton,Qt::NoButton);
        QCoreApplication::sendEvent(window,&release); QTRY_COMPARE(client.undoDepth(),2);
        const QPointF local=canvas->mapFromScene(center);
        const auto beforeZoom=canvas->documentPoint(local);
        QWheelEvent wheel(center,window->mapToGlobal(center),{},QPoint(0,120),Qt::NoButton,Qt::NoModifier,Qt::NoScrollPhase,false);
        QCoreApplication::sendEvent(window,&wheel);
        QVERIFY(QLineF(beforeZoom,canvas->documentPoint(local)).length()<.001);
        const auto beforePan=canvas->documentPoint(local);
        QTest::mousePress(window,Qt::MiddleButton,Qt::NoModifier,center);
        QTest::mouseMove(window,center+QPoint(40,20));
        QTest::mouseRelease(window,Qt::MiddleButton,Qt::NoModifier,center+QPoint(40,20));
        QVERIFY(QLineF(beforePan,canvas->documentPoint(local+QPointF(40,20))).length()<.001);
        QCOMPARE(client.undoDepth(),2);
        const auto group=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachGroup(group); QTRY_COMPARE(workspace.floatingGroups().size(),1);
        QTRY_COMPARE(applicationWindows().size(),2);
        workspace.returnGroup(workspace.floatingGroups().first().toMap().value("id").toString());
        QTRY_COMPARE(applicationWindows().size(),1);
        const QString notes=workspace.addCustomPanel("Palette","palette");
        auto source=workspace.rightGroups().first().toMap().value("id").toString();
        workspace.detachPanel(source,notes); QTRY_COMPARE(applicationWindows().size(),2);
        auto floating=workspace.floatingGroups().first().toMap().value("id").toString();
        QQuickItem *target=nullptr;
        QTRY_VERIFY((target=findVisualItem(window->contentItem(),"dockGroup:"+source)));
        QTRY_VERIFY(target->width()>=180 && target->height()>=180);
        const auto dropPosition=target->mapToScene(QPointF(target->width()/2,18)).toPoint();
        QMimeData mime;
        mime.setData("application/x-drawverse-panel",QJsonDocument(QJsonObject{{"group",floating},{"panel",notes},{"whole",false}}).toJson());
        QDragEnterEvent enter(dropPosition,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);
        QCoreApplication::sendEvent(window,&enter); QVERIFY(enter.isAccepted());
        QDropEvent drop(dropPosition,Qt::MoveAction,&mime,Qt::LeftButton,Qt::NoModifier);
        QCoreApplication::sendEvent(window,&drop); QVERIFY(drop.isAccepted());
        QTRY_COMPARE(workspace.floatingGroups().size(),0); QTRY_COMPARE(applicationWindows().size(),1);
        QCOMPARE(workspace.rightGroups().size(),2);
        QVERIFY(workspace.rightGroups().first().toMap().value("panels").toStringList().contains(notes));
        workspace.detachPanel(source,notes); QTRY_COMPARE(applicationWindows().size(),2);
        QQuickWindow *floatingWindow=nullptr;
        for(auto *candidate:applicationWindows()) if(candidate!=window) floatingWindow=qobject_cast<QQuickWindow*>(candidate);
        QVERIFY(floatingWindow); floatingWindow->close();
        QTRY_COMPARE(workspace.floatingGroups().size(),0); QTRY_COMPARE(applicationWindows().size(),1);
        QCOMPARE(warnings,QStringList());
        QSignalSpy stopped(&client,&PaintCoreClient::stopped); client.shutdown(); QTRY_COMPARE(stopped.size(),1);
        QVERIFY(window->close()); QVERIFY(!window->isVisible());
    }
    void panelTabsRetractWithoutExpandingRailsAndKeepSharedPresentation() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join('\n')));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        // The default right column groups navigator/layers/history with the shared colour panel.
        const auto navigator=workspace.groupForPanel("navigator");
        QVERIFY(!navigator.isEmpty());QVERIFY(!workspace.groupForPanel("color").isEmpty());
        for(const auto &id:workspace.columnGroups(navigator))workspace.setColumnCollapsed(id,true);
        for(const auto &id:workspace.columnGroups(navigator))QVERIFY(workspace.groupDefinition(id).value("icons").toBool());
        auto *icon=findVisualItem(main->contentItem(),"railPanel:navigator");QVERIFY(icon);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());
        auto flyout=[&]()->QQuickWindow*{for(auto *window:applicationWindows())if(window->isVisible() && window->objectName().startsWith("panelFlyout:") && window->objectName().endsWith(":navigator"))return qobject_cast<QQuickWindow*>(window);return nullptr;};QTRY_VERIFY(flyout());
        QPointer<QQuickWindow> row=flyout();
        // Other already-open category windows stay untouched.
        auto *colorIcon=findVisualItem(main->contentItem(),"railPanel:color");QVERIFY(colorIcon);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,colorIcon->mapToScene({14,14}).toPoint());
        auto other=[&]()->QQuickWindow*{for(auto *window:applicationWindows())if(window->isVisible() && window->objectName().startsWith("panelFlyout:") && window->objectName().endsWith(":color") && window!=row)return qobject_cast<QQuickWindow*>(window);return nullptr;};QTRY_VERIFY(other());QPointer<QQuickWindow> otherRow=other();
        // Request 11: the shared top drag/tab bar of a floating column is translucent 75%; the docked bar stays opaque.
        auto *flyoutBar=findVisualItem(row->contentItem(),"panelTabBar:"+navigator);QVERIFY(flyoutBar);QVERIFY(std::abs(flyoutBar->property("color").value<QColor>().alphaF()-.75)<.01);
        // Request 6: double clicking the header retracts this presentation without re-expanding the source rail.
        auto *header=findVisualItem(row->contentItem(),"groupGrip:"+navigator);QVERIFY(header);QTest::mouseDClick(row,Qt::LeftButton,Qt::NoModifier,header->mapToScene({10,4}).toPoint());QTRY_VERIFY(row.isNull());
        for(const auto &id:workspace.columnGroups(navigator))QVERIFY(workspace.groupDefinition(id).value("icons").toBool());
        QVERIFY(icon->isVisible());QCOMPARE(workspace.groupDefinition(navigator).value("icons").toBool(),true);QVERIFY(!otherRow.isNull());
        // Request 6: a panel tab double click retracts the same way and must not expand the collapsed rail.
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,icon->mapToScene({14,14}).toPoint());QTRY_VERIFY(flyout());row=flyout();
        auto *tab=findVisualItem(row->contentItem(),"panelTab:layers");QVERIFY(tab);QTest::mouseDClick(row,Qt::LeftButton,Qt::NoModifier,tab->mapToScene({8,10}).toPoint());QTRY_VERIFY(row.isNull());
        for(const auto &id:workspace.columnGroups(navigator))QVERIFY(workspace.groupDefinition(id).value("icons").toBool());
        QVERIFY(!otherRow.isNull());QVERIFY(workspace.groupDefinition(workspace.groupForPanel("color")).value("icons").toBool());
        otherRow->close();QTRY_VERIFY(otherRow.isNull());
        // Docked columns keep their semantics: a tab double click still collapses the column into its icon rail.
        workspace.setColumnCollapsed(navigator,false);
        auto *dockedBar=findVisualItem(main->contentItem(),"panelTabBar:"+navigator);QVERIFY(dockedBar);QTRY_VERIFY(std::abs(dockedBar->property("color").value<QColor>().alphaF()-1.)<.01);
        auto *dockedTab=findVisualItem(main->contentItem(),"panelTab:layers");QVERIFY(dockedTab);QTest::mouseDClick(main,Qt::LeftButton,Qt::NoModifier,dockedTab->mapToScene({8,10}).toPoint());
        QTRY_VERIFY(workspace.groupDefinition(navigator).value("icons").toBool());for(const auto &id:workspace.columnGroups(navigator))QVERIFY(workspace.groupDefinition(id).value("icons").toBool());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void textFieldsKeepSpaceAndHistoryReplayStaysStepped() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        client.newTransparentDocument(48,48);QTRY_COMPARE(client.documentWidth(),48);QTRY_VERIFY(client.ready());client.setBrushRadius(3);client.setBrushColor(Qt::black);
        // Request 25: a focused text input keeps space while the canvas stays out of the shared pan state.
        auto *view=main->findChild<CanvasItem*>("mainCanvas");QVERIFY(view);
        QQmlComponent component(&engine);
        component.setData("import QtQuick.Controls\nTextField { objectName:\"spacePanField\" }",QUrl());
        auto *field=qobject_cast<QQuickItem*>(component.create(engine.rootContext()));
        QVERIFY(field);field->setParentItem(main->contentItem());field->setWidth(120);field->setHeight(24);field->setProperty("text",QString("78"));field->forceActiveFocus();QTRY_COMPARE(main->activeFocusItem(),field);
        const auto undoBefore=client.undoDepth();QTest::keyClick(main,Qt::Key_Space);QVERIFY(!view->spacePanning());QTRY_COMPARE(field->property("text").toString(),QString("78 "));QCOMPARE(client.undoDepth(),undoBefore);
        QTest::keyClick(main,Qt::Key_Backspace);QTRY_COMPARE(field->property("text").toString(),QString("78"));
        // Request 27: four replay steps must be observable one at a time, not an instant jump.
        const auto group=workspace.groupForPanel("history");workspace.setActive(group,"history");
        auto *historyTab=findVisualItem(main->contentItem(),"panelTab:history");QVERIFY(historyTab);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,historyTab->mapToScene({8,10}).toPoint());QTRY_VERIFY(findVisualItem(main->contentItem(),"historyList"));
        client.setBrushRadius(3);InputSample sample;sample.position={24,24};sample.pressure=1;
        for(int i=0;i<4;++i) {const auto stroke=i+1;client.setBrushColor(i%2?Qt::black:Qt::white);QVERIFY(client.beginStroke(sample));client.endStroke();QTRY_COMPARE(client.undoDepth(),stroke);QTRY_VERIFY(!client.layerEditBusy());}
        QQuickItem *list=findVisualItem(main->contentItem(),"historyList");QVERIFY(list);QTRY_VERIFY(list->property("count").toInt()>0);
        auto *timer=main->findChild<QObject*>("historyReplayTimer");QVERIFY(timer);QCOMPARE(timer->property("interval").toInt(),20);QVERIFY(!timer->property("running").toBool());
        auto *previous=findVisualItem(main->contentItem(),"historyEntry:0");QVERIFY(previous);QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,previous->mapToScene({20,12}).toPoint());
        QTRY_VERIFY(timer->property("running").toBool());
        QList<int> seen;QStringList trace;
        for(int i=0;i<40;++i) {const int depth=client.undoDepth();if(seen.isEmpty() || seen.last()!=depth){seen.append(depth);trace.append(QString::number(depth));}if(depth==0)break;QTest::qWait(5);}
        QTRY_COMPARE(client.undoDepth(),0);QCOMPARE(client.redoDepth(),4);
        QVERIFY2(seen.size()>=3 && seen.first()==4 && seen.last()==0,qPrintable("single-step replay did not walk through every depth: "+trace.join(",")));
        bool stoppedRunning=false;
        for(int i=0;i<100 && !stoppedRunning;++i) {stoppedRunning=!timer->property("running").toBool();if(!stoppedRunning)QTest::qWait(5);}
        QVERIFY2(stoppedRunning,qPrintable(QString("replay timer kept running: undo=%1 redo=%2 target=%3").arg(client.undoDepth()).arg(client.redoDepth()).arg(list->parentItem()?list->parentItem()->property("targetDepth").toInt():-99)));
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void editMenuOpensKeyboardShortcutList() {
        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        auto *action=main->findChild<QObject*>("keyboardShortcutsAction");QVERIFY(action);QVERIFY(action->property("enabled").toBool());
        auto *dialog=main->findChild<QObject*>("keyboardShortcuts");QVERIFY(dialog);QVERIFY(QMetaObject::invokeMethod(action,"triggered"));
        QTRY_VERIFY(dialog->property("opened").toBool());
        // The list is real: one row per bound shortcut. Assert on the rendered pixels so a dialog
        // that only reports a model without drawing rows would still fail.
        QObject *list=nullptr;QTRY_VERIFY((list=dialog->findChild<QObject*>("shortcutList")));QTRY_COMPARE(list->property("count").toInt(),26);
        auto *dialogContent=qobject_cast<QQuickItem*>(dialog->property("contentItem").value<QObject*>());QVERIFY(dialogContent);
        QQuickItem *listItem=qobject_cast<QQuickItem*>(list);QVERIFY(listItem);
        QImage shot;bool grabbed=false;
        QSharedPointer<QQuickItemGrabResult> grab=dialogContent->grabToImage();
        QVERIFY(grab);connect(grab.data(),&QQuickItemGrabResult::ready,this,[&]{shot=grab->image();grabbed=true;});
        QTRY_VERIFY(grabbed);QVERIFY(!shot.isNull());
        const auto origin=listItem->mapToScene({0,0})-dialogContent->mapToScene({0,0});int painted=0;
        for(int y=0;y<qMin(60,shot.height()-int(origin.y()));++y)for(int x=0;x<qMin(200,shot.width());++x)
            if(shot.pixelColor(int(origin.x())+x,int(origin.y())+y).value()>90)++painted;
        QVERIFY2(painted>40,qPrintable(QString("shortcut rows did not render: %1 lit pixels").arg(painted)));
        QVERIFY(QMetaObject::invokeMethod(dialog,"close"));QTRY_VERIFY(!dialog->property("opened").toBool());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
    void selectionToolVariantsUseCornerMarkAndRightClickPanel() {        QTemporaryDir temp;PaintCoreClient client(nullptr,temp.filePath("storage.ini"));WorkspaceManager workspace(temp.filePath("layout.ini"));QQmlApplicationEngine engine;QStringList warnings;
        connect(&engine,&QQmlEngine::warnings,this,[&](const QList<QQmlError>&errors){for(const auto &e:errors)warnings.append(e.toString());});engine.rootContext()->setContextProperty("PaintClient",&client);engine.rootContext()->setContextProperty("Workspace",&workspace);engine.load(QUrl("qrc:/qml/app/Main.qml"));QVERIFY2(!engine.rootObjects().isEmpty(),qPrintable(warnings.join("\n")));auto *main=qobject_cast<QQuickWindow*>(engine.rootObjects().first());QVERIFY(main);QTRY_VERIFY(client.ready());
        auto *selection=findVisualItem(main->contentItem(),"selectionTool");QVERIFY(selection);
        // The corner triangle is the affordance that tells the user the slot has more shapes.
        QObject *mark=main->findChild<QObject*>("variantMark:selectionTool");QVERIFY(mark);QVERIFY(mark->property("visible").toBool());
        auto *panel=main->findChild<QObject*>("toolVariantPanel");QVERIFY(panel);QTRY_VERIFY(!panel->property("visible").toBool());
        // Left click only activates the tool, it never switches the shape.
        client.setSelectionTool(1);QTRY_COMPARE(client.selectionTool(),1);
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,selection->mapToScene({13,13}).toPoint());QTRY_COMPARE(client.selectionTool(),1);
        // Right click opens the floating list of shapes.
        QTest::mouseClick(main,Qt::RightButton,Qt::NoModifier,selection->mapToScene({13,13}).toPoint());QTRY_VERIFY(panel->property("visible").toBool());
        // Popup content belongs to the popup's own window, so search it from the panel content.
        auto *panelContent=qobject_cast<QQuickItem*>(panel->property("contentItem").value<QObject*>());QVERIFY(panelContent);
        auto *rectangle=findVisualItem(panelContent,"variantEntry:rectangle"),*ellipse=findVisualItem(panelContent,"variantEntry:ellipse");
        QVERIFY(rectangle && ellipse);
        QVERIFY(rectangle->isEnabled());QVERIFY(ellipse->isEnabled());
        // The wand remains unsupported; the lasso is now a separate usable family.
        QVERIFY(findVisualItem(main->contentItem(),"wandTool"));
        auto *lassoSlot=qobject_cast<QQuickItem*>(findVisualItem(main->contentItem(),"lassoTool"));
        QVERIFY(lassoSlot);QVERIFY(lassoSlot->isEnabled());
        QVERIFY(!findVisualItem(panelContent,"variantEntry:lasso"));
        QVERIFY(!findVisualItem(panelContent,"variantEntry:wand"));
        // Picking a shape from the panel is what changes the active shape.
        QTest::mouseClick(main,Qt::LeftButton,Qt::NoModifier,findVisualItem(panelContent,"variantTrigger:ellipse")->mapToScene({20,15}).toPoint());QTRY_COMPARE(client.selectionTool(),2);QTRY_VERIFY(!panel->property("visible").toBool());
        QTest::mouseClick(main,Qt::RightButton,Qt::NoModifier,selection->mapToScene({13,13}).toPoint());QTRY_VERIFY(panel->property("visible").toBool());
        auto *selectedState=findVisualItem(panelContent,"toolVariantState:ellipse");QVERIFY(selectedState);QTRY_COMPARE(selectedState->property("text").toString(),QString("✓"));
        QVERIFY(QMetaObject::invokeMethod(panel,"close"));QTRY_VERIFY(!panel->property("visible").toBool());
        QCOMPARE(warnings,QStringList());QSignalSpy stopped(&client,&PaintCoreClient::stopped);client.shutdown();QTRY_COMPARE(stopped.size(),1);
    }
};
int main(int argc,char **argv) {
    configureUiScale();
    QQuickWindow::setDefaultAlphaBuffer(true);
#ifdef Q_OS_WIN
    // Offscreen does not use the Windows font database. Use installed system fonts.
    if (qEnvironmentVariable("QT_QPA_PLATFORM") == "offscreen")
        qputenv("QT_QPA_FONTDIR",QDir(qEnvironmentVariable("WINDIR")).filePath("Fonts").toUtf8());
#endif
    QGuiApplication app(argc,argv); Q_INIT_RESOURCE(ui_resources);
    QQuickStyle::setStyle("Basic"); qmlRegisterType<CanvasItem>("DrawVerse",1,0,"PaintCanvas");
    qmlRegisterType<ColorWheelItem>("DrawVerse",1,0,"ColorWheel");
    qmlRegisterType<SelectionOverlay>("DrawVerse",1,0,"SelectionOutline");
    qmlRegisterType<MenuSurface>("DrawVerse",1,0,"MenuSurface");
    qmlRegisterType<DocumentManager>("DrawVerse",1,0,"DocumentManager");
    UiTests tests; return QTest::qExec(&tests,argc,argv);
}
#include "ui_tests.moc"
