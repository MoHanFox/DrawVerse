#pragma once
#include <QResource>
#include <qqml.h>
#include "ShortcutSettings.h"


inline void initializeDrawVerseUiResources() {
    static const int shortcutType=qmlRegisterSingletonType<ShortcutSettings>("DrawVerse",1,0,"ShortcutSettings",[](QQmlEngine *,QJSEngine *) -> QObject * {return new ShortcutSettings;});
    Q_UNUSED(shortcutType);
    Q_INIT_RESOURCE(ui_resources);
    // Direct qrc URLs do not import the packaging module. Keep its cache
    // registration alive when linking the UI as a static library.
    Q_INIT_RESOURCE(qmlcache_drawverse_ui);
}
