#include "platform/TabletInput.h"
#include <QGuiApplication>
#ifdef Q_OS_WIN
#include <QtGui/private/qguiapplication_p.h>
#endif

namespace {QString activeBackend=QStringLiteral("系统默认");}
QString selectTabletInputBackend(const QString &mode,const std::function<bool(bool)> &setWinTab,
                                const std::function<bool()> &winTabEnabled) {
    setWinTab(mode!=QStringLiteral("windows-ink"));
    return winTabEnabled()?QStringLiteral("wintab"):QStringLiteral("windows-ink");
}
void configureTabletInput(const QString &mode) {
#ifdef Q_OS_WIN
    if(auto *native=qGuiApp->nativeInterface<QNativeInterface::Private::QWindowsApplication>()) {
        activeBackend=selectTabletInputBackend(mode,[native](bool enabled){return native->setWinTabEnabled(enabled);},
                                               [native]{return native->isWinTabEnabled();});
        qInfo().noquote()<<"Tablet input backend:"<<activeBackend<<"requested:"<<mode;
    }
#else
    Q_UNUSED(mode);
#endif
}
QString activeTabletInputBackend(){return activeBackend;}
