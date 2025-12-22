import asyncio
import json
import math
import random
from datetime import datetime, timedelta, timezone

import websockets


async def handle_client(websocket):
    cnt = 0.0
    while True:
        start_time = datetime.now(timezone.utc) - timedelta(days=7)
        data = []
        for i in range(100):
            time = start_time + timedelta(hours=i * 2)
            data.append(
                {
                    "time": time.isoformat() + "Z",
                    "y1": math.sin(i / 10 + cnt)
                    + random.uniform(-0.2, 0.2),  # Sine wave with random noise
                    "y2": math.sin(i / 5 + cnt) * 0.8
                    + random.uniform(-0.1, 0.1),  # Different frequency sine wave
                }
            )
        cnt += 1.0
        print("data.len=", len(data))
        await websocket.send(json.dumps(data))
        await asyncio.sleep(0.01)


async def main():
    async with websockets.serve(handle_client, "127.0.0.1", 8080):
        print("WebSocket server started on ws://127.0.0.1:8080")
        await asyncio.Future()  # run forever


if __name__ == "__main__":
    asyncio.run(main())
