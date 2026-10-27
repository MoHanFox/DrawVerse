#pragma once
#include "paint_api.h"
#include <QByteArray>
#include <QHash>
#include <QRectF>
#include <QSize>
#include <QString>
#include <QVector>
#include <atomic>
#include <memory>
#include <mutex>

// Immutable CPU transport only. No QObject, document mutation, or GPU handle.
struct CompositionTileData {
    QByteArray bytes;
    ~CompositionTileData();
    static std::shared_ptr<CompositionTileData> allocate();
};
struct CompositionPacket {
    PaintCompositionInfo info{};
    QVector<PaintCompositionNode> nodes;
    QVector<PaintCompositionTile> tiles;
    QVector<std::shared_ptr<const CompositionTileData>> data;
    QRectF region;
    QSize pixels;
    int lod=0;
};
using CompositionPacketPtr=std::shared_ptr<const CompositionPacket>;
struct CompositionViewRequest {
    quint64 serial=0,epoch=0,generation=0;
    QRectF region;
    QSize pixels;
    bool enabled=false;
};
struct CompositionReadback {
    std::mutex mutex;
    QByteArray linear,display;
    std::atomic<int> completed{0};
};
struct CompositionCounters {
    std::atomic<quint64> uploads{0},uploadBytes{0},passes{0},adoptions{0},revision{0},epoch{0},request{0},textureBytes{0};
    std::atomic<int> failures{0};
    std::atomic<int> testReadbacks{0};
    std::atomic<quint64> composedPixels{0},lastDirtyArea{0};
    std::atomic<quint64> nodeRevision{0},nodeRequest{0};
    // Set before attaching the Canvas in correctness tests only. Normal null.
    std::shared_ptr<CompositionReadback> readback;
};

// BackendWorker owns this reader. Each poll has bounded work; cold IO never
// runs in the GUI/render thread. A held lease keeps its source immutable.
class CompositionReader {
public:
    ~CompositionReader();
    void reset(PaintCore *core,PaintSession *session);
    CompositionPacketPtr poll(PaintCore *core,PaintSession *session,const PaintSessionInfo &state,
                              const CompositionViewRequest &view,QString &fallback);
private:
    PaintCore *m_core=nullptr;
    PaintSession *m_session=nullptr;
    quint64 m_lease=0,m_next=0,m_attemptRevision=~quint64(0),m_serial=0,m_epoch=0,m_generation=0;
    std::shared_ptr<CompositionPacket> m_packet;
    QHash<quint64,std::weak_ptr<const CompositionTileData>> m_cache;
};
