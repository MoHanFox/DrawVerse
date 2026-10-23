#pragma once
// Opt-in, one-slot observation of the node actually adopted by CanvasItem.
// No GPU objects or GUI QObject access cross this boundary. The ordinary path
// has no observer, allocation, lock, timer, signal or extra update request.
#include <QRectF>
#include <QSizeF>
#include <QThread>
#include "platform/InteractionTrace.h"
#include <mutex>
#include <vector>

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
    struct Receipt {CanvasPresentationIdentity identity;quint64 epoch=0,thread=0;qint64 ns=0;};
    // Configured before installing the observer. The normal one-slot protocol
    // does not allocate history; C3 explicitly requests a bounded history.
    explicit CanvasPresentationObservation(size_t capacity=0):capacity(qMin(capacity,size_t(4096))) {}
    void adopted(const CanvasPresentationIdentity &identity) {
        std::lock_guard lock(mutex);
        receipt.thread=quint64(quintptr(QThread::currentThreadId()));
        if(!(receipt.identity==identity)){
            receipt.identity=identity;++receipt.epoch;receipt.ns=InteractionTrace::now();
            if(capacity){if(history.size()<capacity)history.push_back(receipt);else ++dropped;}
        }
    }
    Receipt snapshot() const {std::lock_guard lock(mutex);return receipt;}
    std::vector<Receipt> samples() const {std::lock_guard lock(mutex);return history;}
    quint64 overflow() const {std::lock_guard lock(mutex);return dropped;}
private:
    mutable std::mutex mutex;
    Receipt receipt;
    size_t capacity=0;
    std::vector<Receipt> history;
    quint64 dropped=0;
};
