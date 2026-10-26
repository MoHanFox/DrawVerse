#include "platform/TabletInput.h"
#include <QGuiApplication>
#include <QPointingDevice>
#include <QTabletEvent>
#include <QPointer>
#include <QAbstractNativeEventFilter>
#include <QQuickWindow>
#include <QFile>
#include <QElapsedTimer>
#include <QMouseEvent>
#include <QTextStream>
#include <array>
#ifdef Q_OS_WIN
#include <QtGui/private/qguiapplication_p.h>
#include <QtCore/qt_windows.h>
#endif

namespace {
QString activeBackend=QStringLiteral("系统默认");
#ifdef Q_OS_WIN
struct TabletButtonSample {
    quint64 timestamp=0;
    QEvent::Type phase=QEvent::None;
    Qt::MouseButton button=Qt::NoButton;
    QPointF global;
};
std::array<TabletButtonSample,32> tabletButtons;
size_t nextTabletButton=0;
void rememberTabletButton(const QTabletEvent &event) {
    if(!event.timestamp())return;
    // Several application filters may observe the same event before Quick.
    for(const auto &sample:tabletButtons)
        if(sample.timestamp==event.timestamp() && sample.phase==event.type() && sample.button==event.button() && sample.global==event.globalPosition())return;
    tabletButtons[nextTabletButton]={event.timestamp(),event.type(),event.button(),event.globalPosition()};
    nextTabletButton=(nextTabletButton+1)%tabletButtons.size();
}
bool matchesTabletButton(const QMouseEvent &event) {
    if(!event.timestamp() || event.deviceType()!=QInputDevice::DeviceType::Mouse)return false;
    const auto phase=event.type()==QEvent::MouseButtonRelease?QEvent::TabletRelease:QEvent::TabletPress;
    for(const auto &sample:tabletButtons) {
        const auto delta=sample.global-event.globalPosition();
        // Gaomon's legacy mouse stream has no Windows pen signature, but keeps
        // the exact WinTab packet timestamp. Only coordinate rounding differs.
        if(sample.timestamp==event.timestamp() && sample.phase==phase && sample.button==event.button()
            && delta.x()*delta.x()+delta.y()*delta.y()<=4.)return true;
    }
    return false;
}
class TabletMouseFilter final:public QObject,public QAbstractNativeEventFilter {
public:
    explicit TabletMouseFilter(QObject *parent):QObject(parent){
        const auto path=qEnvironmentVariable("DRAWVERSE_TABLET_TRACE");
        if(!path.isEmpty()) {
            trace.setFileName(path);
            if(trace.open(QIODevice::WriteOnly|QIODevice::Truncate|QIODevice::Text)) {
                clock.start();
                record(QStringLiteral("start"));
            }
        }
        qApp->installEventFilter(this);
        qApp->installNativeEventFilter(this);
    }
    ~TabletMouseFilter() override {if(qApp)qApp->removeNativeEventFilter(this);}
    bool eventFilter(QObject *watched,QEvent *event) override {
        if(!qobject_cast<QWindow*>(watched))return false;
        if(trace.isOpen())switch(event->type()) {
        case QEvent::MouseButtonPress:case QEvent::MouseButtonRelease:case QEvent::MouseButtonDblClick:
        case QEvent::TabletPress:case QEvent::TabletRelease: {
            const auto *pointer=static_cast<QSinglePointEvent*>(event);
            const auto *device=pointer->pointingDevice();
            const auto source=dynamic_cast<QMouseEvent*>(event);
            record(QStringLiteral("qt type=%1 timestamp=%2 target=%3 name=%4 device=%5 id=%6 source=%7 buttons=%8 pos=%9,%10 spontaneous=%11")
                .arg(int(event->type())).arg(pointer->timestamp()).arg(QString::fromLatin1(watched->metaObject()->className()))
                .arg(watched->objectName()).arg(device?device->name():QStringLiteral("null"))
                .arg(device?device->systemId():-1).arg(source?int(source->source()):-1)
                .arg(int(pointer->buttons())).arg(pointer->globalPosition().x()).arg(pointer->globalPosition().y()).arg(event->spontaneous()));
            break;
        }
        default:break;
        }
        return rejectTabletInputEvent(event);
    }
    bool nativeEventFilter(const QByteArray &type,void *message,qintptr *result) override {
        if(type!="windows_generic_MSG")return false;
        const auto *native=static_cast<const MSG*>(message);
        if(trace.isOpen() && ((native->message>=WM_LBUTTONDOWN && native->message<=WM_XBUTTONDBLCLK)
            || native->message==WM_POINTERDOWN || native->message==WM_POINTERUP)) {
            record(QStringLiteral("native msg=0x%1 time=%2 hwnd=0x%3 extra=0x%4 wparam=0x%5 lparam=0x%6 suppressible=%7")
                .arg(native->message,0,16).arg(native->time).arg(quintptr(native->hwnd),0,16)
                .arg(quintptr(GetMessageExtraInfo()),0,16).arg(quintptr(native->wParam),0,16).arg(quintptr(native->lParam),0,16)
                .arg(isRedundantTabletMouseMessage(activeBackend,native->message,quintptr(GetMessageExtraInfo()))));
        }
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
private:
    void record(const QString &line) {
        if(!trace.isOpen() || records>=10000)return;
        QTextStream out(&trace);
        out<<clock.elapsed()<<" backend="<<activeBackend<<" "<<line<<Qt::endl;
        ++records;
    }
    QFile trace;
    QElapsedTimer clock;
    int records=0;
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
    tabletButtons={};nextTabletButton=0;
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
#ifdef Q_OS_WIN
        if(activeBackend==QStringLiteral("wintab") && (event->type()==QEvent::TabletPress || event->type()==QEvent::TabletRelease)) {
            const auto *tablet=static_cast<QTabletEvent*>(event);
            if(tablet->pointingDevice() && tablet->pointingDevice()->name()==QStringLiteral("wintab"))rememberTabletButton(*tablet);
        }
#endif
        break;
    case QEvent::MouseButtonPress:case QEvent::MouseButtonRelease:case QEvent::MouseButtonDblClick:
#ifdef Q_OS_WIN
        if(activeBackend==QStringLiteral("wintab") && matchesTabletButton(*static_cast<QMouseEvent*>(event))) {
            event->accept();return true;
        }
#endif
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
