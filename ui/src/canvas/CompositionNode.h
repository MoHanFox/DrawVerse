#pragma once
#include "engine/CompositionPacket.h"
#include <QSGSimpleTextureNode>
#include <functional>
class QQuickWindow;

// All methods run in the owning window's rendering thread. The fallback
// callback must queue work to the GUI; it must never mutate the document.
class CompositionNode : public QSGSimpleTextureNode {
public:
    static quint64 reservedPayloadBytes();
    static CompositionNode *create(QQuickWindow *window,CompositionPacketPtr packet,
                                  std::shared_ptr<CompositionCounters> counters,
                                  std::function<void(QString)> fallback,
                                  std::function<void(CompositionPacketPtr)> adopted);
    virtual bool sync(CompositionPacketPtr packet)=0;
    virtual bool ready()const=0;
};
