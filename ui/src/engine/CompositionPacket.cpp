#include "CompositionPacket.h"
#include <QElapsedTimer>
#include <QScopeGuard>
#include <algorithm>
#include <cmath>

namespace {
std::atomic<quint64> sourceBytes{0},geometryBytes{0};
constexpr quint64 geometryCharge=512*1024,geometryLimit=32*1024*1024;
constexpr quint64 sourceLimit=512ull*1024*1024;
template<class T> T dto(){T value{};value.struct_size=sizeof(T);return value;}
}
CompositionTransform::~CompositionTransform(){geometryBytes.fetch_sub(geometryCharge);}
std::shared_ptr<CompositionTransform> CompositionTransform::allocate(){
    auto bytes=geometryBytes.load();
    do {if(bytes>geometryLimit-geometryCharge)return {};}
    while(!geometryBytes.compare_exchange_weak(bytes,bytes+geometryCharge));
    auto undo=qScopeGuard([]{geometryBytes.fetch_sub(geometryCharge);});
    try {auto value=std::make_shared<CompositionTransform>();undo.dismiss();return value;}
    catch(const std::bad_alloc &){return {};}
}
CompositionTileData::~CompositionTileData(){sourceBytes.fetch_sub(65536);}
std::shared_ptr<CompositionTileData> CompositionTileData::allocate(){
    auto bytes=sourceBytes.load();
    do {if(bytes>sourceLimit-65536)return {};}
    while(!sourceBytes.compare_exchange_weak(bytes,bytes+65536));
    auto undo=qScopeGuard([]{sourceBytes.fetch_sub(65536);});
    try {
        auto result=std::make_shared<CompositionTileData>();undo.dismiss();
        result->bytes=QByteArray(65536,Qt::Uninitialized);return result;
    }catch(const std::bad_alloc &){return {};}
}
CompositionReader::~CompositionReader(){reset(m_core,m_session);}
void CompositionReader::reset(PaintCore *core,PaintSession *session){
    if(m_lease && core && session)paint_session_release_composition(core,session,m_lease);
    m_lease=0;m_packet.reset();m_cache.clear();m_core=nullptr;m_session=nullptr;
    m_attemptRevision=~quint64(0);m_serial=m_epoch=m_generation=0;
}
CompositionPacketPtr CompositionReader::poll(PaintCore *core,PaintSession *session,const PaintSessionInfo &state,
                                            const CompositionViewRequest &view,QString &fallback){
    if(!view.enabled || view.generation!=state.document_generation){
        reset(core,session);fallback=QStringLiteral("CPU: disabled or active editing session");return {};
    }
    if(m_serial!=view.serial || m_epoch!=view.epoch || m_generation!=view.generation){
        reset(core,session);m_serial=view.serial;m_epoch=view.epoch;m_generation=view.generation;
    }
    m_core=core;m_session=session;
    if(!m_lease){
        if(qEnvironmentVariable("DRAWVERSE_COMPOSITION_FAIL")=="transport"){fallback=QStringLiteral("CPU: injected composition transport budget");return {};}
        if(m_attemptRevision==state.revision)return {};
        m_attemptRevision=state.revision;
        if(view.pixels.isEmpty() || view.region.isEmpty()){
            fallback=QStringLiteral("CPU: composition view unavailable");return {};
        }
        const auto area=quint64(state.width)*state.height;
        const auto scale=std::min(view.region.width()/view.pixels.width(),view.region.height()/view.pixels.height());
        const int lod=scale>=1?std::clamp(int(std::ceil(std::log2(scale))),0,20):0;
        if(!area || area>4194304 || state.width>4096 || state.height>4096 || lod>6 ||
           view.pixels.isEmpty() || view.pixels.width()>4096 || view.pixels.height()>4096 ||
           quint64(view.pixels.width())*view.pixels.height()>4194304){
            fallback=QStringLiteral("CPU: composition extent/LOD budget");return {};
        }
        auto request=dto<PaintCompositionRequest>();request.version=1;request.features=PAINT_COMPOSITION_NORMAL|PAINT_COMPOSITION_SELF_MASK|PAINT_COMPOSITION_LINEAR_RGBA32F|PAINT_COMPOSITION_FIXED_MASK|PAINT_COMPOSITION_BLEND_MODES|PAINT_COMPOSITION_GROUPS|PAINT_COMPOSITION_DISSOLVE|PAINT_COMPOSITION_CLIPPING;
        request.viewport=dto<PaintViewport>();request.viewport.enabled=1;request.viewport.document_generation=view.generation;
        request.viewport.x=view.region.x();request.viewport.y=view.region.y();request.viewport.width=view.region.width();request.viewport.height=view.region.height();
        request.viewport.pixel_width=quint32(view.pixels.width());request.viewport.pixel_height=quint32(view.pixels.height());
        request.revision=state.revision;request.request_id=++m_next;request.device_epoch=view.epoch;
        request.max_nodes=64;request.max_tiles=1024;request.max_pinned_bytes=64ull*1024*1024;request.max_metadata_bytes=2ull*1024*1024;
        const auto status=paint_session_request_composition(core,session,&request,&m_lease);
        if(status!=PAINT_OK){fallback=QStringLiteral("CPU: composition request %1").arg(status);return {};}
        m_packet=std::make_shared<CompositionPacket>();m_packet->region=view.region;m_packet->pixels=view.pixels;m_packet->lod=lod;
    }
    auto abandon=[&](PaintStatus status){
        paint_session_release_composition(core,session,m_lease);m_lease=0;m_packet.reset();
        if(status==PAINT_BUSY || status==PAINT_CANCELLED)m_attemptRevision=~quint64(0);
        fallback=QStringLiteral("CPU: composition transport %1").arg(status);
        return CompositionPacketPtr{};
    };
    if(m_packet->nodes.isEmpty()){
        auto info=dto<PaintCompositionInfo>();const auto status=paint_session_composition_info(core,session,m_lease,&info);
        if(status!=PAINT_OK)return abandon(status);
        if(info.state==PAINT_COMPOSITION_PENDING)return {};
        if(info.state!=PAINT_COMPOSITION_READY)return abandon(info.status);
        if(info.node_count>64 || info.tile_count>1024)return abandon(PAINT_LIMIT_EXCEEDED);
        m_packet->info=info;
        for(quint32 i=0;i<info.node_count;++i){
            auto node=dto<PaintCompositionNode>();const auto result=paint_session_composition_node(core,session,&info.key,i,&node);
            if(result!=PAINT_OK)return abandon(result);m_packet->nodes.append(node);
        }
        if(info.required_features & (PAINT_COMPOSITION_GROUPS|PAINT_COMPOSITION_DISSOLVE|PAINT_COMPOSITION_CLIPPING)){
            const auto count=info.reserved[0],surfaces=info.reserved[1]>>32;const auto output=quint32(info.reserved[1]);
            if(!count || count>quint64(info.node_count)*(info.required_features & PAINT_COMPOSITION_CLIPPING?16:4)+1 || surfaces<2 || surfaces>quint64(info.required_features & PAINT_COMPOSITION_CLIPPING?85:34) || output>1)return abandon(PAINT_LIMIT_EXCEEDED);
            m_packet->surfaceCount=int(surfaces);m_packet->outputSurface=int(output);
            QVector<bool> initialized(int(surfaces),false);
            for(quint32 i=0;i<quint32(count);++i){
                auto op=dto<PaintCompositionOperation>();const auto result=paint_session_composition_operation(core,session,&info.key,i,&op);
                if(result!=PAINT_OK)return abandon(result);
                if(op.kind>PAINT_COMPOSITION_APPLY_PREPARED || op.node_index>=info.node_count || op.output>=surfaces ||
                   op.source>=surfaces || op.backdrop>=surfaces ||
                   (op.reserved[0] && !(info.required_features & PAINT_COMPOSITION_DISSOLVE)) ||
                   (op.kind>=PAINT_COMPOSITION_MASKED_RASTER && !(info.required_features & PAINT_COMPOSITION_CLIPPING)))return abandon(PAINT_UNSUPPORTED);
                const bool readsBack=op.kind!=PAINT_COMPOSITION_CLEAR && op.kind!=PAINT_COMPOSITION_MASKED_RASTER && op.kind!=PAINT_COMPOSITION_MASK_SURFACE;
                const bool readsSource=op.kind==PAINT_COMPOSITION_BLEND_SURFACE || op.kind==PAINT_COMPOSITION_INFLUENCE ||
                    op.kind==PAINT_COMPOSITION_MASK_SURFACE || op.kind==PAINT_COMPOSITION_CLIP_SURFACE ||
                    op.kind==PAINT_COMPOSITION_CLIP_INFLUENCE || op.kind==PAINT_COMPOSITION_APPLY_PREPARED;
                if(readsBack && (!initialized[int(op.backdrop)] || op.output==op.backdrop))return abandon(PAINT_UNSUPPORTED);
                if(readsSource && (!initialized[int(op.source)] || op.output==op.source))return abandon(PAINT_UNSUPPORTED);
                if(op.reserved[1]){
                    const auto coverage=op.reserved[1]-1;
                    if(!(info.required_features & PAINT_COMPOSITION_CLIPPING) || coverage>=surfaces || !initialized[int(coverage)] || coverage==op.output ||
                       (op.kind!=PAINT_COMPOSITION_CLIP_RASTER && op.kind!=PAINT_COMPOSITION_CLIP_SURFACE && op.kind!=PAINT_COMPOSITION_CLIP_INFLUENCE))return abandon(PAINT_UNSUPPORTED);
                }
                const auto kind=m_packet->nodes[int(op.node_index)].kind;
                if(((op.kind==PAINT_COMPOSITION_DRAW_RASTER || op.kind==PAINT_COMPOSITION_MASKED_RASTER || op.kind==PAINT_COMPOSITION_CLIP_RASTER) && kind!=PAINT_COMPOSITION_RASTER) ||
                   ((op.kind==PAINT_COMPOSITION_BLEND_SURFACE || op.kind==PAINT_COMPOSITION_INFLUENCE || op.kind==PAINT_COMPOSITION_MASK_SURFACE || op.kind==PAINT_COMPOSITION_CLIP_SURFACE || op.kind==PAINT_COMPOSITION_CLIP_INFLUENCE) && kind!=PAINT_COMPOSITION_GROUP) ||
                   (op.kind==PAINT_COMPOSITION_APPLY_PREPARED && kind==PAINT_COMPOSITION_MASK))return abandon(PAINT_UNSUPPORTED);
                initialized[int(op.output)]=true;m_packet->operations.append(op);
            }
            if(!initialized[int(output)])return abandon(PAINT_UNSUPPORTED);
        }else if(info.reserved[0] || info.reserved[1])return abandon(PAINT_UNSUPPORTED);
        for(quint32 i=0;i<info.tile_count;++i){
            auto tile=dto<PaintCompositionTile>();const auto result=paint_session_composition_tile(core,session,&info.key,i,&tile);
            if(result!=PAINT_OK)return abandon(result);
            if(tile.required_bytes!=65536 || tile.format!=4)return abandon(PAINT_UNSUPPORTED);
            m_packet->tiles.append(tile);
        }
        m_packet->data.reserve(info.tile_count);
    }
    QElapsedTimer elapsed;elapsed.start();int reads=0;
    while(m_packet->data.size()<m_packet->tiles.size()){
        const auto index=m_packet->data.size();const auto identity=m_packet->tiles[index].content_id;
        auto data=m_cache.value(identity).lock();
        if(!data){
            auto mutableData=CompositionTileData::allocate();if(!mutableData)return abandon(PAINT_LIMIT_EXCEEDED);
            uint64_t required=0;
            const auto status=paint_session_read_composition_tile(core,session,&m_packet->info.key,quint32(index),
                reinterpret_cast<uint8_t*>(mutableData->bytes.data()),65536,&required);
            if(status!=PAINT_OK)return abandon(status);
            if(required!=65536)return abandon(PAINT_INTERNAL_ERROR);
            data=std::move(mutableData);if(m_cache.size()>=2048)m_cache.clear();m_cache.insert(identity,data);++reads;
        }
        m_packet->data.append(std::move(data));
        if(reads>=128 || elapsed.elapsed()>=4)return {};
    }
    auto result=CompositionPacketPtr(std::move(m_packet));paint_session_release_composition(core,session,m_lease);m_lease=0;
    fallback.clear();return result;
}
