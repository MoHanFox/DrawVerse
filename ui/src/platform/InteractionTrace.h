#pragma once
// Investigation-only recorder. Disabled unless explicitly started by the opt-in harness.
#include <QJsonArray>
#include <QJsonObject>
#include <QThread>
#include <atomic>
#include <chrono>
#include <mutex>
#include <vector>

namespace InteractionTrace {
inline std::atomic<bool> enabled{false};
inline std::atomic<quint64> nextId{0};
inline std::atomic<int> activeScopes{0};
inline thread_local quint64 parent=0;
inline thread_local int operation=-1;
inline qint64 now() { return std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now().time_since_epoch()).count(); }
struct Event {const char *name; qint64 start,end; quint64 id,parent,thread; int operation; double value;};
inline std::mutex mutex;
inline std::vector<Event> events;
inline quint64 dropped=0;
inline constexpr size_t capacity=200000;
inline void append(Event event) {
    std::lock_guard lock(mutex);
    if(events.size()<capacity)events.push_back(event);else ++dropped;
}
inline void start(bool active) {
    std::lock_guard lock(mutex);events.clear();events.reserve(capacity);dropped=0;enabled=active;
}
struct Scope {
    Event event{}; bool active;
    explicit Scope(const char *name):active(enabled.load(std::memory_order_relaxed)) {
        if(active){++activeScopes;event={name,now(),0,++nextId,parent,quint64(quintptr(QThread::currentThreadId())),operation,0};parent=event.id;}
    }
    ~Scope(){if(active){event.end=now();parent=event.parent;append(event);--activeScopes;}}
};
inline void point(const char *name,double value) {
    if(!enabled.load(std::memory_order_relaxed))return;
    const auto time=now();append({name,time,time,++nextId,parent,quint64(quintptr(QThread::currentThreadId())),operation,value});
}
inline QJsonArray finish() {
    enabled=false;std::lock_guard lock(mutex);QJsonArray result;
    for(const auto &e:events)result.append(QJsonObject{{"name",e.name},{"start_ns",double(e.start)},{"duration_us",double(e.end-e.start)/1000},
        {"id",double(e.id)},{"parent",double(e.parent)},{"thread",QString::number(e.thread)},{"operation",e.operation},{"value",e.value}});
    return result;
}
}
