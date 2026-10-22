#include "platform/UiResources.h"
#include "canvas/CanvasItem.h"
#include "color/ColorWheelItem.h"
#include "canvas/SelectionOverlay.h"
#include "workspace/WorkspaceManager.h"
#include "documents/DocumentManager.h"
#include "app/PaintingBenchmark.h"
#include "platform/UiScale.h"
#include "platform/TabletInput.h"
#include <QSettings>
#include "platform/MenuSurface.h"
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickWindow>
#include <QQuickStyle>
#include <QTimer>
#include <QPalette>
#include <QIcon>
#include <QTemporaryDir>
#include <QDir>
#include <QScreen>
#ifdef DRAWVERSE_QML_SOURCE_DIR
#include "app/QmlSourceWatcher.h"
#include <QCommandLineParser>
#include <QFileInfo>
#include <QQmlDebuggingEnabler>
#endif
#include <cmath>
#include <memory>

int main(int argc,char **argv) {
    configureUiScale();
    QQuickWindow::setDefaultAlphaBuffer(true);
    QGuiApplication app(argc,argv);
    app.setOrganizationName("DrawVerse"); app.setApplicationName("DrawVerse");
    const auto profileIndex=app.arguments().indexOf("--profile-dir");
    QString profileDir;
    if(profileIndex>=0) {
        if(profileIndex+1>=app.arguments().size() || app.arguments()[profileIndex+1].startsWith("--")){qCritical("--profile-dir requires a directory");return 2;}
        profileDir=QDir(app.arguments()[profileIndex+1]).absolutePath();
        if(!QDir().mkpath(profileDir)){qCritical("Cannot create profile directory");return 2;}
        qInfo().noquote()<<"Isolated profile:"<<profileDir;
        for(auto *screen:app.screens())qInfo()<<"Screen:"<<screen->name()<<"logical DPI:"<<screen->logicalDotsPerInch()<<"DPR:"<<screen->devicePixelRatio();
    }
    const auto inputSettings=profileDir.isEmpty()?std::make_unique<QSettings>(QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse")
        :std::make_unique<QSettings>(QDir(profileDir).filePath("DrawVerse.ini"),QSettings::IniFormat);
    configureTabletInput(inputSettings->value("input/tabletApi","auto").toString());
#ifdef DRAWVERSE_QML_SOURCE_DIR
    // Only this separate developer executable enables the Qt Creator preview service.
    QQmlDebuggingEnabler::enableDebugging(true);
    QCommandLineParser parser;
    parser.setApplicationDescription("DrawVerse QML source preview (temporary preferences and workspace)");
    parser.addHelpOption();
    parser.addOption({"qml-root","Directory containing the editable QML sources.","directory",DRAWVERSE_QML_SOURCE_DIR});
    parser.addOption({"qml-entry","QML window to load, relative to --qml-root.","file","app/Main.qml"});
    parser.addOption({"canvas","Create a temporary canvas in app/Main.qml."});
    parser.addOption({"watch","Reload the isolated preview session when QML or an asset is saved."});
    parser.addOption({"profile-dir","Use an explicit isolated preferences and workspace directory.","directory"});
    parser.process(app);
    qputenv("QML_DISABLE_DISK_CACHE","1");
    const QDir qmlRoot(parser.value("qml-root"));
    const QFileInfo qmlEntry(qmlRoot.filePath(parser.value("qml-entry")));
    if(!qmlEntry.isFile()) {qCritical().noquote()<<"QML preview entry does not exist:"<<qmlEntry.absoluteFilePath();return 2;}
    if(parser.isSet("canvas") && parser.value("qml-entry")!="app/Main.qml") {qCritical("--canvas requires app/Main.qml");return 2;}
#endif
    if(app.arguments().contains("--migrate-workspace")) { WorkspaceManager workspace(profileDir.isEmpty()?QString():QDir(profileDir).filePath("workspace.ini")); workspace.saveLayout(); return 0; }
    initializeDrawVerseUiResources();
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
    qmlRegisterType<MenuSurface>("DrawVerse",1,0,"MenuSurface");
    qmlRegisterType<DocumentManager>("DrawVerse",1,0,"DocumentManager");
#ifdef DRAWVERSE_QML_SOURCE_DIR
    bool reloadRequested=false;
    const auto runPreview=[&]() -> int {
#endif
    const QStringList arguments=app.arguments();
    const int previewIndex=arguments.indexOf("--preview");
    const QString previewPath=previewIndex>=0 && previewIndex+1<arguments.size()?arguments[previewIndex+1]:QString();
    const int benchmarkIndex=arguments.indexOf("--benchmark");
    const QString benchmarkPath=benchmarkIndex>=0 && benchmarkIndex+1<arguments.size()?arguments[benchmarkIndex+1]:QString();
    const QString testPath=!benchmarkPath.isEmpty()?benchmarkPath:previewPath;
    QTemporaryDir testPreferences;
    bool isolatedPreferences=!testPath.isEmpty();
#ifdef DRAWVERSE_QML_SOURCE_DIR
    isolatedPreferences=true;
#endif
    if(isolatedPreferences && !testPreferences.isValid())return 1;
    PaintCoreClient client(nullptr,!profileDir.isEmpty()?QDir(profileDir).filePath("DrawVerse.ini"):isolatedPreferences?testPreferences.filePath("preferences.ini"):QString());
    QString workspacePath=!profileDir.isEmpty()?QDir(profileDir).filePath("workspace.ini"):testPath.isEmpty()?QString():testPath+".workspace.ini";
#ifdef DRAWVERSE_QML_SOURCE_DIR
    if(profileDir.isEmpty())workspacePath=testPreferences.filePath("workspace.ini");
    qInfo().noquote()<<"QML preview source:"<<qmlEntry.absoluteFilePath();
    qInfo().noquote()<<"QML preview preferences:"<<(profileDir.isEmpty()?testPreferences.path():profileDir);
#endif
    WorkspaceManager workspace(workspacePath);
#ifndef DRAWVERSE_QML_SOURCE_DIR
    if(!workspace.applyDeveloperConsole())qWarning("Developer console could not be opened");
#endif
    if(!previewPath.isEmpty()) workspace.resetLayout();
#ifdef DRAWVERSE_QML_SOURCE_DIR
    std::unique_ptr<QmlSourceWatcher> sourceWatcher;
    if(parser.isSet("watch"))sourceWatcher=std::make_unique<QmlSourceWatcher>(qmlRoot.absolutePath(),[&] {
        qInfo("QML source changed; restarting the temporary preview session");
        reloadRequested=true;app.exit(0);
    });
#endif
    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty("PaintClient",&client);
    engine.rootContext()->setContextProperty("Workspace",&workspace);
    if(!testPath.isEmpty())engine.setInitialProperties({{"createInitialDocument",true}});
#ifdef DRAWVERSE_QML_SOURCE_DIR
    if(parser.isSet("canvas"))engine.setInitialProperties({{"createInitialDocument",true}});
    engine.load(QUrl::fromLocalFile(qmlEntry.absoluteFilePath()));
#else
    engine.load(QUrl("qrc:/qml/app/Main.qml"));
#endif
    if(engine.rootObjects().isEmpty()) {
#ifdef DRAWVERSE_QML_SOURCE_DIR
        if(!parser.isSet("watch"))return 1;
        engine.loadData(R"(
import QtQuick
import QtQuick.Window
Window {
    visible: true; width: 640; height: 240; color: "#17191c"; title: "DrawVerse QML preview error"
    Text {
        anchors.fill: parent; anchors.margins: 24; color: "#efb2ab"; wrapMode: Text.Wrap
        text: "QML 加载失败。请查看 Qt Creator 的应用输出，修正源码并保存后会自动重试。"
    }
}
)");
        if(engine.rootObjects().isEmpty())return 1;
#else
        return 1;
#endif
    }
    if(!testPath.isEmpty())QObject::connect(&client,&PaintCoreClient::stopped,&app,&QCoreApplication::quit);
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
#ifdef DRAWVERSE_QML_SOURCE_DIR
    };
    int previewResult=0;
    do {reloadRequested=false;previewResult=runPreview();}while(reloadRequested);
    return previewResult;
#endif
}
