#include "canvas/ResidentPreviewTextures.h"
#include <QQuickWindow>
#include "platform/InteractionTrace.h"
#include <algorithm>
#include <cmath>
#include <map>
#include <mutex>
#include <tuple>

namespace {
std::mutex registryMutex;
std::map<std::tuple<quintptr,quint64,qint64>,std::weak_ptr<ResidentPreviewTextures>> registry;
}
int ResidentPreviewTextures::mipLevel(qreal pixelsPerTexel,bool nearest) {
    if(nearest || !std::isfinite(pixelsPerTexel) || pixelsPerTexel<=0 || pixelsPerTexel>=.5)return 0;
    return std::clamp(int(std::floor(std::log2(1/pixelsPerTexel))),0,12);
}
std::shared_ptr<ResidentPreviewTextures> ResidentPreviewTextures::acquire(QQuickWindow *window,const ResidentPreviewAssets &assets,
    const ResidentPreviewFrame *geometry,qreal density) {
    const auto key=std::make_tuple(reinterpret_cast<quintptr>(window),assets.sourceId,assets.images[0].cacheKey());
    std::shared_ptr<ResidentPreviewTextures> next;
    {
        std::lock_guard lock(registryMutex);
        for(auto it=registry.begin();it!=registry.end();)if(it->second.expired())it=registry.erase(it);else ++it;
        if(const auto found=registry.find(key);found!=registry.end())next=found->second.lock();
    }
    if(!next){
        InteractionTrace::Scope trace("move.render.createTextures");
        if(trace.active)trace.event.value=double(assets.sourceId);
        next=std::make_shared<ResidentPreviewTextures>();next->sourceId=assets.sourceId;next->m_uploads=assets.uploads;
        for(size_t i=0;i<3;++i)next->m_levels[i].push_back(assets.images[i]);
        QImage empty(1,1,QImage::Format_RGBA8888_Premultiplied);empty.fill(Qt::transparent);
        next->m_selection=window->createTextureFromImage(assets.selection.isNull()?empty:assets.selection,QQuickWindow::TextureHasAlphaChannel);
        if(!next->m_selection)return {};
        next->m_selection->setFiltering(QSGTexture::Linear);next->images[3]=next->m_selection;
        next->m_pending.push_back(next->m_selection);
        {std::lock_guard lock(registryMutex);registry[key]=next;}
    }
    if(!next->select(window,assets,geometry,density))return {};
    return next;
}
bool ResidentPreviewTextures::select(QQuickWindow *window,const ResidentPreviewAssets &assets,const ResidentPreviewFrame *geometry,qreal density) {
    // Estimate the greatest local source stretch, including perspective/Warp.
    // Static images use document density. Moving pixels also include transform
    // scale and their source UV span. Only the requested mip is uploaded.
    qreal movingDensity=0;
    if(geometry && geometry->meshPreview && geometry->columns>=2 && geometry->rows>=2 &&
       quint64(geometry->columns)*geometry->rows==quint64(geometry->vertices.size())){
        const auto &vertices=geometry->vertices;
        const auto stretch=[&](const auto &a,const auto &b){
            const qreal texels=std::hypot(qreal(b[2]-a[2])*assets.images[0].width(),qreal(b[3]-a[3])*assets.images[0].height());
            if(texels>1e-9)movingDensity=std::max(movingDensity,density*std::hypot(qreal(b[0]-a[0]),qreal(b[1]-a[1]))/texels);
        };
        for(quint32 y=0;y<geometry->rows;++y)for(quint32 x=0;x<geometry->columns;++x){
            const auto at=y*geometry->columns+x;
            if(x+1<geometry->columns)stretch(vertices[at],vertices[at+1]);
            if(y+1<geometry->rows)stretch(vertices[at],vertices[at+geometry->columns]);
        }
    }
    bool changed=false;
    for(size_t i=0;i<3;++i){
        const auto &base=m_levels[i].front();
        qreal screenDensity=density;
        if(geometry && i>0)screenDensity=density*std::max(qreal(geometry->documentSize.width())/base.width(),qreal(geometry->documentSize.height())/base.height());
        if(i==0)screenDensity=movingDensity;
        int level=assets.meshPreview?mipLevel(screenDensity,geometry && i==0 && geometry->meshNearest):0;
        auto &levels=m_levels[i];
        while(int(levels.size())<=level && (levels.back().width()>1 || levels.back().height()>1)){
            const auto previous=levels.back();
            levels.push_back(previous.scaled(std::max(1,previous.width()/2),std::max(1,previous.height()/2),Qt::IgnoreAspectRatio,Qt::SmoothTransformation));
            if(levels.back().isNull())return false;
        }
        level=std::min(level,int(levels.size())-1);
        auto &textures=m_textures[i];auto found=textures.find(level);
        if(found==textures.end()){
            auto *texture=window->createTextureFromImage(levels[size_t(level)],QQuickWindow::TextureHasAlphaChannel);
            if(!texture)return false;
            texture->setFiltering(assets.meshPreview?QSGTexture::Linear:QSGTexture::Nearest);
            texture->setMipmapFiltering(QSGTexture::None);
            texture->setHorizontalWrapMode(QSGTexture::ClampToEdge);texture->setVerticalWrapMode(QSGTexture::ClampToEdge);
            found=textures.emplace(level,texture).first;m_pending.push_back(texture);
            assets.uploadedTextureBytes->fetch_add(quint64(levels[size_t(level)].sizeInBytes()),std::memory_order_relaxed);
            InteractionTrace::point("transform.texture.mip",level);
            InteractionTrace::point("transform.texture.uploadBytes",double(levels[size_t(level)].sizeInBytes()));
        }
        if(i==0)assets.sourceMip->store(quint32(level),std::memory_order_relaxed);
        if(images[i]!=found->second){images[i]=found->second;changed=true;}
    }
    if(changed)++samplingRevision;
    return true;
}
ResidentPreviewTextures::~ResidentPreviewTextures(){
    for(auto &textures:m_textures)for(const auto &[level,texture]:textures){Q_UNUSED(level);delete texture;}
    delete m_selection;
}
void ResidentPreviewTextures::commit(QSGMaterialShader::RenderState &state) {
    if(m_pending.empty())return;
    InteractionTrace::Scope trace("move.render.uploadBatch");
    if(trace.active)trace.event.value=double(sourceId);
    for(auto *texture:m_pending)texture->commitTextureOperations(state.rhi(),state.resourceUpdateBatch());
    m_pending.clear();m_uploads->fetch_add(1,std::memory_order_relaxed);
}
