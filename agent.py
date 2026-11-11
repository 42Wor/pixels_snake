import socket
import json
import heapq
import time

COLS = 60
ROWS = 60

class SnakeEnv:
    def __init__(self, host="127.0.0.1", port=9999):
        print(f"Connecting to Rust Game at {host}:{port}...")
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect((host, port))
        self.file = self.sock.makefile('r')
        print("Connected successfully!\n")

    def step(self, action: int, render: bool = True):
        cmd = {"action": action, "render": render, "reset": False}
        self.sock.sendall((json.dumps(cmd) + "\n").encode())
        line = self.file.readline()
        return json.loads(line)

    def reset(self):
        cmd = {"action": 0, "render": True, "reset": True}
        self.sock.sendall((json.dumps(cmd) + "\n").encode())
        line = self.file.readline()
        return json.loads(line)

    def close(self):
        self.sock.close()


# =====================================================================
# A* (A-Star) Pathfinding Algorithm
# =====================================================================

def manhattan_distance(p1, p2):
    """Heuristic h(n): Manhattan distance between two points."""
    return abs(p1[0] - p2[0]) + abs(p1[1] - p2[1])

def get_neighbors(pos):
    x, y = pos
    return [
        ((x, y - 1), "Up"),
        ((x, y + 1), "Down"),
        ((x - 1, y), "Left"),
        ((x + 1, y), "Right"),
    ]

def a_star_search(start, goal, obstacles, snake_body):
    """
    Finds the shortest collision-free path from start to goal.
    Returns: List of coordinates [(x1,y1), (x2,y2), ...] to the target.
    """
    # Build impassable set (walls, obstacles, and snake's own body)
    blocked = set(obstacles) | set(snake_body)
    
    # Priority Queue stores: (f_score, tie_breaker_counter, current_node)
    count = 0
    open_set = []
    heapq.heappush(open_set, (0, count, start))
    
    came_from = {}
    g_score = {start: 0}

    while open_set:
        _, _, current = heapq.heappop(open_set)

        if current == goal:
            # Reconstruct path backwards from goal to start
            path = []
            while current in came_from:
                path.append(current)
                current = came_from[current]
            path.reverse()
            return path

        for neighbor, _ in get_neighbors(current):
            nx, ny = neighbor

            # Check border bounds (1 to COLS-2, 1 to ROWS-2)
            if nx <= 0 or nx >= COLS - 1 or ny <= 0 or ny >= ROWS - 1:
                continue

            # Check if tile is an obstacle or snake body (unless it's the goal food)
            if neighbor in blocked and neighbor != goal:
                continue

            tentative_g = g_score[current] + 1

            if tentative_g < g_score.get(neighbor, float('inf')):
                came_from[neighbor] = current
                g_score[neighbor] = tentative_g
                f_score = tentative_g + manhattan_distance(neighbor, goal)
                count += 1
                heapq.heappush(open_set, (f_score, count, neighbor))

    return None  # No safe path found (e.g., trapped)


def pick_survival_move(head, obstacles, snake_body):
    """Fallback: If trapped with no path to food, find ANY valid open tile."""
    blocked = set(obstacles) | set(snake_body)
    for neighbor, _ in get_neighbors(head):
        nx, ny = neighbor
        if 0 < nx < COLS - 1 and 0 < ny < ROWS - 1 and neighbor not in blocked:
            return neighbor
    return None


def to_relative_action(current_dir, target_pos, head):
    """
    Converts a target coordinate into relative actions:
    0 = Straight, 1 = Turn Right, 2 = Turn Left
    """
    tx, ty = target_pos
    hx, hy = head

    # Determine desired cardinal direction
    if tx > hx:   desired = "Right"
    elif tx < hx: desired = "Left"
    elif ty > hy: desired = "Down"
    else:         desired = "Up"

    if desired == current_dir:
        return 0  # Continue Straight

    turn_right_map = {"Up": "Right", "Right": "Down", "Down": "Left", "Left": "Up"}
    if turn_right_map[current_dir] == desired:
        return 1  # Turn 90° Clockwise (Right)
    
    return 2      # Turn 90° Counter-Clockwise (Left)


# =====================================================================
# Main Loop: Pathfinding Agent in Action
# =====================================================================

if __name__ == "__main__":
    env = SnakeEnv()
    data = env.reset()

    print("Pathfinding Agent active! Watch the snake navigate around obstacles...")

    while True:
        if data["done"]:
            print(f"Round finished! Final Score: {data['score']}. Restarting...")
            time.sleep(0.1)
            data = env.reset()

        head = tuple(data["head"])
        food = tuple(data["food"])
        obstacles = [tuple(p) for p in data["obstacles"]]
        snake = [tuple(p) for p in data["snake"]]
        current_dir = data["dir"]

        # 1. Run A* Search to find the shortest route to the apple
        path = a_star_search(head, food, obstacles, snake)

        if path and len(path) > 0:
            next_tile = path[0]
        else:
            # Fallback survival move if the apple is temporarily boxed in
            next_tile = pick_survival_move(head, obstacles, snake)
            if not next_tile:
                next_tile = head  # unavoidable collision

        # 2. Convert coordinate to relative steering
        action = to_relative_action(current_dir, next_tile, head)

        # 3. Step forward and render
        data = env.step(action, render=True)