import QtQuick
import QtQuick.Controls

// The public value stays in model units; only the track uses normalized coordinates.
FocusScope {
    id: root
    property real from: 0
    property real to: 100
    property real value: from
    property real stepSize: 1
    property bool exponential: false
    property alias tooltip: track.tooltip
    readonly property real position: positionForValue(value)
    // Keep the caller's value binding intact while delivering a pending edit.
    property real pendingValue: NaN
    readonly property real editedValue: isNaN(pendingValue) ? value : pendingValue
    signal moved()
    implicitWidth: track.implicitWidth
    implicitHeight: track.implicitHeight

    function clamp(v) { return Math.max(from, Math.min(to, v)) }
    function mapped(v) { return exponential ? Math.sign(v)*Math.log(1+Math.abs(v)) : v }
    function positionForValue(v) {
        const span = mapped(to)-mapped(from)
        return span > 0 ? (mapped(clamp(v))-mapped(from))/span : 0
    }
    function valueAt(p) {
        if (p <= 0 || to <= from) return from
        if (p >= 1) return to
        const v = mapped(from)+p*(mapped(to)-mapped(from))
        return clamp(exponential ? Math.sign(v)*(Math.exp(Math.abs(v))-1) : v)
    }
    function moveTo(v) {
        pendingValue = clamp(stepSize > 0 ? from+Math.round((v-from)/stepSize)*stepSize : v)
        try { moved() } finally { pendingValue = NaN }
    }
    CompactSlider {
        id: track
        objectName: root.objectName+"Track"
        anchors.fill: parent
        focus: true
        from: 0; to: 1; stepSize: 0
        value: root.position
        onMoved: root.moveTo(root.valueAt(value))
        Keys.onPressed: event => {
            if (event.key === Qt.Key_Left || event.key === Qt.Key_Down)
                root.moveTo(root.value-root.stepSize)
            else if (event.key === Qt.Key_Right || event.key === Qt.Key_Up)
                root.moveTo(root.value+root.stepSize)
            else if (event.key === Qt.Key_Home) root.moveTo(root.from)
            else if (event.key === Qt.Key_End) root.moveTo(root.to)
            else return
            event.accepted = true
        }
    }
}
