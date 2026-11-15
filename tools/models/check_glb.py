#!/usr/bin/env python3
"""Checks the exported models against the game's conventions (stdlib only).

    python3 tools/models/check_glb.py   # checks assets/models/{kit,blaster,grenade,avatars,reload,powerups,
                                        #   vehicles,props,warpgate,weapons}.glb

kit.glb: every piece of tools/models/kit_dims.json is a mesh named exactly the piece name whose
POSITION extent (union over primitives, read from the binary buffer) equals the piece bounds
within +-0.025 m, protruding at most 0.02 m beyond them, and whose `body` primitives alone also
fill the bounds. All meshes: NORMAL and TEXCOORD_0 on every primitive, only known material
names. avatars.glb: the 9 heads (each with its two eye nodes), the shared eyeball and eyelid,
torso and both hands by exact mesh name, only the avatar
materials, hit_1..3 on the torso, and size sanity (heads within +-0.35 m of the eye point in X/Z
and at most 0.6 m tall, torso top at y = 0, hands within 0.15 m of the palm centre, body segments
hanging their length down -Y from the joint); the eyeball has `eyes` and `sclera`, every part
`flow`. Each animal's tail: pieces `tail_<animal>_<k>` from k = 0, each with its joint node
`tail_<animal>_j<k>` and one more at the tip, reaching along -Z from the joint to the next one
(within 0.12 m in X and Y of it). reload.glb: one mesh `reload_station` inside its
2 m cell (|x|, |z| <= 1), on the floor, with nothing between 0.04 m and 2.25 m (walkable).
Halo materials (`glow`, `eye_glow`, `halo_white`) and `flow` must have UV V within 0..1 and
`flow` UV U >= 0 (arc length). structures.glb: every structure of tools/models/structures.json
is a mesh by its name whose body fills the solids' bounds (x, z within +-0.025 m, standing on
the floor up to its highest solid) and whose accents stand at most 0.02 m (lamps: their
housings, 0.5 m) beyond them; only `body`, `accent`, `glow` and `lamp`. vehicles.glb,
props.glb, warpgate.glb, weapons.glb (`PARTS`): exactly the listed meshes, each under its
triangle budget and inside its box (hull hit boxes, pivot frames, pad radii and heights,
weapon lengths), reaching where its tip must be (barrel muzzles, the VTOL's skids on y = 0),
the listed nodes at their positions, and the warpgate's pillars matching the colliders.
Exits non-zero on any mismatch.
"""

import json
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
TOL = 0.025
PROUD = 0.02

COMPONENTS = {5120: "b", 5121: "B", 5122: "h", 5123: "H", 5125: "I", 5126: "f"}
WIDTH = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}


def load_glb(path):
    data = open(path, "rb").read()
    magic, version, total = struct.unpack_from("<4sII", data, 0)
    if magic != b"glTF" or version != 2:
        raise ValueError("%s: not a glTF 2 binary" % path)
    off, gltf, binary = 12, None, b""
    while off < total:
        clen, ctype = struct.unpack_from("<II", data, off)
        chunk = data[off + 8: off + 8 + clen]
        if ctype == 0x4E4F534A:
            gltf = json.loads(chunk.decode("utf-8"))
        elif ctype == 0x004E4942:
            binary = chunk
        off += 8 + clen
    return gltf, binary


def read_accessor(gltf, binary, index):
    acc = gltf["accessors"][index]
    view = gltf["bufferViews"][acc["bufferView"]]
    fmt = COMPONENTS[acc["componentType"]]
    n = WIDTH[acc["type"]]
    size = struct.calcsize(fmt)
    stride = view.get("byteStride", size * n)
    base = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    return [struct.unpack_from("<%d%s" % (n, fmt), binary, base + i * stride)
            for i in range(acc["count"])]


def extent(points):
    mn = [min(p[i] for p in points) for i in range(3)]
    mx = [max(p[i] for p in points) for i in range(3)]
    return mn, mx


def tri_count(gltf, prim):
    if "indices" in prim:
        return gltf["accessors"][prim["indices"]]["count"] // 3
    return gltf["accessors"][prim["attributes"]["POSITION"]]["count"] // 3


class Checker:
    def __init__(self):
        self.errors = []

    def err(self, msg):
        self.errors.append(msg)
        print("FAIL", msg)


def check_common(ck, label, gltf, binary, allowed):
    mats = [m.get("name") for m in gltf.get("materials", [])]
    meshes = {}
    for mesh in gltf.get("meshes", []):
        pts_all, body, tris = [], [], 0
        for prim in mesh["primitives"]:
            attrs = prim["attributes"]
            for a in ("POSITION", "NORMAL", "TEXCOORD_0"):
                if a not in attrs:
                    ck.err("%s/%s: primitive without %s" % (label, mesh["name"], a))
            mname = mats[prim["material"]] if "material" in prim else None
            if mname not in allowed:
                ck.err("%s/%s: unexpected material %r" % (label, mesh["name"], mname))
            pts = read_accessor(gltf, binary, attrs["POSITION"])
            acc = gltf["accessors"][attrs["POSITION"]]
            mn, mx = extent(pts)
            if "min" in acc and any(abs(a - b) > 1e-4 for a, b in zip(acc["min"] + acc["max"], mn + mx)):
                ck.err("%s/%s: accessor min/max disagree with the data" % (label, mesh["name"]))
            pts_all += pts
            if mname == "body":
                body += pts
            tris += tri_count(gltf, prim)
        meshes[mesh["name"]] = (pts_all, body, tris)
    return meshes


HALOS = {"glow", "eye_glow", "halo_white"}


def check_uvs(ck, label, gltf, binary):
    mats = [m.get("name") for m in gltf.get("materials", [])]
    for mesh in gltf.get("meshes", []):
        for prim in mesh["primitives"]:
            mname = mats[prim["material"]] if "material" in prim else None
            if mname not in HALOS | {"flow"}:
                continue
            uv = read_accessor(gltf, binary, prim["attributes"]["TEXCOORD_0"])
            if any(v < -1e-4 or v > 1 + 1e-4 for _, v in uv):
                ck.err("%s/%s: %s UV V outside 0..1" % (label, mesh["name"], mname))
            if mname == "flow" and any(u < -1e-4 for u, _ in uv):
                ck.err("%s/%s: flow UV U (arc length) negative" % (label, mesh["name"]))


def check_kit(ck):
    dims = json.load(open(os.path.join(HERE, "kit_dims.json")))
    gltf, binary = load_glb(os.path.join(ROOT, "assets/models/kit.glb"))
    meshes = check_common(ck, "kit", gltf, binary, {"body", "accent", "ceiling_accent", "glow"})
    for name, piece in dims["pieces"].items():
        if name not in meshes:
            ck.err("kit: no mesh named %r" % name)
            continue
        pts, body, tris = meshes[name]
        bmn, bmx = piece["bounds"]
        mn, mx = extent(pts)
        ok = True
        for ax in range(3):
            for got, want, sign in ((mn[ax], bmn[ax], -1), (mx[ax], bmx[ax], 1)):
                if abs(got - want) > TOL or sign * (got - want) > PROUD + 1e-6:
                    ck.err("kit/%s: axis %s extent %.4f vs bounds %.4f" % (name, "xyz"[ax], got, want))
                    ok = False
        if not body:
            ck.err("kit/%s: no body primitive" % name)
        else:
            mn, mx = extent(body)
            for ax in range(3):
                for got, want in ((mn[ax], bmn[ax]), (mx[ax], bmx[ax])):
                    if abs(got - want) > TOL:
                        ck.err("kit/%s: body does not fill bounds on %s (%.4f vs %.4f)"
                               % (name, "xyz"[ax], got, want))
                        ok = False
        print("%-4s kit/%-14s %5d tris" % ("ok" if ok else "BAD", name, tris))
    # Glow fade direction: on floor_tile the halo starts (V = 0) at the edge lines.
    mesh = next((m for m in gltf["meshes"] if m["name"] == "floor_tile"), None)
    mats = [m.get("name") for m in gltf.get("materials", [])]
    for prim in (mesh or {"primitives": []})["primitives"]:
        if mats[prim["material"]] != "glow":
            continue
        pos = read_accessor(gltf, binary, prim["attributes"]["POSITION"])
        uv = read_accessor(gltf, binary, prim["attributes"]["TEXCOORD_0"])
        for p, t in zip(pos, uv):
            edge = max(abs(p[0]), abs(p[2])) > 0.985
            if edge and t[1] > 0.01 and min(abs(p[0]), abs(p[2])) > 0.985:
                ck.err("kit/floor_tile: glow V is %.2f at the edge line (want 0)" % t[1])
                break


def check_model(ck, file, allowed, required, nodes=()):
    path = os.path.join(ROOT, "assets/models", file)
    if not os.path.exists(path):
        ck.err("%s missing" % file)
        return
    gltf, binary = load_glb(path)
    meshes = check_common(ck, file, gltf, binary, allowed)
    used = {gltf["materials"][p["material"]]["name"]
            for m in gltf["meshes"] for p in m["primitives"] if "material" in p}
    for r in required:
        if r not in used:
            ck.err("%s: material %r not used" % (file, r))
    names = {n.get("name"): n for n in gltf.get("nodes", [])}
    for n in nodes:
        if n not in names:
            ck.err("%s: no node %r" % (file, n))
        else:
            print("     %s node %s at %s" % (file, n, names[n].get("translation", [0, 0, 0])))
    for name, (pts, _, tris) in meshes.items():
        mn, mx = extent(pts)
        print("ok   %s/%s %d tris, extent %s .. %s" % (file, name, tris,
              [round(v, 3) for v in mn], [round(v, 3) for v in mx]))


AVATAR_HEADS = ("wolf", "cat", "deer", "fox", "bear", "rabbit", "owl", "boar", "dragon")
# Body segments: (name, nominal length down -Y from the joint at the origin).
AVATAR_SEGMENTS = (("neck", 0.2), ("pelvis", 0.12), ("upper_arm", 0.27), ("forearm", 0.25),
                   ("thigh", 0.38), ("shin", 0.37), ("boot", 0.08))


def check_avatars(ck):
    path = os.path.join(ROOT, "assets/models/avatars.glb")
    if not os.path.exists(path):
        ck.err("avatars.glb missing")
        return
    gltf, binary = load_glb(path)
    allowed = {"body", "fur_light", "accent", "glow", "hit_1", "hit_2", "hit_3", "eyes",
               "eye_glow", "flow", "sclera", "glow_white"}
    meshes = check_common(ck, "avatars.glb", gltf, binary, allowed)
    check_uvs(ck, "avatars.glb", gltf, binary)
    mats = [m.get("name") for m in gltf.get("materials", [])]
    used = {m["name"]: {mats[p["material"]] for p in m["primitives"] if "material" in p}
            for m in gltf["meshes"]}
    heads = ["head_" + h for h in AVATAR_HEADS]
    segments = [n for n, _ in AVATAR_SEGMENTS]
    nodes = {n.get("name"): n for n in gltf.get("nodes", [])}
    eyeballs = ["eyeball"] + sorted(m for m in meshes if m.startswith("eyeball_"))
    for name in heads + eyeballs + ["eyelid", "torso"] + HANDS + segments:
        if name not in meshes:
            ck.err("avatars.glb: no mesh named %r" % name)
            continue
        pts, _, tris = meshes[name]
        mn, mx = extent(pts)
        ok = True

        def bad(msg):
            nonlocal ok
            ck.err("avatars.glb/%s: %s" % (name, msg))
            ok = False
        need = {"body", "accent", "flow"}
        seg = dict(AVATAR_SEGMENTS).get(name)
        if name == "torso":
            need |= {"hit_1", "hit_2", "hit_3"}
        elif name.startswith("eyeball"):
            need = {"body", "eyes", "sclera"}
        elif name == "eyelid":
            need = {"body", "accent"}
        for m in sorted(need - used[name]):
            bad("material %r not used" % m)
        if name.startswith("head_"):
            if max(abs(mn[0]), mx[0], abs(mn[2]), mx[2]) > 0.35:
                bad("extends more than 0.35 m from the eye point in X/Z")
            if mx[1] - mn[1] > 0.6:
                bad("%.3f m tall (max 0.6)" % (mx[1] - mn[1]))
            if not (mn[1] < 0 < mx[1] and mn[2] < 0 < mx[2]):
                bad("origin (eye point) is not inside the head's extent")
            # The eye frames: one each side, within the head, 2.5-6 cm eyeballs, looking ahead.
            for side, sx in (("l", -1.0), ("r", 1.0)):
                node = nodes.get("%s_eye_%s" % (name, side))
                if node is None:
                    bad("no node %s_eye_%s" % (name, side))
                    continue
                t = node.get("translation", [0, 0, 0])
                sc = node.get("scale", [1, 1, 1])
                if t[0] * sx < 0.02 or t[0] * sx > 0.1 or t[2] > 0.0 or abs(t[1]) > 0.001:
                    bad("eye %s at %s (want it beside the origin, ahead of it)" % (side, t))
                if max(sc) - min(sc) > 1e-5 or not 0.025 <= sc[0] <= 0.06:
                    bad("eye %s scale %s (want a uniform 2.5-6 cm radius)" % (side, sc))
        elif name.startswith("eyeball") or name == "eyelid":
            lim = 1.13 if name == "eyelid" else 1.03
            if max(max(abs(a), abs(b)) for a, b in zip(mn, mx)) > lim:
                bad("extends more than %.2f from the eye's centre (unit radius)" % lim)
        elif name == "torso":
            if abs(mx[1]) > 0.03:
                bad("top at y = %.3f (want 0, the neck point)" % mx[1])
            if mn[1] > -0.3:
                bad("does not hang down from the origin")
        elif seg is not None:
            if not (mn[1] < -seg + 0.03 and mx[1] < 0.06):
                bad("does not hang %.2f m down -Y from its joint" % seg)
            if max(abs(mn[0]), mx[0]) > 0.14 or max(abs(mn[2]), mx[2]) > 0.2:
                bad("wider than a body segment")
        else:
            if max(max(abs(a), abs(b)) for a, b in zip(mn, mx)) > 0.15:
                bad("extends more than 0.15 m from the palm centre")
        print("%-4s avatars.glb/%-12s %4d tris, extent %s .. %s, materials %s" % (
            "ok" if ok else "BAD", name, tris, [round(v, 3) for v in mn],
            [round(v, 3) for v in mx], ",".join(sorted(used[name]))))
    tails = []
    for animal in AVATAR_HEADS:
        k = 0
        while "tail_%s_%d" % (animal, k) in meshes:
            name = "tail_%s_%d" % (animal, k)
            tails.append(name)
            a = nodes.get("tail_%s_j%d" % (animal, k))
            b = nodes.get("tail_%s_j%d" % (animal, k + 1))
            pts, _, tris = meshes[name]
            mn, mx = extent(pts)
            ok = a is not None and b is not None
            if not ok:
                ck.err("avatars.glb/%s: no joint nodes j%d and j%d" % (name, k, k + 1))
            else:
                ta, tb = a.get("translation", [0, 0, 0]), b.get("translation", [0, 0, 0])
                piece = sum((u - v) ** 2 for u, v in zip(ta, tb)) ** 0.5
                if not (mn[2] < -0.6 * piece and mx[2] < 0.06):
                    ck.err("avatars.glb/%s: does not reach %.2f m along -Z" % (name, piece))
                    ok = False
                if max(abs(mn[0]), mx[0], abs(mn[1]), mx[1]) > 0.12 or mn[2] < -piece - 0.12:
                    ck.err("avatars.glb/%s: wider or longer than its piece" % name)
                    ok = False
            print("%-4s avatars.glb/%-12s %4d tris, extent %s .. %s, materials %s" % (
                "ok" if ok else "BAD", name, tris, [round(v, 3) for v in mn],
                [round(v, 3) for v in mx], ",".join(sorted(used[name]))))
            k += 1
        if k == 0:
            ck.err("avatars.glb: no tail for %s" % animal)
    extra = set(meshes) - set(heads) - set(segments) - set(eyeballs) - set(tails) - {
        "torso", "eyelid"} - set(HANDS)
    if extra:
        ck.err("avatars.glb: unexpected meshes %s" % sorted(extra))


HANDS = ["hand_left", "hand_right", "hand_grip_left", "hand_grip_right", "hand_point_left",
         "hand_point_right"]


def check_reload(ck):
    path = os.path.join(ROOT, "assets/models/reload.glb")
    if not os.path.exists(path):
        ck.err("reload.glb missing")
        return
    gltf, binary = load_glb(path)
    meshes = check_common(ck, "reload.glb", gltf, binary, {"body", "glow_white", "halo_white"})
    check_uvs(ck, "reload.glb", gltf, binary)
    mats = [m.get("name") for m in gltf.get("materials", [])]
    if set(meshes) != {"reload_station"}:
        ck.err("reload.glb: want exactly one mesh 'reload_station', got %s" % sorted(meshes))
    for mesh in gltf["meshes"]:
        used = {mats[p["material"]] for p in mesh["primitives"] if "material" in p}
        for m in sorted({"body", "glow_white", "halo_white"} - used):
            ck.err("reload.glb/%s: material %r not used" % (mesh["name"], m))
        pts, _, tris = meshes[mesh["name"]]
        mn, mx = extent(pts)
        ok = True
        if max(abs(mn[0]), mx[0], abs(mn[2]), mx[2]) > 1.0 + 1e-4:
            ck.err("reload.glb/%s: leaves its 2 m cell" % mesh["name"])
            ok = False
        if mn[1] < -1e-4:
            ck.err("reload.glb/%s: below the floor (y = %.3f)" % (mesh["name"], mn[1]))
            ok = False
        blocking = [p for p in pts if 0.04 + 1e-4 < p[1] < 2.25 - 1e-4]
        if blocking:
            ck.err("reload.glb/%s: %d vertices between 0.04 m and 2.25 m, e.g. %s"
                   % (mesh["name"], len(blocking), [round(v, 3) for v in blocking[0]]))
            ok = False
        print("%-4s reload.glb/%s %d tris, extent %s .. %s, materials %s" % (
            "ok" if ok else "BAD", mesh["name"], tris, [round(v, 3) for v in mn],
            [round(v, 3) for v in mx], ",".join(sorted(used))))


def check_structures(ck):
    data = json.load(open(os.path.join(HERE, "structures.json")))
    path = os.path.join(ROOT, "assets/models/structures.glb")
    if not os.path.exists(path):
        ck.err("structures.glb missing")
        return
    gltf, binary = load_glb(path)
    meshes = check_common(ck, "structures", gltf, binary, {"body", "accent", "glow", "lamp"})
    check_uvs(ck, "structures", gltf, binary)
    for name, spec in data.items():
        if name not in meshes:
            ck.err("structures: no mesh named %r" % name)
            continue
        pts, body, tris = meshes[name]
        corners = [(x, s["y"][0], z) for s in spec["solids"] for x, z in s["poly"]]
        corners += [(x, s["y"][1], z) for s in spec["solids"] for x, z in s["poly"]]
        want_mn, want_mx = extent(corners)
        ok = True
        bmn, bmx = extent(body)
        for ax in range(3):
            if abs(bmn[ax] - want_mn[ax]) > TOL or abs(bmx[ax] - want_mx[ax]) > TOL:
                # (Lamp housings are body too, above the parapets they stand on.)
                if not (ax == 1 and spec["lamps"] and bmx[1] <= want_mx[1] + 0.5):
                    ck.err("structures/%s: body extent on %s %.3f..%.3f vs solids %.3f..%.3f"
                           % (name, "xyz"[ax], bmn[ax], bmx[ax], want_mn[ax], want_mx[ax]))
                    ok = False
        mn, mx = extent(pts)
        slack = 0.5 if spec["lamps"] else PROUD
        if any(mn[ax] < want_mn[ax] - slack or mx[ax] > want_mx[ax] + slack for ax in range(3)):
            ck.err("structures/%s: accents stand out of its solids (%s .. %s)" % (name, mn, mx))
            ok = False
        print("%-4s structures/%-14s %6d tris" % ("ok" if ok else "BAD", name, tris))
    extra = set(meshes) - set(data)
    if extra:
        ck.err("structures.glb: unexpected meshes %s" % sorted(extra))


# ---------------------------------------------------------------------------------------------
# Vehicles, props, warpgate, weapons: mesh name -> (max tris, box min, box max, reach), where
# reach is {axis: value} a vertex must come within REACH of (barrel tips, the ground).
# Boxes are inclusive with BOX_TOL slack. A fifth element EXACT marks a collider box: the `body`
# primitives must fill it exactly (within 2 mm) and nothing may stand more than EXACT_TOL
# beyond it (accents on the faces).

BOX_TOL = 0.03
EXACT = "exact"
EXACT_TOL = 0.012
REACH = 0.03
INF = 1e9


def _hull(hx, hy, hz, slack=0.1):
    return ((-hx - slack, -0.001, -hz - slack), (hx + slack, 2 * hy + slack, hz + slack))


# The plain surfaces any model may use besides `body` (README.md, Materials).
SURFACES = {"armor", "paint", "metal", "rubber", "glass", "heat", "polymer"}

PARTS = {
    "vehicles.glb": {
        "allowed": {"body", "accent", "glow", "glow_white", "halo_white", "flow", "tread"} | SURFACES,
        # Each material of a mesh is an entity and a draw call of its own.
        "max_materials": 10,
        "required": ("body", "accent", "glow", "glow_white", "flow"),
        "meshes": {
            "light_tank_hull": (4000,) + _hull(1.6, 1.25, 2.8) + ({1: 0.0},),
            "light_tank_turret": (4000, (-1.25, -0.3, -1.4), (1.25, 1.55, 1.6), {}),
            "light_tank_wheel": (4000, (-0.25, -0.32, -0.32), (0.25, 0.32, 0.32), {1: -0.31}),
            "light_tank_sprocket": (4000, (-0.25, -0.32, -0.32), (0.25, 0.32, 0.32), {1: -0.3}),
            "light_tank_barrel": (4000, (-0.5, -0.4, -3.2), (0.5, 0.4, 0.0), {2: -3.2}),
            "light_aaa_hull": (4000,) + _hull(1.6, 1.25, 2.8) + ({1: 0.0},),
            "light_aaa_turret": (4000, (-1.0, -0.3, -1.0), (1.0, 1.05, 1.0), {}),
            "light_aaa_barrel": (4000, (-0.75, -0.4, -2.4), (0.75, 0.4, 0.5), {2: -2.4}),
            "heavy_tank_hull": (4000,) + _hull(2.2, 1.5, 3.8) + ({1: 0.0},),
            "heavy_tank_turret": (4000, (-1.7, -0.25, -2.1), (1.7, 2.0, 2.3), {}),
            "heavy_tank_wheel": (4000, (-0.3, -0.39, -0.39), (0.3, 0.39, 0.39), {1: -0.38}),
            "heavy_tank_sprocket": (4000, (-0.3, -0.36, -0.36), (0.3, 0.36, 0.36), {1: -0.34}),
            "heavy_tank_barrel": (4000, (-0.6, -0.4, -4.4), (0.6, 0.4, 0.0), {2: -4.4}),
            "heavy_tank_mg_turret": (4000, (-0.5, -0.25, -0.5), (0.5, 0.4, 0.5), {}),
            "heavy_tank_mg": (4000, (-0.3, -0.17, -1.1), (0.3, 0.17, 0.35), {2: -1.1}),
            "spawn_bus_hull": (4000,) + _hull(1.7, 1.8, 5.0, 0.0) + ({1: 0.45},),
            "spawn_bus_wheel": (4000, (-0.25, -0.7, -0.7), (0.25, 0.7, 0.7), {1: -0.7}),
            "spawn_bus_mast": (4000, (-0.4, 0.0, -0.4), (0.4, 3.2, 0.4), {1: 0.0}),
            "vtol_body": (4000,) + _hull(3.6, 1.4, 4.2) + ({1: 0.0},),
            "vtol_engine": (4000, (-0.9, -0.8, -0.9), (0.9, 0.55, 0.9), {}),
            "harvester_hull": (4000,) + _hull(2.0, 1.8, 5.5, 0.0) + ({1: 0.5},),
            "harvester_wheel": (4000, (-0.3, -0.9, -0.9), (0.3, 0.9, 0.9), {1: -0.9}),
            "harvester_turret": (4000, (-0.9, -0.5, -0.9), (0.9, 0.6, 0.9), {1: -0.45}),
            "harvester_barrel": (4000, (-0.6, -0.6, -2.0), (0.6, 0.6, 0.5), {2: -2.0}),
            "rover_hull": (4000,) + _hull(1.25, 0.95, 2.3) + ({},),
            "rover_wheel": (4000, (-0.2, -0.5, -0.5), (0.2, 0.5, 0.5), {1: -0.5}),
            "rover_turret": (4000, (-0.4, -0.2, -0.3), (0.4, 0.5, 0.4), {}),
            "rover_barrel": (4000, (-0.2, -0.15, -1.1), (0.2, 0.15, 0.3), {2: -1.1}),
            "interceptor_hull": (4000,) + _hull(3.0, 1.1, 4.0) + ({},),
            "dropship_hull": (4000,) + _hull(4.2, 2.0, 7.0) + ({},),
            "dropship_turret": (4000, (-0.5, -0.45, -0.5), (0.5, 0.45, 0.5), {}),
            "dropship_barrel": (4000, (-0.2, -0.15, -1.3), (0.2, 0.15, 0.3), {2: -1.3}),
        },
        "nodes": {"light_tank_muzzle": (0, 0, -3.2), "light_aaa_muzzle": (0, 0, -2.4),
                  "heavy_tank_muzzle": (0, 0, -4.4), "heavy_tank_mg_muzzle": (0, 0, -1.1),
                  "vtol_muzzle_l": (-2.6, 0.8, -3.0), "vtol_muzzle_r": (2.6, 0.8, -3.0),
                  "harvester_muzzle": (0, 0, -2.0), "rover_muzzle": (0, 0, -1.1),
                  "dropship_muzzle": (0, 0, -1.3), "interceptor_muzzle_l": (-0.4, 0.9, -4.1),
                  "interceptor_muzzle_r": (0.4, 0.9, -4.1)},
    },
    "props.glb": {
        "allowed": {"body", "accent", "glow", "glow_white", "halo_white", "flow", "resource"},
        "required": ("body", "accent", "glow", "glow_white", "halo_white", "flow", "resource"),
        "meshes": {
            "terminal_equipment": (2000, (-0.5, 0, -0.25), (0.5, 1.9, 0.25), {1: 1.9}),
            "terminal_medical": (2000, (-0.5, 0, -0.25), (0.5, 1.9, 0.25), {1: 1.9}),
            "terminal_vehicle": (2000, (-0.5, 0, -0.25), (0.5, 1.9, 0.25), {1: 1.9}),
            "console_core": (2000, (-0.72, 0, -0.72), (0.72, 2.6, 0.72), {1: 0.0}),
            "turret_base": (2000, (-1.0, 0, -1.0), (1.0, 1.2, 1.0), {1: 1.2}),
            "turret_head": (2000, (-0.65, -0.4, -1.4), (0.65, 0.45, 0.6), {1: -0.4, 2: -1.4}),
            "vehicle_pad": (2000, (-7.0, 0, -7.0), (7.0, 0.06, 7.0), {}),
            "rearm_pad": (2000, (-9.6, 0, -9.6), (9.6, 3.1, 9.6), {1: 3.0}),
            "spawn_tube": (2000, (-0.75, 0, -0.75), (0.75, 2.45, 0.75), {1: 0.0}),
            "wall_turret_base": (2000, (-1.3, 0, -1.3), (1.3, 1.8, 1.3), {1: 1.8}),
            "wall_turret_head": (2000, (-1.35, -0.6, -2.6), (1.35, 0.8, 1.4),
                                 {1: -0.6, 2: -2.6}),
            "generator": (2000, (-2.0, 0, -2.0), (2.0, 3.5, 2.0), {1: 3.5}),
            "generator_core": (2000, (-0.8, 0.8, -0.8), (0.8, 2.4, 0.8), {1: 2.4}),
            "resource_silo": (2000, (-4.0, 0, -4.3), (4.0, 12.3, 4.0), {1: 12.0}),
            "resource_silo_fill": (2000, (-3.6, 0, -3.6), (3.6, 1.0, 3.6), {1: 1.0}),
            "resource_node": (2000, (-2.1, -0.3, -2.1), (2.1, 3.5, 2.1), {}),
            "spitfire_base": (2000, (-0.42, 0, -0.27), (0.42, 0.86, 0.46), {1: 0.86}),
            "spitfire_head": (2000, (-0.2, -0.04, -0.44), (0.2, 0.17, 0.16), {1: -0.04, 2: -0.44}),
            "at_mine": (2000, (-0.36, 0, -0.36), (0.36, 0.125, 0.36), {1: 0.12}),
            "ammo_pack": (2000, (-0.3, 0, -0.2), (0.3, 0.4, 0.2), {1: 0.4}),
            # Engineer construction: the collider boxes (crates/ps-sim build pieces).
            "build_wall": (2000, (-3.0, 0, -0.4), (3.0, 3.5, 0.4), {}, EXACT),
            "bastion_base": (2000, (-1.6, 0, -1.6), (1.6, 2.9, 1.6), {}, EXACT),
            "bastion_head": (2000, (-0.9, -0.4, -1.7), (0.9, 0.7, 0.9), {}, EXACT),
            "forward_tube": (2000, (-1.2, 0, -1.2), (1.2, 3.2, 1.2), {}, EXACT),
            "mobile_silo": (2000, (-1.8, 0, -1.8), (1.8, 4.5, 1.8), {}, EXACT),
            "mobile_silo_fill": (2000, (-1.37, 0, -1.37), (1.37, 1.0, 1.37), {1: 1.0}),
            "capture_pylon": (2000, (-0.85, 0, -0.85), (0.85, 4.0, 0.85), {1: 0.0}),
        },
        "nodes": {"turret_head_muzzle_l": (-0.5, 0.02, -1.4),
                  "turret_head_muzzle_r": (0.5, 0.02, -1.4),
                  "wall_turret_muzzle_l": (-0.55, 0.0, -2.6),
                  "wall_turret_muzzle_r": (0.55, 0.0, -2.6),
                  "spitfire_head_muzzle": (0.0, 0.0, -0.44),
                  "bastion_head_muzzle": (0.0, 0.0, -1.7)},
    },
    # The canyon caves' alien base equipment: in the frames and bounds of props.glb's.
    "alien.glb": {
        "allowed": {"body", "accent", "glow", "glow_white", "halo_white", "flow"},
        "required": ("body", "accent", "glow", "glow_white"),
        "meshes": {
            "alien_terminal_equipment": (2000, (-0.5, 0, -0.25), (0.5, 1.9, 0.25), {1: 1.9}),
            "alien_terminal_medical": (2000, (-0.5, 0, -0.25), (0.5, 1.9, 0.25), {1: 1.9}),
            "alien_terminal_vehicle": (2000, (-0.5, 0, -0.25), (0.5, 1.9, 0.25), {1: 1.9}),
            "alien_console_core": (2000, (-0.72, 0, -0.72), (0.72, 2.6, 0.72), {1: 0.0}),
            "alien_generator": (2000, (-2.0, 0, -2.0), (2.0, 3.5, 2.0), {1: 3.5}),
            "alien_generator_core": (2000, (-0.8, 0.8, -0.8), (0.8, 2.4, 0.8), {1: 2.4}),
            "alien_vehicle_pad": (2000, (-7.0, 0, -7.0), (7.0, 0.06, 7.0), {}),
            "alien_rearm_pad": (2000, (-9.6, 0, -9.6), (9.6, 3.1, 9.6), {1: 3.0}),
            "alien_spawn_tube": (2000, (-0.75, 0, -0.75), (0.75, 2.45, 0.75), {1: 0.0}),
            "alien_resource_silo": (2000, (-4.0, 0, -4.3), (4.0, 12.3, 4.0), {1: 12.3}),
            "alien_resource_silo_fill": (2000, (-3.6, 0, -3.6), (3.6, 1.0, 3.6), {1: 1.0}),
            "alien_wall_turret_base": (2000, (-1.3, 0, -1.3), (1.3, 1.8, 1.3), {1: 1.8}),
            "alien_wall_turret_head": (2000, (-1.4, -0.6, -2.6), (1.4, 0.8, 1.4),
                                       {1: -0.6, 2: -2.6}),
            "alien_capture_pylon": (2000, (-0.85, 0, -0.85), (0.85, 4.0, 0.85), {1: 0.0}),
        },
        "nodes": {},
    },
    "warpgate.glb": {
        "allowed": {"body", "accent", "glow", "glow_white", "halo_white", "flow"},
        "required": ("body", "accent", "glow"),
        "meshes": {
            "warpgate": (6000, (-45, -1, -45), (45, 55.05, 45), {1: -1.0}),
            "warpgate_core": (4000, (-14, 0, -14), (14, 50, 14), {1: 50.0}),
            "warpgate_shell": (4000, (-38, 0, -38), (38, 48, 38), {1: 48.0}),
            "warpgate_rings": (4000, (-23.5, 8, -23.5), (23.5, 40, 23.5), {}),
        },
        "nodes": {},
    },
    "weapons.glb": {
        "allowed": {"body", "accent", "glow", "glow_white", "halo_white"} | SURFACES,
        "max_materials": 8,
        "required": ("body", "accent", "glow_white"),
        "meshes": {
            "rifle": (2000, (-0.05, -0.1, -0.45), (0.05, 0.2, 0.3), {2: -0.43}),
            "scatter": (2000, (-0.07, -0.1, -0.4), (0.07, 0.2, 0.25), {2: -0.38}),
            "lance": (2000, (-0.05, -0.1, -0.7), (0.05, 0.2, 0.35), {2: -0.68}),
            "striker": (2000, (-0.12, -0.1, -0.65), (0.12, 0.25, 0.5), {2: -0.62}),
            "pistol": (2000, (-0.04, -0.1, -0.2), (0.04, 0.15, 0.07), {2: -0.19}),
            "medgun": (2000, (-0.08, -0.1, -0.3), (0.08, 0.2, 0.17), {2: -0.29}),
            "repair_gun": (2000, (-0.08, -0.1, -0.38), (0.08, 0.22, 0.2), {2: -0.37}),
            "skyhook": (2000, (-0.1, -0.1, -0.62), (0.1, 0.3, 0.5), {2: -0.6}),
            # The factions' rifles.
            "ember": (2000, (-0.05, -0.1, -0.42), (0.05, 0.2, 0.25), {2: -0.4}),
            "volt": (2000, (-0.05, -0.1, -0.49), (0.05, 0.2, 0.35), {2: -0.47}),
            "hive": (2000, (-0.06, -0.1, -0.44), (0.06, 0.2, 0.32), {2: -0.42}),
            # A MAX's arm guns: a cradle round the suit's hand in place of the grip, the
            # wrist's ball behind it. The pounder's barrel clusters turn (`pounder_spin`, about
            # its own axis, set at the `pounder_spin_N` nodes).
            "pounder": (2000, (-0.16, -0.1, -0.32), (0.16, 0.36, 0.23), {2: -0.31}),
            "pounder_spin": (2000, (-0.05, -0.05, -0.6), (0.05, 0.05, -0.3), {2: -0.6}),
            "mauler": (2000, (-0.15, -0.1, -0.63), (0.15, 0.33, 0.21), {2: -0.62}),
            "burster": (2000, (-0.12, -0.1, -1.0), (0.12, 0.38, 0.23), {2: -1.0}),
        },
        "nodes": {"rifle_muzzle": (0, 0.085, -0.43), "scatter_muzzle": (0, 0.088, -0.38),
                  "lance_muzzle": (0, 0.085, -0.68), "striker_muzzle": (0, 0.15, -0.62),
                  "pistol_muzzle": (0, 0.085, -0.19), "medgun_muzzle": (0, 0.09, -0.29),
                  "repair_gun_muzzle": (0, 0.09, -0.37), "skyhook_muzzle": (0, 0.15, -0.6),
                  "ember_muzzle": (0, 0.085, -0.4), "volt_muzzle": (0, 0.085, -0.47),
                  "hive_muzzle": (0, 0.085, -0.42),
                  "pounder_muzzle": (0, 0.135, -0.6), "mauler_muzzle": (0, 0.14, -0.62),
                  "burster_muzzle": (0, 0.21, -1.0),
                  "pounder_spin_0": (-0.085, 0.135, 0), "pounder_spin_1": (0.085, 0.135, 0)},
    },
    # The MAX exosuit (max.py): each mesh in the frame of the infantry part it replaces.
    "max.glb": {
        "allowed": {"body", "accent", "glow", "glow_white", "halo_white", "flow", "eyes",
                    "hit_1", "hit_2", "hit_3"} | SURFACES,
        "max_materials": 12,
        "required": ("armor", "metal", "accent", "hit_1", "hit_2", "hit_3", "eyes"),
        "meshes": {
            "max_torso": (2500, (-0.34, -0.53, -0.26), (0.34, 0.1, 0.36), {1: -0.52}),
            "max_helmet": (2500, (-0.2, -0.31, -0.15), (0.2, 0.23, 0.25), {}),
            "max_pauldron": (2500, (-0.11, -0.28, -0.19), (0.21, 0.18, 0.19), {}),
            "max_upper_arm": (2500, (-0.1, -0.29, -0.1), (0.1, 0.05, 0.11), {1: -0.27}),
            "max_forearm": (2500, (-0.1, -0.26, -0.11), (0.1, 0.07, 0.12), {1: -0.25}),
            "max_fist": (2500, (-0.085, -0.085, -0.12), (0.085, 0.085, 0.165), {2: 0.075}),
            "max_pelvis": (2500, (-0.25, -0.27, -0.18), (0.25, 0.01, 0.17), {}),
            "max_thigh": (2500, (-0.12, -0.45, -0.12), (0.12, 0.06, 0.12), {1: -0.38}),
            "max_shin": (2500, (-0.1, -0.4, -0.12), (0.1, 0.03, 0.13), {1: -0.37}),
            "max_boot": (2500, (-0.1, -0.081, -0.25), (0.1, 0.07, 0.15), {1: -0.08}),
        },
        "nodes": {},
    },
}


def check_parts(ck, file, spec):
    path = os.path.join(ROOT, "assets/models", file)
    if not os.path.exists(path):
        ck.err("%s missing" % file)
        return None, None
    gltf, binary = load_glb(path)
    meshes = check_common(ck, file, gltf, binary, spec["allowed"])
    check_uvs(ck, file, gltf, binary)
    used = {gltf["materials"][p["material"]]["name"]
            for m in gltf["meshes"] for p in m["primitives"] if "material" in p}
    for r in spec["required"]:
        if r not in used:
            ck.err("%s: material %r not used" % (file, r))
    for m in gltf["meshes"]:
        n = len({p.get("material") for p in m["primitives"]})
        if n > spec.get("max_materials", 99):
            ck.err("%s/%s: %d materials (at most %d)" % (file, m["name"], n, spec["max_materials"]))
    extra = set(meshes) - set(spec["meshes"])
    if extra:
        ck.err("%s: unexpected meshes %s" % (file, sorted(extra)))
    for name, entry in spec["meshes"].items():
        max_tris, bmn, bmx, reach = entry[:4]
        exact = len(entry) > 4 and entry[4] == EXACT
        tol = EXACT_TOL if exact else BOX_TOL
        if name not in meshes:
            ck.err("%s: no mesh named %r" % (file, name))
            continue
        pts, body, tris = meshes[name]
        mn, mx = extent(pts)
        ok = True
        if tris > max_tris:
            ck.err("%s/%s: %d tris (budget %d)" % (file, name, tris, max_tris))
            ok = False
        if exact:
            cmn, cmx = extent(body)
            for ax in range(3):
                if abs(cmn[ax] - bmn[ax]) > 0.002 or abs(cmx[ax] - bmx[ax]) > 0.002:
                    ck.err("%s/%s: body %s extent %.3f..%.3f, collider %.3f..%.3f" % (
                        file, name, "xyz"[ax], cmn[ax], cmx[ax], bmn[ax], bmx[ax]))
                    ok = False
        for ax in range(3):
            if mn[ax] < bmn[ax] - tol or mx[ax] > bmx[ax] + tol:
                ck.err("%s/%s: %s extent %.3f..%.3f outside %.3f..%.3f" % (
                    file, name, "xyz"[ax], mn[ax], mx[ax], bmn[ax], bmx[ax]))
                ok = False
        for ax, want in reach.items():
            if min(abs(p[ax] - want) for p in pts) > REACH:
                ck.err("%s/%s: nothing reaches %s = %.3f" % (file, name, "xyz"[ax], want))
                ok = False
        print("%-4s %s/%-20s %5d tris, extent %s .. %s" % (
            "ok" if ok else "BAD", file, name, tris, [round(v, 3) for v in mn],
            [round(v, 3) for v in mx]))
    nodes = {n.get("name"): n for n in gltf.get("nodes", [])}
    for name, want in spec["nodes"].items():
        node = nodes.get(name)
        if node is None:
            ck.err("%s: no node %r" % (file, name))
            continue
        t = node.get("translation", [0, 0, 0])
        if any(abs(a - b) > 1e-3 for a, b in zip(t, want)):
            ck.err("%s: node %s at %s (want %s)" % (file, name, t, list(want)))
        else:
            print("ok   %s node %s at %s" % (file, name, [round(v, 3) for v in t]))
    return gltf, binary


def check_warpgate_pillars(ck, gltf, binary):
    """The pillars' corners (game colliders) must be vertices of the `warpgate` body."""
    import math
    mats = [m.get("name") for m in gltf.get("materials", [])]
    mesh = next((m for m in gltf["meshes"] if m["name"] == "warpgate"), None)
    if mesh is None:
        return
    body = []
    for prim in mesh["primitives"]:
        if mats[prim["material"]] == "body":
            body += read_accessor(gltf, binary, prim["attributes"]["POSITION"])
    keys = {(round(p[0] * 200), round(p[1] * 200), round(p[2] * 200)) for p in body}
    missing = 0
    for i in range(8):
        a = i / 8 * 2 * math.pi + 0.26
        cx, cz = 40 * math.cos(a), 40 * math.sin(a)
        for k in range(6):
            b = a + k * math.pi / 3
            for y in (-1.0, 55.0):
                p = (cx + 2.4 * math.cos(b), y, cz + 2.4 * math.sin(b))
                if not any((round(p[0] * 200) + dx, round(p[1] * 200), round(p[2] * 200) + dz)
                           in keys for dx in (-1, 0, 1) for dz in (-1, 0, 1)):
                    missing += 1
    if missing:
        ck.err("warpgate.glb: %d pillar collider corners are not body vertices" % missing)
    else:
        print("ok   warpgate.glb pillars match the colliders (96 corners)")


def main():
    ck = Checker()
    check_kit(ck)
    check_model(ck, "blaster.glb", {"body", "accent", "glow", "hit_1", "hit_2", "hit_3", "muzzle"},
                ("body", "accent", "hit_1", "hit_2", "hit_3", "muzzle"), nodes=("muzzle",))
    check_model(ck, "grenade.glb", {"body", "glow_white", "glow"}, ("body", "glow_white"))
    check_avatars(ck)
    check_reload(ck)
    check_structures(ck)
    check_model(ck, "powerups.glb", {"body", "accent", "hit_1", "hit_2", "hit_3", "muzzle"},
                ("body", "accent", "hit_1", "hit_2", "hit_3", "muzzle"), nodes=("shotgun_muzzle",))
    for file, spec in PARTS.items():
        gltf, binary = check_parts(ck, file, spec)
        if file == "warpgate.glb" and gltf is not None:
            check_warpgate_pillars(ck, gltf, binary)
    if ck.errors:
        print("%d problem(s)" % len(ck.errors))
        sys.exit(1)
    print("all good")


if __name__ == "__main__":
    main()
