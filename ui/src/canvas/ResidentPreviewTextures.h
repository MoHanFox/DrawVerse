#pragma once
#include "engine/LiquifyPreview.h"
#include <QSGMaterialShader>
#include <QSGTexture>
class QQuickWindow;

// Render-thread ownership only. The registry keeps weak references; nodes retain
// at most the identity and moving sources of their active edit session.
class ResidentPreviewTextures final {
public:
    static std::shared_ptr<ResidentPreviewTextures> acquire(QQuickWindow *window,const ResidentPreviewAssets &assets);
    ~ResidentPreviewTextures();
    void commit(QSGMaterialShader::RenderState &state);
    quint64 sourceId=0;
    std::array<QSGTexture*,4> images{};
private:
    std::shared_ptr<std::atomic_uint64_t> m_uploads;
    bool m_committed=false;
};
