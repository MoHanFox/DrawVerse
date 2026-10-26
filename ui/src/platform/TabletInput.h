#pragma once
#include <QString>
#include <functional>

class QPointingDevice;
class QEvent;

// Selection is performed once before any application windows are created.
// Probe the actual enabled state: Qt can return true even when no driver opens.
QString selectTabletInputBackend(const QString &mode,const std::function<bool(bool)> &setWinTab,
                                const std::function<bool()> &winTabEnabled);
void configureTabletInput(const QString &mode);
QString activeTabletInputBackend();
bool acceptTabletInputDevice(const QString &backend,const QPointingDevice *device);
// Call before either canvas handling or tablet-to-mouse menu forwarding.
bool rejectTabletInputEvent(QEvent *event);

// Native Windows pen promotion is separate from Tablet and may appear as a
// primary-mouse event in Qt. Only suppress the redundant WinTab client stream.
bool isRedundantTabletMouseMessage(const QString &backend,quint32 message,quintptr extraInfo);
