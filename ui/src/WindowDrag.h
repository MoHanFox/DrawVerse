#pragma once
#include <QObject>
#include <QPointer>
#include <QPoint>
class QWindow;

// Moves the real native window while retaining the pointer through reparenting.
class WindowDrag final : public QObject {
    Q_OBJECT
public:
    explicit WindowDrag(QObject *parent=nullptr):QObject(parent) {}
    ~WindowDrag() override;
    bool active() const {return !m_window.isNull();}
    void start(QWindow *window,QPoint global,QPoint offset);
    void finish(bool cancelled=false);
signals:
    void moved(QPoint global,bool suppressed);
    void finished(bool cancelled);
protected:
    bool eventFilter(QObject *,QEvent *) override;
private:
    QPointer<QWindow> m_window;
    QPoint m_offset,m_last;
    bool m_suppressed=false;
};
