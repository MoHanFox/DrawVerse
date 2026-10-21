#pragma once
#include <QCryptographicHash>
#include <QDirIterator>
#include <QFile>
#include <QFileSystemWatcher>
#include <QMap>
#include <QTimer>
#include <functional>

// Developer-only, event-driven reloads. Hashing after the debounce also handles
// editors that replace files atomically or preserve their timestamp and length.
class QmlSourceWatcher final : public QObject {
public:
    QmlSourceWatcher(const QString &root,std::function<void()> reload):m_root(root),m_reload(std::move(reload)) {
        m_timer.setInterval(300);m_timer.setSingleShot(true);
        connect(&m_watcher,&QFileSystemWatcher::fileChanged,this,[this]{m_timer.start();});
        connect(&m_watcher,&QFileSystemWatcher::directoryChanged,this,[this]{m_timer.start();});
        m_snapshot=scan();refreshPaths();
        connect(&m_timer,&QTimer::timeout,this,[this] {
            const auto next=scan();refreshPaths();
            if(next!=m_snapshot){m_snapshot=next;m_reload();}
        });
    }
private:
    QMap<QString,QByteArray> scan() const {
        QMap<QString,QByteArray> result;
        QDirIterator files(m_root,{"*.qml","*.js","*.mjs","qmldir","*.png","*.jpg","*.jpeg","*.svg","*.webp"},QDir::Files,QDirIterator::Subdirectories);
        while(files.hasNext()){const auto path=files.next();QFile file(path);if(file.open(QIODevice::ReadOnly))result.insert(path,QCryptographicHash::hash(file.readAll(),QCryptographicHash::Sha256));}
        return result;
    }
    void refreshPaths() {
        const auto oldFiles=m_watcher.files(),oldDirectories=m_watcher.directories();
        if(!oldFiles.isEmpty())m_watcher.removePaths(oldFiles);
        if(!oldDirectories.isEmpty())m_watcher.removePaths(oldDirectories);
        QStringList paths{m_root};
        QDirIterator directories(m_root,QDir::Dirs|QDir::NoDotAndDotDot,QDirIterator::Subdirectories);
        while(directories.hasNext())paths.append(directories.next());
        paths.append(m_snapshot.keys());m_watcher.addPaths(paths);
    }
    QString m_root;
    std::function<void()> m_reload;
    QMap<QString,QByteArray> m_snapshot;
    QFileSystemWatcher m_watcher;
    QTimer m_timer;
};
