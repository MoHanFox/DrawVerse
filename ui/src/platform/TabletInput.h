#pragma once
#include <QString>
#include <functional>

// Selection is performed once before any application windows are created.
// Probe the actual enabled state: Qt can return true even when no driver opens.
QString selectTabletInputBackend(const QString &mode,const std::function<bool(bool)> &setWinTab,
                                const std::function<bool()> &winTabEnabled);
void configureTabletInput(const QString &mode);
QString activeTabletInputBackend();
