#include "paint_api.h"
#include <array>
#include <atomic>
#include <cstddef>
#include <cstdio>
#include <thread>
#include <type_traits>
#include <vector>
#include "async_smoke.h"
#include "storage_smoke.h"

#define CHECK(condition) do { if (!(condition)) { std::fprintf(stderr, "C++ ABI check failed at line %d: %s\n", __LINE__, #condition); return 1; } } while (false)
static_assert(std::is_standard_layout_v<PaintPoint>);
static_assert(sizeof(PaintPoint) == 64 && offsetof(PaintPoint, timestamp_ns) == 56);
static_assert(sizeof(PaintStrokeDesc) == 56 && offsetof(PaintStrokeDesc, linear_rgba) == 28);
static_assert(sizeof(PaintTile) == 56 && offsetof(PaintTile, revision) == 48);
static_assert(sizeof(PaintEvent) == 56);
static_assert(std::is_standard_layout_v<PaintStorageOptions> && sizeof(PaintStorageOptions) == 48);
static_assert(offsetof(PaintStorageOptions, path) == 8 && sizeof(PaintStorageInfo) == 32);

struct Kernel {
    PaintCore *core = nullptr;
    PaintDocument *doc = nullptr;
    PaintSubscription *subscription = nullptr;
    Kernel() = default;
    Kernel(const Kernel &) = delete;
    Kernel &operator=(const Kernel &) = delete;
    ~Kernel() noexcept {
        if (subscription) paint_core_unsubscribe(core, &subscription);
        if (doc) paint_document_destroy(core, &doc);
        if (core) paint_core_destroy(&core);
    }
};

static void callback(const PaintEvent *event, void *user) noexcept {
    auto &count = *static_cast<std::atomic<unsigned> *>(user);
    if (event && event->struct_size == sizeof(PaintEvent)) count.fetch_add(1, std::memory_order_relaxed);
}
static_assert(noexcept(callback(nullptr, nullptr)));

int main() {
    CHECK(storage_smoke() == 0);
    // Callback storage outlives the RAII subscription owner.
    std::atomic<unsigned> events{0};
    Kernel kernel;
    CHECK(paint_core_create(&kernel.core) == PAINT_OK);
    PaintDocumentDesc desc{};
    desc.struct_size = sizeof(desc); desc.width = 32; desc.height = 32;
    desc.working_space = PAINT_WORKING_LINEAR_SRGB; desc.pixel_format = PAINT_STORAGE_RGBA32F_PREMULTIPLIED;
    CHECK(paint_core_new_document(kernel.core, &desc, &kernel.doc) == PAINT_OK);
    CHECK(paint_core_subscribe(kernel.core, callback, &events, &kernel.subscription) == PAINT_OK);

    PaintStrokeDesc stroke{};
    stroke.struct_size = sizeof(stroke); stroke.layer_id = 1; stroke.mode = PAINT_MODE_PAINT;
    stroke.radius = 4; stroke.opacity = 1; stroke.spacing = 0.25f; stroke.linear_rgba[2] = 1; stroke.linear_rgba[3] = 1;
    PaintPoint point{};
    point.struct_size = sizeof(point); point.tool = PAINT_TOOL_PEN; point.pressure = 0.5f; point.x = 16.5; point.y = 16.5;
    CHECK(paint_core_begin_stroke(kernel.core, kernel.doc, &stroke, &point) == PAINT_OK);
    CHECK(paint_core_end_stroke(kernel.core, kernel.doc) == PAINT_OK);
    std::array<float, 4> pixel{};
    PaintTile tile{};
    tile.struct_size = sizeof(tile); tile.format = PAINT_TILE_RGBA32F_LINEAR_PREMULTIPLIED;
    tile.data = reinterpret_cast<uint8_t *>(pixel.data()); tile.stride = sizeof(pixel); tile.capacity = sizeof(pixel);
    CHECK(paint_core_read_tile(kernel.core, kernel.doc, 16, 16, 1, 1, &tile) == PAINT_OK);
    CHECK(pixel[2] == 0.5f && pixel[3] == 0.5f);
    CHECK(paint_core_undo(kernel.core, kernel.doc) == PAINT_OK);
    CHECK(paint_core_redo(kernel.core, kernel.doc) == PAINT_OK);
    CHECK(events.load() >= 4);

    // Separate caller-owned output memory per reader; read-side synchronization lives in Rust.
    std::atomic<bool> failed{false};
    std::vector<std::jthread> readers;
    for (int thread = 0; thread < 4; ++thread) {
        readers.emplace_back([&] {
            for (int iteration = 0; iteration < 100; ++iteration) {
                PaintDocumentInfo info{}; info.struct_size = sizeof(info);
                if (paint_document_info(kernel.core, kernel.doc, &info) != PAINT_OK || info.width != 32 || info.undo_depth != 1) failed.store(true);
            }
        });
    }
    readers.clear(); // join before lifecycle operations
    CHECK(!failed.load());
    CHECK(paint_core_unsubscribe(kernel.core, &kernel.subscription) == PAINT_OK);
    const auto count = events.load();
    CHECK(paint_core_begin_stroke(kernel.core, kernel.doc, &stroke, &point) == PAINT_OK);
    CHECK(paint_core_cancel_stroke(kernel.core, kernel.doc) == PAINT_OK);
    CHECK(events.load() == count);
    CHECK(async_smoke(kernel.core) == 0);
    std::puts("C++20 ABI RAII, pressure, history, noexcept callback, and concurrent-reader checks passed.");
    return 0;
}
