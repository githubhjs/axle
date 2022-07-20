#ifndef AWM_DOCK_H
#define AWM_DOCK_H

#include <libagx/lib/shapes.h>

// PT: Must match the definitions in the corresponding Rust file

#define AWM_DOCK_HEIGHT 32
#define AWM_DOCK_SERVICE_NAME "com.axle.awm_dock"

// Sent from awm to the dock
#define AWM_DOCK_WINDOW_CREATED 817
typedef struct awm_dock_window_created_event {
    uint32_t event;
    uint32_t window_id;
    uint32_t title_len;
    const char title[64];
} awm_dock_window_created_event_t;

#define AWM_DOCK_WINDOW_TITLE_UPDATED 818
typedef struct awm_dock_window_title_updated_event {
    uint32_t event;
    uint32_t window_id;
    uint32_t title_len;
    const char title[64];
} awm_dock_window_title_updated_event_t;

#define AWM_DOCK_WINDOW_MINIMIZE_REQUESTED 819
typedef struct awm_dock_window_minimize_requested_event {
    uint32_t event;
    uint32_t window_id;
} awm_dock_window_minimize_requested_event_t;

#define AWM_DOCK_WINDOW_MINIMIZE_WITH_INFO 820
typedef struct awm_dock_window_minimize_with_info_event {
    uint32_t event;
    uint32_t window_id;
    Rect task_view_frame;
} awm_dock_window_minimize_with_info_event_t;

#endif