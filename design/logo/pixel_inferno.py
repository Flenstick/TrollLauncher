import sys
PAL = {'O':'#2D060E','K':'#3C0812','D':'#520B18','M':'#660E1B','R':'#C6202D','L':'#E5705C','C':'#ECDCCB'}
W = H = 16
front = set()
for x in range(1, 14):
    for y in range(1, 6):
        front.add((x, y))
for x in range(5, 10):
    for y in range(6, 14):
        front.add((x, y))
depth = {(x + 1, y + 1) for x, y in front} - front
shape = front | depth
grid = {}
for p in depth:
    grid[p] = 'D'
for x, y in front:
    c = 'R'
    if (x, y - 1) not in front:
        c = 'L'
    elif (x - 1, y) not in front:
        c = 'L'
    elif (x + 1, y) not in front or (x, y + 1) not in front:
        c = 'M'
    grid[(x, y)] = c
for x in (5, 9):
    for y in range(2, 6):
        grid[(x, y)] = 'M'
for x in range(6, 9):
    grid[(x, 9)] = 'M'
for x in range(2, 5):
    grid[(x, 2)] = 'C'
grid[(2, 3)] = 'C'
grid[(6, 7)] = 'C'
outline = set()
for x, y in shape:
    for dx, dy in ((1,0),(-1,0),(0,1),(0,-1),(1,1),(-1,-1),(1,-1),(-1,1)):
        q = (x + dx, y + dy)
        if q not in shape and 0 <= q[0] < W and 0 <= q[1] < H:
            outline.add(q)
def convex_corner(q):
    x, y = q
    n = lambda a, b: (a, b) in shape
    orth = n(x+1,y) or n(x-1,y) or n(x,y+1) or n(x,y-1)
    return not orth
for q in outline:
    if not convex_corner(q):
        grid[q] = 'O'
rows = [''.join(grid.get((x, y), '.') for x in range(W)) for y in range(H)]
print('\n'.join(rows))
rects = ''.join(f'<rect x="{x}" y="{y}" width="1" height="1" fill="{PAL[c]}"/>'
                for y, row in enumerate(rows) for x, c in enumerate(row) if c != '.')
open(sys.argv[1], 'w').write(f'<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" shape-rendering="crispEdges">{rects}</svg>\n')
