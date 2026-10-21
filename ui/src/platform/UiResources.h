#pragma once
#include <QResource>

inline void initializeDrawVerseUiResources() {
    Q_INIT_RESOURCE(ui_resources);
    // Direct qrc URLs do not import the packaging module. Keep its cache
    // registration alive when linking the UI as a static library.
    Q_INIT_RESOURCE(qmlcache_drawverse_ui);
}
