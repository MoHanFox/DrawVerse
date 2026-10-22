#pragma once
// Investigation-only recorder. Disabled unless explicitly started by the opt-in harness.
#include <QJsonArray>
#include <QJsonObject>
#include <QThread>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <mutex>
#include <vector>

namespace InteractionTrace {
#ifdef DRAWVERSE_TRACE_TEST_HOOKS
enum class TestStage { BeforeAdmission, Accepted, Stopped };
inline void (*testHook)(TestStage)=nullptr;
inline void checkpoint(TestStage stage){if(testHook)testHook(stage);}
#define TRACE_CHECKPOINT(stage) checkpoint(TestStage::stage)
#else
#define TRACE_CHECKPOINT(stage) ((void)0)
#endif
// Observational flag for optional call-site metadata, not an admission permit.
inline std::atomic<bool> enabled{false};
// Odd = accepting, even = stopped; each start gets a new epoch. The disabled
// path is one atomic read, with no mutex or allocation.
inline std::atomic<quint64> gate{0};
inline std::atomic<quint64> nextId{0};
inline thread_local quint64 parent=0;
inline thread_local int operation=-1;
inline qint64 now() { return std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now().time_since_epoch()).count(); }
struct Event {const char *name; qint64 start,end; quint64 id,parent,thread; int operation; double value;};
inline std::mutex mutex;
inline std::mutex lifecycle;
inline std::condition_variable drained;
inline int activeRecords=0;
inline std::vector<Event> events;
inline quint64 dropped=0;
inline constexpr size_t capacity=200000;
inline bool admit() {
    const auto epoch=gate.load(std::memory_order_acquire);
    if(!(epoch&1))return false;
    TRACE_CHECKPOINT(BeforeAdmission);
    {
        std::lock_guard lock(mutex);
        // Reject candidates paused across stop, including across a restart.
        if(gate.load(std::memory_order_relaxed)!=epoch)return false;
        ++activeRecords;
    }
    TRACE_CHECKPOINT(Accepted);
    return true;
}
inline void complete(Event event) {
    std::lock_guard lock(mutex);
    if(events.size()<capacity)events.push_back(event);else ++dropped;
    if(--activeRecords==0)drained.notify_all();
}
// Caller holds lifecycle. Never wait while owning the record mutex.
inline void stopAndDrain() {
    {
        std::lock_guard lock(mutex);
        const auto epoch=gate.load(std::memory_order_relaxed);
        if(epoch&1)gate.store(epoch+1,std::memory_order_release);
        enabled.store(false,std::memory_order_release);
    }
    TRACE_CHECKPOINT(Stopped);
    std::unique_lock lock(mutex);
    drained.wait(lock,[]{return activeRecords==0;});
}
inline void start(bool active) {
    std::lock_guard control(lifecycle);
    stopAndDrain();
    std::lock_guard lock(mutex);
    events.clear();dropped=0;
    if(active){events.reserve(capacity);gate.fetch_add(1,std::memory_order_release);}
    enabled.store(active,std::memory_order_release);
}
struct Scope {
    Event event{}; bool active;
    explicit Scope(const char *name):active(admit()) {
        if(active){event={name,now(),0,++nextId,parent,quint64(quintptr(QThread::currentThreadId())),operation,0};parent=event.id;}
    }
    Scope(const Scope &)=delete;
    Scope &operator=(const Scope &)=delete;
    ~Scope(){if(active){event.end=now();parent=event.parent;complete(event);}}
};
inline void point(const char *name,double value) {
    if(!admit())return;
    const auto time=now();complete({name,time,time,++nextId,parent,quint64(quintptr(QThread::currentThreadId())),operation,value});
}
// Invoke outside recorded scopes. Serializes start/finish, closes admission,
// drains both Scope and point, then freezes/exports with no possible append.
inline QJsonArray finish() {
    std::lock_guard control(lifecycle);stopAndDrain();
    std::lock_guard lock(mutex);QJsonArray result;
    for(const auto &e:events)result.append(QJsonObject{{"name",e.name},{"start_ns",double(e.start)},{"duration_us",double(e.end-e.start)/1000},
        {"id",double(e.id)},{"parent",double(e.parent)},{"thread",QString::number(e.thread)},{"operation",e.operation},{"value",e.value}});
    return result;
}
#undef TRACE_CHECKPOINT
}
