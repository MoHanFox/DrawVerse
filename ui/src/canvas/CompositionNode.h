#pragma once
#include "engine/CompositionPacket.h"
#include <QSGSimpleTextureNode>
#include <QObject>
#include <functional>
class QQuickWindow;

class CompositionBudgetNotifier final : public QObject {
    Q_OBJECT
public:
    using QObject::QObject;
signals:
    void resourcesReleased();
};

// Creation/sync run in the owning window's rendering thread. The fallback
// callback must queue work to the GUI; it must never mutate the document.
class CompositionNode : public QSGSimpleTextureNode {
public:
    static quint64 reservedPayloadBytes();
    static int textureLimit(QQuickWindow *window);
    static quint64 releasedPayloadSerial();
    static bool supportsWindow(QQuickWindow *window);
    // GUI thread, app-owned; release notifications are coalesced from rendering.
    static CompositionBudgetNotifier *budgetNotifier();
    static CompositionNode *create(QQuickWindow *window,CompositionPacketPtr packet,
                                  std::shared_ptr<CompositionCounters> counters,
                                  std::function<void(QString,CompositionPacketPtr)> fallback,
                                  std::function<void(CompositionPacketPtr)> adopted);
    virtual bool sync(CompositionPacketPtr packet)=0;
    virtual bool ready()const=0;
};
