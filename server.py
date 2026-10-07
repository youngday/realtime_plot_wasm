import asyncio
import json
import math
import random
from datetime import datetime, timedelta, timezone

import websockets

# Seconds between frames. Matches `examples/server.rs` (100 ms); the previous
# 0.01 s pushed 100 points at 100 Hz, i.e. ten times the chart's redraw rate.
SEND_INTERVAL_SECS = 0.1


async def handle_client(websocket):
    cnt = 0.0
    # Computed once, like `examples/server.rs`: recomputing it per frame slid the
    # whole x-axis window forward with the wall clock.
    start_time = datetime.now(timezone.utc) - timedelta(days=7)
    while True:
        data = []
        for i in range(100):
            time = start_time + timedelta(hours=i * 2)
            data.append(
                {
                    # `isoformat()` on a tz-aware datetime already ends in
                    # "+00:00"; appending "Z" on top produced
                    # "...+00:00Z", which the Rust client's RFC 3339 parser
                    # rejects as trailing input, dropping every frame.
                    "time": time.isoformat().replace("+00:00", "Z"),
                    "y1": math.sin(i / 10 + cnt)
                    + random.uniform(-0.2, 0.2),  # Sine wave with random noise
                    "y2": math.sin(i / 5 + cnt) * 0.8
                    + random.uniform(-0.1, 0.1),  # Different frequency sine wave
                }
            )
        cnt += 1.0
        await websocket.send(json.dumps(data))
        await asyncio.sleep(SEND_INTERVAL_SECS)


async def main():
    async with websockets.serve(handle_client, "127.0.0.1", 8080):
        print("WebSocket server started on ws://127.0.0.1:8080")
        await asyncio.Future()  # run forever


if __name__ == "__main__":
    asyncio.run(main())
