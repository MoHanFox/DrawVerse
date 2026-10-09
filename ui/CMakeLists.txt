find_package(Qt6 6.5 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2 Test)
set(CMAKE_AUTOMOC ON)

add_library(drawverse_ui STATIC
    src/WindowDrag.h src/WindowDrag.cpp
    src/DocumentManager.h src/DocumentManager.cpp
    src/DockTree.h src/DockTree.cpp
    src/UiScale.h
    src/MenuSurface.h src/MenuSurface.cpp
    src/PaintCoreClient.h src/PaintCoreClient.cpp
    src/CanvasItem.h src/CanvasItem.cpp
    src/SelectionOverlay.h src/SelectionOverlay.cpp
    src/ColorWheelItem.h src/ColorWheelItem.cpp
    src/BrushLibrary.h src/BrushLibrary.cpp
    src/WorkspaceManager.h src/WorkspaceManager.cpp)
target_compile_features(drawverse_ui PUBLIC cxx_std_20)
target_include_directories(drawverse_ui PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src")
target_link_libraries(drawverse_ui PUBLIC Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::QuickControls2
    PRIVATE drawverse_ffi_static)

qt_add_resources(drawverse_ui ui_resources PREFIX "/" BASE "${CMAKE_CURRENT_SOURCE_DIR}"
    FILES qml/qmldir qml/Theme.qml qml/GlassMenu.qml qml/GlassMenuItem.qml qml/Main.qml qml/PanelGroup.qml qml/BrushLibraryPanel.qml qml/BrushSettingsPanel.qml
          qml/CanvasPane.qml qml/DocumentArea.qml qml/DocumentWindow.qml qml/PanelFlyout.qml qml/DockWorkspace.qml qml/DockTile.qml qml/DockDivider.qml qml/DockEdgePreview.qml
          qml/SideMenuWindow.qml qml/FloatingPanel.qml qml/ToolStrip.qml qml/PanelContent.qml qml/Icon.qml qml/IconButton.qml qml/CompactSpinBox.qml qml/CompactSlider.qml qml/CompactComboBox.qml qml/ResizeFrame.qml qml/HsvSlider.qml qml/assets/logo.png qml/StoragePreferences.qml qml/LayersPanel.qml qml/LayerPercent.qml qml/LayerLock.qml
          qml/TransparencyGrid.qml qml/LayerThumbnail.qml qml/assets/transparency.png)

qt_add_executable(drawverse src/main.cpp src/PaintingBenchmark.h src/PaintingBenchmark.cpp)
target_link_libraries(drawverse PRIVATE drawverse_ui)
set_target_properties(drawverse PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>")

qt_add_executable(qt_ui_tests tests/ui_tests.cpp)
target_link_libraries(qt_ui_tests PRIVATE drawverse_ui Qt6::Test)
set_target_properties(qt_ui_tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>")
set_target_properties(qt_ui_tests PROPERTIES WIN32_EXECUTABLE FALSE)
add_test(NAME qt_ui_tests COMMAND qt_ui_tests -o "${CMAKE_BINARY_DIR}/qt-ui-tests.txt,txt" -o "-,txt")
set_tests_properties(qt_ui_tests PROPERTIES TIMEOUT 90 ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software")
add_test(NAME qt_ui_highdpi COMMAND qt_ui_tests qmlMouseTabletAndFloatingWindows
    mainWindowRestoresWithoutNativeFrameOrClippedContent
    draggingPanelsSelectsNearestExposedEdgeAndShowsEdgeGlow
    panelTabsSwapWithinTheirBarBeforeDetaching
    floatingCanvasAndToolsDockAcrossNativeWindows
    panelColumnsUseOneHeaderAndDragTheActualWindow
    iconRailsSwitchSlideDismissAndDragPanels
    panelsReturnToCanvasOnlyAndEmptyWorkspaceEdges
    uiScaleAppliesToMainPanelAndToolWindowsWithoutChangingLayout
    menusReopenWithoutGhostsAndFloatBesidePanels
    layerControlsCommitOnceAndWorkInFloatingPanels moveToolMouseKeyboardAndPositionLock
    transparentCanvasAndNavigatorUseCheckerboardWithoutChangingPixels
    groupsCollapseMoveUndoAndSaveWorkAcrossFloatingPanels
    masksDefaultsDepthAndDragDropWorkAcrossFloatingPanels
    inlineMasksClippingAltClicksAndOpaquePreviewsWorkInFloatingPanels
    referenceWorkspaceDockingAndColorPicker
    defaultLayoutRestoresTwoDockedColumnsAndMenuEntry
    toolStripFloatsDocksPersistsAndTabsJoinContent
    brushLibrarySettingsAndEnginePreviewsShareSelectionAcrossWindows
    selectionsDragCombineUndoConstrainAndPersist
    -o "${CMAKE_BINARY_DIR}/qt-ui-highdpi.txt,txt" -o "-,txt")
set_tests_properties(qt_ui_highdpi PROPERTIES TIMEOUT 90
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QT_SCALE_FACTOR=2")

if(MSVC)
    foreach(target IN ITEMS drawverse_ui drawverse qt_ui_tests)
        target_compile_options(${target} PRIVATE /W4 /WX /utf-8)
    endforeach()
else()
    foreach(target IN ITEMS drawverse_ui drawverse qt_ui_tests)
        target_compile_options(${target} PRIVATE -Wall -Wextra -Werror)
    endforeach()
endif()

# Dynamic Qt keeps the UI kit replaceable. Stage its runtime and QML imports for direct launch.
if(WIN32)
    get_target_property(qt_qmake Qt6::qmake IMPORTED_LOCATION)
    get_filename_component(qt_bin "${qt_qmake}" DIRECTORY)
    find_program(qt_deploy NAMES windeployqt HINTS "${qt_bin}" REQUIRED NO_DEFAULT_PATH)
    add_custom_target(deploy_qt ALL
        COMMAND "${qt_deploy}" "$<IF:$<CONFIG:Debug>,--debug,--release>" --no-translations --qmldir "${CMAKE_CURRENT_SOURCE_DIR}/qml"
                "$<TARGET_FILE:drawverse>"
        COMMAND "${qt_deploy}" "$<IF:$<CONFIG:Debug>,--debug,--release>" --no-translations --qmldir "${CMAKE_CURRENT_SOURCE_DIR}/qml"
                "$<TARGET_FILE:qt_ui_tests>"
        # windeployqt selects qwindows by default; Qt Test explicitly uses offscreen.
        COMMAND "${CMAKE_COMMAND}" -E copy_if_different
                "${qt_bin}/../plugins/platforms/qoffscreen$<$<CONFIG:Debug>:d>.dll"
                "$<TARGET_FILE_DIR:qt_ui_tests>/platforms/"
        DEPENDS drawverse qt_ui_tests VERBATIM)
endif()
