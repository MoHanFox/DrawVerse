#include "CompositionNode.h"
#include <QQuickWindow>
#include <QFile>
#include <QSGRendererInterface>
#include <QtEndian>
#include <QSet>
#include <array>
#include <algorithm>
#include <cmath>
#include <cstring>
#include <map>
#include <tuple>
#ifdef DRAWVERSE_HAS_COMPOSITION_RHI
#include <rhi/qrhi.h>
static void initializeCompositionResources(){Q_INIT_RESOURCE(composition_shaders);}

namespace {
constexpr quint64 textureLimit=256ull*1024*1024;
constexpr quint64 processTextureLimit=2ull*1024*1024*1024;
std::atomic<quint64> reservedTextureBytes{0};
// Reserve payload for the current allocation plus every possible deferred frame.
// This is deliberately conservative; driver overhead is not a payload byte count.
class TextureReservation final {
public:
    ~TextureReservation(){release();}
    bool grow(quint64 value){
        if(value<=bytes)return true;
        const auto extra=value-bytes;auto current=reservedTextureBytes.load();
        do{if(extra>processTextureLimit-current)return false;}
        while(!reservedTextureBytes.compare_exchange_weak(current,current+extra));
        bytes=value;return true;
    }
    void release(){if(!released.exchange(true))reservedTextureBytes.fetch_sub(bytes);}
    void retire(QQuickWindow *window,int frames,const std::shared_ptr<TextureReservation> &self){
        const auto done=[self]{self->release();for(const auto &c:self->connections)QObject::disconnect(c);};
        connections[0]=QObject::connect(window,&QQuickWindow::beforeRendering,window,[self,frames,done]()mutable{if(--frames<=0)done();},Qt::DirectConnection);
        connections[1]=QObject::connect(window,&QQuickWindow::sceneGraphInvalidated,window,done,Qt::DirectConnection);
        connections[2]=QObject::connect(window,&QObject::destroyed,done);
    }
private:
    quint64 bytes=0;
    std::atomic<bool> released{false};
    std::array<QMetaObject::Connection,3> connections;
};
struct Parameters {
    std::array<qint32,4> dimensions{},source{},mask{},sourceOrigin{},maskOrigin{};
    std::array<float,4> factors{};
};
static_assert(sizeof(Parameters)==96);
class DirectTexture final : public QSGTexture {
public:
    QRhiTexture *value=nullptr;
    qint64 comparisonKey()const override{return qint64(quintptr(value));}
    QRhiTexture *rhiTexture()const override{return value;}
    QSize textureSize()const override{return value?value->pixelSize():QSize();}
    bool hasAlphaChannel()const override{return true;}
    bool hasMipmaps()const override{return false;}
};
struct Target {
    std::unique_ptr<QRhiTexture> texture;
    std::unique_ptr<QRhiRenderPassDescriptor> pass;
    std::unique_ptr<QRhiTextureRenderTarget> target;
};
struct Pass {
    std::unique_ptr<QRhiBuffer> uniform;
    std::unique_ptr<QRhiShaderResourceBindings> bindings;
    std::unique_ptr<QRhiGraphicsPipeline> pipeline;
    Parameters parameters;
    int output=0;
};
struct PageMap {
    std::unique_ptr<QRhiTexture> texture;
    QByteArray bytes;
    QSize size;
    QPoint origin;
};
class RhiCompositionNode final : public CompositionNode {
public:
    RhiCompositionNode(QQuickWindow *w,CompositionPacketPtr p,std::shared_ptr<CompositionCounters> c,std::function<void(QString)> f)
        :window(w),packet(std::move(p)),counters(std::move(c)),fallback(std::move(f)){}
    ~RhiCompositionNode()override{
        QObject::disconnect(connection);
        if(reservation)reservation->retire(window,inFlightFrames+2,reservation);
    }
    bool initialize(){
        if(qEnvironmentVariable("DRAWVERSE_COMPOSITION_FAIL")=="init")return fail("CPU: injected composition initialization failure");
        auto *ri=window->rendererInterface();
        if(ri->graphicsApi()!=QSGRendererInterface::Direct3D11)return fail("CPU: composition requires D3D11");
        rhi=static_cast<QRhi*>(ri->getResource(window,QSGRendererInterface::RhiResource));
        if(!rhi || !rhi->isTextureFormatSupported(QRhiTexture::RGBA32F,QRhiTexture::RenderTarget) ||
           rhi->resourceLimit(QRhi::TextureSizeMax)<4096)return fail("CPU: composition format/extent capability");
        const auto source=[](const char *name){QFile file(name);if(!file.open(QIODevice::ReadOnly))return QShader{};return QShader::fromSerialized(file.readAll());};
        vertex=source(":/composition/composition.vert.qsb");fragment=source(":/composition/composition.frag.qsb");
        if(!vertex.isValid() || !fragment.isValid())return fail("CPU: composition shaders unavailable");
        sampler.reset(rhi->newSampler(QRhiSampler::Nearest,QRhiSampler::Nearest,QRhiSampler::None,QRhiSampler::ClampToEdge,QRhiSampler::ClampToEdge));
        if(!sampler->create())return fail("CPU: composition sampler allocation");
        const int frameSlots=rhi->resourceLimit(QRhi::FramesInFlight);
        if(frameSlots<1 || frameSlots>8)return fail("CPU: composition in-flight budget");
        inFlightFrames=frameSlots;reservation=std::make_shared<TextureReservation>();
        if(!reserveTargets(packet))return false;
        atlas.reset(rhi->newTexture(QRhiTexture::RGBA32F,{2048,2048}));
        if(!atlas->create())return fail("CPU: composition atlas allocation");
        staging.resize(frameSlots);
        connection=QObject::connect(window,&QQuickWindow::beforeRendering,window,[this]{record();},Qt::DirectConnection);
        auto p=std::move(packet);return sync(std::move(p));
    }
    bool sync(CompositionPacketPtr p)override{
        if(failed || !p || p->info.key.device_epoch!=(packet?packet->info.key.device_epoch:p->info.key.device_epoch))return false;
        if(packet && p->info.key.document_generation==packet->info.key.document_generation &&
           p->info.key.request_id<packet->info.key.request_id)return true;
        if(packet==p)return true;
        updateDirty(p);packet=std::move(p);dirty=true;
        counters->epoch=packet->info.key.device_epoch;counters->request=packet->info.key.request_id;
        return prepareTargets();
    }
    bool ready()const override{return rendered && !failed;}
private:
    bool fail(const char *message){if(!failed){failed=true;++counters->failures;fallback(QString::fromUtf8(message));}return false;}
    bool createTarget(Target &t,QRhiTexture::Format format,QSize size){
        t.texture.reset(rhi->newTexture(format,size,1,QRhiTexture::RenderTarget|QRhiTexture::UsedAsTransferSource));
        if(!t.texture->create())return false;
        t.target.reset(rhi->newTextureRenderTarget(QRhiTextureRenderTargetDescription(QRhiColorAttachment(t.texture.get()))));
        t.target->setFlags(QRhiTextureRenderTarget::PreserveColorContents);
        t.pass.reset(t.target->newCompatibleRenderPassDescriptor());t.target->setRenderPassDescriptor(t.pass.get());
        return t.target->create();
    }
    bool reserveTargets(const CompositionPacketPtr &p){
        const quint64 charge=64ull*1024*1024+p->info.document_width*p->info.document_height*32+
            quint64(p->pixels.width())*p->pixels.height()*4+2ull*1024*1024;
        if(charge>textureLimit)return fail("CPU: composition GPU payload budget");
        if(!reservation->grow(charge*quint64(inFlightFrames+2)))return fail("CPU: composition process/in-flight GPU payload budget");
        counters->textureBytes=charge;return true;
    }
    bool prepareTargets(){
        const QSize doc(int(packet->info.document_width),int(packet->info.document_height));
        const auto pixels=packet->pixels;
        if(doc==documentSize && pixels==outputSize)return true;
        // Resources are owned by this render node. QRhi defers native destruction
        // as required; no cross-window texture registry participates.
        passes.clear();maps.clear();axisX.reset();axisY.reset();
        if(!reserveTargets(packet))return false;
        if(doc!=documentSize){
            for(auto &t:linear)t={};
            if(!createTarget(linear[0],QRhiTexture::RGBA32F,doc) || !createTarget(linear[1],QRhiTexture::RGBA32F,doc))return fail("CPU: composition render-target allocation");
            changed=QRect(QPoint(),doc);
        }
        display={};if(!createTarget(display,QRhiTexture::RGBA8,pixels))return fail("CPU: composition display-target allocation");
        documentSize=doc;outputSize=pixels;texture.value=display.texture.get();setTexture(&texture);setFiltering(QSGTexture::Linear);
        markDirty(QSGNode::DirtyMaterial);rendered=false;
        return true;
    }
    void updateDirty(const CompositionPacketPtr &next){
        const QRect full(0,0,int(next->info.document_width),int(next->info.document_height));
        if(!packet || next->info.key.document_generation!=packet->info.key.document_generation ||
           next->info.document_width!=packet->info.document_width || next->info.document_height!=packet->info.document_height ||
           next->nodes.size()!=packet->nodes.size()){changed=full;return;}
        for(int i=0;i<next->nodes.size();++i){
            auto a=next->nodes[i],b=packet->nodes[i];a.first_tile=a.tile_count=b.first_tile=b.tile_count=0;
            if(std::memcmp(&a,&b,sizeof(a))){changed=full;return;}
        }
        using Key=std::tuple<int,qint32,qint32>;
        using Entry=std::pair<quint64,QRect>;
        const auto entries=[&](const CompositionPacket &p){
            std::map<Key,Entry> result;
            for(int n=0;n<p.nodes.size();++n){
                const auto &node=p.nodes[n];
                for(quint32 i=node.first_tile;i<node.first_tile+node.tile_count;++i){
                    const auto &tile=p.tiles[int(i)];
                    const qint64 left=qint64(tile.tile_x)*64+node.offset_x,top=qint64(tile.tile_y)*64+node.offset_y;
                    const qint64 x0=std::max<qint64>(0,left),y0=std::max<qint64>(0,top);
                    const qint64 x1=std::min<qint64>(full.width(),left+64),y1=std::min<qint64>(full.height(),top+64);
                    result.emplace(Key{n,tile.tile_x,tile.tile_y},Entry{tile.content_id,x0<x1 && y0<y1?QRect(int(x0),int(y0),int(x1-x0),int(y1-y0)):QRect()});
                }
            }
            return result;
        };
        const auto before=entries(*packet),after=entries(*next);changed={};
        for(const auto &[key,value]:before){const auto at=after.find(key);if(at==after.end() || at->second.first!=value.first)changed=changed.united(value.second);}
        for(const auto &[key,value]:after){const auto at=before.find(key);if(at==before.end() || at->second.first!=value.first)changed=changed.united(value.second);}
    }
    bool map(PageMap &out,const PaintCompositionNode &node,const QHash<quint64,int> &locations){
        const int ox=node.offset_x,oy=node.offset_y;
        if(std::abs(qint64(ox))>1048576 || std::abs(qint64(oy))>1048576 || node.white_width>INT_MAX || node.white_height>INT_MAX)
            return fail("CPU: composition coordinate range");
        const auto floor64=[](int x){return x>=0?x/64:(x-63)/64;};
        out.origin={floor64(-ox),floor64(-oy)};
        const int w=floor64(documentSize.width()-1-ox)-out.origin.x()+1;
        const int h=floor64(documentSize.height()-1-oy)-out.origin.y()+1;
        out.size={w,h};out.bytes=QByteArray(w*h*16,'\0');
        for(quint32 i=node.first_tile;i<node.first_tile+node.tile_count;++i){
            if(i>=quint32(packet->tiles.size()))return fail("CPU: composition tile range");
            const auto &tile=packet->tiles[int(i)];const int x=tile.tile_x-out.origin.x(),y=tile.tile_y-out.origin.y();
            if(x<0 || y<0 || x>=w || y>=h)continue;
            const float slot=float(locations.value(tile.content_id,-1)+1);
            std::memcpy(out.bytes.data()+(y*w+x)*16,&slot,4);
        }
        out.texture.reset(rhi->newTexture(QRhiTexture::RGBA32F,out.size));return out.texture->create();
    }
    bool makePass(Pass &pass,Target &target,QRhiTexture *back,QRhiTexture *a,QRhiTexture *b,QRhiTexture *c){
        pass.uniform.reset(rhi->newBuffer(QRhiBuffer::Dynamic,QRhiBuffer::UniformBuffer,sizeof(Parameters)));
        if(!pass.uniform->create())return false;
        pass.bindings.reset(rhi->newShaderResourceBindings());pass.bindings->setBindings({
            QRhiShaderResourceBinding::uniformBuffer(0,QRhiShaderResourceBinding::FragmentStage,pass.uniform.get()),
            QRhiShaderResourceBinding::sampledTexture(1,QRhiShaderResourceBinding::FragmentStage,back,sampler.get()),
            QRhiShaderResourceBinding::sampledTexture(2,QRhiShaderResourceBinding::FragmentStage,a,sampler.get()),
            QRhiShaderResourceBinding::sampledTexture(3,QRhiShaderResourceBinding::FragmentStage,b,sampler.get()),
            QRhiShaderResourceBinding::sampledTexture(4,QRhiShaderResourceBinding::FragmentStage,c,sampler.get())});
        if(!pass.bindings->create())return false;
        pass.pipeline.reset(rhi->newGraphicsPipeline());
        pass.pipeline->setFlags(QRhiGraphicsPipeline::UsesScissor);
        pass.pipeline->setShaderStages({{QRhiShaderStage::Vertex,vertex},{QRhiShaderStage::Fragment,fragment}});
        pass.pipeline->setShaderResourceBindings(pass.bindings.get());pass.pipeline->setRenderPassDescriptor(target.pass.get());
        return pass.pipeline->create();
    }
    std::unique_ptr<QRhiTexture> axis(int count,double start,double span,int limit,QByteArray &bytes){
        bytes=QByteArray(count*16,'\0');
        for(int i=0;i<count;++i){const double p=start+(double(i)+.5)*span/count;const float value=p<0 || p>=limit?-1.f:float(std::floor(p));std::memcpy(bytes.data()+i*16,&value,4);}
        std::unique_ptr<QRhiTexture> result(rhi->newTexture(QRhiTexture::RGBA32F,{count,1}));
        if(!result->create())return {};return result;
    }
    bool prepare(QRhiResourceUpdateBatch *batch){
        if(!prepareTargets())return false;
        passes.clear();maps.clear();
        QHash<quint64,int> locations;
        // Retain every source needed by this packet before choosing eviction slots.
        for(const auto &tile:packet->tiles)if(cache.contains(tile.content_id))locations.insert(tile.content_id,cache.value(tile.content_id));
        QSet<int> occupied;for(auto slot:locations)occupied.insert(slot);
        int next=0;quint64 bytes=0,uploads=0;
        const int frame=rhi->currentFrameSlot();
        if(frame<0 || frame>=staging.size())return fail("CPU: composition frame slot");
        staging[frame].clear();
        for(int i=0;i<packet->tiles.size();++i){
            const auto id=packet->tiles[i].content_id;if(locations.contains(id))continue;
            while(occupied.contains(next))++next;
            if(next>=1024)return fail("CPU: composition cache full");
            const auto previous=slotIds[size_t(next)];if(previous)cache.remove(previous);
            cache.insert(id,next);slotIds[size_t(next)]=id;locations.insert(id,next);occupied.insert(next);
            QRhiTextureSubresourceUploadDescription upload(packet->data[i]->bytes);upload.setDestinationTopLeft({(next%32)*64,(next/32)*64});upload.setSourceSize({64,64});
            batch->uploadTexture(atlas.get(),QRhiTextureUploadDescription({QRhiTextureUploadEntry(0,0,upload)}));
            staging[frame].append(packet->data[i]);++uploads;bytes+=65536;
        }
        counters->uploads.fetch_add(uploads);counters->uploadBytes.fetch_add(bytes);
        maps.resize(size_t(packet->nodes.size()));
        for(int i=0;i<packet->nodes.size();++i){
            if(!map(maps[size_t(i)],packet->nodes[i],locations))return fail("CPU: composition page table allocation");
            batch->uploadTexture(maps[size_t(i)].texture.get(),QRhiTextureUploadDescription({QRhiTextureUploadEntry(0,0,QRhiTextureSubresourceUploadDescription(maps[size_t(i)].bytes))}));
        }
        if(maps.empty())return fail("CPU: composition empty plan");
        QByteArray xs,ys;axisX=axis(outputSize.width(),packet->region.x(),packet->region.width(),documentSize.width(),xs);
        axisY=axis(outputSize.height(),packet->region.y(),packet->region.height(),documentSize.height(),ys);
        if(!axisX || !axisY)return fail("CPU: composition axis allocation");
        batch->uploadTexture(axisX.get(),QRhiTextureUploadDescription({QRhiTextureUploadEntry(0,0,QRhiTextureSubresourceUploadDescription(xs))}));
        batch->uploadTexture(axisY.get(),QRhiTextureUploadDescription({QRhiTextureUploadEntry(0,0,QRhiTextureSubresourceUploadDescription(ys))}));
        int current=0;
        if(!changed.isEmpty()){
            auto clear=std::make_unique<Pass>();clear->output=0;auto &p=clear->parameters;
            p.dimensions={documentSize.width(),documentSize.height(),outputSize.width(),outputSize.height()};p.factors={0,0,0,2};
            if(!makePass(*clear,linear[0],linear[1].texture.get(),atlas.get(),maps[0].texture.get(),maps[0].texture.get()))return fail("CPU: composition clear pipeline allocation");
            batch->updateDynamicBuffer(clear->uniform.get(),0,sizeof(p),&p);passes.push_back(std::move(clear));
        }
        for(int i=0;i<packet->nodes.size();++i){
            const auto &node=packet->nodes[i];if(node.kind!=0 || !node.visible)continue;
            if(changed.isEmpty()){current=1-current;continue;}
            const int mi=node.mask_index<quint32(packet->nodes.size())?int(node.mask_index):i;
            const auto &mask=packet->nodes[mi];
            const auto &srcMap=maps[size_t(i)],&maskMap=maps[size_t(mi)];
            auto pass=std::make_unique<Pass>();auto &p=pass->parameters;
            pass->output=1-current;
            p.dimensions={documentSize.width(),documentSize.height(),outputSize.width(),outputSize.height()};
            p.source={node.offset_x,node.offset_y,qint32(node.white_width),qint32(node.white_height)};
            p.mask={mask.offset_x,mask.offset_y,qint32(mask.white_width),qint32(mask.white_height)};
            p.sourceOrigin={srcMap.origin.x(),srcMap.origin.y(),0,0};p.maskOrigin={maskMap.origin.x(),maskMap.origin.y(),0,0};
            p.factors={node.opacity*node.fill,mi!=i && mask.visible?mask.opacity:0.f,0,0};
            if(!makePass(*pass,linear[1-current],linear[current].texture.get(),atlas.get(),srcMap.texture.get(),maskMap.texture.get()))return fail("CPU: composition pipeline allocation");
            batch->updateDynamicBuffer(pass->uniform.get(),0,sizeof(p),&p);passes.push_back(std::move(pass));current=1-current;
        }
        auto pass=std::make_unique<Pass>();auto &p=pass->parameters;
        pass->output=2;
        p.dimensions={documentSize.width(),documentSize.height(),outputSize.width(),outputSize.height()};p.factors={0,0,float(1<<packet->lod),1};
        if(!makePass(*pass,display,linear[current].texture.get(),axisX.get(),axisY.get(),maps[0].texture.get()))return fail("CPU: composition display pipeline allocation");
        batch->updateDynamicBuffer(pass->uniform.get(),0,sizeof(p),&p);passes.push_back(std::move(pass));return true;
    }
    void record(){
        if(failed || !dirty)return;
        if(qEnvironmentVariable("DRAWVERSE_COMPOSITION_FAIL")=="upload"){fail("CPU: injected composition upload failure");return;}
        auto *ri=window->rendererInterface();
        if(ri->getResource(window,QSGRendererInterface::RhiResource)!=rhi || rhi->isDeviceLost()){fail("CPU: composition device epoch lost");return;}
        auto *cb=static_cast<QRhiCommandBuffer*>(ri->getResource(window,QSGRendererInterface::RhiRedirectCommandBuffer));
        if(!cb){auto *swap=static_cast<QRhiSwapChain*>(ri->getResource(window,QSGRendererInterface::RhiSwapchainResource));if(swap)cb=swap->currentFrameCommandBuffer();}
        if(!cb){fail("CPU: composition window command buffer");return;}
        auto *batch=rhi->nextResourceUpdateBatch();
        if(!prepare(batch)){batch->release();return;}
        cb->resourceUpdate(batch);
        int finalLinear=0;
        for(const auto &node:packet->nodes)if(node.kind==0 && node.visible)finalLinear=1-finalLinear;
        for(size_t i=0;i<passes.size();++i){
            const auto &pass=*passes[i];const bool last=pass.output==2;auto &target=last?display:linear[size_t(pass.output)];const auto size=last?outputSize:documentSize;
            cb->beginPass(target.target.get(),Qt::transparent,{1,0});cb->setGraphicsPipeline(pass.pipeline.get());cb->setViewport({0,0,float(size.width()),float(size.height())});
            const auto scissor=last?QRect(QPoint(),size):changed;
            // QRhi scissor coordinates are bottom-left even on D3D11.
            cb->setScissor({scissor.x(),size.height()-scissor.y()-scissor.height(),scissor.width(),scissor.height()});
            cb->setShaderResources(pass.bindings.get());cb->draw(3);cb->endPass();
        }
        counters->passes.fetch_add(passes.size());++counters->adoptions;counters->revision=packet->info.key.revision;
        if(!changed.isEmpty()){
            const quint64 area=quint64(changed.width())*changed.height();counters->lastDirtyArea=area;
            counters->composedPixels.fetch_add(area*(passes.size()-1));
        }
        if(counters->readback && !readIssued){
            readIssued=true;auto *reads=rhi->nextResourceUpdateBatch();
            counters->testReadbacks.fetch_add(2);
            for(int i=0;i<2;++i){
                auto result=std::make_shared<QRhiReadbackResult>();const auto receiver=counters->readback;
                result->completed=[result,receiver,i]{
                    {std::lock_guard lock(receiver->mutex);(i?receiver->display:receiver->linear)=result->data;}
                    ++receiver->completed;auto completed=std::move(result->completed);
                };
                reads->readBackTexture(QRhiReadbackDescription(i?display.texture.get():linear[finalLinear].texture.get()),result.get());
            }
            cb->resourceUpdate(reads);
        }
        dirty=false;if(!rendered){rendered=true;fallback({});}
    }
    QQuickWindow *window;
    CompositionPacketPtr packet;
    std::shared_ptr<CompositionCounters> counters;
    std::function<void(QString)> fallback;
    QMetaObject::Connection connection;
    QRhi *rhi=nullptr;
    int inFlightFrames=0;
    std::shared_ptr<TextureReservation> reservation;
    bool dirty=true,failed=false,rendered=false,readIssued=false;
    DirectTexture texture;
    QShader vertex,fragment;
    QSize documentSize,outputSize;
    QRect changed;
    std::unique_ptr<QRhiSampler> sampler;
    std::unique_ptr<QRhiTexture> atlas,axisX,axisY;
    std::array<Target,2> linear;
    Target display;
    std::vector<PageMap> maps;
    std::vector<std::unique_ptr<Pass>> passes;
    QHash<quint64,int> cache;
    std::array<quint64,1024> slotIds{};
    QVector<QVector<std::shared_ptr<const CompositionTileData>>> staging;
};
}
#endif
quint64 CompositionNode::reservedPayloadBytes(){
#ifdef DRAWVERSE_HAS_COMPOSITION_RHI
    return reservedTextureBytes.load();
#else
    return 0;
#endif
}
CompositionNode *CompositionNode::create(QQuickWindow *window,CompositionPacketPtr packet,std::shared_ptr<CompositionCounters> counters,std::function<void(QString)> fallback){
#ifdef DRAWVERSE_HAS_COMPOSITION_RHI
    static const bool initialized=(initializeCompositionResources(),true);Q_UNUSED(initialized);
    auto *node=new RhiCompositionNode(window,std::move(packet),std::move(counters),std::move(fallback));
    if(!node->initialize()){delete node;return nullptr;}return node;
#else
    Q_UNUSED(window);Q_UNUSED(packet);Q_UNUSED(counters);fallback(QStringLiteral("CPU: Qt 6.6+ QRhi adapter unavailable"));return nullptr;
#endif
}
