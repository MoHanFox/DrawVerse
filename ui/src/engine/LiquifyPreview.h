#pragma once
#include <QImage>
#include <QMetaType>
#include <QSize>
#include <QVector>
#include <array>
#include <memory>
#include <atomic>

// Immutable CPU presentation packet. GPU resources never cross the render thread.
struct ResidentPreviewAssets {
    quint64 sourceId=0;
    QImage selection;
    std::array<float,4> identity{};
    std::array<QImage,3> images;
    // Count render-thread upload batches, without exposing graphics objects to the GUI.
    std::shared_ptr<std::atomic_uint64_t> uploads=std::make_shared<std::atomic_uint64_t>(0);
};
struct ResidentPreviewFrame : ResidentPreviewAssets {
    quint64 generation=0,revision=0;
    quint32 columns=0,rows=0;
    QSize documentSize;
    bool focus=false,forward=false;
    std::shared_ptr<const ResidentPreviewAssets> prepared;
    QVector<std::array<float,4>> vertices;
};
using LiquifyPreviewFrame = ResidentPreviewFrame;
using ResidentPreviewPtr=std::shared_ptr<const ResidentPreviewFrame>;
using LiquifyPreviewPtr=ResidentPreviewPtr;
Q_DECLARE_METATYPE(ResidentPreviewPtr)
