#include "PaintCoreClient.h"
#include "paint_api.h"
#include <QMutex>
#include <QMutexLocker>
#include <QTimer>
#include <QFileInfo>
#include <QSettings>
#include <QDir>
#include <QBuffer>
#include <QElapsedTimer>
#include <QHash>
#include <QDrag>
#include <QMimeData>
#include <algorithm>
#include <atomic>
#include <cmath>

Q_DECLARE_METATYPE(PaintSessionInfo)
Q_DECLARE_METATYPE(PaintFileJobInfo)
namespace {
enum Operation { New = 1, Begin, Move, End, Cancel, Undo, Redo, Add, Remove, Select, Properties, NewWhite, AddMask };
template<class T> T dto() { T value{}; value.struct_size = sizeof(T); return value; }
float linear(float v) { return v <= .04045f ? v / 12.92f : std::pow((v + .055f) / 1.055f, 2.4f); }
QString immediateError(PaintStatus status) {
    uint64_t size = 0; paint_error_message(nullptr, 0, &size);
    QByteArray message(static_cast<qsizetype>(size), '\0');
    paint_error_message(reinterpret_cast<uint8_t *>(message.data()), size, &size);
    return QStringLiteral("核心错误 %1：%2").arg(status).arg(QString::fromUtf8(message));
}
PaintPoint point(const InputSample &s) {
    auto p = dto<PaintPoint>();
    p.x = s.position.x(); p.y = s.position.y(); p.pressure = s.pressure;
    p.tilt_x = s.tiltX; p.tilt_y = s.tiltY; p.rotation = s.rotation;
    p.tangential_pressure = s.tangentialPressure; p.buttons = s.buttons;
    p.capabilities = s.capabilities; p.tool = s.tool; p.timestamp_ns = s.timestamp;
    return p;
}
}
// GUI calls through this bridge only enqueue small commands or view requests.
// The worker owns the handles and clears the bridge before joining Rust workers.
struct BackendConnection {
    QMutex mutex;
    PaintCore *core = nullptr;
    PaintSession *session = nullptr;
    std::atomic_bool framePending[2]{};
    std::atomic_bool metadataPending{false};
    std::atomic<quint64> fileJob{0};
};
class BackendWorker final : public QObject {
    Q_OBJECT
public:
    explicit BackendWorker(std::shared_ptr<BackendConnection> connection, QString settingsFile) : m_settingsFile(std::move(settingsFile)), m_connection(std::move(connection)) {
        m_timer = new QTimer(this); m_timer->setInterval(16);
        connect(m_timer, &QTimer::timeout, this, &BackendWorker::poll);
    }
    void boot() {
        auto version = dto<PaintVersion>();
        if (!check(paint_core_version(&version))) return;
        if (version.major != PAINT_ABI_MAJOR || version.minor < 7) {
            emit failure(QStringLiteral("剪贴蒙版需要核心 ABI 1.7 或兼容后续版本")); return;
        }
        auto settings = preferences();
        m_saved = QVariantMap{{"directory", settings->value("storage/directory", "")},
            {"memoryMiB", settings->value("storage/memoryMiB", 256)},
            {"scratchGiB", settings->value("storage/scratchGiB", 8)},
            {"reserveMiB", settings->value("storage/reserveMiB", 512)}};
        QByteArray path; const auto options = storageOptions(m_saved, path);
        const auto created = paint_core_create_with_storage(&options, &m_core);
        if (created != PAINT_OK) {
            const auto message = immediateError(created);
            emit storageState(m_saved, {}, {}, message); emit storageDone(false);
            emit failure(message); return;
        }
        m_active = m_saved;
        inspectStorage();
        auto caps = dto<PaintCapabilities>();
        if (!check(paint_core_capabilities(m_core, &caps))) return;
        constexpr uint64_t required = PAINT_FEATURE_ASYNC_SESSION | PAINT_FEATURE_CPU_VIEWPORT | PAINT_FEATURE_FILE_IO | PAINT_FEATURE_STORAGE_SETTINGS | PAINT_FEATURE_LAYER_APPEARANCE | PAINT_FEATURE_LAYER_GROUPS | PAINT_FEATURE_MASKS | PAINT_FEATURE_CLIPPING;
        if ((caps.features & required) != required) { emit failure(QStringLiteral("当前核心缺少异步视口能力")); return; }
        auto desc = dto<PaintDocumentDesc>(); desc.width = 960; desc.height = 640;
        desc.working_space = PAINT_WORKING_LINEAR_SRGB; desc.pixel_format = PAINT_STORAGE_RGBA32F_PREMULTIPLIED;
        if (!check(paint_session_create(m_core, &desc, &m_session))) return;
        { QMutexLocker lock(&m_connection->mutex); m_connection->core = m_core; m_connection->session = m_session; }
        auto initial=dto<PaintCommand>();initial.kind=PAINT_COMMAND_NEW_WHITE_DOCUMENT;initial.width=960;initial.height=640;
        if(!check(paint_session_submit(m_core,m_session,&initial,&m_bootSequence))) return;
        m_previewClock.start(); poll(); m_timer->start();
    }
    // Disk queries and QSettings synchronization stay on the backend thread.
    void inspectStorage() { evaluateStorage(m_saved, false); }
    void saveStorage(const QVariantMap &values) { evaluateStorage(values, true); }
    void referencePreview(quint64 id, bool add) {
        if(add) {
            if(m_previewReferences.contains(id)) ++m_previewReferences[id];
            else if(m_previewTargets.size()<4096) { m_previewReferences[id]=1; m_previewTargets.append(id); }
        } else if(m_previewReferences.contains(id)) {
            if(--m_previewReferences[id]<=0) { m_previewReferences.remove(id); m_previewTargets.removeAll(id); }
        }
        m_previewIndex=0;
    }
    void stop() {
        if (m_stopped) return;
        m_stopped = true; m_timer->stop();
        { QMutexLocker lock(&m_connection->mutex); m_connection->session = nullptr; m_connection->core = nullptr; }
        // Release the GUI lock before cancellation and thread joins.
        if (m_session) check(paint_session_destroy(m_core, &m_session));
        if (m_core) check(paint_core_destroy(&m_core));
        emit finished();
    }
signals:
    void storageState(QVariantMap saved, QVariantMap active, QVariantMap info, QString message);
    void storageDone(bool success);
    void metadata(PaintSessionInfo info, QVariantList layers);
    void pixels(int view, QImage image, QRectF region, quint64 generation, quint64 revision, quint64 request);
    void failure(QString message);
    void completed(bool workerThread);
    void finished();
    void fileResult(PaintFileJobInfo info, QString message);
private:
    std::unique_ptr<QSettings> preferences() const {
        if (!m_settingsFile.isEmpty()) return std::make_unique<QSettings>(m_settingsFile, QSettings::IniFormat);
        return std::make_unique<QSettings>(QSettings::IniFormat, QSettings::UserScope, "DrawVerse", "DrawVerse");
    }
    static PaintStorageOptions storageOptions(const QVariantMap &values, QByteArray &path) {
        path = values.value("directory").toString().toUtf8();
        auto options = dto<PaintStorageOptions>(); options.path = reinterpret_cast<const uint8_t *>(path.constData());
        options.path_length = static_cast<uint64_t>(path.size());
        // Reject negative/overflowing values before multiplying; Rust validates the exact byte bounds.
        const auto memory = values.value("memoryMiB").toLongLong();
        const auto scratch = values.value("scratchGiB").toLongLong();
        const auto reserve = values.value("reserveMiB").toLongLong();
        if (memory < 1 || memory > 1024 || scratch < 1 || scratch > 64 || reserve < 0 || reserve > 1048576) return options;
        options.resident_bytes = static_cast<uint64_t>(memory) * 1024 * 1024;
        options.scratch_bytes = static_cast<uint64_t>(scratch) * 1024 * 1024 * 1024;
        options.min_free_bytes = static_cast<uint64_t>(reserve) * 1024 * 1024;
        return options;
    }
    void evaluateStorage(const QVariantMap &values, bool save) {
        if (m_stopped) { emit storageDone(false); return; }
        QByteArray path; const auto options = storageOptions(values, path);
        auto info = dto<PaintStorageInfo>();
        const auto status = paint_storage_inspect(&options, &info);
        if (status != PAINT_OK) {
            emit storageState(m_saved, m_active, {}, immediateError(status)); emit storageDone(false); return;
        }
        if (save) {
            auto settings = preferences();
            for (auto it=values.cbegin(); it!=values.cend(); ++it) settings->setValue("storage/"+it.key(),it.value());
            settings->sync();
            if (settings->status()!=QSettings::NoError) {
                emit storageState(m_saved,m_active,{},QStringLiteral("无法保存设置，请检查配置目录权限和可用空间"));
                emit storageDone(false); return;
            }
            m_saved = values;
        }
        const QVariantMap report{{"availableGiB",static_cast<double>(info.available_bytes)/(1024.*1024.*1024.)},
            {"removedRuns",info.removed_runs},{"removedMiB",static_cast<double>(info.removed_bytes)/(1024.*1024.)},
            {"systemDirectory",QDir::tempPath()}};
        emit storageState(m_saved,m_active,report,save ? QStringLiteral("设置已保存，重启 DrawVerse 后生效") : QString());
        emit storageDone(true);
    }
    bool check(PaintStatus status) {
        if (status == PAINT_OK) return true;
        emit failure(immediateError(status)); return false;
    }
    bool publish(const PaintSessionInfo &info) {
        if (m_connection->metadataPending.load()) return false;
        QVariantList layers;
        if (info.document_generation != m_previewGeneration) { m_thumbnails.clear(); m_thumbnailRevisions.clear(); m_previewGeneration = info.document_generation; }
        // Exact publication IDs prevent mixing layers from different edits.
        for (uint32_t i = 0; i < info.layer_count; ++i) {
            auto layer = dto<PaintLayerInfo>();
            auto status = paint_session_layer_info(m_core, m_session, info.publication, i, &layer);
            if (status == PAINT_BUSY) return false;
            if (!check(status)) return false;
            QByteArray name(static_cast<qsizetype>(layer.name_length), '\0'); uint64_t required = 0;
            status = paint_session_layer_name(m_core, m_session, info.publication, layer.layer_id,
                reinterpret_cast<uint8_t *>(name.data()), layer.name_length, &required);
            if (status == PAINT_BUSY) return false;
            if (!check(status)) return false;
            auto appearance = dto<PaintLayerAppearance>();
            status = paint_session_layer_appearance(m_core,m_session,info.publication,layer.layer_id,&appearance);
            if (status == PAINT_BUSY) return false;
            if (!check(status)) return false;
            auto hierarchy=dto<PaintLayerHierarchy>();
            status=paint_session_layer_hierarchy(m_core,m_session,info.publication,layer.layer_id,&hierarchy);
            if(status==PAINT_BUSY) return false;
            if(!check(status)) return false;
            auto clipping=dto<PaintLayerClipping>();
            status=paint_session_layer_clipping(m_core,m_session,info.publication,layer.layer_id,&clipping);
            if(status==PAINT_BUSY) return false;
            if(!check(status)) return false;
            layers.prepend(QVariantMap{{"id", QVariant::fromValue<qulonglong>(layer.layer_id)},
                {"name", QString::fromUtf8(name)}, {"visible", layer.visible != 0}, {"opacity", layer.opacity},
                {"fill",appearance.fill},{"blendMode",appearance.blend_mode},{"locks",appearance.locks},
                {"mask",hierarchy.kind==PAINT_LAYER_MASK},{"group",hierarchy.kind==PAINT_LAYER_GROUP},{"parent",QVariant::fromValue<qulonglong>(hierarchy.parent_id)},{"depth",hierarchy.depth},{"effectiveLocks",hierarchy.effective_locks},
                {"offsetX",appearance.offset_x},{"offsetY",appearance.offset_y},{"dissolveSeed",appearance.dissolve_seed},
                {"thumbnail",m_thumbnails.value(layer.layer_id)}});
            auto entry=layers.first().toMap();entry.insert("clipped",clipping.enabled!=0);
            entry.insert("clipBase",QVariant::fromValue<qulonglong>(clipping.base_layer_id));layers[0]=entry;
        }
        m_connection->metadataPending.store(true);
        emit metadata(info, layers); m_publication = info.publication;
        return true;
    }
    void poll() {
        if (!m_session || m_stopped) return;
        auto info = dto<PaintSessionInfo>();
        if (!check(paint_session_info(m_core, m_session, &info))) return;
        if(info.completed_sequence<m_bootSequence) return;
        if ((info.publication != m_publication || info.flags != m_flags) && !publish(info)) return;
        m_flags = info.flags;
        pollPreviews(info);
        if (info.completed_sequence != m_completed) {
            m_completed = info.completed_sequence; emit completed(QThread::currentThread() == thread());
        }
        if (info.last_error_status != PAINT_OK &&
            (info.last_error_sequence != m_errorSequence || info.last_error_status != m_errorStatus)) {
            uint64_t size = 0;
            auto status = paint_session_error_message(m_core, m_session, info.last_error_sequence, nullptr, 0, &size);
            if (status == PAINT_BUFFER_TOO_SMALL || status == PAINT_OK) {
                QByteArray message(static_cast<qsizetype>(size), '\0');
                if (paint_session_error_message(m_core, m_session, info.last_error_sequence,
                    reinterpret_cast<uint8_t *>(message.data()), size, &size) == PAINT_OK) {
                    emit failure(QStringLiteral("任务错误 %1：%2").arg(info.last_error_status).arg(QString::fromUtf8(message)));
                    m_errorSequence = info.last_error_sequence;
                    m_errorStatus = info.last_error_status;
                }
            }
        }
        if (info.flags & PAINT_SESSION_FAILED) { m_timer->stop(); return; }
        const auto fileJob = m_connection->fileJob.load();
        if (fileJob && fileJob != m_deliveredFile) {
            auto file = dto<PaintFileJobInfo>();
            if (check(paint_session_file_info(m_core, m_session, fileJob, &file)) && file.state >= PAINT_FILE_SUCCEEDED) {
                // The actor publishes document metadata immediately after the file result.
                // Deliver metadata first so Save/Discard decisions see the current document.
                if (file.state == PAINT_FILE_SUCCEEDED &&
                    ((file.kind == PAINT_FILE_OPEN && info.document_generation != file.result_generation) ||
                     (file.kind == PAINT_FILE_SAVE && file.format == PAINT_FILE_OPENRASTER &&
                      info.document_generation == file.source_generation && info.revision == file.source_revision &&
                      (info.flags & PAINT_SESSION_MODIFIED)))) return;
                uint64_t size = 0;
                auto status = paint_session_file_message(m_core, m_session, fileJob, nullptr, 0, &size);
                if (status == PAINT_OK || status == PAINT_BUFFER_TOO_SMALL) {
                    QByteArray message(static_cast<qsizetype>(size), '\0');
                    if (check(paint_session_file_message(m_core, m_session, fileJob,
                        reinterpret_cast<uint8_t *>(message.data()), size, &size))) {
                        m_deliveredFile = fileJob; emit fileResult(file, QString::fromUtf8(message));
                    }
                }
            }
        }
        for (uint32_t view = 0; view < 2; ++view) {
            // At most one queued QImage per view when the GUI is busy.
            if (m_connection->framePending[view].load()) continue;
            auto frame = dto<PaintFrameInfo>();
            auto status = paint_session_frame_info(m_core, m_session, view, &frame);
            if (status == PAINT_BUSY) continue;
            if (!check(status)) return;
            if (frame.frame_id == m_frames[view]) continue;
            QImage image(static_cast<int>(frame.pixel_width), static_cast<int>(frame.pixel_height), QImage::Format_RGBA8888_Premultiplied);
            if (image.isNull()) { emit failure(QStringLiteral("无法分配视口显示缓存")); return; }
            auto tile = dto<PaintTile>(); tile.format = PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED;
            tile.data = image.bits(); tile.capacity = static_cast<uint64_t>(image.sizeInBytes());
            tile.stride = static_cast<uint64_t>(image.bytesPerLine());
            status = paint_session_read_frame(m_core, m_session, view, frame.request_id, frame.frame_id, &tile);
            if (status == PAINT_BUSY) continue;
            if (!check(status)) return;
            m_frames[view] = frame.frame_id;
            m_connection->framePending[view].store(true);
            emit pixels(static_cast<int>(view), image, QRectF(frame.x, frame.y, frame.width, frame.height),
                frame.document_generation, frame.revision, frame.request_id);
        }
    }
    // One background preview at a time, only after a quiet document interval.
    // Slot 2 is bounded by the core and is cancelled whenever painting resumes.
    void disablePreview(const PaintSessionInfo &info) {
        auto view=dto<PaintViewport>(); view.view_id=2; view.document_generation=info.document_generation;
        uint64_t ignored=0; paint_session_set_viewport(m_core,m_session,&view,&ignored); m_previewRequest=0;
    }
    void pollPreviews(const PaintSessionInfo &info) {
        const auto now=m_previewClock.elapsed();
        if(info.document_generation!=m_seenGeneration || info.revision!=m_seenRevision || info.stroke_active) {
            m_seenGeneration=info.document_generation; m_seenRevision=info.revision; m_quietSince=now;
            m_previewIndex=0;
            if(m_previewRequest) disablePreview(info);
            return;
        }
        if(now-m_quietSince<200) return;
        if(m_previewRequest) {
            auto frame=dto<PaintFrameInfo>();
            const auto status=paint_session_frame_info(m_core,m_session,2,&frame);
            if(status==PAINT_BUSY) return;
            if(status!=PAINT_OK) { disablePreview(info); ++m_previewIndex; return; }
            if(frame.request_id!=m_previewRequest || frame.document_generation!=info.document_generation || frame.revision!=info.revision) return;
            QImage image(static_cast<int>(frame.pixel_width),static_cast<int>(frame.pixel_height),QImage::Format_RGBA8888_Premultiplied);
            if(image.isNull()) { disablePreview(info); ++m_previewIndex; return; }
            auto tile=dto<PaintTile>(); tile.format=PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED;
            tile.data=image.bits(); tile.capacity=static_cast<uint64_t>(image.sizeInBytes()); tile.stride=static_cast<uint64_t>(image.bytesPerLine());
            if(paint_session_read_frame(m_core,m_session,2,frame.request_id,frame.frame_id,&tile)!=PAINT_OK) return;
            QByteArray png; QBuffer buffer(&png); buffer.open(QIODevice::WriteOnly);
            if(image.save(&buffer,"PNG")) {
                m_thumbnails[m_previewLayer]="data:image/png;base64,"+QString::fromLatin1(png.toBase64());
                m_thumbnailRevisions[m_previewLayer]=info.revision;
                while(m_thumbnails.size()>128) {
                    auto victim=m_thumbnails.begin();
                    while(victim!=m_thumbnails.end() && m_previewReferences.contains(victim.key())) ++victim;
                    if(victim==m_thumbnails.end()) break;
                    m_thumbnailRevisions.remove(victim.key()); m_thumbnails.erase(victim);
                }
                m_publication=0; // publish changed previews, never a drawing-time model refresh
            }
            disablePreview(info); ++m_previewIndex;
        }
        while(m_previewIndex<m_previewTargets.size() && m_thumbnailRevisions.contains(m_previewTargets[m_previewIndex]) &&
              m_thumbnailRevisions.value(m_previewTargets[m_previewIndex])==info.revision) ++m_previewIndex;
        if(m_previewIndex>=m_previewTargets.size()) return;
        const auto layerId=m_previewTargets[m_previewIndex];
        auto appearance=dto<PaintLayerAppearance>();
        if(paint_session_layer_appearance(m_core,m_session,info.publication,layerId,&appearance)!=PAINT_OK) { ++m_previewIndex; return; }
        auto view=dto<PaintViewport>(); view.view_id=2; view.enabled=1; view.document_generation=info.document_generation;
        view.width=info.width; view.height=info.height;
        const double scale=64./std::max(info.width,info.height);
        view.pixel_width=std::max(1u,static_cast<uint32_t>(std::round(info.width*scale)));
        view.pixel_height=std::max(1u,static_cast<uint32_t>(std::round(info.height*scale)));
        if(paint_session_set_layer_preview(m_core,m_session,layerId,&view,&m_previewRequest)==PAINT_OK) m_previewLayer=layerId;
    }
    QHash<quint64,QString> m_thumbnails;
    QHash<quint64,quint64> m_thumbnailRevisions;
    QHash<quint64,int> m_previewReferences;
    QList<quint64> m_previewTargets;
    QElapsedTimer m_previewClock;
    qint64 m_quietSince=0;
    quint64 m_previewGeneration=0,m_seenGeneration=0,m_seenRevision=0,m_previewRequest=0,m_previewLayer=0;
    qsizetype m_previewIndex=0;
    uint64_t m_bootSequence=0;
    QString m_settingsFile;
    QVariantMap m_saved, m_active;
    std::shared_ptr<BackendConnection> m_connection;
    QTimer *m_timer;
    PaintCore *m_core = nullptr;
    PaintSession *m_session = nullptr;
    quint64 m_publication = 0, m_completed = 0, m_errorSequence = 0, m_frames[2]{};
    quint64 m_deliveredFile = 0;
    uint32_t m_flags = 0;
    PaintStatus m_errorStatus = PAINT_OK;
    bool m_stopped = false;
};

PaintCoreClient::PaintCoreClient(QObject *parent, const QString &settingsFile) : QObject(parent), m_connection(std::make_shared<BackendConnection>()) {
    qRegisterMetaType<PaintSessionInfo>();
    qRegisterMetaType<PaintFileJobInfo>();
    m_worker = new BackendWorker(m_connection, settingsFile); m_worker->moveToThread(&m_thread);
    connect(m_worker,&BackendWorker::storageState,this,[this](QVariantMap saved,QVariantMap active,QVariantMap info,QString message) {
        m_storageSettings=std::move(saved); m_activeStorageSettings=std::move(active);
        m_storageInfo=std::move(info); m_storageMessage=std::move(message); emit storageChanged();
    });
    connect(m_worker,&BackendWorker::storageDone,this,[this](bool success) {
        m_storageBusy=false; emit storageChanged(); emit storageFinished(success);
    });
    connect(&m_thread, &QThread::started, m_worker, &BackendWorker::boot);
    connect(m_worker, &BackendWorker::metadata, this, [this](PaintSessionInfo info, QVariantList layers) {
        m_connection->metadataPending.store(false);
        const bool propertiesChanged = info.document_generation != m_generation
            || static_cast<int>(info.width) != m_width || static_cast<int>(info.height) != m_height
            || static_cast<int>(info.undo_depth) != m_undo || static_cast<int>(info.redo_depth) != m_redo
            || info.active_layer_id != m_active;
        const bool wasReady = m_ready, wasModified = modified(), wasLayerBusy=layerEditBusy();
        const bool changedLayers = m_layers != layers;
        const bool changedGeneration = info.document_generation != m_generation;
        if(changedGeneration) m_nextDefaultLayer=1;
        if(changedGeneration && !m_collapsedGroups.isEmpty()) {m_collapsedGroups.clear(); emit groupExpansionChanged();}
        m_generation = info.document_generation;
        m_revision = info.revision; m_width = static_cast<int>(info.width); m_height = static_cast<int>(info.height);
        m_undo = static_cast<int>(info.undo_depth); m_redo = static_cast<int>(info.redo_depth);
        if (info.completed_sequence >= m_pendingMutation) m_pendingMutation = 0;
        if (info.completed_sequence >= m_pendingLayer) m_pendingLayer=0;
        m_active = info.active_layer_id;
        if (changedLayers) {
            m_layers = std::move(layers);
            if(!m_collapsedGroups.isEmpty()) {
                QSet<quint64> groups;for(const auto &value:m_layers) {const auto layer=value.toMap();if(layer.value("group").toBool()) groups.insert(layer.value("id").toULongLong());}
                const auto previous=m_collapsedGroups;m_collapsedGroups.intersect(groups);
                if(previous!=m_collapsedGroups) emit groupExpansionChanged();
            }
        }
        m_modified = (info.flags & PAINT_SESSION_MODIFIED) != 0 || m_pendingMutation != 0;
        if (info.completed_sequence >= m_pendingNew) m_pendingNew = 0;
        m_backendFailed = (info.flags & PAINT_SESSION_FAILED) != 0;
        m_ready = !m_closing && !m_pendingNew && !m_fileOpening && !m_backendFailed;
        if (changedGeneration) {
            for (int i = 0; i < 2; ++i) {
                m_frames[i] = {}; m_regions[i] = {}; m_frameRevisions[i] = 0;
                m_requests[i] = 0; m_requestedRegions[i] = {}; m_requestedPixels[i] = {};
            }
            emit frameChanged();
        }
        // Pixel revisions arrive through frameChanged. Keep QML's layer delegates
        // alive throughout a stroke; notify their model only for metadata changes.
        if (changedLayers) emit layersChanged();
        if (propertiesChanged || wasReady != m_ready || wasModified != modified() || wasLayerBusy != layerEditBusy()) emit stateChanged();
    });
    connect(m_worker, &BackendWorker::pixels, this, [this](int view, const QImage &image, QRectF region, quint64 generation, quint64 revision, quint64 request) {
        m_connection->framePending[view].store(false);
        if (m_closing || m_pendingNew || generation != m_generation ||
            (m_requests[view] && request != m_requests[view]) || revision < m_frameRevisions[view]) return;
        m_frames[view] = image; m_regions[view] = region; m_frameRevisions[view] = revision; emit frameChanged();
    });
    connect(m_worker, &BackendWorker::failure, this, &PaintCoreClient::showError);
    connect(m_worker, &BackendWorker::fileResult, this, [this](PaintFileJobInfo info, const QString &message) {
        if (info.job_id != m_fileId || m_closing) return;
        m_fileBusy = false; m_fileOpening = false;
        const bool success = info.state == PAINT_FILE_SUCCEEDED;
        if (success && info.result_generation == m_generation) {
            if (info.kind == PAINT_FILE_OPEN) {
                m_sourceUrl = m_fileUrl;
                m_documentUrl = info.format == PAINT_FILE_OPENRASTER ? m_fileUrl : QUrl{};
            } else if (info.format == PAINT_FILE_OPENRASTER) { m_documentUrl = m_fileUrl; m_sourceUrl = m_fileUrl; }
        }
        if (!success) showError(message);
        m_ready = !m_closing && !m_pendingNew && !m_backendFailed;
        emit stateChanged(); emit fileFinished(success);
    });
    connect(m_worker, &BackendWorker::completed, this, &PaintCoreClient::commandCompleted);
    connect(m_worker, &BackendWorker::finished, &m_thread, &QThread::quit, Qt::DirectConnection);
    connect(&m_thread, &QThread::finished, m_worker, &QObject::deleteLater);
    connect(&m_thread, &QThread::finished, this, &PaintCoreClient::stopped);
    m_thread.start();
}
PaintCoreClient::~PaintCoreClient() {
    // Window close uses asynchronous shutdown; this is a lifetime safety net.
    if (m_thread.isRunning()) {
        QMetaObject::invokeMethod(m_worker, &BackendWorker::stop, Qt::BlockingQueuedConnection);
        m_thread.quit(); m_thread.wait();
    }
}
void PaintCoreClient::shutdown() {
    if (m_closing) return;
    m_closing = true; m_ready = false; m_drawing = false;
    emit stateChanged(); QMetaObject::invokeMethod(m_worker, &BackendWorker::stop, Qt::QueuedConnection);
}
bool PaintCoreClient::submit(int type, const InputSample &sample, quint64 id, const QString &text, qreal amount, bool flag) {
    if (m_closing || !m_ready) return false;
    auto command = dto<PaintCommand>(); command.kind = static_cast<uint32_t>(type); command.layer_id = id;
    const auto utf8 = text.toUtf8(); command.text = reinterpret_cast<const uint8_t *>(utf8.constData());
    command.text_length = static_cast<uint64_t>(utf8.size());
    if (type == New || type == NewWhite) { command.width = static_cast<uint32_t>(id); command.height = static_cast<uint32_t>(amount); }
    if (type == Properties) { command.visible = flag ? 1 : 0; command.opacity = static_cast<float>(amount); }
    if (type == Move || type == Begin) command.point = point(sample);
    if (type == Begin) {
        command.stroke = dto<PaintStrokeDesc>(); // Layer 0 resolves active layer in execution order.
        command.stroke.mode = m_eraser || sample.tool == PAINT_TOOL_ERASER ? PAINT_MODE_ERASE : PAINT_MODE_PAINT;
        command.stroke.radius = static_cast<float>(m_radius); command.stroke.opacity = static_cast<float>(m_opacity);
        command.stroke.spacing = .15f;
        command.stroke.linear_rgba[0] = linear(m_color.redF()); command.stroke.linear_rgba[1] = linear(m_color.greenF());
        command.stroke.linear_rgba[2] = linear(m_color.blueF()); command.stroke.linear_rgba[3] = m_color.alphaF();
    }
    uint64_t sequence = 0; PaintStatus status; QString error;
    {
        QMutexLocker lock(&m_connection->mutex);
        if (!m_connection->session) return false;
        status = paint_session_submit(m_connection->core, m_connection->session, &command, &sequence);
        if (status != PAINT_OK) error = immediateError(status);
    }
    if (status != PAINT_OK) {
        if ((type == Move || type == End) && (status == PAINT_LIMIT_EXCEEDED || status == PAINT_CANCELLED)) m_dropStroke = true;
        showError(error); return false;
    }
    if (type == New || type == NewWhite) { m_pendingNew = sequence; m_ready = false; emit stateChanged(); }
    if(type==Select) { m_pendingLayer=sequence; emit stateChanged(); }
    if (type == End || type == Undo || type == Redo || type == Add || type == Remove || type == Properties || type == AddMask) {
        // Accepted edits must protect against an immediate New/Open/Close before
        // the next metadata poll. Failed/no-op commands are reconciled by the actor.
        m_pendingMutation = sequence;
        if(type!=End) m_pendingLayer=sequence;
        m_modified = true; emit stateChanged();
    }
    return true;
}
void PaintCoreClient::requestViewport(int view, QRectF region, QSize pixels, bool enabled) {
    if (!m_ready || m_closing || view < 0 || view > 1) return;
    if (enabled) {
        if (!std::isfinite(region.x()) || !std::isfinite(region.y()) || !std::isfinite(region.width()) ||
            !std::isfinite(region.height()) || region.isEmpty() || pixels.isEmpty()) return;
        const double ratio = std::min({1., 4096. / pixels.width(), 4096. / pixels.height(),
            std::sqrt(4194304. / (static_cast<double>(pixels.width()) * pixels.height()))});
        pixels = {std::max(1, static_cast<int>(pixels.width() * ratio)), std::max(1, static_cast<int>(pixels.height() * ratio))};
    }
    if (m_requests[view] && m_viewEnabled[view] == enabled && m_requestedRegions[view] == region && m_requestedPixels[view] == pixels) return;
    auto viewport = dto<PaintViewport>(); viewport.view_id = static_cast<uint32_t>(view); viewport.enabled = enabled ? 1 : 0;
    viewport.document_generation = m_generation; viewport.x = region.x(); viewport.y = region.y();
    viewport.width = region.width(); viewport.height = region.height();
    viewport.pixel_width = static_cast<uint32_t>(pixels.width()); viewport.pixel_height = static_cast<uint32_t>(pixels.height());
    uint64_t request = 0; PaintStatus status; QString error;
    {
        QMutexLocker lock(&m_connection->mutex);
        if (!m_connection->session) return;
        status = paint_session_set_viewport(m_connection->core, m_connection->session, &viewport, &request);
        if (status != PAINT_OK && status != PAINT_BUSY) error = immediateError(status);
    }
    if (status == PAINT_OK) {
        m_requests[view] = request; m_requestedRegions[view] = region; m_requestedPixels[view] = pixels; m_viewEnabled[view] = enabled;
        if (!enabled) { m_frames[view] = {}; m_regions[view] = {}; emit frameChanged(); }
    } else if (!error.isEmpty()) showError(error);
}
void PaintCoreClient::showError(const QString &error) { m_error = error; emit errorChanged(); }
void PaintCoreClient::clearError() { m_error.clear(); emit errorChanged(); }
void PaintCoreClient::setBrushColor(const QColor &c) { if (c.isValid() && c != m_color) { m_color = c; emit brushChanged(); } }
void PaintCoreClient::setBrushRadius(qreal r) { if (std::isfinite(r)) { m_radius = std::clamp(r, .5, 256.); emit brushChanged(); } }
void PaintCoreClient::setBrushOpacity(qreal o) { if (std::isfinite(o)) { m_opacity = std::clamp(o, 0., 1.); emit brushChanged(); } }
void PaintCoreClient::setEraser(bool e) { if (e != m_eraser || m_moveTool) { m_eraser = e; m_moveTool=false; emit brushChanged(); } }
void PaintCoreClient::setMoveTool(bool enabled) { if(enabled!=m_moveTool && !m_drawing) { m_moveTool=enabled; emit brushChanged(); } }
bool PaintCoreClient::beginStroke(const InputSample &s) {
    if (!m_ready || m_drawing || layerEditBusy()) return false;
    for(const auto &value:m_layers) { const auto layer=value.toMap(); if(layer.value("id").toULongLong()!=m_active) continue;
        if(layer.value("group").toBool()) {showError(QStringLiteral("请选中组内像素图层绘画，或先新建图层"));return false;}
        if(layer.value("effectiveLocks").toUInt() & PAINT_LOCK_ALL) {showError(QStringLiteral("当前图层或所属组已完全锁定，请先解锁"));return false;}
    }
    m_dropStroke = false; if (!submit(Begin, s)) return false;
    m_drawing = true; emit stateChanged(); return true;
}
void PaintCoreClient::strokeTo(const InputSample &s) { if (m_drawing && !m_dropStroke) submit(Move, s); }
void PaintCoreClient::endStroke() {
    if (!m_drawing) return;
    if (!m_dropStroke) submit(End);
    m_drawing = false; emit stateChanged();
}
void PaintCoreClient::cancelStroke() {
    if (!m_drawing) return;
    if (!m_dropStroke) submit(Cancel);
    m_dropStroke = true; m_drawing = false; emit stateChanged();
}
void PaintCoreClient::undo() { if (m_ready && !m_drawing) submit(Undo); }
void PaintCoreClient::redo() { if (m_ready && !m_drawing) submit(Redo); }
void PaintCoreClient::addLayer(const QString &name) {
    if (m_ready && !m_drawing && !layerEditBusy() && submit(Add, {}, 0, name) && m_collapsedGroups.remove(m_active)) emit groupExpansionChanged();
}
void PaintCoreClient::removeLayer(quint64 id) { if (m_ready && !m_drawing) submit(Remove, {}, id); }
void PaintCoreClient::selectLayer(quint64 id) { if (m_ready && !m_drawing) submit(Select, {}, id); }
void PaintCoreClient::setLayerProperties(quint64 id, bool v, qreal opacity) {
    if (m_ready && !m_drawing && !layerEditBusy() && std::isfinite(opacity)) submit(Properties, {}, id, {}, std::clamp(opacity, 0., 1.), v);
}
bool PaintCoreClient::setAppearance(quint64 id,const QString &field,const QVariant &value) {
    if(!m_ready || m_drawing || layerEditBusy()) return false;
    QVariantMap layer;
    for(const auto &entry:m_layers) if(entry.toMap().value("id").toULongLong()==id) { layer=entry.toMap(); break; }
    if(layer.isEmpty()) return false;
    layer[field]=value;
    auto options=dto<PaintLayerAppearance>(); options.locks=layer.value("locks").toUInt();
    options.blend_mode=layer.value("blendMode").toUInt(); options.fill=layer.value("fill").toFloat();
    options.offset_x=layer.value("offsetX").toInt(); options.offset_y=layer.value("offsetY").toInt(); options.dissolve_seed=layer.value("dissolveSeed").toUInt();
    uint64_t sequence=0; PaintStatus status; QString error;
    { QMutexLocker lock(&m_connection->mutex); if(!m_connection->session) return false;
      status=paint_session_set_layer_appearance(m_connection->core,m_connection->session,id,&options,&sequence);
      if(status!=PAINT_OK) error=immediateError(status);
    }
    if(status!=PAINT_OK) { showError(error); return false; }
    m_pendingLayer=m_pendingMutation=sequence; m_modified=true; emit stateChanged(); return true;
}
bool PaintCoreClient::setLayerFill(quint64 id,qreal fill) { return std::isfinite(fill) && fill>=0 && fill<=1 && setAppearance(id,"fill",fill); }
bool PaintCoreClient::setLayerBlend(quint64 id,int blend) { return blend>=0 && blend<=26 && setAppearance(id,"blendMode",blend); }
bool PaintCoreClient::setLayerLocks(quint64 id,int locks) { return locks>=0 && locks<=7 && setAppearance(id,"locks",locks); }
void PaintCoreClient::requestLayerPreview(quint64 id) {
    if(!m_closing) QMetaObject::invokeMethod(m_worker,[worker=m_worker,id]{worker->referencePreview(id,true);},Qt::QueuedConnection);
}
void PaintCoreClient::releaseLayerPreview(quint64 id) {
    if(!m_closing) QMetaObject::invokeMethod(m_worker,[worker=m_worker,id]{worker->referencePreview(id,false);},Qt::QueuedConnection);
}
bool PaintCoreClient::moveLayer(quint64 id,int dx,int dy) {
    if(!m_ready || m_drawing || layerEditBusy() || (!dx && !dy)) return false;
    uint64_t sequence=0; PaintStatus status; QString error;
    { QMutexLocker lock(&m_connection->mutex); if(!m_connection->session) return false;
      status=paint_session_move_layer(m_connection->core,m_connection->session,id,dx,dy,&sequence);
      if(status!=PAINT_OK) error=immediateError(status);
    }
    if(status!=PAINT_OK) { showError(error); return false; }
    m_pendingLayer=m_pendingMutation=sequence; m_modified=true; emit stateChanged(); return true;
}
bool PaintCoreClient::submitGroup(quint32 kind,quint64 id,quint64 parent,const QString &name) {
    if(!m_ready || m_drawing || layerEditBusy() || m_fileBusy) return false;
    const auto bytes=name.toUtf8();auto request=dto<PaintGroupRequest>();request.kind=kind;request.layer_id=id;request.parent_id=parent;
    if(kind==PAINT_GROUP_WRAP) {request.name=reinterpret_cast<const uint8_t*>(bytes.constData());request.name_length=static_cast<uint64_t>(bytes.size());}
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session) return false;
        status=paint_session_group(m_connection->core,m_connection->session,&request,&sequence);
        if(status!=PAINT_OK) error=immediateError(status);
    }
    if(status!=PAINT_OK) {showError(error);return false;}
    m_pendingLayer=m_pendingMutation=sequence;m_modified=true;emit stateChanged();return true;
}
bool PaintCoreClient::groupLayer(quint64 id,const QString &name) {return submitGroup(PAINT_GROUP_WRAP,id,0,name);}
bool PaintCoreClient::ungroupLayer(quint64 id) {return submitGroup(PAINT_GROUP_UNGROUP,id,0);}
bool PaintCoreClient::reparentLayer(quint64 id,quint64 parent) {return submitGroup(PAINT_GROUP_REPARENT,id,parent);}
bool PaintCoreClient::addMask(quint64 id) {return !layerEditBusy() && !m_drawing && !m_fileBusy && submit(AddMask,{},id);}
bool PaintCoreClient::setLayerClipping(quint64 id,bool enabled) {
    if(!m_ready || m_drawing || layerEditBusy() || m_fileBusy) return false;
    auto options=dto<PaintLayerClipping>();options.enabled=enabled?1u:0u;
    uint64_t seq=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
        status=paint_session_set_layer_clipping(m_connection->core,m_connection->session,id,&options,&seq);
        if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}
    m_pendingLayer=m_pendingMutation=seq;m_modified=true;emit stateChanged();return true;
}
bool PaintCoreClient::toggleActiveClipping() {
    quint64 id=m_active;
    for(const auto &v:m_layers){const auto l=v.toMap();if(l.value("id").toULongLong()==id && l.value("mask").toBool()){id=l.value("parent").toULongLong();break;}}
    for(const auto &v:m_layers){const auto l=v.toMap();if(l.value("id").toULongLong()==id)return setLayerClipping(id,!l.value("clipped").toBool());}
    return false;
}
bool PaintCoreClient::dropLayer(quint64 id,quint64 target,int placement) {
    if(!m_ready || m_drawing || layerEditBusy() || m_fileBusy) return false;
    auto r=dto<PaintLayerDrop>();r.layer_id=id;r.target_id=target;r.placement=static_cast<uint32_t>(placement);
    uint64_t seq=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;status=paint_session_drop_layer(m_connection->core,m_connection->session,&r,&seq);if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}
    m_pendingLayer=m_pendingMutation=seq;m_modified=true;
    if(placement==0 && m_collapsedGroups.remove(target))emit groupExpansionChanged();
    emit stateChanged();return true;
}
QString PaintCoreClient::layerDragPayload(quint64 id) const {
    return QString::number(m_generation)+":"+QString::number(id);
}
bool PaintCoreClient::acceptLayerDrop(const QString &payload,quint64 target,int placement) {
    const auto parts=payload.split(':');bool ok=false;
    if(parts.size()!=2 || parts[0].toULongLong()!=m_generation)return false;
    const auto id=parts[1].toULongLong(&ok);return ok && dropLayer(id,target,placement);
}
void PaintCoreClient::beginLayerDrag(quint64 id) {
    if(!m_ready || m_drawing || layerEditBusy())return;
    for(const auto &v:m_layers)if(v.toMap().value("id").toULongLong()==id) {
        if(v.toMap().value("mask").toBool() || (v.toMap().value("effectiveLocks").toUInt() & 4))return;
        QDrag drag(this);auto *mime=new QMimeData;mime->setData("application/x-drawverse-layer",layerDragPayload(id).toUtf8());drag.setMimeData(mime);drag.exec(Qt::MoveAction);return;
    }
}
void PaintCoreClient::addDefaultLayer() {
    const auto number=[](int n){const QString digits=QStringLiteral("零一二三四五六七八九");const QString units=QStringLiteral("千百十");QString result;bool zero=false;for(int d=1000,u=0;d>1;d/=10,++u){const int v=n/d;n%=d;if(v){if(zero)result+=digits[0];if(!(d==10 && v==1 && result.isEmpty()))result+=digits[v];result+=units[u];zero=false;}else if(!result.isEmpty() && n)zero=true;}if(n){if(zero)result+=digits[0];result+=digits[n];}return result;};
    // Names do not count groups/masks and do not reuse an existing default name.
    if(!m_ready || m_drawing || layerEditBusy() || m_fileBusy) return;
    QSet<QString> names;
    for(const auto &v:m_layers) names.insert(v.toMap().value("name").toString());
    int serial=m_nextDefaultLayer;QString name;
    for(;serial<=4096;++serial){name=QStringLiteral("图层")+number(serial);if(!names.contains(name))break;}
    if(serial>4096){showError(QStringLiteral("默认图层编号达到上限"));return;}
    m_nextDefaultLayer=serial+1;addLayer(name);
}
QVariantList PaintCoreClient::collapsedGroups() const {
    QVariantList result;for(auto id:m_collapsedGroups) result.append(QVariant::fromValue<qulonglong>(id));return result;
}
void PaintCoreClient::toggleGroupExpanded(quint64 id) {
    for(const auto &value:m_layers) {const auto layer=value.toMap();if(layer.value("id").toULongLong()==id && layer.value("group").toBool()) {
        if(!m_collapsedGroups.remove(id)) m_collapsedGroups.insert(id);emit groupExpansionChanged();return;
    }}
}
void PaintCoreClient::newTransparentDocument(int w,int h) {
    if(w<1 || h<1 || w>1000000 || h>1000000) {showError(QStringLiteral("画布尺寸支持 1–1,000,000 像素"));return;}
    if(m_ready && !m_drawing && !m_fileBusy && submit(New,{},static_cast<quint64>(w),{},h)) {m_documentUrl={};m_sourceUrl={};emit stateChanged();}
}
void PaintCoreClient::newDocument(int w, int h) {
    if (w < 1 || h < 1 || w > 1000000 || h > 1000000) { showError(QStringLiteral("画布尺寸支持 1–1,000,000 像素")); return; }
    if (m_ready && !m_drawing && !m_fileBusy && submit(NewWhite, {}, static_cast<quint64>(w), {}, h)) {
        m_documentUrl = {}; m_sourceUrl = {}; emit stateChanged();
    }
}
QString PaintCoreClient::documentName() const {
    return m_sourceUrl.isEmpty() ? QStringLiteral("未命名") : QFileInfo(m_sourceUrl.toLocalFile()).fileName();
}
bool PaintCoreClient::openDocument(const QUrl &path) { return submitFile(path, PAINT_FILE_OPEN, PAINT_FILE_AUTO); }
bool PaintCoreClient::saveDocument(const QUrl &path, int format) { return submitFile(path, PAINT_FILE_SAVE, format); }
bool PaintCoreClient::submitFile(const QUrl &path, int kind, int format) {
    if (!m_ready || m_drawing || m_fileBusy || m_closing) return false;
    if (!path.isLocalFile() || path.toLocalFile().isEmpty()) { showError(QStringLiteral("请选择本地文件")); return false; }
    auto request = dto<PaintFileRequest>(); request.kind = static_cast<uint32_t>(kind);
    request.format = static_cast<uint32_t>(format); request.quality = 95; request.document_generation = m_generation;
    for (auto &channel : request.linear_background) channel = 1;
    const auto bytes = path.toLocalFile().toUtf8(); request.path = reinterpret_cast<const uint8_t *>(bytes.constData());
    request.path_length = static_cast<uint64_t>(bytes.size());
    uint64_t job = 0; PaintStatus status = PAINT_BUSY; QString error;
    { QMutexLocker lock(&m_connection->mutex);
      if (m_connection->session) status = paint_session_file_submit(m_connection->core, m_connection->session, &request, &job);
      if (status == PAINT_OK) m_connection->fileJob.store(job); else error = immediateError(status);
    }
    if (status != PAINT_OK) { showError(error); return false; }
    clearError(); m_fileId = job; m_fileUrl = path; m_fileBusy = true; m_fileOpening = kind == PAINT_FILE_OPEN;
    if (m_fileOpening) m_ready = false;
    emit stateChanged(); return true;
}
void PaintCoreClient::cancelFile() {
    if (!m_fileBusy || m_closing) return;
    PaintStatus status = PAINT_OK;
    { QMutexLocker lock(&m_connection->mutex);
      if (m_connection->session) status = paint_session_file_cancel(m_connection->core, m_connection->session, m_fileId);
    }
    if (status != PAINT_OK && status != PAINT_BUSY) showError(immediateError(status));
}
#include "PaintCoreClient.moc"

bool PaintCoreClient::inspectStorage() {
    if (m_closing || m_storageBusy) return false;
    m_storageBusy=true; emit storageChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker]{worker->inspectStorage();},Qt::QueuedConnection);
    return true;
}
bool PaintCoreClient::saveStorageSettings(const QString &directory,int memoryMiB,int scratchGiB,int reserveMiB) {
    if (m_closing || m_storageBusy) return false;
    const QVariantMap values{{"directory",directory},{"memoryMiB",memoryMiB},{"scratchGiB",scratchGiB},{"reserveMiB",reserveMiB}};
    m_storageBusy=true; m_storageMessage.clear(); emit storageChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,values]{worker->saveStorage(values);},Qt::QueuedConnection);
    return true;
}
