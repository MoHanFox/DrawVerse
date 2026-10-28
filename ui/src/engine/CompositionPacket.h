#pragma once
#include "paint_api.h"
#include <QByteArray>
#include <QHash>
#include <QRectF>
#include <QSize>
#include <QString>
#include <QVector>
#include <atomic>
#include <array>
#include <memory>
#include <mutex>
#include <optional>

// Supported window backends decide capability separately. An explicit override
// remains available for CPU reference/recovery; tests may change it per case.
inline bool compositionDisplayRequested(){
    return !qEnvironmentVariableIsSet("DRAWVERSE_GPU_COMPOSITION") ||
        qEnvironmentVariableIntValue("DRAWVERSE_GPU_COMPOSITION")==1;
}

// Immutable CPU transport only. No QObject, document mutation, or GPU handle.
struct CompositionTileData {
    QByteArray bytes;
    ~CompositionTileData();
    static std::shared_ptr<CompositionTileData> allocate();
};
struct CompositionTransform {
    ~CompositionTransform();
    static std::shared_ptr<CompositionTransform> allocate();
    PaintTransform geometry{};
    PaintTransformDisplayGeometry display{};
    QByteArray vertices; // maximum 145*145*16 bytes, produced by the pure Rust helper
    QRectF sourceBounds;
    int node=-1;
    double scale=1;
};
struct CompositionAdjustment {
    int node=-1,kind=0;
    bool neutral=false;
    std::array<float,4> parameters{};
    QByteArray lut; // 4097 RGBA32F entries, immutable CPU transport
};
struct CompositionPacket {
    PaintCompositionInfo info{};
    QVector<PaintCompositionNode> nodes;
    QVector<PaintCompositionOperation> operations;
    int surfaceCount=2,outputSurface=0;
    QVector<PaintCompositionTile> tiles;
    QVector<std::shared_ptr<const CompositionTileData>> data;
    QRectF region;
    QSize pixels;
    int lod=0;
    // Local display geometry is independent of the authoritative source key.
    quint64 geometrySequence=0,editSequence=0;
    QPoint presentationDelta;
    bool preview=false;
    std::shared_ptr<const CompositionTransform> transform;
    std::shared_ptr<const CompositionAdjustment> adjustment;
};
using CompositionPacketPtr=std::shared_ptr<const CompositionPacket>;
struct CompositionViewRequest {
    quint64 serial=0,epoch=0,generation=0;
    QRectF region;
    QSize pixels;
    bool enabled=false;
};
// Correctness-only observer. A result belongs to one explicit immutable
// packet and request version; normal presentation never creates this observer.
struct CompositionReadback {
    struct Identity {
        quint64 epoch,generation,revision,request,lease,publication,geometry;
        quint32 view;
        QRectF region;
        QSize pixels;
        bool operator==(const Identity &)const=default;
    };
    static Identity identityOf(const CompositionPacket &p){
        const auto &k=p.info.key;
        return {k.device_epoch,k.document_generation,k.revision,k.request_id,k.lease_id,k.publication,p.geometrySequence,k.view_id,p.region,p.pixels};
    }
    void request(const CompositionPacket &packet){
        std::lock_guard lock(mutex);target=identityOf(packet);received=0;linear.clear();display.clear();completed=0;
        geometry=packet.geometrySequence;++version;
    }
    std::optional<quint64> pending(const CompositionPacket &packet,quint64 last){
        std::lock_guard lock(mutex);const auto v=version.load();
        if(!v || v==last || completed.load()==2 || !(target==identityOf(packet)))return {};
        return v;
    }
    bool accept(quint64 v,const Identity &identity,int output,QByteArray data){
        std::lock_guard lock(mutex);
        if(v!=version.load() || !(target==identity) || output<0 || output>1 || (received&(1<<output)))return false;
        (output?display:linear)=std::move(data);received|=1<<output;++completed;return true;
    }
    std::mutex mutex;
    QByteArray linear,display;
    std::atomic<int> completed{0};
    std::atomic<quint64> version{0},geometry{0};
private:
    Identity target{};
    int received=0;
};
struct CompositionCounters {
    std::atomic<quint64> uploads{0},uploadBytes{0},passes{0},adoptions{0},revision{0},epoch{0},request{0},textureBytes{0};
    std::atomic<int> failures{0};
    std::atomic<int> testReadbacks{0};
    std::atomic<quint64> composedPixels{0},lastDirtyArea{0};
    std::atomic<quint64> nodeRevision{0},nodeRequest{0},nodeGeometry{0},nodeEpoch{0};
    std::atomic<quint64> handoffRevision{0};
    std::atomic<int> preciseTransform{0};
    std::atomic<qint64> lastUploadNs{0},lastSubmitNs{0};
    std::atomic<quint64> lastSubmitGeometry{0};
    std::atomic<quint64> pipelineCreates{0},adjustmentUpdates{0};
    std::atomic<quint64> geometryReuses{0},pageTableCreates{0},axisCreates{0},parameterBufferCreates{0},bindingCreates{0};
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
