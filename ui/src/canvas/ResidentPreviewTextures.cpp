#include "canvas/ResidentPreviewTextures.h"
#include <QQuickWindow>
#include "platform/InteractionTrace.h"
#include <map>
#include <mutex>

namespace {
std::mutex registryMutex;
std::map<std::pair<quintptr,quint64>,std::weak_ptr<ResidentPreviewTextures>> registry;
}
std::shared_ptr<ResidentPreviewTextures> ResidentPreviewTextures::acquire(QQuickWindow *window,const ResidentPreviewAssets &assets) {
    const auto key=std::make_pair(reinterpret_cast<quintptr>(window),assets.sourceId);
    {
        std::lock_guard lock(registryMutex);
        for(auto it=registry.begin();it!=registry.end();)if(it->second.expired())it=registry.erase(it);else ++it;
        if(const auto found=registry.find(key);found!=registry.end())if(auto existing=found->second.lock())return existing;
    }
    InteractionTrace::Scope trace("move.render.createTextures");
    if(trace.active)trace.event.value=double(assets.sourceId);
    if(trace.active){qint64 bytes=assets.selection.sizeInBytes();for(const auto &image:assets.images)bytes+=image.sizeInBytes();InteractionTrace::point("move.texture.hostBytes",double(bytes));}
    auto next=std::make_shared<ResidentPreviewTextures>();next->sourceId=assets.sourceId;next->m_uploads=assets.uploads;
    QImage empty(1,1,QImage::Format_RGBA8888_Premultiplied);empty.fill(Qt::transparent);
    for(size_t i=0;i<4;++i){
        const auto &image=i==3?(assets.selection.isNull()?empty:assets.selection):assets.images[i];
        next->images[i]=window->createTextureFromImage(image,QQuickWindow::TextureHasAlphaChannel);
        if(!next->images[i])return {};
        next->images[i]->setFiltering(i==3?QSGTexture::Linear:QSGTexture::Nearest);
        next->images[i]->setHorizontalWrapMode(QSGTexture::ClampToEdge);next->images[i]->setVerticalWrapMode(QSGTexture::ClampToEdge);
    }
    // A window's nodes are serialized on its own scene-graph thread. Different
    // windows never share graphics resources, even for the same immutable source.
    {std::lock_guard lock(registryMutex);registry[key]=next;}
    return next;
}
ResidentPreviewTextures::~ResidentPreviewTextures(){for(auto *texture:images)delete texture;}
void ResidentPreviewTextures::commit(QSGMaterialShader::RenderState &state) {
    if(m_committed)return;
    InteractionTrace::Scope trace("move.render.uploadBatch");
    if(trace.active)trace.event.value=double(sourceId);
    for(auto *texture:images)texture->commitTextureOperations(state.rhi(),state.resourceUpdateBatch());
    m_committed=true;m_uploads->fetch_add(1,std::memory_order_relaxed);
}
