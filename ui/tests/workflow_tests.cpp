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

};
int main(int argc,char **argv) {
    QGuiApplication app(argc,argv);Q_INIT_RESOURCE(ui_resources);QQuickStyle::setStyle("Basic");
    WorkflowTests test;return QTest::qExec(&test,argc,argv);
}
#include "workflow_tests.moc"
