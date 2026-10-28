#pragma once
#include <QtGlobal>
#include <algorithm>
#ifdef Q_OS_WIN
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#endif
inline int defaultMemoryMiB(){
#ifdef Q_OS_WIN
    MEMORYSTATUSEX info{};info.dwLength=sizeof(info);
    if(GlobalMemoryStatusEx(&info))return int(std::min<quint64>(1048576,info.ullTotalPhys/(2ull*1024*1024)));
#endif
    return 2048;
}
