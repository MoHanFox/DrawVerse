cmake_minimum_required(VERSION 3.21)
project(DrawVerseUI LANGUAGES CXX)

find_package(Qt6 6.5 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2 Test)
set(CMAKE_AUTOMOC ON)

add_library(drawverse_ui STATIC
    src/platform/DeveloperConsole.h src/platform/DeveloperConsole.cpp
    src/platform/WindowDrag.h src/platform/WindowDrag.cpp
    src/documents/DocumentManager.h src/documents/DocumentManager.cpp
    src/workspace/DockTree.h src/workspace/DockTree.cpp
    src/platform/UiScale.h
    src/platform/MenuSurface.h src/platform/MenuSurface.cpp
    src/platform/MenuBlurLayer.h src/platform/MenuBlurLayer.cpp
    src/engine/PaintCoreClient.h src/engine/PaintCoreClient.cpp
    src/canvas/CanvasItem.h src/canvas/CanvasItem.cpp
    src/canvas/SelectionOverlay.h src/canvas/SelectionOverlay.cpp
    src/color/ColorWheelItem.h src/color/ColorWheelItem.cpp
    src/brushes/BrushLibrary.h src/brushes/BrushLibrary.cpp
    src/brushes/BrushCurveModes.h
    src/workspace/WorkspaceManager.h src/workspace/WorkspaceManager.cpp)
target_compile_features(drawverse_ui PUBLIC cxx_std_20)
target_include_directories(drawverse_ui PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src")
target_link_libraries(drawverse_ui PUBLIC Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::QuickControls2
    PRIVATE drawverse_ffi_static)
if(WIN32)
    target_link_libraries(drawverse_ui PRIVATE dwmapi windowsapp CoreMessaging)
endif()

# Keep the resource manifest and its rebuild dependencies in one place.
set(_drawverse_ui_resources
    qml/app/Main.qml qml/app/WorkspaceStatusBar.qml
    qml/workspace/DockWorkspace.qml qml/workspace/DockTile.qml qml/workspace/DockDivider.qml qml/workspace/DockEdgePreview.qml qml/workspace/DockHintWindow.qml qml/workspace/FloatingPanel.qml qml/workspace/PanelGroup.qml qml/workspace/PanelFlyout.qml qml/workspace/PanelContent.qml
    qml/documents/DocumentArea.qml qml/documents/DocumentWindow.qml
    qml/canvas/CanvasPane.qml qml/canvas/TransformOverlay.qml qml/canvas/TransparencyGrid.qml qml/canvas/EyedropperRing.qml
    qml/brushes/BrushLibraryPanel.qml qml/brushes/BrushSettingsPanel.qml qml/brushes/PressureCurveEditor.qml qml/brushes/SpeedCurveEditor.qml
    qml/brushes/BrushResourceThumbnail.qml qml/brushes/BrushResourcePicker.qml
    qml/layers/LayersPanel.qml qml/layers/LayerLock.qml qml/layers/LayerThumbnail.qml
    qml/color/HsvSlider.qml
    qml/tools/ToolStrip.qml qml/tools/ToolVariantsPanel.qml qml/tools/SelectionOptionsBar.qml qml/tools/BucketOptionsBar.qml qml/tools/WandOptionsBar.qml qml/tools/SmudgeOptionsBar.qml qml/tools/LiquifyOptionsBar.qml qml/tools/TransformOptionsBar.qml
    qml/preferences/PreferencesPanel.qml qml/preferences/StoragePreferences.qml qml/preferences/KeyboardShortcutsDialog.qml
    qml/controls/CompactCheckBox.qml qml/controls/CompactComboBox.qml qml/controls/CompactSlider.qml qml/controls/ParameterSlider.qml qml/controls/PanelDialog.qml qml/controls/CompactSpinBox.qml qml/controls/CompactTextField.qml qml/controls/Icon.qml qml/controls/IconButton.qml qml/controls/ComboPopupWindow.qml qml/controls/LayerPercent.qml
    qml/menus/GlassMenu.qml qml/menus/GlassMenuItem.qml qml/menus/SideMenuWindow.qml
    qml/windows/PanelWindow.qml qml/windows/ResizeFrame.qml qml/windows/WindowOutline.qml
    qml/theme/Theme.qml
    qml/theme/qmldir qml/assets/logo.png qml/assets/transparency.png)
qt_add_resources(drawverse_ui ui_resources PREFIX "/" BASE "${CMAKE_CURRENT_SOURCE_DIR}"
    FILES ${_drawverse_ui_resources})

# Editing QML or an asset must recompile the generated resource object.
get_target_property(_drawverse_qrc_sources drawverse_ui SOURCES)
set(_drawverse_qml_paths "")
foreach(_drawverse_qml ${_drawverse_ui_resources})
    list(APPEND _drawverse_qml_paths "${CMAKE_CURRENT_SOURCE_DIR}/${_drawverse_qml}")
endforeach()
foreach(_drawverse_qrc ${_drawverse_qrc_sources})
    if(_drawverse_qrc MATCHES "qrc_ui_resources\\.cpp$")
        set_property(SOURCE "${_drawverse_qrc}" APPEND PROPERTY OBJECT_DEPENDS "${_drawverse_qml_paths}")
    endif()
endforeach()



qt_add_executable(drawverse src/app/main.cpp src/app/PaintingBenchmark.h src/app/PaintingBenchmark.cpp)
target_link_libraries(drawverse PRIVATE drawverse_ui)
set_target_properties(drawverse PROPERTIES WIN32_EXECUTABLE TRUE RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>")

add_custom_target(drawverse_license ALL
    COMMAND "${CMAKE_COMMAND}" -E copy_if_different
        "${CMAKE_SOURCE_DIR}/LICENSE" "$<TARGET_FILE_DIR:drawverse>/LICENSE"
    DEPENDS drawverse
    COMMENT "Staging the project license alongside the application"
    VERBATIM)

qt_add_executable(qt_ui_tests tests/ui_tests.cpp)
target_link_libraries(qt_ui_tests PRIVATE drawverse_ui Qt6::Test)
set_target_properties(qt_ui_tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>")
set_target_properties(qt_ui_tests PROPERTIES WIN32_EXECUTABLE FALSE)
if(MSVC)
    target_compile_options(qt_ui_tests PRIVATE /bigobj)
endif()
add_test(NAME qt_ui_tests COMMAND qt_ui_tests -o "${CMAKE_BINARY_DIR}/qt-ui-tests.txt,txt" -o "-,txt")
set_tests_properties(qt_ui_tests PROPERTIES TIMEOUT 90 ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software")
add_test(NAME qt_ui_highdpi COMMAND qt_ui_tests qmlMouseTabletAndFloatingWindows
    foldedNeighboursKeepTheirSizes independentFoldedColumnsKeepExpandedNeighboursAndRestore
    mainWindowRestoresWithoutNativeFrameOrClippedContent
    maximizedTitleBarDragRestoresAndFollowsPointer nativeMenuBlurLayerFollowsWindowAndHidesBeforeMinimize
    brushPresetClicksKeepEraserAndShareActualStrokeSettings
    dockSeparatorsUseThinDarkGrayAcrossMainAndFloatingColumns
    draggingPanelsSelectsNearestExposedEdgeAndShowsEdgeGlow
    panelTabsSwapWithinTheirBarBeforeDetaching
    floatingCanvasAndToolsDockAcrossNativeWindows
    panelColumnsUseOneHeaderAndDragTheActualWindow
    iconRailsSwitchSlideDismissAndDragPanels
    floatingIconRailsKeepBackgroundSidePanelsAndReturnState
    panelPresentationRemembersFlyoutsAndPaintsActualHeaderAlpha
    compactStatusReflectsDocumentAndZoomControlsPreserveCenter
    categoryFlyoutsShareRowsTabsAndRetractWithoutExpanding
    navigatorTracksVisibleDocumentAndSharesContentAcrossWindows
    movingWholeCategoryColumnsPreservesRowsAndOuterEdges
    toolsDockAlongWholeColumnAndStayResizableAfterPanelLeaves
    collapsedExpandButtonDragsWithoutExpandingAndPixelsStayInPlace
    colorClicksAndCapsLockControlBrushOutline
    topMenusUseNativeWindowsAboveFloatingPanels
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
    lassoPolygonAndMarqueeShareOptionsHistoryAndPersistence
    denseLassoPaintingUpdatesFramesAndRestoresSelectionAfterUndo
    bucketOptionsMouseTabletAndFloatingCanvasFillOnce
    polygonDoubleClickClosureAndTabletFreehandUseSameInputPath
    selectionsDragCombineUndoConstrainAndPersist
    panelTabsRetractWithoutExpandingRailsAndKeepSharedPresentation
    textFieldsKeepSpaceAndHistoryReplayStaysStepped
    editMenuOpensKeyboardShortcutList
    selectionToolVariantsUseCornerMarkAndRightClickPanel
    closingDockedCategoryKeepsOtherHeightsAndFullToolColumn
    layerThumbnailsUpdateWithoutRebuildingRows
    categorizedMenusHoverIntoCompactNativeSubmenus
    selectedLayersDeleteDragAndGroupChoicesAreAtomic
    newGroupContainsWholeSelectionFromBothUiEntrypoints
    layerBlendHoverPreviewsWithoutHistoryAndCommitsOnce
    firstLayersPanelOpenIncubatesWithoutBlockingTheEventLoop
    altEyedropperSamplesWithoutHistoryAndBracketKeysResize
    selectionOutlineMarchesWhileVisibleAndStopsOtherwise
    shiftDragConstrainsSelectionToSquareAndCircle
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
