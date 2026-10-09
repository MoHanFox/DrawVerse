#pragma once
#include <QObject>
#include <QPointer>
#include <memory>
class QWindow;

// A non-interactive native backdrop, independent of the owner's state changes.
class MenuBlurLayer final : public QObject {
public:
    explicit MenuBlurLayer(QObject *parent=nullptr);
    ~MenuBlurLayer() override;
    bool setEnabled(QWindow *window,bool enabled,int height);
    void setRadius(int radius);
    void sync();
    void hide();
    void clear();
    quintptr nativeHandle() const {return m_handle;}
private:
    struct CompositionState;
    std::unique_ptr<CompositionState> m_composition;
    QPointer<QWindow> m_window;
    QMetaObject::Connection m_destroyed;
    quintptr m_handle=0;
    [[maybe_unused]] int m_height=28;
    int m_radius=10;
    bool m_enabled=false;
    [[maybe_unused]] bool m_syncing=false;
};
