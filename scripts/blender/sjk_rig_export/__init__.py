"""SJK Rig Model Export: a rigged character into an SJK rig model player folder.

The 3D view's sidebar gets an SJK tab. Pick an armature, name the model and press
Export: the armature and the meshes it deforms are written as glTF (+Y up, rest pose,
skins, no animation or shape keys) into `<GameData>/base/models/players/<name>/`,
each material reduced to its base colour image, with an `sjk_rig.cfg` beside them.
In the game, `/model <name>` wears it. The blend file is left as it was: the export
works on temporary copies of the meshes and materials and removes them afterwards.

See docs/rig-models.md in the SJK repository for the format and its limits.
"""

import os
import re

import bpy
from bpy.props import BoolProperty, EnumProperty, PointerProperty, StringProperty
from bpy.types import AddonPreferences, Operator, Panel, PropertyGroup

# The joints each bone map preset drives, as `rig_model.rs` `PRESETS` names them
# (keep the two in step). Only used to tell the user whether the skeleton fits.
PRESET_JOINTS = {
    "mannequin": [
        "pelvis", "spine_01", "spine_02", "spine_03", "spine_04", "spine_05",
        "neck_01", "neck_02", "head",
        *(
            f"{part}_{side}"
            for side in ("l", "r")
            for part in ("upperarm", "lowerarm", "hand", "thigh", "calf", "foot")
        ),
    ],
}

PRESET_ITEMS = [
    ("mannequin", "Unreal mannequin", "pelvis, spine_01, upperarm_l, hand_r, thumb_01_l..."),
]

# Joints the game cannot do without: `scale auto` fits the pelvis, the saber hand.
REQUIRED = ("pelvis", "hand_r")

# Where a Steam or retail install usually keeps GameData, tried when none is set.
GAME_DATA_GUESSES = (
    r"C:\Program Files (x86)\Steam\steamapps\common\Jedi Academy\GameData",
    r"C:\Program Files\Steam\steamapps\common\Jedi Academy\GameData",
    r"C:\Program Files (x86)\LucasArts\Star Wars Jedi Knight Jedi Academy\GameData",
    os.path.expanduser("~/.steam/steam/steamapps/common/Jedi Academy/GameData"),
)

# Image nodes or sockets that hold a material's base colour, as exporters name them.
COLOR_WORDS = ("base color", "basecolor", "diffuse", "albedo")
COLOR_SUFFIX = re.compile(r"(_d|_diffuse|_basecolor|_base_color|_albedo|_bc|_col|_color)$")

MANIFEST = "sjk_rig.cfg"


def preferences(context):
    return context.preferences.addons[__package__].preferences


def game_data(context):
    """The GameData folder from the preferences, else the first usual one found."""
    chosen = bpy.path.abspath(preferences(context).game_data).strip()
    if chosen:
        return chosen
    return next((path for path in GAME_DATA_GUESSES if os.path.isdir(path)), "")


def model_name(settings):
    """The folder name: lower case letters, digits and underscores."""
    name = settings.model_name.strip() or (settings.armature.name if settings.armature else "")
    return re.sub(r"[^a-z0-9_]+", "_", name.lower()).strip("_")


def deformed_meshes(context, armature, visible_only):
    """The meshes `armature` deforms (an Armature modifier on it, or parented to it)."""
    meshes = []
    for obj in context.scene.objects:
        if obj.type != "MESH":
            continue
        rigged = any(m.type == "ARMATURE" and m.object == armature for m in obj.modifiers)
        if not rigged and obj.parent != armature:
            continue
        if visible_only and not obj.visible_get():
            continue
        meshes.append(obj)
    return meshes


def joint_report(armature, preset):
    """(found, expected, missing required) for the preset's joints on `armature`."""
    names = {bone.name for bone in armature.data.bones}
    expected = PRESET_JOINTS.get(preset, [])
    found = sum(1 for joint in expected if joint in names)
    missing = [joint for joint in REQUIRED if joint not in names]
    return found, len(expected), missing


def base_color_image(material):
    """The image a material's base colour most likely comes from, or None."""
    if material is None or material.node_tree is None:
        return None
    best = None
    for node in material.node_tree.nodes:
        if node.type != "TEX_IMAGE" or node.image is None:
            continue
        sockets = [link.to_socket.name.lower() for link in node.outputs["Color"].links]
        label = (node.label or node.name).lower()
        stem = os.path.splitext(node.image.name.lower())[0]
        if any(socket in COLOR_WORDS for socket in sockets):
            rank = 0
        elif label in COLOR_WORDS:
            rank = 1
        elif COLOR_SUFFIX.search(stem):
            rank = 2
        elif node.image.colorspace_settings.name == "sRGB":
            rank = 3
        else:
            continue
        if best is None or rank < best[0]:
            best = (rank, node.image)
    return best[1] if best else None


def plain_material(source):
    """A temporary Principled material with `source`'s base colour image."""
    material = bpy.data.materials.new(f"SJK_export_{source.name if source else 'none'}")
    if material.node_tree is None:  # before Blender 5, materials start without nodes
        material.use_nodes = True
    tree = material.node_tree
    bsdf = next(node for node in tree.nodes if node.type == "BSDF_PRINCIPLED")
    image = base_color_image(source)
    if image is not None:
        texture = tree.nodes.new("ShaderNodeTexImage")
        texture.image = image
        tree.links.new(texture.outputs["Color"], bsdf.inputs["Base Color"])
    elif source is not None:
        bsdf.inputs["Base Color"].default_value = source.diffuse_color
    return material


def write_manifest(path, preset):
    lines = [
        "// SJK rig model, written by the SJK Rig Model Export add-on.",
        "// Format: docs/rig-models.md in the SJK repository.",
        "model model.gltf",
        f"preset {preset}",
        "scale auto",
        "",
    ]
    with open(path, "w", encoding="utf-8", newline="\n") as file:
        file.write("\n".join(lines))


class SJKRigExportPreferences(AddonPreferences):
    bl_idname = __package__

    game_data: StringProperty(
        name="GameData folder",
        description="Jedi Academy's GameData folder (empty: found automatically)",
        subtype="DIR_PATH",
        default="",
    )

    def draw(self, context):
        self.layout.prop(self, "game_data")
        self.layout.label(text=f"In use: {game_data(context) or 'not found'}")


class SJKRigExportSettings(PropertyGroup):
    armature: PointerProperty(
        name="Armature",
        type=bpy.types.Object,
        poll=lambda _self, obj: obj.type == "ARMATURE",
    )
    model_name: StringProperty(
        name="Model name",
        description="The folder under models/players and the name /model takes "
        "(empty: from the armature's name)",
        default="",
    )
    preset: EnumProperty(name="Bone map", items=PRESET_ITEMS, default="mannequin")
    visible_only: BoolProperty(
        name="Visible meshes only",
        description="Leave out hidden meshes (other LODs, spare parts)",
        default=True,
    )
    overwrite_manifest: BoolProperty(
        name="Rewrite sjk_rig.cfg",
        description="Replace an existing manifest (and its map lines) with a fresh one",
        default=False,
    )


class SJK_OT_export_rig_model(Operator):
    bl_idname = "sjk.export_rig_model"
    bl_label = "Export to SJK"
    bl_description = "Write the armature and its meshes as an SJK rig model player"
    bl_options = {"REGISTER"}

    @classmethod
    def poll(cls, context):
        settings = context.scene.sjk_rig_export
        return settings.armature is not None and settings.armature.type == "ARMATURE"

    def invoke(self, context, event):
        folder = self.folder(context)
        if folder and os.path.exists(os.path.join(folder, "model.gltf")):
            return context.window_manager.invoke_confirm(
                self, event, message=f"Replace the model in {folder}?"
            )
        return self.execute(context)

    def folder(self, context):
        root = game_data(context)
        name = model_name(context.scene.sjk_rig_export)
        if not root or not name:
            return ""
        return os.path.join(root, "base", "models", "players", name)

    def execute(self, context):
        settings = context.scene.sjk_rig_export
        armature = settings.armature
        name = model_name(settings)
        root = game_data(context)
        if not root or not os.path.isdir(root):
            self.report({"ERROR"}, "Set the GameData folder in the add-on's preferences")
            return {"CANCELLED"}
        if not name:
            self.report({"ERROR"}, "Give the model a name")
            return {"CANCELLED"}
        _, _, missing = joint_report(armature, settings.preset)
        if missing:
            self.report({"ERROR"}, f"The armature has no {', '.join(missing)} joint")
            return {"CANCELLED"}
        meshes = deformed_meshes(context, armature, settings.visible_only)
        if not meshes:
            self.report({"ERROR"}, "No mesh is deformed by this armature")
            return {"CANCELLED"}
        if not armature.visible_get():
            self.report({"ERROR"}, "Show the armature first (it must be selectable)")
            return {"CANCELLED"}
        folder = self.folder(context)
        os.makedirs(folder, exist_ok=True)
        try:
            self.export(context, armature, meshes, os.path.join(folder, "model.gltf"))
        except Exception as error:  # report it in Blender rather than a traceback
            self.report({"ERROR"}, f"Export failed: {error}")
            return {"CANCELLED"}
        manifest = os.path.join(folder, MANIFEST)
        if settings.overwrite_manifest or not os.path.exists(manifest):
            write_manifest(manifest, settings.preset)
        self.report({"INFO"}, f"Exported to {folder}. In the game: /model {name}")
        return {"FINISHED"}

    def export(self, context, armature, meshes, path):
        if context.mode != "OBJECT":
            bpy.ops.object.mode_set(mode="OBJECT")
        selected = [obj for obj in context.selected_objects]
        active = context.view_layer.objects.active
        pose_position = armature.data.pose_position
        copies, meshes_data, materials = [], [], {}
        try:
            armature.data.pose_position = "REST"
            for source in meshes:
                copy = source.copy()
                copy.data = source.data.copy()
                copy.name = f"SJK_export_{source.name}"
                context.scene.collection.objects.link(copy)
                copies.append(copy)
                meshes_data.append(copy.data)
                if copy.data.shape_keys:
                    copy.shape_key_clear()
                for index, slot in enumerate(copy.material_slots):
                    key = slot.material.name if slot.material else ""
                    if key not in materials:
                        materials[key] = plain_material(slot.material)
                    if slot.link == "OBJECT":
                        slot.material = materials[key]
                    else:
                        copy.data.materials[index] = materials[key]
            for obj in context.view_layer.objects:
                obj.select_set(False)
            armature.select_set(True)
            for copy in copies:
                copy.select_set(True)
            context.view_layer.objects.active = armature
            bpy.ops.export_scene.gltf(
                filepath=path,
                export_format="GLTF_SEPARATE",
                use_selection=True,
                export_animations=False,
                export_morph=False,
                export_skins=True,
                export_yup=True,
                export_apply=False,
                export_rest_position_armature=True,
                export_materials="EXPORT",
                export_image_format="AUTO",
            )
        finally:
            for copy in copies:
                bpy.data.objects.remove(copy, do_unlink=True)
            for data in meshes_data:
                if data.users == 0:
                    bpy.data.meshes.remove(data)
            for material in materials.values():
                bpy.data.materials.remove(material)
            armature.data.pose_position = pose_position
            for obj in context.view_layer.objects:
                obj.select_set(obj in selected)
            context.view_layer.objects.active = active


class SJK_PT_rig_export(Panel):
    bl_label = "SJK Rig Model"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "SJK"

    def draw(self, context):
        layout = self.layout
        settings = context.scene.sjk_rig_export
        layout.prop(settings, "armature")
        armature = settings.armature
        if armature is not None:
            found, expected, missing = joint_report(armature, settings.preset)
            meshes = deformed_meshes(context, armature, settings.visible_only)
            icon = "ERROR" if missing or found < expected // 2 else "CHECKMARK"
            box = layout.box()
            box.label(text=f"Bone map joints found: {found} of {expected}", icon=icon)
            if missing:
                box.label(text=f"Missing: {', '.join(missing)}", icon="CANCEL")
            box.label(text=f"Meshes: {len(meshes)}", icon="MESH_DATA")
        layout.prop(settings, "model_name")
        layout.prop(settings, "preset")
        layout.prop(settings, "visible_only")
        layout.prop(settings, "overwrite_manifest")
        root = game_data(context)
        name = model_name(settings)
        if root and name:
            layout.label(text=f".../base/models/players/{name}", icon="FILE_FOLDER")
        elif not root:
            layout.label(text="GameData not found: set it in the preferences", icon="ERROR")
        layout.operator(SJK_OT_export_rig_model.bl_idname, icon="EXPORT")
        note = layout.column(align=True)
        note.scale_y = 0.8
        note.label(text="For your own files, on your own disk.")
        note.label(text="Never upload or share models you have no rights to.")


CLASSES = (
    SJKRigExportPreferences,
    SJKRigExportSettings,
    SJK_OT_export_rig_model,
    SJK_PT_rig_export,
)


def register():
    for cls in CLASSES:
        bpy.utils.register_class(cls)
    bpy.types.Scene.sjk_rig_export = PointerProperty(type=SJKRigExportSettings)


def unregister():
    del bpy.types.Scene.sjk_rig_export
    for cls in reversed(CLASSES):
        bpy.utils.unregister_class(cls)
