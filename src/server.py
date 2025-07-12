import asyncio
import websockets
import json
from datetime import datetime, timedelta
import random
import math

async def handle_client(websocket):
    cnt=0.0
    while True:
        start_time = datetime.utcnow() - timedelta(days=7)
        data = []
        for i in range(100):
            time = start_time + timedelta(hours=i*2)
            data.append({
                "time": time.isoformat() + "Z",
                "y1": i+cnt,
                "y2": i+cnt
            })
        cnt+=1.0
        
        await websocket.send(json.dumps(data))
        await asyncio.sleep(1)

async def main():
    async with websockets.serve(handle_client, "localhost", 8080):
        print("WebSocket server started on ws://localhost:8080")
        await asyncio.Future()  # run forever

if __name__ == "__main__":
    asyncio.run(main())