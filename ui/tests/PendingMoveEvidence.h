#pragma once
// One-test, opt-in failure evidence. No extra grabs, asynchronous callbacks or product waits.
#include "canvas/CanvasItem.h"
#include "platform/InteractionTrace.h"
#include <QtTest>
#include <QQuickWindow>
#include <QQuickItemGrabResult>
#include <QJsonDocument>
#include <QDir>
#include <QFile>
#include <QSaveFile>
#include <QScreen>
#include <QPointer>
#include <QScopeGuard>

class PendingMoveEvidence final : public QObject {
public:
    PendingMoveEvidence(CanvasItem *canvas,PaintCoreClient *client,QQuickWindow *window,quint64 owner)
        :m_canvas(canvas),m_client(client),m_window(window),m_owner(owner),
         m_directory(qEnvironmentVariable("DRAWVERSE_PENDING_MOVE_EVIDENCE")) {
        m_enabled=!m_directory.isEmpty();
        if(!m_enabled)return;
        if(!QDir(m_directory).exists() || QFile::exists(m_directory+"/pending-evidence.json"))
            qFatal("Pending move evidence requires an existing fresh output directory");
        InteractionTrace::start(true);
        qApp->installEventFilter(this);
        mark("observer-start");
    }
    ~PendingMoveEvidence() override {
        if(!m_enabled)return;
        qApp->removeEventFilter(this);
        mark("test-scope-exit");
        const auto trace=InteractionTrace::finish();
        QJsonObject report{{"schema",1},{"captures",m_captures},{"checkpoints",m_states},{"input_events",m_inputs},
            {"dropped",m_dropped},{"trace",trace},{"trace_dropped",double(InteractionTrace::dropped)},
            {"association","grab request/completion are GUI snapshots, NOT proof of the rendered frame identity; correlate trace separately"},
            {"any_pixel_match",m_anyPixelMatch},{"pending_assertion_passed",m_assertionPassed}};
        if(!m_firstFailure.isNull())report["first_failure_image_saved"]=m_firstFailure.save(m_directory+"/pending-first-failure.png");
        if(!m_lastFailure.isNull())report["last_failure_image_saved"]=m_lastFailure.save(m_directory+"/pending-last-failure.png");
        if(!m_success.isNull())report["success_image_saved"]=m_success.save(m_directory+"/pending-success.png");
        QSaveFile out(m_directory+"/pending-evidence.json");
        if(!out.open(QIODevice::WriteOnly) || out.write(QJsonDocument(report).toJson())<0 || !out.commit())
            qWarning("Unable to export pending move evidence");
    }
    void phase(const char *name) {m_phase=name;mark(name);}
    void expect(quint64 generation,QPoint delta,int depth) {m_generation=generation;m_delta=delta;m_depth=depth;}
    void passed() {m_assertionPassed=true;phase("pending-pixels-passed");}
    static bool acceptedContext(const QJsonObject &s,quint64 generation,QPoint delta,int depth) {
        const auto move=s["move"].toObject(),frame=s["frame"].toObject(),owner=s["owner_metadata"].toObject();
        return s["generation"].toDouble()==double(generation) && s["undo_depth"].toInt()==depth
            && s["error"].toString().isEmpty() && move["active"].toBool() && !move["starting"].toBool()
            && !move["closing"].toBool() && !move["dirty"].toBool() && move["submittedKind"].toInt()==4
            && s["sequence"].toDouble()>0 && move["completedSequence"].toDouble()>=s["sequence"].toDouble()
            && QPoint(move["deltaX"].toInt(),move["deltaY"].toInt())==delta
            && owner["id"].toDouble()==s["owner"].toDouble() && s["owner"].toDouble()>0
            && QPoint(owner["offsetX"].toInt(),owner["offsetY"].toInt())==delta
            && frame["generation"].toDouble()==double(generation) && frame["view"].toInt(-1)==0
            && frame["revision"].toDouble()==s["revision"].toDouble();
    }
    void mark(const char *name) {
        if(!m_enabled)return;
        append(m_states,QJsonObject{{"name",name},{"state",state()}});
    }
    QImage capture() {
        const auto id=++m_request;
        QJsonObject sample;
        if(m_enabled)sample={{"id",double(id)},{"phase",m_phase},{"request_ns",double(InteractionTrace::now())},{"request_state",state()}};
        // Same serialized grab and 5000ms completion bound as the original test.
        // The shared result lives through the wait. Nothing captures stack state asynchronously.
        const auto grab=m_canvas?m_canvas->grabToImage():QSharedPointer<QQuickItemGrabResult>{};
        QMetaObject::Connection ready;
        const auto disconnect=qScopeGuard([&]{QObject::disconnect(ready);});
        if(m_enabled && grab)ready=connect(grab.data(),&QQuickItemGrabResult::ready,this,[&]{
            sample["ready_ns"]=double(InteractionTrace::now());sample["ready_state"]=state();
        }); // GUI callback disconnected before any captured stack state can go away.
        const bool completed=grab && QTest::qWaitFor([&]{return !grab->image().isNull();},5000);
        QImage image=completed?grab->image():QImage{};
        if(m_enabled){
            sample["complete_ns"]=double(InteractionTrace::now());sample["complete_state"]=state();
            sample["completion_meaning"]="ready signal when recorded; complete_ns is GUI wait return, neither identifies render frame";
            sample["grab_created"]=bool(grab);sample["completed"]=completed;sample["null"]=image.isNull();
            sample["width"]=image.width();sample["height"]=image.height();sample["format"]=int(image.format());
            m_lastCapture=sample;
        }
        return image;
    }
    bool pendingMatches(const QImage &image) {
        // Evaluate both points from this one image, preserving the original AND and thresholds.
        const auto a=point(image,{100,1080},true),b=point(image,{800,1080},false);
        const bool pixels=!image.isNull() && a["matches"].toBool() && b["matches"].toBool();
        const auto current=state();
        const bool context=acceptedContext(current,m_generation,m_delta,m_depth);
        const bool good=pixels && context;
        m_anyPixelMatch=m_anyPixelMatch || pixels;
        if(m_enabled){
            auto sample=m_lastCapture;sample["white_point"]=a;sample["red_point"]=b;
            sample["pixel_and_result"]=pixels;sample["accepted_context"]=context;
            sample["and_result"]=good;sample["assertion_state"]=current;sample["assertion_ns"]=double(InteractionTrace::now());
            append(m_captures,sample);
            if(good)m_success=image;
            else {if(m_firstFailure.isNull())m_firstFailure=image;m_lastFailure=image;}
        }
        return good;
    }
protected:
    bool eventFilter(QObject *receiver,QEvent *event) override {
        if(!m_enabled)return false;
        const auto type=event->type();
        const bool application=type==QEvent::ApplicationDeactivate || type==QEvent::ApplicationStateChange;
        const bool target=receiver==m_canvas || receiver==m_window;
        const bool relevant=type==QEvent::MouseButtonPress || type==QEvent::MouseMove || type==QEvent::MouseButtonRelease
            || type==QEvent::UngrabMouse || type==QEvent::WindowDeactivate || type==QEvent::Hide || type==QEvent::FocusOut;
        if(application || (target && relevant)){
            QJsonObject row{{"type",int(type)},{"receiver",QString::number(quintptr(receiver))},{"state",state()}};
            if(type==QEvent::MouseButtonPress || type==QEvent::MouseMove || type==QEvent::MouseButtonRelease){
                const auto *mouse=static_cast<QMouseEvent*>(event);row["x"]=mouse->position().x();row["y"]=mouse->position().y();
                row["buttons"]=int(mouse->buttons());row["button"]=int(mouse->button());row["source"]=int(mouse->source());
                row["device"]=QString::number(mouse->pointingDevice()->systemId());
            }
            append(m_inputs,row);
        }
        return false; // Observe only; never alter acceptance, focus or cancellation.
    }
private:
    QJsonObject point(const QImage &image,QPointF document,bool expectWhite) const {
        const auto local=m_canvas?(m_canvas->documentRect().topLeft()+document*m_canvas->zoom()).toPoint():QPoint{};
        const QPoint pixel=local*(m_window?m_window->devicePixelRatio():1.);
        const bool inside=image.rect().contains(pixel);
        const auto color=inside?image.pixelColor(pixel):QColor{};
        const bool white=inside && color.red()>230 && color.green()>230 && color.blue()>230;
        const bool red=inside && color.red()>180 && color.green()<70 && color.blue()<70;
        QJsonObject result{{"document",QJsonArray{document.x(),document.y()}},{"local_rounded",QJsonArray{local.x(),local.y()}},
            {"image_coordinate",QJsonArray{pixel.x(),pixel.y()}},{"inside",inside},{"white",white},{"red",red},
            {"expected",expectWhite?"white":"red"},{"matches",expectWhite?white:red}};
        if(inside)result["rgba"]=QJsonArray{color.red(),color.green(),color.blue(),color.alpha()};
        return result;
    }
    QJsonObject state() const {
        if(!m_client || !m_canvas || !m_window)return {{"objects_alive",false}};
        const auto receipt=m_client->moveReceipt();const auto preview=m_client->residentPreview();const auto assets=m_client->movePreparation();
        QJsonObject result{{"ns",double(InteractionTrace::now())},{"owner",double(m_owner)},
            {"canvas",QString::number(quintptr(m_canvas.data()))},{"window",QString::number(quintptr(m_window.data()))},
            {"generation",double(m_client->generation())},{"revision",double(m_client->revision())},
            {"sequence",double(m_client->moveSequence())},{"move",QJsonObject::fromVariantMap(m_client->moveObservation())},
            {"frame",QJsonObject::fromVariantMap(m_client->frameIdentity())},{"frame_key",double(m_client->frame().cacheKey())},
            {"receipt",QJsonObject{{"kind",receipt.kind},{"sequence",double(receipt.sequence)},{"generation",double(receipt.generation)},{"revision",double(receipt.revision)}}},
            {"undo_depth",m_client->undoDepth()},{"error",m_client->lastError()},{"active",m_window->isActive()},
            {"visible",m_window->isVisible()},{"canvas_visible",m_canvas->isVisible()},
            {"mouse_grabber",QString::number(quintptr(m_window->mouseGrabberItem()))},
            {"gpu_display",m_canvas->gpuDisplay()},{"dpr",m_window->devicePixelRatio()},
            {"zoom",m_canvas->zoom()},{"document_origin",QJsonArray{m_canvas->documentRect().x(),m_canvas->documentRect().y()}},
            {"prepare_request",double(m_client->movePreparationRequest())},{"prepared_source",assets?double(assets->sourceId):0},
            {"prepare_uploads",assets?double(assets->uploads->load()):0},
            {"preview_source",preview?double(preview->sourceId):0},{"preview_revision",preview?double(preview->revision):0},
            {"gui_path",preview?"resident":"normal_or_interactive_frame (render trace required)"}};
        for(const auto &row:m_client->layers())if(row.toMap().value("id").toULongLong()==m_owner)result["owner_metadata"]=QJsonObject::fromVariantMap(row.toMap());
        return result;
    }
    void append(QJsonArray &array,const QJsonObject &row){if(array.size()<256)array.append(row);else ++m_dropped;}
    QPointer<CanvasItem> m_canvas;QPointer<PaintCoreClient> m_client;QPointer<QQuickWindow> m_window;
    quint64 m_owner=0,m_request=0,m_generation=0;QPoint m_delta;int m_depth=0;
    QString m_directory,m_phase="initial";bool m_enabled=false,m_anyPixelMatch=false,m_assertionPassed=false;
    int m_dropped=0;QJsonArray m_captures,m_states,m_inputs;QJsonObject m_lastCapture;
    QImage m_firstFailure,m_lastFailure,m_success;
};
