#pragma once
#include <QImage>
#include <QMetaType>
#include <QSize>
#include <QVector>
#include <array>
#include <memory>

// Immutable CPU presentation packet. GPU resources never cross the render thread.
struct ResidentPreviewFrame {
    quint64 generation=0,revision=0,sourceId=0;
    quint32 columns=0,rows=0;
    QSize documentSize;
    bool focus=false,forward=false;
    QImage selection;
    std::array<float,4> identity{};
    std::array<QImage,3> images;
    QVector<std::array<float,4>> vertices;
};
using LiquifyPreviewFrame = ResidentPreviewFrame;
using ResidentPreviewPtr=std::shared_ptr<const ResidentPreviewFrame>;
using LiquifyPreviewPtr=ResidentPreviewPtr;
Q_DECLARE_METATYPE(ResidentPreviewPtr)
