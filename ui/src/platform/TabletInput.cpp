#include "platform/TabletInput.h"
#include <QGuiApplication>
#include <QPointingDevice>
#include <QTabletEvent>
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
bool acceptTabletInputDevice(const QString &backend,const QPointingDevice *device) {
    // Qt's Windows pointer handler keeps producing "wmpointer" tablet events
    // after WinTab is enabled. Mixing both device streams duplicates hover and
    // contact packets, so only the selected backend may drive canvas gestures.
    return backend!=QStringLiteral("wintab") || !device || device->name()!=QStringLiteral("wmpointer");
}
bool rejectTabletInputEvent(QEvent *event) {
    switch(event->type()) {
    case QEvent::TabletPress:case QEvent::TabletMove:case QEvent::TabletRelease:
    case QEvent::TabletEnterProximity:case QEvent::TabletLeaveProximity:
        if(!acceptTabletInputDevice(activeTabletInputBackend(),static_cast<QTabletEvent*>(event)->pointingDevice())) {
            event->accept();return true;
        }
        break;
    default:break;
    }
    return false;
}
