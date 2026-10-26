#include "platform/TabletInput.h"
#include <QGuiApplication>
#include <QPointingDevice>
#include <QTabletEvent>
#include <QPointer>
#include <QAbstractNativeEventFilter>
#include <QQuickWindow>
#ifdef Q_OS_WIN
#include <QtGui/private/qguiapplication_p.h>
#include <QtCore/qt_windows.h>
#endif

namespace {
QString activeBackend=QStringLiteral("系统默认");
#ifdef Q_OS_WIN
class TabletMouseFilter final:public QObject,public QAbstractNativeEventFilter {
public:
    explicit TabletMouseFilter(QObject *parent):QObject(parent){qApp->installNativeEventFilter(this);}
    ~TabletMouseFilter() override {if(qApp)qApp->removeNativeEventFilter(this);}
    bool nativeEventFilter(const QByteArray &type,void *message,qintptr *result) override {
        if(type!="windows_generic_MSG")return false;
        const auto *native=static_cast<const MSG*>(message);
        if(!isRedundantTabletMouseMessage(activeBackend,native->message,quintptr(GetMessageExtraInfo())))return false;
        // Native system dialogs do not use Quick's Tablet-to-mouse fallback.
        // Only the application's Quick windows can own the redundant stream.
        for(auto *window:QGuiApplication::allWindows()) {
            if(qobject_cast<QQuickWindow*>(window) && window->handle() && reinterpret_cast<HWND>(window->winId())==native->hwnd) {
                if(result)*result=0;
                return true;
            }
        }
        return false;
    }
};
QPointer<TabletMouseFilter> mouseFilter;
#endif
}
QString selectTabletInputBackend(const QString &mode,const std::function<bool(bool)> &setWinTab,
                                const std::function<bool()> &winTabEnabled) {
    setWinTab(mode!=QStringLiteral("windows-ink"));
    return winTabEnabled()?QStringLiteral("wintab"):QStringLiteral("windows-ink");
}
void configureTabletInput(const QString &mode) {
#ifdef Q_OS_WIN
    if(!mouseFilter)mouseFilter=new TabletMouseFilter(qApp);
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

bool isRedundantTabletMouseMessage(const QString &backend,quint32 message,quintptr extraInfo) {
#ifdef Q_OS_WIN
    // Windows promotes pen input independently of Qt's unhandled-Tablet fallback.
    // The low-byte touch flag must remain available to the touch mouse path.
    if(backend!=QStringLiteral("wintab") || (extraInfo & 0xffffff00u)!=0xff515700u || (extraInfo & 0x80u))return false;
    switch(message) {
    case WM_MOUSEMOVE:
    case WM_LBUTTONDOWN:case WM_LBUTTONUP:case WM_LBUTTONDBLCLK:
    case WM_RBUTTONDOWN:case WM_RBUTTONUP:case WM_RBUTTONDBLCLK:
    case WM_MBUTTONDOWN:case WM_MBUTTONUP:case WM_MBUTTONDBLCLK:
    case WM_XBUTTONDOWN:case WM_XBUTTONUP:case WM_XBUTTONDBLCLK:
        return true;
    default:return false;
    }
#else
    Q_UNUSED(backend);Q_UNUSED(message);Q_UNUSED(extraInfo);
    return false;
#endif
}
