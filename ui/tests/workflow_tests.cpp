#include "engine/PaintCoreClient.h"
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
#include "platform/MenuSurface.h"
#include "workspace/WorkspaceManager.h"

static QQuickItem *findVisual(QQuickItem *item,const QString &name){if(item->objectName()==name)return item;for(auto *child:item->childItems())if(auto *found=findVisual(child,name))return found;return nullptr;}

class WorkflowTests : public QObject {
    Q_OBJECT
private slots:
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
    qmlRegisterType<MenuSurface>("DrawVerse",1,0,"MenuSurface");
    WorkflowTests test;return QTest::qExec(&test,argc,argv);
}
#include "workflow_tests.moc"
