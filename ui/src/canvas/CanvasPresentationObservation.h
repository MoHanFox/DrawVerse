#pragma once
// Opt-in, one-slot observation of the node actually adopted by CanvasItem.
// No GPU objects or GUI QObject access cross this boundary. The ordinary path
// has no observer, allocation, lock, timer, signal or extra update request.
#include <QRectF>
#include <QSizeF>
#include <QThread>
#include <mutex>

struct CanvasPresentationIdentity {
    quintptr canvas=0,window=0;
    quint64 generation=0,source=0,revision=0;
    qint64 imageKey=0;
    QRectF region,document;
    QSizeF size;
    qreal zoom=0;
    bool resident=false,valid=false;
    bool operator==(const CanvasPresentationIdentity &) const = default;
};
struct CanvasPresentationObservation {
    struct Receipt {CanvasPresentationIdentity identity;quint64 epoch=0,thread=0;};
    void adopted(const CanvasPresentationIdentity &identity) {
        std::lock_guard lock(mutex);
        if(!(receipt.identity==identity)){receipt.identity=identity;++receipt.epoch;}
        receipt.thread=quint64(quintptr(QThread::currentThreadId()));
    }
    Receipt snapshot() const {std::lock_guard lock(mutex);return receipt;}
private:
    mutable std::mutex mutex;
    Receipt receipt;
};
