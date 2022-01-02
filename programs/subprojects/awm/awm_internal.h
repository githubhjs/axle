#ifndef AWM_INTERNAL_H
#define AWM_INTERNAL_H

#include <stdint.h>

typedef void(*awm_timer_cb_t)(void* ctx);

typedef struct awm_timer {
    uint32_t start_time;
    uint32_t duration;
    uint32_t fires_after;
    awm_timer_cb_t invoke_cb;
    void* invoke_ctx;
} awm_timer_t;

void awm_timer_start(uint32_t duration, awm_timer_cb_t timer_cb, void* invoke_ctx);

ca_layer* video_memory_layer(void);
ca_layer* physical_video_memory_layer(void);
Rect _draw_cursor(ca_layer* dest);

#endif