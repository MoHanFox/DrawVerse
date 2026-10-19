#pragma once
#include <QImage>
#include <QMetaType>
#include <QSize>
#include <QVector>
#include <array>
#include <memory>

// Immutable CPU presentation packet. GPU resources never cross the render thread.
struct LiquifyPreviewFrame {
    quint64 generation=0,revision=0,sourceId=0;
    quint32 columns=0,rows=0;
    QSize documentSize;
    bool focus=false;
    std::array<QImage,3> images;
    QVector<std::array<float,4>> vertices;
};
using LiquifyPreviewPtr=std::shared_ptr<const LiquifyPreviewFrame>;
Q_DECLARE_METATYPE(LiquifyPreviewPtr)
