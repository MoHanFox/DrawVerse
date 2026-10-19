#include "engine/PaintCoreClient.h"
#include <QtTest>
#include <QQmlEngine>
#include <QQmlComponent>
#include <QQuickStyle>
#include <QTemporaryDir>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>

class WorkflowTests : public QObject {
    Q_OBJECT
private slots:
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
    WorkflowTests test;return QTest::qExec(&test,argc,argv);
}
#include "workflow_tests.moc"
