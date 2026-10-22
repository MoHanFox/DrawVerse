cmake_minimum_required(VERSION 3.21)
project(DrawVerseUI LANGUAGES CXX)

find_package(Qt6 6.5 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2 Test ShaderTools)
set(CMAKE_AUTOMOC ON)

add_library(drawverse_ui STATIC
    src/platform/DeveloperConsole.h src/platform/DeveloperConsole.cpp
    src/platform/WindowDrag.h src/platform/WindowDrag.cpp
    src/documents/DocumentManager.h src/documents/DocumentManager.cpp
    src/workspace/DockTree.h src/workspace/DockTree.cpp
    src/platform/TabletInput.h src/platform/TabletInput.cpp
    src/platform/UiResources.h src/platform/UiScale.h
    src/platform/MenuSurface.h src/platform/MenuSurface.cpp
    src/platform/MenuBlurLayer.h src/platform/MenuBlurLayer.cpp
    src/engine/PaintCoreClient.h src/engine/PaintCoreClient.cpp
    src/canvas/CanvasItem.h src/canvas/CanvasItem.cpp
    src/canvas/LiquifyPreviewNode.h src/canvas/LiquifyPreviewNode.cpp
    src/canvas/ResidentPreviewTextures.h src/canvas/ResidentPreviewTextures.cpp
    src/canvas/SelectionOverlay.h src/canvas/SelectionOverlay.cpp
    src/color/ColorWheelItem.h src/color/ColorWheelItem.cpp
    src/brushes/BrushLibrary.h src/brushes/BrushLibrary.cpp src/brushes/BrushToolIsolation.cpp
    src/brushes/BrushCurveModes.h
    src/workspace/WorkspaceManager.h src/workspace/WorkspaceManager.cpp)
target_compile_features(drawverse_ui PUBLIC cxx_std_20)
target_include_directories(drawverse_ui PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src")
target_link_libraries(drawverse_ui PUBLIC Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::QuickControls2
    PRIVATE drawverse_ffi_static)
if(WIN32)
    target_link_libraries(drawverse_ui PRIVATE dwmapi windowsapp CoreMessaging Qt6::GuiPrivate)
endif()

# Keep the resource manifest and its rebuild dependencies in one place.
set(_drawverse_ui_resources
    qml/app/Main.qml qml/app/WelcomeScreen.qml qml/app/WorkspaceStatusBar.qml
    qml/workspace/DockWorkspace.qml qml/workspace/DockTile.qml qml/workspace/DockDivider.qml qml/workspace/DockEdgePreview.qml qml/workspace/DockHintWindow.qml qml/workspace/FloatingPanel.qml qml/workspace/PanelGroup.qml qml/workspace/PanelFlyout.qml qml/workspace/PanelContent.qml
    qml/documents/DocumentArea.qml qml/documents/DocumentWindow.qml
    qml/canvas/CanvasPane.qml qml/canvas/TaskProgressOverlay.qml qml/canvas/SelectionPresentation.qml qml/canvas/AdaptiveCursor.qml qml/canvas/TransformOverlay.qml qml/canvas/TransparencyGrid.qml qml/canvas/EyedropperRing.qml
    qml/brushes/BrushLibraryPanel.qml qml/brushes/BrushSettingsPanel.qml qml/brushes/PressureCurveEditor.qml qml/brushes/SpeedCurveEditor.qml
    qml/brushes/BrushResourceThumbnail.qml qml/brushes/BrushResourcePicker.qml qml/brushes/BrushTipShapeControl.qml
    qml/layers/LayersPanel.qml qml/layers/LayerLock.qml qml/layers/LayerThumbnail.qml
    qml/color/HsvSlider.qml
    qml/tools/ToolStrip.qml qml/tools/ToolVariantsPanel.qml qml/tools/SelectionOptionsBar.qml qml/tools/BucketOptionsBar.qml qml/tools/WandOptionsBar.qml qml/tools/SmudgeOptionsBar.qml qml/tools/LiquifyOptionsBar.qml qml/tools/TransformOptionsBar.qml
    qml/preferences/PreferencesPanel.qml qml/preferences/StoragePreferences.qml qml/preferences/KeyboardShortcutsDialog.qml
    qml/controls/CompactCheckBox.qml qml/controls/CompactComboBox.qml qml/controls/CompactSlider.qml qml/controls/MappedSlider.qml qml/controls/ParameterSlider.qml qml/controls/PanelDialog.qml qml/controls/CompactSpinBox.qml qml/controls/CompactTextField.qml qml/controls/Icon.qml qml/controls/IconButton.qml qml/controls/ComboPopupWindow.qml qml/controls/LayerPercent.qml
    qml/menus/GlassMenu.qml qml/menus/GlassMenuItem.qml qml/menus/SideMenuWindow.qml
    qml/windows/PanelWindow.qml qml/windows/ResizeFrame.qml qml/windows/WindowOutline.qml
    qml/theme/Theme.qml
    qml/theme/qmldir qml/assets/logo.png qml/assets/transparency.png)
# Keep existing qrc:/qml URLs and relative imports, but build bytecode rather
# than parsing all application QML on the first interactive frame. The private
# packaging URI is the existing resource directory, not a new runtime import.
set(_drawverse_ui_qml "")
set(_drawverse_ui_assets "")
foreach(_drawverse_resource IN LISTS _drawverse_ui_resources)
    if(_drawverse_resource MATCHES "\\.qml$")
        list(APPEND _drawverse_ui_qml "${_drawverse_resource}")
        string(REGEX REPLACE "^qml/" "" _drawverse_alias "${_drawverse_resource}")
        set_source_files_properties("${_drawverse_resource}" PROPERTIES QT_RESOURCE_ALIAS "${_drawverse_alias}")
    else()
        list(APPEND _drawverse_ui_assets "${_drawverse_resource}")
    endif()
endforeach()
# Keep existing relative-directory imports instead of adding qmldir redirects.
set(_drawverse_qml_directory_options "")
if(Qt6_VERSION VERSION_GREATER_EQUAL 6.8)
    list(APPEND _drawverse_qml_directory_options NO_GENERATE_EXTRA_QMLDIRS)
endif()
set_source_files_properties(qml/theme/Theme.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
set_target_properties(drawverse_ui PROPERTIES QT_QMLCACHEGEN_ARGUMENTS "--only-bytecode")
qt_add_qml_module(drawverse_ui URI qml VERSION 1.0
    OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/qml"
    RESOURCE_PREFIX "/" NO_PLUGIN NO_GENERATE_QMLTYPES ${_drawverse_qml_directory_options}
    QML_FILES ${_drawverse_ui_qml})
qt_add_resources(drawverse_ui ui_resources PREFIX "/" BASE "${CMAKE_CURRENT_SOURCE_DIR}"
    FILES ${_drawverse_ui_assets})
qt_add_shaders(drawverse_ui liquify_shaders PREFIX "/shaders" BASE "${CMAKE_CURRENT_SOURCE_DIR}/shaders"
    GLSL "300es,330" FILES shaders/liquify.vert shaders/liquify.frag shaders/adaptive-cursor.frag shaders/transform-composite.frag shaders/selection-ants.frag)

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

# Developer host: real C++ types, editable disk QML, isolated settings. Not shipped by default.
qt_add_executable(drawverse_qml_preview EXCLUDE_FROM_ALL src/app/main.cpp src/app/PaintingBenchmark.h src/app/PaintingBenchmark.cpp)
target_link_libraries(drawverse_qml_preview PRIVATE drawverse_ui)
target_compile_definitions(drawverse_qml_preview PRIVATE DRAWVERSE_QML_SOURCE_DIR="${CMAKE_CURRENT_SOURCE_DIR}/qml")
set_target_properties(drawverse_qml_preview PROPERTIES WIN32_EXECUTABLE FALSE RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>")
if(MSVC)
    target_compile_options(drawverse_qml_preview PRIVATE /W4 /WX /utf-8)
endif()

add_custom_target(drawverse_license ALL
    COMMAND "${CMAKE_COMMAND}" -E copy_if_different
        "${CMAKE_SOURCE_DIR}/LICENSE" "$<TARGET_FILE_DIR:drawverse>/LICENSE"
    DEPENDS drawverse
    COMMENT "Staging the project license alongside the application"
    VERBATIM)

qt_add_executable(qt_ui_tests tests/ui_tests.cpp)

qt_add_executable(qt_transform_tests tests/transform_tests.cpp)
target_link_libraries(qt_transform_tests PRIVATE drawverse_ui Qt6::Test Qt6::GuiPrivate)
set_target_properties(qt_transform_tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>" WIN32_EXECUTABLE FALSE)
add_test(NAME qt_transform_tests COMMAND qt_transform_tests)
set_tests_properties(qt_transform_tests PROPERTIES TIMEOUT 90 ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software")

qt_add_executable(qt_workflow_tests tests/workflow_tests.cpp)
target_link_libraries(qt_workflow_tests PRIVATE drawverse_ui Qt6::Test Qt6::QmlPrivate)
target_compile_definitions(qt_workflow_tests PRIVATE DRAWVERSE_QML_TEST_SOURCE_DIR="${CMAKE_CURRENT_SOURCE_DIR}/qml")
set_target_properties(qt_workflow_tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>" WIN32_EXECUTABLE FALSE)
add_test(NAME qt_workflow_tests COMMAND qt_workflow_tests)
set_tests_properties(qt_workflow_tests PROPERTIES TIMEOUT 90 ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software")

# Focused selection geometry regression and reproducible complexity benchmark.
qt_add_executable(qt_selection_tests tests/selection_tests.cpp src/canvas/SelectionOverlay.cpp src/canvas/SelectionOverlay.h)
target_include_directories(qt_selection_tests PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src")
target_compile_features(qt_selection_tests PRIVATE cxx_std_20)
target_link_libraries(qt_selection_tests PRIVATE Qt6::Quick Qt6::Test)
set_target_properties(qt_selection_tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>" WIN32_EXECUTABLE FALSE)
add_test(NAME qt_selection_tests COMMAND qt_selection_tests)
set_tests_properties(qt_selection_tests PROPERTIES TIMEOUT 60 ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software")
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
    brushPresetClicksKeepEraserAndUseSelectedPresetSize
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
    textFieldsKeepSpaceAndHistorySelectsDirectly
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
    add_custom_target(drawverse_qml_preview_runtime
        COMMAND "${qt_deploy}" "$<IF:$<CONFIG:Debug>,--debug,--release>" --no-translations --qmldir "${CMAKE_CURRENT_SOURCE_DIR}/qml"
                "$<TARGET_FILE:drawverse_qml_preview>"
        DEPENDS drawverse_qml_preview VERBATIM)
    add_custom_target(drawverse_runtime
        COMMAND "${qt_deploy}" "$<IF:$<CONFIG:Debug>,--debug,--release>" --no-translations --qmldir "${CMAKE_CURRENT_SOURCE_DIR}/qml"
                "$<TARGET_FILE:drawverse>"
        DEPENDS drawverse drawverse_license VERBATIM)
    add_custom_target(deploy_qt ALL
        COMMAND "${qt_deploy}" "$<IF:$<CONFIG:Debug>,--debug,--release>" --no-translations --qmldir "${CMAKE_CURRENT_SOURCE_DIR}/qml"
                "$<TARGET_FILE:qt_ui_tests>"
        # windeployqt selects qwindows by default; Qt Test explicitly uses offscreen.
        COMMAND "${CMAKE_COMMAND}" -E copy_if_different
                "${qt_bin}/../plugins/platforms/qoffscreen$<$<CONFIG:Debug>:d>.dll"
                "$<TARGET_FILE_DIR:qt_ui_tests>/platforms/"
        DEPENDS drawverse_runtime qt_ui_tests VERBATIM)
else()
    add_custom_target(drawverse_runtime DEPENDS drawverse drawverse_license)
    add_custom_target(drawverse_qml_preview_runtime DEPENDS drawverse_qml_preview)
endif()

# Manual investigation only: excluded from default builds and CTest.
qt_add_executable(qt_brush_interaction_baseline EXCLUDE_FROM_ALL tests/brush_interaction_baseline.cpp)
target_link_libraries(qt_brush_interaction_baseline PRIVATE drawverse_ui Qt6::Test)
execute_process(COMMAND git rev-parse HEAD WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}" OUTPUT_VARIABLE _interaction_revision OUTPUT_STRIP_TRAILING_WHITESPACE)
target_compile_definitions(qt_brush_interaction_baseline PRIVATE
    DRAWVERSE_INTERACTION_BASELINE="7f5bd4c3acd507fc24069350a7a66a276b992976"
    DRAWVERSE_INTERACTION_REVISION="${_interaction_revision}")
set_target_properties(qt_brush_interaction_baseline PROPERTIES WIN32_EXECUTABLE FALSE RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>")

qt_add_executable(qt_interaction_trace_tests EXCLUDE_FROM_ALL tests/interaction_trace_tests.cpp)
target_include_directories(qt_interaction_trace_tests PRIVATE src)
target_link_libraries(qt_interaction_trace_tests PRIVATE Qt6::Test)
set_target_properties(qt_interaction_trace_tests PROPERTIES WIN32_EXECUTABLE FALSE RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>")

qt_add_executable(qt_color_state_tests EXCLUDE_FROM_ALL tests/color_state_tests.cpp)
target_link_libraries(qt_color_state_tests PRIVATE drawverse_ui Qt6::Test)
set_target_properties(qt_color_state_tests PROPERTIES WIN32_EXECUTABLE FALSE RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin/$<CONFIG>")
