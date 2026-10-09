#include "WindowDrag.h"
#include <QGuiApplication>
#include <QWindow>
#include <QMouseEvent>
#include <QKeyEvent>

WindowDrag::~WindowDrag(){if(m_window)m_window->setMouseGrabEnabled(false);}
void WindowDrag::start(QWindow *window,QPoint global,QPoint offset) {
    if(active() || !window)return;
    m_window=window;m_offset=offset;m_last=global;
    m_suppressed=false;qApp->installEventFilter(this);
    window->raise();window->requestActivate();window->setMouseGrabEnabled(true);
    window->setPosition(global-offset);emit moved(global,false);
    // A very fast release can precede the deferred window creation/grab.
    if(!QGuiApplication::mouseButtons().testFlag(Qt::LeftButton))finish();
}
void WindowDrag::finish(bool cancelled) {
    if(!active())return;
    qApp->removeEventFilter(this);m_window->setMouseGrabEnabled(false);m_window=nullptr;
    emit finished(cancelled);
}
bool WindowDrag::eventFilter(QObject *,QEvent *event) {
    if(!active())return false;
    if(event->type()==QEvent::MouseMove || event->type()==QEvent::MouseButtonRelease) {
        const auto *mouse=static_cast<QMouseEvent*>(event);m_last=mouse->globalPosition().toPoint();
#ifdef Q_OS_MACOS
        m_suppressed=mouse->modifiers().testFlag(Qt::MetaModifier);
#else
        m_suppressed=mouse->modifiers().testFlag(Qt::ControlModifier);
#endif
        m_window->setPosition(m_last-m_offset);emit moved(m_last,m_suppressed);
        if(event->type()==QEvent::MouseButtonRelease && mouse->button()==Qt::LeftButton)finish();
        return true;
    }
    if(event->type()==QEvent::KeyPress || event->type()==QEvent::KeyRelease) {
        const auto *key=static_cast<QKeyEvent*>(event);
        if(key->key()==Qt::Key_Escape && event->type()==QEvent::KeyPress){finish(true);return true;}
#ifdef Q_OS_MACOS
        const bool modifier=key->key()==Qt::Key_Meta;
#else
        const bool modifier=key->key()==Qt::Key_Control;
#endif
        if(modifier){m_suppressed=event->type()==QEvent::KeyPress;emit moved(m_last,m_suppressed);return true;}
    }
    return false;
}
