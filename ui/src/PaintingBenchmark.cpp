#include "PaintingBenchmark.h"
#include "PaintCoreClient.h"
#include <QCoreApplication>
#include <QElapsedTimer>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QQuickWindow>
#include <QTimer>
#include <algorithm>
#include <cmath>
#include <memory>
#include <vector>

namespace {
struct Measurement {
    QElapsedTimer clock;
    std::vector<double> frameGaps,tickGaps;
    qint64 lastFrame=-1,lastTick=0,inputEnd=0;
    quint64 revision=0;
    int sample=0,stage=0,frames=0,presentations=0;
};
double percentile(std::vector<double> values,double fraction) {
    if(values.empty()) return 0;
    std::sort(values.begin(),values.end());
    return values[static_cast<size_t>(fraction*static_cast<double>(values.size()-1))];
}
}
void startPaintingBenchmark(PaintCoreClient &client,QQuickWindow &window,const QString &output) {
    auto state=std::make_shared<Measurement>();
    auto *timer=new QTimer(&client); timer->setInterval(8); timer->setTimerType(Qt::PreciseTimer);
    QObject::connect(&client,&PaintCoreClient::frameChanged,&client,[state,&client] {
        if(state->stage!=1 && state->stage!=2) return;
        if(client.frameRevision()==state->revision) return;
        state->revision=client.frameRevision(); const auto now=state->clock.elapsed();
        if(state->lastFrame>=0) state->frameGaps.push_back(static_cast<double>(now-state->lastFrame));
        state->lastFrame=now; ++state->frames;
    });
    QObject::connect(&window,&QQuickWindow::frameSwapped,&client,[state] {if(state->stage==1 || state->stage==2) ++state->presentations;});
    QObject::connect(timer,&QTimer::timeout,&client,[state,timer,&client,&window,output] {
        if(state->stage==0) {
            if(!client.ready() || client.frame().isNull()) return;
            client.setBrushRadius(24); client.setBrushColor(QColor("#2ea99d"));
            state->clock.start(); state->stage=1; state->revision=client.frameRevision();
        }
        const auto now=state->clock.elapsed();
        if(state->stage==1) {
            if(state->sample>0) state->tickGaps.push_back(static_cast<double>(now-state->lastTick));
            state->lastTick=now;
            const int i=state->sample,step=i%100,row=i/100;
            InputSample s; s.position={80.+step*8.,80.+row*85.+std::sin(step*.08)*30.};
            s.pressure=static_cast<float>(.25+.7*std::sin((step+1)*3.14159265/200));s.tool=1;s.capabilities=1;
            if(step==0) {if(!client.beginStroke(s)) {timer->stop();client.shutdown();return;}}
            else client.strokeTo(s);
            if(step==99) client.endStroke();
            if(++state->sample==600) {state->stage=2;state->inputEnd=now;}
        } else if(state->stage==2 && !client.drawing() && client.undoDepth()==6 && client.frameRevision()>=client.revision()) {
            state->stage=3;timer->stop();
            const QJsonObject report{{"samples",state->sample},{"frame_updates",state->frames},{"presentations",state->presentations},
                {"duration_ms",now},{"catchup_ms",now-state->inputEnd},{"frame_gap_p50_ms",percentile(state->frameGaps,.5)},
                {"frame_gap_p95_ms",percentile(state->frameGaps,.95)},{"gui_tick_p95_ms",percentile(state->tickGaps,.95)},
                {"dpr",window.devicePixelRatio()},{"output_width",client.frame().width()},{"output_height",client.frame().height()},
                {"error",client.lastError()}};
            QFile file(output); if(file.open(QIODevice::WriteOnly)) file.write(QJsonDocument(report).toJson());
            else qWarning("Painting benchmark report could not be written");
            client.shutdown();
        }
    });
    timer->start();QTimer::singleShot(30000,&client,[&client,timer]{timer->stop();client.shutdown();});
}
