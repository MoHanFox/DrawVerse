// Standalone recorder tests: hooks are absent from product translation units.
#define DRAWVERSE_TRACE_TEST_HOOKS
#include "platform/InteractionTrace.h"
#include <QtTest>
#include <future>
#include <condition_variable>
#include <thread>

namespace {
class Latch {
    std::mutex mutex;
    std::condition_variable condition;
    bool signaled=false;
public:
    void count_down(){std::lock_guard lock(mutex);signaled=true;condition.notify_all();}
    void wait(){std::unique_lock lock(mutex);condition.wait(lock,[&]{return signaled;});}
};
struct Barrier {
    InteractionTrace::TestStage pause;
    Latch entered, resume, stopped;
};
Barrier *barrier=nullptr;
void hook(InteractionTrace::TestStage stage) {
    if(stage==InteractionTrace::TestStage::Stopped)barrier->stopped.count_down();
    if(stage==barrier->pause){barrier->entered.count_down();barrier->resume.wait();}
}
void record(bool point) {
    if(point)InteractionTrace::point("worker.point",7);
    else {InteractionTrace::Scope scope("worker.scope");}
}
}
class InteractionTraceTests : public QObject {
    Q_OBJECT
private slots:
    void admittedRecordDrains_data(){QTest::addColumn<bool>("point");QTest::newRow("scope")<<false;QTest::newRow("point")<<true;}
    void admittedRecordDrains() {
        QFETCH(bool,point);
        InteractionTrace::start(true);
        Barrier b{InteractionTrace::TestStage::Accepted};barrier=&b;InteractionTrace::testHook=hook;
        std::thread worker([=]{record(point);});b.entered.wait();
        auto stopped=std::async(std::launch::async,[]{return InteractionTrace::finish();});
        b.stopped.wait();
        const bool returnedTooEarly=stopped.wait_for(std::chrono::milliseconds(100))==std::future_status::ready;
        b.resume.count_down();worker.join();const auto frozen=stopped.get();
        InteractionTrace::testHook=nullptr;
        QVERIFY2(!returnedTooEarly,"finish returned before an already admitted record completed");
        QCOMPARE(frozen.size(),1);QCOMPARE(InteractionTrace::finish(),frozen);
        record(point);QCOMPARE(InteractionTrace::finish(),frozen); // stopped arrivals rejected
    }
    void staleArrivalCannotEnterRestart_data(){admittedRecordDrains_data();}
    void staleArrivalCannotEnterRestart() {
        QFETCH(bool,point);
        InteractionTrace::start(true);
        Barrier b{InteractionTrace::TestStage::BeforeAdmission};barrier=&b;InteractionTrace::testHook=hook;
        std::thread worker([=]{record(point);});b.entered.wait();
        const auto old=InteractionTrace::finish();
        InteractionTrace::start(true);
        b.resume.count_down();worker.join();InteractionTrace::testHook=nullptr;
        InteractionTrace::point("new.session",1);const auto fresh=InteractionTrace::finish();
        QCOMPARE(old.size(),0);QCOMPARE(fresh.size(),1);
        QCOMPARE(fresh[0].toObject().value("name").toString(),QString("new.session"));
    }
};
QTEST_GUILESS_MAIN(InteractionTraceTests)
#include "interaction_trace_tests.moc"
