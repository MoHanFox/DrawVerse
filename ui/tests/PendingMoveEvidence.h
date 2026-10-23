#pragma once
// One-test acceptance protocol, with opt-in bounded export. No extra grabs or product waits.
#include "canvas/CanvasItem.h"
#include "canvas/CanvasPresentationObservation.h"
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
    PendingMoveEvidence(CanvasItem *canvas,PaintCoreClient *client,QQuickWindow *window,quint64 owner,size_t historyCapacity=0,bool memoryEvidence=false)
        :m_canvas(canvas),m_client(client),m_window(window),m_owner(owner),
         m_directory(qEnvironmentVariable("DRAWVERSE_PENDING_MOVE_EVIDENCE")) {
        m_enabled=!m_directory.isEmpty();m_record=m_enabled || memoryEvidence;
        if(m_enabled && (!QDir(m_directory).exists() || QFile::exists(m_directory+"/pending-evidence.json")))
            qFatal("Pending move evidence requires an existing fresh output directory");
        if(m_enabled)InteractionTrace::start(true);
        m_presentation=std::make_shared<CanvasPresentationObservation>(historyCapacity);canvas->observePresentation(m_presentation);
        qApp->installEventFilter(this);
        const auto changed=[this]{observeContext();};
        connect(client,&PaintCoreClient::stateChanged,this,changed);
        connect(client,&PaintCoreClient::frameChanged,this,changed);
        connect(client,&PaintCoreClient::errorChanged,this,changed);
        connect(canvas,&CanvasItem::viewChanged,this,changed);
        connect(canvas,&CanvasItem::clientChanged,this,changed);
        connect(canvas,&QQuickItem::visibleChanged,this,changed);
        connect(canvas,&QQuickItem::widthChanged,this,changed);connect(canvas,&QQuickItem::heightChanged,this,changed);
        connect(canvas,&CanvasItem::residentPresentationChanged,this,changed);
        observeContext();
        mark("observer-start");
    }
    ~PendingMoveEvidence() override {
        qApp->removeEventFilter(this);
        if(m_canvas)m_canvas->observePresentation({});
        if(!m_enabled)return;
        mark("test-scope-exit");
        const auto trace=InteractionTrace::finish();
        QJsonObject report{{"schema",2},{"captures",m_captures},{"checkpoints",m_states},{"input_events",m_inputs},{"planned_inputs",m_plans},
            {"dropped",m_dropped},{"trace",trace},{"trace_dropped",double(InteractionTrace::dropped)},
            {"association","serial Canvas grab; stable GUI content, input and render-adopted identity epochs from request through ready and assertion; not physical presentation latency"},
            {"scenario_invalid",m_invalid},{"input_epoch",double(m_inputEpoch)},{"grab_failed",m_grabFailed},
            {"any_pixel_match",m_anyPixelMatch},{"pending_assertion_passed",m_assertionPassed}};
        if(!m_firstFailure.isNull())report["first_failure_image_saved"]=m_firstFailure.save(m_directory+"/pending-first-failure.png");
        if(!m_lastFailure.isNull())report["last_failure_image_saved"]=m_lastFailure.save(m_directory+"/pending-last-failure.png");
        if(!m_success.isNull())report["success_image_saved"]=m_success.save(m_directory+"/pending-success.png");
        QSaveFile out(m_directory+"/pending-evidence.json");
        if(!out.open(QIODevice::WriteOnly) || out.write(QJsonDocument(report).toJson())<0 || !out.commit())
            qWarning("Unable to export pending move evidence");
    }
    void phase(const char *name) {m_phase=name;mark(name);}
    QJsonObject observation() {observeContext();return state();}
    quint64 planId() const {return m_planId;}
    std::shared_ptr<CanvasPresentationObservation> presentation() const {return m_presentation;}
    void acceptResidentContextForProbe() {m_allowResident=true;}
    QJsonObject evidenceData() const {return {{"input_events",m_inputs},{"planned_inputs",m_plans},{"captures",m_captures},{"dropped",m_dropped}};}
    void expect(quint64 generation,QPoint delta,int depth) {
        m_generation=generation;m_delta=delta;m_depth=depth;m_fixedView=viewIdentity(state());m_armed=true;
    }
    void passed() {m_assertionPassed=true;m_armed=false;phase("pending-pixels-passed");}
    void send(QEvent::Type type,QPointF position) {
        // A specific live event object, not source/device/time proximity, proves
        // test provenance at the QWindow ingress. Quick delivers it normally.
        QMouseEvent event(type,position,m_window->mapToGlobal(position),type==QEvent::MouseMove?Qt::NoButton:Qt::LeftButton,
            type==QEvent::MouseButtonRelease?Qt::NoButton:Qt::LeftButton,Qt::NoModifier);
        const auto id=++m_planId;const auto start=InteractionTrace::now();m_plannedEvent=&event;
        QCoreApplication::sendEvent(m_window,&event);m_plannedEvent=nullptr;
        if(m_armed && type==QEvent::MouseButtonPress)m_beginSequence=m_client->moveSequence();
        if(type==QEvent::MouseMove)m_targetPlan=id;
        if(m_record)append(m_plans,{{"id",double(id)},{"type",int(type)},{"window",QString::number(quintptr(m_window.data()))},
            {"x",position.x()},{"y",position.y()},{"button",int(event.button())},{"buttons",int(event.buttons())},
            {"modifiers",int(event.modifiers())},{"start_ns",double(start)},{"end_ns",double(InteractionTrace::now())},{"accepted",event.isAccepted()}});
        observeContext();
    }
    static bool acceptedContext(const QJsonObject &s,quint64 generation,QPoint delta,int depth,bool allowResident=false) {
        const auto move=s["move"].toObject(),frame=s["frame"].toObject(),owner=s["owner_metadata"].toObject();
        return s["generation"].toDouble()==double(generation) && s["undo_depth"].toInt()==depth
            && s["error"].toString().isEmpty() && move["active"].toBool() && !move["starting"].toBool()
            && !move["closing"].toBool() && !move["dirty"].toBool() && move["submittedKind"].toInt()==4
            && s["sequence"].toDouble()>0 && move["completedSequence"].toDouble()>=s["sequence"].toDouble()
            && QPoint(move["deltaX"].toInt(),move["deltaY"].toInt())==delta
            && owner["id"].toDouble()==s["owner"].toDouble() && s["owner"].toDouble()>0
            && QPoint(owner["offsetX"].toInt(),owner["offsetY"].toInt())==delta
            && frame["generation"].toDouble()==double(generation) && frame["view"].toInt(-1)==0
            && (frame["revision"].toDouble()==s["revision"].toDouble()
                || (allowResident && s["preview_source"].toDouble()>0 && s["preview_revision"]==s["revision"]));
    }
    static QJsonObject viewIdentity(const QJsonObject &s) {
        QJsonObject view;for(const auto *key:{"generation","canvas","window","client_matches","zoom","document_origin","size","dpr"})view[key]=s[key];
        return view;
    }
    static bool associated(const QJsonObject &sample,quint64 generation,QPoint delta,int depth,bool allowResident=false) {
        if(sample["id"].toInt()<=0 || !sample["completed"].toBool() || sample["image_key"].toString().isEmpty()
            || sample["ready_id"]!=sample["id"])return false;
        const auto request=sample["request_state"].toObject();
        const auto render=request["render"].toObject();
        if(!render["valid"].toBool() || request["scenario_invalid"].toBool() || request["target_plan"].toInt()<2
            || request["context_epoch"].toInt()<=0 || request["input_epoch"].toInt()<=0 || render["epoch"].toInt()<=0
            || request["frame_key"].toString().isEmpty() || !request["client_matches"].toBool()
            || request["begin_sequence"].toInt()<=0 || request["target_sequence"].toInt()<=request["begin_sequence"].toInt()
            || request["sequence"]!=request["target_sequence"]
            || request["canvas"]!=sample["grab_canvas"] || render["canvas"]!=request["canvas"]
            || render["window"]!=request["window"] || render["generation"]!=request["generation"]
            || render["document_origin"]!=request["document_origin"] || render["zoom"]!=request["zoom"]
            || render["size"]!=request["size"])return false;
        if(render["resident"].toBool()) {
            if(render["source"]!=request["preview_source"] || render["revision"]!=request["revision"])return false;
        } else if(render["image_key"]!=request["frame_key"] || render["region"]!=request["frame_region"])return false;
        for(const auto *key:{"request_state","ready_state","complete_state","assertion_state"}) {
            const auto s=sample[key].toObject();
            if(!acceptedContext(s,generation,delta,depth,allowResident && render["resident"].toBool()) || s["scenario_invalid"].toBool()
                || !s["active"].toBool() || !s["visible"].toBool() || !s["canvas_visible"].toBool())return false;
            for(const auto *field:{"context_epoch","input_epoch","target_plan","canvas","window","frame","frame_key","preview_source",
                    "preview_revision","render","dpr","zoom","document_origin","size","sequence","begin_sequence","target_sequence"})
                if(s[field]!=request[field])return false;
        }
        return true;
    }
    void mark(const char *name) {
        if(!m_record)return;
        append(m_states,QJsonObject{{"name",name},{"state",state()}});
    }
    QImage capture() {
        if(m_grabFailed)return {}; // No second outstanding grab after a timeout.
        const auto id=++m_request;
        observeContext();
        QJsonObject sample{{"id",double(id)},{"phase",m_phase},{"grab_canvas",QString::number(quintptr(m_canvas.data()))},
            {"request_ns",double(InteractionTrace::now())},{"request_state",state()}};
        // Same serialized grab and 5000ms completion bound as the original test.
        // The shared result lives through the wait. Nothing captures stack state asynchronously.
        const auto grab=m_canvas?m_canvas->grabToImage():QSharedPointer<QQuickItemGrabResult>{};
        QMetaObject::Connection ready;
        const auto disconnect=qScopeGuard([&]{QObject::disconnect(ready);});
        if(grab)ready=connect(grab.data(),&QQuickItemGrabResult::ready,this,[&]{
            observeContext();sample["ready_id"]=double(id);
            sample["ready_ns"]=double(InteractionTrace::now());sample["ready_state"]=state();
        }); // GUI callback disconnected before any captured stack state can go away.
        const bool completed=grab && QTest::qWaitFor([&]{return !grab->image().isNull();},5000);
        if(!completed){m_grabFailed=true;m_failedGrab=grab;}
        QImage image=completed?grab->image():QImage{};
        observeContext();
        {
            sample["complete_ns"]=double(InteractionTrace::now());sample["complete_state"]=state();
            sample["completion_meaning"]="serial same-item grab ready; identity established only by associated() stable-state protocol, not latest getter alone";
            sample["grab_created"]=bool(grab);sample["completed"]=completed;sample["null"]=image.isNull();
            sample["width"]=image.width();sample["height"]=image.height();sample["format"]=int(image.format());
            sample["image_key"]=QString::number(image.cacheKey());
            m_lastCapture=sample;
        }
        return image;
    }
    bool pendingMatches(const QImage &image) {
        // Evaluate both points from this one image, preserving the original AND and thresholds.
        const auto request=m_lastCapture["request_state"].toObject();
        const auto a=point(image,request,{100,1080},true),b=point(image,request,{800,1080},false);
        const bool pixels=!image.isNull() && a["matches"].toBool() && b["matches"].toBool();
        observeContext();const auto current=state();
        m_lastCapture["assertion_state"]=current;
        const bool context=associated(m_lastCapture,m_generation,m_delta,m_depth,m_allowResident)
            && m_lastCapture["image_key"].toString()==QString::number(image.cacheKey());
        const bool good=pixels && context;
        m_anyPixelMatch=m_anyPixelMatch || pixels;
        if(m_record){
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
        const auto type=event->type();
        const bool application=type==QEvent::ApplicationDeactivate || type==QEvent::ApplicationStateChange;
        const bool target=receiver==m_canvas || receiver==m_window;
        const bool mouse=type==QEvent::MouseButtonPress || type==QEvent::MouseMove || type==QEvent::MouseButtonRelease;
        const bool relevant=mouse || type==QEvent::UngrabMouse || type==QEvent::WindowDeactivate || type==QEvent::Hide || type==QEvent::FocusOut
            || type==QEvent::Wheel || type==QEvent::TabletMove || type==QEvent::TabletPress || type==QEvent::TabletRelease
            || type==QEvent::KeyPress || type==QEvent::KeyRelease || type==QEvent::Resize || type==QEvent::Move;
        if((receiver==m_window && relevant) || application){
            ++m_inputEpoch;
            if(m_armed && event!=m_plannedEvent)m_invalid=true;
        }
        if(application || (target && relevant)){
            if(!m_record)return false;
            QJsonObject row{{"type",int(type)},{"receiver",QString::number(quintptr(receiver))},{"state",state()}};
            row["provenance"]=receiver==m_window?(event==m_plannedEvent?"planned_exact_event":"unplanned_window_ingress"):
                "item_delivery_not_a_provenance_claim";
            if(receiver==m_window && event==m_plannedEvent)row["plan_id"]=double(m_planId);
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
    void observeContext() {
        const auto s=state();QJsonObject token;
        // stateChanged also emits viewChanged without a transform change.
        // Compare actual identities rather than notification counts.
        if(m_armed && viewIdentity(s)!=m_fixedView)m_invalid=true;
        if(m_armed && !m_targetSequence && m_targetPlan && m_beginSequence
            && acceptedContext(s,m_generation,m_delta,m_depth,m_allowResident) && m_client->moveSequence()>m_beginSequence)
            m_targetSequence=m_client->moveSequence(); // Actual accepted UPDATE, never a guessed +1.
        for(const auto *key:{"owner","canvas","window","generation","revision","sequence","move","frame","frame_key",
                "frame_region","owner_metadata","undo_depth","error","active","visible","canvas_visible","dpr","zoom",
                "document_origin","size","preview_source","preview_revision"})token[key]=s[key];
        if(token!=m_context){m_context=token;++m_contextEpoch;}
    }
    QJsonObject point(const QImage &image,const QJsonObject &context,QPointF document,bool expectWhite) const {
        const auto origin=context["document_origin"].toArray();
        const auto local=(QPointF(origin[0].toDouble(),origin[1].toDouble())+document*context["zoom"].toDouble()).toPoint();
        const QPoint pixel=local*context["dpr"].toDouble();
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
            {"client_matches",m_canvas->client()==m_client && m_canvas->window()==m_window},
            {"generation",double(m_client->generation())},{"revision",double(m_client->revision())},
            {"sequence",double(m_client->moveSequence())},{"move",QJsonObject::fromVariantMap(m_client->moveObservation())},
            {"frame",QJsonObject::fromVariantMap(m_client->frameIdentity())},{"frame_key",QString::number(m_client->frame().cacheKey())},
            {"receipt",QJsonObject{{"kind",receipt.kind},{"sequence",double(receipt.sequence)},{"generation",double(receipt.generation)},{"revision",double(receipt.revision)}}},
            {"undo_depth",m_client->undoDepth()},{"error",m_client->lastError()},{"active",m_window->isActive()},
            {"visible",m_window->isVisible()},{"canvas_visible",m_canvas->isVisible()},
            {"mouse_grabber",QString::number(quintptr(m_window->mouseGrabberItem()))},
            {"gpu_display",m_canvas->gpuDisplay()},{"dpr",m_window->devicePixelRatio()},
            {"zoom",m_canvas->zoom()},{"document_origin",QJsonArray{m_canvas->documentRect().x(),m_canvas->documentRect().y()}},
            {"size",QJsonArray{m_canvas->width(),m_canvas->height()}},{"scenario_invalid",m_invalid},
            {"input_epoch",double(m_inputEpoch)},{"context_epoch",double(m_contextEpoch)},{"target_plan",double(m_targetPlan)},
            {"begin_sequence",double(m_beginSequence)},{"target_sequence",double(m_targetSequence)},
            {"prepare_request",double(m_client->movePreparationRequest())},{"prepared_source",assets?double(assets->sourceId):0},
            {"prepare_uploads",assets?double(assets->uploads->load()):0},
            {"preview_source",preview?double(preview->sourceId):0},{"preview_revision",preview?double(preview->revision):0},
            {"gui_path",preview?"resident":"normal_or_interactive_frame (render trace required)"}};
        const auto rect=[](QRectF r){return QJsonArray{r.x(),r.y(),r.width(),r.height()};};
        result["frame_region"]=rect(m_client->frameRegion(0));
        const auto receiptRender=m_presentation->snapshot();const auto &p=receiptRender.identity;
        result["render"]=QJsonObject{{"epoch",double(receiptRender.epoch)},{"thread",QString::number(receiptRender.thread)},
            {"valid",p.valid},{"canvas",QString::number(p.canvas)},{"window",QString::number(p.window)},
            {"generation",double(p.generation)},{"source",double(p.source)},{"revision",double(p.revision)},
            {"image_key",QString::number(p.imageKey)},{"region",rect(p.region)},
            {"document_origin",QJsonArray{p.document.x(),p.document.y()}},{"zoom",p.zoom},
            {"size",QJsonArray{p.size.width(),p.size.height()}},{"resident",p.resident}};
        for(const auto &row:m_client->layers())if(row.toMap().value("id").toULongLong()==m_owner)result["owner_metadata"]=QJsonObject::fromVariantMap(row.toMap());
        return result;
    }
    void append(QJsonArray &array,const QJsonObject &row){if(array.size()<256)array.append(row);else ++m_dropped;}
    QPointer<CanvasItem> m_canvas;QPointer<PaintCoreClient> m_client;QPointer<QQuickWindow> m_window;
    std::shared_ptr<CanvasPresentationObservation> m_presentation;
    const QEvent *m_plannedEvent=nullptr;
    quint64 m_planId=0,m_targetPlan=0,m_inputEpoch=0,m_contextEpoch=0,m_beginSequence=0,m_targetSequence=0;
    bool m_armed=false,m_invalid=false,m_grabFailed=false;QJsonObject m_context,m_fixedView;
    QSharedPointer<QQuickItemGrabResult> m_failedGrab;
    quint64 m_owner=0,m_request=0,m_generation=0;QPoint m_delta;int m_depth=0;
    QString m_directory,m_phase="initial";bool m_enabled=false,m_record=false,m_allowResident=false,m_anyPixelMatch=false,m_assertionPassed=false;
    int m_dropped=0;QJsonArray m_captures,m_states,m_inputs,m_plans;QJsonObject m_lastCapture;
    QImage m_firstFailure,m_lastFailure,m_success;
};
