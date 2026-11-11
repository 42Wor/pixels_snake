import socket
import json
import time
import random

class SnakeEnv:
    def __init__(self, host="127.0.0.1", port=9999):
        print(f"Connecting to Rust Game at {host}:{port}...")
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect((host, port))
        self.file = self.sock.makefile('r')
        print("Connected successfully!")

    def step(self, action: int, render: bool = False):
        """
        action: 0 = Continue Straight, 1 = Turn Right, 2 = Turn Left
        render: True = Watch visually (human speed), False = Turbo (pure calculation)
        """
        cmd = {"action": action, "render": render, "reset": False}
        self.sock.sendall((json.dumps(cmd) + "\n").encode())
        
        line = self.file.readline()
        data = json.loads(line)
        return data["state"], data["reward"], data["done"], data["score"]

    def reset(self):
        cmd = {"action": 0, "render": False, "reset": True}
        self.sock.sendall((json.dumps(cmd) + "\n").encode())
        line = self.file.readline()
        data = json.loads(line)
        return data["state"]

    def close(self):
        self.sock.close()


if __name__ == "__main__":
    env = SnakeEnv()

    # ==========================================================
    # 1. TURBO MODE DEMO: Train 500 steps silently in < 0.1 sec!
    # ==========================================================
    print("\n--- Running TURBO MODE (render=False) ---")
    start_time = time.time()
    env.reset()
    
    for step_num in range(500):
        # Pick action (0: straight, 1: right, 2: left)
        action = random.choice([0, 0, 0, 1, 2])
        state, reward, done, score = env.step(action, render=False)
        if done:
            env.reset()

    elapsed = time.time() - start_time
    print(f"Turbo finished 500 steps in {elapsed:.3f} seconds! (Speed: {500/elapsed:.0f} steps/sec)")

    # ==========================================================
    # 2. WATCH MODE DEMO: Slow down and watch it live on screen!
    # ==========================================================
    print("\n--- Running WATCH MODE (render=True). Look at the game window! ---")
    state = env.reset()
    
    for _ in range(100):
        # A simple reflex rule: if danger straight, turn right!
        danger_straight = state[0]
        if danger_straight:
            action = 1  # Turn right
        else:
            action = 0  # Go straight
            
        state, reward, done, score = env.step(action, render=True)
        if done:
            print(f"Snake died with score: {score}. Resetting...")
            state = env.reset()

    env.close()
    print("Done!")