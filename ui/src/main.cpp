#include "CanvasItem.h"
#include "ColorWheelItem.h"
#include "SelectionOverlay.h"
#include "WorkspaceManager.h"
#include "PaintingBenchmark.h"
#include "UiScale.h"
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickWindow>
#include <QQuickStyle>
#include <QTimer>
#include <QPalette>
#include <QIcon>
#include <QTemporaryDir>
#include <cmath>

int main(int argc,char **argv) {
    configureUiScale();
    QGuiApplication app(argc,argv);
    app.setOrganizationName("DrawVerse"); app.setApplicationName("DrawVerse");
    if(app.arguments().contains("--migrate-workspace")) { WorkspaceManager workspace; workspace.saveLayout(); return 0; }
    Q_INIT_RESOURCE(ui_resources);
    app.setWindowIcon(QIcon(":/qml/assets/logo.png"));
    QQuickStyle::setStyle("Basic");
    QPalette palette;
    palette.setColor(QPalette::Window,QColor("#1c1e21")); palette.setColor(QPalette::WindowText,QColor("#bababa"));
    palette.setColor(QPalette::Base,QColor("#111214")); palette.setColor(QPalette::Text,QColor("#bababa"));
    palette.setColor(QPalette::Button,QColor("#25262b")); palette.setColor(QPalette::ButtonText,QColor("#bababa"));
    palette.setColor(QPalette::Highlight,QColor("#3b3d42")); palette.setColor(QPalette::HighlightedText,QColor("#23b5ee"));
    app.setPalette(palette);
    qmlRegisterType<CanvasItem>("DrawVerse",1,0,"PaintCanvas");
    qmlRegisterType<ColorWheelItem>("DrawVerse",1,0,"ColorWheel");
    qmlRegisterType<SelectionOverlay>("DrawVerse",1,0,"SelectionOutline");
    const QStringList arguments=app.arguments();
    const int previewIndex=arguments.indexOf("--preview");
    const QString previewPath=previewIndex>=0 && previewIndex+1<arguments.size()?arguments[previewIndex+1]:QString();
    const int benchmarkIndex=arguments.indexOf("--benchmark");
    const QString benchmarkPath=benchmarkIndex>=0 && benchmarkIndex+1<arguments.size()?arguments[benchmarkIndex+1]:QString();
    const QString testPath=!benchmarkPath.isEmpty()?benchmarkPath:previewPath;
    QTemporaryDir testPreferences;
    if(!testPath.isEmpty() && !testPreferences.isValid())return 1;
    PaintCoreClient client(nullptr,testPath.isEmpty()?QString():testPreferences.filePath("preferences.ini"));
    WorkspaceManager workspace(testPath.isEmpty()?QString():testPath+".workspace.ini");
    if(!previewPath.isEmpty()) workspace.resetLayout();
    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty("PaintClient",&client);
    engine.rootContext()->setContextProperty("Workspace",&workspace);
    engine.load(QUrl("qrc:/qml/Main.qml"));
    if(engine.rootObjects().isEmpty()) return 1;
    auto *mainWindow=qobject_cast<QQuickWindow*>(engine.rootObjects().first());
    if(workspace.needsReferenceLayout() || !testPath.isEmpty()) {
        workspace.applyReferenceLayout(mainWindow->x(),mainWindow->y(),mainWindow->width(),mainWindow->height());
        client.setBrushColor(QColor("#f5e3ce"));
        if(testPath.isEmpty())workspace.saveLayout();
    }
    QObject::connect(&client,&PaintCoreClient::stopped,&app,&QCoreApplication::quit);
    if(!benchmarkPath.isEmpty()) {
        startPaintingBenchmark(client,*qobject_cast<QQuickWindow*>(engine.rootObjects().first()),benchmarkPath);
    }
    if(!previewPath.isEmpty()) {
        // Opt-in deterministic visual QA draws through the same ABI adapter as user input.
        auto *timer=new QTimer(&app); timer->setInterval(100);
        QObject::connect(timer,&QTimer::timeout,&app,[&,timer,stage=0]() mutable {
            if(!client.ready()) return;
            if(stage==0 && !client.frame().isNull()) {
                client.setBrushColor(QColor("#2ea99d")); client.setBrushRadius(24);
                InputSample s; s.capabilities=1; s.tool=1;
                for(int stroke=0;stroke<3;++stroke) {
                    s.position={170.,200.+stroke*100}; s.pressure=.15f; client.beginStroke(s);
                    for(int i=1;i<=80;++i) { s.position={170.+i*7,200.+stroke*100+std::sin(i*.07)*65}; s.pressure=static_cast<float>(.15+.8*std::sin(i*3.14159265/160)); client.strokeTo(s); }
                    client.endStroke();
                }
                stage=1;
            } else if(stage==1 && client.undoDepth()==3) { stage=2; }
            else if(stage==2 && client.frameRevision() >= client.revision()) {
                auto *window=qobject_cast<QQuickWindow*>(engine.rootObjects().first());
                const bool saved=window && window->grabWindow().save(previewPath);
                if(!saved) qWarning("Preview screenshot could not be saved");
                timer->stop(); client.shutdown();
            }
        });
        timer->start();
        QTimer::singleShot(30000,&app,[&] { client.shutdown(); });
    }
    return app.exec();
}
