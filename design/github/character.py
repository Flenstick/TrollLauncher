# Renders a Minecraft player from a skin PNG with red rim light on a transparent background.
#   blender -b -P design/github/character.py -- skin.png out.png [--pose wave|walk|idle]
#       [--slim] [--glow mask.png] [--yaw 25] [--size 1200x1600]
import math
import sys

import bpy
from mathutils import Euler, Vector

ARGS = sys.argv[sys.argv.index("--") + 1:]
SKIN, OUT = ARGS[0], ARGS[1]


def opt(name, default):
    return ARGS[ARGS.index(name) + 1] if name in ARGS else default


POSE = opt("--pose", "wave")
SLIM = "--slim" in ARGS
GLOW = opt("--glow", None)
YAW = float(opt("--yaw", "25"))
WIDTH, HEIGHT = (int(v) for v in opt("--size", "1200x1600").split("x"))
S = 0.1

bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene


def material():
    m = bpy.data.materials.new("Skin")
    m.use_nodes = True
    nodes, links = m.node_tree.nodes, m.node_tree.links
    bsdf = nodes["Principled BSDF"]
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = bpy.data.images.load(SKIN)
    tex.interpolation = "Closest"
    links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    links.new(tex.outputs["Alpha"], bsdf.inputs["Alpha"])
    bsdf.inputs["Roughness"].default_value = 0.7
    if GLOW:
        mask = nodes.new("ShaderNodeTexImage")
        mask.image = bpy.data.images.load(GLOW)
        mask.image.colorspace_settings.name = "Non-Color"
        mask.interpolation = "Closest"
        bsdf.inputs["Emission Color"].default_value = (1.0, 0.02, 0.0, 1)
        mul = nodes.new("ShaderNodeMath")
        mul.operation = "MULTIPLY"
        mul.inputs[1].default_value = 9.0
        links.new(mask.outputs["Color"], mul.inputs[0])
        links.new(mul.outputs[0], bsdf.inputs["Emission Strength"])
    return m


MAT = material()


def box(name, uv, size, origin, pivot, inflate=0.0):
    """A skin box. `uv`=(u, v), `size`=(w, h, d) in skin pixels, `origin`=min corner (x, y, z)."""
    u, v = uv
    w, h, d = size
    x0, y0, z0 = (c - inflate for c in origin)
    x1, y1, z1 = origin[0] + w + inflate, origin[1] + d + inflate, origin[2] + h + inflate
    faces = [
        # corners BL, BR, TR, TL as seen from outside; texture rect (px, py, pw, ph)
        ([(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1)], (u + d, v + d, w, h)),
        ([(x1, y1, z0), (x0, y1, z0), (x0, y1, z1), (x1, y1, z1)], (u + 2 * d + w, v + d, w, h)),
        ([(x0, y1, z0), (x0, y0, z0), (x0, y0, z1), (x0, y1, z1)], (u, v + d, d, h)),
        ([(x1, y0, z0), (x1, y1, z0), (x1, y1, z1), (x1, y0, z1)], (u + d + w, v + d, d, h)),
        ([(x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)], (u + d, v, w, d)),
        ([(x0, y1, z0), (x1, y1, z0), (x1, y0, z0), (x0, y0, z0)], (u + d + w, v, w, d)),
    ]
    verts, polys, uvs = [], [], []
    for corners, (px, py, pw, ph) in faces:
        base = len(verts)
        verts += [Vector(c) * S for c in corners]
        polys.append((base, base + 1, base + 2, base + 3))
        uvs += [(px / 64, 1 - (py + ph) / 64), ((px + pw) / 64, 1 - (py + ph) / 64),
                ((px + pw) / 64, 1 - py / 64), (px / 64, 1 - py / 64)]
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata([tuple(v) for v in verts], [], polys)
    layer = mesh.uv_layers.new()
    for i, loop in enumerate(mesh.loops):
        layer.data[i].uv = uvs[loop.vertex_index]
    mesh.materials.append(MAT)
    obj = bpy.data.objects.new(name, mesh)
    scene.collection.objects.link(obj)
    # move origin to the pivot so rotations bend at joints
    obj.data.transform(__import__("mathutils").Matrix.Translation(-Vector(pivot) * S))
    obj.location = Vector(pivot) * S
    return obj


arm = 3 if SLIM else 4
parts = {
    "head": [((0, 0), (8, 8, 8), (-4, -4, 24), 0), ((32, 0), (8, 8, 8), (-4, -4, 24), .5)],
    "body": [((16, 16), (8, 12, 4), (-4, -2, 12), 0), ((16, 32), (8, 12, 4), (-4, -2, 12), .25)],
    "arm_r": [((40, 16), (arm, 12, 4), (-4 - arm, -2, 12), 0), ((40, 32), (arm, 12, 4), (-4 - arm, -2, 12), .25)],
    "arm_l": [((32, 48), (arm, 12, 4), (4, -2, 12), 0), ((48, 48), (arm, 12, 4), (4, -2, 12), .25)],
    "leg_r": [((0, 16), (4, 12, 4), (-4, -2, 0), 0), ((0, 32), (4, 12, 4), (-4, -2, 0), .25)],
    "leg_l": [((16, 48), (4, 12, 4), (0, -2, 0), 0), ((0, 48), (4, 12, 4), (0, -2, 0), .25)],
}
pivots = {
    "head": (0, 0, 24), "body": (0, 0, 12),
    "arm_r": (-4 - arm / 2, 0, 22), "arm_l": (4 + arm / 2, 0, 22),
    "leg_r": (-2, 0, 12), "leg_l": (2, 0, 12),
}
poses = {
    "wave": {"head": (8, 0, -12), "arm_r": (0, 0, 0), "arm_l": (0, 0, 0),
             "leg_r": (14, 0, 2), "leg_l": (-14, 0, -2)},
    "walk": {"head": (5, 0, -10), "arm_r": (-30, 0, 0), "arm_l": (30, 0, 0),
             "leg_r": (28, 0, 0), "leg_l": (-28, 0, 0)},
    "idle": {"head": (4, 0, -8), "arm_r": (-6, 0, -4), "arm_l": (6, 0, 4),
             "leg_r": (4, 0, 0), "leg_l": (-4, 0, 0)},
}
pose = poses[POSE]
rig = bpy.data.objects.new("Rig", None)
scene.collection.objects.link(rig)
for name, boxes in parts.items():
    for index, (uv, size, origin, inflate) in enumerate(boxes):
        obj = box(f"{name}{index}", uv, size, origin, pivots[name], inflate)
        obj.parent = rig
        if name in pose:
            obj.rotation_euler = Euler([math.radians(a) for a in pose[name]])
if POSE == "wave":
    # left arm raised high and swung outwards
    for obj in rig.children:
        if obj.name.startswith("arm_l"):
            obj.rotation_euler = Euler((math.radians(-10), math.radians(-150), math.radians(-10)))
rig.rotation_euler = Euler((0, 0, math.radians(YAW)))

# camera: slightly low hero angle
cam_data = bpy.data.cameras.new("Camera")
cam_data.lens = 70
cam = bpy.data.objects.new("Camera", cam_data)
scene.collection.objects.link(cam)
cam.location = Vector((0, -9.5, 1.1))
target = Vector((0, 0, 1.75))
cam.rotation_euler = (target - cam.location).to_track_quat("-Z", "Y").to_euler()
scene.camera = cam


def light(name, kind, location, energy, color, size=1.0):
    data = bpy.data.lights.new(name, kind)
    data.energy = energy
    data.color = color
    if kind == "AREA":
        data.size = size
    obj = bpy.data.objects.new(name, data)
    obj.location = location
    obj.rotation_euler = (Vector((0, 0, 1.6)) - Vector(location)).to_track_quat("-Z", "Y").to_euler()
    scene.collection.objects.link(obj)


light("Key", "AREA", (-3.5, -5, 4.5), 900, (1.0, 0.93, 0.88), 3)
light("Fill", "AREA", (4, -4, 1.5), 180, (0.75, 0.8, 1.0), 4)
light("RimL", "AREA", (-2.2, 4.5, 3.2), 5000, (1.0, 0.12, 0.12), 1.5)
light("RimR", "AREA", (2.4, 4.5, 2.4), 5000, (1.0, 0.18, 0.15), 1.5)

world = bpy.data.worlds.new("World")
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.02, 0.01, 0.012, 1)
world.node_tree.nodes["Background"].inputs[1].default_value = 0.6
scene.world = world

scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x, scene.render.resolution_y = WIDTH, HEIGHT
scene.render.film_transparent = True
scene.render.image_settings.file_format = "PNG"
scene.render.image_settings.color_mode = "RGBA"
scene.view_settings.view_transform = "AgX"
scene.view_settings.look = "AgX - Punchy"
scene.render.filepath = OUT
bpy.ops.render.render(write_still=True)
