#pragma once
#include "engine/LiquifyPreview.h"
#include <QSGMaterialShader>
#include <QSGTexture>
#include <map>
#include <vector>
class QQuickWindow;

// Render-thread ownership only. The registry keeps weak references; nodes retain
// at most the identity and moving sources of their active edit session.
class ResidentPreviewTextures final {
public:
    static std::shared_ptr<ResidentPreviewTextures> acquire(QQuickWindow *window,const ResidentPreviewAssets &assets,
        const ResidentPreviewFrame *geometry=nullptr,qreal pixelsPerDocumentPixel=1);
    static int mipLevel(qreal pixelsPerTexel,bool nearest=false);
    ~ResidentPreviewTextures();
    void commit(QSGMaterialShader::RenderState &state);
    quint64 sourceId=0,samplingRevision=0;
    std::array<QSGTexture*,4> images{};
private:
    std::shared_ptr<std::atomic_uint64_t> m_uploads;
    bool select(QQuickWindow *window,const ResidentPreviewAssets &assets,const ResidentPreviewFrame *geometry,qreal density);
    std::array<std::vector<QImage>,3> m_levels;
    std::array<std::map<int,QSGTexture*>,3> m_textures;
    QSGTexture *m_selection=nullptr;
    std::vector<QSGTexture*> m_pending;
};
